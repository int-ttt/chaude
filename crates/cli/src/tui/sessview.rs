//! /view session browser (Go sessview.go; resume.go session list; importclaude.go readClaudeTranscript).
//! Level 1: the Claude Code sessions this chaude process spawned, listed below the input box.
//! Level 2: the user prompts of one session, one at a time. Esc goes back one level.
//! The readers here take paths and file text; the overlay returns Actions and the caller owns the transcript.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthStr;

use super::bar::clip;
use super::render::{DIM_ON, FG_OFF, ORANGE_ON, RESET, trunc_chars, wrap_cells};

// Go sessActiveWindow: a session written this recently is marked active (●).
pub const ACTIVE_WINDOW: Duration = Duration::from_secs(10 * 60);
// Go sessRows: session rows shown below the input box at once.
pub const SESS_ROWS: usize = 5;
// Go ownSessionList: shown when this process has no Claude Code session yet.
pub const NO_OWN: &str = "no Claude Code sessions from this chaude session yet";

// One Claude Code transcript (Go sessionInfo; the fields /view and the list need).
#[derive(Clone, Debug)]
pub struct SessionInfo {
    pub id: String,
    pub path: PathBuf,
    pub modified: SystemTime,
    pub turns: usize,
    pub first: String,
    pub last: String,
}

// Go readClaudeTranscript, user prompts only. A user text event that is not meta or sidechain starts a
// turn; a plain string starts a new turn unless it begins with "<", and array text fills the turn only
// while it is still empty. Assistant events and the rest of the transcript are not needed here.
pub fn parse_prompts(content: &str) -> Vec<String> {
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
            || ev["isSidechain"].as_bool() == Some(true)
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

fn flush(out: &mut Vec<String>, cur: &mut String) {
    if !cur.is_empty() {
        out.push(std::mem::take(cur));
    }
}

// Go readClaudeTranscript errors: an unreadable file, or no turns at all.
pub fn read_prompts(path: &Path) -> Result<Vec<String>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let prompts = parse_prompts(&String::from_utf8_lossy(&bytes));
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
        let Ok(prompts) = read_prompts(&path) else {
            continue;
        };
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

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

// Go sessionList: the sessions of cwd's Claude Code project dir. Empty cwd means the process cwd.
pub fn session_list(cwd: &str) -> Result<Vec<SessionInfo>, String> {
    let home = home_dir().ok_or_else(|| "$HOME is not defined".to_string())?;
    let cwd = if cwd.is_empty() {
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    } else {
        cwd.to_string()
    };
    let dir = project_dir(&home, &cwd);
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
    pub sel: usize,
    pub id: String,
    pub turns: Option<Vec<String>>,
    pub cur: usize,
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
        let prompts = read_prompts(&s.path).map_err(|e| format!("view: {e}"))?;
        if prompts.is_empty() {
            return Err(format!("no prompts in session {}", short_id(&s.id)));
        }
        self.id = s.id.clone();
        self.turns = Some(prompts);
        self.cur = 0;
        Ok(())
    }

    // Wheel over the level 1 list (Go wheelSel on the /view cursor). Level 2 ignores the wheel.
    pub fn wheel(&mut self, up: bool) {
        if self.turns.is_none() {
            wheel_sel(&mut self.sel, self.list.len(), up);
        }
    }

    // Go svKey. Level 2: arrows and paging step through prompts, esc goes back to level 1. Level 1: up/down
    // (k/j) select, enter opens the prompts, esc/q/ctrl+c leave /view.
    pub fn key(&mut self, key: KeyEvent) -> Action {
        let plain = key.modifiers == KeyModifiers::NONE;
        let ctrl_c = key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL;
        if let Some(turns) = &self.turns {
            let last = turns.len().saturating_sub(1);
            match key.code {
                KeyCode::Left | KeyCode::Up | KeyCode::PageUp => {
                    self.cur = self.cur.saturating_sub(1)
                }
                KeyCode::Char('k') if plain => self.cur = self.cur.saturating_sub(1),
                KeyCode::Right | KeyCode::Down | KeyCode::PageDown => {
                    self.cur = (self.cur + 1).min(last)
                }
                KeyCode::Char('j') if plain => self.cur = (self.cur + 1).min(last),
                KeyCode::Home => self.cur = 0,
                KeyCode::End => self.cur = last,
                KeyCode::Esc => {
                    self.turns = None;
                    self.cur = 0;
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
            let t = format!("{mark}{}  {}", short_id(&s.id), s.label());
            out.push(if i == self.sel {
                format!("{ORANGE_ON}{}{FG_OFF}", clip(&format!("> {t}"), width))
            } else {
                format!("{DIM_ON}{}{RESET}", clip(&format!("  {t}"), width))
            });
        }
        out
    }

    // Go svPromptLines: level 2, exactly hh rows that replace the transcript. Empty at level 1.
    // Prompt text wraps to width-2 cells; when it does not fit, the last shown row is "… (+N lines)".
    pub fn prompt_lines(&self, width: usize, hh: usize) -> Vec<String> {
        let Some(turns) = &self.turns else {
            return Vec::new();
        };
        let Some(text) = turns.get(self.cur) else {
            return Vec::new();
        };
        let head_plain = format!(
            "Session {}  prompt {}/{}",
            short_id(&self.id),
            self.cur + 1,
            turns.len()
        );
        let head_text = clip(&head_plain, width);
        let hint = clip(
            "  ←/→ prev/next · esc back",
            width.saturating_sub(UnicodeWidthStr::width(head_text.as_str())),
        );
        let head = format!("{ORANGE_ON}{head_text}{FG_OFF}{DIM_ON}{hint}{RESET}");
        let cw = width.saturating_sub(2).max(10);
        let all: Vec<String> = wrap_cells(text, cw)
            .split('\n')
            .map(str::to_string)
            .collect();
        let room = hh.saturating_sub(1).max(1);
        let mut l = vec![head];
        if all.len() > room {
            l.extend(all[..room - 1].iter().cloned());
            l.push(format!(
                "{DIM_ON}… (+{} lines){RESET}",
                all.len() - (room - 1)
            ));
        } else {
            l.extend(all);
        }
        l.resize(hh, String::new());
        l
    }
}

// Local date and time parts [year, month, day, hour, minute] of t (Go time.Format in local time).
#[cfg(unix)]
fn local_parts(t: SystemTime) -> [i64; 5] {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as libc::time_t)
        .unwrap_or(0);
    // SAFETY: localtime_r writes into the zeroed tm we own and reads only the local `secs`.
    let tm = unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&secs, &mut tm);
        tm
    };
    [
        tm.tm_year as i64 + 1900,
        tm.tm_mon as i64 + 1,
        tm.tm_mday as i64,
        tm.tm_hour as i64,
        tm.tm_min as i64,
    ]
}

