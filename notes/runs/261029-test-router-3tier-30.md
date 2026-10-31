# test: router 3-tier (haiku|sonnet|opus), 30 samples, production args
date: 2026-10-29
status: done (Sonnet 30/30 completed, Haiku 90/90 completed; expected tiers fixed before any model call)
repo_edits: none
commits: none
prompt_src: internal/harness/classifier.go classifyPrompt (current 3-tier text; old small/large prompt not used)
samples_src: notes/runs/261029-haiku-router-30-samples-list.md (same 30 prompts as 261029-test-router-30-prod-args.md)
expected_src: notes/runs/261029-plan-router-3tier.md tier definitions (haiku / sonnet / opus)
expected_size: haiku -> small; sonnet, opus -> large (comparable to old run labels)
runner: /tmp/router3t/run.py (not in repo); raw: /tmp/router3t/out/records.jsonl
prompt_copy: /tmp/router3t/classify_prompt.txt (extracted from classifier.go)

## prod args (classifier.go:141-144 + flags requested)
claude -p <in> --model <sonnet|haiku> --output-format json --system-prompt <classifyPrompt> --tools "" --no-session-persistence --max-budget-usd 0.10 --strict-mcp-config
env: env -u ANTHROPIC_API_KEY ; cwd: fresh temp dir ; timeout 120s ; parallel 3 ; retry 1 per failed call
in: "current session model: sonnet (keep it unless topic is new)\nprevious user prompt: \nuser prompt:\n<prompt>" (same as prior run)
models: sonnet = alias "sonnet" (conductorModel); haiku = alias "haiku"
failed call: rc!=0, timeout, non-JSON output, or no topic/task line (same as classify() error path)
missing tier: not a failure; tier missing or invalid -> sonnet (size-only legacy line maps small->haiku), counted as size_defaulted

## pre-check
ANTHROPIC_API_KEY: unset
ANTHROPIC_AUTH_TOKEN: unset
ANTHROPIC_BASE_URL: unset
CLAUDE_CODE_USE_BEDROCK: unset
CLAUDE_CODE_USE_VERTEX: unset
claude: /Users/leekwangseok/.local/bin/claude (2.1.312 (Claude Code))

## expected (written before any model call)
| id | expected tier | expected size | prompt (verbatim) |
|----|---------------|---------------|-------------------|
| P1 | haiku | small | README 오타 하나 고쳐줘 |
| P2 | haiku | small | parser.go의 이 함수 이름만 바꿔줘 |
| P3 | haiku | small | 버전 문자열을 0.7.3으로 올려줘 |
| P4 | opus | large | 멀티 프로세스 재연결 설계를 검토하고 트레이드오프를 정리해줘 |
| P5 | sonnet | large | 릴리즈 전에 전체 파일 동기화와 태그 생성까지 해줘 |
| P6 | opus | large | 이 에러가 왜 나는지 모르겠어, 원인 찾아줘 |
| P7 | haiku | small | config 파일 한 줄 주석 추가 |
| P8 | opus | large | 여러 파일의 로깅 방식을 통일하는 계획 세워줘 |
| P9 | haiku | small | 테스트 하나 실행하고 결과만 알려줘 |
| P10 | opus | large | 두 방식 중 어느 쪽이 나은지 의견 줘 |
| N1 | haiku | small | internal/harness/tui.go에서 'Recieve' 오타를 'Receive'로 바꿔줘 |
| N2 | haiku | small | notes/DECISIONS.md 파일이 몇 줄인지 세어줘 |
| N3 | haiku | small | README.md에 'TODO' 문자열이 몇 번 나오는지 알려줘 |
| N4 | haiku | small | clients.go의 기본 타임아웃 상수 값이 뭔지 알려줘 |
| N5 | haiku | small | dispatch_test.go 테스트 하나만 돌려서 결과 한 줄로 알려줘 |
| N6 | haiku | small | Makefile의 build 타겟 위에 주석 한 줄 추가해줘 |
| N7 | haiku | small | notes/research/billing.md 첫 번째 제목이 뭔지 찾아줘 |
| N8 | haiku | small | classifier.go에서 'when unsure' 문자열이 있는 줄 번호 알려줘 |
| N9 | haiku | small | children.go의 패키지 주석 한 줄을 더 짧게 고쳐줘 |
| N10 | haiku | small | cleanenv_test.go에 있는 테스트 함수 이름 목록 뽑아줘 |
| L1 | opus | large | 라우터 분류 결과를 캐시하는 기능을 새로 설계하고 파일별 변경 계획 세워줘 |
| L2 | opus | large | 멀티 프로세스 환경에서 세션 재연결 방식 두 가지를 비교하고 추천해줘 |
| L3 | sonnet | large | tui, repl, core 세 파일의 에러 처리 방식을 하나로 통일하는 리팩터링 진행해줘 |
| L4 | sonnet | large | 다음 릴리즈 0.8.0 준비해줘: 버전 올리기, CHANGELOG 정리, 태그 생성까지 |
| L5 | opus | large | 테스트가 간헐적으로 실패하는데 원인이 뭔지 모르겠어, 찾아서 고쳐줘 |
| L6 | opus | large | 비용 상한을 세션 단위와 일 단위 중 어느 쪽으로 두는 게 나은지 트레이드오프 정리해줘 |
| L7 | opus | large | 병렬 터미널 실행 시 로그 파일이 섞이는 문제, 해결 방향을 전략적으로 정해줘 |
| L8 | opus | large | Haiku 워커 풀 크기를 어떻게 정할지 설계 문서 초안 쓰고 여러 파일 영향 분석해줘 |
| L9 | opus | large | 라우터 프롬프트를 도메인별로 나누는 게 맞을지 판단하고 근거 제시해줘 |
| L10 | sonnet | large | 서브에이전트 반환 포맷을 표준화하는 작업을 여러 패키지에 걸쳐 진행해줘 |

