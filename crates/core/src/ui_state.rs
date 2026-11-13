//! UI-neutral state shared by the TUI and the GUI: the user Action enum, the status-bar data (BarInput) and the
//! prompt-history push rule. No terminal, render or editor types here; the TUI keeps Editor, RefCell and render.

use std::collections::BTreeMap;

use crate::api::{CoderContext, InterruptSource};
use crate::slash::PermMode;

// history.go maxHistory
pub const MAX_HISTORY: usize = 500;

// Go inputHistory.Add: blanks and a repeat of the last entry are skipped; only the newest MAX_HISTORY are kept.
// Returns whether the entry was added.
pub fn push_history(items: &mut Vec<String>, s: String) -> bool {
    if !s.is_empty() && items.last() != Some(&s) {
        items.push(s);
        if items.len() > MAX_HISTORY {
            items.drain(..items.len() - MAX_HISTORY);
        }
        return true;
    }
    false
}

// What the event loop must do after a key press.
#[derive(Debug, PartialEq)]
pub enum Action {
    Continue,
    Submit(String),
    Interrupt(InterruptSource),
    Quit,
    // Esc while a reply is held: abandon its queued-task wait (Engine::cancel_wait).
    CancelWait,
    // /usage: the caller asks the engine for the one-shot run (UiEvent::Usage). Never a turn.
    Usage,
    // /update: the caller runs the release check off the UI thread; the result returns as Wake::Note (tui.go:1166).
    Update,
    // /doctor: the caller runs doctor::checks_now() off the UI thread; the rendered lines return as Wake::Note.
    Doctor,
    // Answer to the open permission prompt (Go tui.go key(): y/Enter allow, n/Esc deny, Ctrl+C deny + interrupt).
    Answer {
        id: String,
        allow: bool,
        interrupt: bool,
    },
    // Shift+Tab: the engine cycles the permission mode (Go setMode(Next())); it answers with ModeChanged.
    CycleMode,
    // /mode <name>: the engine sets the permission mode and answers with ModeChanged.
    SetMode(PermMode),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BarInput {
    // Seconds since the current turn started; None while idle.
    pub turn_secs: Option<u64>,
    // Running subagents as (description, elapsed seconds).
    pub agents: Vec<(String, u64)>,
    pub model: String,
    pub tokens: u64,
    pub cost_usd: f64,
    pub context_pct: u32,
    pub queued: usize,
    // Main-context limit in tokens (the welcome threshold); used by the idle Go-style status row.
    pub limit: u64,
    // Status line extras (Go statusLine): rate window text, rotations, permission mode ("default" or empty is
    // hidden), and foreground / background agent counts.
    pub rate: Option<String>,
    pub rot: u32,
    pub mode: String,
    pub fg: usize,
    pub bg: usize,
    // Agents a held reply waits for (UiEvent::ReplyHeld pending); None when the reply is not held.
    pub waiting: Option<usize>,
    // Latest context tokens per role (UiEvent::RoleContext): P = prompter, T = tui, C = coder.
    pub role_tokens: BTreeMap<String, u64>,
    // Live resident coders in spawn order (UiEvent::CoderContexts); empty hides the coder line.
    pub coders: Vec<CoderContext>,
    // Main's token counts of the last finished turn as (input, output, cache) (UiEvent::TurnTokens); shown on the idle row.
    pub turn_tokens: Option<(u64, u64, u64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_history_skips_blank_and_repeat_of_last() {
        let mut items = Vec::new();
        assert!(push_history(&mut items, "a".into()));
        assert!(!push_history(&mut items, "".into()));
        assert!(!push_history(&mut items, "a".into()));
        assert!(push_history(&mut items, "b".into()));
        assert!(push_history(&mut items, "a".into()));
        assert_eq!(items, ["a", "b", "a"]);
    }

    #[test]
    fn push_history_keeps_only_the_newest_max_entries() {
        let mut items = Vec::new();
        for i in 0..MAX_HISTORY + 3 {
            push_history(&mut items, format!("e{i}"));
        }
        assert_eq!(items.len(), MAX_HISTORY);
        assert_eq!(items[0], "e3");
    }

    #[test]
    fn bar_input_defaults_to_idle() {
        let b = BarInput::default();
        assert_eq!(b.turn_secs, None);
        assert!(b.coders.is_empty());
        assert_eq!(b.turn_tokens, None);
    }

    #[test]
    fn action_carries_its_payload() {
        assert_eq!(Action::Submit("hi".into()), Action::Submit("hi".into()));
        assert_eq!(
            Action::SetMode(PermMode::Plan),
            Action::SetMode(PermMode::Plan)
        );
    }
}
