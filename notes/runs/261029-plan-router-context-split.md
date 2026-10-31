# 계획: 라우터 컨텍스트 분리 (Haiku 라우터에 최소 컨텍스트)
date: 2026-10-29
status: 계획만. 소스 수정 없음.

## 목표
- 라우터 호출이 받는 텍스트를 고정 프롬프트 + 사용자 프롬프트 + 짧은 세션 메모로 한정한다.
- CLAUDE.md(8005자), 시스템 규칙(약 885자), 훅 주입이 라우터에 닿지 않음을 코드와 실측으로 보장한다.
- 이 상태에서 Haiku 라우터가 Sonnet 수준 정확도를 내는지 확인한다.

## 현재 흐름
근거(evidence):
- classifier.go:14-24 classifyPrompt가 라우터 고정 프롬프트. classifier.go:122-123 에서 `--system-prompt classifyPrompt`, `--tools ""`, `--no-session-persistence`로 전달.
- classifier.go:117-121 user 메시지 = 현재 모델 + 직전 프롬프트(200자) + 워커 후보 + 사용자 프롬프트(= "짧은 세션 메모" 역할).
- classifier.go:125 cmd.Dir = os.TempDir() → 프로젝트 CLAUDE.md 미로드 의도.
- classifier.go:14 conductorModel = "sonnet". 현재 운영 라우터는 Sonnet.
- 시스템 규칙 append(repl.go:184-191)는 메인 세션 args에만 있고 classify()에는 없음.
- 주입 인벤토리(261029-explore-prompt-injection-inventory.md): CLAUDE.md, 시스템 규칙, 훅은 메인 호출 기준 기록.
- 테스트(261029-test-router-injection-minimal-vs-full.md): F 조건 = CLAUDE.md를 user 메시지에 직접 붙임(Haiku 3/10, 파싱 실패 6). M 조건 8/10, Sonnet M 10/10, F 9/10.
추론(inference, 미검증):
- 테스트 F 조건은 운영 classify()와 같지 않다. 운영 코드는 이미 CLAUDE.md를 직접 붙이지 않는다. 따라서 "실제 흐름에서 Haiku가 풀 컨텍스트를 받는다"는 점은 코드상 확인되지 않았다.
- 남는 누출 후보: (a) 사용자 전역 설정(~/.claude/settings.json)의 훅·MCP·플러그인이 -p 호출에도 로드, (b) Claude Code 기본 도구/컨텍스트 오버헤드(테스트 M에서도 호출당 cache_creation 약 12k 토큰), (c) TempDir 안에 CLAUDE.md가 생길 가능성.
- 테스트 M은 라우터 프롬프트를 user 메시지로 줬고 운영은 system prompt로 준다. 둘은 동일 조건이 아니다.

## 분리 설계
라우터 호출 규약(한 곳에서만 정의):
- system: classifyPrompt 그대로(`--system-prompt`로 기본 시스템 프롬프트 대체).
- user: 현재 모델, 직전 프롬프트 200자, 후보, 사용자 프롬프트. 추가 텍스트 없음.
- 격리 플래그: `--tools ""`, `--strict-mcp-config`, `--setting-sources ""`(훅·사용자 설정 차단), `--disable-slash-commands`, `--max-turns 1`, `--no-session-persistence`.
- cwd: 호출마다 빈 임시 디렉터리(os.MkdirTemp), 종료 후 삭제.
- 모델: 상수 대신 설정값(기본 현 정책 유지 = sonnet). Haiku 전환은 검증 통과 후 결정.
- 보호장치: 호출 직전 입력 길이 상한 검사(고정 프롬프트 + 입력이 예상 범위를 넘으면 로그).

