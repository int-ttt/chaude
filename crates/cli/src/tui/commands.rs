// Slash commands for the inline TUI: parsing, suggestions, help text, and dispatch to an enum of actions.
// Nothing here runs an engine effect. Each side effect comes back as a Dispatch variant for the caller to
// execute. Ported from deprecated/src/main/intt/internal/harness/tui.go (slashCmds, busyOK, suggestions,
// runSlash) and mode.go. Plain functions, unit-tested without a terminal.
// The event loop is not wired to these yet, hence the module-level allow.
#![allow(dead_code)]

// tui.go:192-230 (slashCmds). Order and text are the Go source's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlashCmd {
    pub name: &'static str,
    pub desc: &'static str,
}

pub const SLASH_CMDS: &[SlashCmd] = &[
    SlashCmd {
        name: "/help",
        desc: "show commands and keys",
    },
    SlashCmd {
        name: "/clear",
        desc: "start a fresh session (handoff note carried over)",
    },
    SlashCmd {
        name: "/status",
        desc: "model, context tokens, session",
    },
    SlashCmd {
        name: "/model",
        desc: "show or switch model: /model <name>",
    },
    SlashCmd {
        name: "/settings",
        desc: "session retention, resident coder slots, main model (saved on esc; applies on next session start)",
    },
    SlashCmd {
        name: "/resume",
        desc: "list record topics, or show one: /resume [n|slug] (/resume claude [q]: Claude Code sessions, filtered by q)",
    },
    SlashCmd {
        name: "/import",
        desc: "import a session: /import [id|latest]",
    },
    SlashCmd {
        name: "/view",
        desc: "browse Claude Code sessions (↑/↓ enter, ←/→ prompts, esc back): /view [id-prefix]",
    },
    SlashCmd {
        name: "/rotate",
        desc: "hand off to a fresh session",
    },
    SlashCmd {
        name: "/usage",
        desc: "plan usage and limits (from claude)",
    },
    SlashCmd {
        name: "/cost",
        desc: "usage / cost (from claude)",
    },
    SlashCmd {
        name: "/context",
        desc: "context usage by category (from claude)",
    },
    SlashCmd {
        name: "/compact",
        desc: "compact the child's context (from claude)",
    },
    SlashCmd {
        name: "/config",
        desc: "show/set config: /config key=value (from claude)",
    },
    SlashCmd {
        name: "/mcp",
        desc: "MCP server status (from claude)",
    },
    SlashCmd {
        name: "/agents",
        desc: "subagents info (from claude)",
    },
    SlashCmd {
        name: "/init",
        desc: "analyze repo, propose CLAUDE.md (runs a turn)",
    },
    SlashCmd {
        name: "/review",
        desc: "review current changes (runs a turn)",
    },
    SlashCmd {
        name: "/doctor",
        desc: "check installation and config (runs a turn)",
    },
    SlashCmd {
        name: "/mode",
        desc: "permission mode: default|acceptEdits|plan|auto (shift+tab cycles)",
    },
    SlashCmd {
        name: "/mouse",
        desc: "toggle mouse wheel scroll (off = select text)",
    },
    SlashCmd {
        name: "/version",
        desc: "chaude and Claude Code versions",
    },
    SlashCmd {
        name: "/update",
        desc: "check for a newer chaude and install it (restart chaude to use it)",
    },
    SlashCmd {
        name: "/exit",
        desc: "exit",
    },
    SlashCmd {
        name: "/quit",
        desc: "exit",
    },
];

// tui.go:841 (busyOK). /usage is also allowed while busy, handled before this set in the Go enter path.
// /settings is in the Go map but missing from the parity spec's C26 list.
pub const BUSY_OK: &[&str] = &[
    "/usage",
    "/version",
    "/help",
    "/status",
    "/mode",
    "/mouse",
    "/view",
    "/settings",
];

// tui.go:844 (busySlashMsg).
pub const BUSY_SLASH_MSG: &str = "생각 중에는 /usage, /version, /help, /status, /mode, /mouse, /view 만 사용할 수 있습니다. 답변이 끝난 뒤 다시 입력해 주세요.";

// repl.go:1167-1168 (passthroughCmds). /usage is in the Go map too, but runSlash intercepts it first.
pub const PASSTHROUGH: &[&str] = &[
    "/cost", "/context", "/compact", "/config", "/mcp", "/agents", "/init", "/review", "/doctor",
];

