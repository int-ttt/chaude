# 검토: 261031-plan-interactive-child.md

verdict: 방향은 맞으나 측정 없이 B를 고려하기는 이릅니다. 측정을 먼저 하고, 그 결과로 C 유지 또는 B 시제품을 결정하도록 계획을 보완해야 합니다.

## 1. 누락된 점
- 원인 가설이 없습니다. "-p가 더 쓴다"는 체감에는 모드 자체 외에 다음 원인이 섞여 있을 수 있습니다.
  - H1 하네스 부가 호출: 프롬프트마다 `claude -p --model sonnet` 분류기 (DECISIONS.md:16, classifier.go), `claude -p /usage` 폴링 (DECISIONS.md:19,45, usagepoll.go). 이 one-shot 호출은 매번 캐시가 비어 있는 상태로 시작하므로 시스템 프롬프트 전체가 cache write로 잡힙니다.
  - H2 캐시 TTL: Claude Code는 구독 한도 안에서 "main conversation"에만 1h TTL을 요청합니다 (prompt-cache-ttl-clear.md:25). -p 자식이 5m으로 잡히면 5분 넘게 쉬었다가 다음 턴에서 매번 재작성이 일어납니다. 반대로 1h write는 2배 단가입니다 (같은 파일:13-14).
  - H3 의도된 지출: 59:59 유휴 회전과 캐시 갱신 (DECISIONS.md:9), 모델별 상시 자식 프로세스 여러 개 (0.8.0). 각 자식이 자기 캐시 접두부를 따로 유지합니다.
  - H4 과금 경로: -p에서는 ANTHROPIC_API_KEY가 있으면 항상 그 키를 씁니다 (billing.md:10). 자식 env에 키가 상속되면 구독이 아니라 API로 과금됩니다.
- 판정 기준(임계값)이 없습니다. 몇 % 차이부터 B를 할지 정해야 합니다.
- 수용 기준이 "목록이 들어 있을 것"뿐입니다. 측정 결과 표와 결정 규칙이 수용 기준이어야 합니다.
- 순서: 미확인 2~9번은 B를 할 때만 필요합니다. 1번과 8번(측정)을 먼저 하고, B로 갈 때만 나머지를 확인하도록 순서를 나눠야 합니다.

## 2. 측정 방법 (돈 들지 않는 것부터)
- M0 (지출 0): notes/metrics/*.jsonl을 stage별(classify, usagepoll, main, rotate)로 합산합니다. 토큰 합계와 total_cost_usd 추정치를 단계별 비율로 표에 적습니다. H1, H3 비중이 바로 나옵니다.
- M1 (지출 0): 자식 프로세스 env에 ANTHROPIC_API_KEY가 있는지 확인합니다 (core.go 실행부, `ps eww <pid>`). 있으면 H4가 확정됩니다.
- M2 (TTL 확인): 기존 -p 기록과 ~/.claude/projects/<slug>/*.jsonl 인터랙티브 기록에서 usage.cache_creation의 ephemeral_5m_input_tokens와 ephemeral_1h_input_tokens를 비교합니다. -p 쪽이 5m이면 H2가 확정됩니다. (필드 이름은 미확인이므로 실제 레코드에서 확인합니다.)
- M3 (A/B, 소량 지출): 다른 세션이 없는 시간대에 같은 모델, 같은 cwd, 같은 고정 프롬프트 5턴 스크립트로 (a) 인터랙티브 `claude`와 (b) `claude -p --input-format stream-json`을 실행합니다. 각 3회, 턴 사이 간격은 0초와 6분 두 조건으로 합니다. 분류기와 usagepoll은 끈 상태로 합니다. 기록 항목은 턴별 input, cache_creation(5m/1h), cache_read, output 토큰과, 실행 전후 /usage 5시간 %입니다. %는 정수 단위로 거칠기 때문에 토큰 가중합(FACTS.md 단가)을 주 지표로, % 변화는 보조 지표로 씁니다.
- 판정: 토큰이 같은데 %만 다르면 모드 자체의 차이(서버 측)입니다. 토큰이 다르면 TTL이나 프롬프트 차이로 보고 C에서 고칩니다.

## 3. 선택지
- A PTY 화면 스크래핑: 제외에 동의합니다. 구조화 신호(session_id, usage, can_use_tool)를 잃습니다.
- B PTY 입력 + 트랜스크립트 tail + 훅: M3에서 동일 토큰 조건으로 5시간 % 차이가 15% 이상, 3회 모두 같은 방향일 때만 시제품을 만듭니다. 그 경우에도 하나의 모델 자식에만 플래그(기본 끔)로 넣고, stream-json 경로는 남겨 둡니다. 실시간 스트리밍, 권한 대화(can_use_tool), cost 표시가 줄어든다는 점을 사용자에게 먼저 확인받아야 합니다.
- C 현행 유지 + 보완 (권장): M0~M2 결과에 따라 다음을 적용합니다.
  - 자식 env에서 ANTHROPIC_API_KEY 제거 (H4)
  - 자식에 1h TTL 강제 설정. 설정 키나 env 이름은 code.claude.com prompt-caching 문서에서 확인합니다 (prompt-cache-ttl-clear.md:26) (H2)
  - 분류기 호출 줄이기: 로컬 규칙 우선, 애매할 때만 haiku 사용. usagepoll 주기 늘리기 (H1)
  - 유휴 회전 시 캐시 갱신 비용을 metrics에 따로 기록 (H3)
- 권장: M0 -> M1 -> M2 -> C 보완 -> M3 순서입니다. B는 M3 기준을 넘을 때만 진행합니다.

## 4. 미확인 목록 (보완)
1. 구독 OAuth에서 -p 사용량이 구독에 포함되는지 (billing.md:18, 공식 문장 없음)
2. -p 자식의 캐시 TTL이 5m인지 1h인지 (M2)
3. 자식 env의 ANTHROPIC_API_KEY 상속 여부 (M1)
4. 5시간 한도가 동시 세션 사이에 공유되는지 (billing.md:12). 공유된다면 M3는 반드시 단독으로 실행해야 합니다.
5. usage.cache_creation 세부 필드 이름과 트랜스크립트 JSONL 기록 여부
6. -p와 인터랙티브의 시스템 프롬프트, 도구 정의 크기 차이 (첫 턴 cache_creation으로 비교)
7. 기존 계획 2~9번 (B를 선택할 때만 필요)

## 5. 바꾸면 안 되는 것
- stream-json 기반 권한 처리, task-notification 감지, metrics 기록 형식
- 5시간 상한 60% 정책 (DECISIONS.md:45). 측정만을 위해 상한까지 지출하지 않습니다.
