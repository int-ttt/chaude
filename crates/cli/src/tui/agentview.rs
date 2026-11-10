//! Ctrl+T agent view (Go agentview.go: avKey, agentViewLines, cycleAgent, openAgentKey; drain/tracker
//! onSubEvent and sortedAll). Read-only live view of one sub-agent.
//! Pure over `AgentRec` snapshots: the caller owns the tracker, feeds sub-agent stream events through
//! `sub_event_lines` + `push_log`, sorts with `sort_all`, and closes the view when the agent is gone.
// Not wired into mod.rs yet (TUI A owns it), so dead-code warnings are allowed until the hook calls land.
#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::bar::{AgentState, agent_status_text, clip, fmt_secs};
use super::render::{DIM_ON, FG_OFF, ORANGE_ON, RESET};

// Go maxAgentLog: the live transcript kept per agent (oldest lines drop first).
pub const MAX_AGENT_LOG: usize = 400;
// Go openAgentKey: shown when there is no agent to view.
pub const NO_AGENTS: &str = "no agents yet";
// Go agentViewLines: header, hint, and the empty-body placeholder.
const HEAD: &str = "Agent (read-only)";
const HINT: &str = "  esc/q close · tab next · ↑/↓ pgup/pgdn scroll";
const NO_OUTPUT: &str = "(no output from this agent yet)";

// One agent as the tracker holds it (Go agentRec, the fields the view reads).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentRec {
    pub id: String,
    pub task: String,   // Agent call's description (Go Task)
    pub prompt: String, // first line of the prompt handed to the agent (Go Prompt)
    pub model: String,  // empty: inherits the session model
    pub name: String,   // session name shown in the panel
    pub failed: bool,
    pub done: bool,
    pub interrupted: bool,
    pub started: bool,    // false: queued (Go Started.IsZero())
    pub started_at: u64, // start time as a sortable key (ms since epoch); only compared when both started
    pub last_active: u64, // finished rows: end (or last activity) in ms since epoch; 0 when unknown
    pub log: Vec<String>,
}

impl AgentRec {
    // Go agentState (tui.go:1505-1517).
    pub fn state(&self) -> AgentState {
        super::bar::agent_state(self.failed, self.interrupted, self.done, self.started)
    }

    // The title after the dot, in the agent list format: "HH:MM:SS <status> · <type> · <model> (<task>) (<dur>)".
    // Go agentStatusText (tui.go:1527) gives the middle part (summary = Task, else the first prompt line). A running
    // agent runs to `now_ms`; a finished one to its last_active. Time and duration show once their start is known.
    pub fn status_text(&self, now_ms: u64) -> String {
        let sum = if self.task.is_empty() {
            &self.prompt
        } else {
            &self.task
        };
        let mut text = agent_status_text(self.state(), &self.model, &self.name, sum);
        let end = if matches!(self.state(), AgentState::Running) {
            now_ms
        } else {
            self.last_active
        };
        if self.started_at > 0 && end > self.started_at {
            text.push_str(&format!(" ({})", fmt_secs((end - self.started_at) / 1000)));
        }
        if self.started_at > 0 {
            text = format!(
                "{} {text}",
                super::clock_hhmmss((self.started_at / 1000) as i64)
            );
        }
        text
    }
}