// repl.go:1171 (unsupportedCmds).
pub const UNSUPPORTED: &[&str] = &["/memory", "/permissions"];

// tui.go:1062 (help key line, appended after the command list).
pub const HELP_KEYS: &str = "esc esc rewind · up/down history · esc interrupt · y/n permission · pgup/pgdn scroll · tab complete";

// tui.go:1147 region, mode.go:9-10. Names are the Go modeNames order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermMode {
    Default,
    AcceptEdits,
    Plan,
    Auto,
}

impl PermMode {
    pub fn name(self) -> &'static str {
        match self {
            PermMode::Default => "default",
            PermMode::AcceptEdits => "acceptEdits",
            PermMode::Plan => "plan",
            PermMode::Auto => "auto",
        }
    }

    // mode.go Next(): shift+tab cycles default -> acceptEdits -> plan -> auto -> default.
    pub fn next(self) -> Self {
        match self {
            PermMode::Default => PermMode::AcceptEdits,
            PermMode::AcceptEdits => PermMode::Plan,
            PermMode::Plan => PermMode::Auto,
            PermMode::Auto => PermMode::Default,
        }
    }
}

// mode.go parseMode: case-insensitive; "accept" and "acceptedits" alias AcceptEdits.
pub fn parse_mode(s: &str) -> Option<PermMode> {
    let s = s.trim().to_lowercase();
    if s == "accept" || s == "acceptedits" {
        return Some(PermMode::AcceptEdits);
    }
    [
        PermMode::Default,
        PermMode::AcceptEdits,
        PermMode::Plan,
        PermMode::Auto,
    ]
    .into_iter()
    .find(|m| m.name().to_lowercase() == s)
}

// tui.go:1100-1110 (note text for /update, shown before the off-thread check starts).
pub const UPDATE_START_NOTE: &str = "checking for updates...";

// Runtime values the Go handlers read from the Repl. The caller fills these in.
#[derive(Debug, Clone, Copy)]
pub struct Ctx<'a> {
    pub model: &'a str,
    pub last_ctx: u64,
    pub threshold: u64,
    pub rotations: u64,
    pub session_id: Option<&'a str>,
    pub mode: PermMode,
    pub chaude_version: &'a str,
    pub claude_version: &'a str,
    pub busy: bool,
}

// Effects the caller must run. The Note variant is a transcript line and needs no engine call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dispatch {
    Note(String),
    Quit,
    // Caller flips the mouse state and shows mouse_note(new_state).
    ToggleMouse,
    OpenSettings,
    OpenUsage,
    OpenView { arg: String },
    // Raw args after /resume. The record and session lookup happens in the caller.
    Resume { args: Vec<String> },
    // Turn is sent to the engine as the full line, with a user block shown first.
    Import { line: String, arg: String },
    Model { line: String, name: String },
    Rotate { line: String },
    Clear,
    Passthrough { line: String },
    // Caller shows UPDATE_START_NOTE, then runs the check off the UI thread.
    Update,
    SetMode(PermMode),
}

pub fn mouse_note(on: bool) -> &'static str {
    if on {
        "mouse wheel scroll ON (shift+drag selects text in most terminals)"
    } else {
        "mouse OFF: text selection works; scroll with pgup/pgdn, shift+up/down"
    }
}

// tui.go:1057-1063 (runSlash /help). One "%-8s %s" row per command, then the key line.
pub fn help_text() -> String {
    let mut rows: Vec<String> = SLASH_CMDS
        .iter()
        .map(|c| format!("{:<8} {}", c.name, c.desc))
        .collect();
    rows.push(HELP_KEYS.to_string());
    rows.join("\n")
}

// tui.go:1071-1079 (/status). Empty session id shows "-". Session id is cut to 8 chars.
pub fn status_line(ctx: &Ctx) -> String {
    let sid = match ctx.session_id {
        Some(s) if !s.is_empty() => s.chars().take(8).collect::<String>(),
        _ => "-".to_string(),
    };
    format!(
        "model {} · ctx {}/{} · rotations {} · session {}",
        ctx.model,
        commas(ctx.last_ctx),
        commas(ctx.threshold),
        ctx.rotations,
        sid
    )
}

// repl.go:122 (commas): thousands separators.
pub fn commas(n: u64) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

// tui.go:1080 (/model with no args).
pub fn model_note(model: &str) -> String {
    format!(
        "model {}  (switch: /model <name>, hands off context to the new session)",
        model
    )
}

