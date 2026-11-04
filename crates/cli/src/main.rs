//! chaude command line. With no arguments it opens the inline TUI; gen-settings, --version and help are ported.
//! The other commands are not ported yet.

mod mcp_resident;
mod tui;

use std::env;
use std::process;

use chaude_core::api::Config;
use chaude_core::{orch, settings};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "usage: chaude [command] [args]
  (no command)           interactive session (inline TUI; plain line mode when stdin is not a TTY)
  mcp-resident <tcp:addr>  stdio bridge to the resident coder MCP socket (used by the main child's mcp.json)
  gen-settings [windows|mac|linux] [--out FILE] [--src FILE] [--python CMD]   OS-specific .claude/settings.json
  --version, -v          print version
  -h, --help             print this help";

// Go repl.go:1316 + applyOrchEnv: --orch <mode> (default m0) is normalized and exported as CHAUDE_ORCH for
// child processes and hooks. Runs before any thread starts, so the environment write is sound.
fn apply_orch(args: &[String]) -> Result<&'static str, String> {
    let raw = args
        .iter()
        .position(|a| a == "--orch")
        .and_then(|i| args.get(i + 1))
        .map_or("", String::as_str);
    let mode = orch::parse(raw)?;
    // SAFETY: called from main before any other thread is spawned.
    unsafe { env::set_var(orch::ENV_KEY, mode) };
    Ok(mode)
}

// Go repl.go:1316-1334: an orch mode with a default main model (orchDefaultModel) replaces the model when
// --model is not given; otherwise the base model stays. m0 has no default, so it keeps the base.
fn orch_model(mode: &str, model_set: bool, base: &str) -> String {
    let default = orch::default_model(mode);
    if !model_set && !default.is_empty() {
        default.to_string()
    } else {
        base.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orch_model_keeps_given_model() {
        assert_eq!(orch_model("b", true, "sonnet"), "sonnet");
        assert_eq!(orch_model("e", true, "sonnet"), "sonnet");
    }

    #[test]
    fn orch_model_uses_orch_default_when_model_absent() {
        assert_eq!(orch_model("b", false, "sonnet"), "opus");
        assert_eq!(orch_model("e", false, "opus"), "haiku");
    }

    #[test]
    fn orch_model_m0_keeps_base() {
        assert_eq!(orch_model("m0", false, "sonnet"), "sonnet");
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("gen-settings") => settings::gen_settings_main(&args[1..]),
        Some("mcp-resident") => mcp_resident::main(&args[1..]),
        Some("--version" | "-v" | "version") => {
            println!("chaude {VERSION}");
            0
        }
        Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            0
        }
        None | Some("--orch") => match apply_orch(&args) {
            Err(e) => {
                eprintln!("chaude: {e}\n{USAGE}");
                2
            }
            Ok(mode) => {
                let mut cfg = Config::default();
                let model_set = args
                    .iter()
                    .any(|a| a == "--model" || a.starts_with("--model="));
                cfg.model = orch_model(mode, model_set, &cfg.model);
                match tui::run(cfg) {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("chaude: {e}");
                        1
                    }
                }
            }
        },
        Some(cmd) => {
            eprintln!("chaude: {cmd}: not ported to Rust yet\n{USAGE}");
            2
        }
    };
    process::exit(code);
}
