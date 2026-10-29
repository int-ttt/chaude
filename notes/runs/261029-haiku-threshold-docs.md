# 261029 threshold docs update
changed: CLAUDE.md line 27 (only line touched)
  before: "Auto-compaction (150k window) bounds context; state lives"
  after:  "Auto-compaction (150k window) bounds context; rotation threshold: Haiku main 80k (new session handoff), Opus and Sonnet main 270k; state lives"
changed: notes/DECISIONS.md appended row (line 41)
  "- [rotate] 2026-10-29 user: rotation threshold Haiku main 80k (new session handoff); Opus and Sonnet main 270k. Overview length cap (user decision, policy; no existing code limit): Haiku overview 3,000 chars; Opus/Sonnet overview 6,000 chars."
kept: 150k stays as auto-compaction window (separate mechanism from rotation threshold); not deleted.
conflict: DECISIONS row 40 (2026-10-29) says Opus/Sonnet mains do not switch by threshold; new 270k for Opus/Sonnet contradicts it. Not resolved here. 사용자 확인 필요.
self-check: grep 80k and 270k present in both files (PASS); grep 150k still present in CLAUDE.md line 27 (compaction).
