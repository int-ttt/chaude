# effort 비교: unset / low / medium (메인 Opus, claude -p)

key: date: 2026-11-02
key: commit: 87bb1d32e0b4dc6eb435fe1b34a07bb98e994c0f (기본 medium 변경, .claude/settings.json)
key: model: claude-opus-5-5 (claude 2.1.316)
key: prompt: "Compute 17*23 + 9*8. Reply with only the integer." (expected 463)
key: effort-mechanism: chaude는 --effort 플래그를 넘기지 않음 (internal/harness/core.go sessionArgs에 effort 없음). 프로젝트 cwd의 .claude/settings.json effortLevel이 메인 effort를 정함. 실험은 temp dir의 .claude/settings.json에 effortLevel 주입 (unset = 키 없음 `{}`).
key: cmd: (cd <tmp>/<level> && claude -p "<prompt>" --model claude-opus-5-5 --output-format json --setting-sources project)
key: setting-sources-project-only: 사용자 레벨 ~/.claude/settings.json에 effortLevel "medium"이 있어 unset이 medium으로 섞이지 않도록 project만 로드. chaude 메인 실행과 다른 점.
key: tmp-base: /tmp/effort-bench.hVMahc (temp dir별로 cwd가 달라 system prompt prefix가 달라 캐시가 분리됨)

## 결과 (run별)

| level | run | cost_usd | total_tokens | duration_ms | answer | cache_creation | cache_read | output | thinking |
|---|---|---|---|---|---|---|---|---|---|
| unset | 1 | 0.121740 | 15214 | 1705 | 463 | 15209 | 0 | 3 | 0 |
| unset | 2 | 0.003110 | 15214 | 2498 | 463 | 0 | 15209 | 3 | 0 |
| low | 1 | 0.051999 | 15211 | 2789 | 463 | 6268 | 8938 | 3 | 0 |
| low | 2 | 0.003109 | 15211 | 1616 | 463 | 0 | 15206 | 3 | 0 |
| medium | 1 | 0.052024 | 15214 | 2765 | 463 | 6271 | 8938 | 3 | 0 |
| medium | 2 | 0.003110 | 15214 | 1581 | 463 | 0 | 15209 | 3 | 0 |

total_tokens = input + cache_creation + cache_read + output (input 2 in every run).

## 평균 (n=2 per level)

| level | avg cost_usd | avg total_tokens | avg duration_ms | correct |
|---|---|---|---|---|
| unset | 0.0624 | 15214 | 2101.5 | 2/2 |
| low | 0.0276 | 15211 | 2202.5 | 2/2 |
| medium | 0.0276 | 15214 | 2173.0 | 2/2 |

주의: run 1은 캐시 생성(쓰기) 비용이 들어 비싸고, run 2는 캐시 읽기라 $0.003대. 평균은 캐시 상태에 지배됨. 세 레벨 모두 run 1이 cache write였던 것은 레벨별 temp dir이 달라 prefix가 새로 만들어진 탓이며 effort 효과가 아님.
주의: 출력 3 토큰, thinking 0 (모든 run). 이 프롬프트에서는 effort가 출력/추론 토큰에 드러나지 않음. effort가 실제 적용됐는지 이 실험으로는 증명 안 됨.

## smoke: 빌드된 chaude

- cmd: cd src/main/intt && go build -o <tmp>/chaude-smoke ./cmd/chaude -> BUILD_OK
- cmd: chaude-smoke gen-settings {mac,linux,windows} -> 세 OS 모두 "effortLevel": "medium"
- sessionArgs (core.go) effort 플래그: 없음 (grep 0건). 메인 effort는 settings.json에서 옴.
- TUI 실제 기동(pty)은 하지 않음.

## raw (claude -p --output-format json 원본)

