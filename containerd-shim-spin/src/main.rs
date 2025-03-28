use containerd_shim_wasm::shim::{Cli, Config};
use engine::SpinShim;

mod constants;
mod engine;
mod source;
mod trigger;
mod utils;

fn main() {
    // Configure the shim to have only error level logging for performance improvements.
    let shim_config = Config {
        default_log_level: "error".to_string(),
        ..Default::default()
    };
    SpinShim::run(shim_config);
}
