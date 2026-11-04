#![cfg(unix)]
// Parity with Go harness tests (group 2, part 2): resident coder slots, MCP round trip and config,
// agent prompt loading, and the idle spend live replay. Engine tests use a fake `claude` shell script
// (Config::claude_bin override); helpers are copied from tests/engine.rs.

use std::fs;
use std::io::{self, Cursor, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chaude_core::api::{Config, Engine, UiEvent};
use chaude_core::mcp::{self, CoderFn};
use chaude_core::proc;
use chaude_core::resident;
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
    let dir = std::env::temp_dir().join(format!("chaude-p2-{name}-{}-{nanos}", process::id()));
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

// Each user frame n sleeps 0.5 s, then prints turn{n}.jsonl. The coder role sleeps 0.2 s per task.
fn fake(name: &str, turns: &[&str]) -> Fake {
    let dir = temp_dir(name);
    // SAFETY: set before any engine thread of this test starts; other engine tests are serialized by lock().
    unsafe { std::env::set_var("HARNESS_RECORD_DIR", dir.join("record")) };
    for (i, f) in turns.iter().enumerate() {
        fs::copy(fixture(f), dir.join(format!("turn{}.jsonl", i + 1))).unwrap();
    }
    fs::write(
        dir.join("prompter.jsonl"),
        "{\"type\":\"result\",\"result\":\"caveman summary\"}\n",
    )
    .unwrap();
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
while IFS= read -r line; do
  if [ "$role" = coder ]; then
    case "$line" in
      *'"type":"user"'*)
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
        residents: false,
        // Clients and agents state stay in the test dir, never ~/.chaude.
        agents_file: Some(f.dir.join("agents.json")),
        clients_file: Some(f.dir.join("clients.json")),
        ..Config::default()
    }
}

const WAIT: Duration = Duration::from_secs(60);

// Polls a file until it contains needle (or WAIT passes). The fake script writes files lazily.
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

// Collects coder events until `n` CoderReply events arrived, or a coder Failed stage ends the wait.
fn until_coder_done(rx: &Receiver<UiEvent>, n: usize) -> Vec<UiEvent> {
    let mut out = Vec::new();
    let mut replies = 0;
    while replies < n {
        let ev = rx.recv_timeout(WAIT).expect("engine went silent");
        if matches!(ev, UiEvent::CoderReply { .. }) {
            replies += 1;
        }
        let failed = matches!(
            &ev,
            UiEvent::Stage { slot, phase: StagePhase::Failed(_) } if slot == "coder"
        );
        out.push(ev);
        if failed {
            break;
        }
    }
    out
}

// Go TestResidentCoderSlotReuse: two tasks in a row reuse the base coder; no extra child is spawned.
#[test]
fn resident_coder_slot_reuse_spawns_no_extra() {
    let _g = lock();
    let f = fake("slot-reuse", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.coder("first");
    until_coder_done(&rx, 1);
    h.coder("first again");
    until_coder_done(&rx, 1);
    let pids = wait_for(&f.dir.join("coder.pids"), " base\n");
    let lines: Vec<&str> = pids.lines().collect();
    assert_eq!(lines.len(), 2, "{pids}");
    assert!(lines.iter().all(|l| l.ends_with(" base")), "{pids}");
    assert!(!pids.contains(" extra"), "{pids}");
    h.shutdown();
}

// Go TestResidentCoderBusyRefuses: with no extra slot, a second task while the base coder is busy is refused.
#[test]
fn resident_coder_busy_refuses_second_task() {
    let _g = lock();
    let f = fake("busy", &["text_turn.jsonl"]);
    let mut cfg = config(&f);
    cfg.residents = true;
    cfg.extra_coders = 0;
    cfg.cwd = Some(f.dir.clone());
    let (h, rx) = Engine::start(cfg);
    h.coder("first");
    h.coder("second");
    let evs = until_coder_done(&rx, 1);
    assert!(
        evs.iter().any(|e| matches!(
            e,
            UiEvent::Stage { slot, phase: StagePhase::Failed(msg) }
                if slot == "coder" && msg.contains("busy")
        )),
        "{evs:?}"
    );
    h.shutdown();
}

// Go TestMCPRoundTripFakeSession: initialize, tools/list, tools/call returns the coder text as the result,
// and a notification gets no reply. Rust reply() passes the CoderFn text through; the summary is the coder side's job.
#[test]
fn mcp_protocol_answers_and_skips_notifications() {
    let run: Arc<CoderFn> =
        Arc::new(|task: &str| Ok(format!("status: done\nchanged: a.rs: {task}")));
    let init = mcp::reply(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        run.as_ref(),
    )
    .expect("initialize reply");
    assert!(init.contains(r#""id":1"#), "{init}");
    let list = mcp::reply(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        run.as_ref(),
    )
    .expect("tools/list reply");
    assert!(list.contains(mcp::TOOL), "{list}");
    let call = mcp::reply(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"resident_coder","arguments":{"task":"rename"}}}"#,
        run.as_ref(),
    )
    .expect("tools/call reply");
    assert!(call.contains("status: done"), "{call}");
    assert!(call.contains("changed: a.rs: rename"), "{call}");
    assert_eq!(
        mcp::reply(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            run.as_ref()
        ),
        None
    );
}

// Shared output buffer so the test can read what the bridge wrote after it returns.
#[derive(Clone, Default)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// Go TestMCPRoundTripFakeSession over the socket: the bridge reaches the server, then MCP lines flow.
#[test]
fn mcp_bridge_round_trip_over_loopback() {
    let token = mcp::new_token();
    let run: Arc<CoderFn> =
        Arc::new(|task: &str| Ok(format!("status: done\nchanged: b.rs: {task}")));
    let sock = temp_dir("mcp").join("mcp.sock");
    let addr = mcp::serve(run, &token, &sock).expect("serve");
    let input = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"resident_coder","arguments":{"task":"x"}}}"#,
        "\n",
    );
    let out = SharedBuf::default();
    mcp::bridge_with_token(&addr, Some(&token), Cursor::new(input), out.clone()).expect("bridge");
    let text = String::from_utf8(out.0.lock().unwrap().clone()).unwrap();
    assert!(text.contains("changed: b.rs: x"), "{text}");
}