### unset run1
```json
{"duration_api_ms":1202,"stop_reason":"end_turn","session_id":"e90151ac-2cd4-4d6f-bc04-289f560ea0a7","total_cost_usd":0.12174,"usage":{"input_tokens":2,"cache_creation_input_tokens":15209,"cache_read_input_tokens":0,"output_tokens":3,"output_tokens_details":{"thinking_tokens":0},"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":15209,"ephemeral_5m_input_tokens":0},"inference_geo":"global","iterations":[{"input_tokens":2,"output_tokens":3,"cache_read_input_tokens":0,"cache_creation_input_tokens":15209,"cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":15209},"type":"message"}],"speed":"standard","fallback_credit":null},"modelUsage":{"claude-opus-5-5":{"inputTokens":2,"outputTokens":3,"cacheReadInputTokens":0,"cacheCreationInputTokens":15209,"webSearchRequests":0,"costUSD":0.12174,"contextWindow":1000000,"maxOutputTokens":128000,"thinkingTokens":0,"cacheCreationEphemeral5mInputTokens":0,"cacheCreationEphemeral1hInputTokens":15209,"canonicalModel":"claude-opus-5-5","provider":"firstParty","costBasis":"list"}},"permission_denials":[],"terminal_reason":"completed","fast_mode_state":"off","fast_mode_disabled_reason":"sdk_opt_in_required","subagent_stats":{"spawned":0,"requested":{"background":0,"foreground":0,"unset":0},"started_in_background":0,"max_depth":0,"spawned_by_subagents":0,"completed":0,"failed":0,"killed":{"parent":0,"user":0,"system":0},"refused":{"depth_limit":0,"concurrency_limit":0,"budget":0},"by_type":{}},"safety_stops":0,"is_error":false,"num_turns":1,"subtype":"success","api_error_status":null,"result":"463","ttft_ms":1671,"type":"result","duration_ms":1705,"uuid":"0f727b98-c50e-4c60-8eeb-af7083909d3f","ttft_stream_ms":1321,"time_to_request_ms":503,"first_content_frame_ms":1322,"queued_turn_count":0,"result_index":0}
```

### unset run2
```json
{"duration_api_ms":2044,"stop_reason":"end_turn","session_id":"58bc5305-76f0-4be7-ac8f-7b650a433ce8","total_cost_usd":0.0031098000000000002,"usage":{"input_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":15209,"output_tokens":3,"output_tokens_details":{"thinking_tokens":0},"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":0},"inference_geo":"global","iterations":[{"input_tokens":2,"output_tokens":3,"cache_read_input_tokens":15209,"cache_creation_input_tokens":0,"cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard","fallback_credit":null},"modelUsage":{"claude-opus-5-5":{"inputTokens":2,"outputTokens":3,"cacheReadInputTokens":15209,"cacheCreationInputTokens":0,"webSearchRequests":0,"costUSD":0.0031098000000000002,"contextWindow":1000000,"maxOutputTokens":128000,"thinkingTokens":0,"cacheCreationEphemeral5mInputTokens":0,"cacheCreationEphemeral1hInputTokens":0,"canonicalModel":"claude-opus-5-5","provider":"firstParty","costBasis":"list"}},"permission_denials":[],"terminal_reason":"completed","fast_mode_state":"off","fast_mode_disabled_reason":"sdk_opt_in_required","subagent_stats":{"spawned":0,"requested":{"background":0,"foreground":0,"unset":0},"started_in_background":0,"max_depth":0,"spawned_by_subagents":0,"completed":0,"failed":0,"killed":{"parent":0,"user":0,"system":0},"refused":{"depth_limit":0,"concurrency_limit":0,"budget":0},"by_type":{}},"safety_stops":0,"is_error":false,"num_turns":1,"subtype":"success","api_error_status":null,"result":"463","ttft_ms":2476,"type":"result","duration_ms":2498,"uuid":"ff946e32-ee8c-4d48-8ac7-d6e652da0058","ttft_stream_ms":2380,"time_to_request_ms":453,"first_content_frame_ms":2380,"queued_turn_count":0,"result_index":0}
```

