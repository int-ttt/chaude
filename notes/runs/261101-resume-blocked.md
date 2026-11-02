task: chaude /resume upgrade (query filter, session grouping, /clear split)
status: blocked
cause: concurrent writers in same tree; build and tests cannot pass
live_build_errors:
  - internal/harness/repl.go:1348 r.clearMark undefined
  - internal/harness/tui.go:579 m.sendImport undefined
  - internal/harness/tui.go:962 m.sendImport undefined
parallel_writer_evidence:
  - internal/harness/resume_group_test.go mtime 16:21:17 (my write 16:20:28 of resume_group.go; test file I wrote was replaced)
  - resume_group_test.go uses: groupSessions(l, marks) 2-arg, gs[0].latest, sessionGroup.Files, matchGroups, recordClearMark, loadClearMarks (not in my resume_group.go)
  - repl.go clearWithHandoff calls r.clearMark() (record-side /clear marker, not defined)
  - resume_test.go TestResumeSourceArg removed by other writer (16:21:17)
my_side_state:
  - importclaude.go: clearCommandMark const, Cwd/Name fields, loadImportArg (gofmt clean)
  - resume.go: sessionInfo Cwd/Name, resumeTarget, importSession uses loadImportArg, resumeLine uses groupList
  - resume_group.go: sessionGroup, filterGroups, groupSessions(l), continues (prev.Cleared || cur.ClearHead split), groupList
  - resume_group_test.go: overwritten by other writer (my version lost)
scratch_verify: /tmp/chaude-verify (copy + stub clearMark/sendImport); build OK
scratch_test_result: resume_group_test.go (other writer) fails to compile: too many arguments in call to groupSessions, undefined matchGroups, recordClearMark, loadClearMarks
no_action: did not overwrite other writer's repl.go, tui.go, resume_group_test.go; no commit
next: main agent to decide which design (my groupSessions(l) vs other writer's marks-based) survives; then build + go test ./...