// Go sortedAll: running agents first (by start time, then id), then finished ones the same way.
pub fn sort_all(recs: &mut [AgentRec]) {
    recs.sort_by(|a, b| {
        a.done
            .cmp(&b.done)
            .then(a.started_at.cmp(&b.started_at))
            .then(a.id.cmp(&b.id))
    });
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

// Go toolSummary (tui.go:67-84): the first known input key (newlines folded, cut at 100 chars), else the
// compact JSON of the input (cut at 100). Empty for no input.
pub fn tool_summary(input: &Value) -> String {
    let Some(obj) = input.as_object() else {
        return String::new();
    };
    for k in [
        "command",
        "file_path",
        "pattern",
        "path",
        "url",
        "query",
        "description",
        "prompt",
    ] {
        let v = s(input, k);
        if !v.is_empty() {
            return trunc(&v.replace('\n', " "), 100);
        }
    }
    if obj.is_empty() {
        return String::new();
    }
    trunc(&input.to_string(), 100)
}

// Go resultText (tui.go:86): a string as is, else the text parts joined by newlines.
fn result_text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Array(l)) => l
            .iter()
            .map(|x| s(x, "text"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

// Go trunc (repl.go:217): cut to n runes.
fn trunc(x: &str, n: usize) -> String {
    x.chars().take(n).collect()
}

// Go onSubEvent formatting: the lines one sub-agent stream event adds to its log, with the parent tool-use id
// (the Agent call id) that owns them. None for main-thread events, unknown types, or no output.
pub fn sub_event_lines(ev: &Value) -> Option<(String, Vec<String>)> {
    let parent = s(ev, "parent_tool_use_id");
    if parent.is_empty() {
        return None;
    }
    let content = ev.pointer("/message/content").and_then(Value::as_array);
    let mut add = Vec::new();
    match s(ev, "type") {
        "assistant" => {
            for b in content.into_iter().flatten() {
                match s(b, "type") {
                    "text" => {
                        let t = s(b, "text").trim();
                        if !t.is_empty() {
                            add.extend(t.split('\n').map(str::to_string));
                        }
                    }
                    "tool_use" => {
                        let mut n = s(b, "name").to_string();
                        let a = tool_summary(b.get("input").unwrap_or(&Value::Null));
                        if !a.is_empty() {
                            n = format!("{n}({a})");
                        }
                        add.push(format!("● {n}"));
                    }
                    _ => {}
                }
            }
        }
        "user" => {
            for b in content.into_iter().flatten() {
                if s(b, "type") == "tool_result" {
                    let text = result_text(b.get("content"));
                    let t = text.trim();
                    let l: Vec<&str> = t.split('\n').collect();
                    let mut x = format!("  ⎿  {}", trunc(l[0], 160));
                    if l.len() > 1 {
                        x.push_str(&format!(" … (+{} lines)", l.len() - 1));
                    }
                    add.push(x);
                }
            }
        }
        _ => {}
    }
    if add.is_empty() {
        None
    } else {
        Some((parent.to_string(), add))
    }
}

// One agent's live log: its lines, and index-aligned the tool-use id of each call and result line (None for text).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentLog {
    pub lines: Vec<String>,
    pub ids: Vec<Option<String>>,
}

// Leads of a tool call line and of a tool result line (see sub_event_lines).
const CALL_LEAD: &str = "● ";
const RESULT_LEAD: &str = "  ⎿  ";

// Go onSubEvent cap: append, then keep only the newest MAX_AGENT_LOG lines. A tool result whose id matches a call
// in the log goes directly under that call, so parallel calls keep their own results whatever the arrival order.
// A result with no matching call (or no id) is appended.
pub fn push_log(log: &mut AgentLog, add: Vec<String>, ids: Vec<Option<String>>) {
    let ids = ids.into_iter().chain(std::iter::repeat(None));
    for (line, id) in add.into_iter().zip(ids) {
        let call = match &id {
            Some(k) if line.starts_with(RESULT_LEAD) => log
                .ids
                .iter()
                .zip(&log.lines)
                .position(|(i, l)| i.as_ref() == Some(k) && l.starts_with(CALL_LEAD)),
            _ => None,
        };
        let at = call.map_or(log.lines.len(), |c| c + 1);
        log.lines.insert(at, line);
        log.ids.insert(at, id);
    }
    let n = log.lines.len();
    if n > MAX_AGENT_LOG {
        log.lines.drain(..n - MAX_AGENT_LOG);
        log.ids.drain(..n - MAX_AGENT_LOG);
    }
}

// Local arrival stamp for log lines (core lines carry no time). The stamp follows the line's lead ("● " tool,
// "  ⎿  " result) so the lead and the wrap hang stay first; text lines get it at column 0. Blank and already
// stamped lines are kept as they are.
pub fn stamp_lines(lines: Vec<String>, hhmmss: &str) -> Vec<String> {
    lines
        .into_iter()
        .map(|l| {
            if l.is_empty() {
                return l;
            }
            let lead = ["● ", "  ⎿  "]
                .into_iter()
                .find(|p| l.starts_with(*p))
                .unwrap_or("");
            let (head, rest) = l.split_at(lead.len());
            if has_stamp(rest) {
                l
            } else {
                format!("{head}{hhmmss} {rest}")
            }
        })
        .collect()
}

// True when `s` starts with "HH:MM:SS " (the shape clock_hhmmss writes).
fn has_stamp(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 9
        && b[8] == b' '
        && b[..8].iter().enumerate().all(|(i, c)| {
            if i == 2 || i == 5 {
                *c == b':'
            } else {
                c.is_ascii_digit()
            }
        })
}

// Go agentView: `off` = lines scrolled up from the tail.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentView {
    pub id: String,
    pub off: usize,
}