### low run1
```json
{"duration_api_ms":2214,"stop_reason":"end_turn","session_id":"18123e53-1e05-4104-a748-ef5bb59cb7c3","total_cost_usd":0.0519996,"usage":{"input_tokens":2,"cache_creation_input_tokens":6268,"cache_read_input_tokens":8938,"output_tokens":3,"output_tokens_details":{"thinking_tokens":0},"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":6268,"ephemeral_5m_input_tokens":0},"inference_geo":"global","iterations":[{"input_tokens":2,"output_tokens":3,"cache_read_input_tokens":8938,"cache_creation_input_tokens":6268,"cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":6268},"type":"message"}],"speed":"standard","fallback_credit":null},"modelUsage":{"claude-opus-5-5":{"inputTokens":2,"outputTokens":3,"cacheReadInputTokens":8938,"cacheCreationInputTokens":6268,"webSearchRequests":0,"costUSD":0.0519996,"contextWindow":1000000,"maxOutputTokens":128000,"thinkingTokens":0,"cacheCreationEphemeral5mInputTokens":0,"cacheCreationEphemeral1hInputTokens":6268,"canonicalModel":"claude-opus-5-5","provider":"firstParty","costBasis":"list"}},"permission_denials":[],"terminal_reason":"completed","fast_mode_state":"off","fast_mode_disabled_reason":"sdk_opt_in_required","subagent_stats":{"spawned":0,"requested":{"background":0,"foreground":0,"unset":0},"started_in_background":0,"max_depth":0,"spawned_by_subagents":0,"completed":0,"failed":0,"killed":{"parent":0,"user":0,"system":0},"refused":{"depth_limit":0,"concurrency_limit":0,"budget":0},"by_type":{}},"safety_stops":0,"is_error":false,"num_turns":1,"subtype":"success","api_error_status":null,"result":"463","ttft_ms":2769,"type":"result","duration_ms":2789,"uuid":"4fe8a606-1af9-45fc-ade4-86c42411d765","ttft_stream_ms":2740,"time_to_request_ms":572,"first_content_frame_ms":2740,"queued_turn_count":0,"result_index":0}
```

### low run2
```json
{"duration_api_ms":1264,"stop_reason":"end_turn","session_id":"7858f9a5-5e5a-4441-8afb-b10991d76ccc","total_cost_usd":0.0031092000000000003,"usage":{"input_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":15206,"output_tokens":3,"output_tokens_details":{"thinking_tokens":0},"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":0},"inference_geo":"global","iterations":[{"input_tokens":2,"output_tokens":3,"cache_read_input_tokens":15206,"cache_creation_input_tokens":0,"cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard","fallback_credit":null},"modelUsage":{"claude-opus-5-5":{"inputTokens":2,"outputTokens":3,"cacheReadInputTokens":15206,"cacheCreationInputTokens":0,"webSearchRequests":0,"costUSD":0.0031092000000000003,"contextWindow":1000000,"maxOutputTokens":128000,"thinkingTokens":0,"cacheCreationEphemeral5mInputTokens":0,"cacheCreationEphemeral1hInputTokens":0,"canonicalModel":"claude-opus-5-5","provider":"firstParty","costBasis":"list"}},"permission_denials":[],"terminal_reason":"completed","fast_mode_state":"off","fast_mode_disabled_reason":"sdk_opt_in_required","subagent_stats":{"spawned":0,"requested":{"background":0,"foreground":0,"unset":0},"started_in_background":0,"max_depth":0,"spawned_by_subagents":0,"completed":0,"failed":0,"killed":{"parent":0,"user":0,"system":0},"refused":{"depth_limit":0,"concurrency_limit":0,"budget":0},"by_type":{}},"safety_stops":0,"is_error":false,"num_turns":1,"subtype":"success","api_error_status":null,"result":"463","ttft_ms":1567,"type":"result","duration_ms":1616,"uuid":"6567a2c9-22ef-42f9-b46d-2e1e6d15454a","ttft_stream_ms":1146,"time_to_request_ms":352,"first_content_frame_ms":1146,"queued_turn_count":0,"result_index":0}
```

