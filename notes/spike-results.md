# Phase 0 spike results
asof: 2026-10-28
claude: 2.1.312 (Claude Code), macOS, model haiku (claude-haiku-5-5), login = existing subscription (init apiKeySource: none)
evidence: samples/*.jsonl (raw recordings), spike/*.py (probes), tests/test_acceptance.py
tag: OK=verified here, PARTIAL, N/A=not tested

## [V] items
- stream-json in/out: OK. cmd `claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages`. `--verbose` required for stream-json output [seen in usage; not tested without]. Input line: {"type":"user","message":{"role":"user","content":"..."}}. Multiple user lines on one process = multiple turns, one session_id.
- event types seen: system(init,status,thinking_tokens,hook_started,hook_response) user(replay only with --replay-user-messages) stream_event(message_start,content_block_start/delta/stop,message_delta,message_stop) assistant rate_limit_event result control_request control_response. Unknown types must be kept verbatim (core.py does for unparsable lines).
- partial streaming: OK. stream_event/content_block_delta/delta.type=text_delta .text; thinking deltas also present. Subagent events carry parent_tool_use_id (filter on null for main thread).
- usage fields: OK. assistant.message.usage and result.usage: input_tokens, cache_creation_input_tokens, cache_read_input_tokens, output_tokens (+cache_creation{ephemeral_1h/5m}, iterations[], service_tier). result also: total_cost_usd, modelUsage, num_turns, duration_ms, ttft_ms, permission_denials, terminal_reason, is_error, subtype. NOTE result.usage may sum multiple API calls in a turn; per-call context size = last main-thread assistant usage (used by harness as ctx).
- baseline: fresh session in near-empty cwd already ~19K ctx tokens (system prompt+tools; claude-lean cwd with hooks larger). Rotation threshold must be well above ~20K; cache_creation on first turn after rotation ~= baseline.
- rate_limit_event: OK, bonus. rate_limit_info.unifiedWindows.five_hour/seven_day.utilization (0-1), resetsAt. Harness shows 5h utilization per turn -> direct way to measure harness-vs-interactive limit burn (design s8, s11). Whether utilization is exact/stable: uncertain.
- permission: OK via `--permission-prompt-tool stdio`. Emits {"type":"control_request","request_id","request":{"subtype":"can_use_tool","tool_name","input","permission_suggestions","tool_use_id"}}; reply {"type":"control_response","response":{"subtype":"success","request_id","response":{"behavior":"allow","updatedInput":{...}}}} (deny: behavior deny + message). Verified with Write: file created after allow. `--permission-prompt-tool` is not in --help (hidden flag); `--permission-prompts host` alone does NOT emit control_requests (tool is denied, appears in result.permission_denials). `--permission-prompts none` = auto-deny. Tools already allowed by user settings (e.g. Bash echo) never prompt.
- interrupt: OK. Send {"type":"control_request","request_id":..,"request":{"subtype":"interrupt"}} -> control_response success, then result subtype error_during_execution, is_error true, terminal_reason aborted_streaming. Same process stays alive and next turn works (verified). No SIGINT needed, so Windows-safe path likely [Windows untested].
- session id / resume: OK. init.session_id on every event; --session-id <uuid> honored (verified). `--resume <id>` in new process keeps same id and remembers history (verified: recalled first-turn word). `--resume <id> --fork-session` gives new id with history (verified).
- hooks in -p: OK. Project hooks of claude-lean loaded (hook_started/hook_response events with --include-hook-events: 4 each for 2 turns).
- slash in -p: PARTIAL. `/context` sent as a user message returned context table (Tokens used / window; haiku window shown 150k) in result. Custom skills/commands not tested. Interactive-only ones (/model, /login) not tested.
- image input: PARTIAL. Base64 image content block accepted by protocol (no error), but model answered it could not process the image; my 1x1 PNG may be invalid or model/protocol limits. Inconclusive; retest with a real image.
- ANTHROPIC_API_KEY: harness strips it from child env (core.clean_env); init apiKeySource none confirmed. --bare not used.
- fork subagent default / disabling: N/A.
- Windows (.cmd resolution, signals): N/A.
- credentials: harness never reads keychain/credentials files; only spawns `claude` and passes env minus API key.

## Minimal REPL (harness/)
- `python3 -m harness [--model haiku] [--threshold N] [--max-auto-rotations N] [--transcript PATH] [--show-rotation] [--auto-allow]`; commands /rotate /quit; Ctrl-C = interrupt.
- per turn line: ctx (last call input+cr+cc), in/cr/cc/out (result.usage), cost, 5h utilization, session id prefix, rotation count.
- rotation: after result, if ctx > threshold -> ask old session for handoff note (hidden turn), snapshot = note + changed files + last N turns verbatim (cap chars), kill child, spawn fresh session (no --resume), snapshot prepended to next user message. Screen never cleared; marker line only with --show-rotation. Own transcript JSONL (user/assistant/rotate/permission records, snapshot stored).
- acceptance: samples/accept_run.txt (session da2968d9 -> c192ffc6, codeword PELICAN-42 recalled after rotation); tests/test_acceptance.py passes 2/2 (rotation carries ZEBRA-77; >3 text_delta events streamed).

## Doubts / next
- handoff turn costs one extra API call per rotation; not yet measured net savings (needs a long session run).
- snapshot quality only tested on a one-fact conversation.
- python: default python3 is 3.9.6 on this Mac; code runs on 3.9 (tested) and 3.13 (probes).
- permission prompt blocks the reader (readline on same stdin); fine for spike.

## Auto-compact override (verified 2026-10-28)
- default rotation threshold 200k; child env CLAUDE_CODE_AUTO_COMPACT_WINDOW=250000 (--compact-window, 0=off). spike/probe_autocompact.py: /context in sonnet showed window 1m -> 250k with env and also with CLI flag `--autocompact 250000`; autocompact buffer stays 33k, so compaction ~217k > 200k threshold. Note: it LOWERS the window for 1m models, raises for 150k (haiku). No settings files touched.
- update: user chose window 300000 (rotation threshold stays 200k). Compaction ~267k (300k-33k buffer).
