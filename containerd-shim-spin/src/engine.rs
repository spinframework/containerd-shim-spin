use std::{collections::HashSet, env, hash::Hash};

use anyhow::{Context, Result};
use containerd_shim_wasm::{
    sandbox::{
        context::{RuntimeContext, WasmLayer},
        Sandbox,
    },
    shim::{version, Compiler, Shim, Version},
};
use futures::future;
use log::info;
use spin_app::locked::LockedApp;
use spin_factor_outbound_networking::validate_service_chaining_for_components;
use spin_trigger::cli::NoCliArgs;
use spin_trigger_http::HttpTrigger;
use spin_trigger_redis::RedisTrigger;
use trigger_command::CommandTrigger;
use trigger_mqtt::MqttTrigger;
use trigger_sqs::SqsTrigger;

use crate::{
    constants,
    source::Source,
    trigger::{
        self, get_supported_triggers, COMMAND_TRIGGER_TYPE, HTTP_TRIGGER_TYPE, MQTT_TRIGGER_TYPE,
        REDIS_TRIGGER_TYPE, SQS_TRIGGER_TYPE,
    },
    utils::{
        configure_application_variables_from_environment_variables, initialize_cache,
        is_wasm_content, parse_addr,
    },
};

pub struct SpinShim;
pub struct SpinCompiler(wasmtime::Engine);

#[derive(Default)]
pub struct SpinSandbox;

impl Shim for SpinShim {
    type Sandbox = SpinSandbox;

    fn name() -> &'static str {
        "spin"
    }

    fn version() -> Version {
        version!()
    }

    fn supported_layers_types() -> &'static [&'static str] {
        &[
            constants::OCI_LAYER_MEDIA_TYPE_WASM,
            constants::OCI_LAYER_MEDIA_TYPE_WASM_WKG,
            spin_oci::client::ARCHIVE_MEDIATYPE,
            spin_oci::client::DATA_MEDIATYPE,
            spin_oci::client::SPIN_APPLICATION_MEDIA_TYPE,
        ]
    }

    #[allow(refining_impl_trait)]
    async fn compiler() -> Option<SpinCompiler> {
        // the host expects epoch interruption to be enabled, so this has to be
        // turned on for the components we compile.
        let mut config = wasmtime::Config::default();
        config.epoch_interruption(true);
        // Turn off native unwinding to avoid faulty libunwind detection error
        // TODO: This can be removed once the Wasmtime fix is brought into Spin
        // Issue to track: https://github.com/fermyon/spin/issues/2889
        config.native_unwind_info(false);
        Some(SpinCompiler(wasmtime::Engine::new(&config).unwrap()))
    }
}

impl Sandbox for SpinSandbox {
    async fn run_wasi(&self, ctx: &impl RuntimeContext) -> Result<i32> {
        // Set the container environment variables which will be collected by Spin's
        // [environment variable provider]. We use these variables to configure both the Spin runtime
        // and the Spin application per the [SKIP 003] proposal.
        //
        // TODO: This is a temporary solution to allow Spin to collect the container environment variables.
        // We should later look into other variable providers to collect container variables.
        //
        // [SKIP 003]: https://github.com/spinkube/skips/tree/main/proposals/003-shim-runtime-options
        // [environment variable provider]: https://github.com/fermyon/spin/blob/v3.0.0/crates/variables/src/env.rs
        ctx.envs().iter().for_each(|v| {
            let (key, value) = v.split_once('=').unwrap_or((v.as_str(), ""));
            env::set_var(key, value);
        });

        info!("setting up wasi");

        let (abortable, abort_handle) = futures::future::abortable(self.wasm_exec_async(ctx));
        ctrlc::set_handler(move || abort_handle.abort())?;

        match abortable.await {
            Ok(Ok(())) => {
                info!("run_wasi shut down: exiting");
                Ok(0)
            }
            Ok(Err(err)) => {
                log::error!("run_wasi ERROR >>>  failed: {err:?}");
                Err(err)
            }
            Err(aborted) => {
                info!("Received signal to abort: {aborted:?}");
                Ok(0)
            }
        }
    }

