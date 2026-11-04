//! Ctrl+T agent view (Go agentview.go: avKey, agentViewLines, cycleAgent, openAgentKey; drain/tracker
//! onSubEvent and sortedAll). Read-only live view of one sub-agent.
//! Pure over `AgentRec` snapshots: the caller owns the tracker, feeds sub-agent stream events through
//! `sub_event_lines` + `push_log`, sorts with `sort_all`, and closes the view when the agent is gone.
// Not wired into mod.rs yet (TUI A owns it), so dead-code warnings are allowed until the hook calls land.
#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::bar::{AgentState, agent_status_text, clip};
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
    pub started: bool,   // false: queued (Go Started.IsZero())
    pub started_at: u64, // start time as a sortable key (ms since epoch); only compared when both started
    pub log: Vec<String>,
}

impl AgentRec {
    // Go agentState (tui.go:1505-1517).
    pub fn state(&self) -> AgentState {
        super::bar::agent_state(self.failed, self.interrupted, self.done, self.started)
    }

    // Go agentStatusText (tui.go:1527): summary = Task, else the first prompt line.
    pub fn status_text(&self, session_model: &str) -> String {
        let sum = if self.task.is_empty() {
            &self.prompt
        } else {
            &self.task
        };
        agent_status_text(self.state(), &self.model, session_model, &self.name, sum)
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

// Go onSubEvent cap: append, then keep only the newest MAX_AGENT_LOG lines.
pub fn push_log(log: &mut Vec<String>, add: Vec<String>) {
    log.extend(add);
    let n = log.len();
    if n > MAX_AGENT_LOG {
        log.drain(..n - MAX_AGENT_LOG);
    }
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
}

impl AgentView {
    // Go openAgentView: the view starts at the tail.
    pub fn open(id: &str) -> Self {
        AgentView {
            id: id.to_string(),
            off: 0,
        }
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
        let page = vp_h.saturating_sub(2).max(1);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Action::Close,
            KeyCode::Char('c') if ctrl => return Action::Close,
            KeyCode::Tab if shift => self.cycle(recs, -1),
            KeyCode::Tab | KeyCode::Right => self.cycle(recs, 1),
            KeyCode::BackTab | KeyCode::Left => self.cycle(recs, -1),
            KeyCode::Up | KeyCode::Char('k') => self.off += 1,
            KeyCode::Down | KeyCode::Char('j') => self.off = self.off.saturating_sub(1),
            KeyCode::PageUp => self.off += page,
            KeyCode::PageDown => self.off = self.off.saturating_sub(page),
            KeyCode::Home => self.off = MAX_AGENT_LOG + 1000,
            KeyCode::End => self.off = 0,
            _ => {}
        }
        Action::None
    }

    // Go agentViewLines: exactly `hh` rows, replacing the transcript. `rec` is the snapshot of `self.id`.
    // Clamps the scroll offset to the body like Go, so the caller can keep `self` across frames.
    pub fn lines(
        &mut self,
        rec: &AgentRec,
        session_model: &str,
        width: usize,
        hh: usize,
    ) -> Vec<String> {
        let head_a = clip(HEAD, width);
        let head_b = clip(
            HINT,
            width.saturating_sub(UnicodeWidthStr::width(head_a.as_str())),
        );
        let head = format!("{ORANGE_ON}{head_a}{FG_OFF}{DIM_ON}{head_b}{RESET}");

        let dot = format!("\x1b[38;5;{}m●{FG_OFF}", rec.state().dot_color());
        let status = clip(
            &format!(" {}", rec.status_text(session_model)),
            width.saturating_sub(3),
        );
        let title = format!("  {dot}{DIM_ON}{status}{RESET}");

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

        let room = hh.saturating_sub(2).max(1);
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
        v.key(press(KeyCode::Char('k')), &recs, 12);
        v.key(press(KeyCode::Up), &recs, 12);
        assert_eq!(v.off, 2);
        v.key(press(KeyCode::Down), &recs, 12);
        assert_eq!(v.off, 1);
        v.key(press(KeyCode::Char('j')), &recs, 12);
        v.key(press(KeyCode::Char('j')), &recs, 12);
        assert_eq!(v.off, 0, "floored at the tail");
        v.key(press(KeyCode::PageUp), &recs, 12);
        assert_eq!(v.off, 10);
        v.key(press(KeyCode::PageDown), &recs, 12);
        assert_eq!(v.off, 0);
        v.key(press(KeyCode::PageUp), &recs, 1);
        assert_eq!(v.off, 1, "page size is at least 1");
        v.key(press(KeyCode::Home), &recs, 12);
        assert_eq!(v.off, MAX_AGENT_LOG + 1000);
        v.key(press(KeyCode::End), &recs, 12);
        assert_eq!(v.off, 0);
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
        let mut log = Vec::new();
        push_log(&mut log, (0..399).map(|i| format!("l{i}")).collect());
        push_log(&mut log, vec!["x".into(), "y".into()]);
        assert_eq!(log.len(), MAX_AGENT_LOG);
        assert_eq!(log[0], "l1");
        assert_eq!(log.last().map(String::as_str), Some("y"));
    }

    // Go agentViewLines: header, title, then the newest body rows; exactly hh rows; placeholder when empty;
    // scroll offset clamped to the body.
    #[test]
    fn draw_is_exact_height_and_clamps_scroll() {
        let mut r = rec("a", 1, false);
        let mut v = AgentView::open("a");
        let rows = v.lines(&r, "opus", 80, 6);
        assert_eq!(rows.len(), 6);
        assert!(rows[0].contains(HEAD));
        assert!(rows[1].contains("실행 중-opus-worker(fix bug)"));
        assert_eq!(rows[2], format!("{DIM_ON}{NO_OUTPUT}{RESET}"));
        assert_eq!(rows[3], format!("{DIM_ON}task: do it{RESET}"));

        r.log = (1..=10).map(|i| format!("row{i}")).collect();
        v.off = 100;
        let rows = v.lines(&r, "opus", 80, 6);
        assert_eq!(v.off, 6, "clamped to body - room");
        assert_eq!((rows[2].as_str(), rows[5].as_str()), ("row1", "row4"));

        v.off = 0;
        let rows = v.lines(&r, "opus", 80, 6);
        assert_eq!((rows[2].as_str(), rows[5].as_str()), ("row7", "row10"));
    }

    // 100x30 transcript size: the title shows the agent name, never the id; the body is exactly 30 rows
    // and the scroll offset clamps to the last 28 log rows (header and title use 2 rows).
    #[test]
    fn draw_100x30_shows_name_not_id_and_clamps_scroll() {
        let mut r = rec("agent-zz9", 1, false);
        r.log = (1..=300).map(|i| format!("row{i}")).collect();
        let mut v = AgentView::open("agent-zz9");
        let rows = v.lines(&r, "opus", 100, 30);
        assert_eq!(rows.len(), 30);
        assert!(rows[1].contains("worker"), "{:?}", rows[1]);
        assert!(!rows[0].contains("agent-zz9") && !rows[1].contains("agent-zz9"));
        assert_eq!((rows[2].as_str(), rows[29].as_str()), ("row273", "row300"));

        v.off = 10_000;
        let rows = v.lines(&r, "opus", 100, 30);
        assert_eq!(v.off, 272, "clamped to 300 - 28");
        assert_eq!((rows[2].as_str(), rows[29].as_str()), ("row1", "row28"));
    }

    // Go agentViewLines: body wraps to max(10, width - 2) columns.
    #[test]
    fn body_wraps_to_width() {
        let mut r = rec("a", 1, false);
        r.log = vec!["abcdefghijklmnop".into()];
        let mut v = AgentView::open("a");
        let rows = v.lines(&r, "opus", 12, 5);
        assert_eq!(
            (rows[2].as_str(), rows[3].as_str()),
            ("abcdefghij", "klmnop")
        );
    }
}
