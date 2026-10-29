# batch-input-thinking
change: tui.go doneMsg merges m.queue via mergePrompts (join "\n\n") -> one turn
change: mainStatusText() "생각중-<model>-메인(<첫줄 요약>)" replaces spinner text (fg agent / busy); agents rows unchanged (agentStatusText, stRunning "실행 중" kept)
change: placeholder(): idle "대기 - 프롬프트 입력", busy "생각중 - 추가 입력은 한 번에 합쳐 처리"
tests: batchinput_test.go (merge, status); panel_test.go "생각중..."->"생각중-", TestQueueSentOnDoneInOrder -> merged
doubt: agent rows still say "실행 중" not "생각중"; merged prompt shown as one user block; simultaneous input while idle not merged (only queue while busy)