    async fn can_handle(&self, _ctx: &impl RuntimeContext) -> Result<()> {
        Ok(())
    }
}

impl SpinSandbox {
    async fn wasm_exec_async(&self, ctx: &impl RuntimeContext) -> Result<()> {
        let cache = initialize_cache().await?;
        let app_source = Source::from_ctx(ctx, &cache).await?;
        let mut locked_app = app_source.to_locked_app(&cache).await?;
        if let Ok(components_env) = env::var(constants::SPIN_COMPONENTS_TO_RETAIN_ENV) {
            let components = components_env
                .split(',')
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>();
            locked_app = spin_app::retain_components(
                locked_app,
                &components,
                &[&validate_service_chaining_for_components],
            )
            .with_context(|| {
                format!(
                    "failed to resolve application with only [{components:?}] components retained by configured environment variable {}", constants::SPIN_COMPONENTS_TO_RETAIN_ENV
                )
            })?;
        }
        configure_application_variables_from_environment_variables(&locked_app)?;
        let trigger_cmds = get_supported_triggers(&locked_app)
            .with_context(|| format!("Couldn't find trigger executor for {app_source:?}"))?;
        spin_telemetry::init(version!().version.to_string())?;

        self.run_trigger(ctx, &trigger_cmds, locked_app, app_source)
            .await
    }

    async fn run_trigger(
        &self,
        ctx: &impl RuntimeContext,
        trigger_types: &HashSet<String>,
        app: LockedApp,
        app_source: Source,
    ) -> Result<()> {
        let mut loader = spin_trigger::loader::ComponentLoader::default();
        match app_source {
            Source::OciSpin | Source::OciWkg(_) => unsafe {
                // Configure the loader to support loading AOT compiled components..
                // Since all components were compiled by the shim (during `precompile`),
                // this operation can be considered safe.
                loader.enable_loading_aot_compiled_components();
            },
            // Currently, it is only possible to precompile applications distributed using
            // `spin registry push`
            Source::File(_) => {}
        };

        let mut futures_list = Vec::new();
        let mut trigger_type_map = Vec::new();
        // The `HOSTNAME` environment variable should contain the fully unique container name
        let app_id = std::sync::Arc::<str>::from(
            std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
        );
        for trigger_type in trigger_types.iter() {
            let app = spin_app::App::new(app_id.clone(), app.clone());
            let f = match trigger_type.as_str() {
                HTTP_TRIGGER_TYPE => {
                    let address_str = env::var(constants::SPIN_HTTP_LISTEN_ADDR_ENV)
                        .unwrap_or_else(|_| constants::SPIN_ADDR_DEFAULT.to_string());
                    let address = parse_addr(&address_str)?;
                    let cli_args = spin_trigger_http::CliArgs {
                        address,
                        tls_cert: None,
                        tls_key: None,
                        find_free_port: false,
                        http1_max_buf_size: None,
                        max_instance_reuse_count: None,
                        max_instance_concurrent_reuse_count: None,
                        request_timeout: None,
                        idle_instance_timeout: spin_trigger_http::Range::Value(
                            std::time::Duration::from_secs(1),
                        ),
                    };
                    trigger::run::<HttpTrigger>(cli_args, app, &loader).await?
                }
                REDIS_TRIGGER_TYPE => trigger::run::<RedisTrigger>(NoCliArgs, app, &loader).await?,
                SQS_TRIGGER_TYPE => trigger::run::<SqsTrigger>(NoCliArgs, app, &loader).await?,
                COMMAND_TRIGGER_TYPE => {
                    let cli_args = trigger_command::CliArgs {
                        guest_args: ctx.args().to_vec(),
                    };
                    trigger::run::<CommandTrigger>(cli_args, app, &loader).await?
                }
                MQTT_TRIGGER_TYPE => {
                    let cli_args = trigger_mqtt::CliArgs { test: false };
                    trigger::run::<MqttTrigger>(cli_args, app, &loader).await?
                }
                _ => {
                    // This should never happen as we check for supported triggers in get_supported_triggers
                    unreachable!()
                }
            };

            trigger_type_map.push(trigger_type.clone());
            futures_list.push(f);
        }

        info!(" >>> notifying main thread we are about to start");

        // exit as soon as any of the trigger completes/exits
        let (result, index, rest) = future::select_all(futures_list).await;
        let trigger_type = &trigger_type_map[index];

        info!(" >>> trigger type '{trigger_type}' exited");

        drop(rest);

        result
    }
}