// Go TestResidentMCPConfigWritten: the config is written once, with the bridge command and the token in env.
#[test]
fn mcp_config_is_written_with_bridge_command_and_token_env() {
    let run: Arc<CoderFn> = Arc::new(|_: &str| Ok(String::new()));
    let cfg = mcp::start_owned(run).expect("start_owned");
    let text = fs::read_to_string(cfg.path()).expect("mcp.json");
    let v: serde_json::Value = serde_json::from_str(&text).expect("json");
    let chaude = &v["mcpServers"]["chaude"];
    assert_eq!(chaude["args"][0], "mcp-resident", "{text}");
    assert!(chaude["env"][mcp::TOKEN_ENV].is_string(), "{text}");
    let mode = fs::metadata(cfg.path()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn mcp_config_args_point_at_unix_socket() {
    let run: Arc<CoderFn> = Arc::new(|_: &str| Ok(String::new()));
    let cfg = mcp::start_owned(run).expect("start_owned");
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(cfg.path()).unwrap()).unwrap();
    let addr = v["mcpServers"]["chaude"]["args"][1]
        .as_str()
        .unwrap_or_default();
    assert!(addr.ends_with("mcp.sock"), "{addr}");
}

// Go TestMainStrictMCP: the main child gets --strict-mcp-config with its MCP config, and keeps slash commands.
#[test]
fn main_child_args_use_strict_mcp_and_keep_slash_commands() {
    let main = proc::Shape {
        strict_mcp: true,
        mcp_config: Some(PathBuf::from("/tmp/mcp.json")),
        ..proc::Shape::default()
    };
    let args = proc::build_args("haiku", None, &main);
    assert!(args.iter().any(|a| a == "--strict-mcp-config"), "{args:?}");
    let at = args
        .iter()
        .position(|a| a == "--mcp-config")
        .expect("--mcp-config");
    assert_eq!(args[at + 1], "/tmp/mcp.json");
    assert!(
        !args.iter().any(|a| a == "--disable-slash-commands"),
        "{args:?}"
    );
}

fn temp_cwd(name: &str) -> PathBuf {
    let dir = temp_dir(name);
    fs::create_dir_all(dir.join(".claude").join("agents")).unwrap();
    dir
}

// Go TestAgentPromptLoaded: the agent body comes from cwd/.claude/agents/<name>.md with frontmatter stripped.
#[test]
fn agent_prompt_body_loads_from_cwd_without_frontmatter() {
    let cwd = temp_cwd("agent-body");
    fs::write(
        cwd.join(".claude").join("agents").join("haiku-prompter.md"),
        "---\nname: haiku-prompter\n---\n\nBODY LINE\n",
    )
    .unwrap();
    let body = resident::agent_body(&cwd, "haiku-prompter").expect("body");
    assert!(body.contains("BODY LINE"), "{body}");
    assert!(!body.contains("name: haiku-prompter"), "{body}");
}

// Go TestAgentPromptMissingNotice: a missing agent file should start a default session with a Korean notice.
// Diff: Rust agent_body returns Err("no agent file ...") and spawn fails; no notice, no default session.
#[test]
#[ignore = "parity: Rust errors on a missing agent file, Go prints a notice and starts a default session"]
fn agent_prompt_missing_gives_korean_notice_and_default_session() {
    let cwd = temp_cwd("agent-missing");
    let err = resident::agent_body(&cwd, "haiku-missing").expect_err("missing agent");
    assert!(err.contains("에이전트 규칙 없음"), "{err}");
}

// Go TestIdleSpendLiveReplay (parse-only): every line of the recorded background turn is valid JSON.
// Go skips when testdata is missing; the Rust fixture is checked in, so it must exist.
#[test]
fn idle_spend_live_fixture_parses_line_by_line() {
    let text = fs::read_to_string(fixture("bg_turn_live.jsonl")).expect("fixture");
    let mut seen_result = false;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let v: serde_json::Value = serde_json::from_str(line).expect("json line");
        if v["type"] == "result" {
            seen_result = true;
        }
    }
    assert!(seen_result, "fixture has no result line");
}

// Go TestIdleSpendRow: a subagent's output tokens count into sub_output_tokens and the Agent spawn into spawns.
// Diff: Rust has no background spend row (record_turn saves result_model_usage "{}" and no bg row).
#[test]
#[ignore = "parity: Rust has no background spend row (no sub_output_tokens / spawns)"]
fn idle_spend_row_counts_subagent_tokens_and_spawns() {
    panic!("parity: background spend row is not implemented in Rust");
}
