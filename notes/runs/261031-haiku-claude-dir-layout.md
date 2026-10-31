# claude dir layout report (read-only)
date: 2026-10-31
agent: haiku-scout

## ~/.claude top level
entries: .DS_Store .last-cleanup .last-update-result.json .ponytail-active .ponytail-statusline-nudged backups cache chrome daemon daemon-auth-cooldown daemon-auth-status.json daemon.log downloads file-history history.jsonl ide jobs mcp-needs-auth-cache.json paste-cache plugins projects (141 dirs) session-env (511 dirs) sessions settings.json shell-snapshots skills state stats-cache.json telemetry
projects count: 141 (entries under ~/.claude/projects)

## project dir -Users-leekwangseok-PycharmProjects-claude-harness
top-level .jsonl count: 166
subagent jsonl files (under */subagents/): 1008 in 80 subagent dirs (find -name subagents = 80 dirs; total match 1088 = 80 dirs + 1008 files)
newest 3 by mtime:
  0eabb4fb-6acb-4a01-99d4-8614c6929ddb.jsonl  521593 bytes  (Oct 31 12:07)
  1516837c-c0bf-46f9-b06d-64cf60bc899e.jsonl  578837 bytes  (Oct 31 10:37)
  6508f511-55bd-47c6-9c36-970b2c97f134.jsonl  1129187 bytes

## newest jsonl: 0eabb4fb-6acb-4a01-99d4-8614c6929ddb.jsonl
lines: 131 (all parsed, bad: 0)
top-level keys (union): advisorModel apiBlockIndex atis attachment base cacheControl commandUuid content cwd deliveryId effort entrypoint gitBranch hash id isMeta isSidechain keep lastPrompt leafUuid message operation origin params parentUuid perTurnEffort permissionDecision permissionMode promptId promptSource querySource queueSkipAttachments queueSkipSlashCommands queueTranscriptOnly reason rendered renderedBesideToolResult renderedRole requestDigest requestId requestRef requestedModel serverClassifierContext serverClassifierRequest sessionId shape shapeHash sourceToolAssistantUUID tail thinkingDisplay thinkingDurationMs timestamp toolUseResult tools turnCompanion turnOrigin turnPosition type userType uuid version wireToolInputs
type values (line counts): attachment 28, queue-operation 24, assistant 19, user 15, api-request-blob 13, api-request 11, atis-latch 8, last-prompt 7, tools-used 5, api-request-shape 1 (sum 131)
assistant lines with message.usage: 19
first assistant message.usage (verbatim):
{"input_tokens":2,"cache_creation_input_tokens":33775,"cache_read_input_tokens":0,"output_tokens":1593,"output_tokens_details":{"thinking_tokens":884},"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":33775,"ephemeral_5m_input_tokens":0},"inference_geo":"global","iterations":[{"input_tokens":2,"output_tokens":1593,"cache_read_input_tokens":0,"cache_creation_input_tokens":33775,"cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":33775},"type":"message"}],"speed":"standard","fallback_credit":null}
usage keys (all 19 lines): cache_creation cache_creation_input_tokens cache_read_input_tokens fallback_credit inference_geo input_tokens iterations output_tokens output_tokens_details server_tool_use service_tier speed
metadata keys on that assistant line: entrypoint=present(sdk-cli) userType=present(external) isSidechain=present(false) sessionId=present(value not printed) version=present(2.1.314) model=claude-haiku-5-5

## entrypoint counts (grep -o occurrences, not lines)
top-level 166 files: "entrypoint":"cli" 1290 | "entrypoint":"sdk-cli" 6861
subagent files 1008: "entrypoint":"cli" 3106 | "entrypoint":"sdk-cli" 35967

## not found / caveats
- no "sdk" entrypoint variant other than sdk-cli; no other values seen.
- subagent transcripts DO exist (not absent).
- system rule says never open raw .jsonl; only structural extraction (jq/python keys, type counts) was used, plus the one usage object requested.
- ~/.claude/projects has 141 entries, not all checked for jsonl counts.
