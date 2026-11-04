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
            UiEvent::TurnStats {
                duration_ms: 42,
                cost_turn_usd: 0.01,
                cost_usd: 0.01,
                ctx: 100,
                delegations: 0,
            },
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
fn unparsed_child_line_is_traced_and_turn_still_ends() {
    let _g = lock();
    let f = fake("unparsed", &["unparsed_turn.jsonl"]);
    let trace = f.dir.join("transcript.jsonl");
    let cfg = Config {
        transcript: Some(trace.clone()),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("hi");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::Error(_))),
        "{evs:?}"
    );
    assert!(
        evs.iter().any(|e| matches!(e, UiEvent::TurnEnd { .. })),
        "{evs:?}"
    );
    let log = wait_for(&trace, "unparsed");
    assert_eq!(log.lines().count(), 1, "{log}");
    assert!(
        log.contains(r#""kind":"unparsed""#) && log.contains(r#""raw":"not json at all""#),
        "{log}"
    );
    h.shutdown();
}

#[test]
fn queued_prompts_go_out_as_one_merged_turn_on_done() {
    let _g = lock();
    let f = fake("merge", &["text_turn.jsonl", "text_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("first");
    h.submit("second");
    h.submit("third");
    let evs = until_idle(&rx);
    assert_eq!(
        control(&evs),
        vec![
            UiEvent::Busy(true),
            UiEvent::Queued(1),
            UiEvent::Queued(2),
            UiEvent::TurnEnd { duration_ms: 42 },
            UiEvent::Queued(0),
            UiEvent::TurnEnd { duration_ms: 42 },
            UiEvent::Busy(false),
        ]
    );
    let log = wait_for(&f.dir.join("stdin.log"), r"second\n\nthird");
    assert_eq!(log.matches(r#""type":"user""#).count(), 2, "{log}");
    assert!(log.contains(r"second\n\nthird"), "{log}");
    h.shutdown();
}

#[test]
fn background_only_agent_does_not_queue_next_prompt() {
    let _g = lock();
    let f = fake("bgonly", &["bg_turn_live.jsonl", "text_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("run in background");
    until_idle(&rx);
    h.submit("now");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::Queued(_))),
        "{evs:?}"
    );
    assert_eq!(control(&evs)[0], UiEvent::Busy(true), "{evs:?}");
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
fn can_use_tool_in_default_mode_asks_ui_and_sends_answer() {
    let _g = lock();
    let f = fake("perm", &["permission_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("rm it");
    let evs = until_idle(&rx);
    assert!(
        evs.contains(&UiEvent::PermissionRequest {
            id: "perm-1".into(),
            tool: "Bash".into(),
            summary: "rm -rf x".into(),
        }),
        "{evs:?}"
    );
    h.answer_permission("perm-1", true);
    let log = wait_for(&f.dir.join("stdin.log"), "control_response");
    assert!(log.contains(r#""request_id":"perm-1""#), "{log}");
    assert!(log.contains(r#""behavior":"allow""#), "{log}");
    assert!(log.contains("rm -rf x"), "{log}");
    h.shutdown();
}

#[test]
fn plan_mode_denies_bash_without_prompt_and_unknown_mode_is_reported() {
    let _g = lock();
    let f = fake("perm-plan", &["permission_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.set_mode("bogus");
    h.set_mode("plan");
    h.submit("rm it");
    let evs = until_idle(&rx);
    assert!(
        evs.contains(&UiEvent::Error(
            "unknown mode bogus (default | acceptEdits | plan | auto)".into()
        )),
        "{evs:?}"
    );
    assert!(
        evs.contains(&UiEvent::ModeChanged {
            mode: "plan".into()
        }),
        "{evs:?}"
    );
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::PermissionRequest { .. })),
        "{evs:?}"
    );
    let log = wait_for(&f.dir.join("stdin.log"), "control_response");
    assert!(log.contains(r#""behavior":"deny""#), "{log}");
    assert!(log.contains("plan mode: do not modify anything"), "{log}");
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
    assert!(evs.contains(&UiEvent::Interrupted), "{evs:?}");
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

// tool_result text follows Go resultText; the async launch marks the agent as background, not ended.
#[test]
fn tool_result_text_and_background_agent_flag_are_emitted() {
    let _g = lock();
    let f = fake("bg-flag", &["bg_turn_live.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("run in background");
    let evs = until_idle(&rx);
    let id = "toolu_01QKrZwzJmDuTJ8a4RJUWNgn".to_string();
    assert!(
        evs.iter().any(|e| matches!(
            e,
            UiEvent::ToolResult { id: i, text } if *i == id && text.starts_with("Async agent launched")
        )),
        "{evs:?}"
    );
    assert!(
        evs.contains(&UiEvent::AgentBackground { id: id.clone() }),
        "{evs:?}"
    );
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id: i, .. } if *i == id)),
        "{evs:?}"
    );
    h.shutdown();
}

// rate_limit_event carries the raw rate_limit_info object (Go setRate) for the TUI.
#[test]
fn rate_limit_event_emits_raw_info() {
    let _g = lock();
    let f = fake("rate", &["bg_turn_live.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("rate limit");
    let evs = until_idle(&rx);
    let info = evs
        .iter()
        .find_map(|e| match e {
            UiEvent::RateLimit { info } => Some(info.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no RateLimit: {evs:?}"));
    assert_eq!(info["rateLimitType"], "five_hour");
    assert_eq!(info["unifiedWindows"]["five_hour"]["utilization"], 0.38);
    h.shutdown();
}

// TurnStats carries the per-turn measurement (Go record_turn.go keys) just before TurnEnd.
#[test]
fn turn_stats_reports_turn_cost_delegations_and_context() {
    let _g = lock();
    let f = fake("turn-stats", &["agent_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("scan");
    let evs = until_idle(&rx);
    let pos = evs
        .iter()
        .position(|e| matches!(e, UiEvent::TurnStats { .. }))
        .unwrap_or_else(|| panic!("no TurnStats: {evs:?}"));
    assert!(
        matches!(evs.get(pos + 1), Some(UiEvent::TurnEnd { .. })),
        "TurnStats must precede TurnEnd: {evs:?}"
    );
    let UiEvent::TurnStats {
        cost_turn_usd,
        cost_usd,
        ctx,
        delegations,
        ..
    } = &evs[pos]
    else {
        unreachable!()
    };
    assert!((cost_usd - 0.02).abs() < 1e-9, "{evs:?}");
    assert!((cost_turn_usd - 0.02).abs() < 1e-9, "{evs:?}");
    assert_eq!(*delegations, 1, "{evs:?}");
    let last_tokens = evs
        .iter()
        .filter_map(|e| match e {
            UiEvent::Status { tokens, .. } => Some(*tokens),
            _ => None,
        })
        .next_back()
        .expect("status");
    assert_eq!(*ctx, last_tokens, "{evs:?}");
    h.shutdown();
}

#[test]
fn auto_allow_answers_asks_without_ui_prompt() {
    let _g = lock();
    let f = fake("perm-auto", &["permission_turn.jsonl"]);
    let cfg = Config {
        auto_allow: true,
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("rm it");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::PermissionRequest { .. })),
        "{evs:?}"
    );
    let log = wait_for(&f.dir.join("stdin.log"), "control_response");
    assert!(log.contains(r#""behavior":"allow""#), "{log}");
    h.shutdown();
}

#[test]
fn mode_without_name_reports_current_mode() {
    let _g = lock();
    let f = fake("mode-report", &["text_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.set_mode("");
    let ev = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("mode report");
    assert_eq!(
        ev,
        UiEvent::ModeChanged {
            mode: "default".into()
        }
    );
    h.shutdown();
}

#[test]
fn pending_permission_is_dropped_when_turn_is_abandoned() {
    let _g = lock();
    let perm = "{\"type\":\"control_request\",\"request_id\":\"perm-1\",\"request\":{\"subtype\":\"can_use_tool\",\"tool_name\":\"Bash\",\"input\":{\"command\":\"rm -rf x\"}}}\n";
    let f = fake_gap("perm-hang", perm, "", "0");
    let cfg = Config {
        interrupt_grace: Duration::from_millis(200),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("rm it");
    let mut evs = Vec::new();
    loop {
        let ev = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("permission prompt");
        let prompt = matches!(ev, UiEvent::PermissionRequest { .. });
        evs.push(ev);
        if prompt {
            break;
        }
    }
    h.interrupt(InterruptSource::CtrlC);
    let rest = until_idle(&rx);
    assert!(
        matches!(rest.last(), Some(UiEvent::Busy(false))),
        "{rest:?}"
    );
    h.answer_permission("perm-1", true);
    let ev = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("reply to stale answer");
    assert_eq!(
        ev,
        UiEvent::Error("no pending permission request perm-1".into())
    );
    h.shutdown();
}

#[test]
fn budget_interrupts_turn_once_when_in_flight_estimate_reaches_cap() {
    let _g = lock();
    let f = fake("budget-mid", &["budget_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.task_budget_usd = 0.1;
    let (h, rx) = Engine::start(cfg);
    h.submit("big");
    let evs = until_idle(&rx);
    let stop = evs
        .iter()
        .position(|e| matches!(e, UiEvent::BudgetStop { .. }))
        .unwrap_or_else(|| panic!("no BudgetStop: {evs:?}"));
    let UiEvent::BudgetStop { spent_usd, cap_usd } = &evs[stop] else {
        unreachable!()
    };
    assert!(
        (spent_usd - 0.5).abs() < 1e-9 && (cap_usd - 0.1).abs() < 1e-9,
        "{evs:?}"
    );
    assert!(
        matches!(evs.get(stop + 1), Some(UiEvent::TurnStats { .. })),
        "BudgetStop must precede TurnStats: {evs:?}"
    );
    assert!(
        matches!(evs.get(stop + 2), Some(UiEvent::TurnEnd { .. })),
        "TurnStats must precede TurnEnd: {evs:?}"
    );
    assert!(
        evs.iter().any(|e| matches!(e, UiEvent::Interrupted)),
        "{evs:?}"
    );
    let log = wait_for(&f.dir.join("stdin.log"), "interrupt");
    assert_eq!(
        log.matches(r#""request":{"subtype":"interrupt"}"#).count(),
        1,
        "{log}"
    );
    h.shutdown();
}

#[test]
fn budget_refuses_prompt_whose_next_call_estimate_exceeds_cap() {
    let _g = lock();
    let f = fake("budget-refuse", &["agent_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.task_budget_usd = 0.01;
    let (h, rx) = Engine::start(cfg);
    h.submit("hi");
    thread::sleep(Duration::from_millis(400));
    let evs: Vec<UiEvent> = rx.try_iter().collect();
    assert!(
        evs.iter().any(|e| matches!(e, UiEvent::BudgetStop { .. })),
        "{evs:?}"
    );
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::Busy(true))),
        "refused prompt must not start a turn: {evs:?}"
    );
    let log = fs::read_to_string(f.dir.join("stdin.log")).unwrap_or_default();
    assert!(!log.contains("\"type\":\"user\""), "{log}");
    h.shutdown();
}

#[test]
fn budget_status_reports_spent_and_cap_after_each_turn() {
    let _g = lock();
    let f = fake("budget-status", &["agent_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.task_budget_usd = 10.0;
    let (h, rx) = Engine::start(cfg);
    h.submit("scan");
    let evs = until_idle(&rx);
    let got = evs.iter().find_map(|e| match e {
        UiEvent::BudgetStatus { spent_usd, cap_usd } => Some((*spent_usd, *cap_usd)),
        _ => None,
    });
    let (spent, cap) = got.unwrap_or_else(|| panic!("no BudgetStatus: {evs:?}"));
    assert!(
        (spent - 0.02).abs() < 1e-9 && (cap - 10.0).abs() < 1e-9,
        "{evs:?}"
    );
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::BudgetStop { .. })),
        "{evs:?}"
    );
    h.shutdown();
}

// Waits for the reply to a usage() call: Usage(text) or an "usage:" Error. Session spawn errors are skipped,
// because Config::claude_bin is shared by the session child and the one-shot run.
fn usage_reply(rx: &Receiver<UiEvent>) -> UiEvent {
    loop {
        let ev = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("engine went silent");
        match &ev {
            UiEvent::Usage { .. } => return ev,
            UiEvent::Error(m) if m.starts_with("usage:") => return ev,
            _ => {}
        }
    }
}

#[test]
fn usage_answers_with_result_text_from_one_shot_run() {
    let _g = lock();
    let f = fake("usage", &[]);
    // Wrapper: `-p /usage` prints the JSON result; every other argv (the session child) runs the fake.
    let usage = f.dir.join("usage-claude");
    let script = format!(
        "#!/bin/sh\ncase \"$*\" in\n  *'/usage'*) printf '%s\\n' '{{\"type\":\"result\",\"result\":\"Plan: Max 5x\"}}'; exit 0;;\nesac\nexec '{}' \"$@\"\n",
        f.bin
    );
    fs::write(&usage, script).unwrap();
    fs::set_permissions(&usage, fs::Permissions::from_mode(0o755)).unwrap();
    let mut cfg = config(&f);
    cfg.claude_bin = usage.display().to_string();
    let (h, rx) = Engine::start(cfg);
    h.usage();
    match usage_reply(&rx) {
        UiEvent::Usage { text } => assert_eq!(text, "Plan: Max 5x"),
        other => panic!("expected Usage, got {other:?}"),
    }
    h.shutdown();
}

#[test]
fn usage_missing_binary_reports_error() {
    let _g = lock();
    let f = fake("usage-missing", &[]);
    let mut cfg = config(&f);
    cfg.claude_bin = f.dir.join("no-such-claude").display().to_string();
    let (h, rx) = Engine::start(cfg);
    h.usage();
    match usage_reply(&rx) {
        UiEvent::Error(msg) => assert!(msg.starts_with("usage:"), "{msg}"),
        other => panic!("expected Error, got {other:?}"),
    }
    h.shutdown();
}

// Go metrics: a finished turn appends one "turn" record to <metrics_dir>/<UTC date>.jsonl before TurnEnd.
#[test]
fn turn_is_logged_to_metrics_jsonl() {
    let _g = lock();
    let f = fake("metrics", &["text_turn.jsonl"]);
    let metrics = f.dir.join("metrics");
    let cfg = Config {
        metrics_dir: Some(metrics.clone()),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("hi");
    until_idle(&rx);
    let files: Vec<PathBuf> = fs::read_dir(&metrics)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1, "{files:?}");
    let text = fs::read_to_string(&files[0]).unwrap();
    let rec: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(rec["stage"], "turn", "{text}");
    assert!(rec["ts"].is_string(), "{text}");
    assert_eq!(rec["flags"]["context_window"], 200_000, "{text}");
    assert!(rec["duration_ms"].is_u64(), "{text}");
    assert!(rec["cost_turn_usd"].is_f64(), "{text}");
}

#[test]
fn context_rotation_hands_off_and_fresh_child_gets_snapshot() {
    let _g = lock();
    let f = fake("rotate", &["rotate_turn.jsonl", "handoff_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let evs = until_idle(&rx);
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::Rotated { reason } if reason == "ctx 100 >= 50"))
    );
    // The handoff turn is hidden: its note never reaches the UI as text.
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::TextDelta { text, .. } if text.contains("Goal")))
    );
    h.submit("again");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "## Current user message");
    assert!(log.contains("Write a handoff note"));
    let frame = log
        .lines()
        .find(|l| l.contains("## Current user message"))
        .expect("snapshot frame");
    assert!(frame.contains("Goal: finish the context rotation test"));
    assert!(frame.contains("again"));
    h.shutdown();
}

#[test]
fn rotation_waits_for_background_agent_then_runs_at_defer_timeout() {
    let _g = lock();
    let f = fake("defer", &["bg_rotate_turn.jsonl", "handoff_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let evs = until_idle(&rx);
    // The agent never reports a result, so the rotation is deferred: no handoff prompt yet.
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::Rotated { .. })),
        "{evs:?}"
    );
    assert!(
        !f.dir.join("stdin.log").exists()
            || !fs::read_to_string(f.dir.join("stdin.log"))
                .unwrap()
                .contains("Write a handoff note")
    );
    // After the defer timeout the rotation runs while idle.
    let mut rotated = false;
    while let Ok(e) = rx.recv_timeout(Duration::from_secs(5)) {
        if matches!(e, UiEvent::Rotated { .. }) {
            rotated = true;
            break;
        }
    }
    assert!(rotated);
    h.shutdown();
}

#[test]
fn handoff_snapshot_lists_running_background_agent_with_result_path() {
    let _g = lock();
    let f = fake("bgsnap", &["bg_async_turn.jsonl", "handoff_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    until_idle(&rx);
    // The async agent never reports a result, so the rotation runs at the defer timeout.
    loop {
        let ev = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("rotation never ran");
        if matches!(ev, UiEvent::Rotated { .. }) {
            break;
        }
    }
    h.submit("again");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "## Current user message");
    let frame = log
        .lines()
        .find(|l| l.contains("## Current user message"))
        .expect("snapshot frame");
    assert!(frame.contains("## Background agents (results in notes/runs/)"));
    assert!(frame.contains("- scan (toolu_bg1): running, result file: notes/runs/"));
    assert!(frame.contains("-agent-toolu_bg1.md"));
    h.shutdown();
}

#[test]
fn resumed_background_agent_is_active_again_until_task_notification() {
    let _g = lock();
    let f = fake("resume", &["resume_agent_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let pos = |want: &dyn Fn(&UiEvent) -> bool| evs.iter().position(want);
    let launched = pos(&|e| matches!(e, UiEvent::AgentBackground { id } if id == "toolu_bg1"));
    let resumed = pos(
        &|e| matches!(e, UiEvent::AgentResumed { id, description, .. } if id == "toolu_bg1" && description == "scan"),
    );
    let ended = pos(&|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1"));
    assert!(launched.is_some(), "{evs:?}");
    assert!(resumed.is_some(), "{evs:?}");
    assert!(ended.is_some(), "{evs:?}");
    assert!(launched < resumed && resumed < ended, "{evs:?}");
    // The failed second SendMessage must not resume the agent again.
    let resumes = evs
        .iter()
        .filter(|e| matches!(e, UiEvent::AgentResumed { .. }))
        .count();
    assert_eq!(resumes, 1, "{evs:?}");
    h.shutdown();
}

#[test]
fn abandoned_turn_ends_foreground_agent_but_not_background_agent() {
    let _g = lock();
    let hang = concat!(
        r#"{"type":"system","subtype":"init","session_id":"s1","model":"claude-haiku-test"}"#,
        "\n",
        r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_bg","name":"Agent","input":{"description":"bg","prompt":"p","subagent_type":"general"}}]},"session_id":"s1","parent_tool_use_id":null}"#,
        "\n",
        r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_bg","content":"Async agent launched successfully.","is_error":false}]},"session_id":"s1","parent_tool_use_id":null}"#,
        "\n",
        r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_fg","name":"Agent","input":{"description":"fg","prompt":"p","subagent_type":"general"}}]},"session_id":"s1","parent_tool_use_id":null}"#,
        "\n",
    );
    let f = fake_stray(
        "abandon_bg",
        hang,
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
        if matches!(ev, UiEvent::AgentStart { ref id, .. } if id == "toolu_fg") {
            break;
        }
    }
    h.interrupt(InterruptSource::EscKey);
    let evs = until_idle(&rx);
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_fg")),
        "{evs:?}"
    );
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg")),
        "{evs:?}"
    );
    h.shutdown();
}
