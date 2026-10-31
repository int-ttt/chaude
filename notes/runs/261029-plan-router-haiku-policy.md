# plan: router on Haiku, Haiku-tuned prompt (rev2, review merged)
date: 2026-10-29
status: plan only (no source/CLAUDE.md/DECISIONS.md edits yet)
evidence_src: notes/runs/261029-test-router-3tier-30.md (3-tier, 30 prompts x 3 repeats)
review_src: notes/runs/261029-plan-router-haiku-policy-review.md

## 목표
- 분류기 호출(프롬프트당 1회) 비용 절감. 30샘플 비용 Sonnet $0.134 vs Haiku(90콜) $0.028 -> 콜당 약 $0.0045 vs $0.0003 (약 15배).
- 목표치: 콜당 비용 90% 이상 절감, 아래 통과 기준 1-4 충족.
- 관리 용이성: 모델/프롬프트 선택을 설정 1개 + 상수 2개로 모은다.

## 현재 상태
- classifier.go:14 `conductorModel = "sonnet"` 상수, :141 `--model conductorModel`. conductor_test.go:112 가 "sonnet" 고정 검사.
- CHAUDE_ROUTER_MODEL 은 internal/harness 어디에도 없음(grep 확인, 리뷰 4). 현재 롤백 = 코드 되돌리기.
- 프롬프트 classifier.go:15-24 `classifyPrompt` 약 1,990바이트(토큰 추정 480-520, 미측정).
- 파서 classifier.go:75-132, 5줄(topic/tier/kind/worker/task). tier 누락/무효는 sonnet. 파서 변경 없음.
- 3-tier 결과(30샘플): Sonnet 30/30; Haiku 87/90, 일관성 27/30, haiku 누수 0건.
- CLAUDE.md:32 "the classifier is always Sonnet and only tags size"(낡음); DECISIONS.md:32 [conductor] 줄이 sonnet classifier 명시.

## 변경 1: 모델 설정 (먼저 구현, 기본값은 sonnet 유지) [결정 4]
- `routerModel()` 1개: `CHAUDE_ROUTER_MODEL` 을 os.Getenv 로 읽음(chaude 프로세스에서; cleanEnv는 자식에만 영향), trim+lowercase, 허용값 haiku|sonnet.
  - 빈 값/무효 값 -> `routerModelDefault`(하드코딩 "haiku" 아님) + stderr 경고 1회. 오타("sonet")가 롤백 중 조용히 haiku로 남는 것을 막는다.
  - 실행 중인 chaude 는 재시작해야 반영됨(문서화).
- 프롬프트 상수 2개: 현행 문안 그대로 `classifyPromptSonnet`, 새 `classifyPromptHaiku`. 해석된 모델로 선택. 그래야 `=sonnet` 롤백이 검증된 기존 동작을 그대로 복원.
- 메트릭 stage=classify 에 `router_model`, `router_prompt`(예: "haiku/v2", "sonnet/v1") 필드 추가.
- 테스트(conductor_test.go:112 교체): unset -> 기본값; "sonnet"/"SONNET " -> sonnet; "haiku" -> haiku; "opus"/"x" -> 기본값; 선택된 프롬프트가 모델과 일치.
- 기본값을 haiku로 바꾸는 것은 통과 후 `routerModelDefault` 한 줄 변경. 이후 롤백은 설정만.
- 구현: haiku-coder 1개 인스턴스, 끝나면 `go test ./internal/harness`.

## 변경 2: Haiku용 프롬프트 [결정 3]
순서: (a) 개발셋=기존 30프롬프트(notes/runs/261029-haiku-router-30-samples-list.md)로만 튜닝, 초안마다 Haiku 3회 -> (b) 동결, sha256을 평가셋 파일 머리말에 기록, 이후 수정 금지 -> (c) 별도 에이전트가 평가셋 작성(변경 3 검증 참조).
유지 조건:
- 출력: 정확히 5줄 topic/tier/kind/worker/task, 고정 어휘, "nothing else". "Always output topic, tier, kind and worker." 유지. [결정 5: 형식 유지]
- 현행 티어 규칙을 하나도 빼지 않는다. 초안에서 빠졌던 것 복원: "version bump with exact text"(haiku), "one-step edit of one file with a clear scope"(haiku), "clear multi-step task"/"comparison summary"(sonnet), "new feature or refactor with a given scope"->sonnet vs "design is open"->opus, "repeated failure", "'opus로'". 옛 문구 -> 새 문구 대응표를 평가셋 파일에 첨부.
- 어떤 규칙에도 안 맞으면 sonnet(절대 haiku 아님). haiku는 명시적 haiku 규칙 일치일 때만. 초안의 "first match opus->sonnet->haiku"는 무일치가 정의되지 않으므로 폐기.
- 동률 규칙은 현행 의미 그대로: "unsure haiku/sonnet -> sonnet; unsure sonnet/opus -> sonnet". 초안의 "항상 상위"는 채택하지 않음(opus 과분류 증가, 변수 혼입).
- topic: "When unsure, same." + 기존 new/same 정의. kind/worker: "A lookup or edit that is tier haiku is sent straight to a Haiku session", "worker only for kind edit, else new" 유지, "session.worker:" 빠진 공백만 수정.
- task 접미 'reply: Korean 존댓말, ... before replying' 은 바이트 단위 보존.
- 예시: 티어당 1개, 한국어, 전형적 사례, 개발셋/평가셋에 없는 새 문장. (초안의 예시 3개는 30샘플셋 출처라 사용 금지)
- 규칙은 영문, 한국어 트리거 단어는 괄호로 포함(설계, 트레이드오프, 추천, 전략, 원인, 계획, 'opus로').
- 길이 -30% 는 참고 지표(게이트 아님). 길이 때문에 규칙을 자르지 않는다.
- 구현 단위: `classifyPromptHaiku` 상수 작성 1건(haiku-coder, 문안은 튜닝 후 확정).

