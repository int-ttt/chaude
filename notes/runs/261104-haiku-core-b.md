# 261104 haiku core-b (rust-port phase 3)
status: partial
commits:
- d50d91f (pre-existing, verified) resident MCP loopback requires per-run token line
- 2cd168d fix(core): mcp::new_token reads 32 bytes from /dev/urandom (was RandomState, ~128 bits)
- ab3d012 feat(core): record.rs, Go record.go file format
done:
- item 1 MCP loopback auth: done (token in mcp.json env CHAUDE_MCP_TOKEN, 0600, not argv; wrong/missing token gets no reply)
- item 2 record: record.rs covers render, parse_entries, escape/unescape, topic_of (cat/cat-switch), slugify, next_number, list_files, last_record_num, append_turn (File::lock), dir_for, Stamp
remaining:
- item 2 tail: record_turn.go (turn assembly from stream events, 169 LOC), record_folder.go (project folders, 168 LOC); engine wiring not started
- item 3 orch: orch.go (63 LOC), orchws.go (134 LOC)
- item 4 rotation details: rotate_defer.go (107 LOC); idle rotation already in f6aca83
- item 5 import/--from-claude: importclaude.go (233 LOC), importview.go (214 LOC)
- item 6 metrics: metrics.rs exists (Metrics::log); metrics.go (62 LOC) + metricscmd.go (110 LOC) not yet diffed for parity
open:
- mcp::start leaks temp dir chaude-mcp-<pid>-<nanos> per call; listener thread lives until exit (Go also never removes its dir)
- Stamp needs local wall-clock from caller; crate has no time-zone dep
- record::dir_for shells out to git; callers must pass cwd
hooks needed in coder A files (api.rs, handoff.rs, resident.rs, UiEvent, tests/engine.rs): none for items 1-2
tests: cargo fmt -p chaude-core -- --check PASS; cargo build -p chaude-core PASS; cargo test -p chaude-core PASS (98 unit, 54 engine); cargo clippy -p chaude-core -- -D warnings PASS
not pushed
