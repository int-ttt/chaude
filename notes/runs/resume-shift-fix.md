# resume-shift-fix (2026-10-29, opus-architect)
status: fixed in tree, not committed
bug2_shift_cause: JediTerm TerminalPanel.drawInputMethodUncommitedChars xCoord=(cursor.getCoordX()+1)*charWidth -> IME preedit drawn 1 cell right of real cursor; committed text correct. Row was never the issue (0.6.11 curUp+bgH fix irrelevant).
bug2_fix: tui.go imeColShift(): CHAUDE_IME_SHIFT=<int> override; TERMINAL_EMULATOR=JetBrains-JediTerm -> -1; else 0. syncCursor curCol=max(1,1+2+col+1+imeShift).
bug1_cause_a: TUI never rendered imported turns; importSession only printed a status note -> screen shows no history.
bug1_cause_b: after import, route() (conductor model switch) / model change called rotate(): handoffPrompt to fresh context-less child, then pendingSnapshot overwritten -> imported context lost.
bug1_fix: importedMsg{turns} (tui.go) replaces m.blocks with user/asst blocks (last 100 turns + note); resume.go importSession sends it when r.ui!=nil; repl.go rotate(): if pendingSnapshot!="" -> Close, start, keep snapshot, log outcome=snapshot_kept, no handoff.
seams: repl.go var newSessionFn=NewSession; core.go Session.Close returns early if cmd==nil (fake).
tests: resumeshift_test.go TestJediTermIMEColumnShift, TestImportShowsTurnsInTUI, TestRotateKeepsPendingSnapshot. pre-fix: all 3 FAIL (col 8 want 7; blocks missing turns; rotate blocked on handoff 3s). post-fix: PASS.
checks: go build ./... ok; go vet ./... ok; go test -count=1 -race ./internal/harness ok.
unverified: real JetBrains screen; new JetBrains "reworked" terminal may lack +1 offset -> CHAUDE_IME_SHIFT=0; -1 shift moves the committed-text cursor too (cursor not drawn by JediTerm while composing; app draws its own reverse cell).
