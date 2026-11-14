//! Context rotation rules and handoff-note helpers (Go repl.go thresholdFor/shouldRotate, handoff.go).
//! Pure functions only: the actor decides when to rotate and runs the handoff turn.

use std::path::Path;

// Go haikuThreshold / opusThreshold: context tokens at which a rotation fires.
pub const HAIKU_THRESHOLD: u64 = 80_000;
pub const OPUS_THRESHOLD: u64 = 200_000;

// Go handoffPrompt: the hidden turn that asks the child for the handoff note.
pub const HANDOFF_PROMPT: &str = "Write a handoff note for a fresh session that will continue this work. No tools. \
Sections: Goal; Decisions; Files created/changed; In-progress work; Open questions; \
User preferences/instructions; Any exact codes, names or numbers the user gave. Be exact and brief.";

// Go preamble: prepended to the first prompt of a rotated session.
pub const PREAMBLE: &str = "[Context carried over from a previous session that was rotated to save tokens. \
Treat it as your memory of the conversation so far; do not mention the rotation.]";

// Go recoPrefix: the line the handoff note carries forward verbatim.
const RECO_PREFIX: &str = "추천:";

// Opus uses OPUS_THRESHOLD; every other model uses the configured threshold (Go thresholdFor).
pub fn threshold_for(model: &str, configured: u64) -> u64 {
    if model.contains("opus") {
        OPUS_THRESHOLD
    } else {
        configured
    }
}

// Rotation fires at the threshold itself (Go shouldRotate, >=).
pub fn should_rotate(ctx: u64, threshold: u64) -> bool {
    ctx >= threshold
}

// A resident agent (haiku-coder, tui) after a reply: rotates when its context reaches the model's threshold.
// configured 0 turns rotation off, as for the main child.
pub fn rotation_due(ctx: u64, model: &str, configured: u64) -> bool {
    configured > 0 && should_rotate(ctx, threshold_for(model, configured))
}

// Returns the last recommendation line of reply through the end of reply, trailing whitespace trimmed.
// Leading '*', '#', '>' and spaces before the marker are allowed. Empty when there is none.
pub fn extract_reco(reply: &str) -> String {
    let mut start = None;
    let mut offset = 0;
    for line in reply.split_inclusive('\n') {
        let body = line.trim_start_matches(['*', '#', '>', ' ', '\t']);
        if body.starts_with(RECO_PREFIX) {
            start = Some(offset);
        }
        offset += line.len();
    }
    match start {
        Some(s) => reply[s..].trim_end().to_string(),
        None => String::new(),
    }
}

// Required handoff sections; a section counts as present when any alternative appears (case-insensitive).
const SECTIONS: [(&str, &[&str]); 7] = [
    ("goal", &["goal"]),
    ("decision", &["decision"]),
    ("files", &["files"]),
    ("in-progress", &["in-progress", "in progress"]),
    ("open question", &["open question"]),
    ("preference", &["preference", "instruction"]),
    ("exact", &["exact"]),
];

