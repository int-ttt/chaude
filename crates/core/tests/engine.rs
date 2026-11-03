#![cfg(unix)]
// Engine tests against a fake `claude` shell script (Config::claude_bin override).
// The script logs each stdin line to <dir>/stdin.log and, on the Nth user frame, replays <dir>/turnN.jsonl.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc::Receiver;
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chaude_core::api::{Config, Engine, InterruptSource, Role, UiEvent};

// Scripts are written and exec'd per test; serializing tests avoids ETXTBSY races.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("chaude-e1-{name}-{}-{nanos}", process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct Fake {
    dir: PathBuf,
    bin: String,
}

// Each user frame n sleeps 0.5 s, then prints turn{n}.jsonl. The sleep makes queueing deterministic.
fn fake(name: &str, turns: &[&str]) -> Fake {
    let dir = temp_dir(name);
    for (i, f) in turns.iter().enumerate() {
        fs::copy(fixture(f), dir.join(format!("turn{}.jsonl", i + 1))).unwrap();
    }
    let script = r#"#!/bin/sh
n=0
while IFS= read -r line; do
  printf '%s\n' "$line" >> 'DIR/stdin.log'
  case "$line" in
    *'"type":"user"'*)
      n=$((n + 1))
      sleep 0.5
      cat 'DIR/turn'$n'.jsonl'
      ;;
  esac
done
"#
    .replace("DIR", &dir.display().to_string());
    let bin = dir.join("claude");
    fs::write(&bin, script).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    Fake {
        bin: bin.display().to_string(),
        dir,
    }
}

fn config(f: &Fake) -> Config {
    Config {
        claude_bin: f.bin.clone(),
        model: "opus".into(),
        ..Config::default()
    }
}

// Collects events up to and including Busy(false).
fn until_idle(rx: &Receiver<UiEvent>) -> Vec<UiEvent> {
    let mut out = Vec::new();
    loop {
        let ev = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("engine went silent");
        let idle = ev == UiEvent::Busy(false);
        out.push(ev);
        if idle {
            return out;
        }
    }
}

