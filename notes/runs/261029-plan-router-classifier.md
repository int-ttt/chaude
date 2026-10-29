# plan: router classifier misrouting (large tagged small)
goal: fewer large/judgment tasks tagged size:small (which skips design step)
constraint: model stays Sonnet; only prompt text + parsing if needed; do not commit

## evidence (haiku-scout)
- classifyPrompt: internal/harness/classifier.go:15-22; size rule: "small = single-file or mechanical edits, lookup, search, count, run/monitor, short explanation, small fix. large = multi-file or multi-step changes, design/architecture, hard diagnosis, weighing trade-offs, judgment calls, strategy/priority/roadmap, 'opus로', repeated failure."
- no "when unsure" rule for size (only topic has "When unsure, same")
- parseRouteFull :65: ok=false if size not small|large; kind missing -> other silently; classify returns os.ErrInvalid on parse fail (:130); route() logs metrics error and passes raw line, no switch
- planHint :31-33 attached :94-95 when model haiku && size large
- tests: conductor_test.go (TestParseRoute :9, TestMainRouteSwitchPolicy :44, TestHaikuPlanWorkflow :71). No test for missing-tag fallback.

## changes
1. classifyPrompt size rule: add explicit large signals: multi-file change; design/architecture question; release/deploy/version bump/merge; ambiguous or open-ended scope; multi-step plan; trade-off/comparison/recommendation; new feature; refactor; diagnosis with unknown cause.
2. Add rule: "size: when unsure between small and large, large." Define small narrowly: only if single file/one-step AND clear scope.
3. Parsing: size tag missing/invalid currently -> ok=false (no routing). Decide: keep ok=false for missing topic/task, but for missing/invalid size alone default to large (safe side) rather than failing. Keep kind default other.
4. planHint text: keep wording; no change unless review finds issue.
5. Tests (conductor_test.go): 
   - classifyPrompt contains new signal keywords and "unsure" rule
   - parseRouteFull: missing size -> ok with large (new behavior); invalid size value -> large; missing topic -> ok=false; missing task -> ok=false; missing kind -> other
   - size large + haiku current -> planHint appended (existing coverage check)
6. Verify: go build ./... ; go test ./... ; sonnet-reviewer on diff.

## risks
- over-large routing raises cost (Opus/plan workflow for trivial tasks); mitigate by keeping small examples explicit
- defaulting missing size to large changes ok semantics; ensure callers (pickModel) handle it

## merged from review (final spec)
0. baseline: go test ./internal/harness -run 'TestParseRoute|TestMainRouteSwitchPolicy|TestHaikuPlanWorkflow' before edits
A. parseRouteFull: normalize size value (trim spaces and trailing "."); missing/invalid size -> "large" with ok=true (topic/task still required). Update conductor_test.go:14 bad case "topic: same\ntask: x" -> move to ok case expecting large.
B. classifyPrompt: keep literals "topic: same|new" and "size: small|large" verbatim; add large signals + "when unsure between small and large, large" + narrow small definition.
C. routeResult gets Size field; route() metrics log size and size_defaulted=true when defaulted.
D. tests: missing size -> large + kind other; "Small." -> small; "medium" -> large; missing topic / missing task -> ok=false; pickModel topic new, no size, cur opus -> opus, delegate=false; haiku main + missing size -> planHint appended; prompt asserts only "size: small|large" and the unsure rule text.
E. docs (CLAUDE.md:31, DECISIONS.md:32,38) NOT edited (out of scope); flagged as follow-up.
F. live test size assertion skipped (costs API); flagged.
