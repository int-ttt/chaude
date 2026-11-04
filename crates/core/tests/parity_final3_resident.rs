// Engine-level bodies for the resident MCP rows of plan/rust-test-parity.md (final3 workstream).
// Go sources: harness/resident_agent_test.go (TestMainStrictMCP, spawn-once body) and
// harness/resident_coder_test.go (TestResidentMCPConfigWritten, created-once body).
// A fake claude script logs every child argv to args.log. Only the main child gets --mcp-config
// (api.rs mcp_config), so the logged lines with that flag are the main spawns.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc::Receiver;
use std::sync::{Mutex, MutexGuard, Once};
use std::thread;
use std::time::{Duration, Instant};

use chaude_core::api::{Config, Engine, UiEvent};

// Longest wait for one engine event or file write. Same bound as engine.rs, for loaded machines.
const WAIT: Duration = Duration::from_secs(60);

// Engine tests share the record dir env var, so they run one at a time.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// Empty directory under the system temp dir, unique per process and tag.
fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chaude-final3-res-{}-{tag}", process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

// Fake claude: logs its argv, then answers every user line with one text block and a result.
// Returns the script path and the test dir.
fn fake_claude(tag: &str) -> (PathBuf, PathBuf) {
    static RECORD: Once = Once::new();
    RECORD.call_once(|| {
        // Turn records go under a test dir, never the repo record/ folder.
        // SAFETY: runs once, before any engine thread of this test binary starts reading the env.
        unsafe { std::env::set_var("HARNESS_RECORD_DIR", fresh_dir("record")) };
    });
    let dir = fresh_dir(tag);
    let turn = dir.join("turn.jsonl");
    fs::write(
        &turn,
        "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"ok\"}],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\
         {\"type\":\"result\",\"session_id\":\"s1\"}\n",
    )
    .expect("write turn");
    let log = dir.join("args.log");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{log}'\nwhile IFS= read -r line; do\n  case \"$line\" in\n    *'\"type\":\"user\"'*) cat '{turn}' ;;\n  esac\ndone\n",
        log = log.display(),
        turn = turn.display(),
    );
    let bin = dir.join("claude");
    fs::write(&bin, script).expect("write fake claude");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod fake claude");
    (bin, dir)
}

fn engine_cfg(bin: &Path, dir: &Path) -> Config {
    Config {
        claude_bin: bin.display().to_string(),
        model: "haiku".into(),
        residents: true,
        agents_file: Some(dir.join("agents.json")),
        clients_file: Some(dir.join("clients.json")),
        ..Config::default()
    }
}

// Collects events up to and including Busy(false).
fn until_idle(rx: &Receiver<UiEvent>) {
    loop {
        if rx.recv_timeout(WAIT).expect("engine went silent") == UiEvent::Busy(false) {
            return;
        }
    }
}

// Argv lines of the main children: every logged line that carries --mcp-config.
fn main_spawns(log: &Path) -> Vec<String> {
    fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|l| l.contains("--mcp-config"))
        .map(str::to_string)
        .collect()
}

// Polls args.log until n main spawns are logged (or WAIT passes).
fn wait_spawns(log: &Path, n: usize) -> Vec<String> {
    let t0 = Instant::now();
    loop {
        let spawns = main_spawns(log);
        if spawns.len() >= n || t0.elapsed() > WAIT {
            return spawns;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

// The value after --mcp-config in one argv line.
fn mcp_path(line: &str) -> String {
    let mut it = line.split_whitespace();
    while let Some(tok) = it.next() {
        if tok == "--mcp-config" {
            return it.next().unwrap_or_default().to_string();
        }
    }
    String::new()
}

// Go TestMainStrictMCP (spawn-once body): the haiku main child starts once with --mcp-config and
// --strict-mcp-config, keeps slash commands, and a full turn does not start it again.
#[test]
fn main_spawned_once_with_strict_mcp() {
    let _g = lock();
    let (bin, dir) = fake_claude("main-once");
    let log = dir.join("args.log");
    let (h, rx) = Engine::start(engine_cfg(&bin, &dir));
    let spawns = wait_spawns(&log, 1);
    assert_eq!(
        spawns.len(),
        1,
        "main spawned with --mcp-config before turn: {spawns:?}"
    );
    h.submit("hello");
    until_idle(&rx);
    let spawns = wait_spawns(&log, 1);
    assert_eq!(
        spawns.len(),
        1,
        "main spawned with --mcp-config after turn: {spawns:?}"
    );
    let args = &spawns[0];
    assert!(
        args.contains("--strict-mcp-config"),
        "main missing --strict-mcp-config: {args}"
    );
    assert!(
        !args.contains("--disable-slash-commands"),
        "main must keep slash commands: {args}"
    );
    h.shutdown();
}

// Go TestResidentMCPConfigWritten (created-once body): a respawned main child (idle rotation hands
// off to a fresh child) gets the same mcp.json path, so the config is written once per engine.
#[test]
fn mcp_config_path_reused_after_respawn() {
    let _g = lock();
    let (bin, dir) = fake_claude("mcp-once");
    let log = dir.join("args.log");
    let mut cfg = engine_cfg(&bin, &dir);
    cfg.idle_rotate = Duration::from_secs(1);
    let (h, rx) = Engine::start(cfg);
    assert_eq!(wait_spawns(&log, 1).len(), 1, "no first main spawn");
    h.submit("first");
    until_idle(&rx);
    // No further prompt: the child sits idle past the 1 s idle rotation and hands off on its own.
    let t0 = Instant::now();
    loop {
        if let UiEvent::Rotated { .. } = rx.recv_timeout(WAIT).expect("engine went silent") {
            break;
        }
        assert!(t0.elapsed() < WAIT, "no idle rotation");
    }
    let spawns = wait_spawns(&log, 2);
    assert_eq!(
        spawns.len(),
        2,
        "main spawns with --mcp-config after rotation: {spawns:?}"
    );
    let first = mcp_path(&spawns[0]);
    assert!(!first.is_empty(), "no --mcp-config path: {}", spawns[0]);
    assert_eq!(mcp_path(&spawns[1]), first, "mcp.json must be created once");
    h.shutdown();
}