## 변경 파일과 함수 (유닛당 파일 1개, 함수 1개)
1. U1 internal/harness/classifier.go · classify(): 격리 플래그 추가, cwd를 빈 임시 디렉터리로 교체.
2. U2 internal/harness/classifier.go · routerModel() 신설: conductorModel 상수 대신 환경/설정에서 읽고 기본값 sonnet.
3. U3 internal/harness/classifier_test.go(신규) · TestClassifyArgs: 인자 구성(플래그, system prompt 사용, CLAUDE.md 문자열 미포함)을 실행 없이 검사. 이를 위해 인자 생성부를 buildRouterArgs()로 분리하는 것은 U1에 포함.
4. U4 scripts/router-eval.sh(신규) · 30샘플 비교 드라이버. 운영 인자 구성과 동일하게 호출.
- 의존: U1 → U3, U4. U2는 독립. 5개 미만 파일.

## 검증
30샘플 테스트(호출 전에 고정):
- 프롬프트 30개와 기대 size(small/large), kind를 호출 전에 notes/runs/261029-router-eval-set.md에 확정·기록. 이전 10개(P1-P10) 포함, 20개 신규. small/large 15:15.
- 기대값은 사용자 판단 기준. 이전 "version bump" 같은 애매 항목은 분류기 규칙(large) 기준으로 확정 후 기록, 이후 수정 금지.
- 모델 2종(Sonnet, Haiku)을 모두 분리 컨텍스트(U1 인자)로 각 30회 호출, 총 60회. 반복 없음(분산 미측정을 결과에 명시).
- 예상 비용: 이전 평균 Sonnet 약 $0.05, Haiku 약 $0.003/호출 → 약 $1.6. 상한 $2.50, 누적이 넘으면 중단.
- 측정: size 정확도, kind 정확도, 파싱 실패 수, 호출당 비용, usage(cache_creation).
- 누출 점검: 호출 1건의 입력 토큰 수를 고정 프롬프트 + 입력 추정치와 비교, 훅 실행 로그 없음 확인.

## 가정
- A1 운영 classify()는 이미 CLAUDE.md를 받지 않는다. 확인: 임시 디렉터리 cwd로 호출 후 응답/usage에서 CLAUDE.md 문구 유무, 또는 입력 토큰 비교.
- A2 사용자 전역 설정이 -p 호출에 훅·MCP를 싣는다. 확인: `--setting-sources ""` 유무로 입력 토큰 차이 측정.
- A3 `--setting-sources`, `--disable-slash-commands`가 현 claude 2.1.312에 존재한다. 확인: `claude --help`.
- A4 system prompt 방식(운영)이 user 메시지 방식(테스트 M)보다 Haiku 정확도에 불리하지 않다. 확인: 30샘플에서 운영 방식으로 측정(테스트 M 8/10과 비교).
- A5 Haiku 실패 원인은 CLAUDE.md이고 모델 한계가 아니다. 확인: 분리 컨텍스트 Haiku 파싱 실패 0이면 지지.
- A6 `--max-turns 1`이 분리 컨텍스트에서 정상 응답을 막지 않는다. 확인: 파싱 실패 수.

## 롤백 조건
- Haiku 분리 정확도가 Sonnet 분리 정확도보다 30개 중 2개 이상 낮으면 라우터는 Sonnet 유지(U2 기본값 그대로, 변경 없음).
- 파싱 실패가 Haiku에서 1건이라도 있으면 보고하고 해당 건을 오답 처리.
- U1 격리 플래그가 Sonnet 정확도를 기존(10샘플 기준 9-10/10 수준)보다 떨어뜨리면 U1만 되돌림(git revert 단위: classifier.go).

## 열린 질문 (사용자 결정)
- 라우터 모델 정책을 바꿀지 여부: 검증 후 Haiku가 기준을 충족해도 비용 절감(호출당 약 $0.05 → $0.003)을 취할지, Sonnet을 유지할지.
- 30샘플 중 신규 20개의 출처: 실제 사용 로그(notes/metrics)에서 뽑을지 새로 작성할지.
- 격리 플래그가 사용자 전역 훅을 막아도 괜찮은지(라우터에 한정되므로 영향 없다고 추론, 확인 필요).
