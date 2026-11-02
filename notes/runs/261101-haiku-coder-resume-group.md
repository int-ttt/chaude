status: blocked
task: chaude TUI /resume upgrade (group + query filter + clear boundary)
cause: internal/harness/resume_group.go replaced on disk at 16:20:28 by another writer; my helpers gone
on_disk_design: sessionGroup{Sessions}; groupSessions(l) 1-arg; matches(q); filterGroups; continues(prev,cur); groupMaxGap=48h; groupMinOverlap=0.2
missing_for_build: clearMark (repl.go:1348), sendImport (tui.go:579, :962), matchGroups, loadImportArg, mergeTranscripts, recordClearMark, loadClearMarks, clearCommandMark const
test_build_errors: resume_group_test.go:57 groupSessions arity (3 vs 2 args in my test); undefined matchGroups; unknown field Files
git_state: M importclaude.go repl.go resume.go resume_test.go tui.go; ?? resume_group.go resume_group_test.go
concurrent_writers: claude -p processes active (pids 88674-88676, 89314); LEDGER 16:18 and 16:22 [agent] entries
attempts: 0 edits to resume_group.go (not overwritten, per no-silent-overwrite)
build_result: go build ./... FAIL (not rerun after this note)
decision_needed: keep on-disk design (adapt resume.go, tui.go, repl.go, resume_group_test.go) OR restore my design (overwrite resume_group.go after writer stops)