// What the caller does after a key: keep the view open, or close it (Go avKey sets m.av = nil).
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Close,
    // Rows to scroll, positive toward the tail (the caller clamps them like the main pane's scroll).
    Scroll(isize),
}

impl AgentView {
    // Go openAgentView: the view starts at the tail.
    pub fn open(id: &str) -> Self {
        AgentView {
            id: id.to_string(),
            off: 0,
        }
    }

    // Go tui.go wheel branch on the agent view: up adds 3 lines of scroll back, down removes 3 (floored at the tail).
    pub fn wheel(&mut self, up: bool) {
        self.off = if up {
            self.off + 3
        } else {
            self.off.saturating_sub(3)
        };
    }

    // Go openAgentKey: the first agent of the sorted list (`recs` must already be sorted with sort_all).
    // None when there are no agents; the caller then shows NO_AGENTS as a note.
    pub fn open_first(recs: &[AgentRec]) -> Option<Self> {
        recs.first().map(|a| AgentView::open(&a.id))
    }

    // Go cycleAgent: next (d = 1) or previous (d = -1) agent in the sorted list, wrapping. The offset resets as
    // the Go code builds a new view. An id missing from the list leaves the view unchanged.
    pub fn cycle(&mut self, recs: &[AgentRec], d: isize) {
        let n = recs.len() as isize;
        if let Some(i) = recs.iter().position(|a| a.id == self.id) {
            let j = (i as isize + d).rem_euclid(n) as usize;
            *self = AgentView::open(&recs[j].id);
        }
    }

    // Go avKey: read-only. Only close, switch and scroll keys act; everything else is dropped.
    // `recs` is the sorted list; `vp_h` is the transcript height (page size is max(1, vp_h - 2)).
    pub fn key(&mut self, key: KeyEvent, recs: &[AgentRec], vp_h: usize) -> Action {
        let page = vp_h.saturating_sub(2).max(1) as isize;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Action::Close,
            KeyCode::Char('c') if ctrl => return Action::Close,
            KeyCode::Tab if shift => self.cycle(recs, -1),
            KeyCode::Tab | KeyCode::Right => self.cycle(recs, 1),
            KeyCode::BackTab | KeyCode::Left => self.cycle(recs, -1),
            // The caller clamps the scroll to the oldest line and the tail (the main pane's scroll path).
            KeyCode::Up | KeyCode::Char('k') => return Action::Scroll(-1),
            KeyCode::Down | KeyCode::Char('j') => return Action::Scroll(1),
            KeyCode::PageUp => return Action::Scroll(-page),
            KeyCode::PageDown => return Action::Scroll(page),
            KeyCode::Home => return Action::Scroll(isize::MIN / 2),
            KeyCode::End => return Action::Scroll(isize::MAX / 2),
            _ => {}
        }
        Action::None
    }

    // Go agentViewLines: exactly `hh` rows, replacing the transcript. `rec` is the snapshot of `self.id`.
    // Clamps the scroll offset to the body like Go, so the caller can keep `self` across frames.
    pub fn lines(&mut self, rec: &AgentRec, width: usize, hh: usize, now_ms: u64) -> Vec<String> {
        let head_a = clip(HEAD, width);
        let head_b = clip(
            HINT,
            width.saturating_sub(UnicodeWidthStr::width(head_a.as_str())),
        );
        let head = format!("{ORANGE_ON}{head_a}{FG_OFF}{DIM_ON}{head_b}{RESET}");

        let dot = format!("\x1b[38;5;{}m●{FG_OFF}", rec.state().dot_color());
        let status = clip(
            &format!(" {}", rec.status_text(now_ms)),
            width.saturating_sub(3),
        );
        let title = format!("  {dot}{DIM_ON}{status}{RESET}");

        let body = body_rows(rec, width);

        let room = window_rows(hh);
        self.off = self.off.min(body.len().saturating_sub(room));
        let end = body.len() - self.off;
        let from = end.saturating_sub(room);

        let mut out = vec![head, title];
        out.extend(body[from..end].iter().cloned());
        out.resize(hh, String::new());
        out.truncate(hh);
        out
    }
}

// Rows above the log window: the head and the title.
pub const HEAD_ROWS: usize = 2;

