#![cfg(unix)]
// Engine tests against a fake `claude` shell script (Config::claude_bin override).
// The script logs each stdin line to <dir>/stdin.log and, on the Nth user frame, replays <dir>/turnN.jsonl.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc::Receiver;
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chaude_core::api::{CoderContext, Config, Engine, InterruptSource, Role, UiEvent};
use chaude_core::clients::{ClientRegistry, PARKED};
use chaude_core::handoff;
use chaude_core::import_claude::ClaudeImport;
use chaude_core::resident::StagePhase;

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

// The test dir holds the fake script, the records and the clients/agents state files; removed on drop.
impl Drop for Fake {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

// Each user frame n sleeps 0.5 s, then prints turn{n}.jsonl. The sleep makes queueing deterministic.
fn fake(name: &str, turns: &[&str]) -> Fake {
    let dir = temp_dir(name);
    for (i, f) in turns.iter().enumerate() {
        fs::copy(fixture(f), dir.join(format!("turn{}.jsonl", i + 1))).unwrap();
    }
    // The prompter reply; an empty file makes the fake prompter exit on its first prompt.
    fs::write(
        dir.join("prompter.jsonl"),
        "{\"type\":\"result\",\"result\":\"caveman summary\"}\n",
    )
    .unwrap();
    // The tui reply; an empty file makes the fake tui exit on its first prompt. Its agent body carries TUIMARK.
    fs::write(
        dir.join("tui.jsonl"),
        "{\"type\":\"result\",\"result\":\"tui answer\"}\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join(".claude").join("agents")).unwrap();
    fs::write(
        dir.join(".claude").join("agents").join("haiku-tui.md"),
        "---\nname: haiku-tui\n---\n\nTUIMARK translator body\n",
    )
    .unwrap();
    // The coder reply: status and changed lines, which summarize_reply keeps. Its agent body carries CODERMARK.
    fs::write(
        dir.join("coder.jsonl"),
        "{\"type\":\"result\",\"result\":\"status: done\\nchanged: a.rs: edit\",\"session_id\":\"c1\"}\n",
    )
    .unwrap();
    fs::write(
        dir.join(".claude").join("agents").join("haiku-coder.md"),
        "---\nname: haiku-coder\n---\n\nCODERMARK coder body\n",
    )
    .unwrap();
    let script = r#"#!/bin/sh
n=0
case "$*" in
  *TUIMARK*) role=tui ;;
  *CODERMARK*) role=coder ;;
  *" --system-prompt "*) role=prompter ;;
  *) role=main ;;
esac
printf '%s\n' "$role" >> 'DIR/spawned.log'
# Every coder child records its pid at spawn, before any task: the base coder is the one spawned at boot.
[ "$role" = coder ] && printf '%s boot\n' "$$" >> 'DIR/coder.boot.pids'
[ "$role" = prompter ] && [ -f 'DIR/prompter.idle' ] && cat 'DIR/prompter.idle'
[ "$role" = main ] && printf '%s\n' "$*" >> 'DIR/args.log'
# One file per process: the spawns run at once, and a long argv can split across writes to a shared log.
printf '%s %s\n' "$role" "$(printf '%s' "$*" | tr '\n' ' ')" > "DIR/role_args.$$.log"
[ "$role" = main ] && printf '%s %s\n' "$$" "$*" >> 'DIR/main.pids'
while IFS= read -r line; do
  if [ "$role" = coder ]; then
    case "$line" in
      *'"type":"user"'*)
        # Record this child's pid against its task: "first" is slot 0 (base), anything else an extra slot.
        case "$line" in
          *first*) echo "$$ base" >> 'DIR/coder.pids' ;;
          *) echo "$$ extra" >> 'DIR/coder.pids' ;;
        esac
        sleep 0.2
        cat 'DIR/coder.jsonl'
        ;;
    esac
    continue
  fi
  if [ "$role" = tui ]; then
    case "$line" in
      *'"type":"user"'*)
        printf '%s\n' "$line" >> 'DIR/tui.log'
        [ -s 'DIR/tui.jsonl' ] || exit 1
        cat 'DIR/tui.jsonl'
        ;;
    esac
    continue
  fi
  if [ "$role" = prompter ]; then
    case "$line" in
      *'"type":"user"'*)
        [ -s 'DIR/prompter.jsonl' ] || exit 1
        cat 'DIR/prompter.jsonl'
        ;;
    esac
    continue
  fi
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
        // Baseline tests assert the non-resident event stream; stage tests opt in.
        residents: false,
        // Old semantics: a turn ends at once while its Agent/Task calls run (no ReplyHeld hold).
        agent_wait: false,
        // Clients and agents state stay in the test dir, never ~/.chaude.
        agents_file: Some(f.dir.join("agents.json")),
        clients_file: Some(f.dir.join("clients.json")),
        // Record turns land under the test dir, never the repo record/ folder.
        record_dir: Some(f.dir.join("record")),
        // notes/tmp (tasks, mcp, usage) lands under the test dir, never the process cwd (crates/core).
        cwd: Some(f.dir.clone()),
        ..Config::default()
    }
}

// Longest wait for one engine event or file write. A passing run never reaches it; it only keeps a
// loaded machine (parallel cargo builds) from failing a test that is still making progress.
const WAIT: Duration = Duration::from_secs(60);