## 변경 3: 정책 문서 (검증 통과 후에만)
CLAUDE.md 32행만 수정(38행 불변):
- 전: "the classifier is always Sonnet and only tags size"
- 후: "the classifier is Haiku by default (env CHAUDE_ROUTER_MODEL=sonnet falls back) and tags tier haiku|sonnet|opus"
- `[routing: large ...]` 표현 정리는 별건(열린 질문 1).
DECISIONS.md 맨 아래 새 줄 1개(현재 42행, 기존 줄 불변):
- `- [router] 2026-10-29 user: <승인 발언 원문> -> prompt router runs on Haiku (default; CHAUDE_ROUTER_MODEL=sonnet fallback) with a Haiku-tuned prompt; adopted only if the 150-prompt gates pass (0 distinct non-haiku prompts sent to haiku, per-tier error limits, 3-run consistency >=90%); else stay on sonnet. Supersedes the "--model sonnet classifier" part of the 261029 [conductor] line.`
- 승인 발언 원문: 현재 확보 안 됨(아래 승인 확인 참조). 메인이 사용자 발언을 받아 채운다. 지어내지 않음.

## 승인 확인
- notes/pastes/542019b6.md 는 서브에이전트(a183b109e7f461899) 인계 보고서이며, 하네스 표기상 사용자 메시지가 아니고 승인 권한이 없음. 사용자 승인 문장 없음 -> 인용할 원문 없음. 따라서 DECISIONS.md 항목은 사용자 승인 발언을 받기 전까지 작성하지 않는다.

## 검증 [결정 1, 2]
평가셋 150프롬프트: haiku 50 / sonnet 50 / opus 50 (티어당 최소 30, 50이면 1건 오류=2%p).
- 기존 30샘플과 겹침 0(그쪽은 개발셋). "50 고정 + 50 보조" 구분 폐기: 전부 라벨링·동결·채점.
- 작성자: 별도 haiku-drafter 새 인스턴스. 티어 정의만 보고 프롬프트 문안은 보지 않음(키워드 맞춤형 쉬운 문제 방지). 프롬프트 동결 후에 작성.
- 라벨은 어떤 모델 호출 전에 확정·저장(`notes/runs/261029-test-router-haiku-150.md`), 호출 후 변경 금지. 선택: 사용자 라벨 확인(열린 질문 2).
- 경계 사례(haiku/sonnet 사이)는 라벨 sonnet, 사전 표시, 세트의 10% 이하. 게이트 1에 포함.
- 비-haiku 프롬프트 100개 이상(sonnet 50 + opus 50 = 100).
실행 3개 팔, 운영 인자 그대로, cwd 임시 디렉터리, env -u ANTHROPIC_API_KEY, 누적 비용 $3 초과 시 중단:
- (a) Sonnet + 현행 프롬프트 x1 (기준선)
- (b) Haiku + 새 프롬프트 x3
- (c) Haiku + 현행 프롬프트 x3 (모델 효과와 프롬프트 효과 분리, 검증된 대체안)
- 총 150 + 450 + 450 = 1,050콜, 비용 약 $1.0 (Sonnet $0.68 + Haiku $0.14 x 2).
- 러너 드리프트 방지: /tmp/router3t/run.py 는 claude 직접 호출이므로, 빌드된 코드의 프롬프트를 덤프하는 테스트 헬퍼(env 플래그)로 파일에 쓰고 동결 sha256과 대조. 프롬프트 수동 복사 금지.
통과 기준 (b 기준, 전부 충족):
1. 누수: 비-haiku 기대인데 haiku로 보낸 서로 다른 프롬프트 수 = 0. 3회 중 한 번이라도 새면 그 프롬프트는 누수 1건. 하드 게이트. 참고: Sonnet 기준선 누수도 보고(Sonnet도 새면 라벨/규칙 문제).
2. 정확도(같은 프롬프트 쌍 비교, 3회 평균 오류 수):
   a. 전체: Haiku 오류 <= Sonnet 오류 + 3건(150의 2%p).
   b. 티어별: Haiku 오류 <= Sonnet 해당 티어 오류 + 2건.
   c. opus->sonnet 과소분류 <= Sonnet 기준선 + 2건.
