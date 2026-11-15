//! /view session browser drawing and keys (Go svKey, sessLines, svPromptLines). The model is the shared core
//! `chaude_core::session_view`; this file adds the terminal rows, the key map and the wrapped prompt body.

use std::time::SystemTime;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthStr;

use super::bar::clip;
use super::render::{DIM_ON, FG_OFF, GREY_ON, ORANGE_ON, RESET, wrap_cells};

pub use chaude_core::session_view::*;

// Go sessRows: session rows shown below the input box at once.
pub const SESS_ROWS: usize = 5;

// The body lines of the shown prompt, wrapped to the overlay width (at least 10 cells). Empty at level 1.
fn body_lines(sv: &SessView, width: usize) -> Vec<String> {
    let Some(text) = sv.shown() else {
        return Vec::new();
    };
    wrap_cells(text, width.saturating_sub(2).max(10))
        .split('\n')
        .map(str::to_string)
        .collect()
}

// Go svScroll: the clamp counts the wrapped body lines against vp_h - 1 rows (at least 1).
fn scroll(sv: &mut SessView, width: usize, vp_h: usize, top: usize) {
    let len = body_lines(sv, width).len();
    sv.scroll_to(len, vp_h.saturating_sub(1).max(1), top);
}

// Wheel: level 1 moves the list cursor (Go wheelSel); level 2 scrolls the prompt text 3 lines per notch
// (Go tui.go svScroll off±3). width and vp_h are the terminal columns and the body rows.
pub fn wheel(sv: &mut SessView, up: bool, width: usize, vp_h: usize) {
    if !sv.in_prompts() {
        sv.move_sel(up);
        return;
    }
    let top = if up {
        sv.off.saturating_sub(3)
    } else {
        sv.off + 3
    };
    scroll(sv, width, vp_h, top);
}

