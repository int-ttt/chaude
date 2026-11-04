# core B run: record turn/folder + mcp guard (2026-11-04)
status: partial
branch: rust-port
commits:
  - 8b976db fix(core): mcp config temp dir has a guard that removes it on drop
  - 0bd579b feat(core): record project folders port from Go record_folder.go
  - 3c0a753 feat(core): record turn port from Go record_turn.go
done:
  - item0 mcp temp dir guard: mcp::McpConfig (Drop removes dir), mcp::start_owned; mcp::start kept as wrapper
  - item1 record_folder.rs: folder_name, project_name/dir, migrate_root, list_projects (mtime), filter_projects, load_project, project_lines (date fn passed in)
  - item1 record_turn.rs: skip, measure_line (serde_json map, Go %.4g), RecordState{topic_for, save}, resume, pick
  - record.rs: cat_mark_span + strip_cat_marks (cat_mark kept as wrapper)
  - plan/rust-port.md phase 3 updated
checks:
  - cargo fmt -p chaude-core -- --check -> PASS
  - cargo build -p chaude-core -> PASS
  - cargo test -p chaude-core -> PASS (117 unit, 55 engine; record_ filter 15 pass)
  - cargo clippy -p chaude-core -- -D warnings -> PASS
hooks_for_A (api.rs, not edited):
  - mcp_cfg field: Option<PathBuf> -> Option<mcp::McpConfig> via mcp::start_owned; mcp_config() maps to path (leak fix takes effect)
  - turn end: call record_turn::RecordState::save(root=record::dir_for(cwd), cwd, prompt, &Stamp from local time); set last_reply/last_measure before it
  - /resume (plain path): record_turn::resume(root, arg, date_fn) ; /clear sets RecordState.split = true
  - TUI owner: crates/cli/src/tui/records.rs duplicates record_folder/record_turn logic; switch to core later (not edited)
remaining (not started):
  - item2 orch (orch.go 63 LOC, orchws.go 134 LOC)
  - item3 rotation details (rotate_defer.go 107 LOC)
  - item4 import/--from-claude (importclaude.go 233 LOC, importview.go 214 LOC)
  - item5 metrics parity (metrics.go 62 LOC, metricscmd.go 110 LOC; crates/core/src/metrics.rs exists)
notes:
  - working tree has unrelated changes: crates/core/src/handoff.rs (A), plan/caveman-structure.md, .claude/hooks/main-thread-guard.py. Not touched.
  - Korean strings in record_folder/record_turn are Go program output, kept verbatim.