impl Compiler for SpinCompiler {
    fn cache_key(&self) -> impl Hash {
        self.0.precompile_compatibility_hash()
    }

    async fn compile(&self, layers: &[WasmLayer]) -> Result<Vec<Option<Vec<u8>>>> {
        // Runwasi expects layers to be returned in the same order, so wrap each layer in an option, setting non Wasm layers to None
        let precompiled_layers = layers
            .iter()
            .map(|layer| match is_wasm_content(layer) {
                Some(wasm_layer) => {
                    log::info!(
                        "Precompile called for wasm layer {:?}",
                        wasm_layer.config.digest()
                    );
                    if wasmtime::Engine::detect_precompiled(&wasm_layer.layer).is_some() {
                        log::info!("Layer already precompiled {:?}", wasm_layer.config.digest());
                        Ok(Some(wasm_layer.layer))
                    } else {
                        let component =
                            spin_componentize::componentize_if_necessary(&wasm_layer.layer)?;
                        let precompiled = self.0.precompile_component(&component)?;
                        Ok(Some(precompiled))
                    }
                }
                None => Ok(None),
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(precompiled_layers)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use oci_spec::image::{Digest, MediaType};

    use super::*;

    #[tokio::test]
    async fn precompile() {
        let module = wat::parse_str("(module)").unwrap();
        let wasmtime_engine = wasmtime::Engine::default();
        let component = wasmtime::component::Component::new(&wasmtime_engine, "(component)")
            .unwrap()
            .serialize()
            .unwrap();
        let wasm_layers: Vec<WasmLayer> = vec![
            // Needs to be precompiled
            WasmLayer {
                layer: module.clone(),
                config: oci_spec::image::Descriptor::new(
                    MediaType::Other(constants::OCI_LAYER_MEDIA_TYPE_WASM.to_string()),
                    1024,
                    Digest::from_str(
                        "sha256:6c3c624b58dbbcd3c0dd82b4c53f04194d1247c6eebdaab7c610cf7d66709b3b",
                    )
                    .unwrap(),
                ),
            },
            // Precompiled
            WasmLayer {
                layer: component.to_owned(),
                config: oci_spec::image::Descriptor::new(
                    MediaType::Other(constants::OCI_LAYER_MEDIA_TYPE_WASM.to_string()),
                    1024,
                    Digest::from_str(
                        "sha256:6c3c624b58dbbcd3c0dd82b4c53f04194d1247c6eebdaab7c610cf7d66709b3b",
                    )
                    .unwrap(),
                ),
            },
            // Content that should be skipped
            WasmLayer {
                layer: vec![],
                config: oci_spec::image::Descriptor::new(
                    MediaType::Other(spin_oci::client::DATA_MEDIATYPE.to_string()),
                    1024,
                    Digest::from_str(
                        "sha256:6c3c624b58dbbcd3c0dd82b4c53f04194d1247c6eebdaab7c610cf7d66709b3b",
                    )
                    .unwrap(),
                ),
            },
        ];
        let compiler = SpinCompiler(wasmtime_engine);
        let precompiled = compiler
            .compile(&wasm_layers)
            .await
            .expect("compile failed");
        assert_eq!(precompiled.len(), 3);
        assert_ne!(precompiled[0].as_deref().expect("no first entry"), module);
        assert_eq!(
            precompiled[1].as_deref().expect("no second entry"),
            component
        );
        assert!(precompiled[2].is_none());
    }
}