// No libc off unix: UTC, so the list stays well-formed.
#[cfg(not(unix))]
fn local_parts(t: SystemTime) -> [i64; 5] {
    utc_parts(
        t.duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    )
}

// UTC date and time parts for unix seconds (civil-from-days).
#[cfg(any(not(unix), test))]
fn utc_parts(secs: i64) -> [i64; 5] {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let rem = secs.rem_euclid(86_400);
    [y, m, d, rem / 3600, rem % 3600 / 60]
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
    fn utc_parts_matches_known_epoch() {
        // 1700000000 = 2023-11-14 22:13:20 UTC.
        assert_eq!(utc_parts(1_700_000_000), [2023, 11, 14, 22, 13]);
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
        assert_eq!(v.key(press(KeyCode::Esc)), Action::Close);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Go TestViewSessionSelectionAndPromptNavigation.
    #[test]
    fn selection_and_prompt_navigation_follow_go() {
        let (dir, list) = fixture("nav");
        let now = SystemTime::now();
        let mut v = SessView::open(list, "").unwrap();
        assert_eq!(v.key(press(KeyCode::Down)), Action::None);
        assert_eq!(v.sel, 1, "arrow down selects the second session");
        v.key(press(KeyCode::Up));
        assert_eq!(v.key(press(KeyCode::Enter)), Action::None);
        assert!(v.turns.is_some());
        let shown = v.prompt_lines(80, 6).join("\n");
        assert!(
            shown.contains("prompt 1/2") && shown.contains("first prompt"),
            "{shown}"
        );
        assert!(v.list_lines(80, now).is_empty(), "level 2 drops the list");
        v.key(press(KeyCode::Right));
        assert!(v.prompt_lines(80, 6).join("\n").contains("second prompt"));
        v.key(press(KeyCode::Esc));
        assert!(v.turns.is_none(), "esc from prompts returns to the list");
        assert_eq!(v.key(press(KeyCode::Esc)), Action::Close);
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
        v.wheel(false);
        assert_eq!(v.sel, 1, "wheel down");
        v.wheel(false);
        assert_eq!(v.sel, 0, "wheel down past end wraps");
        v.wheel(true);
        assert_eq!(v.sel, 1, "wheel up wraps");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