3. 일관성: 150프롬프트 중 3회 모두 같은 티어 >= 90%.
4. 총 비용 <= $3; 파싱 실패 0건(재시도 후); tier 누락 0건; 파싱된 haiku 실행 task의 접미가 바이트 단위 100% 일치(실패 시 열린 질문 3의 코드측 접미).
참고 지표: 과대 분류 비율, 프롬프트 길이, 콜당 비용, Sonnet 기준선 변동.
(b)가 1-3에서 (c)보다 낫지 않고 (c)가 통과하면 Haiku + 현행 프롬프트로 출시.
재튜닝 금지: 실패 시 롤백. 이후 재튜닝은 새 평가셋(최소 실패 티어 50개 교체) 필요.

## 롤백
- 트리거: 기준 1-4 중 하나라도 실패.
- 조치: 기본값 sonnet 유지, 문서 미반영, 실패는 LESSONS.md 기록. 도입 후 롤백은 `CHAUDE_ROUTER_MODEL=sonnet`(설정만, 현행 프롬프트 복원, chaude 재시작).
- 운영 감시: router_model/router_prompt 메트릭으로 비용·정확도 분리. 정정/재요청 비율 지표는 미확인.

## 가정 (검증 방법)
1. 새 프롬프트에서도 누수 0 -> 기준 1 (30샘플 0/90은 구 프롬프트 기준).
2. 프롬프트 토큰 감소 -> usage input_tokens 측정(추정치 미측정).
3. 15배 비용 차이 유지 -> 러너 total_cost_usd 합.
4. `claude -p --model haiku` 운영 환경 동작 -> 3-tier 테스트 90/90로 확인, 재확인.
5. 파서 불변 -> 파서 단위 테스트 + 파싱 실패 0건.
6. 라벨이 사용자 의도와 일치 -> 경계 사례 사전 표시, 사용자 확인 요청.

## 순서 (단위)
1. 변경 1 구현(env + 프롬프트 상수 2개 + 메트릭 + 테스트, 기본 sonnet) -> go test ./internal/harness.
2. 개발셋 30개로 Haiku 프롬프트 튜닝 -> 동결(sha256).
3. 별도 에이전트가 150프롬프트 평가셋 작성·동결(50/50/50, 경계 <=10%) -> 선택적 사용자 라벨 확인.
4. 실행: (a) Sonnet+현행 x1, (b) Haiku+새 x3, (c) Haiku+현행 x3, 상한 $3.
5. 기준 1-4 판정 -> 통과: routerModelDefault="haiku"(+ 프롬프트 선택) + 변경 3 문서(사용자 승인 발언 확보 후); 실패: sonnet 유지, LESSONS.md.
위임: 1,2 haiku-coder; 3 haiku-drafter(새 인스턴스, 프롬프트 문안 비공개); 4 haiku-runner.

## 열린 질문
1. CLAUDE.md:32 의 `[routing: large ...]` 표현 정리는 별도 작업으로 둘지.
2. 평가셋 라벨을 사용자가 사전 확인할지(호출 전).
3. 후속 후보(지금 안 함): task 끝 문구('reply: Korean 존댓말...')를 코드가 붙이고 프롬프트는 간결한 task만 요청. 기준 4의 접미 검사 실패 또는 도입 후 출력 토큰이 지배적일 때, 5줄 형식 재검토는 1-2주 router_model 메트릭 이후.
4. DECISIONS.md 새 줄에 넣을 사용자 승인 발언 원문(미확보, 메인이 제공).

## 반영 기록
- 결정 1: 누수 0건을 하드 게이트로 유지, 서로 다른 프롬프트 기준 집계, 비-haiku 100개 이상, 경계 라벨 sonnet(10% 이하), Sonnet 누수는 참고 지표. 반영 위치: 검증.
- 결정 2: 평가셋 150(50/50/50, 티어당 최소 30), 전체 +3건 / 티어별 +2건 / opus->sonnet +2건 한도, 비용 약 $1.0으로 갱신, 50+50 구분 폐기. 반영 위치: 검증.
- 결정 3: 30샘플 개발셋 튜닝 -> 동결 -> 별도 에이전트 평가셋, 현행 규칙 전부 유지, 무일치=sonnet, 동률은 현행 의미(초안의 "항상 상위" 폐기), (c) Haiku+현행 팔 추가, 러너 sha256 대조. 반영 위치: 변경 2, 검증, 순서.
- 결정 4: CHAUDE_ROUTER_MODEL 먼저 구현(기본값·경고·프롬프트 상수 2개·메트릭 필드·테스트), 롤백은 설정만. 반영 위치: 변경 1, 롤백, 순서.
- 결정 5: 5줄 형식 유지, task 끝 문구 코드측 부착은 후속 후보. 반영 위치: 변경 2, 열린 질문 3.
- 기타: 길이 -30% 목표는 게이트에서 참고 지표로 강등(리뷰 3). 이전 열린 질문 1(한 줄+태그)은 결정 5로 닫음.
- 승인 발언: notes/pastes/542019b6.md 에 없음(서브에이전트 보고서). DECISIONS 줄의 원문 칸은 비워 둠.
