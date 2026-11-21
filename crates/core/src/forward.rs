//! Forward rule for the child session (Go repl.go forwardRule and forwardRuleFor): the Haiku main hands
//! design and multi-step work to the opus-architect agent. An Opus main already has that judgment, so it gets
//! no rule.

// Go forwardRule: appended to a non-Opus child's system prompt.
pub const FORWARD_RULE: &str = "[forward] Haiku main: send the user's prompt verbatim plus a short summary to opus-architect in ONE foreground call; relay its result via haiku-output. Decide nothing yourself.";

// Go forwardRuleFor: the rule for a child on model. Any model name containing "opus" (case-insensitive)
// gets an empty rule.
pub fn forward_rule_for(model: &str) -> &'static str {
    if model.to_lowercase().contains("opus") {
        return "";
    }
    FORWARD_RULE
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go TestForwardRuleFor
    #[test]
    fn forward_rule_for_skips_opus_models() {
        assert_eq!(forward_rule_for("haiku"), FORWARD_RULE);
        assert_eq!(forward_rule_for(""), FORWARD_RULE);
        assert_eq!(forward_rule_for("opus"), "");
        assert_eq!(forward_rule_for("claude-opus-4"), "");
        assert!(FORWARD_RULE.contains("opus-architect"));
        assert!(FORWARD_RULE.contains("haiku-output"));
        assert!(FORWARD_RULE.len() <= 200);
        // CLAUDE.md: verbatim prompt -> opus-architect -> haiku-output; no router agents.
        assert!(!FORWARD_RULE.contains("haiku-scout"));
        assert!(!FORWARD_RULE.contains("haiku-prompter"));
    }
}
