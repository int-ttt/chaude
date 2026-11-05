//! chaude command line. With no arguments it opens the inline TUI; gen-settings, --version and help are ported.
//! The other commands are not ported yet.

use chaude_cli::{mcp_resident, repl_args, tui};

use std::env;
use std::path::{Path, PathBuf};
use std::process;

use chaude_core::api::Config;
use chaude_core::import_claude::{self, ClaudeImport};
use chaude_core::rotate;
use chaude_core::{metrics_report, orch, settings};

const VERSION: &str = env!("CARGO_PKG_VERSION");

// Go cmd/chaude/main.go usage, verbatim. The helper-script list is scripts.Names (crates/scripts NAMES).
const USAGE: &str = "usage: chaude <command> [args]
  repl [flags]           terminal REPL over claude -p stream-json with session rotation (default with no command)
  metrics [--dir D] [--csv]   summarize per-turn metrics (record/metrics)
  --from-claude ID|latest|PATH [--import-dry-run]   start a repl from a plain Claude Code session
  --version              print version
  gen-settings <os>      print/write OS-specific .claude/settings.json (mac|linux|windows) [--out FILE]
  clean [dir] [--yes]    remove the claude-lean harness from a project (dry-run; --yes backs up then removes)
  update                 check GitHub for a newer release and replace this binary (same as /update in the REPL)
  charcount | claude-history | compress-archive | export-history | extract-transcript | search-history | usage-report   helper scripts (run with -h for flags)";

// Go repl.go:1316 + applyOrchEnv: --orch <mode> (default m0) is normalized and exported as CHAUDE_ORCH for
// child processes and hooks. Runs before any thread starts, so the environment write is sound.
fn apply_orch(raw: &str) -> Result<&'static str, String> {
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

// Go importFromClaude dry run: prints the report and returns 0 when the recent handoff note survives the
// snapshot the next prompt would start from, 1 otherwise.
fn import_dry_run(path: &Path, imp: &ClaudeImport) -> i32 {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let note = import_claude::import_note(stem);
    let snap = rotate::with_reco(
        &imp.reco,
        &import_claude::import_snapshot(
            imp,
            &note,
            import_claude::DEFAULT_TAIL_TURNS,
            import_claude::DEFAULT_SNAPSHOT_CHARS,
        ),
    );
    let preserved = rotate::reco_preserved(&imp.reco, &snap);
    print!(
        "{}",
        import_claude::dry_run_report(imp, path, &snap, preserved)
    );
    if preserved { 0 } else { 1 }
}

// The interactive session: --orch and --model are applied, then the TUI runs. Returns the exit code.
fn run_tui(args: &[String]) -> i32 {
    let parsed = match repl_args::parse(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("chaude: {e}\n{USAGE}");
            return 2;
        }
    };
    if parsed.help() {
        println!("{USAGE}");
        return 0;
    }
    let mode = match apply_orch(parsed.orch()) {
        Ok(mode) => mode,
        Err(e) => {
            eprintln!("chaude: {e}\n{USAGE}");
            return 2;
        }
    };
    // Go repl.go: CHAUDE_MAX_WORKERS is read only when --max-workers is absent (resolved in apply).
    let env_workers = env::var("CHAUDE_MAX_WORKERS").unwrap_or_default();
    let mut cfg = Config::default();
    if let Err(e) = parsed.apply(&mut cfg, &env_workers) {
        eprintln!("chaude: {e}\n{USAGE}");
        return 2;
    }
    cfg.model = resolve_model(
        mode,
        parsed.model(),
        settings::chaude_settings::main_model(cfg.cwd.as_deref()),
    );
    let cwd = cfg
        .cwd
        .clone()
        .or_else(|| env::current_dir().ok())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| ".".into());
    let startup = match parsed.claude_source() {
        None => None,
        Some(arg) => match import_claude::load_import_arg(&arg, &cwd) {
            Ok(loaded) => Some(loaded),
            Err(e) => {
                eprintln!("chaude: import failed: {e}");
                return 1;
            }
        },
    };
    if parsed.import_dry_run() {
        return match &startup {
            Some((path, imp)) => import_dry_run(path, imp),
            None => {
                eprintln!("chaude: --import-dry-run needs --from-claude\n{USAGE}");
                2
            }
        };
    }
    // Go repl.go: project settings are ensured at startup; a failure is reported and startup continues.
    let home = env::var_os("HOME").map(PathBuf::from);
    if let Err(e) = settings::project::ensure_project_settings(Path::new(&cwd), home.as_deref()) {
        eprintln!("project settings: {e}");
    }
    // Go repl.go:1357: the startup prune of old records uses the loaded SessionRetentionDays.
    let set = tui::settings::load(tui::settings::file_for(Some(Path::new(&cwd))).as_deref());
    tui::settings::prune_records_at_startup(Path::new(&cwd), &set, std::time::SystemTime::now());
    match tui::run(cfg, startup.map(|(_, imp)| imp)) {
        Ok(()) => 0,
        Err(e) => {
            // Go tui.go:1408-1412: a program error prints "harness: <err>" and exits 1.
            eprintln!("harness: {e}");
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

fn main() {
    // Go main.go: update.CleanupOld() runs before dispatch.
    chaude_update::cleanup_old();
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("gen-settings") => settings::gen_settings_main(&args[1..]),
        Some("mcp-resident") => mcp_resident::main(&args[1..]),
        // Go main.go: `repl` runs the session with the flags after it.
        Some("repl") => run_tui(&args[1..]),
        Some("--version" | "-v" | "version") => {
            println!("chaude {VERSION}");
            0
        }
        Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            0
        }
        // Go main.go default branch: any leading -flag runs the repl (flags are parsed by run_tui).
        None => run_tui(&args),
        Some(a) if a.starts_with('-') => run_tui(&args),
        Some("metrics") => metrics_main(&args[1..]),
        Some("clean") => chaude_clean::main(&args[1..]),
        Some("update") => chaude_update::run_cli(VERSION, &args[1..]),
        Some(cmd) if chaude_scripts::has(cmd) => chaude_scripts::run(cmd, &args[1..]),
        Some(cmd) => {
            eprintln!("chaude: {cmd}: not ported to Rust yet\n{USAGE}");
            2
        }
    };
    process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn usage_lists_the_helper_scripts() {
        assert!(USAGE.contains(chaude_scripts::NAMES));
    }
}