// Polls a file until it contains needle (or 10 s pass). The fake script writes stdin.log lazily.
fn wait_for(path: &Path, needle: &str) -> String {
    let t0 = Instant::now();
    loop {
        let s = fs::read_to_string(path).unwrap_or_default();
        if s.contains(needle) || t0.elapsed() > Duration::from_secs(10) {
            return s;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn control(evs: &[UiEvent]) -> Vec<UiEvent> {
    evs.iter()
        .filter(|e| {
            matches!(
                e,
                UiEvent::Busy(_) | UiEvent::Queued(_) | UiEvent::TurnEnd { .. }
            )
        })
        .cloned()
        .collect()
}

#[test]
fn text_turn_emits_deltas_then_done() {
    let _g = lock();
    let f = fake("text", &["text_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("hi");
    let evs = until_idle(&rx);
    let status = |tokens, cost_usd| UiEvent::Status {
        model: "claude-opus-test".into(),
        tokens,
        cost_usd,
        context_pct: 0,
    };
    assert_eq!(
        evs,
        vec![
            UiEvent::Busy(true),
            status(0, 0.0),
            UiEvent::TextDelta {
                role: Role::Main,
                text: "Hel".into()
            },
            UiEvent::TextDelta {
                role: Role::Main,
                text: "lo".into()
            },
            UiEvent::MessageDone { role: Role::Main },
            status(100, 0.0),
            status(100, 0.01),
            UiEvent::TurnEnd { duration_ms: 42 },
            UiEvent::Busy(false),
        ]
    );
    h.shutdown();
}

#[test]
fn submit_while_busy_queues_and_never_interrupts() {
    let _g = lock();
    let f = fake("queue", &["text_turn.jsonl", "text_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("one");
    h.submit("two");
    let evs = until_idle(&rx);
    assert_eq!(
        control(&evs),
        vec![
            UiEvent::Busy(true),
            UiEvent::Queued(1),
            UiEvent::TurnEnd { duration_ms: 42 },
            UiEvent::Queued(0),
            UiEvent::TurnEnd { duration_ms: 42 },
            UiEvent::Busy(false),
        ]
    );
    let log = fs::read_to_string(f.dir.join("stdin.log")).unwrap();
    assert_eq!(log.matches(r#""type":"user""#).count(), 2, "{log}");
    assert!(!log.contains("interrupt"), "{log}");
    let (one, two) = (log.find("\"one\"").unwrap(), log.find("\"two\"").unwrap());
    assert!(one < two, "{log}");
    h.shutdown();
}

#[test]
fn agent_tool_maps_to_agent_start_and_end_and_hides_subagent_tools() {
    let _g = lock();
    let f = fake("agent", &["agent_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("scan");
    let evs = until_idle(&rx);
    let rel: Vec<UiEvent> = evs
        .iter()
        .filter(|e| {
            matches!(
                e,
                UiEvent::ToolStart { .. }
                    | UiEvent::ToolEnd { .. }
                    | UiEvent::AgentStart { .. }
                    | UiEvent::AgentEnd { .. }
                    | UiEvent::TextDelta { .. }
            )
        })
        .cloned()
        .collect();
    assert_eq!(rel.len(), 5, "{rel:?}");
    assert!(
        matches!(&rel[0], UiEvent::ToolStart { id, name, summary }
            if id == "toolu_agent1" && name == "Agent" && summary == "scan repo"),
        "{rel:?}"
    );
    assert!(
        matches!(&rel[1], UiEvent::AgentStart { id, description, .. }
            if id == "toolu_agent1" && description == "scan repo"),
        "{rel:?}"
    );
    assert_eq!(
        rel[2],
        UiEvent::TextDelta {
            role: Role::Agent("toolu_agent1".into()),
            text: "inside".into()
        }
    );
    assert_eq!(
        rel[3],
        UiEvent::ToolEnd {
            id: "toolu_agent1".into(),
            ok: true
        }
    );
    assert!(
        matches!(&rel[4], UiEvent::AgentEnd { id, .. } if id == "toolu_agent1"),
        "{rel:?}"
    );
    assert!(
        !evs.iter().any(|e| format!("{e:?}").contains("toolu_sub1")),
        "{evs:?}"
    );
    h.shutdown();
}

#[test]
fn interrupt_is_logged_with_source_and_sent_only_when_busy() {
    let _g = lock();
    let f = fake("interrupt", &["text_turn.jsonl"]);
    let log_path = f.dir.join("interrupt.log");
    let cfg = Config {
        log_path: Some(log_path.clone()),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("hi");
    h.interrupt(InterruptSource::EscKey);
    until_idle(&rx);
    h.interrupt(InterruptSource::CtrlC);
    let log = wait_for(&log_path, "source=ctrl_c");
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 2, "{log}");
    assert!(lines[0].contains("source=esc busy=true sent=true"), "{log}");
    assert!(
        lines[1].contains("source=ctrl_c busy=false sent=false"),
        "{log}"
    );
    let stdin = wait_for(&f.dir.join("stdin.log"), "interrupt");
    assert_eq!(
        stdin
            .matches(r#""request":{"subtype":"interrupt"}"#)
            .count(),
        1,
        "{stdin}"
    );
    h.shutdown();
}

#[test]
fn can_use_tool_request_is_denied_with_error() {
    let _g = lock();
    let f = fake("perm", &["permission_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("rm it");
    let evs = until_idle(&rx);
    assert!(
        evs.contains(&UiEvent::Error(
            "permission denied for Bash: no prompt handler".into()
        )),
        "{evs:?}"
    );
    let log = wait_for(&f.dir.join("stdin.log"), "control_response");
    assert!(log.contains(r#""request_id":"perm-1""#), "{log}");
    assert!(log.contains(r#""behavior":"deny""#), "{log}");
    h.shutdown();
}

#[test]
fn spawn_failure_reports_error_and_rejects_work() {
    let _g = lock();
    let cfg = Config {
        claude_bin: "/nonexistent/chaude-e1/claude".into(),
        ..Config::default()
    };
    let (h, rx) = Engine::start(cfg);
    let first = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("spawn error event");
    assert!(
        matches!(&first, UiEvent::Error(m) if m.starts_with("claude spawn failed")),
        "{first:?}"
    );
    h.submit("hi");
    let second = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("rejection event");
    assert_eq!(second, UiEvent::Error("claude is not running".into()));
    h.shutdown();
}

#[test]
fn background_agent_replay_has_no_agent_end_for_async_launch() {
    let _g = lock();
    let f = fake("bg", &["bg_turn_live.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("run in background");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{evs:?}"
    );
    // The fixture repeats the one Agent tool_use id in two messages; tool ids are deduped, so one start.
    let starts = evs
        .iter()
        .filter(|e| matches!(e, UiEvent::AgentStart { .. }))
        .count();
    assert_eq!(starts, 1, "{evs:?}");
    let async_id = "toolu_01QKrZwzJmDuTJ8a4RJUWNgn";
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == async_id)),
        "{evs:?}"
    );
    h.shutdown();
}

// Fake whose first user frame prints `first`, sleeps `gap` seconds, then prints `second`.
fn fake_gap(name: &str, first: &str, second: &str, gap: &str) -> Fake {
    let dir = temp_dir(name);
    fs::write(dir.join("turn1a.jsonl"), first).unwrap();
    fs::write(dir.join("turn1b.jsonl"), second).unwrap();
    let script = r#"#!/bin/sh
while IFS= read -r line; do
  printf '%s\n' "$line" >> 'DIR/stdin.log'
  case "$line" in
    *'"type":"user"'*)
      cat 'DIR/turn1a.jsonl'
      sleep GAP
      cat 'DIR/turn1b.jsonl'
      ;;
  esac
done
"#
    .replace("DIR", &dir.display().to_string())
    .replace("GAP", gap);
    let bin = dir.join("claude");
    fs::write(&bin, script).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    Fake {
        bin: bin.display().to_string(),
        dir,
    }
}

// agent_turn.jsonl split after its first line: the Agent tool_use, then everything else (incl. its result).
fn agent_turn_split() -> (String, String) {
    let all = fs::read_to_string(fixture("agent_turn.jsonl")).unwrap();
    let (head, tail) = all.split_once('\n').unwrap();
    (format!("{head}\n"), tail.to_string())
}

#[test]
fn slow_agent_result_is_delivered_and_timer_is_paused_without_interrupt() {
    let _g = lock();
    let (head, tail) = agent_turn_split();
    // Timeout 1 s, result arrives after 2.5 s (2x the timeout): the Agent tool_use is open, so the timer must stay paused.
    let f = fake_gap("slow-agent", &head, &tail, "2.5");
    let cfg = Config {
        inactivity_timeout: Duration::from_millis(1000),
        log_path: Some(f.dir.join("interrupt.log")),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("scan");
    let evs = until_idle(&rx);
    assert!(
        evs.contains(&UiEvent::ToolEnd {
            id: "toolu_agent1".into(),
            ok: true
        }),
        "{evs:?}"
    );
    assert!(
        evs.contains(&UiEvent::TurnEnd { duration_ms: 7 }),
        "late reply must complete the turn: {evs:?}"
    );
    assert!(matches!(evs.last(), Some(UiEvent::Busy(false))), "{evs:?}");
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::Error(m) if m.contains("still running"))),
        "timer must not fire while a tool_use is open: {evs:?}"
    );
    let stdin = fs::read_to_string(f.dir.join("stdin.log")).unwrap();
    assert!(!stdin.contains("interrupt"), "{stdin}");
    assert!(!f.dir.join("interrupt.log").exists());
    h.shutdown();
}

#[test]
fn silent_turn_emits_still_running_notice_and_never_interrupts() {
    let _g = lock();
    // No output for 2.5 s with no open tool_use: the 1 s timer fires, notices, and leaves the turn alone.
    let f = fake_gap(
        "silent",
        "",
        &fs::read_to_string(fixture("text_turn.jsonl")).unwrap(),
        "2.5",
    );
    let cfg = Config {
        inactivity_timeout: Duration::from_millis(1000),
        log_path: Some(f.dir.join("interrupt.log")),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("hi");
    let evs = until_idle(&rx);
    let notices = evs
        .iter()
        .filter(|e| matches!(e, UiEvent::Error(m) if m.starts_with("still running")))
        .count();
    assert!(notices >= 1, "{evs:?}");
    assert!(
        evs.contains(&UiEvent::TextDelta {
            role: Role::Main,
            text: "Hel".into()
        }),
        "late output must still arrive: {evs:?}"
    );
    assert!(matches!(evs.last(), Some(UiEvent::Busy(false))), "{evs:?}");
    let stdin = fs::read_to_string(f.dir.join("stdin.log")).unwrap();
    assert!(!stdin.contains("interrupt"), "{stdin}");
    assert!(!f.dir.join("interrupt.log").exists());
    h.shutdown();
}

// Go repl.go: after Interrupt the turn waits for its result, then gives up. An open Agent is ended with it.
#[test]
fn interrupted_turn_with_open_agent_is_ended_after_grace() {
    let _g = lock();
    let (head, _tail) = agent_turn_split();
    // The fake emits only the Agent tool_use and never a result.
    let f = fake_gap("hang-agent", &head, "", "0");
    let cfg = Config {
        interrupt_grace: Duration::from_millis(200),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("scan");
    let mut evs = Vec::new();
    loop {
        let ev = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("engine went silent");
        let started = matches!(ev, UiEvent::AgentStart { .. });
        evs.push(ev);
        if started {
            break;
        }
    }
    h.interrupt(InterruptSource::EscKey);
    evs.extend(until_idle(&rx));
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_agent1")),
        "open agent must end with the turn: {evs:?}"
    );
    assert!(
        evs.iter().any(|e| matches!(e, UiEvent::TurnEnd { .. })),
        "{evs:?}"
    );
    assert!(matches!(evs.last(), Some(UiEvent::Busy(false))), "{evs:?}");
    h.shutdown();
}

#[test]
fn interrupted_turn_without_result_is_ended_after_grace() {
    let _g = lock();
    let f = fake_gap("hang-plain", "", "", "0");
    let cfg = Config {
        interrupt_grace: Duration::from_millis(200),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("hi");
    h.interrupt(InterruptSource::CtrlC);
    let evs = until_idle(&rx);
    assert!(
        evs.iter().any(|e| matches!(e, UiEvent::TurnEnd { .. })),
        "{evs:?}"
    );
    assert!(matches!(evs.last(), Some(UiEvent::Busy(false))), "{evs:?}");
    h.shutdown();
}

// Each user frame n: the first prints the hanging turn; later ones print a stale result (of the abandoned
// turn) and then the reply. The stale result must not end the later turn.
fn fake_stray(name: &str, hang: &str, reply: &str) -> Fake {
    let dir = temp_dir(name);
    fs::write(dir.join("hang.jsonl"), hang).unwrap();
    fs::write(dir.join("reply.jsonl"), reply).unwrap();
    let script = r#"#!/bin/sh
n=0
while IFS= read -r line; do
  printf '%s\n' "$line" >> 'DIR/stdin.log'
  case "$line" in
    *'"type":"user"'*)
      n=$((n + 1))
      if [ $n -eq 1 ]; then
        cat 'DIR/hang.jsonl'
      else
        printf '%s\n' '{"type":"result","session_id":"x"}'
        cat 'DIR/reply.jsonl'
      fi
      ;;
  esac
done
"#
    .replace("DIR", &dir.display().to_string());
    let bin = dir.join("claude");
    fs::write(&bin, script).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    Fake {
        bin: bin.display().to_string(),
        dir,
    }
}

#[test]
fn stray_result_of_abandoned_turn_does_not_end_next_turn() {
    let _g = lock();
    let (head, _tail) = agent_turn_split();
    let f = fake_stray(
        "stray",
        &head,
        &fs::read_to_string(fixture("text_turn.jsonl")).unwrap(),
    );
    let cfg = Config {
        interrupt_grace: Duration::from_millis(200),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("scan");
    loop {
        let ev = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("engine went silent");
        if matches!(ev, UiEvent::AgentStart { .. }) {
            break;
        }
    }
    h.interrupt(InterruptSource::EscKey);
    let first = until_idle(&rx);
    assert!(
        matches!(first.last(), Some(UiEvent::Busy(false))),
        "{first:?}"
    );
    h.submit("next");
    let evs = until_idle(&rx);
    let text = evs
        .iter()
        .position(|e| {
            matches!(
                e,
                UiEvent::TextDelta {
                    role: Role::Main,
                    ..
                }
            )
        })
        .unwrap_or_else(|| panic!("reply lost: {evs:?}"));
    let end = evs
        .iter()
        .position(|e| matches!(e, UiEvent::TurnEnd { .. }))
        .unwrap_or_else(|| panic!("no TurnEnd: {evs:?}"));
    assert!(
        text < end,
        "stale result ended the next turn before its reply: {evs:?}"
    );
    assert!(matches!(evs.last(), Some(UiEvent::Busy(false))), "{evs:?}");
    h.shutdown();
}
