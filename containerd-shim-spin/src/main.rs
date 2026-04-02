use containerd_shim_wasm::shim::{Cli, Config};
use engine::SpinShim;

mod constants;
mod engine;
mod source;
mod trigger;
mod utils;

fn main() {
    let shim_config = Config {
        default_log_level: "info".to_string(),
        ..Default::default()
    };
    SpinShim::run(shim_config);
}
