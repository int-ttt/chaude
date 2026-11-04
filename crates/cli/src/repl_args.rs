//! Go repl.go Main flag set (`chaude repl [flags]`, and any leading -flag). Parsed like Go lean.ParseInterspersed:
//! flags may appear anywhere, `-x` and `--x` are both accepted, `--x=v` and `--x v` both work, `--` ends the flags.
//! Defaults and value checks follow Go; the values are applied to chaude_core::api::Config.

use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

use chaude_core::api::{Config, resolve_max_workers, worker_floor};
use chaude_core::{osenv, record};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Str,
    Int,
    Float,
    Bool,
    Dur,
}

// Every Go repl flag. The kind drives parsing only; defaults are applied in apply().
const FLAGS: &[(&str, Kind)] = &[
    ("model", Kind::Str),
    ("cwd", Kind::Str),
    ("threshold", Kind::Int),
    ("compact-window", Kind::Int),
    ("max-auto-rotations", Kind::Int),
    ("transcript", Kind::Str),
    ("tail-turns", Kind::Int),
    ("snapshot-chars", Kind::Int),
    ("auto-allow", Kind::Bool),
    ("show-rotation", Kind::Bool),
    ("metrics-dir", Kind::Str),
    ("no-handoff-validate", Kind::Bool),
    ("max-drain", Kind::Int),
    ("drain-timeout", Kind::Dur),
    ("rotate-defer-timeout", Kind::Dur),
    ("rotate-cooldown", Kind::Dur),
    ("no-category-rotate", Kind::Bool),
    ("no-delegation-rule", Kind::Bool),
    ("agents-file", Kind::Str),
    ("from-claude", Kind::Str),
    ("import-dry-run", Kind::Bool),
    ("cache-ttl", Kind::Str),
    ("task-budget-usd", Kind::Float),
    ("idle-rotate", Kind::Dur),
    ("idle-notice", Kind::Dur),
    ("clients-file", Kind::Str),
    ("max-workers", Kind::Int),
    ("orch", Kind::Str),
];

// Go defaults that are not plain Config defaults (repl.go flag definitions).
const GO_THRESHOLD: u64 = 80_000;
const GO_COMPACT_WINDOW: u32 = 400_000;
const GO_TAIL_TURNS: usize = 4;
const GO_SNAPSHOT_CHARS: usize = 24_000;
const GO_IDLE_NOTICE: Duration = Duration::from_secs(30 * 60);

/// Raw flag values as typed (bools are stored as "true"/"false"). Flags not typed are absent.
pub struct Parsed {
    raw: BTreeMap<&'static str, String>,
    help: bool,
}

/// Parses the flag words. Value checks happen in `apply`. Err is a Go-style flag error (exit 2).
pub fn parse(args: &[String]) -> Result<Parsed, String> {
    let mut raw = BTreeMap::new();
    let mut help = false;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == "--" {
            break;
        }
        // Positionals are ignored, as Go's repl ignores them.
        let Some(body) = a.strip_prefix('-') else {
            continue;
        };
        let body = body.strip_prefix('-').unwrap_or(body);
        if body.is_empty() {
            continue;
        }
        if body == "h" || body == "help" {
            help = true;
            continue;
        }
        let (name, inline) = match body.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (body, None),
        };
        let Some(&(fname, kind)) = FLAGS.iter().find(|(n, _)| *n == name) else {
            return Err(format!("flag provided but not defined: -{name}"));
        };
        let value = match (kind, inline) {
            (Kind::Bool, None) => "true".to_string(),
            (Kind::Bool, Some(v)) => match parse_bool(&v) {
                Some(b) => b.to_string(),
                None => return Err(format!("invalid boolean value {v:?} for flag -{name}")),
            },
            (_, Some(v)) => v,
            (_, None) => {
                let v = args
                    .get(i)
                    .cloned()
                    .ok_or_else(|| format!("flag needs an argument: -{name}"))?;
                i += 1;
                v
            }
        };
        raw.insert(fname, value);
    }
    Ok(Parsed { raw, help })
}

impl Parsed {
    pub fn help(&self) -> bool {
        self.help
    }

    /// --model; None when absent (the caller resolves orch and settings defaults).
    pub fn model(&self) -> Option<String> {
        self.raw.get("model").cloned()
    }

    /// --orch raw value; "" when absent (orch::parse maps "" to the default mode).
    pub fn orch(&self) -> &str {
        self.raw.get("orch").map_or("", String::as_str)
    }

    /// --from-claude value (session id, "latest", or a transcript path).
    pub fn claude_source(&self) -> Option<String> {
        self.raw.get("from-claude").cloned()
    }

    pub fn import_dry_run(&self) -> bool {
        self.raw.get("import-dry-run").is_some_and(|v| v == "true")
    }