// Log rows shown in a view `hh` rows tall (at least one).
pub fn window_rows(hh: usize) -> usize {
    hh.saturating_sub(HEAD_ROWS).max(1)
}

// The wrapped log of `rec`, one entry per screen row: the rows the window shows and the agent pane selects.
pub fn body_rows(rec: &AgentRec, width: usize) -> Vec<String> {
    let cw = 10.max(width.saturating_sub(2));
    let mut body: Vec<String> = Vec::new();
    for l in &rec.log {
        body.extend(wrap(l, cw));
    }
    if rec.log.is_empty() {
        body.push(format!("{DIM_ON}{NO_OUTPUT}{RESET}"));
        if !rec.prompt.is_empty() {
            body.push(format!("{DIM_ON}task: {}{RESET}", rec.prompt));
        }
    }
    body
}

// lipgloss Width(cw) stand-in: hard-wraps one log line by display column (no word wrap). An empty line
// stays one empty row.
fn wrap(line: &str, cw: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut used = 0;
    for c in line.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > cw && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
            used = 0;
        }
        cur.push(c);
        used += w;
    }
    out.push(cur);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rec(id: &str, started_at: u64, done: bool) -> AgentRec {
        AgentRec {
            id: id.into(),
            name: "worker".into(),
            task: "fix bug".into(),
            prompt: "do it".into(),
            started: true,
            started_at,
            done,
            ..Default::default()
        }
    }

    fn press(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    // Go TestAgentViewClickReadOnlyClose (agentview_test.go:21): typing is dropped; esc and q close;
    // ctrl+t (open_first) reopens.
    #[test]
    fn read_only_keys_close_and_reopen() {
        let recs = vec![rec("a1", 1, false)];
        let mut v = AgentView::open("a1");
        assert_eq!(v.key(press(KeyCode::Char('h')), &recs, 20), Action::None);
        assert_eq!(v.key(press(KeyCode::Enter), &recs, 20), Action::None);
        assert_eq!(v.key(press(KeyCode::Esc), &recs, 20), Action::Close);
        assert_eq!(
            AgentView::open_first(&recs).map(|x| x.id),
            Some("a1".into())
        );
        assert_eq!(AgentView::open_first(&[]), None, "no agents yet");
        assert_eq!(v.key(press(KeyCode::Char('q')), &recs, 20), Action::Close);
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(v.key(ctrl_c, &recs, 20), Action::Close);
    }

    // Go sortedAll + cycleAgent: running first, tab/right next, shift+tab/left previous, wrapping.
    #[test]
    fn tab_cycles_over_sorted_agents_and_wraps() {
        let mut recs = vec![rec("b", 2, true), rec("a", 1, false), rec("c", 3, false)];
        sort_all(&mut recs);
        let ids: Vec<&str> = recs.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["a", "c", "b"]);
        let mut v = AgentView::open("a");
        v.off = 5;
        v.key(press(KeyCode::Tab), &recs, 20);
        assert_eq!((v.id.as_str(), v.off), ("c", 0), "switch resets the offset");
        v.key(press(KeyCode::Tab), &recs, 20);
        assert_eq!(v.id, "b");
        v.key(press(KeyCode::Right), &recs, 20);
        assert_eq!(v.id, "a", "wraps");
        v.key(press(KeyCode::BackTab), &recs, 20);
        assert_eq!(v.id, "b");
        v.key(press(KeyCode::Left), &recs, 20);
        assert_eq!(v.id, "c");
    }

    // Go avKey scroll keys: up/k older, down/j newer (floored at the tail), pgup/pgdn by max(1, h-2),
    // home jumps past the cap, end returns to the tail.
    #[test]
    fn scroll_keys_follow_go() {
        let recs = vec![rec("a", 1, false)];
        let mut v = AgentView::open("a");
        assert_eq!(
            v.key(press(KeyCode::Char('k')), &recs, 12),
            Action::Scroll(-1)
        );
        assert_eq!(v.key(press(KeyCode::Up), &recs, 12), Action::Scroll(-1));
        assert_eq!(v.key(press(KeyCode::Down), &recs, 12), Action::Scroll(1));
        assert_eq!(
            v.key(press(KeyCode::Char('j')), &recs, 12),
            Action::Scroll(1)
        );
        assert_eq!(
            v.key(press(KeyCode::PageUp), &recs, 12),
            Action::Scroll(-10)
        );
        assert_eq!(
            v.key(press(KeyCode::PageDown), &recs, 12),
            Action::Scroll(10)
        );
        assert_eq!(
            v.key(press(KeyCode::PageUp), &recs, 1),
            Action::Scroll(-1),
            "page size is at least 1"
        );
        assert_eq!(
            v.key(press(KeyCode::Home), &recs, 12),
            Action::Scroll(isize::MIN / 2)
        );
        assert_eq!(
            v.key(press(KeyCode::End), &recs, 12),
            Action::Scroll(isize::MAX / 2)
        );
        assert_eq!(
            v.off, 0,
            "keys no longer move the offset; the caller clamps it"
        );
    }

    #[test]
    fn wheel_scrolls_the_log_by_three_like_go() {
        let mut v = AgentView::open("a");
        v.wheel(true);
        assert_eq!(v.off, 3);
        v.wheel(false);
        v.wheel(false);
        assert_eq!(v.off, 0, "floored at the tail");
    }

    // Go onSubEvent: text lines, "● tool(summary)" and "  ⎿  result … (+N lines)". No parent: ignored.
    #[test]
    fn sub_events_format_like_go() {
        let text = json!({"type": "assistant", "parent_tool_use_id": "a1", "message": {"content": [
            {"type": "text", "text": " working on SUBLINE\nsecond "}]}});
        let (p, add) = sub_event_lines(&text).unwrap();
        assert_eq!(p, "a1");
        assert_eq!(add, vec!["working on SUBLINE", "second"]);

        let tool = json!({"type": "assistant", "parent_tool_use_id": "a1", "message": {"content": [
            {"type": "tool_use", "name": "Bash", "input": {"command": "ls\n-la"}}]}});
        assert_eq!(sub_event_lines(&tool).unwrap().1, vec!["● Bash(ls -la)"]);

        let res = json!({"type": "user", "parent_tool_use_id": "a1", "message": {"content": [
            {"type": "tool_result", "content": "line1\nline2\nline3"}]}});
        assert_eq!(
            sub_event_lines(&res).unwrap().1,
            vec!["  ⎿  line1 … (+2 lines)"]
        );

        let main =
            json!({"type": "assistant", "message": {"content": [{"type": "text", "text": "hi"}]}});
        assert_eq!(sub_event_lines(&main), None, "main-thread event ignored");
        let other = json!({"type": "system", "parent_tool_use_id": "a1"});
        assert_eq!(sub_event_lines(&other), None);
    }

    // Go onSubEvent cap (maxAgentLog = 400): the newest lines are kept.
    #[test]
    fn log_keeps_newest_400_lines() {
        let mut log = AgentLog::default();
        push_log(
            &mut log,
            (0..399).map(|i| format!("l{i}")).collect(),
            vec![],
        );
        push_log(&mut log, vec!["x".into(), "y".into()], vec![]);
        assert_eq!(log.lines.len(), MAX_AGENT_LOG);
        assert_eq!(log.ids.len(), MAX_AGENT_LOG);
        assert_eq!(log.lines[0], "l1");
        assert_eq!(log.lines.last().map(String::as_str), Some("y"));
    }

    // A tool result goes under the call with its id, whatever the order the results arrive in.
    #[test]
    fn results_sit_under_their_calls() {
        let id = |s: &str| Some(s.to_string());
        let mut log = AgentLog::default();
        push_log(
            &mut log,
            vec!["● Bash(ls)".into(), "● Bash(cat)".into()],
            vec![id("A"), id("B")],
        );
        // Results in reverse order: each one lands under its own call.
        push_log(&mut log, vec!["  ⎿  cat out".into()], vec![id("B")]);
        push_log(&mut log, vec!["  ⎿  ls out".into()], vec![id("A")]);
        assert_eq!(
            log.lines,
            vec!["● Bash(ls)", "  ⎿  ls out", "● Bash(cat)", "  ⎿  cat out"]
        );
        // A result that arrives after a third call started still lands under its own call.
        let mut log = AgentLog::default();
        push_log(
            &mut log,
            vec!["● Bash(ls)".into(), "● Bash(cat)".into()],
            vec![id("A"), id("B")],
        );
        push_log(&mut log, vec!["● Read(x)".into()], vec![id("C")]);
        push_log(&mut log, vec!["  ⎿  ls out".into()], vec![id("A")]);
        push_log(&mut log, vec!["  ⎿  cat out".into()], vec![id("B")]);
        assert_eq!(
            log.lines,
            vec![
                "● Bash(ls)",
                "  ⎿  ls out",
                "● Bash(cat)",
                "  ⎿  cat out",
                "● Read(x)"
            ]
        );
        assert_eq!(log.ids.len(), log.lines.len());
        // No id, or an id with no call in the log: appended as today.
        push_log(
            &mut log,
            vec!["  ⎿  orphan".into(), "  ⎿  unknown".into()],
            vec![None, id("Z")],
        );
        assert_eq!(log.lines[5..], ["  ⎿  orphan", "  ⎿  unknown"]);
    }

    // Go agentViewLines: header, title, then the newest body rows; exactly hh rows; placeholder when empty;
    // scroll offset clamped to the body.
    #[test]
    fn draw_is_exact_height_and_clamps_scroll() {
        let mut r = rec("a", 1, false);
        let mut v = AgentView::open("a");
        let rows = v.lines(&r, 80, 6, 0);
        assert_eq!(rows.len(), 6);
        assert!(rows[0].contains(HEAD));
        assert!(rows[1].contains("실행 중 · worker (fix bug)"));
        assert_eq!(rows[2], format!("{DIM_ON}{NO_OUTPUT}{RESET}"));
        assert_eq!(rows[3], format!("{DIM_ON}task: do it{RESET}"));

        r.log = (1..=10).map(|i| format!("row{i}")).collect();
        v.off = 100;
        let rows = v.lines(&r, 80, 6, 0);
        assert_eq!(v.off, 6, "clamped to body - room");
        assert_eq!((rows[2].as_str(), rows[5].as_str()), ("row1", "row4"));

        v.off = 0;
        let rows = v.lines(&r, 80, 6, 0);
        assert_eq!((rows[2].as_str(), rows[5].as_str()), ("row7", "row10"));
    }

    // 100x30 transcript size: the title shows the agent name, never the id; the body is exactly 30 rows
    // and the scroll offset clamps to the last 28 log rows (header and title use 2 rows).
    #[test]
    fn draw_100x30_shows_name_not_id_and_clamps_scroll() {
        let mut r = rec("agent-zz9", 1, false);
        r.log = (1..=300).map(|i| format!("row{i}")).collect();
        let mut v = AgentView::open("agent-zz9");
        let rows = v.lines(&r, 100, 30, 0);
        assert_eq!(rows.len(), 30);
        assert!(rows[1].contains("worker"), "{:?}", rows[1]);
        assert!(!rows[0].contains("agent-zz9") && !rows[1].contains("agent-zz9"));
        assert_eq!((rows[2].as_str(), rows[29].as_str()), ("row273", "row300"));

        v.off = 10_000;
        let rows = v.lines(&r, 100, 30, 0);
        assert_eq!(v.off, 272, "clamped to 300 - 28");
        assert_eq!((rows[2].as_str(), rows[29].as_str()), ("row1", "row28"));
    }

    // Go agentViewLines: body wraps to max(10, width - 2) columns.
    #[test]
    fn body_wraps_to_width() {
        let mut r = rec("a", 1, false);
        r.log = vec!["abcdefghijklmnop".into()];
        let mut v = AgentView::open("a");
        let rows = v.lines(&r, 12, 5, 0);
        assert_eq!(
            (rows[2].as_str(), rows[3].as_str()),
            ("abcdefghij", "klmnop")
        );
    }

    // The Ctrl+T title uses the list format: HH:MM:SS, status, type, model, task, then the elapsed time. A finished
    // row's elapsed runs to its last_active; a running one's runs to the given now.
    #[test]
    fn title_uses_list_format_with_clock_and_elapsed() {
        let done = AgentRec {
            model: "claude-haiku-5-5".into(),
            last_active: 1_700_000_125_000,
            ..rec("a", 1_700_000_000_000, true)
        };
        let text = done.status_text(0);
        let (clock, rest) = text.split_at(8);
        assert!(
            clock.chars().enumerate().all(|(i, c)| match i {
                2 | 5 => c == ':',
                _ => c.is_ascii_digit(),
            }),
            "{text}"
        );
        assert_eq!(rest, " 완료 · worker · haiku 5.5 (fix bug) (2m05s)");

        let running = rec("b", 1_700_000_000_000, false);
        assert!(
            running
                .status_text(1_700_000_065_000)
                .ends_with(" 실행 중 · worker (fix bug) (1m05s)"),
            "{}",
            running.status_text(1_700_000_065_000)
        );
    }
}
