//! chaude command line. With no arguments it opens the inline TUI; gen-settings, --version and help are ported.
//! The other commands are not ported yet.

mod mcp_resident;
mod tui;

use std::env;
use std::path::PathBuf;
use std::process;

use chaude_core::api::Config;
use chaude_core::{metrics_report, orch, settings};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "usage: chaude [command] [args]
  (no command)           interactive session (inline TUI; plain line mode when stdin is not a TTY)
  mcp-resident <tcp:addr>  stdio bridge to the resident coder MCP socket (used by the main child's mcp.json)
  gen-settings [windows|mac|linux] [--out FILE] [--src FILE] [--python CMD]   OS-specific .claude/settings.json
  metrics [--dir D] [--csv]  summary of the per-turn metrics (or CSV rows); --dir defaults to <record>/metrics
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

// Go repl.go:1316-1334: --model <id> (Go flag -model) wins; otherwise an orch mode with a default main model
// (orchDefaultModel) replaces the settings model; otherwise the project settings main_model is used.
fn resolve_model(mode: &str, flag: Option<String>, settings_model: &str) -> String {
    if let Some(m) = flag {
        return m;
    }
    let default = orch::default_model(mode);
    if default.is_empty() {
        settings_model
    } else {
        default
    }
    .to_string()
}

// --model <v> or --model=<v>. A missing value is an error (Go flag package behavior).
fn parse_model(args: &[String]) -> Result<Option<String>, String> {
    for (i, a) in args.iter().enumerate() {
        if let Some(v) = a.strip_prefix("--model=") {
            return Ok(Some(v.to_string()));
        }
        if a == "--model" {
            return args
                .get(i + 1)
                .cloned()
                .map(Some)
                .ok_or_else(|| "--model needs a value".to_string());
        }
    }
    Ok(None)
}

// The interactive session: --orch and --model are applied, then the TUI runs. Returns the exit code.
fn run_tui(args: &[String]) -> i32 {
    let mode = match apply_orch(args) {
        Ok(mode) => mode,
        Err(e) => {
            eprintln!("chaude: {e}\n{USAGE}");
            return 2;
        }
    };
    let flag = match parse_model(args) {
        Ok(flag) => flag,
        Err(e) => {
            eprintln!("chaude: {e}\n{USAGE}");
            return 2;
        }
    };
    let mut cfg = Config::default();
    cfg.model = resolve_model(
        mode,
        flag,
        settings::chaude_settings::main_model(cfg.cwd.as_deref()),
    );
    match tui::run(cfg) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("chaude: {e}");
            1
        }
    }
}

// Go MetricsMain: `chaude metrics [--dir D] [--csv]`. Prints the summary, or the CSV rows with --csv.
fn metrics_main(args: &[String]) -> i32 {
    let mut dir = "auto".to_string();
    let mut csv = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--csv" => csv = true,
            "--dir" => match it.next() {
                Some(v) => dir = v.clone(),
                None => {
                    eprintln!("chaude metrics: --dir needs a value\n{USAGE}");
                    return 2;
                }
            },
            _ if a.starts_with("--dir=") => dir = a["--dir=".len()..].to_string(),
            _ => {
                eprintln!("chaude metrics: unknown argument {a}\n{USAGE}");
                return 2;
            }
        }
    }
    let dir = if dir == "auto" {
        let cwd = env::current_dir().unwrap_or_else(|_| ".".into());
        chaude_core::record::dir_for(&cwd).join("metrics")
    } else {
        PathBuf::from(dir)
    };
    if csv {
        print!("{}", metrics_report::csv(&dir));
    } else {
        print!("{}", metrics_report::summary(&dir));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn model_flag_given_wins_over_orch_and_settings() {
        assert_eq!(
            resolve_model("b", Some("sonnet".into()), "claude-fable-5-1"),
            "sonnet"
        );
        assert_eq!(
            resolve_model("e", Some("opus".into()), "claude-sonnet-5-5"),
            "opus"
        );
    }

    #[test]
    fn model_flag_absent_orch_default_overrides_settings() {
        assert_eq!(resolve_model("b", None, "claude-sonnet-5-5"), "opus");
        assert_eq!(resolve_model("e", None, "claude-opus-5-5"), "haiku");
    }

    #[test]
    fn model_flag_absent_m0_uses_settings_value() {
        assert_eq!(
            resolve_model("m0", None, "claude-sonnet-5-5"),
            "claude-sonnet-5-5"
        );
        assert_eq!(
            resolve_model("m0", None, "claude-opus-5-5"),
            "claude-opus-5-5"
        );
    }

    #[test]
    fn parse_model_forms() {
        assert_eq!(parse_model(&strs(&["--orch", "b"])), Ok(None));
        assert_eq!(
            parse_model(&strs(&["--model", "sonnet"])),
            Ok(Some("sonnet".into()))
        );
        assert_eq!(
            parse_model(&strs(&["--model=fable"])),
            Ok(Some("fable".into()))
        );
        assert!(parse_model(&strs(&["--model"])).is_err());
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
        None | Some("--orch" | "--model") => run_tui(&args),
        Some(a) if a.starts_with("--model=") => run_tui(&args),
        Some("metrics") => metrics_main(&args[1..]),
        Some(cmd) => {
            eprintln!("chaude: {cmd}: not ported to Rust yet\n{USAGE}");
            2
        }
    };
    process::exit(code);
}