    /// Sets every Config field from the flags, with Go defaults for the absent ones. `env_workers` is the
    /// CHAUDE_MAX_WORKERS value ("" when unset).
    pub fn apply(&self, cfg: &mut Config, env_workers: &str) -> Result<(), String> {
        let cwd = match self.raw.get("cwd") {
            Some(v) if !v.is_empty() => Some(PathBuf::from(v)),
            _ => None,
        };
        let state = osenv::state_dir();
        let metrics_base = cwd
            .clone()
            .or_else(|| env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        cfg.cwd = cwd;

        cfg.rotate_threshold = self.num("threshold", GO_THRESHOLD)?;
        cfg.compact_window = self.num("compact-window", GO_COMPACT_WINDOW)?;
        cfg.max_auto_rotations = self.num("max-auto-rotations", -1i64)?;
        cfg.transcript = self.path_or("transcript", Some(state.join("transcript.jsonl")));
        cfg.tail_turns = self.num("tail-turns", GO_TAIL_TURNS)?;
        cfg.snapshot_chars = self.num("snapshot-chars", GO_SNAPSHOT_CHARS)?;
        cfg.auto_allow = self.flag("auto-allow");
        cfg.show_rotation = self.flag("show-rotation");
        cfg.metrics_dir = match self.raw.get("metrics-dir").map(String::as_str) {
            None | Some("auto") => Some(record::dir_for(&metrics_base).join("metrics")),
            Some("") => None,
            Some(p) => Some(PathBuf::from(p)),
        };
        cfg.no_handoff_validate = self.flag("no-handoff-validate");
        cfg.max_drain = self.num("max-drain", 3usize)?;
        cfg.drain_timeout = self.dur("drain-timeout", Duration::from_secs(60 * 60))?;
        cfg.rotate_defer_timeout =
            self.dur("rotate-defer-timeout", Duration::from_secs(10 * 60))?;
        cfg.rotate_cooldown = self.dur("rotate-cooldown", Duration::from_secs(2 * 60))?;
        cfg.no_category_rotate = self.flag("no-category-rotate");
        cfg.no_delegation_rule = self.flag("no-delegation-rule");
        cfg.agents_file = self.path_or("agents-file", Some(state.join("agents.json")));
        cfg.cache_ttl = match self.raw.get("cache-ttl") {
            Some(v) if matches!(v.as_str(), "auto" | "1h" | "5m") => v.clone(),
            Some(v) => return Err(format!("--cache-ttl must be auto, 1h or 5m (got {v:?})")),
            None => "1h".to_string(),
        };
        cfg.task_budget_usd = match self.raw.get("task-budget-usd") {
            Some(v) => v.parse::<f64>().map_err(|_| bad("task-budget-usd", v))?,
            None => 0.0,
        };
        cfg.idle_rotate = self.dur("idle-rotate", Duration::from_secs(3599))?;
        cfg.inactivity_timeout = self.dur("idle-notice", GO_IDLE_NOTICE)?;
        cfg.clients_file = self.path_or("clients-file", Some(state.join("clients.json")));

        let flag = match self.int("max-workers")? {
            Some(n) if n < 1 => return Err(format!("max-workers must be >= 1 (got {n})")),
            Some(n) => Some(usize::try_from(n).map_err(|_| bad("max-workers", &n.to_string()))?),
            None => None,
        };
        cfg.max_workers = resolve_max_workers(flag, env_workers, cfg.extra_coders)?;
        // Go repl.go: the resident floor applies only when neither --max-workers nor CHAUDE_MAX_WORKERS is set.
        if flag.is_none() && env_workers.trim().is_empty() {
            cfg.max_workers = worker_floor(cfg.max_workers, cfg.extra_coders);
        }
        Ok(())
    }

    fn flag(&self, name: &str) -> bool {
        self.raw.get(name).is_some_and(|v| v == "true")
    }

    fn int(&self, name: &str) -> Result<Option<i64>, String> {
        self.raw
            .get(name)
            .map(|v| v.parse::<i64>().map_err(|_| bad(name, v)))
            .transpose()
    }

    fn num<T: TryFrom<i64>>(&self, name: &str, default: T) -> Result<T, String> {
        match self.int(name)? {
            Some(n) => T::try_from(n).map_err(|_| bad(name, &n.to_string())),
            None => Ok(default),
        }
    }

    fn dur(&self, name: &str, default: Duration) -> Result<Duration, String> {
        match self.raw.get(name) {
            Some(v) => parse_duration(v).ok_or_else(|| bad(name, v)),
            None => Ok(default),
        }
    }

    // "" turns the feature off (None); an absent flag uses the default.
    fn path_or(&self, name: &str, default: Option<PathBuf>) -> Option<PathBuf> {
        match self.raw.get(name) {
            Some(v) if v.is_empty() => None,
            Some(v) => Some(PathBuf::from(v)),
            None => default,
        }
    }
}

// Go flag package message for a bad value.
fn bad(name: &str, value: &str) -> String {
    format!("invalid value {value:?} for flag -{name}: parse error")
}

// strconv.ParseBool spellings.
fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => Some(true),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => Some(false),
        _ => None,
    }
}

