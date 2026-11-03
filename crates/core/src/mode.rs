// Permission modes (Go harness/mode.go). A mode answers can_use_tool before any prompt:
// allow and deny are final, ask goes to the UI. default asks; acceptEdits auto-allows file edits;
// plan allows read-only tools and denies everything that changes state; auto allows everything.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermMode {
    Default,
    AcceptEdits,
    Plan,
    Auto,
}

const ORDER: [PermMode; 4] = [
    PermMode::Default,
    PermMode::AcceptEdits,
    PermMode::Plan,
    PermMode::Auto,
];

impl PermMode {
    pub fn as_str(self) -> &'static str {
        match self {
            PermMode::Default => "default",
            PermMode::AcceptEdits => "acceptEdits",
            PermMode::Plan => "plan",
            PermMode::Auto => "auto",
        }
    }

    pub fn next(self) -> PermMode {
        let i = ORDER.iter().position(|m| *m == self).unwrap_or(0);
        ORDER[(i + 1) % ORDER.len()]
    }

    // Case-insensitive; "accept" is a short form of acceptEdits (Go parseMode).
    pub fn parse(s: &str) -> Option<PermMode> {
        let s = s.trim().to_ascii_lowercase();
        if s == "accept" || s == "acceptedits" {
            return Some(PermMode::AcceptEdits);
        }
        ORDER
            .into_iter()
            .find(|m| m.as_str().to_ascii_lowercase() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Deny(&'static str),
    Ask,
}

pub const PLAN_DENY_MSG: &str = "plan mode: do not modify anything or run commands; research read-only and present a plan for the user to approve";

const EDIT_TOOLS: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];
const READ_TOOLS: [&str; 10] = [
    "Read",
    "Grep",
    "Glob",
    "LS",
    "WebFetch",
    "WebSearch",
    "TodoWrite",
    "Agent",
    "Task",
    "NotebookRead",
];

// Go decideMode: the mode verdict for a tool. Anything not decided here is Ask.
pub fn decide(mode: PermMode, tool: &str) -> Verdict {
    match mode {
        PermMode::Auto => Verdict::Allow,
        PermMode::AcceptEdits if EDIT_TOOLS.contains(&tool) => Verdict::Allow,
        PermMode::Plan if READ_TOOLS.contains(&tool) => Verdict::Allow,
        PermMode::Plan => Verdict::Deny(PLAN_DENY_MSG),
        _ => Verdict::Ask,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go TestModeCycleAndParse
    #[test]
    fn cycle_and_parse() {
        let mut m = PermMode::Default;
        let mut seen = Vec::new();
        for _ in 0..ORDER.len() {
            seen.push(m.as_str());
            m = m.next();
        }
        assert_eq!(m, PermMode::Default);
        assert_eq!(seen[2], "plan");
        assert_eq!(PermMode::parse("Plan"), Some(PermMode::Plan));
        assert_eq!(PermMode::parse("accept"), Some(PermMode::AcceptEdits));
        assert_eq!(PermMode::parse("zzz"), None);
    }

    // Go TestDecideMode
    #[test]
    fn decide_mode_table() {
        use PermMode::*;
        let cases = [
            (Default, "Edit", Verdict::Ask),
            (AcceptEdits, "Edit", Verdict::Allow),
            (AcceptEdits, "Bash", Verdict::Ask),
            (Plan, "Read", Verdict::Allow),
            (Plan, "Bash", Verdict::Deny(PLAN_DENY_MSG)),
            (Plan, "Write", Verdict::Deny(PLAN_DENY_MSG)),
            (Auto, "Bash", Verdict::Allow),
        ];
        for (m, tool, want) in cases {
            assert_eq!(decide(m, tool), want, "{m:?} {tool}");
        }
    }
}
