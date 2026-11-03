//! chaude command line. With no arguments it opens the inline TUI; gen-settings, --version and help are ported.
//! The other commands are not ported yet.

mod tui;

use std::env;
use std::process;

use chaude_core::api::Config;
use chaude_core::settings;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "usage: chaude [command] [args]
  (no command)           interactive session (inline TUI; plain line mode when stdin is not a TTY)
  gen-settings [windows|mac|linux] [--out FILE] [--src FILE] [--python CMD]   OS-specific .claude/settings.json
  --version, -v          print version
  -h, --help             print this help";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("gen-settings") => settings::gen_settings_main(&args[1..]),
        Some("--version" | "-v" | "version") => {
            println!("chaude {VERSION}");
            0
        }
        Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            0
        }
        Some(cmd) => {
            eprintln!("chaude: {cmd}: not ported to Rust yet\n{USAGE}");
            2
        }
        None => match tui::run(Config::default()) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("chaude: {e}");
                1
            }
        },
    };
    process::exit(code);
}