// Go time.ParseDuration for the forms the repl flags take: "0", "90s", "60m", "1h30m", "1.5h".
fn parse_duration(s: &str) -> Option<Duration> {
    if s == "0" {
        return Some(Duration::ZERO);
    }
    if s.is_empty() {
        return None;
    }
    let mut total = 0f64;
    let mut rest = s;
    while !rest.is_empty() {
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(rest.len());
        let num: f64 = rest[..end].parse().ok()?;
        rest = &rest[end..];
        let unit_end = rest
            .find(|c: char| c.is_ascii_digit() || c == '.')
            .unwrap_or(rest.len());
        let secs = match &rest[..unit_end] {
            "ns" => 1e-9,
            "us" | "µs" => 1e-6,
            "ms" => 1e-3,
            "s" => 1.0,
            "m" => 60.0,
            "h" => 3600.0,
            _ => return None,
        };
        total += num * secs;
        rest = &rest[unit_end..];
    }
    Duration::try_from_secs_f64(total).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn cfg_for(args: &[&str], env_workers: &str) -> Result<Config, String> {
        let p = parse(&strs(args))?;
        let mut cfg = Config::default();
        p.apply(&mut cfg, env_workers)?;
        Ok(cfg)
    }

    #[test]
    fn defaults_match_go() {
        let cfg = cfg_for(&[], "").unwrap();
        assert_eq!(cfg.rotate_threshold, 80_000);
        assert_eq!(cfg.compact_window, 400_000);
        assert_eq!(cfg.max_auto_rotations, -1);
        assert_eq!(cfg.tail_turns, 4);
        assert_eq!(cfg.snapshot_chars, 24_000);
        assert!(!cfg.auto_allow && !cfg.show_rotation && !cfg.no_handoff_validate);
        assert!(
            cfg.metrics_dir.is_some(),
            "metrics-dir auto = <record>/metrics"
        );
        assert_eq!(cfg.max_drain, 3);
        assert_eq!(cfg.drain_timeout, Duration::from_secs(3600));
        assert_eq!(cfg.rotate_defer_timeout, Duration::from_secs(600));
        assert_eq!(cfg.rotate_cooldown, Duration::from_secs(120));
        assert!(!cfg.no_category_rotate && !cfg.no_delegation_rule);
        assert_eq!(cfg.cache_ttl, "1h");
        assert_eq!(cfg.task_budget_usd, 0.0);
        assert_eq!(cfg.idle_rotate, Duration::from_secs(3599));
        assert_eq!(cfg.inactivity_timeout, Duration::from_secs(1800));
        assert!(
            cfg.transcript.is_some() && cfg.agents_file.is_some() && cfg.clients_file.is_some()
        );
        // Go repl.go floor: with neither --max-workers nor CHAUDE_MAX_WORKERS set, the cap is at least
        // residentMaxWorkers (8) + extra coders (Config default 4) = 12.
        assert_eq!(cfg.max_workers, 8 + cfg.extra_coders);
    }

    #[test]
    fn model_and_import_flags_are_read_back() {
        let p = parse(&strs(&[
            "--model=sonnet",
            "--from-claude",
            "latest",
            "--import-dry-run",
        ]))
        .unwrap();
        assert_eq!(p.model().as_deref(), Some("sonnet"));
        assert_eq!(p.claude_source().as_deref(), Some("latest"));
        assert!(p.import_dry_run());
        assert_eq!(parse(&strs(&["--orch", "b"])).unwrap().orch(), "b");
    }

    #[test]
    fn numeric_flags_apply_in_both_forms() {
        let cfg = cfg_for(
            &[
                "--threshold=90000",
                "--compact-window",
                "0",
                "--max-auto-rotations=2",
                "-tail-turns=6",
                "--snapshot-chars",
                "1000",
            ],
            "",
        )
        .unwrap();
        assert_eq!(cfg.rotate_threshold, 90_000);
        assert_eq!(cfg.compact_window, 0);
        assert_eq!(cfg.max_auto_rotations, 2);
        assert_eq!(cfg.tail_turns, 6);
        assert_eq!(cfg.snapshot_chars, 1000);
    }

    #[test]
    fn bool_flags_take_bare_and_explicit_values() {
        let cfg = cfg_for(
            &[
                "--auto-allow",
                "--show-rotation",
                "--no-handoff-validate=true",
                "--no-delegation-rule=false",
            ],
            "",
        )
        .unwrap();
        assert!(cfg.auto_allow && cfg.show_rotation && cfg.no_handoff_validate);
        assert!(!cfg.no_delegation_rule);
        assert!(cfg_for(&["--auto-allow=maybe"], "").is_err());
    }

    #[test]
    fn duration_flags_and_drain_limits() {
        let cfg = cfg_for(
            &[
                "--drain-timeout=90s",
                "--rotate-defer-timeout",
                "0",
                "--rotate-cooldown=1h30m",
                "--idle-rotate=5m",
                "--idle-notice=2m",
                "--max-drain=0",
            ],
            "",
        )
        .unwrap();
        assert_eq!(cfg.drain_timeout, Duration::from_secs(90));
        assert_eq!(cfg.rotate_defer_timeout, Duration::ZERO);
        assert_eq!(cfg.rotate_cooldown, Duration::from_secs(5400));
        assert_eq!(cfg.idle_rotate, Duration::from_secs(300));
        assert_eq!(cfg.inactivity_timeout, Duration::from_secs(120));
        assert_eq!(cfg.max_drain, 0);
        assert!(cfg_for(&["--drain-timeout=soon"], "").is_err());
    }

    #[test]
    fn empty_path_flags_turn_features_off() {
        let cfg = cfg_for(
            &[
                "--transcript=",
                "--agents-file=",
                "--clients-file=",
                "--metrics-dir=",
            ],
            "",
        )
        .unwrap();
        assert!(cfg.transcript.is_none() && cfg.agents_file.is_none());
        assert!(cfg.clients_file.is_none() && cfg.metrics_dir.is_none());
        let cfg = cfg_for(&["--transcript=/tmp/t.jsonl", "--metrics-dir=/tmp/m"], "").unwrap();
        assert_eq!(cfg.transcript, Some(PathBuf::from("/tmp/t.jsonl")));
        assert_eq!(cfg.metrics_dir, Some(PathBuf::from("/tmp/m")));
    }

    #[test]
    fn cwd_and_cache_ttl_and_budget() {
        let cfg = cfg_for(
            &["--cwd=/tmp/proj", "--cache-ttl=5m", "--task-budget-usd=2.5"],
            "",
        )
        .unwrap();
        assert_eq!(cfg.cwd, Some(PathBuf::from("/tmp/proj")));
        assert_eq!(cfg.cache_ttl, "5m");
        assert_eq!(cfg.task_budget_usd, 2.5);
        let err = cfg_for(&["--cache-ttl=2h"], "").unwrap_err();
        assert_eq!(err, "--cache-ttl must be auto, 1h or 5m (got \"2h\")");
    }

    #[test]
    fn max_workers_flag_then_env_then_default() {
        assert_eq!(cfg_for(&["--max-workers=5"], "9").unwrap().max_workers, 5);
        assert_eq!(cfg_for(&[], "5").unwrap().max_workers, 5);
        assert!(cfg_for(&["--max-workers=0"], "").is_err());
        assert!(cfg_for(&[], "0").is_err());
        assert!(cfg_for(&[], "lots").is_err());
    }

    #[test]
    fn unknown_flag_and_missing_value_are_errors() {
        assert_eq!(
            parse(&strs(&["--nope"])).err().as_deref(),
            Some("flag provided but not defined: -nope")
        );
        assert_eq!(
            parse(&strs(&["--model"])).err().as_deref(),
            Some("flag needs an argument: -model")
        );
        assert!(cfg_for(&["--threshold=high"], "").is_err());
    }

    #[test]
    fn help_and_terminator() {
        assert!(parse(&strs(&["-h"])).unwrap().help());
        assert!(parse(&strs(&["--help"])).unwrap().help());
        // Words after -- are positionals, not flags.
        assert!(parse(&strs(&["--", "--nope"])).is_ok());
        assert!(
            parse(&strs(&["extra", "--auto-allow"]))
                .unwrap()
                .flag("auto-allow")
        );
    }

    #[test]
    fn go_duration_forms() {
        assert_eq!(parse_duration("0"), Some(Duration::ZERO));
        assert_eq!(parse_duration("90s"), Some(Duration::from_secs(90)));
        assert_eq!(parse_duration("1h30m"), Some(Duration::from_secs(5400)));
        assert_eq!(parse_duration("1.5h"), Some(Duration::from_secs(5400)));
        assert_eq!(parse_duration("250ms"), Some(Duration::from_millis(250)));
        assert_eq!(parse_duration("10"), None);
        assert_eq!(parse_duration("-1s"), None);
        assert_eq!(parse_duration(""), None);
    }
}
