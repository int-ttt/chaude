# scout: overview length cap (개요 길이 상한)
date: 2026-10-29
mode: read-only grep, no edits
terms: 개요, overview, 3000, 3,000, 6000, 6,000, 길이, chars

## hits
- notes/DECISIONS.md:41 (2026-10-29 user, [rotate]): "Overview length cap (user decision, policy; no existing code limit): Haiku overview 3,000 chars; Opus/Sonnet overview 6,000 chars." — ONLY place the numbers are stated.
- notes/runs/261029-plan-hybrid-design.md:47: "Length cap for the overview = 사용자 확인 필요." (unresolved at the time)
- notes/runs/261029-plan-hybrid-design.md:93: "3. overview file length cap: 사용자 확인 필요."
- notes/runs/261029-plan-hybrid-design.md:15,43,86 mention overview file (no cap).
- CLAUDE.md: none (no 개요/overview/3000/6000/길이/chars hits).
- Go (internal/harness, cmd, internal/scripts): no cap, no 개요 hit.
  - internal/harness/classifier.go:31-32 planHint: says write plan to notes/runs/<YYMMDD>-plan-<topic>.md, review to ...-review.md. No length rule.
  - unrelated numeric hits (not overview cap): repl.go:865 `--snapshot-chars` default 24000; resume.go:97 and importclaude.go:140,162 snapshot_chars metric; unit_test.go:82-89 cleanEnv(300000) compact window; internal/scripts/extract_transcript.go:93 clip(t, 3000) for pasted text.

## not found
- any Go constant/check enforcing overview length (3000 / 6000 / overview / 개요): none
- CLAUDE.md rule for overview length: none
- value for Opus/Sonnet cap outside DECISIONS.md:41: none

## where the cap should live
- current state: docs only (DECISIONS.md:41 is the policy; plan doc still shows 사용자 확인 필요 at :47, :93 and may need update to 3,000/6,000)
- no Go code writes the overview file; the overview is written by Haiku via planHint (classifier.go:31). A code-level cap would need a hook there or in the writer, which does not exist yet.
