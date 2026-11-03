//! Launcher. Everything lives in the library crate; see [`animation::app`].
//!
//! Command line (native):
//!
//! - `--screenshot <file.png>`: save what the window shows after `--at <secs>` seconds
//!   (default 2), then quit.

use animation::app::{App, config, screenshot_after};
use game_toolkit::prelude::*;

/// Parses the command line, sets up logging (native only) and hands the game to the
/// toolkit's run loop.
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--screenshot") {
        let Some(path) = args.get(i + 1) else {
            eprintln!("--screenshot needs a file name");
            std::process::exit(2);
        };
        let at = match args.iter().position(|a| a == "--at") {
            Some(k) => args
                .get(k + 1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| {
                    eprintln!("--at needs seconds");
                    std::process::exit(2);
                }),
            None => 2.0,
        };
        screenshot_after(path, at);
    }
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    run::<App>(config())
}
