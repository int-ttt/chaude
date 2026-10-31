# 261029 explore: open items, release gates, approval phrase, version, changelog
task: read OPEN.md, DECISIONS.md, DELIVERABLES.md; grep pastes/9e25910e.md for approval phrases; locate version + changelog
status: done (read-only, no repo edits)

## files
- notes/OPEN.md: NOT FOUND (find -iname "OPEN*" excl .git: no match)
- notes/DELIVERABLES.md: NOT FOUND (find -iname "DELIVERABLES*": no match)
- notes/DECISIONS.md: 42 lines, whole file read (no "승인" line; paste says a 승인 원문 칸 was left blank, but none exists here)

## DECISIONS hits (multiclient/multiproc/v0.8.0/release)
- L34 [release] 261029 user: "다음 릴리즈부터 릴리즈에 이전 버전과 변경점 추가해" -> release.yml builds dist/notes.md ("## Changes since <prev tag>" + non-merge commits + compare link); from v0.7.0 on; v0.6.9 hand-edited
- L33 [conductor] 261029 user: "확인하고 0.6.8로 릴리즈해" -> gate: live classifier check (TestClassifierLive, CHAUDE_LIVE=1) before release
- L42 [local-cmd] 261029 user: "알아서 처리, 검증 통과 시 릴리즈" -> gate: verification pass before release
- multiclient / multiproc / v0.8.0: no match in DECISIONS.md

## release rules/gates (from DECISIONS)
- gate: 검증 통과 후 릴리즈 (L42); live test before release (L33)
- notes: auto "Changes since" from v0.7.0 (L34)

## approval phrase in notes/pastes/9e25910e.md
- 승인: L33 header "**승인 관련**"; L34 (assistant) "승인 문구는 없었습니다" -> no approval sentence in paste
- 진행: L37 (assistant) "상한을 두고 진행하겠습니다"; L41 (user) "haiku 최적화도 진행하고"; L49 (resume note) "계속 진행" -> instruction, not approval for policy docs
- OK: not found. go ahead: not found.
- L34 (assistant): CLAUDE.md / DECISIONS.md edits need approval sentence first (not given)

## open items (paste L26-31 "아직 안 된 것", no OPEN.md)
- Haiku prompt tuning + freeze
- 150-case eval set (50 per model) and 3-condition run
- pass-criteria judgment
- default router -> Haiku after pass
- CLAUDE.md and DECISIONS.md edits (blocked on approval, L34)
- v0.8.0 release (paste L44 user: "멀티클로드 구현 후 v0.8.0으로 릴리즈해"); per notes/runs/261029-report-router-multiclient-v080.md no v0.8.0 tag/entry yet

## version
- code source (only one): cmd/chaude/main.go:16 `var version = "0.7.2"`; printed at main.go:42; ldflags -X main.version
- grep "0.7.2" excl metrics: only main.go:16 in code; other hits are notes/runs text only

## changelog
- CHANGELOG.md (repo root): newest heading L3 "## 0.6.6"; no 0.6.7-0.7.2 or 0.8.0 entries
- README.md: shows 0.6.6 per notes/runs/261029-report-router-multiclient-v080.md (not re-checked by me)

## doubts
- OPEN.md / DELIVERABLES.md absent in repo; open items taken from paste only
- paste says DECISIONS.md approval slot left blank; no such slot exists in DECISIONS.md
- version 0.7.2 in code vs CHANGELOG top 0.6.6: changelog stale
