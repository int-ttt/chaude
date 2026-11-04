// Parity with Go history_test.go TestAgentCounts (plan/rust-test-parity.md, final2 workstream).
// Go counts() maps to AgentTracker::running() filtered by AgentRec.background, so the public API is enough.
// Go onToolUse/onToolResult map to on_tool_use/on_tool_result.

use chaude_core::agent_store::AgentTracker;
use serde_json::json;
use std::time::SystemTime;

// Go TestAgentCounts: foreground and background running counts, foreground drops after its result.
#[test]
fn agent_counts() {
    let tr = AgentTracker::new(None, "p", "");
    let now = SystemTime::now();
    tr.on_tool_use("a", "Agent", &json!({"description": "x"}), now);
    tr.on_tool_use(
        "b",
        "Agent",
        &json!({"description": "y", "run_in_background": true}),
        now,
    );
    let counts = |tr: &AgentTracker| {
        let run = tr.running();
        let bg = run.iter().filter(|a| a.background).count();
        (run.len() - bg, bg)
    };
    assert_eq!(counts(&tr), (1, 1));
    tr.on_tool_result("a");
    assert_eq!(counts(&tr), (0, 1));
}
