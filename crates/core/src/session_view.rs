//! The /view session browser model, shared by the TUI and the GUI (Go sessview.go; resume.go session list;
//! importclaude.go readClaudeTranscript).
//! Level 1: the Claude Code sessions this chaude process spawned, one cursor over them.
//! Level 2: the user prompts of one session, one at a time. The readers take paths and file text; the callers draw
//! the rows and own the transcript.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::handoff;
use crate::tmp_layout;
use crate::ui_state::trunc_chars;

// Go sessActiveWindow: a session written this recently is marked active (●).
pub const ACTIVE_WINDOW: Duration = Duration::from_secs(10 * 60);
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

/// HOME, then USERPROFILE; shared with the core (`osenv::try_home_dir`).
pub fn home_dir() -> Option<PathBuf> {
    crate::osenv::try_home_dir()
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

// Result of a key press on the /view browser (the TUI maps its keys to it; the GUI reads its own).
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
    // Purpose by session id (the caller: "main", or the resident role), for the names "<model>-<purpose>(<first prompt>)".
    pub purposes: HashMap<String, String>,
    pub sel: usize,
    pub id: String,
    pub turns: Option<Vec<String>>,
    pub cur: usize,
    // Go sv.off: top body line of the shown prompt (level 2 scroll; the TUI counts wrapped lines).
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

    // Level 2 is open (the prompts of one session are shown).
    pub fn in_prompts(&self) -> bool {
        self.turns.is_some()
    }

    // Level 1: moves the list cursor one row, wrapping (Go wheelSel).
    pub fn move_sel(&mut self, up: bool) {
        wheel_sel(&mut self.sel, self.list.len(), up);
    }

    // Level 2: the previous prompt; the scroll resets.
    pub fn prev_prompt(&mut self) {
        self.cur = self.cur.saturating_sub(1);
        self.off = 0;
    }

    // Level 2: the next prompt, stopping at the last; the scroll resets.
    pub fn next_prompt(&mut self) {
        let last = self.turns.as_ref().map_or(0, Vec::len).saturating_sub(1);
        self.cur = (self.cur + 1).min(last);
        self.off = 0;
    }

    // Level 2 back to level 1 (Go svKey esc).
    pub fn leave_prompts(&mut self) {
        self.turns = None;
        self.cur = 0;
        self.off = 0;
    }

    // The prompt shown at level 2 (None at level 1).
    pub fn shown(&self) -> Option<&str> {
        self.turns.as_ref()?.get(self.cur).map(String::as_str)
    }

    // Go svScroll: sets the top body line, clamped so the last body line can reach the bottom of a window of
    // room rows. body_len is the number of wrapped body lines (the caller wraps to its width).
    pub fn scroll_to(&mut self, body_len: usize, room: usize, top: usize) {
        self.off = top.min(body_len.saturating_sub(room));
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

    // One level 1 row as text, before the caller's colour and clip: a "● " mark when the session was written within
    // ACTIVE_WINDOW (a future mtime counts as active), two cells per sub-agent level, the name and the label.
    pub fn row_text(&self, i: usize, now: SystemTime) -> String {
        let s = &self.list[i];
        // Go time.Since(Mod) < sessActiveWindow; a future mtime counts as active.
        let active = match now.duration_since(s.modified) {
            Ok(d) => d < ACTIVE_WINDOW,
            Err(_) => true,
        };
        let mark = if active { "● " } else { "  " };
        // Go: a sub-agent sits two cells in per level of nesting under its parent.
        let depth = s.agent.as_ref().map_or(0, |a| a.depth);
        format!(
            "{mark}{}{}  {}",
            "  ".repeat(depth),
            self.name_of(&s.id),
            s.label()
        )
    }

    // The level 2 header before the caller's colour and clip: "<name>  prompt n/N".
    pub fn prompt_head(&self) -> String {
        format!(
            "{}  prompt {}/{}",
            self.name_of(&self.id),
            self.cur + 1,
            self.turns.as_ref().map_or(0, Vec::len)
        )
    }
}

// The /view browser over the sessions of one Claude Code project dir (Go ownSessionList): only the sessions whose id
// owned names, with each one's sub-agents under it. Err is the message to show when /view does not open.
pub fn open_in(project: &Path, owned: &HashSet<String>, arg: &str) -> Result<SessView, String> {
    let all = list_sessions(project);
    if all.is_empty() {
        return Err(format!("no Claude Code sessions in {}", project.display()));
    }
    let list = own_sessions(all, owned).map(|l| with_subagents(l, project))?;
    SessView::open(list, arg)
}

// The /view browser for cwd: open_in over cwd's Claude Code project dir.
pub fn open(cwd: &str, owned: &HashSet<String>, arg: &str) -> Result<SessView, String> {
    open_in(&session_dir(cwd)?, owned, arg)
}

// The session ids the exit handoff of cwd names as owned by the previous run (empty without a handoff file).
pub fn handoff_owned(cwd: &str) -> HashSet<String> {
    let base = handoff::handoff_file(&tmp_layout::long_dir(Path::new(cwd)));
    handoff::read_exit_handoff(&handoff::read_path(&base, None))
        .map(|h| h.prev_owned().into_iter().collect())
        .unwrap_or_default()
}

// Local date and time parts [year, month, day, hour, minute] of t (Go time.Format in local time).
pub fn local_parts(t: SystemTime) -> [i64; 5] {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let [y, mo, d, h, mi, _] = crate::localtime::local_civil(secs);
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
