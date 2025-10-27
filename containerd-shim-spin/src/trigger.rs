use std::{collections::HashSet, path::Path};

use anyhow::Result;
use futures::{future::BoxFuture, FutureExt};
use log::{debug, info};
use spin_app::{locked::LockedApp, App};
use spin_runtime_factors::{FactorsBuilder, TriggerAppArgs, TriggerFactors};
use spin_trigger::{
    cli::{FactorsConfig, TriggerAppBuilder, UserProvidedPath},
    loader::ComponentLoader,
    Trigger,
};
use spin_trigger_http::HttpTrigger;
use spin_trigger_redis::RedisTrigger;
use trigger_command::CommandTrigger;
use trigger_mqtt::MqttTrigger;
use trigger_sqs::SqsTrigger;

use crate::constants::{RUNTIME_CONFIG_PATH, SPIN_TRIGGER_WORKING_DIR};

pub(crate) const HTTP_TRIGGER_TYPE: &str = <HttpTrigger as Trigger<TriggerFactors>>::TYPE;
pub(crate) const REDIS_TRIGGER_TYPE: &str = <RedisTrigger as Trigger<TriggerFactors>>::TYPE;
pub(crate) const SQS_TRIGGER_TYPE: &str = <SqsTrigger as Trigger<TriggerFactors>>::TYPE;
pub(crate) const MQTT_TRIGGER_TYPE: &str = <MqttTrigger as Trigger<TriggerFactors>>::TYPE;
pub(crate) const COMMAND_TRIGGER_TYPE: &str = <CommandTrigger as Trigger<TriggerFactors>>::TYPE;

/// Run the trigger with the given CLI args, [`App`] and [`ComponentLoader`].
pub(crate) async fn run<T>(
    cli_args: T::CliArgs,
    app: App,
    loader: &ComponentLoader,
) -> Result<BoxFuture<'static, Result<()>>>
where
    T: Trigger<TriggerFactors> + 'static,
{
    info!(" >>> running {} trigger", T::TYPE);
    let trigger = T::new(cli_args, &app)?;
    let builder: TriggerAppBuilder<_, FactorsBuilder> = TriggerAppBuilder::new(trigger);
    let builder_args = match std::env::var("SPIN_MAX_INSTANCE_MEMORY") {
        Ok(limit) => {
            debug!("Setting instance max memory to {limit} bytes");
            let mut args = TriggerAppArgs::default();
            args.max_instance_memory = limit.parse().ok();
            args
        }
        Err(_) => Default::default(),
    };
    let future = builder
        .run(app, factors_config(), builder_args, loader)
        .await?;
    Ok(future.boxed())
}

/// Configuration for the factors.
fn factors_config() -> FactorsConfig {
    // Load in runtime config if one exists at expected location
    let runtime_config_file = Path::new(RUNTIME_CONFIG_PATH)
        .exists()
        .then(|| RUNTIME_CONFIG_PATH.into());
    // Configure the application state directory path. This is used in the default
    // locations for logs, key value stores, etc.
    FactorsConfig {
        working_dir: SPIN_TRIGGER_WORKING_DIR.into(),
        runtime_config_file,
        // This is the default base for the state_dir (.spin) unless it is
        // explicitly configured via the runtime config.
        local_app_dir: Some(SPIN_TRIGGER_WORKING_DIR.to_string()),
        // Explicitly do not set log dir in order to force logs to be displayed to stdout.
        // Otherwise, would default to the state directory.
        log_dir: UserProvidedPath::Unset,
        ..Default::default()
    }
}

/// get the supported trigger types from the `LockedApp`.
///
/// this function filters the trigger types to only return the ones that are currently supported.
/// If an unsupported trigger type is found, it returns an error indicating which trigger type is unsupported.
///
/// supported trigger types include:
/// - redis
/// - http
/// - sqs
/// - mqtt
/// - command
///
/// Note: this function returns a `HashSet` of supported trigger types. Duplicates are removed.
pub(crate) fn get_supported_triggers(locked_app: &LockedApp) -> anyhow::Result<HashSet<String>> {
    let supported_triggers: HashSet<&str> = HashSet::from([
        HTTP_TRIGGER_TYPE,
        REDIS_TRIGGER_TYPE,
        SQS_TRIGGER_TYPE,
        COMMAND_TRIGGER_TYPE,
        MQTT_TRIGGER_TYPE,
    ]);

    locked_app.triggers.iter()
        .map(|trigger| {
            let trigger_type = &trigger.trigger_type;
            if !supported_triggers.contains(trigger_type.as_str()) {
                Err(anyhow::anyhow!(
                    "Only Http, Redis, MQTT, SQS, and Command triggers are currently supported. Found unsupported trigger: {trigger_type:?}"
                ))
            } else {
                Ok(trigger_type.clone())
            }
        })
        .collect::<anyhow::Result<HashSet<_>>>()
}
