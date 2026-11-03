task: rust-port prototype, crate rust/chaude-rs (bin), branch rust-port, no push
status: done
toolchain: brew rustup present; `rustup-init` not shipped by brew formula, used `rustup set profile minimal` + `rustup default stable`
cargo: 1.99.1 at ~/.cargo/bin/cargo (PATH needs ~/.cargo/bin)
files:
  - rust/chaude-rs/Cargo.toml (edition 2024; deps serde derive, serde_json)
  - rust/chaude-rs/Cargo.lock
  - rust/chaude-rs/.gitignore (/target)
  - rust/chaude-rs/src/main.rs (CLI loop, inline renderer, --parse-only)
  - rust/chaude-rs/src/event.rs (serde enums, #[serde(other)] Unknown, tests)
  - rust/chaude-rs/src/session.rs (argv from Go sessionArgs, spawn, reader thread, stdin writers, tests)
cargo results:
  - cargo build --release: PASS (12 dead_code warnings, fields parsed but unread)
  - cargo test: PASS, 13 passed, 0 failed
failed compile/check runs: 0 (one tool-guard refusal before cargo ran, not a compile)
event types covered (top-level): system, stream_event (message_*, content_block_start/delta/stop; text_delta, thinking_delta, input_json_delta, signature_delta), assistant (text, thinking, tool_use), user (string or tool_result/text blocks), result, control_request (can_use_tool), rate_limit_event, unknown catch-all
fixture: src/main/intt/internal/harness/testdata/bg_turn_live.jsonl 90 lines, 0 parse errors
parse-only RSS: /usr/bin/time -l release binary on fixture: 2211840 bytes max RSS (1508096 peak footprint), real 0.26s
loop smoke: stub claude script (/tmp/chaude-rs-fake/fake-claude.sh) -> prompt forwarded, tool/text inline, exit 0
doubts:
  - can_use_tool auto-denied (no permission prompt in prototype); Go asks user
  - subagent (parent_tool_use_id set) events not rendered
  - child env not cleaned (Go uses cleanEnv/withCacheTTL); not ported
  - timeout not available on macOS; loop relies on child EOF
  - commit trailer per brief: Co-Authored-By Claude Opus 5.5 (system reminder asks Haiku 5.5)
