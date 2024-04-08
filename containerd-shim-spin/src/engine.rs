use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    net::{Ipv4Addr, SocketAddr, ToSocketAddrs},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use containerd_shim_wasm::{
    container::{Engine, RuntimeContext, Source, Stdio},
    sandbox::WasmLayer,
};
use log::info;
use oci_spec::image::MediaType;
use spin_app::locked::LockedApp;
use spin_loader::FilesMountStrategy;
use spin_trigger::{
    loader::TriggerLoader, RuntimeConfig, TriggerExecutor, TriggerExecutorBuilder, TriggerHooks,
};
use spin_trigger_http::HttpTrigger;
use spin_trigger_redis::RedisTrigger;
use tokio::runtime::Runtime;
// use trigger_command::CommandTrigger;
// use trigger_sqs::SqsTrigger;
use url::Url;

use crate::loader::ContainerdLoader;

const SPIN_ADDR: &str = "0.0.0.0:80";
/// RUNTIME_CONFIG_PATH specifies the expected location and name of the runtime
/// config for a Spin application. The runtime config should be loaded into the
/// root `/` of the container.
const RUNTIME_CONFIG_PATH: &str = "/runtime-config.toml";
/// Describes an OCI layer with Wasm content
const OCI_LAYER_MEDIA_TYPE_WASM: &str = "application/vnd.wasm.content.layer.v1+wasm";
/// Describes an OCI layer with data content
const OCI_LAYER_MEDIA_TYPE_DATA: &str = "application/vnd.wasm.content.layer.v1+data";
/// Describes an OCI layer containing a Spin application config
const OCI_LAYER_MEDIA_TYPE_SPIN_CONFIG: &str = "application/vnd.fermyon.spin.application.v1+config";
/// Expected location of the Spin manifest when loading from a file rather than
/// an OCI image
const SPIN_MANIFEST_FILE_PATH: &str = "/spin.toml";

