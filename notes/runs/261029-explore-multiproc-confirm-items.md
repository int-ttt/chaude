task: list user confirm/decision items for v0.8.0 multiproc release
agent: explore
date: 261029
sources_read: 261029-sonnet-analyst-multiproc-s-units.md, 261029-haiku-coder-release-v080.md, 261029-review-multiproc-v080.md, 261029-plan-multiproc-finish.md (no 확인/위험/미해결/질문/decision headings exist; used §0, §1, §4 U7a/U7b, §8 and line 5 decisions)
base: /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/

items:
1. expiredZero semantics deviate from plan
   - plan U7a: expiredZero(d) returns false when d<=0 (plan 141)
   - plan U7b pseudocode: `if expiredZero(deadline) -> false` (plan 149)
   - sonnet: expiredZero(d)=d>0 "can expire"; idleDue uses !expiredZero (sonnet 11)
   - user: confirm the inverted helper meaning is acceptable.
2. idleDue boundary deviates from plan
   - plan: 3598s false / 3599s true (plan 151)
   - sonnet: boundary = deadline itself; tests deadline-1s false, deadline true (sonnet 12)
   - user: confirm 3599s rotate boundary and test change.
3. sweepIdle does not drain busy parked child
   - spec said drain; implementation keeps busy parked child open (sonnet 14, 20)
   - user: accept or require drain.
4. weak tests (reported by sonnet)
   - TestNoResumeOfLiveSessionID: parked child is stub closer, weak (sonnet 25)
   - TestIdleNoticeKeepsHeldOffset: smoke only, no offset assertion (sonnet 23)
   - TestMakeSnapshotStates: snapInvalid string not asserted (sonnet 24)
   - TestPlainLoopCallsCloseAll: same body as TestQuitCallsCloseAll; plain loop not end-to-end (sonnet 22)
   - user: accept or request stronger tests.
5. unresolved risks
   - A3 live child rebinding riskiest; manual 3-model round trip not recorded (plan 222)
   - manual `chaude --idle-rotate 5s` + ps orphan check not recorded as done (plan 198)
   - TTL 5m observed rate unverified until live run (plan 223)
   - review low: sweepIdle registry file I/O on UI goroutine (review 12); typed "/idle-rotate" triggers idleRotateNow (review 11); switchChild start-fail ignores err (review 13)
   - user: decide whether to run manual/live checks before push.
6. scope exclusions (0.8.x, not in 0.8.0)
   - U8b, U5c, U9-budget, U10a, U10b (plan 12, 217)
   - U10a auto TTL default needs approval; default stays 1h (plan 17)
   - G4 live gate TestClassifierLive cost approval pending, $3 cap (plan 18, 221)
   - user: approve or defer U10a and live gate.
7. unstaged files
   - 30 untracked notes files left out of commit (haiku 32-44)
   - notes/LEDGER.md committed with agent-appended lines (haiku 49)
   - gitStatus snapshot at session start: notes/metrics/2026-10-29.jsonl modified, unstaged (not in the three notes)
   - user: stage separately or leave as-is.
8. push and tag status
   - commit NOT pushed (haiku 4, 51)
   - annotated tag v0.8.0 created locally at 5e415b5 (haiku 29); plan says tag only on user request (plan 184); haiku says user requested it (haiku 30)
   - user: confirm tag, decide push of commit and tag.
9. commit trailer model
   - trailer: Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com> (haiku 22; plan 183, D4)
   - user: confirm trailer model name.
10. other doubts
   - CHANGELOG flags not re-verified against --help (haiku 50)
   - go test output not archived beyond stdout tail (haiku 48)
   - classifier.go gofmt unclean, pre-existing (sonnet 28)
   - user: accept or request follow-up.

not_found: 확인/위험/미해결/질문 headings in plan; manual check results; live gate result.