// Collects events up to and including Busy(false).
fn until_idle(rx: &Receiver<UiEvent>) -> Vec<UiEvent> {
    let mut out = Vec::new();
    loop {
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
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
        if s.contains(needle) || t0.elapsed() > WAIT {
            return s;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

// The pid of the main child started with --model opus. Each spawn writes its own role_args.<pid>.log, so the
// name gives the pid without reading main.pids: two spawns appending long argv lines to that shared file can
// interleave bytes and tear the line the opus pid is on.
fn opus_child_pid(dir: &Path) -> u32 {
    let t0 = Instant::now();
    loop {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(pid) = name
                .strip_prefix("role_args.")
                .and_then(|s| s.strip_suffix(".log"))
            else {
                continue;
            };
            let text = fs::read_to_string(e.path()).unwrap_or_default();
            if text.starts_with("main ") && text.contains(" --model opus ") {
                return pid.parse().expect("pid in role_args file name");
            }
        }
        assert!(t0.elapsed() < WAIT, "no opus child in {}", dir.display());
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
            UiEvent::SessionIds {
                main: "s1".into(),
                residents: BTreeMap::new(),
                owned: vec!["s1".into()],
            },
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

// Go acceptance_test.go:81-96 TestLiveStreamingMultipleDeltas (live claude, asserts deltas > 3). The
// fixture stands in for the live stream: 30 text deltas, one per count number.
#[test]
fn live_streaming_multiple_deltas() {
    let _g = lock();
    let f = fake("many-deltas", &["count_deltas_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("Count from 1 to 30, one per line. No tools.");
    let evs = until_idle(&rx);
    let deltas: Vec<&str> = evs
        .iter()
        .filter_map(|e| match e {
            UiEvent::TextDelta {
                role: Role::Main,
                text,
            } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(deltas.len() > 3, "deltas={}", deltas.len());
    assert_eq!(deltas.len(), 30, "one TextDelta per streamed delta");
    let expected: String = (1..=30).map(|i| format!("{i} ")).collect();
    assert_eq!(deltas.concat(), expected, "streamed text in order");
    h.shutdown();
}

// Go TestChildPIDRecordedOnPark (budget_test.go:145-160): parking the active child writes its pid into the registry row.
// The engine starts the configured opus child; a tagged line switches to haiku, which parks the opus child.
#[test]
fn child_pid_recorded_on_park() {
    let _g = lock();
    let f = fake("park-pid", &["text_turn.jsonl"]);
    // Mode f routes lines. The router reads CHAUDE_ORCH once, when the engine is built.
    // SAFETY: set before Engine::start and removed after the run; other engine tests are serialized by lock().
    unsafe { std::env::set_var("CHAUDE_ORCH", "f") };
    // No cwd option on purpose: the clients key is the process cwd asserted below.
    let (h, rx) = Engine::start(Config {
        cwd: None,
        ..config(&f)
    });
    h.submit("[ws:a] first");
    until_idle(&rx);
    // SAFETY: see set_var above.
    unsafe { std::env::remove_var("CHAUDE_ORCH") };
    let opus_pid = opus_child_pid(&f.dir);
    // An engine without a cwd option keys its clients by the absolute process cwd (Go r.a.Cwd), not ".".
    let cwd = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let rec = ClientRegistry::new(f.dir.join("clients.json"))
        .get(&cwd, "opus")
        .expect("parked opus row");
    assert_eq!(rec.state, PARKED, "{rec:?}");
    assert_ne!(rec.child_pid, 0, "{rec:?}");
    assert_eq!(rec.child_pid, opus_pid, "{rec:?}");
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
    // Each queue change sends the waiting texts, oldest first; the merged turn empties them.
    let texts: Vec<Vec<String>> = evs
        .iter()
        .filter_map(|e| match e {
            UiEvent::QueueTexts(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        vec![
            vec!["second".to_string()],
            vec!["second".to_string(), "third".to_string()],
            vec![],
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
    // Subagent tools stay out of the main pane's tool events. Their AgentLog lines (agent pane) carry the id by design.
    assert!(
        !evs.iter().any(|e| matches!(e,
            UiEvent::ToolStart { id, .. }
            | UiEvent::ToolInput { id, .. }
            | UiEvent::ToolEnd { id, .. }
            | UiEvent::ToolResult { id, .. } if id == "toolu_sub1")),
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
    h.set_mode("default");
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

// Go repl.go start(): the harness starts in auto, so a can_use_tool request is allowed without a UI prompt.
#[test]
fn engine_starts_in_auto_mode_and_allows_without_prompt() {
    let _g = lock();
    let f = fake("perm-auto", &["permission_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
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
    let f = fake("spawn-fail", &[]);
    let cfg = Config {
        claude_bin: "/nonexistent/chaude-e1/claude".into(),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    let first = rx.recv_timeout(WAIT).expect("spawn error event");
    assert!(
        matches!(&first, UiEvent::Error(m) if m.starts_with("claude spawn failed")),
        "{first:?}"
    );
    h.submit("hi");
    let second = rx.recv_timeout(WAIT).expect("rejection event");
    assert_eq!(second, UiEvent::Error("claude is not running".into()));
    h.shutdown();
}

// Go repl.go:1364-1368: the first claude spawn is awaited, so a failure reaches the caller (exit 1 in the CLI).
#[test]
fn try_start_returns_the_spawn_failure() {
    let _g = lock();
    let f = fake("try-start-fail", &[]);
    let cfg = Config {
        claude_bin: "/nonexistent/chaude-x01/claude".into(),
        ..config(&f)
    };
    let err = Engine::try_start(cfg).err().expect("spawn error returned");
    assert!(!err.is_empty(), "spawn error text: {err}");
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

// Turn 1 is replayed on the first user frame. On the second (the handoff turn) the child replies, then a second later
// prints <notice>, as a drained child does while its agents run.
fn fake_drain(name: &str, first: &str, second: &str, notice: &str) -> Fake {
    let dir = temp_dir(name);
    fs::write(dir.join("turn1.jsonl"), first).unwrap();
    fs::write(dir.join("turn2.jsonl"), second).unwrap();
    fs::write(dir.join("notice.jsonl"), notice).unwrap();
    let script = r#"#!/bin/sh
n=0
while IFS= read -r line; do
  printf '%s\n' "$line" >> 'DIR/stdin.log'
  printf '%s\n' "$line" >> 'DIR/stdin-'"$$"'.log'
  case "$line" in
    *'"type":"user"'*)
      n=$((n + 1))
      if [ $n -eq 1 ]; then
        cat 'DIR/turn1.jsonl'
      else
        cat 'DIR/turn2.jsonl'
        sleep 1
        cat 'DIR/notice.jsonl'
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

// Owned-task tests. Turn 1 starts the Agent toolu_bg1 and its owned bash task b1 (owned_by_subagent, parent toolu_bg1),
// then replies idle. The usage is above the rotate threshold 50 used by the rotation test.
const OWNED_TURN: &str = concat!(
    "{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"s1\",\"model\":\"claude-haiku-test\"}\n",
    "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"toolu_bg1\",\"name\":\"Agent\",\"input\":{\"description\":\"scan\",\"prompt\":\"look around\",\"subagent_type\":\"general\",\"run_in_background\":true}}]},\"session_id\":\"s1\",\"parent_tool_use_id\":null}\n",
    "{\"type\":\"system\",\"subtype\":\"task_started\",\"task_id\":\"a1\",\"tool_use_id\":\"toolu_bg1\",\"task_type\":\"local_agent\",\"session_id\":\"s1\"}\n",
    "{\"type\":\"system\",\"subtype\":\"task_started\",\"task_id\":\"b1\",\"tool_use_id\":\"toolu_b1\",\"parent_task_id\":\"toolu_bg1\",\"owned_by_subagent\":true,\"task_type\":\"local_bash\",\"session_id\":\"s1\"}\n",
    "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Started\"}],\"usage\":{\"input_tokens\":10,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":90,\"output_tokens\":2}},\"session_id\":\"s1\",\"parent_tool_use_id\":null}\n",
    "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"Started\",\"session_id\":\"s1\",\"duration_ms\":42,\"total_cost_usd\":0.01}\n",
);
// The agent-level completion (tool_use_id of the agent) and the owned bash completion.
const AGENT_DONE: &str = "{\"type\":\"system\",\"subtype\":\"task_notification\",\"task_id\":\"a1\",\"tool_use_id\":\"toolu_bg1\",\"status\":\"completed\",\"session_id\":\"s1\"}\n";
const OWNED_DONE: &str = "{\"type\":\"system\",\"subtype\":\"task_notification\",\"task_id\":\"b1\",\"tool_use_id\":\"toolu_b1\",\"parent_task_id\":\"toolu_bg1\",\"owned_by_subagent\":true,\"status\":\"completed\",\"session_id\":\"s1\"}\n";
const PLAIN_REPLY: &str = "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"ok\",\"session_id\":\"s1\",\"duration_ms\":5,\"total_cost_usd\":0.0}\n";

// Fake for the owned-task tests. Frame 1 prints turns[0] and runs first_bg in the background (a timeline that outlives
// the turn). Frame 2 prints turns[1], then runs second_fg in the foreground. Frames count across processes, so a
// rotated child's first prompt gets turns[2]. The shell bodies may name DIR.
fn fake_owned(name: &str, turns: [&str; 3], first_bg: &str, second_fg: &str) -> Fake {
    let dir = temp_dir(name);
    for (i, t) in turns.iter().enumerate() {
        fs::write(dir.join(format!("turn{}.jsonl", i + 1)), t).unwrap();
    }
    let script = r#"#!/bin/sh
while IFS= read -r line; do
  printf '%s\n' "$line" >> 'DIR/stdin.log'
  printf '%s\n' "$line" >> 'DIR/stdin-'"$$"'.log'
  case "$line" in
    *'"type":"user"'*)
      n=$(( $(cat 'DIR/frames' 2>/dev/null || echo 0) + 1 ))
      printf '%s' "$n" > 'DIR/frames'
      cat 'DIR/turn'$n'.jsonl'
      if [ $n -eq 1 ]; then
        ( FIRST_BG ) &
      fi
      if [ $n -eq 2 ]; then
        SECOND_FG
      fi
      ;;
  esac
done
"#
    .replace("FIRST_BG", first_bg)
    .replace("SECOND_FG", second_fg)
    .replace("DIR", &dir.display().to_string());
    let bin = dir.join("claude");
    fs::write(&bin, script).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    Fake {
        bin: bin.display().to_string(),
        dir,
    }
}

// Collects events until AgentEnd for id, then one more second (a second end would show up there). Returns the events
// and whether flag existed when the AgentEnd arrived: the owned completion writes flag before it prints.
fn until_agent_end(rx: &Receiver<UiEvent>, id: &str, flag: &Path) -> (Vec<UiEvent>, bool) {
    let mut out = Vec::new();
    let mut ended: Option<(Instant, bool)> = None;
    let t0 = Instant::now();
    while t0.elapsed() < WAIT {
        if let Ok(e) = rx.recv_timeout(Duration::from_millis(100)) {
            if ended.is_none() && matches!(&e, UiEvent::AgentEnd { id: i, .. } if i == id) {
                ended = Some((Instant::now(), flag.exists()));
            }
            out.push(e);
        }
        if ended.is_some_and(|(at, _)| at.elapsed() >= Duration::from_secs(1)) {
            break;
        }
    }
    let (_, at_end) = ended.expect("agent never ended");
    (out, at_end)
}

fn agent_ends(evs: &[UiEvent], id: &str) -> usize {
    evs.iter()
        .filter(|e| matches!(e, UiEvent::AgentEnd { id: i, .. } if i == id))
        .count()
}

// An idle reply with an owned bash task still running must not end the agent (idle_result used to end the oldest).
#[test]
fn idle_reply_with_live_owned_task_does_not_end_the_agent() {
    let _g = lock();
    // The child starts an idle turn of its own (no user frame) while the owned bash runs. The owned bash ends
    // first, then the agent-level completion ends the agent.
    let idle = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Checking\"}]},\"session_id\":\"s1\",\"parent_tool_use_id\":null}\n{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"Checking\",\"session_id\":\"s1\",\"duration_ms\":3,\"total_cost_usd\":0.0}\n";
    let body = format!(
        "sleep 1; printf '%s' '{idle}'; sleep 2; touch 'DIR/owned.flag'; printf '%s' '{OWNED_DONE}'; sleep 1; printf '%s' '{AGENT_DONE}'"
    );
    let f = fake_owned(
        "owned-idle",
        [OWNED_TURN, PLAIN_REPLY, PLAIN_REPLY],
        &body,
        ":",
    );
    let flag = f.dir.join("owned.flag");
    let (h, rx) = Engine::start(config(&f));
    h.submit("scan");
    let mut evs = until_idle(&rx);
    // Watch past the idle turn (at 1 s) and before the owned completion (at 3 s).
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(2000) {
        if let Ok(e) = rx.recv_timeout(Duration::from_millis(100)) {
            evs.push(e);
        }
    }
    assert_eq!(
        agent_ends(&evs, "toolu_bg1"),
        0,
        "idle reply ended the agent: {evs:?}"
    );
    let (rest, at_end) = until_agent_end(&rx, "toolu_bg1", &flag);
    evs.extend(rest);
    assert!(
        at_end,
        "agent ended before its owned task completed: {evs:?}"
    );
    assert_eq!(agent_ends(&evs, "toolu_bg1"), 1, "{evs:?}");
    h.shutdown();
}

// Live shape: the owned task names its parent by the agent's task_id (a1), not by the agent's tool_use_id.
// The agent-level completion comes first; the agent must still wait for the owned completion, then end once.
#[test]
fn agent_completion_with_task_id_parent_waits_for_owned_task() {
    let _g = lock();
    let turn = OWNED_TURN.replace(
        "\"parent_task_id\":\"toolu_bg1\"",
        "\"parent_task_id\":\"a1\"",
    );
    let body = format!(
        "sleep 1; printf '%s' '{AGENT_DONE}'; sleep 2; touch 'DIR/owned.flag'; printf '%s' '{OWNED_DONE}'"
    );
    let f = fake_owned(
        "owned-task-id-parent",
        [&turn, PLAIN_REPLY, PLAIN_REPLY],
        &body,
        ":",
    );
    let flag = f.dir.join("owned.flag");
    let mut cfg = config(&f);
    cfg.agent_end_grace = Duration::from_secs(1);
    let (h, rx) = Engine::start(cfg);
    h.submit("scan");
    let mut evs = until_idle(&rx);
    let (rest, at_end) = until_agent_end(&rx, "toolu_bg1", &flag);
    evs.extend(rest);
    assert!(
        at_end,
        "agent ended before its task-id-parented owned task completed: {evs:?}"
    );
    assert_eq!(agent_ends(&evs, "toolu_bg1"), 1, "{evs:?}");
    h.shutdown();
}

// An agent-level completion with an owned task still running stays open until the owned completion, then ends once.
#[test]
fn agent_completion_waits_for_live_owned_task_then_ends_once() {
    let _g = lock();
    let body = format!(
        "sleep 1; printf '%s' '{AGENT_DONE}'; sleep 2; touch 'DIR/owned.flag'; printf '%s' '{OWNED_DONE}'"
    );
    let f = fake_owned(
        "owned-agent-first",
        [OWNED_TURN, PLAIN_REPLY, PLAIN_REPLY],
        &body,
        ":",
    );
    let flag = f.dir.join("owned.flag");
    let mut cfg = config(&f);
    cfg.agent_end_grace = Duration::from_secs(1);
    let (h, rx) = Engine::start(cfg);
    h.submit("scan");
    let mut evs = until_idle(&rx);
    let (rest, at_end) = until_agent_end(&rx, "toolu_bg1", &flag);
    evs.extend(rest);
    assert!(
        at_end,
        "agent-level completion ended the agent before the owned task: {evs:?}"
    );
    assert_eq!(agent_ends(&evs, "toolu_bg1"), 1, "{evs:?}");
    h.shutdown();
}

// The owned bash outlives agent_end_grace (1 s here). The agent-level completion came first, so the agent is pending,
// but the grace must not end it while the owned task still runs. It ends once, after the owned completion.
#[test]
fn agent_pending_on_live_owned_task_is_not_ended_by_grace() {
    let _g = lock();
    let body = format!(
        "sleep 1; printf '%s' '{AGENT_DONE}'; sleep 3; touch 'DIR/owned.flag'; printf '%s' '{OWNED_DONE}'"
    );
    let f = fake_owned(
        "owned-grace",
        [OWNED_TURN, PLAIN_REPLY, PLAIN_REPLY],
        &body,
        ":",
    );
    let flag = f.dir.join("owned.flag");
    let cfg = Config {
        agent_end_grace: Duration::from_secs(1),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("scan");
    let mut evs = until_idle(&rx);
    let (rest, at_end) = until_agent_end(&rx, "toolu_bg1", &flag);
    evs.extend(rest);
    assert!(
        at_end,
        "grace ended the agent while its owned task still ran: {evs:?}"
    );
    assert_eq!(agent_ends(&evs, "toolu_bg1"), 1, "{evs:?}");
    h.shutdown();
}

// Rotation with an owned task on the drained child: the agent stays open on the drained child, its agent-level
// notice comes first, then the owned completion. The "[agent done]" notice reaches the rotated main once.
#[test]
fn rotated_main_keeps_agent_open_for_owned_task_on_drained_child() {
    let _g = lock();
    let handoff = fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap();
    let body = format!(
        "sleep 1; printf '%s' '{AGENT_DONE}'; sleep 2; touch 'DIR/owned.flag'; printf '%s' '{OWNED_DONE}'"
    );
    let f = fake_owned(
        "owned-rotate",
        [OWNED_TURN, &handoff, PLAIN_REPLY],
        ":",
        &body,
    );
    let flag = f.dir.join("owned.flag");
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    cfg.agent_end_grace = Duration::from_secs(1);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    let (rest, at_end) = until_agent_end(&rx, "toolu_bg1", &flag);
    evs.extend(rest);
    assert!(at_end, "drained agent ended before its owned task: {evs:?}");
    assert_eq!(agent_ends(&evs, "toolu_bg1"), 1, "{evs:?}");
    // The completion is consumed by the drained child: the new main gets its notice as its own turn (no submit).
    let log = wait_for(&f.dir.join("stdin.log"), "[agent done] toolu_bg1");
    assert_eq!(
        log.matches("[agent done] toolu_bg1").count(),
        1,
        "completion notice not delivered exactly once: {log}"
    );
    assert!(log.contains("완료"), "{log}");
    h.shutdown();
}

// The drained child consumes the agent's completion, so the new main must get its notice and start a turn on its
// own (no user prompt). The notice used to wait in the queue for the next submit.
#[test]
fn rotated_main_gets_drained_agent_notice_without_a_submit() {
    let _g = lock();
    let handoff = fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap();
    let body = format!(
        "sleep 1; printf '%s' '{AGENT_DONE}'; sleep 2; touch 'DIR/owned.flag'; printf '%s' '{OWNED_DONE}'"
    );
    let f = fake_owned(
        "owned-rotate-auto",
        [OWNED_TURN, &handoff, PLAIN_REPLY],
        ":",
        &body,
    );
    let flag = f.dir.join("owned.flag");
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    cfg.agent_end_grace = Duration::from_secs(1);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    let (rest, at_end) = until_agent_end(&rx, "toolu_bg1", &flag);
    evs.extend(rest);
    assert!(at_end, "drained agent ended before its owned task: {evs:?}");
    assert_eq!(agent_ends(&evs, "toolu_bg1"), 1, "{evs:?}");
    let log = wait_for(&f.dir.join("stdin.log"), "[agent done] toolu_bg1");
    assert_eq!(
        log.matches("[agent done] toolu_bg1").count(),
        1,
        "new main got no notice without a submit: {log}"
    );
    h.shutdown();
}

// A drained child whose last owned child completes is ended at once (killed, reaped), so its native turn (a task
// notification, then a reply) never reaches the TUI. The agent ends once and its notice reaches the new main once.
#[test]
fn drained_child_native_turn_is_interrupted_and_discarded() {
    let _g = lock();
    // The agent must still run when the handoff (frame 2) rotates the old main, so the drain exists. The native job
    // runs in a background subshell of the old child ($$ is its pid): after the kill it is gone, so kill -0 fails and
    // the killed flag is written.
    let f = fake_owned(
        "drain_native_interrupt",
        [
            OWNED_TURN,
            &fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap(),
            PLAIN_REPLY,
        ],
        ":",
        "( sleep 1; touch 'DIR/native.flag'; cat 'DIR/native.jsonl'; sleep 2; kill -0 $$ 2>/dev/null || touch 'DIR/killed.flag' ) &",
    );
    // The owned bash b1 completes first (native.jsonl starts with it), so the agent completion is not deferred.
    fs::write(
        f.dir.join("native.jsonl"),
        format!(
            "{OWNED_DONE}{}",
            fs::read_to_string(fixture("drain_native_notice_turn.jsonl")).unwrap()
        ),
    )
    .unwrap();
    let flag = f.dir.join("native.flag");
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    cfg.agent_end_grace = Duration::from_secs(1);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    let (rest, at_end) = until_agent_end(&rx, "toolu_bg1", &flag);
    evs.extend(rest);
    assert!(
        at_end,
        "drained agent ended before its native turn: {evs:?}"
    );
    assert_eq!(agent_ends(&evs, "toolu_bg1"), 1, "{evs:?}");
    assert!(
        evs.iter()
            .all(|e| !format!("{e:?}").contains("NATIVE-REPLY")),
        "discarded native turn reached the TUI: {evs:?}"
    );
    let killed = f.dir.join("killed.flag");
    let t0 = Instant::now();
    while !killed.exists() && t0.elapsed() < WAIT {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        killed.exists(),
        "old main still alive after its child was done: {evs:?}"
    );
    let row_done = evs.iter().rev().find_map(|e| match e {
        UiEvent::AgentRows { rows } => rows.iter().find(|r| r.id == "toolu_bg1").map(|r| r.done),
        _ => None,
    });
    assert_eq!(row_done, Some(true), "agent row not done: {evs:?}");
    // Each child writes its own stdin-<pid>.log: the notice must reach exactly one child, the new main. The old main
    // (the child that got the first prompt) must not hold it.
    let t0 = Instant::now();
    let notices = loop {
        let logs = child_stdin_logs(&f.dir);
        let found = logs.iter().any(|l| l.contains("[agent done] toolu_bg1"));
        if found || t0.elapsed() > WAIT {
            break logs;
        }
        thread::sleep(Duration::from_millis(20));
    };
    let holders: Vec<&String> = notices
        .iter()
        .filter(|l| l.contains("[agent done] toolu_bg1"))
        .collect();
    assert_eq!(
        holders.len(),
        1,
        "notice not in exactly one child: {notices:?}"
    );
    assert_eq!(
        holders[0].matches("[agent done] toolu_bg1").count(),
        1,
        "new main got no single notice: {}",
        holders[0]
    );
    assert!(
        !holders[0].contains("\"first\""),
        "the killed old main got the notice: {}",
        holders[0]
    );
    h.shutdown();
}

// The drained agent's completion carries its report (task_notification handback_report.text). The new main's
// "[agent done]" notice must carry that report body, so the new main can answer from it.
#[test]
fn rotated_main_notice_carries_report_text() {
    let _g = lock();
    let handoff = fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap();
    let report = "{\"type\":\"system\",\"subtype\":\"task_notification\",\"task_id\":\"a1\",\"tool_use_id\":\"toolu_bg1\",\"status\":\"completed\",\"handback\":\"send\",\"handback_report\":{\"text\":\"DONE-ROT\"},\"session_id\":\"s1\"}\n";
    let body = format!(
        "sleep 1; printf '%s' '{report}'; sleep 2; touch 'DIR/owned.flag'; printf '%s' '{OWNED_DONE}'"
    );
    let f = fake_owned(
        "owned-rotate-report",
        [OWNED_TURN, &handoff, PLAIN_REPLY],
        ":",
        &body,
    );
    let flag = f.dir.join("owned.flag");
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    cfg.agent_end_grace = Duration::from_secs(1);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    let (rest, at_end) = until_agent_end(&rx, "toolu_bg1", &flag);
    evs.extend(rest);
    assert!(at_end, "drained agent ended before its owned task: {evs:?}");
    let log = wait_for(&f.dir.join("stdin.log"), "[agent done] toolu_bg1");
    assert!(log.contains("DONE-ROT"), "notice has no report body: {log}");
    h.shutdown();
}

// The drained child prints the agent's SubagentHandback (final report) and its completion. The done line must use the
// report, not the agent's task (no text line here), so the drain path has to record the handback too.
#[test]
fn drained_agent_done_summary_is_handback_message() {
    let _g = lock();
    let handoff = fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap();
    let handback = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"toolu_hb1\",\"name\":\"SubagentHandback\",\"input\":{\"message\":\"DONE-ROT\"}}]},\"session_id\":\"s1\",\"parent_tool_use_id\":\"toolu_bg1\"}\n";
    let body = format!(
        "sleep 3; printf '%s' '{handback}'; printf '%s' '{AGENT_DONE}'; sleep 2; touch 'DIR/owned.flag'; printf '%s' '{OWNED_DONE}'"
    );
    let f = fake_owned(
        "owned-rotate-handback",
        [OWNED_TURN, &handoff, PLAIN_REPLY],
        &body,
        ":",
    );
    let flag = f.dir.join("owned.flag");
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    // The owned completion restarts the grace; a 60 s grace would race the 60 s WAIT below. Sibling tests use 1 s.
    cfg.agent_end_grace = Duration::from_secs(1);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    let (rest, at_end) = until_agent_end(&rx, "toolu_bg1", &flag);
    evs.extend(rest);
    assert!(at_end, "drained agent ended before its owned task: {evs:?}");
    let summary = evs.iter().find_map(|e| match e {
        UiEvent::AgentEnd { id, summary, .. } if id == "toolu_bg1" => Some(summary.clone()),
        _ => None,
    });
    assert_eq!(summary.as_deref(), Some("DONE-ROT"), "{evs:?}");
    h.shutdown();
}

// Interrupt with an owned task still live: abandon_turn ends the foreground agent once and clears its pending end.
// The agent-level completion came first (pending), the interrupt comes after it, and the owned completion lands
// later. No second AgentEnd may follow.
#[test]
fn interrupt_with_live_owned_task_leaves_no_stale_pending_end() {
    let _g = lock();
    let fg_turn: String = OWNED_TURN
        .replace("\"run_in_background\":true", "\"run_in_background\":false")
        .lines()
        .filter(|l| !l.contains("\"type\":\"result\""))
        .map(|l| format!("{l}\n"))
        .collect();
    let body = format!(
        "sleep 1; printf '%s' '{AGENT_DONE}'; sleep 2; touch 'DIR/owned.flag'; printf '%s' '{OWNED_DONE}'"
    );
    let f = fake_owned(
        "owned-interrupt",
        [&fg_turn, PLAIN_REPLY, PLAIN_REPLY],
        &body,
        ":",
    );
    let flag = f.dir.join("owned.flag");
    let cfg = Config {
        interrupt_grace: Duration::from_millis(300),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("scan");
    // The agent-level completion lands at ~1 s (pending, owned task live). Interrupt after it.
    std::thread::sleep(Duration::from_millis(1500));
    h.interrupt(InterruptSource::EscKey);
    let mut evs = Vec::new();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(6) {
        if let Ok(e) = rx.recv_timeout(Duration::from_millis(100)) {
            evs.push(e);
        }
    }
    assert!(flag.exists(), "owned task never completed: {evs:?}");
    assert_eq!(agent_ends(&evs, "toolu_bg1"), 1, "{evs:?}");
    h.shutdown();
}

// A rotated agent with a live owned task is resumed by SendMessage on the new child. The resume and the idle reply on
// the old child must not end it. It ends once, after the owned completion.
#[test]
fn resumed_agent_with_live_owned_task_ends_once_after_owned_completion() {
    let _g = lock();
    // The launch result carries the agentId, so the later SendMessage resolves to toolu_bg1.
    let mut lines: Vec<&str> = OWNED_TURN.lines().collect();
    lines.insert(
        2,
        "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"toolu_bg1\",\"content\":\"Async agent launched successfully.\\nagentId: abc123\",\"is_error\":false}]},\"session_id\":\"s1\",\"parent_tool_use_id\":null}",
    );
    let turn1 = format!("{}\n", lines.join("\n"));
    let handoff = fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap();
    let resume = concat!(
        "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"toolu_sm\",\"name\":\"SendMessage\",\"input\":{\"to\":\"abc123\",\"summary\":\"recap\"}}]},\"session_id\":\"s1\",\"parent_tool_use_id\":null}\n",
        "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"toolu_sm\",\"content\":\"{\\\"success\\\":true,\\\"message\\\":\\\"Resuming agent abc123\\\",\\\"resumedAgentId\\\":\\\"abc123\\\"}\",\"is_error\":false}]},\"session_id\":\"s1\",\"parent_tool_use_id\":null}\n",
        "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Resumed\"}],\"usage\":{\"input_tokens\":1,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0,\"output_tokens\":1}},\"session_id\":\"s1\",\"parent_tool_use_id\":null}\n",
        "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"Resumed\",\"session_id\":\"s1\",\"duration_ms\":42,\"total_cost_usd\":0.01}\n",
    );
    // The old child's owned bash idles at 2 s, then completes at 6 s; the agent-level notice follows at 7 s.
    let idle = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Checking\"}]},\"session_id\":\"s1\",\"parent_tool_use_id\":null}\n{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"Checking\",\"session_id\":\"s1\",\"duration_ms\":3,\"total_cost_usd\":0.0}\n";
    let body = format!(
        "sleep 2; printf '%s' '{idle}'; sleep 4; touch 'DIR/owned.flag'; printf '%s' '{OWNED_DONE}'; sleep 1; printf '%s' '{AGENT_DONE}'"
    );
    let f = fake_owned("owned-resume", [&turn1, &handoff, resume], &body, ":");
    let flag = f.dir.join("owned.flag");
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    evs.extend(until_idle(&rx));
    h.submit("second");
    evs.extend(until_idle(&rx));
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentResumed { id, .. } if id == "toolu_bg1")),
        "agent was not resumed: {evs:?}"
    );
    let (rest, at_end) = until_agent_end(&rx, "toolu_bg1", &flag);
    evs.extend(rest);
    assert!(at_end, "resumed agent ended before its owned task: {evs:?}");
    assert_eq!(agent_ends(&evs, "toolu_bg1"), 1, "{evs:?}");
    h.shutdown();
}

// Main-stream finish without a report body (captured 261110 norot order): the owned bash completes, the agent resumes
// (task_started), its final text is plain text (no SubagentHandback), and the native task_notification carries no
// handback_report. The native notice reaches main without the body, so chaude queues one "[agent done]" notice
// with the final text. It is sent once, as a turn of its own, and the agent ends once.
#[test]
fn main_finish_without_native_body_sends_final_text_notice_once() {
    let _g = lock();
    let turn2 = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Waiting\"}],\"usage\":{\"input_tokens\":1,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0,\"output_tokens\":1}},\"session_id\":\"s1\",\"parent_tool_use_id\":null}\n{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"Waiting\",\"session_id\":\"s1\",\"duration_ms\":3,\"total_cost_usd\":0.0}\n";
    let resumed = "{\"type\":\"system\",\"subtype\":\"task_started\",\"task_id\":\"a1\",\"tool_use_id\":\"toolu_bg1\",\"task_type\":\"local_agent\",\"session_id\":\"s1\"}";
    let sidechain = "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"DONE-ROT\"}]},\"session_id\":\"s1\",\"parent_tool_use_id\":\"toolu_bg1\"}";
    let body = format!(
        "sleep 1; printf '%s\\n' '{OWNED_DONE}'; printf '%s\\n' '{resumed}'; printf '%s\\n' '{sidechain}'; sleep 1; printf '%s' '{AGENT_DONE}'"
    );
    let f = fake_owned(
        "main-notice-native-no-body",
        [OWNED_TURN, turn2, PLAIN_REPLY],
        ":",
        &body,
    );
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.drain_timeout = Duration::from_secs(60);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    h.submit("second");
    evs.extend(until_idle(&rx));
    let t0 = Instant::now();
    while agent_ends(&evs, "toolu_bg1") == 0 && t0.elapsed() < WAIT {
        evs.extend(rx.recv_timeout(Duration::from_millis(100)).ok());
    }
    // One more second: a second end or a notice turn would show up here.
    let t1 = Instant::now();
    while t1.elapsed() < Duration::from_secs(1) {
        evs.extend(rx.recv_timeout(Duration::from_millis(100)).ok());
    }
    assert_eq!(agent_ends(&evs, "toolu_bg1"), 1, "{evs:?}");
    let log = wait_for(&f.dir.join("stdin.log"), "[agent done] toolu_bg1");
    let notices: Vec<&str> = log
        .lines()
        .filter(|l| l.contains("[agent done] toolu_bg1"))
        .collect();
    assert_eq!(notices.len(), 1, "notice count: {log}");
    assert!(log.contains("DONE-ROT"), "notice has no final text: {log}");
    h.shutdown();
}

// Regression: a background agent whose completion arrives on the drained (old) child. The agent must end at once
// (AgentEnd, terminal), and its "[agent done]" notice must reach the rotated main's next prompt exactly once. The
// drain_timeout is long, so only the completion can end the agent here.
// The drained child's completion is the line Claude printed when the agent finished (captured 261109, line 356 of
// /tmp/cap-rot/rot/stdout.raw, sanitized): a system task_notification with tool_use_id and status, and no
// <task-notification> text. No other record is written.
#[test]
fn drained_agent_completion_is_delivered_once_and_terminal() {
    let _g = lock();
    let notice = "{\"type\":\"system\",\"subtype\":\"task_notification\",\"task_id\":\"a1\",\"run_id\":\"r1\",\"tool_use_id\":\"toolu_bg1\",\"status\":\"completed\",\"output_file\":\"/tmp/tasks/a1.output\",\"summary\":\"done\",\"handback\":\"send\",\"handback_report\":{\"text\":\"DONE-ROT\"},\"uuid\":\"u1\",\"session_id\":\"s1\"}\n";
    let f = fake_drain(
        "drain_complete",
        &fs::read_to_string(fixture("bg_rotate_turn.jsonl")).unwrap(),
        &fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap(),
        notice,
    );
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    // The drained child's completion ends the agent while it is still draining.
    let t0 = Instant::now();
    while !evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1"))
        && t0.elapsed() < WAIT
    {
        if let Ok(e) = rx.recv_timeout(Duration::from_millis(200)) {
            evs.push(e);
        }
    }
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1")),
        "drained agent never reached terminal state: {evs:?}"
    );
    h.submit("second");
    evs.extend(until_idle(&rx));
    let log = wait_for(&f.dir.join("stdin.log"), "[agent done] toolu_bg1");
    assert_eq!(
        log.matches("[agent done] toolu_bg1").count(),
        1,
        "completion notice not delivered exactly once: {log}"
    );
    assert!(log.contains("완료"), "{log}");
    h.shutdown();
}

// Go pollDrains (repl.go:1040-1062): when the drain closes at drain_timeout with the agent still running, the agent
// gets a "[agent done]" notice (타임아웃) in front of the next user message, and it ends (terminal, AgentEnd).
#[test]
fn drained_background_agent_times_out_with_notice_and_ends() {
    let _g = lock();
    // A drained line that is not a completion: the agent is still running when the drain closes.
    let notice = "{\"type\":\"system\",\"subtype\":\"noop\"}\n";
    let f = fake_drain(
        "drain_end",
        &fs::read_to_string(fixture("bg_rotate_turn.jsonl")).unwrap(),
        &fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap(),
        notice,
    );
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(2);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::Rotated { .. })),
        "{evs:?}"
    );
    let mut evs = Vec::new();
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    // Let the drain close at drain_timeout, then send the next user message: it carries the queued notice.
    std::thread::sleep(Duration::from_secs(3));
    // The drain closes during the sleep (on quiet or at the next submit): read its AgentEnd before the next turn.
    let t0 = Instant::now();
    while !evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1"))
        && t0.elapsed() < WAIT
    {
        if let Ok(e) = rx.recv_timeout(Duration::from_millis(200)) {
            evs.push(e);
        }
    }
    h.submit("second");
    evs.extend(until_idle(&rx));
    wait_for(&f.dir.join("stdin.log"), "[agent done] toolu_bg1");
    // Item 5: the notice must sit in the post-rotation main's own stdin (the one that got "second"), exactly once.
    let holders: Vec<String> = child_stdin_logs(&f.dir)
        .into_iter()
        .filter(|l| l.contains("[agent done] toolu_bg1"))
        .collect();
    assert_eq!(
        holders.len(),
        1,
        "notice not delivered exactly once: {holders:?}"
    );
    assert!(
        holders[0].contains("타임아웃") && holders[0].contains("second"),
        "no timed-out done notice on the current main: {}",
        holders[0]
    );
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1")),
        "timed-out drained agent did not reach terminal state: {evs:?}"
    );
    h.shutdown();
}

// Item 1: an agent launched on the first child, rotated into a drain, then resumed (SendMessage) on the new child.
// The live agent must not be settled by the drain's close: no AgentEnd and no 타임아웃 notice (the drain timeout
// passes while the resumed agent still runs). Fails before the resume-branch fix in api.rs.
#[test]
fn resumed_drained_agent_is_not_settled_by_old_drain_close() {
    let _g = lock();
    let dir = temp_dir("drain_resume");
    let resume = concat!(
        "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"toolu_sm\",\"name\":\"SendMessage\",\"input\":{\"to\":\"abc123\",\"summary\":\"recap\"}}]},\"session_id\":\"r1\",\"parent_tool_use_id\":null}\n",
        "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"toolu_sm\",\"content\":\"{\\\"success\\\":true,\\\"message\\\":\\\"Resuming agent abc123\\\",\\\"resumedAgentId\\\":\\\"abc123\\\"}\",\"is_error\":false}]},\"session_id\":\"r1\",\"parent_tool_use_id\":null}\n",
        "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Resumed\"}],\"usage\":{\"input_tokens\":1,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0,\"output_tokens\":1}},\"session_id\":\"r1\",\"parent_tool_use_id\":null}\n",
        "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"Resumed\",\"session_id\":\"r1\",\"duration_ms\":42,\"total_cost_usd\":0.01}\n",
    );
    fs::write(
        dir.join("turn1.jsonl"),
        fs::read_to_string(fixture("bg_agentid_turn.jsonl")).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("turn2.jsonl"),
        fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap(),
    )
    .unwrap();
    fs::write(dir.join("resume.jsonl"), resume).unwrap();
    fs::write(
        dir.join("notice.jsonl"),
        "{\"type\":\"system\",\"subtype\":\"noop\"}\n",
    )
    .unwrap();
    // The first child launches the agent; its handoff turn is the second frame. The new child gets the marker prompt.
    let script = r#"#!/bin/sh
n=0
while IFS= read -r line; do
  printf '%s\n' "$line" >> 'DIR/stdin.log'
  printf '%s\n' "$line" >> 'DIR/stdin-'"$$"'.log'
  case "$line" in
    *'resume-now'*) cat 'DIR/resume.jsonl' ;;
    *'"type":"user"'*)
      n=$((n + 1))
      if [ $n -eq 1 ]; then
        cat 'DIR/turn1.jsonl'
      else
        cat 'DIR/turn2.jsonl'
        sleep 1
        cat 'DIR/notice.jsonl'
      fi
      ;;
  esac
done
"#
    .replace("DIR", &dir.display().to_string());
    let bin = dir.join("claude");
    fs::write(&bin, script).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    let f = Fake {
        bin: bin.display().to_string(),
        dir,
    };
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(2);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    // The rotation finishes after Rotated (RotatedFrom, then Busy(false)); a submit before that is dropped.
    evs.extend(until_idle(&rx));
    h.submit("resume-now");
    evs.extend(until_idle(&rx));
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentResumed { id, .. } if id == "toolu_bg1")),
        "agent was not resumed: {:?} {evs:?}",
        child_stdin_logs(&f.dir)
    );
    // Outlast the drain timeout while the resumed agent still runs.
    std::thread::sleep(Duration::from_secs(3));
    while let Ok(e) = rx.recv_timeout(Duration::from_millis(200)) {
        evs.push(e);
    }
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1")),
        "old drain's close ended the resumed agent: {evs:?}"
    );
    h.submit("third");
    evs.extend(until_idle(&rx));
    let holders: Vec<String> = child_stdin_logs(&f.dir)
        .into_iter()
        .filter(|l| l.contains("[agent done] toolu_bg1"))
        .collect();
    assert!(
        holders.is_empty(),
        "resumed agent got a done notice while still running: {holders:?}"
    );
    h.shutdown();
}

// Contents of each child's own stdin log (fake_drain writes stdin-<pid>.log), one entry per child process.
fn child_stdin_logs(dir: &std::path::Path) -> Vec<String> {
    let mut logs: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("stdin-"))
        .map(|e| fs::read_to_string(e.path()).unwrap_or_default())
        .collect();
    logs.sort();
    logs
}

// Item 2: max_drain 0 means the old child is killed, not drained. Its running agent settles at once (AgentEnd,
// 타임아웃), and that notice reaches the post-rotation child exactly once.
#[test]
fn killed_child_agent_settles_and_notice_reaches_new_child() {
    let _g = lock();
    let notice = "{\"type\":\"system\",\"subtype\":\"noop\"}\n";
    let f = fake_drain(
        "kill_settle",
        &fs::read_to_string(fixture("bg_rotate_turn.jsonl")).unwrap(),
        &fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap(),
        notice,
    );
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    cfg.max_drain = 0;
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    h.submit("second");
    evs.extend(until_idle(&rx));
    let t0 = Instant::now();
    while !evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1"))
        && t0.elapsed() < WAIT
    {
        if let Ok(e) = rx.recv_timeout(Duration::from_millis(200)) {
            evs.push(e);
        }
    }
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1")),
        "agent on the killed child never reached terminal state: {evs:?}"
    );
    wait_for(&f.dir.join("stdin.log"), "[agent done] toolu_bg1");
    let holders: Vec<String> = child_stdin_logs(&f.dir)
        .into_iter()
        .filter(|l| l.contains("[agent done] toolu_bg1"))
        .collect();
    assert_eq!(
        holders.len(),
        1,
        "notice not delivered exactly once: {holders:?}"
    );
    assert!(
        holders[0].contains("타임아웃") && holders[0].contains("second"),
        "notice did not reach the post-rotation child with its prompt: {}",
        holders[0]
    );
    h.shutdown();
}

// Item 1: with max_drain 1 a second rotation while the first drain still runs evicts the oldest drain. The evicted
// drain's agent goes through the same finish path: AgentEnd, one 타임아웃 notice, delivered to the current main once.
#[test]
fn evicted_drain_agent_settles_once_and_notice_reaches_current_main() {
    let _g = lock();
    let notice = "{\"type\":\"system\",\"subtype\":\"noop\"}\n";
    let f = fake_drain(
        "drain_evict",
        &fs::read_to_string(fixture("bg_rotate_turn.jsonl")).unwrap(),
        &fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap(),
        notice,
    );
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    cfg.max_drain = 1;
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    // The first drain still runs (its agent is running): the next rotation drains the new child too.
    h.submit("second");
    evs.extend(until_idle(&rx));
    // Eviction settles the agent (AgentEnd) before rotate_child emits Rotated, so wait for both.
    let t1 = Instant::now();
    while !(evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1"))
        && evs.iter().any(|e| matches!(e, UiEvent::Rotated { .. })))
        && t1.elapsed() < WAIT
    {
        if let Ok(e) = rx.recv_timeout(Duration::from_millis(200)) {
            evs.push(e);
        }
    }
    assert!(
        evs.iter().any(|e| matches!(e, UiEvent::Rotated { .. })),
        "second rotation did not happen: {evs:?}"
    );
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1")),
        "evicted drain agent never reached terminal state: {evs:?}"
    );
    h.submit("third");
    evs.extend(until_idle(&rx));
    wait_for(&f.dir.join("stdin.log"), "[agent done] toolu_bg1");
    let holders: Vec<String> = child_stdin_logs(&f.dir)
        .into_iter()
        .filter(|l| l.contains("[agent done] toolu_bg1"))
        .collect();
    assert_eq!(
        holders.len(),
        1,
        "notice not delivered exactly once: {holders:?}"
    );
    assert!(
        holders[0].contains("타임아웃") && holders[0].contains("third"),
        "notice did not reach the current main with its prompt: {}",
        holders[0]
    );
    h.shutdown();
}

// Go drain.consume (drain.go:317-327, drain_test.go TestDrainConsumeDenies): the drained child's can_use_tool prompt
// gets a deny control_response on its stdin, and each of its lines is added to the transcript as drain_deny or drain_event.
#[test]
fn drained_child_prompt_is_denied_and_lines_are_logged() {
    let _g = lock();
    let notice = "{\"type\":\"control_request\",\"request_id\":\"r-drain\",\"request\":{\"subtype\":\"can_use_tool\"}}\n{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"<task-notification><tool-use-id>toolu_bg1</tool-use-id></task-notification>\"}}\n";
    let f = fake_drain(
        "drain_log",
        &fs::read_to_string(fixture("bg_rotate_turn.jsonl")).unwrap(),
        &fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap(),
        notice,
    );
    let trace = f.dir.join("transcript.jsonl");
    let mut cfg = Config {
        transcript: Some(trace.clone()),
        ..config(&f)
    };
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    until_idle(&rx);
    while !matches!(
        rx.recv_timeout(WAIT).expect("engine went silent"),
        UiEvent::Rotated { .. }
    ) {}
    let stdin = wait_for(&f.dir.join("stdin.log"), "r-drain");
    let deny = stdin
        .lines()
        .find(|l| l.contains("r-drain"))
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .unwrap();
    assert_eq!(deny["type"], "control_response", "{stdin}");
    assert_eq!(deny["response"]["response"]["behavior"], "deny", "{stdin}");
    let t0 = Instant::now();
    let rows = loop {
        let rows: Vec<serde_json::Value> = fs::read_to_string(&trace)
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let has = |kind: &str| rows.iter().any(|r| r["kind"] == kind);
        if (has("drain_deny") && has("drain_event")) || t0.elapsed() > WAIT {
            break rows;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(
        rows.iter().any(|r| r["kind"] == "drain_deny"
            && r["type"] == "control_request"
            && r["session"].is_string()),
        "{rows:?}"
    );
    assert!(
        rows.iter()
            .any(|r| r["kind"] == "drain_event" && r["type"] == "user" && r["session"].is_string()),
        "{rows:?}"
    );
    assert!(rows.iter().all(|r| r["t"].is_f64()), "{rows:?}");
    h.shutdown();
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

// Go markInterrupted: an agent still running at shutdown is stored as interrupted in the agents file.
#[test]
fn running_agent_is_stored_interrupted_on_shutdown() {
    let _g = lock();
    let (head, tail) = agent_turn_split();
    // The result would come after 20 s; shutdown happens while the Agent tool_use is still open.
    let f = fake_gap("agent-store-exit", &head, &tail, "20");
    let (h, rx) = Engine::start(config(&f));
    h.submit("scan");
    loop {
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
        if matches!(ev, UiEvent::AgentStart { .. }) {
            break;
        }
    }
    let agents = f.dir.join("agents.json");
    let before = wait_for(&agents, "\"running\"");
    assert!(
        before.contains("toolu_agent1") && before.contains("\"running\""),
        "{before}"
    );
    h.shutdown();
    let after = fs::read_to_string(&agents).unwrap();
    assert!(after.contains("\"interrupted\""), "{after}");
    assert!(!after.contains("\"running\""), "{after}");
    // The stored agent names its session, so /resume of that session lists it (AgentStore::load).
    assert!(!after.contains("\"parent_session\": \"\""), "{after}");
}

#[test]
fn errored_agent_is_stored_as_failed_not_done() {
    let _g = lock();
    let f = fake("agent-error", &["agent_error_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("scan");
    let evs = until_idle(&rx);
    assert!(
        evs.iter().any(|e| matches!(e,
            UiEvent::ToolEnd { id, ok: false } if id == "toolu_agent1")),
        "{evs:?}"
    );
    h.shutdown();
    let stored = fs::read_to_string(f.dir.join("agents.json")).unwrap();
    assert!(
        stored.contains("toolu_agent1") && stored.contains("\"failed\""),
        "{stored}"
    );
    assert!(!stored.contains("\"done\""), "{stored}");
}

// A fresh start lists no stored agent (the agent list starts empty); the record stays in agents.json. Only /resume
// of its session lists it (api.rs restored_events, unit-tested there).
#[test]
fn stored_agents_are_not_listed_at_fresh_start() {
    let _g = lock();
    let f = fake("agent-restore", &["text_turn.jsonl"]);
    // owner_pid 4000000 is no live process, so startup marks the stored running agent interrupted.
    fs::write(
        f.dir.join("agents.json"),
        r#"[{"id":"toolu_pre1","status":"running","task":"scan repo","summary":"found 3 files","background":true,"owner_pid":4000000,"agent_id":"ag1","started":"1970-01-01T00:00:00.000Z","updated":"1970-01-01T00:00:00.000Z"}]"#,
    )
    .unwrap();
    let (h, rx) = Engine::start(config(&f));
    h.submit("next");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter().any(|e| matches!(e,
            UiEvent::AgentsRestored { rows } if !rows.is_empty())),
        "{evs:?}"
    );
    assert!(
        !evs.iter().any(|e| matches!(e,
            UiEvent::AgentLog { id, .. } | UiEvent::AgentLogAt { id, .. } if id == "toolu_pre1")),
        "{evs:?}"
    );
    h.shutdown();
    let stored = fs::read_to_string(f.dir.join("agents.json")).unwrap();
    assert!(stored.contains("toolu_pre1"), "{stored}");
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
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
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
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
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
    let ev = rx.recv_timeout(WAIT).expect("mode report");
    assert_eq!(
        ev,
        UiEvent::ModeChanged {
            mode: "auto".into()
        }
    );
    h.shutdown();
}

// Config.perm_mode seeds the engine's start mode; the start state is not announced, so /mode with no name reports it.
#[test]
fn configured_start_mode_is_the_engine_mode() {
    let _g = lock();
    let f = fake("mode-start", &["text_turn.jsonl"]);
    let cfg = Config {
        perm_mode: chaude_core::mode::PermMode::Plan,
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.set_mode("");
    let ev = rx.recv_timeout(WAIT).expect("mode report");
    assert_eq!(
        ev,
        UiEvent::ModeChanged {
            mode: "plan".into()
        }
    );
    h.shutdown();
}

// Go askPerm with r.ui == nil: an ask that arrives between turns is denied with the Go note, not queued for a UI.
#[test]
fn plain_mode_denies_idle_permission_ask_with_notice() {
    let _g = lock();
    let f = fake("perm-idle", &["text_turn.jsonl"]);
    // The turn ends with its result; the ask follows while no turn runs.
    let turn = fs::read_to_string(fixture("text_turn.jsonl")).unwrap();
    let ask = "{\"type\":\"control_request\",\"request_id\":\"perm-idle-1\",\"request\":{\"subtype\":\"can_use_tool\",\"tool_name\":\"Bash\",\"input\":{\"command\":\"ls\"}}}\n";
    fs::write(f.dir.join("turn1.jsonl"), format!("{turn}{ask}")).unwrap();
    let cfg = Config {
        no_ui: true,
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.set_mode("default");
    h.submit("hi");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::PermissionRequest { .. })),
        "{evs:?}"
    );
    let note = loop {
        match rx.recv_timeout(WAIT).expect("idle notice") {
            UiEvent::Notice { text } => break text,
            _ => continue,
        }
    };
    assert!(
        note.starts_with(
            "[permission denied while idle] Bash {\"command\":\"ls\"} (use /mode auto"
        ),
        "{note}"
    );
    let log = wait_for(&f.dir.join("stdin.log"), "control_response");
    assert!(log.contains(r#""request_id":"perm-idle-1""#), "{log}");
    assert!(log.contains(r#""behavior":"deny""#), "{log}");
    assert!(log.contains("denied: no interactive prompt"), "{log}");
    h.shutdown();
}

#[test]
fn idle_ask_in_auto_mode_is_answered_not_left_pending() {
    let _g = lock();
    let f = fake("perm-idle-auto", &["text_turn.jsonl"]);
    // The turn ends with its result; the ask follows while no turn runs (Go TestIdleControlRequestAnswered).
    let turn = fs::read_to_string(fixture("text_turn.jsonl")).unwrap();
    let ask = "{\"type\":\"control_request\",\"request_id\":\"perm-idle-2\",\"request\":{\"subtype\":\"can_use_tool\",\"tool_name\":\"Bash\",\"input\":{\"command\":\"ls\"}}}\n";
    fs::write(f.dir.join("turn1.jsonl"), format!("{turn}{ask}")).unwrap();
    let cfg = Config {
        auto_allow: true,
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("hi");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::PermissionRequest { .. })),
        "{evs:?}"
    );
    let log = wait_for(&f.dir.join("stdin.log"), "control_response");
    assert!(log.contains(r#""request_id":"perm-idle-2""#), "{log}");
    assert!(log.contains(r#""behavior":"allow""#), "{log}");
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
    h.set_mode("default");
    h.submit("rm it");
    let mut evs = Vec::new();
    loop {
        let ev = rx.recv_timeout(WAIT).expect("permission prompt");
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
    let ev = rx.recv_timeout(WAIT).expect("reply to stale answer");
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
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
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
    // Go repl.go:1375-1377 flag keys.
    assert_eq!(rec["flags"]["threshold"], 80_000, "{text}");
    assert_eq!(rec["flags"]["handoff_validate"], true, "{text}");
    assert_eq!(rec["flags"]["max_drain"], 3, "{text}");
    assert_eq!(rec["flags"]["drain_timeout_s"], 3600, "{text}");
    assert_eq!(rec["flags"]["compact_window"], 0, "{text}");
    assert_eq!(rec["flags"]["delegation_rule"], true, "{text}");
    assert_eq!(rec["flags"]["category_rotate"], true, "{text}");
    assert!(rec["duration_ms"].is_u64(), "{text}");
    assert!(rec["cost_turn_usd"].is_f64(), "{text}");
}

#[test]
fn idle_rotation_hands_off_after_quiet_child() {
    let _g = lock();
    let f = fake("idle", &["rotate_turn.jsonl", "handoff_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.idle_rotate = Duration::from_secs(1);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    until_idle(&rx);
    // No further prompt: the child sits idle past the 1 s idle rotation and hands off on its own.
    let t0 = Instant::now();
    let reason = loop {
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
        if let UiEvent::Rotated { reason } = ev {
            break reason;
        }
        assert!(t0.elapsed() < Duration::from_secs(10), "no idle rotation");
    };
    assert_eq!(reason, "idle 1s");
    h.shutdown();
}

// Real-time check (slow, so #[ignore]): an observed 5m cache must not shorten main's idle rotation.
// User rule 261109: main idles out at idle_rotate (3570 s). Run with:
//   cargo test -p chaude-core --test engine main_survives_310s_idle_on_5m_cache -- --ignored
#[test]
#[ignore]
fn main_survives_310s_idle_on_5m_cache() {
    let _g = lock();
    let f = fake("idle310", &["text_turn_5m.jsonl", "text_turn_5m.jsonl"]);
    // config() keeps the real default idle_rotate (3570 s); nothing is overridden here.
    let (h, rx) = Engine::start(config(&f));
    h.submit("first");
    let first = until_idle(&rx);
    assert!(
        first
            .iter()
            .any(|e| matches!(e, UiEvent::SessionIds { main, .. } if main == "s1"))
    );
    let pid = opus_child_pid(&f.dir);
    // The 5m cache is observed from the first answer's usage; the idle wait below runs past 290 s.
    thread::sleep(Duration::from_secs(310));
    let quiet: Vec<UiEvent> = rx.try_iter().collect();
    assert!(
        !quiet
            .iter()
            .any(|e| matches!(e, UiEvent::Rotated { .. } | UiEvent::RotatedFrom { .. })),
        "main rotated during 310 s idle: {quiet:?}"
    );
    // Checked before the second prompt: a respawned main would otherwise hide behind a later timeout.
    let spawned = fs::read_to_string(f.dir.join("spawned.log")).unwrap_or_default();
    assert_eq!(
        spawned.lines().filter(|l| *l == "main").count(),
        1,
        "main spawned more than once after 310 s idle: {spawned}"
    );
    assert_eq!(
        opus_child_pid(&f.dir),
        pid,
        "child pid changed after 310 s idle"
    );
    h.submit("second");
    let second = until_idle(&rx);
    assert!(
        !second
            .iter()
            .any(|e| matches!(e, UiEvent::Rotated { .. } | UiEvent::RotatedFrom { .. })),
        "main rotated before second prompt: {second:?}"
    );
    // SessionIds is emitted only when the session changes, so its absence means the session stayed s1.
    assert!(
        !second
            .iter()
            .any(|e| matches!(e, UiEvent::SessionIds { main, .. } if main != "s1")),
        "session changed on second prompt: {second:?}"
    );
    h.shutdown();
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

// The text of the user frame in a stdin log line that holds needle (JSON-escaped in the log, so it is decoded).
fn user_frame_text(log: &str, needle: &str) -> String {
    let line = log
        .lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no frame with {needle}: {log}"));
    let v: serde_json::Value = serde_json::from_str(line).expect("stdin line is JSON");
    v["message"]["content"]
        .as_str()
        .unwrap_or_else(|| panic!("content is not a string: {line}"))
        .to_string()
}

// Rotation with a live agent (launched by the old main, still running when the rotation runs). The old child is drained,
// and the rotated main's first input carries the agent's id, brief and launch time, plus the old main's turns from the
// launch on, newest kept within 12000 chars.
#[test]
fn rotation_with_live_agent_carries_brief_and_capped_turns() {
    let _g = lock();
    let notice = "{\"type\":\"system\",\"subtype\":\"noop\"}\n";
    let f = fake_drain(
        "carry_live",
        &fs::read_to_string(fixture("bg_rotate_stream_turn.jsonl")).unwrap(),
        &fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap(),
        notice,
    );
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    let (h, rx) = Engine::start(cfg);
    // A long first prompt: the carried turn is longer than the cap, so only its newest part (the reply) stays.
    h.submit(&format!("first {}", "z".repeat(20_000)));
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    h.submit("second");
    evs.extend(until_idle(&rx));
    let log = wait_for(&f.dir.join("stdin.log"), "Current user message");
    let msg = user_frame_text(&log, "Current user message");
    assert!(
        msg.contains("## Agents still running from the previous session"),
        "{msg}"
    );
    assert!(msg.contains("toolu_bg1 · scan · launched at"), "{msg}");
    assert!(msg.contains("  brief:\nlook around\n"), "{msg}");
    // The turns section: the body after its header line, up to the current user message.
    let start = msg
        .find("## Previous session turns")
        .unwrap_or_else(|| panic!("no carried turns: {msg}"));
    let rest = &msg[start..];
    let body_start = rest.find(")\n").expect("turns header line") + 2;
    let body_end = rest
        .find("\n\n## Current user message")
        .expect("current user message follows");
    let body = &rest[body_start..body_end];
    assert!(
        body.chars().count() <= 12_000,
        "turns not capped: {}",
        body.chars().count()
    );
    assert!(body.contains("Assistant: Started"), "{body}");
    assert!(msg.ends_with("second"), "{msg}");
    h.shutdown();
}

// Rotation with no live agent: the rotated main gets the snapshot only, with no carried agent or turns block.
#[test]
fn rotation_without_live_agent_carries_no_block() {
    let _g = lock();
    let f = fake("carry_none", &["rotate_turn.jsonl", "handoff_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let evs = until_idle(&rx);
    assert!(evs.iter().any(|e| matches!(e, UiEvent::Rotated { .. })));
    h.submit("second");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "Current user message");
    let msg = user_frame_text(&log, "Current user message");
    assert!(
        msg.contains("Goal: finish the context rotation test"),
        "{msg}"
    );
    assert!(!msg.contains("Agents still running"), "{msg}");
    assert!(!msg.contains("Previous session turns"), "{msg}");
    h.shutdown();
}

// The "[agent done]" notice of an agent the old main launched carries that agent's brief, when it reaches the rotated main.
#[test]
fn done_notice_after_rotation_carries_brief() {
    let _g = lock();
    let notice = "{\"type\":\"system\",\"subtype\":\"task_notification\",\"task_id\":\"a1\",\"run_id\":\"r1\",\"tool_use_id\":\"toolu_bg1\",\"status\":\"completed\",\"output_file\":\"/tmp/tasks/a1.output\",\"summary\":\"done\",\"handback\":\"send\",\"handback_report\":{\"text\":\"DONE-ROT\"},\"uuid\":\"u1\",\"session_id\":\"s1\"}\n";
    let f = fake_drain(
        "done_brief",
        &fs::read_to_string(fixture("bg_rotate_turn.jsonl")).unwrap(),
        &fs::read_to_string(fixture("handoff_turn.jsonl")).unwrap(),
        notice,
    );
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_millis(300);
    cfg.drain_timeout = Duration::from_secs(60);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let mut evs = until_idle(&rx);
    loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { .. } => break,
            e => evs.push(e),
        }
    }
    let t0 = Instant::now();
    while !evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1"))
        && t0.elapsed() < WAIT
    {
        if let Ok(e) = rx.recv_timeout(Duration::from_millis(200)) {
            evs.push(e);
        }
    }
    h.submit("second");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "[agent done] toolu_bg1");
    let msg = user_frame_text(&log, "[agent done] toolu_bg1");
    assert!(msg.contains("brief: look around"), "{msg}");
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
    // The agent never reports a result, so the rotation is deferred: no Rotated event before the turn ends.
    // Only event order is checked. A stdin.log or elapsed-time check would race the 300 ms defer timer.
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::Rotated { .. })),
        "{evs:?}"
    );
    // After the defer timeout the rotation runs while idle. WAIT is a generous upper bound only.
    let mut rotated = false;
    while let Ok(e) = rx.recv_timeout(WAIT) {
        if matches!(e, UiEvent::Rotated { .. }) {
            rotated = true;
            break;
        }
    }
    assert!(rotated);
    h.shutdown();
}

// Go TestRotateKeepsPendingSnapshot (resumeshift_test.go:65): a rotation with a pending import snapshot must not
// send the handoff prompt to the child (its reply would be empty and replace the snapshot). The rotation is held
// by a background agent; the import lands while idle, so the deferred rotation runs with the snapshot pending.
#[test]
fn go_rotate_keeps_pending_snapshot() {
    let _g = lock();
    let f = fake(
        "rotate_keep",
        &["bg_rotate_turn.jsonl", "handoff_turn.jsonl"],
    );
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.rotate_defer_timeout = Duration::from_secs(2);
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::Rotated { .. })),
        "{evs:?}"
    );
    h.install_import(ClaudeImport {
        path: PathBuf::from("/home/u/.claude/projects/p/abc123.jsonl"),
        turns: vec![("earlier question".into(), "earlier answer".into())],
        files: vec!["src/lib.rs".into()],
        reco: "RECO-KEEP".into(),
        ..Default::default()
    });
    let mut rotated = false;
    while let Ok(e) = rx.recv_timeout(WAIT) {
        if matches!(e, UiEvent::Rotated { .. }) {
            rotated = true;
            break;
        }
    }
    assert!(rotated, "deferred rotation never ran");
    // The snapshot was kept: no handoff prompt reached any child.
    let stdin = fs::read_to_string(f.dir.join("stdin.log")).unwrap_or_default();
    assert!(!stdin.contains("Write a handoff note"), "{stdin}");
    h.submit("again");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "## Current user message");
    let frame = log
        .lines()
        .find(|l| l.contains("## Current user message"))
        .expect("snapshot frame");
    assert!(
        frame.contains("imported from a plain Claude Code session abc123"),
        "{frame}"
    );
    h.shutdown();
}

#[test]
fn handoff_snapshot_lists_running_background_agent() {
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
    assert!(frame.contains("## Background agents (running)"));
    // Go joins each header and its body as separate parts with a blank line (the frame is JSON, so \n is escaped).
    assert!(frame.contains(r"## Background agents (running)\n\n- scan (toolu_bg1): running"));
    // No result file is written, so no path is advertised.
    assert!(!frame.contains("result file"));
    assert!(!frame.contains("notes/runs/"));
    h.shutdown();
}

// The Ctrl+T overlay row reads type and model from the panel row of the same agent: a real-shaped Agent call with
// subagent_type and no model input gives a row named by subagent_type, on the model its type prefix names.
#[test]
fn agent_row_of_a_running_typed_agent_carries_type_and_model() {
    let _g = lock();
    let f = fake(
        "agenttype",
        &["agent_type_row_turn.jsonl", "handoff_turn.jsonl"],
    );
    let (h, rx) = Engine::start(config(&f));
    h.submit("first");
    let evs = until_idle(&rx);
    let start = evs
        .iter()
        .find_map(|e| match e {
            UiEvent::AgentStart {
                id, description, ..
            } if id == "toolu_slow1" => Some(description.clone()),
            _ => None,
        })
        .expect("AgentStart for the background launch");
    assert_eq!(start, "SLOW agent sleep 100");
    let rows = evs
        .iter()
        .rev()
        .find_map(|e| match e {
            UiEvent::AgentRows { rows } => Some(rows.clone()),
            _ => None,
        })
        .expect("AgentRows after the background launch");
    let row = rows
        .iter()
        .find(|r| r.id == "toolu_slow1")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(row.task, "SLOW agent sleep 100");
    assert_eq!(row.name, "haiku-runner", "{row:?}");
    assert_eq!(row.model, "haiku", "{row:?}");
    assert!(!row.done && row.started_at > 0, "{row:?}");
    h.shutdown();
}

#[test]
fn agent_rows_carry_panel_fields_for_a_background_launch() {
    let _g = lock();
    let f = fake(
        "agentrows",
        &["bg_agentid_turn.jsonl", "handoff_turn.jsonl"],
    );
    let (h, rx) = Engine::start(config(&f));
    h.submit("first");
    let evs = until_idle(&rx);
    // Rows follow each agent change: the launch result is the last one that matters here.
    let rows = evs
        .iter()
        .rev()
        .find_map(|e| match e {
            UiEvent::AgentRows { rows } => Some(rows.clone()),
            _ => None,
        })
        .expect("AgentRows after the background launch");
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row.id, "toolu_bg1");
    // Go agentRec: Task = description, Prompt = first line of prompt, Name = name else subagent_type.
    assert_eq!(row.task, "scan");
    assert_eq!(row.prompt, "look around");
    assert_eq!(row.name, "general");
    assert_eq!(row.model, "");
    assert!(row.background && !row.done, "{row:?}");
    assert_eq!(row.agent_id, "abc123");
    assert!(row.result_path.is_empty(), "{row:?}");
    assert!(row.started_at > 0);
    h.shutdown();
}

// An agent's row and its end carry the model its own stream ran on, never the main session's model.
#[test]
fn agent_row_model_is_the_agents_own_stream_model() {
    let _g = lock();
    let f = fake("sub_model", &["subagent_model_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let rows = evs
        .iter()
        .rev()
        .find_map(|e| match e {
            UiEvent::AgentRows { rows } => Some(rows.clone()),
            _ => None,
        })
        .expect("AgentRows for the Agent call");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].model, "claude-haiku-5-5", "{rows:?}");
    // The running row shows the stream's model before the agent ends, not the launch guess (the label must match done).
    let running = evs.iter().any(|e| match e {
        UiEvent::AgentRows { rows } => rows
            .iter()
            .any(|r| r.id == "toolu_sub1" && !r.done && r.model == "claude-haiku-5-5"),
        _ => false,
    });
    assert!(running, "running row kept the launch model: {evs:?}");
    let end_model = evs.iter().find_map(|e| match e {
        UiEvent::AgentEnd { id, model, .. } if id == "toolu_sub1" => Some(model.clone()),
        _ => None,
    });
    assert_eq!(end_model.as_deref(), Some("claude-haiku-5-5"), "{evs:?}");
    h.shutdown();
}

#[test]
fn category_switch_hides_marker_and_starts_new_session() {
    let _g = lock();
    let f = fake(
        "catswitch",
        &[
            "cat_marker_turn.jsonl",
            "cat_switch_turn.jsonl",
            "handoff_turn.jsonl",
        ],
    );
    let (h, rx) = Engine::start(config(&f));
    h.submit("first");
    let first = until_idle(&rx);
    let shown: String = first
        .iter()
        .filter_map(|e| match e {
            UiEvent::TextDelta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        shown.contains("Done with the parser."),
        "text lost: {shown}"
    );
    assert!(!shown.contains("[ca"), "marker leaked: {shown}");
    // The [cat-switch: beta] reply starts a new session after the turn (Go repl.go category rotation).
    h.submit("write release notes");
    let reason = loop {
        match rx.recv_timeout(WAIT).expect("engine went silent") {
            UiEvent::Rotated { reason } => break reason,
            UiEvent::TextDelta { text, .. } => {
                assert!(!text.contains("[cat"), "marker leaked: {text}")
            }
            _ => {}
        }
    };
    assert_eq!(reason, "category alpha -> beta");
    h.shutdown();
}

#[test]
fn resident_stage_rows_bracket_main_turn_and_agent_calls() {
    let _g = lock();
    let f = fake("stage_on", &["resume_agent_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let evs = until_idle(&rx);
    let stages: Vec<UiEvent> = evs
        .iter()
        .filter(|e| matches!(e, UiEvent::Stage { .. }))
        .cloned()
        .collect();
    let stage = |slot: &str, phase: StagePhase| UiEvent::Stage {
        slot: slot.into(),
        phase,
    };
    assert_eq!(
        stages,
        vec![
            stage("prompter", StagePhase::Working),
            stage("prompter", StagePhase::Done),
            stage("main", StagePhase::Working),
            stage("agent:general", StagePhase::Working),
            stage("agent:general", StagePhase::Done),
            stage("main", StagePhase::Done),
        ],
        "{evs:?}"
    );
    h.shutdown();
}

#[test]
fn prompter_then_tui_stage_rows_follow_go_order() {
    let _g = lock();
    let f = fake("stage_order", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("plain");
    let evs = until_idle(&rx);
    // Go TestStageLinesOrder: prompter working and done, then tui working and done, in that order.
    let rows: Vec<(String, StagePhase)> = evs
        .iter()
        .filter_map(|e| match e {
            UiEvent::Stage { slot, phase } if slot == "prompter" || slot == "tui" => {
                Some((slot.clone(), phase.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("prompter".to_string(), StagePhase::Working),
            ("prompter".to_string(), StagePhase::Done),
            ("tui".to_string(), StagePhase::Working),
            ("tui".to_string(), StagePhase::Done),
        ],
        "{evs:?}"
    );
    h.shutdown();
}

// Go TestSpawnSlotParksReadingChild (resident_route_test.go:55): a spawned resident child is read while it is idle,
// so output with no task pending is still seen. The fake prompter prints one unsolicited result at start.
#[test]
fn spawned_resident_is_read_while_idle() {
    let _g = lock();
    let f = fake("idle_read", &["text_turn.jsonl"]);
    fs::write(
        f.dir.join("prompter.idle"),
        "{\"type\":\"result\",\"session_id\":\"p-idle\"}\n",
    )
    .unwrap();
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    let deadline = std::time::Instant::now() + WAIT;
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        let ev = rx
            .recv_timeout(left)
            .unwrap_or_else(|e| panic!("no idle prompter id seen: {e}"));
        if let UiEvent::SessionIds { residents, .. } = &ev
            && residents.get("prompter").map(String::as_str) == Some("p-idle")
        {
            break;
        }
    }
    h.shutdown();
}

#[test]
fn session_ids_new_session_and_tool_input_are_emitted() {
    let _g = lock();
    let f = fake("clear", &["clear_turn.jsonl", "clear_after_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("first");
    let first = until_idle(&rx);
    assert!(
        first.contains(&UiEvent::SessionIds {
            main: "s1".into(),
            residents: BTreeMap::new(),
            owned: vec!["s1".into()],
        }),
        "{first:?}"
    );
    let input = first
        .iter()
        .position(|e| {
            *e == UiEvent::ToolInput {
                id: "toolu_bash1".into(),
                input: serde_json::json!({"command": "ls -la"}),
            }
        })
        .unwrap_or_else(|| panic!("{first:?}"));
    let start = first
        .iter()
        .position(|e| matches!(e, UiEvent::ToolStart { id, .. } if id == "toolu_bash1"))
        .unwrap();
    assert_eq!(input, start + 1, "{first:?}");
    // /clear no longer reaches the child as a turn (api.rs clear_session runs a handoff, then restarts),
    // so the NewSession event is covered by clear_command_carries_snapshot_to_a_fresh_session instead.
    h.shutdown();
}

#[test]
fn resident_result_reports_role_context_and_session() {
    let _g = lock();
    let f = fake("tui_ctx", &["text_turn.jsonl"]);
    fs::copy(fixture("resident_usage.jsonl"), f.dir.join("tui.jsonl")).unwrap();
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let evs = until_idle(&rx);
    assert!(
        evs.contains(&UiEvent::RoleContext {
            role: "tui".into(),
            tokens: 103,
        }),
        "{evs:?}"
    );
    assert!(
        evs.contains(&UiEvent::SessionIds {
            main: "s1".into(),
            residents: BTreeMap::from([("tui".to_string(), "tui-s1".to_string())]),
            owned: vec!["s1".into(), "tui-s1".into()],
        }),
        "{evs:?}"
    );
    h.shutdown();
}

#[test]
fn tui_answer_replaces_main_reply_after_turn() {
    let _g = lock();
    let f = fake("tui_on", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let evs = until_idle(&rx);
    let log = wait_for(&f.dir.join("tui.log"), "Opus main return follows");
    assert!(log.contains("Opus main return follows"), "{log}");
    let answer = evs
        .iter()
        .position(|e| {
            *e == UiEvent::TuiAnswer {
                text: "tui answer".into(),
            }
        })
        .unwrap_or_else(|| panic!("{evs:?}"));
    let turn_end = evs
        .iter()
        .position(|e| matches!(e, UiEvent::TurnEnd { .. }))
        .unwrap();
    assert!(answer > turn_end, "{evs:?}");
    h.shutdown();
}

// Go runTurn routed (repl.go): with residents the pane gets only the tui answer, never the streamed main text.
#[test]
fn residents_pane_shows_only_the_tui_answer() {
    let _g = lock();
    let f = fake("tui_on", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter().any(|e| matches!(
            e,
            UiEvent::TextDelta {
                role: Role::Main,
                ..
            }
        )),
        "{evs:?}"
    );
    assert!(
        evs.contains(&UiEvent::TuiAnswer {
            text: "tui answer".into(),
        }),
        "{evs:?}"
    );
    h.shutdown();
}

#[test]
fn tui_exit_keeps_main_reply_and_reports_notice() {
    let _g = lock();
    let f = fake("tui_fail", &["text_turn.jsonl"]);
    fs::write(f.dir.join("tui.jsonl"), "").unwrap();
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let evs = until_idle(&rx);
    assert!(
        evs.contains(&UiEvent::Notice {
            text: "haiku-tui 실패(오류: exited): 메인 원문 표시".into(),
        }),
        "{evs:?}"
    );
    assert!(
        evs.contains(&UiEvent::TextDelta {
            role: Role::Main,
            text: "Hello".into(),
        }),
        "{evs:?}"
    );
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::TuiAnswer { .. })),
        "{evs:?}"
    );
    h.shutdown();
}

// Go TestViaTUIFailureNotice (resident_pipeline_test.go:63): a tui that exits and a tui that never answers both
// keep the main reply and show the haiku-tui notice; the silent one ends at tui_timeout with the timeout notice.
#[test]
fn go_via_tui_failure_notice() {
    let _g = lock();
    let f = fake(
        "tui_failure_notice",
        &["text_turn.jsonl", "text_turn.jsonl"],
    );
    fs::write(f.dir.join("tui.jsonl"), "").unwrap();
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    cfg.tui_timeout = Duration::from_secs(1);
    let (h, rx) = Engine::start(cfg);
    // The tui exits on its first prompt: the notice is the exit failure.
    h.submit("one");
    let evs = until_idle(&rx);
    assert!(
        evs.iter().any(|e| matches!(
            e,
            UiEvent::Notice { text } if text.starts_with("haiku-tui 실패(오류")
        )),
        "{evs:?}"
    );
    // The respawned tui prints a line without a result and stays silent: the timeout notice.
    fs::write(f.dir.join("tui.jsonl"), "{\"type\":\"system\"}\n").unwrap();
    h.submit("two");
    let evs = until_idle(&rx);
    assert!(
        evs.iter().any(|e| matches!(
            e,
            UiEvent::Notice { text } if text.starts_with("haiku-tui 실패(") && text.contains("시간초과")
        )),
        "{evs:?}"
    );
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::TuiAnswer { .. })),
        "{evs:?}"
    );
    h.shutdown();
}

#[test]
fn prompter_summary_wraps_prompt_for_main_child() {
    let _g = lock();
    let f = fake("prompter_on", &["resume_agent_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let evs = until_idle(&rx);
    let stdin = wait_for(&f.dir.join("stdin.log"), "[original]");
    assert!(stdin.contains("caveman summary"), "{stdin}");
    assert!(stdin.contains("[original]"), "{stdin}");
    assert!(
        evs.contains(&UiEvent::Stage {
            slot: "prompter".into(),
            phase: StagePhase::Done,
        }),
        "{evs:?}"
    );
    h.shutdown();
}

#[test]
fn prompter_failure_falls_back_to_raw_prompt() {
    let _g = lock();
    let f = fake("prompter_fail", &["resume_agent_turn.jsonl"]);
    fs::write(f.dir.join("prompter.jsonl"), "").unwrap();
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let evs = until_idle(&rx);
    let stdin = fs::read_to_string(f.dir.join("stdin.log")).unwrap_or_default();
    assert!(!stdin.contains("[original]"), "{stdin}");
    assert!(
        evs.contains(&UiEvent::Notice {
            text: "prompter 실패(오류: exited): 원문 그대로 메인에 보냄".into(),
        }),
        "{evs:?}"
    );
    assert!(
        evs.contains(&UiEvent::Stage {
            slot: "main".into(),
            phase: StagePhase::Done,
        }),
        "{evs:?}"
    );
    h.shutdown();
}

#[test]
fn dead_prompter_slot_is_respawned_and_raw_when_respawn_fails() {
    let _g = lock();
    let f = fake(
        "prompter_respawn",
        &[
            "resume_agent_turn.jsonl",
            "resume_agent_turn.jsonl",
            "resume_agent_turn.jsonl",
        ],
    );
    fs::write(f.dir.join("prompter.jsonl"), "").unwrap();
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    let raw = UiEvent::Notice {
        text: "prompter 실패(오류: exited): 원문 그대로 메인에 보냄".into(),
    };
    // The first prompt kills the prompter child: raw prompt and the failure notice.
    h.submit("one");
    let evs = until_idle(&rx);
    assert!(evs.contains(&raw), "{evs:?}");
    // The respawned child exits again: the respawn failed, so the raw prompt goes out once more.
    h.submit("two");
    let evs = until_idle(&rx);
    assert!(evs.contains(&raw), "{evs:?}");
    // The prompter now replies: the dead slot is respawned and the summary wraps the prompt.
    fs::write(
        f.dir.join("prompter.jsonl"),
        "{\"type\":\"result\",\"result\":\"caveman summary\"}\n",
    )
    .unwrap();
    h.submit("three");
    let evs = until_idle(&rx);
    assert!(
        evs.contains(&UiEvent::Stage {
            slot: "prompter".into(),
            phase: StagePhase::Done,
        }),
        "{evs:?}"
    );
    let stdin = wait_for(&f.dir.join("stdin.log"), "caveman summary");
    assert!(stdin.contains("[original]"), "{stdin}");
    h.shutdown();
}

#[test]
fn residents_spawn_eagerly_with_main_four_slots() {
    let _g = lock();
    let f = fake("eager_slots", &["resume_agent_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, _rx) = Engine::start(cfg);
    // Go residentSlots: main, prompter, tui and coder start together, before any prompt.
    let t0 = Instant::now();
    loop {
        let n = fs::read_to_string(f.dir.join("spawned.log"))
            .unwrap_or_default()
            .lines()
            .count();
        if n >= 4 || t0.elapsed() > Duration::from_secs(10) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    // A fifth spawn would show up within this grace; none may.
    thread::sleep(Duration::from_millis(300));
    let spawned = fs::read_to_string(f.dir.join("spawned.log")).unwrap_or_default();
    let mut roles: Vec<&str> = spawned.lines().collect();
    roles.sort_unstable();
    assert_eq!(roles, ["coder", "main", "prompter", "tui"], "{spawned}");
    h.shutdown();
}

// Go TestResidentSlotsFour (resident_test.go:44), driven through the engine: four slots start, main on cfg.model,
// prompter, tui and coder on haiku.
#[test]
fn resident_slots_fixed_four() {
    let _g = lock();
    let f = fake("slots_four", &["resume_agent_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.model = "opus".to_string();
    cfg.cwd = Some(f.dir.clone());
    let (h, _rx) = Engine::start(cfg);
    // Each spawn writes its own role_args.<pid>.log; the lines are joined here.
    let role_args = |dir: &Path| -> String {
        let mut files: Vec<PathBuf> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("role_args."))
            })
            .collect();
        files.sort();
        files
            .iter()
            .map(|p| fs::read_to_string(p).unwrap_or_default())
            .collect()
    };
    let t0 = Instant::now();
    loop {
        let n = role_args(&f.dir).lines().count();
        if n >= 4 || t0.elapsed() > Duration::from_secs(10) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    // A fifth spawn would show up within this grace; none may.
    thread::sleep(Duration::from_millis(300));
    let text = role_args(&f.dir);
    let mut roles: Vec<&str> = text.lines().filter_map(|l| l.split(' ').next()).collect();
    roles.sort_unstable();
    assert_eq!(roles, ["coder", "main", "prompter", "tui"], "{text}");
    for line in text.lines() {
        let (role, args) = line.split_once(' ').unwrap();
        let want = if role == "main" { "opus" } else { "haiku" };
        let toks: Vec<&str> = args.split_whitespace().collect();
        assert!(
            toks.windows(2).any(|w| w == ["--model", want]),
            "{role} not on {want}: {args}"
        );
    }
    h.shutdown();
}

// Go TestAgentPromptMissingNotice (resident_agent_test.go:37): with no agent file under cwd or HOME, the slot
// shows the Korean notice and still starts as a default session.
#[test]
fn residents_without_agent_files_notice_and_start_default_session() {
    let _g = lock();
    let f = fake("agent_missing", &["text_turn.jsonl"]);
    let empty = f.dir.join("empty");
    fs::create_dir_all(&empty).unwrap();
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(empty.clone());
    // The temp dir stands in for HOME: no process env is mutated, so parallel tests never see it.
    cfg.home = Some(empty.clone());
    let (h, rx) = Engine::start(cfg);
    let want = "에이전트 규칙 없음(haiku-tui): 기본 세션으로 시작";
    let mut notices = Vec::new();
    let found = loop {
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(UiEvent::Notice { text }) if text == want => break true,
            Ok(UiEvent::Notice { text }) => notices.push(text),
            Ok(_) => {}
            Err(_) => break false,
        }
    };
    // The fake labels a role by a marker in its rule, so the tui slot with no rule is logged as "main".
    // Its spawn is the second "main" line, next to the main session.
    let t0 = Instant::now();
    let mut spawned = String::new();
    while t0.elapsed() < Duration::from_secs(10) {
        spawned = fs::read_to_string(f.dir.join("spawned.log")).unwrap_or_default();
        if spawned.lines().filter(|l| *l == "main").count() >= 2 && spawned.contains("prompter") {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    h.shutdown();
    assert!(found, "no missing-agent notice: {notices:?}");
    assert!(
        spawned.lines().filter(|l| *l == "main").count() >= 2 && spawned.contains("prompter"),
        "default sessions did not start: {spawned}"
    );
}

#[test]
fn no_stage_rows_without_residents() {
    let _g = lock();
    let f = fake("stage_off", &["resume_agent_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    assert!(
        !evs.iter().any(|e| matches!(e, UiEvent::Stage { .. })),
        "{evs:?}"
    );
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
fn handback_launch_agentid_lets_sendmessage_resume_show_a_panel_row() {
    // Real a493e252 lines 50, 55, 72, 73, 88: the handback launch result has "agentId:" but no async mark.
    let _g = lock();
    let f = fake(
        "handback_sm",
        &["resume_handback_agentid_sendmessage.jsonl"],
    );
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let launch = "toolu_01NkA89CjWG7xL4D21B2xLoj";
    let resumed = evs.iter().position(|e| {
        matches!(e, UiEvent::AgentResumed { id, send_id, .. }
            if id == launch && send_id == "toolu_01ExYmkwiS5pQLaiwyAWHeY9")
    });
    assert!(resumed.is_some(), "{evs:?}");
    let panel = evs[resumed.unwrap()..].iter().find_map(|e| match e {
        UiEvent::AgentRows { rows } => Some(rows.iter().any(|r| r.id == launch)),
        _ => None,
    });
    assert_eq!(panel, Some(true), "no panel row for resumed agent: {evs:?}");
    h.shutdown();
}

#[test]
fn resumed_agent_ends_by_notice_naming_its_sendmessage_id() {
    // The real resume's task-notification names the SendMessage tool id, not the Agent tool id.
    let _g = lock();
    let f = fake("resume_sm", &["resume_notice_names_sendmessage.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let ended = evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_bg1"));
    assert!(ended, "{evs:?}");
    h.shutdown();
}

// Trimmed from a real queued SendMessage ("Message queued for delivery", no resumedAgentId): the row opens at once
// and closes on the agent's notification, which names the launch tool id.
#[test]
fn queued_send_opens_a_row_until_the_agent_notifies() {
    let _g = lock();
    let f = fake("queued_sm", &["queued_send_closes_on_notice.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let resumed = evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentResumed { id, .. } if id == "toolu_q1"));
    let ended = evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_q1"));
    assert!(resumed && ended, "{evs:?}");
    // The SendMessage call (toolu_qsm) has no ToolEnd of its own: AgentResumed names it as the row that stays open.
    let send_ends = evs
        .iter()
        .any(|e| matches!(e, UiEvent::ToolEnd { id, .. } if id == "toolu_qsm"));
    assert!(!send_ends, "{evs:?}");
    let names_send = evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentResumed { send_id, .. } if send_id == "toolu_qsm"));
    assert!(names_send, "{evs:?}");
    h.shutdown();
}

// Real resume of an agent launched outside this process (no launch in this run): the resumed id keys the row,
// so the SendMessage call opens it and the agent's notice closes it.
#[test]
fn resume_of_an_agent_launched_elsewhere_still_opens_a_row() {
    let _g = lock();
    let f = fake(
        "resume_unknown",
        &["resume_unknown_launch_sendmessage.jsonl"],
    );
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let ab = "ab5377649aa7c5a8f";
    let resumed = evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentResumed { id, send_id, .. } if id == ab && send_id == "toolu_sm"));
    assert!(resumed, "{evs:?}");
    let ended = evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == ab));
    assert!(ended, "{evs:?}");
    let send_ends = evs
        .iter()
        .any(|e| matches!(e, UiEvent::ToolEnd { id, .. } if id == "toolu_sm"));
    assert!(!send_ends, "{evs:?}");
    h.shutdown();
}

// Real resume (success, resumedAgentId): the SendMessage call's ToolEnd is not sent; AgentResumed carries its id.
#[test]
fn resuming_send_names_its_call_in_agent_resumed_instead_of_tool_end() {
    let _g = lock();
    let f = fake("resume_sm_row", &["resume_notice_names_sendmessage.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let names_send = evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentResumed { id, send_id, .. } if id == "toolu_bg1" && send_id == "toolu_sm"));
    assert!(names_send, "{evs:?}");
    let send_ends = evs
        .iter()
        .any(|e| matches!(e, UiEvent::ToolEnd { id, .. } if id == "toolu_sm"));
    assert!(!send_ends, "{evs:?}");
    h.shutdown();
}

// Trimmed from a real SendMessage handback: the agent's <task-notification> arrives while the turn is idle,
// then main speaks on its own. With residents on, that idle reply must still reach the screen (Go idleText).
#[test]
fn idle_handback_reply_is_shown_with_residents_on() {
    let _g = lock();
    let f = fake("idle_handback", &["idle_handback_main_reply.jsonl"]);
    let cfg = Config {
        residents: true,
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let mut evs = until_idle(&rx);
    let shown = |evs: &[UiEvent]| {
        evs.iter()
            .any(|e| matches!(e, UiEvent::MessageDone { role: Role::Main }))
    };
    while !shown(&evs) {
        evs.push(rx.recv_timeout(WAIT).expect("engine went silent"));
    }
    assert!(
        evs.iter().any(|e| matches!(
            e,
            UiEvent::TextDelta { role: Role::Main, text } if text == "Agent reported back"
        )),
        "{evs:?}"
    );
    h.shutdown();
}

#[test]
fn idle_reply_without_notice_finishes_oldest_background_agent() {
    let _g = lock();
    let f = fake("bg_idle", &["bg_idle_reply_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let mut evs = until_idle(&rx);
    // The child speaks up on its own after the turn: the reply has no notice, so the oldest background agent ends.
    let ended = |evs: &[UiEvent]| {
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_idle1"))
    };
    while !ended(&evs) {
        evs.push(rx.recv_timeout(WAIT).expect("engine went silent"));
    }
    // Go TestIdleTickRefreshesWhenBackgroundFinishes (panel_test.go:55) needs the panel to show the
    // agent done with no user input. Rust pushes rows as UiEvent::AgentRows right after AgentEnd.
    let done = |evs: &[UiEvent]| {
        evs.iter()
            .rev()
            .find_map(|e| match e {
                UiEvent::AgentRows { rows } => Some(rows),
                _ => None,
            })
            .and_then(|rows| rows.iter().find(|r| r.id == "toolu_idle1"))
            .is_some_and(|r| r.done)
    };
    while !done(&evs) {
        evs.push(rx.recv_timeout(WAIT).expect("engine went silent"));
    }
    // The list the TUI tick re-reads (App::set_agent_source) holds the same finished row.
    assert!(
        h.agent_rows()
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.id == "toolu_idle1" && r.done)
    );
    h.shutdown();
}

#[test]
fn text_block_notice_finishes_background_agent() {
    let _g = lock();
    let f = fake("bg_text_block", &["bg_text_block_notice_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    // A task-notification sent as a text block (not string content) ends the named agent.
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_tb1")),
        "{evs:?}"
    );
    h.shutdown();
}

// Go TestUserEventNotificationFinishesBackground (panel_test.go:384): a string-content
// <task-notification> in a user event finishes the background agent it names.
#[test]
fn user_event_notification_finishes_background() {
    let _g = lock();
    let f = fake("bg_string_notice", &["bg_string_notice_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_sn1")),
        "{evs:?}"
    );
    // The row the panel shows must be done, not still running.
    let rows = evs.iter().rev().find_map(|e| match e {
        UiEvent::AgentRows { rows } => Some(rows),
        _ => None,
    });
    let done = rows.and_then(|rows| rows.iter().find(|r| r.id == "toolu_sn1"));
    assert!(done.is_some_and(|r| r.done), "{evs:?}");
    h.shutdown();
}

#[test]
fn resume_after_agent_ended_keeps_start_label() {
    let _g = lock();
    let f = fake("resume_late", &["resume_after_end_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    // The task-notification ended the agent and dropped its live info; the resume reuses the start label.
    assert!(
        evs.iter().any(|e| {
            matches!(e, UiEvent::AgentEnd { id, model, .. } if id == "toolu_bg2" && model.is_empty())
        }),
        "{evs:?}"
    );
    assert!(
        evs.iter().any(|e| {
            matches!(e, UiEvent::AgentResumed { id, description, .. } if id == "toolu_bg2" && description == "scan")
        }),
        "{evs:?}"
    );
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
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
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

// Collects events until a coder reply (or a failed coder stage) arrives.
fn until_coder_reply(rx: &Receiver<UiEvent>) -> Vec<UiEvent> {
    let mut out = Vec::new();
    loop {
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
        let done = matches!(ev, UiEvent::CoderReply { .. })
            || matches!(&ev, UiEvent::Stage { slot, phase: StagePhase::Failed(_) } if slot.starts_with("coder"));
        out.push(ev);
        if done {
            return out;
        }
    }
}

#[test]
fn coder_task_emits_working_done_rows_and_reply() {
    let _g = lock();
    let f = fake("coder_on", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.coder("rename the helper");
    let evs = until_coder_reply(&rx);
    let working = evs.iter().position(|e| {
        *e == UiEvent::Stage {
            slot: "coder".into(),
            phase: StagePhase::Working,
        }
    });
    let reply = evs.iter().position(|e| {
        matches!(e, UiEvent::CoderReply { role, text }
            if role == "coder" && text == "status: done\nchanged: a.rs: edit")
    });
    let done = evs.iter().position(|e| {
        *e == UiEvent::Stage {
            slot: "coder".into(),
            phase: StagePhase::Done,
        }
    });
    assert!(
        working.is_some() && reply.is_some() && done.is_some(),
        "{evs:?}"
    );
    assert!(working < done && done < reply, "{evs:?}");
    h.shutdown();
}

#[test]
fn idle_extra_coder_expires_and_base_coder_stays() {
    let _g = lock();
    let f = fake("coder_idle", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    cfg.coder_idle_ttl = Duration::from_millis(300);
    let (h, rx) = Engine::start(cfg);
    // Back to back, so the second task finds slot 0 busy and takes slot 1 (coder2).
    h.coder("first");
    h.coder("second");
    let mut replies = 0;
    while replies < 2 {
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
        if matches!(ev, UiEvent::CoderReply { .. }) {
            replies += 1;
        }
    }
    let pids_file = f.dir.join("coder.pids");
    wait_for(&pids_file, " base");
    let log = wait_for(&pids_file, " extra");
    let pid_of = |tag: &str| {
        log.lines()
            .find_map(|l| l.strip_suffix(tag).map(|p| p.trim().to_string()))
            .unwrap_or_else(|| panic!("{tag} pid missing: {log}"))
    };
    let base = pid_of(" base");
    let extra = pid_of(" extra");
    let alive = |pid: &str| {
        process::Command::new("kill")
            .args(["-0", pid])
            .status()
            .unwrap()
            .success()
    };
    let t0 = Instant::now();
    while alive(&extra) && t0.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(!alive(&extra), "extra coder not expired");
    assert!(alive(&base), "base coder must stay");
    h.shutdown();
}

#[test]
fn exit_handoff_records_main_session_on_shutdown() {
    let _g = lock();
    let f = fake("handoff_on", &["text_turn.jsonl"]);
    let path = f.dir.join(".exit-handoff.json");
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.handoff_path = Some(path.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    until_idle(&rx);
    h.shutdown();
    let text = fs::read_to_string(&path).expect("handoff written");
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["version"], 1, "{text}");
    assert_eq!(v["main_session"], "s1", "{text}");
    assert_eq!(v["cwd"], f.dir.display().to_string().as_str(), "{text}");
}

// A dead harness whose claude child still runs is reported at the next start (Go reconcileStart "[orphan]").
// The owner pid is a reaped child, so it is dead; the test process stands in for a live orphan.
#[test]
fn orphan_notice_for_alive_child_of_dead_owner() {
    let _g = lock();
    let f = fake("orphan_notice", &["text_turn.jsonl"]);
    let path = f.dir.join(".exit-handoff.json");
    let mut dead = process::Command::new("true").spawn().expect("spawn true");
    dead.wait().expect("reap true");
    let live = handoff::ExitHandoff {
        main_session: "old1".into(),
        owner_pid: dead.id(),
        child_pids: BTreeMap::from([
            ("main".to_string(), process::id()),
            ("coder".to_string(), dead.id()),
        ]),
        child_models: BTreeMap::from([
            ("main".to_string(), "opus".to_string()),
            ("coder".to_string(), "haiku".to_string()),
        ]),
        ..Default::default()
    };
    handoff::write_exit_handoff(&handoff::live_handoff_file(&path), &live).unwrap();
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.handoff_path = Some(path.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let evs = until_idle(&rx);
    h.shutdown();
    let orphans: Vec<&UiEvent> = evs
        .iter()
        .filter(|e| matches!(e, UiEvent::OrphanNotice { .. }))
        .collect();
    assert_eq!(
        orphans,
        vec![&UiEvent::OrphanNotice {
            role: "main".into(),
            pid: process::id(),
            model: "opus".into(),
        }],
        "{evs:?}"
    );
}

// A run that was killed leaves its live marker behind; the next start resumes its main session.
#[test]
fn killed_previous_run_starts_main_with_resume() {
    let _g = lock();
    let f = fake("resume_killed", &["text_turn.jsonl"]);
    let path = f.dir.join(".exit-handoff.json");
    let live = handoff::ExitHandoff {
        main_session: "old1".into(),
        ctx_tokens: BTreeMap::from([("main".to_string(), 250_000u64)]),
        ..Default::default()
    };
    handoff::write_exit_handoff(&handoff::live_handoff_file(&path), &live).unwrap();
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.handoff_path = Some(path.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    until_idle(&rx);
    h.shutdown();
    let args = fs::read_to_string(f.dir.join("args.log")).expect("main child ran");
    assert!(args.contains("--resume old1"), "{args}");
    assert!(!handoff::live_handoff_file(&path).exists());
}

// A clean exit with the main context at or above the threshold keeps the fresh-session handoff path.
#[test]
fn clean_exit_above_threshold_starts_fresh() {
    let _g = lock();
    let f = fake("fresh_above", &["text_turn.jsonl"]);
    let path = f.dir.join(".exit-handoff.json");
    let prev = handoff::ExitHandoff {
        main_session: "old2".into(),
        ctx_tokens: BTreeMap::from([("main".to_string(), 250_000u64)]),
        ..Default::default()
    };
    handoff::write_exit_handoff(&path, &prev).unwrap();
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.handoff_path = Some(path.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    until_idle(&rx);
    h.shutdown();
    let args = fs::read_to_string(f.dir.join("args.log")).expect("main child ran");
    assert!(!args.contains("--resume"), "{args}");
}

#[test]
fn failed_coder_task_still_hands_back_a_reply() {
    let _g = lock();
    let f = fake("coder_fail", &["text_turn.jsonl"]);
    // An empty result is a failed task ("coder empty reply"); main must still get it as a reply.
    fs::write(
        f.dir.join("coder.jsonl"),
        "{\"type\":\"result\",\"result\":\"\",\"session_id\":\"c1\"}\n",
    )
    .unwrap();
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.coder("task");
    let evs = until_coder_reply(&rx);
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::CoderReply { text, .. } if text.starts_with("오류:"))),
        "{evs:?}"
    );
    h.shutdown();
}

#[test]
fn idle_resident_exits_then_resumes_the_same_session() {
    let _g = lock();
    let f = fake("coder_exit", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    // The base coder has its own 1 h limit (base_idle_exit_secs); shorten it so the test exits quickly.
    cfg.base_idle_exit_secs = 1;
    let (h, rx) = Engine::start(cfg);
    h.coder("first");
    until_coder_reply(&rx);
    let pids_file = f.dir.join("coder.pids");
    let log = wait_for(&pids_file, " base");
    let pid = log
        .lines()
        .find_map(|l| l.strip_suffix(" base"))
        .unwrap()
        .trim()
        .to_string();
    let alive = |pid: &str| {
        process::Command::new("kill")
            .args(["-0", pid])
            .status()
            .unwrap()
            .success()
    };
    let t0 = Instant::now();
    while alive(&pid) && t0.elapsed() < Duration::from_secs(10) {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(!alive(&pid), "idle resident child was not exited");
    // The next task respawns the slot with --resume on the session the first reply carried (c1).
    h.coder("first");
    until_coder_reply(&rx);
    let pids = fs::read_to_string(&pids_file).unwrap();
    assert_eq!(pids.matches(" base").count(), 2, "{pids}");
    let resumed = fs::read_dir(&f.dir).unwrap().any(|e| {
        fs::read_to_string(e.unwrap().path())
            .is_ok_and(|s| s.starts_with("coder ") && s.contains("--resume c1"))
    });
    assert!(resumed, "respawn did not resume session c1");
    h.shutdown();
}

// The base coder spawns at boot and its idle timer starts there, so it exits with no task ever sent.
#[test]
fn base_coder_idle_timer_starts_at_spawn() {
    let _g = lock();
    let f = fake("coder_boot_idle", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    cfg.base_idle_exit_secs = 1;
    let (h, _rx) = Engine::start(cfg);
    let log = wait_for(&f.dir.join("coder.boot.pids"), " boot");
    let pid = log
        .lines()
        .find_map(|l| l.strip_suffix(" boot"))
        .expect("boot coder pid")
        .trim()
        .to_string();
    let alive = |pid: &str| {
        process::Command::new("kill")
            .args(["-0", pid])
            .status()
            .unwrap()
            .success()
    };
    let t0 = Instant::now();
    while alive(&pid) && t0.elapsed() < Duration::from_secs(10) {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(!alive(&pid), "boot base coder was not exited while idle");
    h.shutdown();
}

#[test]
fn exit_handoff_lists_resident_coder_and_owned_sessions() {
    let _g = lock();
    let f = fake("handoff_res", &["text_turn.jsonl"]);
    let path = f.dir.join(".exit-handoff.json");
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    cfg.handoff_path = Some(path.clone());
    let (h, rx) = Engine::start(cfg);
    h.coder("rename the helper");
    until_coder_reply(&rx);
    // The coder stays alive (idle) until shutdown, so its session is live in the handoff.
    h.shutdown();
    let got = handoff::read_exit_handoff(&path).expect("handoff readable");
    assert_eq!(got.version, 1);
    assert_eq!(
        got.resident_sessions.get("coder").map(String::as_str),
        Some("c1")
    );
    assert!(got.owned_sessions.iter().any(|s| s == "c1"), "{got:?}");
    assert!(got.prev_owned().contains("c1"));
}

#[test]
fn mcp_coder_call_blocks_for_summary_and_rejects_empty_task() {
    let _g = lock();
    let f = fake("mcp_call", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, _rx) = Engine::start(cfg);
    let caller = h.coder_caller();
    let got = caller.call("rename the helper");
    assert!(got.is_ok(), "{got:?}");
    assert!(!got.unwrap().is_empty());
    assert_eq!(caller.call("   "), Err("task is required".to_string()));
    h.shutdown();
}

// The mcp.json temp dir is owned by the engine: it exists while residents run and is removed on shutdown.
#[test]
fn mcp_config_dir_is_removed_on_shutdown() {
    let _g = lock();
    let f = fake("mcp_dir", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    // Config dirs live under cwd's notes/tmp/test; the socket sits in the short system temp dir.
    let short =
        std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("m{}", process::id()));
    cfg.cwd = Some(short.clone());
    let before = mcp_dirs(&short);
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    until_idle(&rx);
    assert_eq!(
        mcp_dirs(&short).len(),
        before.len() + 1,
        "{:?}",
        mcp_dirs(&short)
    );
    h.shutdown();
    assert_eq!(mcp_dirs(&short), before);
}

// A finished turn is saved to the record folder with its prompt and measurement line (Go recordSave).
#[test]
fn finished_turn_is_saved_to_record_folder() {
    let _g = lock();
    let f = fake("record_turn", &["record_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("say hi");
    until_idle(&rx);
    h.shutdown();
    let mut text = String::new();
    for p in files_under(&f.dir.join("record")) {
        text.push_str(&fs::read_to_string(p).unwrap());
    }
    assert!(text.contains("say hi"), "{text}");
    assert!(text.contains("[측정]"), "{text}");
}

// A /clear splits the record: the turn after it goes to a new file (Go recordSave split).
#[test]
fn clear_splits_the_next_saved_turn() {
    let _g = lock();
    let f = fake(
        "record_split",
        &[
            "record_turn.jsonl",
            "record_turn.jsonl",
            "record_turn.jsonl",
        ],
    );
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("say hi");
    until_idle(&rx);
    h.submit("/clear");
    until_idle(&rx);
    h.submit("say hi");
    until_idle(&rx);
    h.shutdown();
    let files = files_under(&f.dir.join("record"))
        .into_iter()
        .filter(|p| fs::read_to_string(p).unwrap_or_default().contains("say hi"))
        .count();
    assert_eq!(files, 2, "the turn after /clear starts a new record file");
}

// Go /rotate (rotate("manual")): a handoff runs on the current child and the fresh child gets the snapshot.
#[test]
fn rotate_command_hands_off_to_a_fresh_session() {
    let _g = lock();
    let f = fake("rotate_cmd", &["rotate_turn.jsonl", "handoff_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("first");
    until_idle(&rx);
    h.submit("/rotate");
    let evs = until_idle(&rx);
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::Rotated { reason } if reason == "manual")),
        "/rotate reports a manual rotation"
    );
    h.submit("again");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "## Current user message");
    assert!(log.contains("Write a handoff note"), "{log}");
    assert!(
        !log.contains("/rotate"),
        "/rotate is not sent to claude: {log}"
    );
    let frame = log
        .lines()
        .find(|l| l.contains("## Current user message"))
        .expect("snapshot frame");
    assert!(
        frame.contains("Goal: finish the context rotation test"),
        "{frame}"
    );
    assert!(frame.contains("again"), "{frame}");
    h.shutdown();
}

// Go /model NAME (runTUI): a rotation with reason "model NAME"; the fresh child starts on NAME, the command is not sent.
#[test]
fn model_command_rotates_to_the_new_model() {
    let _g = lock();
    let f = fake("model_cmd", &["rotate_turn.jsonl", "handoff_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("first");
    until_idle(&rx);
    h.submit("/model sonnet");
    let evs = until_idle(&rx);
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::Rotated { reason } if reason == "model sonnet")),
        "/model reports a rotation with its reason: {evs:?}"
    );
    h.submit("again");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "## Current user message");
    assert!(log.contains("Write a handoff note"), "{log}");
    assert!(
        !log.contains("/model"),
        "/model is not sent to claude: {log}"
    );
    let args = fs::read_to_string(f.dir.join("args.log")).expect("main child ran");
    let last = args.lines().last().unwrap_or_default();
    assert!(
        last.contains("--model sonnet"),
        "fresh child runs sonnet: {args}"
    );
    h.shutdown();
}

// Go /clear (clearWithHandoff): the snapshot of the conversation goes to a fresh session, not to the old one.
#[test]
fn clear_command_carries_snapshot_to_a_fresh_session() {
    let _g = lock();
    let f = fake("clear_cmd", &["rotate_turn.jsonl", "handoff_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("first");
    until_idle(&rx);
    h.submit("/clear");
    until_idle(&rx);
    h.submit("again");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "## Current user message");
    assert!(log.contains("Write a handoff note"), "{log}");
    assert!(
        !log.contains("/clear"),
        "/clear is not sent to claude: {log}"
    );
    let frame = log
        .lines()
        .find(|l| l.contains("## Current user message"))
        .expect("snapshot frame");
    assert!(
        frame.contains("Goal: finish the context rotation test"),
        "{frame}"
    );
    assert!(frame.contains("again"), "{frame}");
    h.shutdown();
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(files_under(&p));
        } else {
            out.push(p);
        }
    }
    out
}

// chaude-mcp-<pid>-* dirs in the temp dir, this process only.
fn mcp_dirs(cwd: &std::path::Path) -> Vec<PathBuf> {
    let prefix = format!("{}-", process::id());
    let mut out: Vec<PathBuf> = fs::read_dir(chaude_core::tmp_layout::test_dir(cwd))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix))
        })
        .collect();
    out.sort();
    out
}

// EngineHandle::resume_session relaunches the idle main child with --resume and records the ids it ran.
#[test]
fn resume_session_relaunches_main_with_resume_flag() {
    let _g = lock();
    let f = fake("resume_mid_run", &["text_turn.jsonl"]);
    let path = f.dir.join(".exit-handoff.json");
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.handoff_path = Some(path.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    until_idle(&rx);
    h.resume_session("old9");
    let mut got = Vec::new();
    loop {
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
        let done = matches!(ev, UiEvent::SessionResumed { .. });
        got.push(ev);
        if done {
            break;
        }
    }
    assert!(
        got.contains(&UiEvent::SessionResumed {
            old_session: "s1".into(),
            session_id: "old9".into(),
        }),
        "{got:?}"
    );
    let args = wait_for(&f.dir.join("args.log"), "--resume old9");
    assert!(args.contains("--resume old9"), "{args}");
    h.shutdown();
    let logged = handoff::read_sessions(&path);
    assert!(
        logged
            .iter()
            .any(|r| r.session_id == "s1" && r.role == handoff::MAIN_ROLE),
        "{logged:?}"
    );
}

// An empty session id is refused with an Error and the engine keeps running.
#[test]
fn resume_session_rejects_empty_id() {
    let _g = lock();
    let f = fake("resume_empty", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.resume_session("");
    let ev = rx.recv_timeout(WAIT).expect("engine went silent");
    assert!(matches!(ev, UiEvent::Error(_)), "{ev:?}");
    h.shutdown();
}

// A resident's session id from its result line is logged to the record folder's session file, once per change.
#[test]
fn resident_session_id_is_logged_to_session_file() {
    let _g = lock();
    let f = fake("resident_sid", &["text_turn.jsonl"]);
    fs::write(
        f.dir.join("prompter.jsonl"),
        "{\"type\":\"result\",\"result\":\"caveman summary\",\"session_id\":\"pr7\"}\n",
    )
    .unwrap();
    let path = f.dir.join(".exit-handoff.json");
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    cfg.residents = true;
    cfg.handoff_path = Some(path.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let evs = until_idle(&rx);
    assert!(
        evs.iter().any(|e| matches!(
            e,
            UiEvent::SessionIds { residents, .. } if residents.get("prompter").map(String::as_str) == Some("pr7")
        )),
        "{evs:?}"
    );
    h.shutdown();
    let logged = handoff::read_sessions(&path);
    assert!(
        logged
            .iter()
            .any(|r| r.session_id == "pr7" && r.role == "prompter"),
        "{logged:?}"
    );
}

// A resume request while a turn runs is refused with an Error; the running turn finishes unchanged.
#[test]
fn resume_session_is_refused_while_busy() {
    let _g = lock();
    let f = fake("resume_busy", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    loop {
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
        if ev == UiEvent::Busy(true) {
            break;
        }
    }
    h.resume_session("old9");
    let evs = until_idle(&rx);
    assert!(
        evs.contains(&UiEvent::Error(
            "cannot resume while a turn is running".into()
        )),
        "{evs:?}"
    );
    assert!(
        !evs.iter()
            .any(|e| matches!(e, UiEvent::SessionResumed { .. })),
        "{evs:?}"
    );
    h.shutdown();
}

#[test]
fn installed_import_snapshot_goes_in_front_of_next_prompt() {
    let _g = lock();
    let f = fake("install_import", &["text_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.install_import(ClaudeImport {
        path: PathBuf::from("/home/u/.claude/projects/p/abc123.jsonl"),
        turns: vec![("earlier question".into(), "earlier answer".into())],
        files: vec!["src/lib.rs".into()],
        reco: "RECO-KEEP".into(),
        ..Default::default()
    });
    h.submit("next");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "## Current user message");
    let frame = log
        .lines()
        .find(|l| l.contains("## Current user message"))
        .expect("snapshot frame");
    assert!(
        frame.contains("imported from a plain Claude Code session abc123"),
        "{frame}"
    );
    assert!(
        frame.contains("earlier question") && frame.contains("src/lib.rs"),
        "{frame}"
    );
    assert!(frame.contains("RECO-KEEP"), "{frame}");
    assert!(frame.contains("next"), "{frame}");
    h.shutdown();
}

#[test]
fn max_auto_rotations_zero_blocks_context_rotation() {
    let _g = lock();
    let f = fake("rotate_cap", &["rotate_turn.jsonl", "handoff_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.max_auto_rotations = 0;
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let evs = until_idle(&rx);
    assert!(!evs.iter().any(|e| matches!(e, UiEvent::Rotated { .. })));
    h.shutdown();
}

#[test]
fn no_handoff_validate_accepts_short_note_without_retry() {
    let _g = lock();
    let f = fake(
        "no_validate",
        &["rotate_turn.jsonl", "handoff_short_turn.jsonl"],
    );
    let mut cfg = config(&f);
    cfg.rotate_threshold = 50;
    cfg.no_handoff_validate = true;
    let (h, rx) = Engine::start(cfg);
    h.submit("first");
    let evs = until_idle(&rx);
    assert!(
        evs.iter()
            .any(|e| matches!(e, UiEvent::Rotated { reason } if reason == "ctx 100 >= 50"))
    );
    // The short note is not retried: the handoff prompt is sent once before the child rotates.
    let log = wait_for(&f.dir.join("stdin.log"), "Write a handoff note");
    assert_eq!(log.matches("Write a handoff note").count(), 1, "{log}");
    h.submit("again");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "## Current user message");
    let frame = log
        .lines()
        .find(|l| l.contains("## Current user message"))
        .expect("snapshot frame");
    assert!(frame.contains("Short note."), "{frame}");
    assert!(!frame.contains("failed validation"), "{frame}");
    h.shutdown();
}

#[test]
fn tail_turns_limits_imported_turns_in_snapshot() {
    let _g = lock();
    let f = fake("tail_turns", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.tail_turns = 1;
    let (h, rx) = Engine::start(cfg);
    h.install_import(ClaudeImport {
        path: PathBuf::from("/home/u/.claude/projects/p/abc123.jsonl"),
        turns: vec![
            ("older question".into(), "older answer".into()),
            ("newer question".into(), "newer answer".into()),
        ],
        ..Default::default()
    });
    h.submit("next");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "## Current user message");
    let frame = log
        .lines()
        .find(|l| l.contains("## Current user message"))
        .expect("snapshot frame");
    assert!(frame.contains("newer question"), "{frame}");
    assert!(!frame.contains("older question"), "{frame}");
    h.shutdown();
}

#[test]
fn snapshot_chars_clips_the_snapshot() {
    let _g = lock();
    let f = fake("snapshot_chars", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.snapshot_chars = 400;
    let (h, rx) = Engine::start(cfg);
    h.install_import(ClaudeImport {
        path: PathBuf::from("/home/u/.claude/projects/p/abc123.jsonl"),
        turns: vec![("question".into(), format!("{}ZZTAIL", "x".repeat(1_000)))],
        ..Default::default()
    });
    h.submit("next");
    until_idle(&rx);
    let log = wait_for(&f.dir.join("stdin.log"), "## Current user message");
    let frame = log
        .lines()
        .find(|l| l.contains("## Current user message"))
        .expect("snapshot frame");
    assert!(frame.contains("[Context carried over"), "{frame}");
    assert!(!frame.contains("ZZTAIL"), "{frame}");
    h.shutdown();
}

#[test]
fn opus_agent_emits_opus_busy_bracket_around_its_run() {
    let _g = lock();
    let f = fake("opus_agent", &["opus_agent_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("scan");
    let evs = until_idle(&rx);
    let on = evs.iter().position(|e| *e == UiEvent::OpusBusy(true));
    let end = evs
        .iter()
        .position(|e| matches!(e, UiEvent::AgentEnd { .. }));
    let off = evs.iter().position(|e| *e == UiEvent::OpusBusy(false));
    assert!(on.is_some() && off.is_some(), "{evs:?}");
    // The opus run ends with its tool result, which comes before AgentEnd.
    assert!(on < off && on < end, "{evs:?}");
    assert_eq!(
        evs.iter()
            .filter(|e| matches!(e, UiEvent::OpusBusy(_)))
            .count(),
        2,
        "{evs:?}"
    );
    // The Agent call's model input rides on AgentEnd (Go doneLine model); the inherited case stays empty.
    let end_model = evs.iter().find_map(|e| match e {
        UiEvent::AgentEnd { model, .. } => Some(model.clone()),
        _ => None,
    });
    assert_eq!(end_model.as_deref(), Some("opus"), "{evs:?}");
    h.shutdown();
}

// Go doneLine what: the subagent's last plain text line (Summary, else lastSummary(Log)); with no text line
// the Task (description) is the fallback.
#[test]
fn agent_end_summary_is_last_subagent_text_else_task() {
    let _g = lock();
    let f = fake("agent_summary", &["agent_summary_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("scan");
    let evs = until_idle(&rx);
    let summary = evs.iter().find_map(|e| match e {
        UiEvent::AgentEnd { summary, .. } => Some(summary.clone()),
        _ => None,
    });
    assert_eq!(summary.as_deref(), Some("found 3 files"), "{evs:?}");
    h.shutdown();

    // The opus fixture's subagent has no text line (only a stream delta), so the done notice falls back to Task.
    let f = fake("opus_summary", &["opus_agent_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("scan");
    let evs = until_idle(&rx);
    let summary = evs.iter().find_map(|e| match e {
        UiEvent::AgentEnd { summary, .. } => Some(summary.clone()),
        _ => None,
    });
    assert_eq!(summary.as_deref(), Some("scan repo"), "{evs:?}");
    h.shutdown();
}

// The done line's what is the sub-agent's final SubagentHandback message, not its interim text line (stdout.raw:331).
#[test]
fn agent_end_summary_is_subagent_handback_message_not_interim_text() {
    let _g = lock();
    let f = fake("agent_handback", &["agent_handback_summary_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("sleep");
    let evs = until_idle(&rx);
    let summary = evs.iter().find_map(|e| match e {
        UiEvent::AgentEnd { summary, .. } => Some(summary.clone()),
        _ => None,
    });
    assert_eq!(summary.as_deref(), Some("DONE-ROT"), "{evs:?}");
    h.shutdown();
}

#[test]
fn opus_prefixed_subagent_type_emits_opus_busy() {
    let _g = lock();
    let f = fake("opus_subagent_type", &["opus_subagent_type_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("scan");
    let evs = until_idle(&rx);
    // No model and no name: only subagent_type "opus-architect" marks the run as opus (Go Name fallback).
    assert!(evs.contains(&UiEvent::OpusBusy(true)), "{evs:?}");
    assert!(evs.contains(&UiEvent::OpusBusy(false)), "{evs:?}");
    h.shutdown();
}

#[test]
fn coder_task_emits_resident_busy_bracket_before_reply() {
    let _g = lock();
    let f = fake("coder_busy", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.coder("rename the helper");
    let evs = until_coder_reply(&rx);
    let on = evs.iter().position(
        |e| matches!(e, UiEvent::ResidentBusy { role, busy: true, .. } if role == "coder"),
    );
    let off = evs.iter().position(
        |e| matches!(e, UiEvent::ResidentBusy { role, busy: false, .. } if role == "coder"),
    );
    let reply = evs
        .iter()
        .position(|e| matches!(e, UiEvent::CoderReply { .. }));
    assert!(on.is_some() && off.is_some() && reply.is_some(), "{evs:?}");
    assert!(on < off && off < reply, "{evs:?}");
    h.shutdown();
}

// The coder line: a live resident coder is listed with its context tokens from its result usage and its limit.
#[test]
fn coder_contexts_list_the_live_coder_with_its_tokens() {
    let _g = lock();
    let f = fake("coder_ctx", &["text_turn.jsonl"]);
    fs::copy(fixture("resident_usage.jsonl"), f.dir.join("coder.jsonl")).unwrap();
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.coder("rename the helper");
    let evs = until_coder_reply(&rx);
    assert!(
        evs.contains(&UiEvent::CoderContexts {
            coders: vec![CoderContext {
                slot: 1,
                tokens: 103,
                limit: 80_000,
            }],
        }),
        "{evs:?}"
    );
    h.shutdown();
}

// Go onSubEvent (repl.go:426, 686): a delegated agent's text, tool calls and tool results reach its log.
#[test]
fn subagent_stream_lines_reach_agent_log() {
    let _g = lock();
    let f = fake("sub_log", &["subagent_log_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let lines: Vec<String> = evs
        .iter()
        .filter_map(|e| match e {
            UiEvent::AgentLog { id, lines, .. } if id == "toolu_sub1" => Some(lines.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(
        lines,
        vec![
            "Looking around".to_string(),
            "● Bash(ls)".to_string(),
            "  ⎿  a.txt … (+1 lines)".to_string(),
        ]
    );
    h.shutdown();
}

// Go idleFinish (repl.go:517-575): a child-initiated reply after a background launch is shown as a main-thread
// message and ends the agent it belongs to.
#[test]
fn idle_reply_is_shown_as_main_message_after_agent_ends() {
    let _g = lock();
    let f = fake("bg_idle_show", &["bg_idle_stream_turn.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let mut evs = until_idle(&rx);
    let ended = |evs: &[UiEvent]| {
        evs.iter()
            .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_idle1"))
    };
    while !ended(&evs) {
        evs.push(rx.recv_timeout(WAIT).expect("engine went silent"));
    }
    let shown = |evs: &[UiEvent]| {
        evs.iter()
            .any(|e| matches!(e, UiEvent::MessageDone { role: Role::Main }))
    };
    while !shown(&evs) {
        evs.push(rx.recv_timeout(WAIT).expect("engine went silent"));
    }
    assert!(evs.iter().any(|e| matches!(
        e,
        UiEvent::TextDelta { role: Role::Main, text } if text == "Scan finished"
    )));
    h.shutdown();
}

// Go idleFinish (repl.go:518-575): the idle reply is also logged as a background turn (metrics stage=turn,
// source=background).
#[test]
fn idle_reply_is_logged_to_metrics_as_background() {
    let _g = lock();
    let f = fake("bg_idle_metrics", &["bg_idle_stream_turn.jsonl"]);
    let metrics = f.dir.join("metrics");
    let cfg = Config {
        metrics_dir: Some(metrics.clone()),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let mut evs = until_idle(&rx);
    while !evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_idle1"))
    {
        evs.push(rx.recv_timeout(WAIT).expect("engine went silent"));
    }
    h.shutdown();
    let text: String = fs::read_dir(&metrics)
        .unwrap()
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .collect();
    assert!(
        text.lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .any(|r| r["source"] == "background"),
        "{text}"
    );
}

// Go TestIdleSpendRow (background row): the idle turn row carries sub_output_tokens, spawns and result_model_usage.
#[test]
fn go_idle_spend_row_background_row() {
    let _g = lock();
    let f = fake("idle_spend_row", &["bg_idle_spend_turn.jsonl"]);
    let metrics = f.dir.join("metrics");
    let cfg = Config {
        metrics_dir: Some(metrics.clone()),
        ..config(&f)
    };
    let (h, rx) = Engine::start(cfg);
    h.submit("go");
    let mut evs = until_idle(&rx);
    while !evs
        .iter()
        .any(|e| matches!(e, UiEvent::AgentEnd { id, .. } if id == "toolu_idle1"))
    {
        evs.push(rx.recv_timeout(WAIT).expect("engine went silent"));
    }
    h.shutdown();
    let rows: Vec<serde_json::Value> = fs::read_dir(&metrics)
        .unwrap()
        .flat_map(|e| {
            fs::read_to_string(e.unwrap().path())
                .unwrap()
                .lines()
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(&l).ok())
        .filter(|r| r["source"] == "background")
        .collect();
    assert_eq!(rows.len(), 1, "background rows: {rows:?}");
    let row = &rows[0];
    assert_eq!(row["sub_output_tokens"], serde_json::json!(7.0), "{row}");
    assert_eq!(row["spawns"], serde_json::json!(1), "{row}");
    assert!(
        row["result_model_usage"]["claude-haiku-4-5"].is_object(),
        "{row}"
    );
}

// Go TestRegistryNilWhenNoFile (cachettl_test.go:23-43): no clients_file means no registry; a set path wires one.
#[test]
fn wire_clients_is_none_without_clients_file() {
    let none = Config {
        clients_file: None,
        ..Config::default()
    };
    assert!(chaude_core::api::wire_clients(&none).is_none());
    let some = Config {
        clients_file: Some(PathBuf::from("/tmp/chaude-wire-clients-test.json")),
        ..Config::default()
    };
    assert!(chaude_core::api::wire_clients(&some).is_some());
}

// A resumed run starts (task_started with the agent's tool id) after the agent ended, with no SendMessage result
// that the engine reads. The row must show running again until the run's own completion.
#[test]
fn task_started_after_agent_end_reopens_its_row() {
    let _g = lock();
    let f = fake("reopen_row", &["resume_task_started_reopens_row.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let last = evs.iter().rev().find_map(|e| match e {
        UiEvent::AgentRows { rows } => Some(rows.clone()),
        _ => None,
    });
    let row = last
        .and_then(|rows| rows.into_iter().find(|r| r.id == "toolu_bg1"))
        .expect("no row for toolu_bg1");
    assert!(
        !row.done,
        "resumed agent row shows done while it runs: {evs:?}"
    );
    h.shutdown();
}

// An agent delivers an interim SubagentHandback, finishes, then resumes (task_started with its tool id) and ends with
// plain text. Its last done line must show the final text, not the interim handback the resumed run replaced.
#[test]
fn resumed_run_final_text_replaces_interim_handback() {
    let _g = lock();
    let f = fake(
        "resume_final_text",
        &["resume_handback_then_final_text.jsonl"],
    );
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let ends: Vec<String> = evs
        .iter()
        .filter_map(|e| match e {
            UiEvent::AgentEnd { id, summary, .. } if id == "toolu_bg1" => Some(summary.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        ends.last().map(String::as_str),
        Some("DONE-ROT"),
        "last done line of the resumed agent: {ends:?}"
    );
    h.shutdown();
}

// The last owned task of an agent completes before the agent's own completion; the agent then resumes. The owned
// completion must not finish the agent: one done line, with the final text, at the agent's own completion.
#[test]
fn owned_completion_before_resume_is_not_a_finish() {
    let _g = lock();
    let f = fake("owned_then_resume", &["resume_owned_then_final_text.jsonl"]);
    let (h, rx) = Engine::start(config(&f));
    h.submit("go");
    let evs = until_idle(&rx);
    let ends: Vec<String> = evs
        .iter()
        .filter_map(|e| match e {
            UiEvent::AgentEnd { id, summary, .. } if id == "toolu_bg1" => Some(summary.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        ends,
        vec!["DONE-ROT".to_string()],
        "done lines of the resumed agent: {ends:?}"
    );
    h.shutdown();
}
