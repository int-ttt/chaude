# review: plan-router-classifier (read-only)
target: notes/runs/261029-plan-router-classifier.md
checked: internal/harness/classifier.go (parseRouteFull :65, pickModel :38, route :138, classify :109), internal/harness/conductor_test.go

## concrete issues
1. test break: conductor_test.go:14 bad-list has "topic: same\ntask: x" expecting ok=false. Plan change 3 (missing size -> large, ok=true) makes it ok=true, so TestParseRoute fails. Plan step 5 does not mention this line. Fix: move that case to an ok-case (size large) or drop it from bad list.
2. planHint blast radius: classifier.go:94 attaches planHint when model=haiku && size=large. Missing or garbled size (e.g. "Small.", "medium") now gets full plan workflow (Haiku plan + opus-architect/sonnet-analyst round). Cost/latency caller: every haiku-main prompt with bad tag.
3. forced opus caller: pickModel :43 delegates only when size=="small". Missing size now -> no delegate, so unrelated side tasks run in Opus main context (no rotation). Add test: topic new, no size, cur opus -> opus, delegate=false (documented, intended?).
4. silent drift: route() :147-163 logs no size and routeResult (:54-56) has no Size field. Cannot count how often default-to-large fires, and no before/after baseline. Add Size to routeResult, log f["size"] and f["size_defaulted"]=true.
5. parse edge: value is lowercased then compared exactly. "small." or "small " currently fails (ok=false); after change it silently becomes large. Either normalize (trim ". ") or accept the drift via metric in item 4.
6. prompt-test coupling: conductor_test.go:64-65 asserts substrings "topic: same|new" and "size: small|large". Keep these literals verbatim when editing classifyPrompt (:15-22). Plan step 5 keyword asserts are brittle; assert only "size: small|large" and the "unsure" rule text.
7. docs: plan has no step for docs. CLAUDE.md:31, DECISIONS.md:32,38 describe size tagging and pickModel/planHint behavior. Update if default-to-large is adopted.

## ordering
- add step 0: run baseline `go test ./internal/harness -run 'TestParseRoute|TestMainRouteSwitchPolicy|TestHaikuPlanWorkflow'` before edits.
- order: parse change + test line 14 fix together (else step 6 fails); prompt text edit after; metric (item 4) before any measurement claim.

## test gaps
- no live size assertion: TestClassifierLive (conductor_test.go:22-41) checks model only; routeResult has no size. Add size or planHint check on the misrouted prompts (costs ~$0.007 each, CHAUDE_LIVE=1).
- no test for missing-size on haiku main (planHint expected) or missing-task/topic still ok=false beyond existing "topic: new\nsize: small" case.
- no test that kind default stays "other" when size missing (plan says keep; add one case).

## kept as-is (checked)
- ok=false for missing topic/task: already covered (conductor_test.go:14).
- worker assignment (:98-104) independent of size: edit-size change does not drop worker reuse.
- classify parse failure returns os.ErrInvalid (:130-131), route returns raw line: unchanged by plan.

## not found
- no other callers of parseRoute/pickModel/pickTarget outside classifier.go and conductor_test.go (grep internal, cmd).
