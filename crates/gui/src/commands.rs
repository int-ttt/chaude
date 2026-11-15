//! GUI side of the slash commands. Core dispatch decides what a line means; this maps each outcome to a GUI effect.
//! The effect is plain data, so the mapping is unit-tested here without egui.

use std::path::Path;

use chaude_core::slash::{self, Ctx, PermMode, SlashOutcome};
use chaude_core::ui_state::BarInput;

// What the app does for a submitted line. Send goes to the engine (turns and the engine-side routes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    // A transcript notice row.
    Note(String),
    // Closes the window (/exit, /quit).
    Quit,
    // Sets the permission mode in the engine, which answers with ModeChanged.
    SetMode(PermMode),
    // Doctor checks run off the UI thread; the report comes back as a notice.
    Doctor,
    // The update check runs off the UI thread; its answer comes back as a notice.
    Update,
    // Plain prompts and the engine-side commands (/clear, /rotate, /model, /cost and the other passthroughs).
    Send,
}

// Note for a picker or toggle the GUI does not have yet. The terminal chaude keeps these.
pub fn not_yet(name: &str) -> String {
    format!("{name}: not in the GUI yet (use the terminal chaude)")
}

// Maps one core outcome to the GUI effect. cwd is the folder /notes-gc works in.
pub fn effect(outcome: SlashOutcome, cwd: &Path) -> Effect {
    match outcome {
        SlashOutcome::Note(s) => Effect::Note(s),
        SlashOutcome::Quit => Effect::Quit,
        SlashOutcome::SetMode(p) => Effect::SetMode(p),
        SlashOutcome::Doctor => Effect::Doctor,
        SlashOutcome::Update => Effect::Update,
        SlashOutcome::NotesGc { args } => {
            Effect::Note(chaude_core::notes_gc::slash_text(cwd, &args))
        }
        SlashOutcome::SendPrompt
        | SlashOutcome::Clear
        | SlashOutcome::Rotate { .. }
        | SlashOutcome::Model { .. }
        | SlashOutcome::Passthrough { .. } => Effect::Send,
        SlashOutcome::OpenUsage => Effect::Note(not_yet("/usage")),
        SlashOutcome::OpenLang | SlashOutcome::SetLang(_) => Effect::Note(not_yet("/lang")),
        SlashOutcome::OpenSettings => Effect::Note(not_yet("/settings")),
        SlashOutcome::OpenView { .. } => Effect::Note(not_yet("/view")),
        SlashOutcome::Resume { .. } => Effect::Note(not_yet("/resume")),
        SlashOutcome::Import { .. } => Effect::Note(not_yet("/import")),
        SlashOutcome::ToggleMouse => {
            Effect::Note("/mouse: the GUI always scrolls with the wheel".into())
        }
    }
}

// The core context for dispatch, read from the bar the GUI shows. An empty session id (none yet, or after a
// rotation) shows "-" in /status.
pub fn ctx<'a>(bar: &'a BarInput, busy: bool, session_id: &'a str) -> Ctx<'a> {
    Ctx {
        model: &bar.model,
        last_ctx: bar.tokens,
        threshold: bar.limit,
        rotations: u64::from(bar.rot),
        session_id: (!session_id.is_empty()).then_some(session_id),
        mode: slash::parse_mode(&bar.mode).unwrap_or(PermMode::Default),
        chaude_version: env!("CARGO_PKG_VERSION"),
        busy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cwd() -> &'static Path {
        Path::new(".")
    }

    #[test]
    fn notes_quit_and_mode_map_to_gui_effects() {
        assert_eq!(
            effect(SlashOutcome::Note("hi".into()), cwd()),
            Effect::Note("hi".into())
        );
        assert_eq!(effect(SlashOutcome::Quit, cwd()), Effect::Quit);
        assert_eq!(
            effect(SlashOutcome::SetMode(PermMode::Plan), cwd()),
            Effect::SetMode(PermMode::Plan)
        );
    }

    #[test]
    fn doctor_and_update_run_as_jobs() {
        assert_eq!(effect(SlashOutcome::Doctor, cwd()), Effect::Doctor);
        assert_eq!(effect(SlashOutcome::Update, cwd()), Effect::Update);
    }

    #[test]
    fn prompts_and_engine_routes_go_to_the_engine() {
        for o in [
            SlashOutcome::SendPrompt,
            SlashOutcome::Clear,
            SlashOutcome::Rotate {
                line: "/rotate".into(),
            },
            SlashOutcome::Model {
                line: "/model x".into(),
                name: "x".into(),
            },
            SlashOutcome::Passthrough {
                line: "/cost".into(),
            },
        ] {
            assert_eq!(effect(o.clone(), cwd()), Effect::Send, "{o:?}");
        }
    }

    #[test]
    fn pickers_without_a_gui_window_say_so() {
        for (o, name) in [
            (SlashOutcome::OpenUsage, "/usage"),
            (SlashOutcome::OpenLang, "/lang"),
            (SlashOutcome::SetLang("ko".into()), "/lang"),
            (SlashOutcome::OpenSettings, "/settings"),
            (SlashOutcome::OpenView { arg: String::new() }, "/view"),
            (SlashOutcome::Resume { args: vec![] }, "/resume"),
            (
                SlashOutcome::Import {
                    line: "/import".into(),
                    arg: "latest".into(),
                },
                "/import",
            ),
        ] {
            assert_eq!(effect(o, cwd()), Effect::Note(not_yet(name)), "{name}");
        }
    }

    #[test]
    fn ctx_reads_the_bar_and_parses_the_mode() {
        let bar = BarInput {
            model: "sonnet".into(),
            tokens: 1200,
            limit: 150_000,
            rot: 2,
            mode: "plan".into(),
            ..Default::default()
        };
        let c = ctx(&bar, true, "abc");
        assert_eq!(c.session_id, Some("abc"));
        assert_eq!(ctx(&bar, false, "").session_id, None);
        let c = ctx(&bar, true, "abc");
        assert_eq!(c.model, "sonnet");
        assert_eq!(c.last_ctx, 1200);
        assert_eq!(c.threshold, 150_000);
        assert_eq!(c.rotations, 2);
        assert_eq!(c.mode, PermMode::Plan);
        assert!(c.busy);
        assert_eq!(
            ctx(
                &BarInput {
                    mode: "nope".into(),
                    ..Default::default()
                },
                false,
                ""
            )
            .mode,
            PermMode::Default
        );
    }
}
