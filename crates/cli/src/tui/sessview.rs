//! /view session browser (Go sessview.go; resume.go session list; importclaude.go readClaudeTranscript).
//! Level 1: the Claude Code sessions this chaude process spawned, listed below the input box.
//! Level 2: the user prompts of one session, one at a time. Esc goes back one level.
//! The readers here take paths and file text; the overlay returns Actions and the caller owns the transcript.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthStr;

use super::bar::clip;
use super::render::{DIM_ON, FG_OFF, GREY_ON, ORANGE_ON, RESET, trunc_chars, wrap_cells};

// Go sessActiveWindow: a session written this recently is marked active (●).
pub const ACTIVE_WINDOW: Duration = Duration::from_secs(10 * 60);
// Go sessRows: session rows shown below the input box at once.
pub const SESS_ROWS: usize = 5;
// Go ownSessionList: shown when this process has no Claude Code session yet.
pub const NO_OWN: &str = "no Claude Code sessions from this chaude session yet";

// A sub-agent's place in its session's tree (its transcript sits in <session>/subagents/).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AgentRef {
    // The Agent tool's subagent_type, e.g. "haiku-coder"; empty when neither the launch nor the meta names it.
    pub kind: String,
    pub description: String,
    // 1 for a sub-agent the session spawned, 2 for one a sub-agent spawned, and so on.
    pub depth: usize,
}

// One Claude Code transcript (Go sessionInfo; the fields /view and the list need).
#[derive(Clone, Debug)]
pub struct SessionInfo {
    pub id: String,
    pub path: PathBuf,
    pub modified: SystemTime,
    pub turns: usize,
    pub first: String,
    pub last: String,
    // The model of the session's assistant turns (the first one), e.g. "claude-haiku-5-5"; empty if none.
    pub model: String,
    // None for a session file; Some for a sub-agent transcript listed under its parent.
    pub agent: Option<AgentRef>,
}

// Go readClaudeTranscript, user prompts only (sub-agent transcripts are all sidechain; see parse_turns).
pub fn parse_prompts(content: &str) -> Vec<String> {
    parse_turns(content, false)
}

// The user prompts of a transcript. With sidechain false, sidechain events are skipped (a session file);
// with true they count (a sub-agent file, whose only events are sidechain). A user text event that is not
// meta starts a turn; a plain string starts a new turn unless it begins with "<", and array text fills the
// turn only while it is still empty. Assistant events and the rest of the transcript are not needed here.
fn parse_turns(content: &str, sidechain: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(ev) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if ev["type"] != "user"
            || (!sidechain && ev["isSidechain"].as_bool() == Some(true))
            || ev["isMeta"].as_bool() == Some(true)
        {
            continue;
        }
        let content = &ev["message"]["content"];
        if let Some(s) = content.as_str() {
            if !s.trim().is_empty() && !s.starts_with('<') {
                flush(&mut out, &mut cur);
                cur = s.to_string();
            }
        } else if let Some(blocks) = content.as_array() {
            for b in blocks {
                let text = b["text"].as_str().unwrap_or("");
                if b["type"] == "text" && cur.is_empty() && !text.starts_with('<') {
                    cur = text.to_string();
                }
            }
        }
    }
    flush(&mut out, &mut cur);
    out
}

// The model named by the first assistant event that has one (the "model" field of its message).
pub fn parse_model(content: &str) -> String {
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find_map(|ev| ev["message"]["model"].as_str().map(str::to_string))
        .unwrap_or_default()
}

fn flush(out: &mut Vec<String>, cur: &mut String) {
    if !cur.is_empty() {
        out.push(std::mem::take(cur));
    }
}

// Go readClaudeTranscript errors: an unreadable file, or no turns at all.
pub fn read_prompts(path: &Path) -> Result<Vec<String>, String> {
    read_turns(path, false)
}

fn read_turns(path: &Path, sidechain: bool) -> Result<Vec<String>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let prompts = parse_turns(&String::from_utf8_lossy(&bytes), sidechain);
    if prompts.is_empty() {
        return Err(format!("no user/assistant turns in {}", path.display()));
    }
    Ok(prompts)
}

// Go listClaudeSessions: the *.jsonl files of dir that hold at least one prompt, newest first.
// Unreadable or empty files are skipped.
pub fn list_sessions(dir: &Path) -> Vec<SessionInfo> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(modified) = e.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        // One read gives the prompts (Go readClaudeTranscript) and the model; unreadable or empty files are skipped.
        let Ok(content) = std::fs::read(&path).map(|b| String::from_utf8_lossy(&b).into_owned())
        else {
            continue;
        };
        let prompts = parse_prompts(&content);
        if prompts.is_empty() {
            continue;
        }
        let id = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push(SessionInfo {
            id,
            path,
            modified,
            turns: prompts.len(),
            first: one_line(&prompts[0]),
            last: one_line(prompts.last().map_or("", String::as_str)),
            model: parse_model(&content),
            agent: None,
        });
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.modified));
    out
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl SessionInfo {
    // Go sessionInfo.label: "<date>  <turns, 3 wide> turns  <first 40> … <last 40>".
    pub fn label(&self) -> String {
        format!(
            "{}  {:>3} turns  {} … {}",
            stamp(self.modified, true),
            self.turns,
            trunc_chars(&self.first, 40),
            trunc_chars(&self.last, 40)
        )
    }
}