// Go svKey. Level 2: up/down/k/j/pgup/pgdn/home/end scroll the prompt text, left/right step through the
// prompts (resetting the scroll), esc goes back to level 1. Level 1: up/down (k/j) select, enter opens
// the prompts, esc/q/ctrl+c leave /view.
pub fn key(sv: &mut SessView, key: KeyEvent, width: usize, vp_h: usize) -> Action {
    let plain = key.modifiers == KeyModifiers::NONE;
    let ctrl_c = key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL;
    if sv.in_prompts() {
        let room = vp_h.saturating_sub(1).max(1);
        match key.code {
            KeyCode::Left => sv.prev_prompt(),
            KeyCode::Right => sv.next_prompt(),
            KeyCode::Up | KeyCode::Char('k') if plain => {
                let top = sv.off.saturating_sub(1);
                scroll(sv, width, vp_h, top)
            }
            KeyCode::Down | KeyCode::Char('j') if plain => {
                let top = sv.off + 1;
                scroll(sv, width, vp_h, top)
            }
            KeyCode::PageUp => {
                let top = sv.off.saturating_sub(room);
                scroll(sv, width, vp_h, top)
            }
            KeyCode::PageDown => {
                let top = sv.off + room;
                scroll(sv, width, vp_h, top)
            }
            KeyCode::Home => scroll(sv, width, vp_h, 0),
            KeyCode::End => scroll(sv, width, vp_h, usize::MAX),
            KeyCode::Esc => sv.leave_prompts(),
            KeyCode::Char('q') if plain => return Action::Close,
            _ if ctrl_c => return Action::Close,
            _ => {}
        }
        return Action::None;
    }
    let n = sv.list.len();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if plain && n > 0 => sv.move_sel(true),
        KeyCode::Down | KeyCode::Char('j') if plain && n > 0 => sv.move_sel(false),
        KeyCode::Enter if sv.sel < n => {
            let i = sv.sel;
            if let Err(e) = sv.open_prompts(i) {
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
pub fn list_lines(sv: &SessView, width: usize, now: SystemTime) -> Vec<String> {
    if sv.in_prompts() || width < 10 {
        return Vec::new();
    }
    let mut out = vec![format!(
        "{ORANGE_ON}{}{FG_OFF}",
        clip("Sessions  (↑/↓ select · enter open · esc close)", width)
    )];
    let n = sv.list.len();
    let from = sv
        .sel
        .saturating_sub(SESS_ROWS / 2)
        .min(n.saturating_sub(SESS_ROWS));
    for i in from..n.min(from + SESS_ROWS) {
        let t = sv.row_text(i, now);
        out.push(if i == sv.sel {
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
pub fn prompt_lines(sv: &SessView, width: usize, hh: usize) -> Vec<String> {
    if sv.shown().is_none() {
        return Vec::new();
    }
    let body = body_lines(sv, width);
    let room = hh.saturating_sub(1).max(1);
    let off = sv.off.min(body.len().saturating_sub(room));
    let end = body.len().min(off + room);
    let head_text = clip(&sv.prompt_head(), width);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, UNIX_EPOCH};

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
        let rows = list_lines(&v, 80, SystemTime::now());
        assert_eq!(rows.len(), 3, "header + 2 sessions");
        assert!(rows[0].contains("Sessions"));
        let all = rows.join("\n");
        assert!(all.contains("sessaaa1") && all.contains("● "), "{all}");
        assert_eq!(key(&mut v, press(KeyCode::Esc), 80, 6), Action::Close);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Go TestViewSessionSelectionAndPromptNavigation.
    #[test]
    fn selection_and_prompt_navigation_follow_go() {
        let (dir, list) = fixture("nav");
        let now = SystemTime::now();
        let mut v = SessView::open(list, "").unwrap();
        assert_eq!(key(&mut v, press(KeyCode::Down), 80, 6), Action::None);
        assert_eq!(v.sel, 1, "arrow down selects the second session");
        key(&mut v, press(KeyCode::Up), 80, 6);
        assert_eq!(key(&mut v, press(KeyCode::Enter), 80, 6), Action::None);
        assert!(v.turns.is_some());
        let shown = prompt_lines(&v, 80, 6).join("\n");
        assert!(
            shown.contains("prompt 1/2") && shown.contains("first prompt"),
            "{shown}"
        );
        assert!(list_lines(&v, 80, now).is_empty(), "level 2 drops the list");
        key(&mut v, press(KeyCode::Right), 80, 6);
        assert!(prompt_lines(&v, 80, 6).join("\n").contains("second prompt"));
        key(&mut v, press(KeyCode::Esc), 80, 6);
        assert!(v.turns.is_none(), "esc from prompts returns to the list");
        assert_eq!(key(&mut v, press(KeyCode::Esc), 80, 6), Action::Close);
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
            !list_lines(&v, 80, SystemTime::now())
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
        wheel(&mut v, false, 80, 6);
        assert_eq!(v.sel, 1, "wheel down");
        wheel(&mut v, false, 80, 6);
        assert_eq!(v.sel, 0, "wheel down past end wraps");
        wheel(&mut v, true, 80, 6);
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
        assert!(prompt_lines(&v, w, h)[0].contains("sessaaa1  prompt 1/2"));
        assert!(prompt_lines(&v, w, h)[0].contains("lines 1-5/10"));
        key(&mut v, press(KeyCode::Down), w, h);
        assert_eq!(v.off, 1);
        for _ in 0..10 {
            key(&mut v, press(KeyCode::Down), w, h);
        }
        assert_eq!(
            v.off, 5,
            "scroll stops when the last line reaches the bottom"
        );
        assert!(prompt_lines(&v, w, h)[0].contains("lines 6-10/10"));
        assert_eq!(prompt_lines(&v, w, h)[1], "l5");
        key(&mut v, press(KeyCode::Home), w, h);
        assert_eq!(v.off, 0);
        key(&mut v, press(KeyCode::End), w, h);
        assert_eq!(v.off, 5);
        key(&mut v, press(KeyCode::PageUp), w, h);
        assert_eq!(v.off, 0, "page up saturates at the top");
        wheel(&mut v, false, w, h);
        assert_eq!(v.off, 3, "wheel down scrolls three lines");
        key(&mut v, press(KeyCode::Right), w, h);
        assert_eq!(
            (v.cur, v.off),
            (1, 0),
            "right steps the prompt and resets the scroll"
        );
        assert_eq!(key(&mut v, press(KeyCode::Esc), w, h), Action::None);
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
        let len = body_lines(&v, w).len();
        let last = len - (h - 1);
        assert!(last > 3, "prompt too short to scroll: body={len} lines");
        key(&mut v, press(KeyCode::Down), w, h);
        assert_eq!(v.off, 1);
        key(&mut v, press(KeyCode::PageDown), w, h);
        assert!(v.off > 1, "pgdown did not move: off={}", v.off);
        key(&mut v, press(KeyCode::End), w, h);
        assert_eq!(v.off, last);
        key(&mut v, press(KeyCode::Down), w, h);
        assert_eq!(v.off, last, "down past end clamps");
        key(&mut v, press(KeyCode::Home), w, h);
        assert_eq!(v.off, 0);
        wheel(&mut v, false, w, h);
        assert_eq!(v.off, 3, "wheel down scrolls three lines");
        wheel(&mut v, true, w, h);
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
        let rows = list_lines(&v, 120, SystemTime::now()).join("\n");
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
        let rows = list_lines(&v, 120, later).join("\n");
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
