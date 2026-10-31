# router 30 sample prompts: list (read-only)
date: 2026-10-29
source_text: /tmp/router30/samples.json (prompt text; not in the two named notes files)
source_labels: /tmp/router30/out results table in notes/runs/261029-test-router-30-prod-args.md
format: ID | expected | prompt (verbatim) | 5-word gloss

P1 | small | README 오타 하나 고쳐줘 | Single typo fix in README
P2 | small | parser.go의 이 함수 이름만 바꿔줘 | Rename one function in parser
P3 | small | 버전 문자열을 0.7.3으로 올려줘 | Version bump, expected small label
P4 | large | 멀티 프로세스 재연결 설계를 검토하고 트레이드오프를 정리해줘 | Reconnect design review with tradeoffs
P5 | large | 릴리즈 전에 전체 파일 동기화와 태그 생성까지 해줘 | Pre-release full sync plus tagging
P6 | large | 이 에러가 왜 나는지 모르겠어, 원인 찾아줘 | Open-ended error root cause investigation
P7 | small | config 파일 한 줄 주석 추가 | Add one-line config comment
P8 | large | 여러 파일의 로깅 방식을 통일하는 계획 세워줘 | Plan logging unification across files
P9 | small | 테스트 하나 실행하고 결과만 알려줘 | Run single test, report result
P10 | large | 두 방식 중 어느 쪽이 나은지 의견 줘 | Opinion choosing between two approaches
N1 | small | internal/harness/tui.go에서 'Recieve' 오타를 'Receive'로 바꿔줘 | Fix typo in a Go file
N2 | small | notes/DECISIONS.md 파일이 몇 줄인지 세어줘 | Count lines of notes file
N3 | small | README.md에 'TODO' 문자열이 몇 번 나오는지 알려줘 | Count string occurrences in README
N4 | small | clients.go의 기본 타임아웃 상수 값이 뭔지 알려줘 | Look up timeout constant value
N5 | small | dispatch_test.go 테스트 하나만 돌려서 결과 한 줄로 알려줘 | Run single test, one-line summary
N6 | small | Makefile의 build 타겟 위에 주석 한 줄 추가해줘 | Add comment above Makefile target
N7 | small | notes/research/billing.md 첫 번째 제목이 뭔지 찾아줘 | Find first heading of file
N8 | small | classifier.go에서 'when unsure' 문자열이 있는 줄 번호 알려줘 | Locate line numbers of string
N9 | small | children.go의 패키지 주석 한 줄을 더 짧게 고쳐줘 | Shorten a package doc comment
N10 | small | cleanenv_test.go에 있는 테스트 함수 이름 목록 뽑아줘 | Extract test function names list
L1 | large | 라우터 분류 결과를 캐시하는 기능을 새로 설계하고 파일별 변경 계획 세워줘 | Design caching feature, plan files
L2 | large | 멀티 프로세스 환경에서 세션 재연결 방식 두 가지를 비교하고 추천해줘 | Compare reconnect approaches, recommend one
L3 | large | tui, repl, core 세 파일의 에러 처리 방식을 하나로 통일하는 리팩터링 진행해줘 | Refactor error handling across files
L4 | large | 다음 릴리즈 0.8.0 준비해줘: 버전 올리기, CHANGELOG 정리, 태그 생성까지 | Full release prep through tagging
L5 | large | 테스트가 간헐적으로 실패하는데 원인이 뭔지 모르겠어, 찾아서 고쳐줘 | Find and fix flaky test
L6 | large | 비용 상한을 세션 단위와 일 단위 중 어느 쪽으로 두는 게 나은지 트레이드오프 정리해줘 | Tradeoffs of cost cap scope
L7 | large | 병렬 터미널 실행 시 로그 파일이 섞이는 문제, 해결 방향을 전략적으로 정해줘 | Strategic fix for interleaved logs
L8 | large | Haiku 워커 풀 크기를 어떻게 정할지 설계 문서 초안 쓰고 여러 파일 영향 분석해줘 | Worker pool design doc draft
L9 | large | 라우터 프롬프트를 도메인별로 나누는 게 맞을지 판단하고 근거 제시해줘 | Judge prompt splitting, justify reasoning
L10 | large | 서브에이전트 반환 포맷을 표준화하는 작업을 여러 패키지에 걸쳐 진행해줘 | Standardize return format across packages

doubts:
- P1-P10 and N/L texts are not in 261029-test-router-sonnet-vs-haiku.md (it has only task summaries) or in 261029-test-router-30-prod-args.md (ids and expected sizes only).
- P3 expected "small" conflicts with classifyPrompt rule ("version bump" listed under large), per 261029-test-router-30-prod-args.md.
- gloss for 'Version bump, expected small label' is written to name the label mismatch; it is the only non-literal gloss.
- bin/chaude search-history was not run: bin/chaude is not present at the repo path.