// Go claudeProjectDir: ~/.claude/projects/<cwd with each non-ASCII-alphanumeric char as "-">.
pub fn project_dir(home: &Path, cwd: &str) -> PathBuf {
    let proj: String = cwd
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    home.join(".claude").join("projects").join(proj)
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

// The Claude Code project dir of cwd. Empty cwd means the process cwd.
pub fn session_dir(cwd: &str) -> Result<PathBuf, String> {
    let home = home_dir().ok_or_else(|| "$HOME is not defined".to_string())?;
    let cwd = if cwd.is_empty() {
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    } else {
        cwd.to_string()
    };
    Ok(project_dir(&home, &cwd))
}

// Go sessionList: the sessions of cwd's Claude Code project dir.
pub fn session_list(cwd: &str) -> Result<Vec<SessionInfo>, String> {
    let dir = session_dir(cwd)?;
    let l = list_sessions(&dir);
    if l.is_empty() {
        return Err(format!("no Claude Code sessions in {}", dir.display()));
    }
    Ok(l)
}

// Go ownSessionList filter: keeps the sessions whose id this process spawned (order kept). `owned` is
// the caller's set of main, rotated and resident child session ids. Sub-agents have no file of their own.
pub fn own_sessions(
    all: Vec<SessionInfo>,
    owned: &HashSet<String>,
) -> Result<Vec<SessionInfo>, String> {
    let out: Vec<SessionInfo> = all.into_iter().filter(|s| owned.contains(&s.id)).collect();
    if out.is_empty() {
        return Err(NO_OWN.to_string());
    }
    Ok(out)
}

// Session rows with each session's sub-agents right below it (subagent_list). `project` is the project dir.
pub fn with_subagents(list: Vec<SessionInfo>, project: &Path) -> Vec<SessionInfo> {
    let mut out = Vec::with_capacity(list.len());
    for s in list {
        let agents = subagent_list(project, &s);
        out.push(s);
        out.extend(agents);
    }
    out
}

// The sub-agents of one session: <project>/<session>/subagents/agent-<id>.jsonl, depth first under the agent
// that spawned them, oldest first. The parent is the transcript holding the Agent call whose tool_result names
// the agent; meta.json (parentAgentId, agentType, description) fills in what no launch gives. Agents without
// prompts are not listed.
pub fn subagent_list(project: &Path, session: &SessionInfo) -> Vec<SessionInfo> {
    let Ok(entries) = std::fs::read_dir(project.join(&session.id).join("subagents")) else {
        return Vec::new();
    };
    let mut files: Vec<(String, SystemTime, PathBuf, String)> = Vec::new();
    for e in entries.flatten() {
        let path = e.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let Some(id) = name
            .strip_prefix("agent-")
            .and_then(|n| n.strip_suffix(".jsonl"))
        else {
            continue;
        };
        let Ok(modified) = e.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let content = String::from_utf8_lossy(&bytes).into_owned();
        files.push((id.to_string(), modified, path, content));
    }
    // Who launched whom: child agent id -> (parent transcript id, subagent_type, description).
    let mut launched: HashMap<String, (String, String, String)> = HashMap::new();
    let session_text = std::fs::read(&session.path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let transcripts = std::iter::once((session.id.as_str(), session_text.as_str())).chain(
        files
            .iter()
            .map(|(id, _, _, text)| (id.as_str(), text.as_str())),
    );
    for (parent, text) in transcripts {
        for (child, (kind, desc)) in launches(text) {
            launched
                .entry(child)
                .or_insert((parent.to_string(), kind, desc));
        }
    }
    let meta = |id: &str| -> serde_json::Value {
        std::fs::read(
            project
                .join(&session.id)
                .join("subagents")
                .join(format!("agent-{id}.meta.json")),
        )
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
    };
    let mut found: Vec<(String, SessionInfo)> = Vec::new();
    for (id, modified, path, content) in files {
        let prompts = parse_turns(&content, true);
        if prompts.is_empty() {
            continue;
        }
        let m = meta(&id);
        let (parent, kind, description) = launched.get(&id).cloned().unwrap_or_default();
        let text_of = |key: &str| m[key].as_str().unwrap_or_default().to_string();
        let parent = if parent.is_empty() {
            m["parentAgentId"]
                .as_str()
                .unwrap_or(&session.id)
                .to_string()
        } else {
            parent
        };
        let kind = if kind.is_empty() {
            text_of("agentType")
        } else {
            kind
        };
        let description = if description.is_empty() {
            text_of("description")
        } else {
            description
        };
        found.push((
            parent,
            SessionInfo {
                id: id.clone(),
                path,
                modified,
                turns: prompts.len(),
                first: one_line(&prompts[0]),
                last: one_line(prompts.last().map_or("", String::as_str)),
                model: parse_model(&content),
                agent: Some(AgentRef {
                    kind,
                    description,
                    depth: 0,
                }),
            },
        ));
    }
    // An agent whose parent is not listed (no prompts, or an unknown id) hangs under the session.
    let ids: HashSet<String> = found.iter().map(|(_, s)| s.id.clone()).collect();
    for (parent, _) in found.iter_mut() {
        if *parent != session.id && !ids.contains(parent.as_str()) {
            *parent = session.id.clone();
        }
    }
    found.sort_by_key(|(_, s)| s.modified);
    let mut kids: HashMap<String, Vec<SessionInfo>> = HashMap::new();
    for (parent, s) in found {
        kids.entry(parent).or_default().push(s);
    }
    let mut out = Vec::new();
    push_tree(&session.id, 1, &mut kids, &mut out);
    out
}

// Depth-first: each agent of `parent` (at `depth`), then the agents it spawned. Each agent is taken out of
// `kids` once, so a parent cycle cannot recurse forever.
fn push_tree(
    parent: &str,
    depth: usize,
    kids: &mut HashMap<String, Vec<SessionInfo>>,
    out: &mut Vec<SessionInfo>,
) {
    for mut s in kids.remove(parent).unwrap_or_default() {
        let id = s.id.clone();
        if let Some(a) = s.agent.as_mut() {
            a.depth = depth;
        }
        out.push(s);
        push_tree(&id, depth + 1, kids, out);
    }
}

// The Agent calls of one transcript, keyed by the agent id their tool_result names:
// agent id -> (subagent_type, description). The result text holds "agentId: <id>".
fn launches(content: &str) -> HashMap<String, (String, String)> {
    let mut by_tool: HashMap<String, (String, String)> = HashMap::new();
    let mut out = HashMap::new();
    for ev in content
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
    {
        let Some(blocks) = ev["message"]["content"].as_array() else {
            continue;
        };
        for b in blocks {
            if b["type"] == "tool_use" && b["name"] == "Agent" {
                let input = &b["input"];
                let kind = input["subagent_type"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let desc = input["description"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                if let Some(id) = b["id"].as_str() {
                    by_tool.insert(id.to_string(), (kind, desc));
                }
            } else if b["type"] == "tool_result" {
                let Some(agent) = agent_id_in(&block_text(&b["content"])) else {
                    continue;
                };
                if let Some(launch) = b["tool_use_id"].as_str().and_then(|t| by_tool.get(t)) {
                    out.insert(agent, launch.clone());
                }
            }
        }
    }
    out
}

// The text of a tool_result content: a string, or the text blocks of an array.
fn block_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(a) => a
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

// The agent id after "agentId: " in a tool_result text (ids are alphanumeric).
fn agent_id_in(text: &str) -> Option<String> {
    let rest = text.split("agentId: ").nth(1)?;
    let id: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    (!id.is_empty()).then_some(id)
}

// Go shortID: the first 8 characters of a session id.
pub fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

// Go matchSessions: indices of the sessions whose id starts with prefix.
pub fn match_sessions(l: &[SessionInfo], prefix: &str) -> Vec<usize> {
    l.iter()
        .enumerate()
        .filter(|(_, s)| s.id.starts_with(prefix))
        .map(|(i, _)| i)
        .collect()
}

// Go formatSessionList: one line per session plus the /view hint.
pub fn format_session_list(l: &[&SessionInfo]) -> String {
    let mut lines: Vec<String> = l
        .iter()
        .map(|s| {
            format!(
                "{}  {}  {:>3} turns  {}",
                short_id(&s.id),
                stamp(s.modified, false),
                s.turns,
                s.first
            )
        })
        .collect();
    lines.push("view one: /view <id-prefix>".to_string());
    lines.join("\n")
}

// Go wheelSel: one row per wheel notch, wrapping around. Shared with the /resume picker.
pub fn wheel_sel(sel: &mut usize, n: usize, up: bool) {
    if n == 0 {
        return;
    }
    *sel = if up {
        (*sel + n - 1) % n
    } else {
        (*sel + 1) % n
    };
}

// Result of a key press on the /view overlay.
#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    // esc/q/ctrl+c at level 1: leave /view (caller moves the transcript to the bottom).
    Close,
    // enter on a session that could not be read: show this note in the transcript (Go note block).
    Note(String),
}

// Go sessView: list (level 1), sel, and for level 2 the chosen session id, its prompts (turns) and cur.
#[derive(Debug, Default)]
pub struct SessView {
    pub list: Vec<SessionInfo>,
    // Purpose by session id (App: "main", or the resident role), for the names "<model>-<purpose>(<first prompt>)".
    pub purposes: HashMap<String, String>,
    pub sel: usize,
    pub id: String,
    pub turns: Option<Vec<String>>,
    pub cur: usize,
    // Go sv.off: top body line of the shown prompt (level 2 scroll).
    pub off: usize,
}

impl SessView {
    // Go openSessView. Empty arg opens level 1. A prefix that matches one session opens its prompts
    // (level 2). Err is the message to show when /view does not open.
    pub fn open(list: Vec<SessionInfo>, arg: &str) -> Result<Self, String> {
        let mut v = SessView {
            list,
            ..Default::default()
        };
        if arg.is_empty() {
            return Ok(v);
        }
        let ms = match_sessions(&v.list, arg);
        match ms.len() {
            0 => return Err(format!("no session matching {arg}")),
            1 => {}
            _ => {
                let refs: Vec<&SessionInfo> = ms.iter().map(|&i| &v.list[i]).collect();
                return Err(format!(
                    "ambiguous prefix, matches:\n{}",
                    format_session_list(&refs)
                ));
            }
        }
        v.open_prompts(ms[0])?;
        Ok(v)
    }

    // Go openSessPrompts: loads the prompts of list[i] and moves to level 2.
    pub fn open_prompts(&mut self, i: usize) -> Result<(), String> {
        let s = &self.list[i];
        let prompts = read_turns(&s.path, s.agent.is_some()).map_err(|e| format!("view: {e}"))?;
        if prompts.is_empty() {
            return Err(format!("no prompts in session {}", short_id(&s.id)));
        }
        self.id = s.id.clone();
        self.turns = Some(prompts);
        self.cur = 0;
        self.off = 0;
        Ok(())
    }

    // Wheel: level 1 moves the list cursor (Go wheelSel); level 2 scrolls the prompt text 3 lines per notch
    // (Go tui.go svScroll off±3). width and vp_h are the terminal columns and the body rows.
    pub fn wheel(&mut self, up: bool, width: usize, vp_h: usize) {
        if self.turns.is_none() {
            wheel_sel(&mut self.sel, self.list.len(), up);
            return;
        }
        let top = if up {
            self.off.saturating_sub(3)
        } else {
            self.off + 3
        };
        self.scroll(width, vp_h, top);
    }

    // Display name of a session: "<model>-<purpose>(<first prompt, 40 chars>)", e.g. "haiku-coder(Fix the build)".
    // A sub-agent is "<subagent_type>(<description, 40 chars>)", e.g. "haiku-coder(Round 3 ...)".
    // Falls back to the 8-character id when the purpose or the model is unknown.
    pub fn name_of(&self, id: &str) -> String {
        let Some(s) = self.list.iter().find(|s| s.id == id) else {
            return short_id(id);
        };
        if let Some(agent) = &s.agent {
            return match (agent.kind.as_str(), agent.description.as_str()) {
                ("", _) => short_id(id),
                (kind, "") => kind.to_string(),
                (kind, desc) => format!("{kind}({})", trunc_chars(desc, 40)),
            };
        }
        match self.purposes.get(id) {
            Some(purpose) if !s.model.is_empty() => {
                let family = s
                    .model
                    .trim_start_matches("claude-")
                    .split('-')
                    .next()
                    .unwrap_or("");
                format!("{family}-{purpose}({})", trunc_chars(&s.first, 40))
            }
            _ => short_id(id),
        }
    }

    // Go svBody: the shown prompt wrapped to the overlay width (at least 10 cells). Empty at level 1.
    fn body(&self, width: usize) -> Vec<String> {
        let Some(text) = self.turns.as_ref().and_then(|t| t.get(self.cur)) else {
            return Vec::new();
        };
        wrap_cells(text, width.saturating_sub(2).max(10))
            .split('\n')
            .map(str::to_string)
            .collect()
    }

    // Go svScroll: sets the top body line, clamped so the last body line can reach the bottom of the
    // body window (vp_h - 1 rows, at least 1).
    fn scroll(&mut self, width: usize, vp_h: usize, top: usize) {
        let len = self.body(width).len();
        let room = vp_h.saturating_sub(1).max(1);
        self.off = top.min(len.saturating_sub(room));
    }

    // Go svKey. Level 2: up/down/k/j/pgup/pgdn/home/end scroll the prompt text, left/right step through the
    // prompts (resetting the scroll), esc goes back to level 1. Level 1: up/down (k/j) select, enter opens
    // the prompts, esc/q/ctrl+c leave /view.
    pub fn key(&mut self, key: KeyEvent, width: usize, vp_h: usize) -> Action {
        let plain = key.modifiers == KeyModifiers::NONE;
        let ctrl_c = key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL;
        if let Some(turns) = &self.turns {
            let last = turns.len().saturating_sub(1);
            let room = vp_h.saturating_sub(1).max(1);
            match key.code {
                KeyCode::Left => {
                    self.cur = self.cur.saturating_sub(1);
                    self.off = 0;
                }
                KeyCode::Right => {
                    self.cur = (self.cur + 1).min(last);
                    self.off = 0;
                }
                KeyCode::Up | KeyCode::Char('k') if plain => {
                    self.scroll(width, vp_h, self.off.saturating_sub(1))
                }
                KeyCode::Down | KeyCode::Char('j') if plain => {
                    self.scroll(width, vp_h, self.off + 1)
                }
                KeyCode::PageUp => self.scroll(width, vp_h, self.off.saturating_sub(room)),
                KeyCode::PageDown => self.scroll(width, vp_h, self.off + room),
                KeyCode::Home => self.scroll(width, vp_h, 0),
                KeyCode::End => self.scroll(width, vp_h, usize::MAX),
                KeyCode::Esc => {
                    self.turns = None;
                    self.cur = 0;
                    self.off = 0;
                }
                KeyCode::Char('q') if plain => return Action::Close,
                _ if ctrl_c => return Action::Close,
                _ => {}
            }
            return Action::None;
        }
        let n = self.list.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') if plain && n > 0 => wheel_sel(&mut self.sel, n, true),
            KeyCode::Down | KeyCode::Char('j') if plain && n > 0 => {
                wheel_sel(&mut self.sel, n, false)
            }
            KeyCode::Enter if self.sel < n => {
                if let Err(e) = self.open_prompts(self.sel) {
                    return Action::Note(e);
                }
            }
            KeyCode::Esc => return Action::Close,
            KeyCode::Char('q') if plain => return Action::Close,
            _ if ctrl_c => return Action::Close,
            _ => {}
        }
        Action::None
    }

    // Go sessLines: the level 1 list below the input box, in the same row style as the background panel.
    // Empty at level 2 (the transcript area shows the prompts) or when width < 10.
    pub fn list_lines(&self, width: usize, now: SystemTime) -> Vec<String> {
        if self.turns.is_some() || width < 10 {
            return Vec::new();
        }
        let mut out = vec![format!(
            "{ORANGE_ON}{}{FG_OFF}",
            clip("Sessions  (↑/↓ select · enter open · esc close)", width)
        )];
        let n = self.list.len();
        let from = self
            .sel
            .saturating_sub(SESS_ROWS / 2)
            .min(n.saturating_sub(SESS_ROWS));
        for i in from..n.min(from + SESS_ROWS) {
            let s = &self.list[i];
            // Go time.Since(Mod) < sessActiveWindow; a future mtime counts as active.
            let active = match now.duration_since(s.modified) {
                Ok(d) => d < ACTIVE_WINDOW,
                Err(_) => true,
            };
            let mark = if active { "● " } else { "  " };
            // Go: a sub-agent sits two cells in per level of nesting under its parent.
            let depth = s.agent.as_ref().map_or(0, |a| a.depth);
            let t = format!(
                "{mark}{}{}  {}",
                "  ".repeat(depth),
                self.name_of(&s.id),
                s.label()
            );
            out.push(if i == self.sel {
                format!("{ORANGE_ON}{}{FG_OFF}", clip(&format!("> {t}"), width))
            } else {
                format!("{GREY_ON}{}{FG_OFF}", clip(&format!("  {t}"), width))
            });
        }
        out
    }

    // Go svPromptLines: level 2, exactly hh rows that replace the transcript. Empty at level 1.
    // Row 0 is "<id>  prompt n/N" with the dim "lines a-b/len" counter; the rest is the body window
    // [off, off + hh - 1). The header is the short session id (Go uses the agent label, which Rust does not carry).
    pub fn prompt_lines(&self, width: usize, hh: usize) -> Vec<String> {
        let Some(turns) = &self.turns else {
            return Vec::new();
        };
        if turns.get(self.cur).is_none() {
            return Vec::new();
        }
        let body = self.body(width);
        let room = hh.saturating_sub(1).max(1);
        let off = self.off.min(body.len().saturating_sub(room));
        let end = body.len().min(off + room);
        let head_plain = format!(
            "{}  prompt {}/{}",
            self.name_of(&self.id),
            self.cur + 1,
            turns.len()
        );
        let head_text = clip(&head_plain, width);
        let hint = clip(
            &format!(
                "  lines {}-{}/{} · ↑/↓ pgup/pgdn scroll · ←/→ prev/next · esc back",
                (off + 1).min(end),
                end,
                body.len()
            ),
            width.saturating_sub(UnicodeWidthStr::width(head_text.as_str())),
        );
        let head = format!("{ORANGE_ON}{head_text}{FG_OFF}{DIM_ON}{hint}{RESET}");
        let mut l = vec![head];
        l.extend(body[off..end].iter().cloned());
        l.resize(hh, String::new());
        l
    }
}

// Local date and time parts [year, month, day, hour, minute] of t (Go time.Format in local time).
fn local_parts(t: SystemTime) -> [i64; 5] {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let [y, mo, d, h, mi, _] = chaude_core::localtime::local_civil(secs);
    [y, mo, d, h, mi]
}

// Go Format("2006-01-02 15:04") when year is true, else Format("01-02 15:04").
pub fn stamp(t: SystemTime, year: bool) -> String {
    let [y, mo, d, h, mi] = local_parts(t);
    if year {
        format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}")
    } else {
        format!("{mo:02}-{d:02} {h:02}:{mi:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn write_session(dir: &Path, id: &str, mod_time: SystemTime, prompts: &[&str]) {
        let mut body = String::new();
        for p in prompts {
            body += &serde_json::json!({"type": "user", "message": {"role": "user", "content": p}})
                .to_string();
            body += "\n";
            body += &serde_json::json!({"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": "ok"}]}})
                .to_string();
            body += "\n";
        }
        let f = dir.join(format!("{id}.jsonl"));
        std::fs::write(&f, body).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&f)
            .unwrap()
            .set_modified(mod_time)
            .unwrap();
    }

    // Two sessions, newest first: sessaaa1 (now, 2 prompts) and sessbbb2 (1 hour ago, 1 prompt).
    fn fixture(tag: &str) -> (PathBuf, Vec<SessionInfo>) {
        let dir =
            std::env::temp_dir().join(format!("chaude-sessview-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let now = SystemTime::now();
        write_session(&dir, "sessaaa1", now, &["first prompt", "second prompt"]);
        write_session(
            &dir,
            "sessbbb2",
            now - Duration::from_secs(3600),
            &["older prompt"],
        );
        let list = list_sessions(&dir);
        (dir, list)
    }

    #[test]
    fn parse_prompts_follows_go_turn_rules() {
        let content = [
            r#"{"type":"user","message":{"content":[{"type":"text","text":"zero"}]}}"#,
            r#"{"type":"user","isMeta":true,"message":{"content":"meta"}}"#,
            r#"{"type":"user","isSidechain":true,"message":{"content":"side"}}"#,
            r#"{"type":"user","message":{"content":"<command-name>/x</command-name>"}}"#,
            r#"{"type":"user","message":{"content":"one"}}"#,
            r#"{"type":"user","message":{"content":[{"type":"text","text":"ignored"}]}}"#,
            "not json",
            "",
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"ok"}]}}"#,
        ]
        .join("\n");
        assert_eq!(parse_prompts(&content), ["zero", "one"]);
    }

    #[test]
    fn list_skips_empty_and_orders_newest_first() {
        let (dir, _) = fixture("list");
        std::fs::write(dir.join("empty.jsonl"), "{}\n").unwrap();
        write_session(
            &dir,
            "spaced",
            SystemTime::now(),
            &["first  old", "last old"],
        );
        let l = list_sessions(&dir);
        let ids: Vec<&str> = l.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids[0], "spaced");
        assert!(
            ids.contains(&"sessaaa1") && !ids.contains(&"empty"),
            "{ids:?}"
        );
        let spaced = &l[0];
        assert_eq!((spaced.turns, spaced.first.as_str()), (2, "first old"));
        assert_eq!(spaced.last, "last old");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn project_dir_replaces_non_alnum_with_dash() {
        assert_eq!(
            project_dir(Path::new("/home/u"), "/work/proj"),
            PathBuf::from("/home/u/.claude/projects/-work-proj")
        );
        assert_eq!(
            project_dir(Path::new("/h"), "/a.b c"),
            PathBuf::from("/h/.claude/projects/-a-b-c")
        );
    }

    #[test]
    fn local_parts_is_the_shared_local_clock() {
        // 1700000000 = 2023-11-14 22:13:20 UTC; the zone is the host's, so compare with the core helper.
        let t = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let [y, mo, d, h, mi, _] = chaude_core::localtime::local_civil(1_700_000_000);
        assert_eq!(local_parts(t), [y, mo, d, h, mi]);
    }

    // Go TestViewEscBacksOutToNormalScreen (pure part): the list draws below the input and esc leaves.
    #[test]
    fn list_draws_sessions_and_esc_closes() {
        let (dir, list) = fixture("draw");
        let mut v = SessView::open(list, "").unwrap();
        let rows = v.list_lines(80, SystemTime::now());
        assert_eq!(rows.len(), 3, "header + 2 sessions");
        assert!(rows[0].contains("Sessions"));
        let all = rows.join("\n");
        assert!(all.contains("sessaaa1") && all.contains("● "), "{all}");
        assert_eq!(v.key(press(KeyCode::Esc), 80, 6), Action::Close);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Go TestViewSessionSelectionAndPromptNavigation.
    #[test]
    fn selection_and_prompt_navigation_follow_go() {
        let (dir, list) = fixture("nav");
        let now = SystemTime::now();
        let mut v = SessView::open(list, "").unwrap();
        assert_eq!(v.key(press(KeyCode::Down), 80, 6), Action::None);
        assert_eq!(v.sel, 1, "arrow down selects the second session");
        v.key(press(KeyCode::Up), 80, 6);
        assert_eq!(v.key(press(KeyCode::Enter), 80, 6), Action::None);
        assert!(v.turns.is_some());
        let shown = v.prompt_lines(80, 6).join("\n");
        assert!(
            shown.contains("prompt 1/2") && shown.contains("first prompt"),
            "{shown}"
        );
        assert!(v.list_lines(80, now).is_empty(), "level 2 drops the list");
        v.key(press(KeyCode::Right), 80, 6);
        assert!(v.prompt_lines(80, 6).join("\n").contains("second prompt"));
        v.key(press(KeyCode::Esc), 80, 6);
        assert!(v.turns.is_none(), "esc from prompts returns to the list");
        assert_eq!(v.key(press(KeyCode::Esc), 80, 6), Action::Close);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Go TestViewArgOpensSessionDirectly.
    #[test]
    fn view_arg_opens_session_directly() {
        let (dir, list) = fixture("arg");
        let v = SessView::open(list.clone(), "sessbbb").unwrap();
        assert_eq!(v.turns, Some(vec!["older prompt".to_string()]));
        assert_eq!(
            SessView::open(list.clone(), "nope").err().as_deref(),
            Some("no session matching nope")
        );
        let amb = SessView::open(list, "sess").err().unwrap_or_default();
        assert!(
            amb.starts_with("ambiguous prefix, matches:\n")
                && amb.contains("view one: /view <id-prefix>"),
            "{amb}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Go TestViewListsOnlyOwnSessions.
    #[test]
    fn view_lists_only_own_sessions() {
        let (dir, list) = fixture("own");
        let none = HashSet::new();
        assert_eq!(
            own_sessions(list.clone(), &none).err().as_deref(),
            Some(NO_OWN)
        );
        let mine: HashSet<String> = ["sessaaa1".to_string()].into();
        let own = own_sessions(list, &mine).unwrap();
        assert_eq!(own.len(), 1);
        assert_eq!(own[0].id, "sessaaa1");
        assert_eq!(
            SessView::open(own.clone(), "sessbbb").err().as_deref(),
            Some("no session matching sessbbb")
        );
        let v = SessView::open(own, "").unwrap();
        assert_eq!(v.list.len(), 1);
        assert!(
            !v.list_lines(80, SystemTime::now())
                .join("\n")
                .contains("sessbbb2")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Go TestViewWheelMovesSessionCursor (wraps around).
    #[test]
    fn wheel_moves_session_cursor_and_wraps() {
        let (dir, list) = fixture("wheel");
        let mut v = SessView::open(list, "").unwrap();
        v.wheel(false, 80, 6);
        assert_eq!(v.sel, 1, "wheel down");
        v.wheel(false, 80, 6);
        assert_eq!(v.sel, 0, "wheel down past end wraps");
        v.wheel(true, 80, 6);
        assert_eq!(v.sel, 1, "wheel up wraps");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Go svKey / svScroll / svPromptLines: level 2 scrolls the body (clamped to vp_h - 1 rows), paging and
    // home/end move the window, left/right reset it, and the header carries the lines a-b/len counter.
    #[test]
    fn prompt_scroll_is_clamped_and_resets_on_switch() {
        let long: String = (0..10).map(|i| format!("l{i}\n")).collect();
        let mut v = SessView {
            id: "sessaaa1".into(),
            turns: Some(vec![long.trim_end().to_string(), "second".into()]),
            ..Default::default()
        };
        let (w, h) = (80, 6);
        assert!(v.prompt_lines(w, h)[0].contains("sessaaa1  prompt 1/2"));
        assert!(v.prompt_lines(w, h)[0].contains("lines 1-5/10"));
        v.key(press(KeyCode::Down), w, h);
        assert_eq!(v.off, 1);
        for _ in 0..10 {
            v.key(press(KeyCode::Down), w, h);
        }
        assert_eq!(
            v.off, 5,
            "scroll stops when the last line reaches the bottom"
        );
        assert!(v.prompt_lines(w, h)[0].contains("lines 6-10/10"));
        assert_eq!(v.prompt_lines(w, h)[1], "l5");
        v.key(press(KeyCode::Home), w, h);
        assert_eq!(v.off, 0);
        v.key(press(KeyCode::End), w, h);
        assert_eq!(v.off, 5);
        v.key(press(KeyCode::PageUp), w, h);
        assert_eq!(v.off, 0, "page up saturates at the top");
        v.wheel(false, w, h);
        assert_eq!(v.off, 3, "wheel down scrolls three lines");
        v.key(press(KeyCode::Right), w, h);
        assert_eq!(
            (v.cur, v.off),
            (1, 0),
            "right steps the prompt and resets the scroll"
        );
        assert_eq!(v.key(press(KeyCode::Esc), w, h), Action::None);
        assert!(v.turns.is_none());
    }

    #[test]
    fn prompt_view_scrolls_inside_transcript_like_go() {
        // Go TestViewPromptScrollsInsideTranscript (sessview_test.go:139).
        let (w, h) = (80, 10);
        let mut v = SessView {
            id: "sessaaa1".into(),
            turns: Some(vec!["long prompt text ".repeat(200)]),
            ..Default::default()
        };
        let len = v.body(w).len();
        let last = len - (h - 1);
        assert!(last > 3, "prompt too short to scroll: body={len} lines");
        v.key(press(KeyCode::Down), w, h);
        assert_eq!(v.off, 1);
        v.key(press(KeyCode::PageDown), w, h);
        assert!(v.off > 1, "pgdown did not move: off={}", v.off);
        v.key(press(KeyCode::End), w, h);
        assert_eq!(v.off, last);
        v.key(press(KeyCode::Down), w, h);
        assert_eq!(v.off, last, "down past end clamps");
        v.key(press(KeyCode::Home), w, h);
        assert_eq!(v.off, 0);
        v.wheel(false, w, h);
        assert_eq!(v.off, 3, "wheel down scrolls three lines");
        v.wheel(true, w, h);
        assert_eq!(v.off, 0, "wheel up returns to the top");
    }

    // Names: "<model>-<purpose>(<first prompt, 40 chars>)"; the 8-char id stays when the purpose or the model is unknown.
    #[test]
    fn session_names_are_model_purpose_and_fall_back_to_the_id() {
        let info = |id: &str, model: &str, first: &str| SessionInfo {
            id: id.into(),
            path: PathBuf::from(format!("{id}.jsonl")),
            modified: SystemTime::now(),
            turns: 1,
            first: first.into(),
            last: first.into(),
            model: model.into(),
            agent: None,
        };
        let mut v = SessView {
            list: vec![
                info("sessaaa1", "claude-haiku-5-5", "Fix the build"),
                info("sessbbb2", "claude-opus-5-5", &"가".repeat(50)),
                info("sessccc3", "", "no model"),
            ],
            purposes: HashMap::from([
                ("sessaaa1".to_string(), "coder".to_string()),
                ("sessbbb2".to_string(), "main".to_string()),
                ("sessccc3".to_string(), "main".to_string()),
            ]),
            ..Default::default()
        };
        assert_eq!(v.name_of("sessaaa1"), "haiku-coder(Fix the build)");
        assert_eq!(
            v.name_of("sessbbb2"),
            format!("opus-main({})", "가".repeat(40))
        );
        assert_eq!(v.name_of("sessccc3"), "sessccc3", "no model: the 8-char id");
        assert_eq!(
            v.name_of("unknown99"),
            "unknown9",
            "not listed: the 8-char id"
        );
        v.purposes.remove("sessaaa1");
        assert_eq!(
            v.name_of("sessaaa1"),
            "sessaaa1",
            "no purpose: the 8-char id"
        );
        v.purposes.insert("sessaaa1".into(), "coder".into());
        let rows = v.list_lines(120, SystemTime::now()).join("\n");
        assert!(rows.contains("haiku-coder(Fix the build)"), "{rows}");
        assert!(
            !rows.contains("sessaaa1"),
            "list shows the name, not the id: {rows}"
        );
        // A sub-agent with neither launch nor meta (no subagent_type) keeps its 8-char id.
        let agent = SessionInfo {
            agent: Some(AgentRef {
                kind: String::new(),
                description: "no launch".into(),
                depth: 1,
            }),
            ..info("agentzz9", "", "no launch")
        };
        v.list.push(agent);
        assert_eq!(v.name_of("agentzz9"), "agentzz9");
    }

    // Real session cut from ~/.claude/projects (testdata/view_subagents): the Agent call and its tool_result
    // name each agent, and the nested haiku-coder (spawned inside haiku-prompter's transcript) sits one level
    // deeper. Its prompts open from its own transcript.
    #[test]
    fn subagents_are_named_from_the_launch_and_nest_under_their_parent() {
        let root = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/tui/testdata/view_subagents"
        ));
        let sessions = list_sessions(root);
        assert_eq!(sessions.len(), 1, "one top-level session");
        let list = with_subagents(sessions, root);
        let ids: Vec<&str> = list.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "9bbfbca7-3109-47f2-84f4-d6c4e268fa2a",
                "a03d20cc60ceb17ef",
                "a34c0230cc855700d"
            ]
        );
        let mut v = SessView::open(list, "").unwrap();
        assert_eq!(
            v.name_of("a03d20cc60ceb17ef"),
            "haiku-prompter(Loop: drain sink + idle gate, checker)"
        );
        assert_eq!(
            v.name_of("a34c0230cc855700d"),
            "haiku-coder(Round 3: verify loop4 R5 item2 role case)"
        );
        assert_eq!(v.list[1].agent.as_ref().unwrap().depth, 1);
        assert_eq!(v.list[2].agent.as_ref().unwrap().depth, 2);
        // Far-future now: every row is inactive, so the mark is blank and the indent is the only difference.
        let later = SystemTime::now() + Duration::from_secs(365 * 24 * 3600);
        let rows = v.list_lines(120, later).join("\n");
        // Row = 2-cell gutter + 2-cell blank mark + 2 cells per level.
        assert!(
            rows.contains("      haiku-prompter(") && !rows.contains("        haiku-prompter("),
            "depth 1 is indented two cells: {rows}"
        );
        assert!(
            rows.contains("        haiku-coder(") && !rows.contains("          haiku-coder("),
            "depth 2 is indented four cells: {rows}"
        );
        v.open_prompts(2).unwrap();
        assert!(
            v.turns.as_ref().unwrap()[0].starts_with("task: (a) commit"),
            "the nested agent's prompt reads from its own transcript"
        );
    }

    // Without the launch (its tool_use and tool_result are cut from the parent), meta.json still names the agent.
    #[test]
    fn meta_names_an_agent_whose_launch_is_missing() {
        let src = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/tui/testdata/view_subagents"
        ));
        let sid = "9bbfbca7-3109-47f2-84f4-d6c4e268fa2a";
        let dir = std::env::temp_dir().join(format!("chaude-view-meta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let subs = dir.join(sid).join("subagents");
        std::fs::create_dir_all(&subs).unwrap();
        let session = std::fs::read_to_string(src.join(format!("{sid}.jsonl"))).unwrap();
        let cut: String = session
            .lines()
            .filter(|l| !l.contains("toolu_01JggA5W2dudwT3XFPNPDCBt"))
            .map(|l| format!("{l}\n"))
            .collect();
        std::fs::write(dir.join(format!("{sid}.jsonl")), cut).unwrap();
        for f in [
            "agent-a03d20cc60ceb17ef.jsonl",
            "agent-a03d20cc60ceb17ef.meta.json",
            "agent-a34c0230cc855700d.jsonl",
            "agent-a34c0230cc855700d.meta.json",
        ] {
            std::fs::copy(src.join(sid).join("subagents").join(f), subs.join(f)).unwrap();
        }
        let sessions = list_sessions(&dir);
        assert_eq!(sessions.len(), 1, "the cut session keeps its prompt");
        let v = SessView::open(with_subagents(sessions, &dir), "").unwrap();
        assert_eq!(
            v.name_of("a03d20cc60ceb17ef"),
            "haiku-prompter(Loop: drain sink + idle gate, checker)",
            "from meta.json"
        );
        assert_eq!(
            v.list[1].agent.as_ref().unwrap().depth,
            1,
            "no parentAgentId: under the session"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_model_reads_the_first_assistant_model() {
        let content = "{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}\nnot json\n{\"type\":\"assistant\",\"message\":{\"model\":\"claude-haiku-5-5\"}}\n";
        assert_eq!(parse_model(content), "claude-haiku-5-5");
        assert_eq!(parse_model("{}\n"), "");
    }

    #[test]
    fn list_match_and_prompts_follow_go_view_helpers() {
        // Partial port of Go TestViewHelpers (resume_test.go:42): list, prefix match, list format and prompts.
        // Go formatTranscript (resume.go:175) has no Rust counterpart, so the stub stays ignored.
        let (dir, list) = fixture("helpers");
        assert_eq!(match_sessions(&list, "sessa").len(), 1);
        assert_eq!(match_sessions(&list, "sess").len(), 2);
        let refs: Vec<&SessionInfo> = list.iter().collect();
        let text = format_session_list(&refs);
        assert!(
            text.contains("sessaaa1") && text.contains("first prompt"),
            "{text}"
        );
        assert_eq!(
            read_prompts(&list[0].path).unwrap(),
            vec!["first prompt".to_string(), "second prompt".to_string()]
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