expected counts: haiku 15 (P1,P2,P3,P7,P9,N1-N10), sonnet 4 (P5,L3,L4,L10), opus 11 (P4,P6,P8,P10,L1,L2,L5-L9)
label doubts (decided before calls, not changed after):
- P3 expected haiku (version bump with exact text); old run labeled small, same size
- L3, L10: scoped multi-file refactor/standardize -> sonnet; could be read as opus (design open)
- P8, L8: "계획/설계 문서" -> opus; could be read as sonnet (analysis)
- sonnet expectations are few (4); tier accuracy for sonnet is therefore weakly determined

## results (per prompt; final parsed tier; "-" = no result)
| id | expected | sonnet | haiku r1 | haiku r2 | haiku r3 | haiku consistent | sonnet size ok | haiku size ok r1/r2/r3 |
|----|----------|--------|----------|----------|----------|------------------|----------------|------------------------|
| P1 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| P2 | haiku | haiku | sonnet | haiku | haiku | NO | Y | N/Y/Y |
| P3 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| P4 | opus | opus | opus | opus | opus | yes | Y | Y/Y/Y |
| P5 | sonnet | sonnet | sonnet | sonnet | sonnet | yes | Y | Y/Y/Y |
| P6 | opus | opus | opus | opus | opus | yes | Y | Y/Y/Y |
| P7 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| P8 | opus | opus | opus | opus | sonnet | NO | Y | Y/Y/Y |
| P9 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| P10 | opus | opus | opus | opus | opus | yes | Y | Y/Y/Y |
| N1 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| N2 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| N3 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| N4 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| N5 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| N6 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| N7 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| N8 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| N9 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| N10 | haiku | haiku | haiku | haiku | haiku | yes | Y | Y/Y/Y |
| L1 | opus | opus | opus | opus | opus | yes | Y | Y/Y/Y |
| L2 | opus | opus | opus | opus | opus | yes | Y | Y/Y/Y |
| L3 | sonnet | sonnet | sonnet | sonnet | opus | NO | Y | Y/Y/Y |
| L4 | sonnet | sonnet | sonnet | sonnet | sonnet | yes | Y | Y/Y/Y |
| L5 | opus | opus | opus | opus | opus | yes | Y | Y/Y/Y |
| L6 | opus | opus | opus | opus | opus | yes | Y | Y/Y/Y |
| L7 | opus | opus | opus | opus | opus | yes | Y | Y/Y/Y |
| L8 | opus | opus | opus | opus | opus | yes | Y | Y/Y/Y |
| L9 | opus | opus | opus | opus | opus | yes | Y | Y/Y/Y |
| L10 | sonnet | sonnet | sonnet | sonnet | sonnet | yes | Y | Y/Y/Y |

## confusion (expected -> predicted, counts)
sonnet: haiku->haiku 15, opus->opus 11, sonnet->sonnet 4
haiku: haiku->haiku 44, haiku->sonnet 1, opus->opus 32, opus->sonnet 1, sonnet->opus 1, sonnet->sonnet 11

## summary
sonnet_completed: 30/30 (calls 30, attempts 30)
sonnet_tier_accuracy: 30/30 = 100.0%
sonnet_size_accuracy: 30/30 = 100.0%
haiku_completed: 90/90 (calls 90, attempts 90, skipped 0)
haiku_r1_tier_accuracy: 29/30 = 96.7%
haiku_r2_tier_accuracy: 30/30 = 100.0%
haiku_r3_tier_accuracy: 28/30 = 93.3%
haiku_tier_accuracy_total: 87/90 = 96.7%
haiku_size_accuracy_total: 89/90 = 98.9%
haiku_consistency (all 3 repeats same tier): 27/30; of which correct: 27
large_to_small (expected sonnet/opus, predicted haiku): sonnet 0, haiku 0 (per repeat r1/r2/r3 in table)
parse_failures (final, after 1 retry): sonnet 0, haiku 0
missing_or_invalid_tier (size_defaulted, final results): sonnet 0, haiku 0
retries_used: 0 calls with a second attempt
cost_usd_total (all attempts incl. failed, from total_cost_usd): 0.162122
cost_usd_sonnet: 0.134306, haiku: 0.027816
budget_stop: no