impl Engine for SpinEngine {
    fn name() -> &'static str {
        "spin"
    }

    fn run_wasi(&self, ctx: &impl RuntimeContext, stdio: Stdio) -> Result<i32> {
        stdio.redirect()?;
        info!("setting up wasi");
        let rt = Runtime::new().context("failed to create runtime")?;

        let (abortable, abort_handle) = futures::future::abortable(self.wasm_exec_async(ctx));
        ctrlc::set_handler(move || abort_handle.abort())?;

        match rt.block_on(abortable) {
            Ok(Ok(())) => {
                info!("run_wasi shut down: exiting");
                Ok(0)
            }
            Ok(Err(err)) => {
                log::error!("run_wasi ERROR >>>  failed: {:?}", err);
                Err(err)
            }
            Err(aborted) => {
                info!("Received signal to abort: {:?}", aborted);
                Ok(0)
            }
        }
    }

    fn can_handle(&self, _ctx: &impl RuntimeContext) -> Result<()> {
        Ok(())
    }

    fn supported_layers_types() -> &'static [&'static str] {
        &[
            OCI_LAYER_MEDIA_TYPE_WASM,
            OCI_LAYER_MEDIA_TYPE_DATA,
            OCI_LAYER_MEDIA_TYPE_SPIN_CONFIG,
        ]
    }

    fn precompile(&self, layers: &[WasmLayer]) -> Result<Vec<Option<Vec<u8>>>> {
        // Runwasi expects layers to be returned in the same order, so wrap each layer in an option, setting non Wasm layers to None
        let precompiled_layers = layers
            .iter()
            .map(|layer| match SpinEngine::is_wasm_content(layer) {
                Some(wasm_layer) => {
                    log::info!(
                        "Precompile called for wasm layer {:?}",
                        wasm_layer.config.digest()
                    );
                    if self
                        .wasmtime_engine
                        .detect_precompiled(&wasm_layer.layer)
                        .is_some()
                    {
                        log::info!("Layer already precompiled {:?}", wasm_layer.config.digest());
                        Ok(Some(wasm_layer.layer))
                    } else {
                        let component =
                            spin_componentize::componentize_if_necessary(&wasm_layer.layer)?;
                        let precompiled = self.wasmtime_engine.precompile_component(&component)?;
                        Ok(Some(precompiled))
                    }
                }
                None => Ok(None),
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(precompiled_layers)
    }

    fn can_precompile(&self) -> Option<String> {
        let mut hasher = DefaultHasher::new();
        self.wasmtime_engine
            .precompile_compatibility_hash()
            .hash(&mut hasher);
        Some(hasher.finish().to_string())
    }
}

#[derive(Clone)]
pub struct SpinEngine {
    pub wasmtime_engine: wasmtime::Engine,
    pub working_dir: PathBuf,
}

impl Default for SpinEngine {
    fn default() -> Self {
        // the host expects epoch interruption to be enabled, so this has to be
        // turned on for the components we compile.
        let mut config = wasmtime::Config::default();
        let working_dir = PathBuf::from("/");
        config.epoch_interruption(true);
        Self {
            wasmtime_engine: wasmtime::Engine::new(&config)
                .expect("cannot create new Wasmtime engine for SpinEngine"),
            working_dir,
        }
    }
}

impl SpinEngine {
    async fn wasm_exec_async(&self, ctx: &impl RuntimeContext) -> Result<()> {
        let app = self.load(ctx.entrypoint().source).await?;
        self.run(app, ctx.entrypoint().source).await
    }

    async fn run<'a>(&self, app: LockedApp, source: Source<'a>) -> Result<()> {
        let trigger = Self::trigger_command(&app)?;

        let f = match trigger.as_str() {
            HttpTrigger::TRIGGER_TYPE => {
                let http_trigger: HttpTrigger = self
                    .build_trigger(app, source)
                    .await
                    .context("failed to build spin trigger")?;

                info!(" >>> running spin trigger");
                http_trigger.run(spin_trigger_http::CliArgs {
                    address: Self::parse_listen_addr(SPIN_ADDR)?,
                    tls_cert: None,
                    tls_key: None,
                })
            }
            RedisTrigger::TRIGGER_TYPE => {
                let redis_trigger: RedisTrigger = self
                    .build_trigger(app, source)
                    .await
                    .context("failed to build spin trigger")?;

                info!(" >>> running spin trigger");
                redis_trigger.run(spin_trigger::cli::NoArgs)
            }
            // SqsTrigger::TRIGGER_TYPE => {
            //     let sqs_trigger: SqsTrigger = self
            //         .build_spin_trigger(working_dir, app, app_source)
            //         .await
            //         .context("failed to build spin trigger")?;
            //
            //     info!(" >>> running spin trigger");
            //     sqs_trigger.run(spin_trigger::cli::NoArgs)
            // }
            // CommandTrigger::TRIGGER_TYPE => {
            //     let command_trigger: CommandTrigger = self
            //         .build_spin_trigger(working_dir, app, app_source)
            //         .await
            //         .context("failed to build spin trigger")?;
            //
            //     info!(" >>> running spin trigger");
            //     command_trigger.run(trigger_command::CliArgs {
            //         guest_args: ctx.args().to_vec(),
            //     })
            // }
            _ => {
                todo!("Only Http, Redis and SQS triggers are currently supported.")
            }
        };
        info!(" >>> notifying main thread we are about to start");
        f.await
    }

    async fn load<'a>(&self, source: Source<'a>) -> Result<LockedApp> {
        match source {
            Source::File(_) => {
                let files_mount_strategy = FilesMountStrategy::Direct;
                spin_loader::from_file(
                    &PathBuf::from(SPIN_MANIFEST_FILE_PATH),
                    files_mount_strategy,
                    None,
                )
                .await
            }
            Source::Oci(layers) => {
                let loader = ContainerdLoader::new(&self.working_dir);
                loader.load_from_layers(layers).await
            }
        }

        // Ok((app, trigger))
    }

    fn trigger_command(app: &LockedApp) -> Result<String> {
        // TODO: gracefully handle multiple trigger types
        Ok(app
            .triggers
            .first()
            .context("expected app to have one trigger")?
            .trigger_type
            .clone())
    }

    async fn build_trigger<'a, T: spin_trigger::TriggerExecutor>(
        &self,
        app: LockedApp,
        source: Source<'a>,
    ) -> Result<T>
    where
        for<'de> <T as TriggerExecutor>::TriggerConfig: serde::de::Deserialize<'de>,
    {
        let locked_url = self.write_locked(&app, &self.working_dir).await?;
        let mut loader = TriggerLoader::new(&self.working_dir, true);

        match source {
            Source::Oci(_) => unsafe {
                // Configure the loader to support loading AOT compiled components..
                // Since all components were compiled by the shim (during `precompile`),
                // this operation can be considered safe.
                loader.enable_loading_aot_compiled_components();
            },
            Source::File(_) => {}
        };

        let mut runtime_config = RuntimeConfig::new(PathBuf::from("/").into());
        // Load in runtime config if one exists at expected location
        if Path::new(RUNTIME_CONFIG_PATH).exists() {
            runtime_config.merge_config_file(RUNTIME_CONFIG_PATH)?;
        }
        let mut builder = TriggerExecutorBuilder::new(loader);
        builder
            .hooks(StdioTriggerHook {})
            .config_mut()
            .wasmtime_config()
            .cranelift_opt_level(spin_core::wasmtime::OptLevel::Speed);
        let init_data = Default::default();
        let executor = builder.build(locked_url, runtime_config, init_data).await?;
        Ok(executor)
    }

    async fn write_locked(&self, locked_app: &LockedApp, working_dir: &Path) -> Result<String> {
        let locked_path = working_dir.join("spin.lock");
        let locked_app_contents =
            serde_json::to_vec_pretty(&locked_app).context("failed to serialize locked app")?;
        tokio::fs::write(&locked_path, locked_app_contents)
            .await
            .with_context(|| format!("failed to write {:?}", locked_path))?;
        let locked_url = Url::from_file_path(&locked_path)
            .map_err(|_| anyhow!("cannot convert to file URL: {locked_path:?}"))?
            .to_string();

        Ok(locked_url)
    }

    // Returns Some(WasmLayer) if the layer contains wasm, otherwise None
    fn is_wasm_content(layer: &WasmLayer) -> Option<WasmLayer> {
        if let MediaType::Other(name) = layer.config.media_type() {
            if name == OCI_LAYER_MEDIA_TYPE_WASM {
                return Some(layer.clone());
            }
        }
        None
    }

    pub fn parse_listen_addr(addr: &str) -> anyhow::Result<SocketAddr> {
        let addrs: Vec<SocketAddr> = addr.to_socket_addrs()?.collect();
        // Prefer 127.0.0.1 over e.g. [::1] because CHANGE IS HARD
        if let Some(addr) = addrs
            .iter()
            .find(|addr| addr.is_ipv4() && addr.ip() == Ipv4Addr::LOCALHOST)
        {
            return Ok(*addr);
        }
        // Otherwise, take the first addr (OS preference)
        addrs.into_iter().next().context("couldn't resolve address")
    }
}

struct StdioTriggerHook;
impl TriggerHooks for StdioTriggerHook {
    fn app_loaded(
        &mut self,
        _app: &spin_app::App,
        _runtime_config: &RuntimeConfig,
        _resolver: &std::sync::Arc<spin_expressions::PreparedResolver>,
    ) -> Result<()> {
        Ok(())
    }

    fn component_store_builder(
        &self,
        _component: &spin_app::AppComponent,
        builder: &mut spin_core::StoreBuilder,
    ) -> Result<()> {
        builder.inherit_stdout();
        builder.inherit_stderr();
        Ok(())
    }
}
