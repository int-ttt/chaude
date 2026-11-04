# p3 remaining (haiku, 261104)
status: partial
done:
  - eefee9c AgentEnd.model (String, empty = inherited); engine.rs + tui hunk committed
  - 5b3e00b api.rs repair (HEAD did not compile: AgentInfo.model misplaced in actor(), tests outside mod tests) + rotate snapshot_kept (Go repl.go)
not_done (p3 rows still open):
  - resume grouping (resume_group.go: groupSessions, matchGroups, matchesTerm, groupList)
  - /clear marks (.clear-marks, recordClearMark, loadClearMarks, TestClearMarks)
  - resume target pick, TestLoadImportArgMergesInTimeOrder, TestReadClaudeTranscriptClearFlags
  - turn spend, parse_usage, usage poll row, turnCostUSD delta
  - observed TTL; proc.rs clean-env test (apply_env test exists from 38ae75b)
  - open-Agent turn wait (TestTurnWaitsForOpenAgentToolResult; keep TestTurnQuietNoticeKeepsWaiting)
  - coordinator pool items (AgentStore put/load, ChildSet park/take/admit, pick_child at dispatch,
    WorkerPool::note_edit): owned by background agents a503c335781e4beaf / a5db51c5aab32f1f9; not started here to avoid duplicates
blockers:
  - clippy -D warnings: crates/core/src/clients.rs:459 very complex type (not api.rs; api.rs:320 is clean now)
  - chaude-cli defaults_match_go (repl_args.rs:299, expects 12): repl_args.rs:196 calls resolve_max_workers only; worker_floor not applied (38ae75b split it out). Owner: CLI.