// tui.go:1145-1155 (/mode with no args).
pub fn mode_note(mode: PermMode) -> String {
    format!(
        "mode {}  (default | acceptEdits | plan | auto; shift+tab cycles)",
        mode.name()
    )
}

// tui.go:1156 (/version).
pub fn version_note(chaude: &str, claude: &str) -> String {
    format!("chaude {} · Claude Code {}", chaude, claude)
}

// tui.go:974-986 (suggestions). Empty when the input is not a slash word, has a space or tab,
// or while busy or a permission prompt is open.
pub fn suggestions(input: &str, busy: bool, perm_open: bool) -> Vec<&'static SlashCmd> {
    if !input.starts_with('/') || input.contains([' ', '\t']) || busy || perm_open {
        return Vec::new();
    }
    SLASH_CMDS
        .iter()
        .filter(|c| c.name.starts_with(input))
        .collect()
}

// tui.go:712-716 (tab key): the selected suggestion (clamped) plus a trailing space.
pub fn complete(input: &str, sel: usize, busy: bool, perm_open: bool) -> Option<String> {
    let sg = suggestions(input, busy, perm_open);
    if sg.is_empty() {
        return None;
    }
    Some(format!("{} ", sg[sel.min(sg.len() - 1)].name))
}

// tui.go:796 (busy gate) and runSlash (tui.go:1051). None means plain text, not a command.
pub fn dispatch(line: &str, ctx: &Ctx) -> Option<Dispatch> {
    let line = line.trim();
    if !line.starts_with('/') {
        return None;
    }
    let f: Vec<&str> = line.split_whitespace().collect();
    let name = f[0];
    let args = &f[1..];
    let first = args.first().copied();

    if ctx.busy && name != "/usage" && !BUSY_OK.contains(&name) {
        return Some(Dispatch::Note(BUSY_SLASH_MSG.to_string()));
    }

    Some(match name {
        "/quit" | "/exit" => Dispatch::Quit,
        "/help" => Dispatch::Note(help_text()),
        "/mouse" => Dispatch::ToggleMouse,
        "/status" => Dispatch::Note(status_line(ctx)),
        "/model" => match first {
            None => Dispatch::Note(model_note(ctx.model)),
            Some(name) => Dispatch::Model {
                line: line.to_string(),
                name: name.to_string(),
            },
        },
        "/settings" => Dispatch::OpenSettings,
        "/resume" => Dispatch::Resume {
            args: args.iter().map(|s| s.to_string()).collect(),
        },
        "/view" => Dispatch::OpenView {
            arg: first.unwrap_or("").to_string(),
        },
        // repl.go:1399-1401: the engine side defaults to "latest".
        "/import" => Dispatch::Import {
            line: line.to_string(),
            arg: first.unwrap_or("latest").to_string(),
        },
        "/mode" => match first {
            None => Dispatch::Note(mode_note(ctx.mode)),
            Some(s) => match parse_mode(s) {
                Some(p) => Dispatch::SetMode(p),
                None => Dispatch::Note(format!(
                    "unknown mode {} (default | acceptEdits | plan | auto)",
                    s
                )),
            },
        },
        "/version" => Dispatch::Note(version_note(ctx.chaude_version, ctx.claude_version)),
        "/update" => Dispatch::Update,
        "/usage" => Dispatch::OpenUsage,
        "/clear" => Dispatch::Clear,
        "/rotate" => Dispatch::Rotate {
            line: line.to_string(),
        },
        n if PASSTHROUGH.contains(&n) => Dispatch::Passthrough {
            line: line.to_string(),
        },
        n if UNSUPPORTED.contains(&n) => Dispatch::Note(format!(
            "{} is interactive-only in claude and not supported in this REPL",
            n
        )),
        n => Dispatch::Note(format!("unknown command {} (try /help)", n)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(busy: bool) -> Ctx<'static> {
        Ctx {
            model: "opus",
            last_ctx: 12345,
            threshold: 150000,
            rotations: 2,
            session_id: Some("abcdef0123456789"),
            mode: PermMode::Default,
            chaude_version: "9.9.9",
            claude_version: "1.2.3",
            busy,
        }
    }

    // Port of busyslash_test.go TestBusySlashAllowedAndRefused.
    #[test]
    fn busy_allows_read_only_and_refuses_conversation_commands() {
        for c in ["/version", "/usage", "/help", "/mode"] {
            let d = dispatch(c, &ctx(true)).unwrap();
            assert_ne!(
                d,
                Dispatch::Note(BUSY_SLASH_MSG.to_string()),
                "{c} refused while busy"
            );
        }
        for c in ["/resume", "/clear", "/rotate", "/model sonnet", "/cost"] {
            assert_eq!(
                dispatch(c, &ctx(true)),
                Some(Dispatch::Note(BUSY_SLASH_MSG.to_string())),
                "{c} not refused while busy"
            );
        }
    }

    // Port of busyslash_test.go TestVersionNoteShowsChaudeVersion.
    #[test]
    fn version_note_shows_chaude_version() {
        assert_eq!(
            dispatch("/version", &ctx(false)),
            Some(Dispatch::Note(
                "chaude 9.9.9 · Claude Code 1.2.3".to_string()
            ))
        );
    }

    // Port of usagebusy_test.go TestUsageSlashAcceptedWhileBusy.
    #[test]
    fn usage_is_accepted_while_busy() {
        assert_eq!(dispatch("/usage", &ctx(true)), Some(Dispatch::OpenUsage));
    }

    // Port of usagebusy_test.go TestUsageAllowedWhileBusy: /exit is refused while busy, and the Go literal matches.
    #[test]
    fn exit_is_refused_while_busy_and_message_matches_go() {
        assert_eq!(
            dispatch("/exit", &ctx(true)),
            Some(Dispatch::Note(BUSY_SLASH_MSG.to_string()))
        );
        assert_eq!(
            BUSY_SLASH_MSG,
            "생각 중에는 /usage, /version, /help, /status, /mode, /mouse, /view 만 사용할 수 있습니다. 답변이 끝난 뒤 다시 입력해 주세요."
        );
    }

    #[test]
    fn plain_text_is_not_a_command() {
        assert_eq!(dispatch("hello /help", &ctx(false)), None);
        assert_eq!(dispatch("  ", &ctx(false)), None);
    }

    #[test]
    fn exit_and_quit_quit() {
        assert_eq!(dispatch("/exit", &ctx(false)), Some(Dispatch::Quit));
        assert_eq!(dispatch(" /quit ", &ctx(false)), Some(Dispatch::Quit));
    }

    #[test]
    fn status_shows_commas_and_truncated_session() {
        assert_eq!(
            dispatch("/status", &ctx(false)),
            Some(Dispatch::Note(
                "model opus · ctx 12,345/150,000 · rotations 2 · session abcdef01".to_string()
            ))
        );
        let mut c = ctx(false);
        c.session_id = None;
        assert!(status_line(&c).ends_with("session -"));
    }

    #[test]
    fn model_without_args_shows_note_and_with_args_switches() {
        assert_eq!(
            dispatch("/model", &ctx(false)),
            Some(Dispatch::Note(
                "model opus  (switch: /model <name>, hands off context to the new session)"
                    .to_string()
            ))
        );
        assert_eq!(
            dispatch("/model sonnet", &ctx(false)),
            Some(Dispatch::Model {
                line: "/model sonnet".to_string(),
                name: "sonnet".to_string()
            })
        );
    }

    #[test]
    fn view_and_import_default_args() {
        assert_eq!(
            dispatch("/view", &ctx(false)),
            Some(Dispatch::OpenView { arg: String::new() })
        );
        assert_eq!(
            dispatch("/view ab12", &ctx(false)),
            Some(Dispatch::OpenView {
                arg: "ab12".to_string()
            })
        );
        assert_eq!(
            dispatch("/import", &ctx(false)),
            Some(Dispatch::Import {
                line: "/import".to_string(),
                arg: "latest".to_string()
            })
        );
    }

    #[test]
    fn resume_passes_raw_args() {
        assert_eq!(
            dispatch("/resume claude foo bar", &ctx(false)),
            Some(Dispatch::Resume {
                args: vec!["claude".into(), "foo".into(), "bar".into()]
            })
        );
    }

    #[test]
    fn mode_no_args_set_and_unknown() {
        assert_eq!(
            dispatch("/mode", &ctx(false)),
            Some(Dispatch::Note(
                "mode default  (default | acceptEdits | plan | auto; shift+tab cycles)".to_string()
            ))
        );
        assert_eq!(
            dispatch("/mode PLAN", &ctx(false)),
            Some(Dispatch::SetMode(PermMode::Plan))
        );
        assert_eq!(
            dispatch("/mode accept", &ctx(false)),
            Some(Dispatch::SetMode(PermMode::AcceptEdits))
        );
        assert_eq!(
            dispatch("/mode nope", &ctx(false)),
            Some(Dispatch::Note(
                "unknown mode nope (default | acceptEdits | plan | auto)".to_string()
            ))
        );
    }

    #[test]
    fn mode_cycles_like_go_next() {
        let mut m = PermMode::Default;
        let names: Vec<&str> = (0..5)
            .map(|_| {
                m = m.next();
                m.name()
            })
            .collect();
        assert_eq!(
            names,
            ["acceptEdits", "plan", "auto", "default", "acceptEdits"]
        );
    }

    #[test]
    fn passthrough_and_turn_commands_map_to_effects() {
        for c in [
            "/cost", "/context", "/compact", "/config", "/mcp", "/agents", "/init", "/review",
            "/doctor",
        ] {
            assert_eq!(
                dispatch(c, &ctx(false)),
                Some(Dispatch::Passthrough {
                    line: c.to_string()
                }),
                "{c}"
            );
        }
        assert_eq!(dispatch("/clear", &ctx(false)), Some(Dispatch::Clear));
        assert_eq!(
            dispatch("/rotate", &ctx(false)),
            Some(Dispatch::Rotate {
                line: "/rotate".to_string()
            })
        );
        assert_eq!(dispatch("/update", &ctx(false)), Some(Dispatch::Update));
        assert_eq!(
            dispatch("/settings", &ctx(false)),
            Some(Dispatch::OpenSettings)
        );
        assert_eq!(dispatch("/usage", &ctx(false)), Some(Dispatch::OpenUsage));
        assert_eq!(dispatch("/mouse", &ctx(false)), Some(Dispatch::ToggleMouse));
    }

    #[test]
    fn unknown_and_unsupported_commands_note() {
        assert_eq!(
            dispatch("/nope", &ctx(false)),
            Some(Dispatch::Note(
                "unknown command /nope (try /help)".to_string()
            ))
        );
        assert_eq!(
            dispatch("/memory", &ctx(false)),
            Some(Dispatch::Note(
                "/memory is interactive-only in claude and not supported in this REPL".to_string()
            ))
        );
    }

    #[test]
    fn suggestions_filter_by_prefix_and_hide_when_blocked() {
        let names: Vec<&str> = suggestions("/re", false, false)
            .iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(names, ["/resume", "/review"]);
        let names: Vec<&str> = suggestions("/ro", false, false)
            .iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(names, ["/rotate"]);
        assert_eq!(suggestions("/", false, false).len(), SLASH_CMDS.len());
        assert!(suggestions("/mo ", false, false).is_empty());
        assert!(suggestions("hi", false, false).is_empty());
        assert!(suggestions("/", true, false).is_empty());
        assert!(suggestions("/", false, true).is_empty());
    }

    #[test]
    fn tab_completes_selected_suggestion_with_space() {
        assert_eq!(
            complete("/mo", 0, false, false),
            Some("/model ".to_string())
        );
        assert_eq!(complete("/mo", 1, false, false), Some("/mode ".to_string()));
        assert_eq!(
            complete("/mo", 9, false, false),
            Some("/mouse ".to_string())
        );
        assert_eq!(complete("/zz", 0, false, false), None);
    }

    #[test]
    fn help_text_lists_every_command_then_keys() {
        let h = help_text();
        let lines: Vec<&str> = h.split('\n').collect();
        assert_eq!(lines.len(), SLASH_CMDS.len() + 1);
        assert_eq!(lines[0], "/help    show commands and keys");
        assert_eq!(lines[SLASH_CMDS.len()], HELP_KEYS);
    }

    #[test]
    fn mouse_notes_match_go() {
        assert_eq!(
            mouse_note(true),
            "mouse wheel scroll ON (shift+drag selects text in most terminals)"
        );
        assert_eq!(
            mouse_note(false),
            "mouse OFF: text selection works; scroll with pgup/pgdn, shift+up/down"
        );
    }

    #[test]
    fn commas_groups_thousands() {
        assert_eq!(commas(0), "0");
        assert_eq!(commas(999), "999");
        assert_eq!(commas(1000), "1,000");
        assert_eq!(commas(1234567), "1,234,567");
    }
}