### medium run1
```json
{"duration_api_ms":2452,"stop_reason":"end_turn","session_id":"d3b430b9-1828-4e16-ba6f-313e771eb30a","total_cost_usd":0.052023599999999996,"usage":{"input_tokens":2,"cache_creation_input_tokens":6271,"cache_read_input_tokens":8938,"output_tokens":3,"output_tokens_details":{"thinking_tokens":0},"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":6271,"ephemeral_5m_input_tokens":0},"inference_geo":"global","iterations":[{"input_tokens":2,"output_tokens":3,"cache_read_input_tokens":8938,"cache_creation_input_tokens":6271,"cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":6271},"type":"message"}],"speed":"standard","fallback_credit":null},"modelUsage":{"claude-opus-5-5":{"inputTokens":2,"outputTokens":3,"cacheReadInputTokens":8938,"cacheCreationInputTokens":6271,"webSearchRequests":0,"costUSD":0.052023599999999996,"contextWindow":1000000,"maxOutputTokens":128000,"thinkingTokens":0,"cacheCreationEphemeral5mInputTokens":0,"cacheCreationEphemeral1hInputTokens":6271,"canonicalModel":"claude-opus-5-5","provider":"firstParty","costBasis":"list"}},"permission_denials":[],"terminal_reason":"completed","fast_mode_state":"off","fast_mode_disabled_reason":"sdk_opt_in_required","subagent_stats":{"spawned":0,"requested":{"background":0,"foreground":0,"unset":0},"started_in_background":0,"max_depth":0,"spawned_by_subagents":0,"completed":0,"failed":0,"killed":{"parent":0,"user":0,"system":0},"refused":{"depth_limit":0,"concurrency_limit":0,"budget":0},"by_type":{}},"safety_stops":0,"is_error":false,"num_turns":1,"subtype":"success","api_error_status":null,"result":"463","ttft_ms":2749,"type":"result","duration_ms":2765,"uuid":"16c8a56e-5e39-44c7-ae87-f8243407d0fb","ttft_stream_ms":2714,"time_to_request_ms":314,"first_content_frame_ms":2714,"queued_turn_count":0,"result_index":0}
```

### medium run2
```json
{"duration_api_ms":1168,"stop_reason":"end_turn","session_id":"c59feaf2-366e-4dec-86c9-c236153ff337","total_cost_usd":0.0031098000000000002,"usage":{"input_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":15209,"output_tokens":3,"output_tokens_details":{"thinking_tokens":0},"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":0},"inference_geo":"global","iterations":[{"input_tokens":2,"output_tokens":3,"cache_read_input_tokens":15209,"cache_creation_input_tokens":0,"cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard","fallback_credit":null},"modelUsage":{"claude-opus-5-5":{"inputTokens":2,"outputTokens":3,"cacheReadInputTokens":15209,"cacheCreationInputTokens":0,"webSearchRequests":0,"costUSD":0.0031098000000000002,"contextWindow":1000000,"maxOutputTokens":128000,"thinkingTokens":0,"cacheCreationEphemeral5mInputTokens":0,"cacheCreationEphemeral1hInputTokens":0,"canonicalModel":"claude-opus-5-5","provider":"firstParty","costBasis":"list"}},"permission_denials":[],"terminal_reason":"completed","fast_mode_state":"off","fast_mode_disabled_reason":"sdk_opt_in_required","subagent_stats":{"spawned":0,"requested":{"background":0,"foreground":0,"unset":0},"started_in_background":0,"max_depth":0,"spawned_by_subagents":0,"completed":0,"failed":0,"killed":{"parent":0,"user":0,"system":0},"refused":{"depth_limit":0,"concurrency_limit":0,"budget":0},"by_type":{}},"safety_stops":0,"is_error":false,"num_turns":1,"subtype":"success","api_error_status":null,"result":"463","ttft_ms":1547,"type":"result","duration_ms":1581,"uuid":"865ed8ba-9b94-42b1-9098-8c0d2f58d6fb","ttft_stream_ms":1172,"time_to_request_ms":412,"first_content_frame_ms":1172,"queued_turn_count":0,"result_index":0}
```