// Problems with a handoff note; empty means valid (Go validateHandoff). reco is accepted for parity but not
// checked here: recoPreserved reports whether it survived.
pub fn validate_handoff(note: &str, changed: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    if note.trim().chars().count() < 200 {
        problems.push("too short".to_string());
    }
    let lower = note.to_lowercase();
    for (name, alts) in SECTIONS {
        if !alts.iter().any(|alt| lower.contains(alt)) {
            problems.push(format!("missing section: {name}"));
        }
    }
    for path in changed {
        let base = Path::new(path)
            .file_name()
            .map(|b| b.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !note.contains(&base) {
            problems.push(format!("file not mentioned: {base}"));
        }
    }
    problems
}

// Prepends the previous recommendation verbatim to the snapshot (Go withReco).
pub fn with_reco(reco: &str, snapshot: &str) -> String {
    if reco.is_empty() {
        return snapshot.to_string();
    }
    format!("## Previous recommendation (verbatim)\n{reco}\n\n{snapshot}")
}

// Reports whether reco appears byte-for-byte in first (Go recoPreserved). Empty reco is always preserved.
pub fn reco_preserved(reco: &str, first: &str) -> bool {
    reco.is_empty() || first.contains(reco)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_note() -> String {
        "## Goal\nImplement handoff validation for the claude-harness REPL so that a fresh session can resume work without losing context.\n\n\
## Decision\nUse stdlib only. Keep the recommendation line verbatim.\n\n\
## Files\n- internal/harness/handoff.go: extractReco, validateHandoff, withReco, recoPreserved.\n- internal/harness/handoff_test.go: unit tests for the four helpers.\n\n\
## In-progress\nTests are written; verification command is pending.\n\n\
## Open question\nNone.\n\n\
## Preference\nUser prefers Korean replies and minimal diffs. Do not touch other files.\n\n\
## Exact\nVerify with go vet and go test -run Reco|Handoff in the module root.\n"
            .to_string()
    }

    fn changed() -> Vec<String> {
        vec![
            "internal/harness/handoff.go".to_string(),
            "internal/harness/handoff_test.go".to_string(),
        ]
    }

    #[test]
    fn threshold_for_opus_and_configured() {
        assert_eq!(threshold_for("haiku", HAIKU_THRESHOLD), 80_000);
        assert_eq!(threshold_for("opus", HAIKU_THRESHOLD), 200_000);
        assert_eq!(threshold_for("claude-opus-5-5", HAIKU_THRESHOLD), 200_000);
        assert_eq!(threshold_for("", HAIKU_THRESHOLD), 80_000);
        assert_eq!(threshold_for("sonnet", 150_000), 150_000);
    }

    #[test]
    fn fable_uses_configured_threshold_not_opus() {
        // Only the Opus family is fixed at 200k (docs/claude-lean/README.md threshold table).
        assert_eq!(threshold_for("fable", HAIKU_THRESHOLD), 80_000);
        assert_eq!(threshold_for("claude-fable-5-2", HAIKU_THRESHOLD), 80_000);
        assert_eq!(threshold_for("claude-sonnet-5-5", 150_000), 150_000);
    }

    #[test]
    fn should_rotate_fires_at_threshold() {
        assert!(!should_rotate(79_999, 80_000));
        assert!(should_rotate(80_000, 80_000));
        assert!(!should_rotate(199_999, 200_000));
        assert!(should_rotate(200_000, 200_000));
    }

    #[test]
    fn rotation_due_resident_haiku_at_80k() {
        // Resident haiku-coder and tui rotate at the configured 80K threshold, after the reply.
        assert!(rotation_due(80_001, "haiku", HAIKU_THRESHOLD));
        assert!(!rotation_due(79_999, "haiku", HAIKU_THRESHOLD));
        assert!(rotation_due(80_000, "haiku", HAIKU_THRESHOLD));
        assert!(!rotation_due(80_001, "haiku", 0));
    }

    #[test]
    fn rotation_due_main_opus_keeps_200k() {
        assert!(rotation_due(200_000, "opus", HAIKU_THRESHOLD));
        assert!(!rotation_due(80_001, "opus", HAIKU_THRESHOLD));
        assert!(!rotation_due(199_999, "claude-opus-5-5", HAIKU_THRESHOLD));
    }

    #[test]
    fn extract_reco_takes_last_marker_block_through_end() {
        let reply = "앞부분 설명\n추천: 첫 번째 (무시됨)\n\n## 결론\n\
**추천:** 테스트를 먼저 실행하세요.\n두 번째 줄 세부 내용\n- 항목 A\n\n\n";
        let want = "**추천:** 테스트를 먼저 실행하세요.\n두 번째 줄 세부 내용\n- 항목 A";
        assert_eq!(extract_reco(reply), want);
    }

    #[test]
    fn extract_reco_none() {
        assert_eq!(extract_reco("추천 없이 그냥 텍스트\n추천은 없음\n"), "");
    }

    #[test]
    fn validate_good_note_has_no_problems() {
        assert!(validate_handoff(&good_note(), &changed()).is_empty());
    }

    #[test]
    fn validate_flags_short_note() {
        let p = validate_handoff("## Goal\nshort", &[]);
        assert!(p.iter().any(|s| s == "too short"), "{p:?}");
    }

    #[test]
    fn validate_flags_missing_section() {
        let note = good_note().split("## Exact").next().unwrap().to_string();
        let p = validate_handoff(&note, &changed());
        assert!(p.iter().any(|s| s == "missing section: exact"), "{p:?}");
    }

    #[test]
    fn validate_flags_unmentioned_file() {
        let mut files = vec!["internal/harness/repl.go".to_string()];
        files.extend(changed());
        let p = validate_handoff(&good_note(), &files);
        assert!(
            p.iter().any(|s| s == "file not mentioned: repl.go"),
            "{p:?}"
        );
    }

    #[test]
    fn with_reco_round_trip_and_preservation() {
        let reply = "설명입니다.\n> **추천:** 다음 단계는 테스트 실행입니다.\n- 이유: 빠른 검증\n";
        let reco = extract_reco(reply);
        assert_eq!(
            reco,
            "> **추천:** 다음 단계는 테스트 실행입니다.\n- 이유: 빠른 검증"
        );
        let snapshot = "## Goal\n핸드오프 테스트 작업 진행 중.\n";
        let out = with_reco(&reco, snapshot);
        assert!(out.starts_with(&format!(
            "## Previous recommendation (verbatim)\n{reco}\n\n"
        )));
        assert!(out.ends_with(snapshot));
        assert!(reco_preserved(&reco, &out));
        assert!(!reco_preserved(&reco, snapshot));
    }

    #[test]
    fn with_reco_empty_is_unchanged() {
        let snapshot = "## Goal\n본문\n";
        assert_eq!(with_reco("", snapshot), snapshot);
        assert!(reco_preserved("", snapshot));
    }
}
