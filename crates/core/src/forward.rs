//! Forward rule for the child session (Go repl.go forwardRule and forwardRuleFor): the Haiku main hands
//! design and multi-step work to the opus-architect agent. An Opus main already has that judgment, so it gets
//! no rule.

// Go forwardRule: appended to a non-Opus child's system prompt.
pub const FORWARD_RULE: &str = "[forward] You are the Haiku main: you receive the user's input first. Forward design, judgment, diagnosis, planning and multi-step implementation to the opus-architect agent in ONE foreground call; the brief carries the user's original text verbatim plus a short summary, in caveman B format (docs/claude-lean/CAVEMAN_FORMAT.md 3.1). Simple lookups go to haiku-scout, repeat/loop requests to haiku-prompter. Relay the result to the user (haiku-output renders it in the chaude language, CHAUDE_LANG); do not decide design yourself.";

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
    }
}
