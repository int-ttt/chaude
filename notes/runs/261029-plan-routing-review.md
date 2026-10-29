# chaude 라우팅 구조 검토 (읽기 전용, 코드 수정 없음)

## Q1. "Haiku가 분류·처리하고, 어려울 때만 Opus/Sonnet 호출" 구조가 맞는가
결론: 처리 구조는 맞지만 분류는 Haiku가 아니라 Sonnet이 합니다. 문서와 구현이 일부 어긋납니다.

근거(사실)
- 분류기는 매 프롬프트마다 별도 `claude -p --model sonnet` 일회성 호출입니다 (internal/harness/classifier.go:14 conductorModel="sonnet", :109-:115 classify). 도구 없음, 세션 없음, cwd=tmpdir (classifier.go:115-117).
- 분류 결과는 topic/size/kind/worker/task 5줄 태그뿐입니다 (classifier.go:14-15). 답변은 생성하지 않습니다.
- 메인 세션은 Haiku 고정입니다. pickModel은 cur가 ""/haiku면 항상 haiku를 반환하고 (classifier.go:38-41), pickTarget도 kind와 무관하게 Haiku입니다 (classifier.go:49-51). 강제 --model sonnet|opus만 유지됩니다.
- size=large면 planHint가 붙습니다 (classifier.go:92-95 (분기 :94)). Haiku가 계획 작성 -> opus-architect/sonnet-analyst가 개선안만 작성 -> Haiku가 병합·실행, Opus/Sonnet은 실행하지 않습니다 (classifier.go:31-34). 즉 "Haiku가 어렵다고 판단해서 호출"이 아니라 "Sonnet 분류기가 large로 태깅 -> 힌트 -> Haiku가 호출"입니다.
- 문서도 같은 내용입니다: CLAUDE.md:31 (Haiku-fixed main, classifier always Sonnet and only tags size), DECISIONS.md:32 (Sonnet 일회성 분류기), :38 (pickModel/pickTarget이 haiku 반환, planHint).

문서 vs 구현 차이
- CLAUDE.md:2 헤더와 :9의 "Opus main" 라우팅 문단은 강제 Opus main일 때만 적용되는 구 규칙입니다 (CLAUDE.md:31 마지막 문장이 명시). 읽는 사람이 혼동하기 쉽습니다.
- DECISIONS.md:30-33은 "Sonnet이 답/위임 대상을 정한다 (target=haiku|sonnet|opus)"는 이전 설계를 기록하고, :38이 이를 덮어씁니다. 현재 구현에서 분류기의 model 출력은 사실상 무의미(항상 haiku)합니다. 의견: 분류기 프롬프트의 kind/worker 중 worker만 의미가 남아 프롬프트가 필요 이상으로 깁니다.
- 분류 단계가 Sonnet이라 "Haiku가 판단"이라는 직관과 다릅니다. 매 프롬프트 Sonnet 비용이 별도 발생합니다 (metrics stage=classify, DECISIONS.md:32).
- 어려움 판단의 주체: 구현상 Sonnet 분류기 1회. Haiku는 힌트가 있을 때만 호출 순서를 따릅니다. Haiku 자체가 난이도를 재판단해 에스컬레이션하는 코드는 확인하지 못했습니다 (확인 범위: classifier.go 전체, CLAUDE.md, DECISIONS.md grep).

## Q2. Opus/Sonnet을 서브에이전트가 아닌 메인 세션으로 돌리는 편이 토큰 효율적인가
결론(의견, 정량 측정 없음): 이 프로젝트의 기본 사용 패턴에서는 서브에이전트가 유리하고, 같은 파일·같은 작업줄기를 여러 턴 이어갈 때만 메인 세션이 유리합니다.

비용 구조(일반 원리, 이 저장소 수치로 검증하지 않음)
- 서브에이전트: 호출당 새 컨텍스트. 비용 = 브리프 + 에이전트 자체 탐색/도구 출력 + 결과 요약. 메인 히스토리를 다시 읽지 않습니다. 단점: 호출마다 시스템 프롬프트·CLAUDE.md를 새로 읽고(캐시 쓰기), 같은 파일을 반복 호출 시 재탐색합니다.
- 메인 세션: 전체 히스토리를 매 턴 다시 읽습니다. 캐시가 살아 있으면 읽기 단가(싸다)지만 컨텍스트가 커질수록 턴당 비용이 선형 증가하고, TTL이 지나면 전체가 쓰기 단가로 재청구됩니다. Opus/Sonnet 단가가 Haiku보다 높아 이 효과가 커집니다.

이 프로젝트 조건(사실)
- 이미 컨텍스트 증가를 제어하는 장치가 많습니다: 270k 이상 회전 (DECISIONS.md:16), 유휴 회전 (:19-:23), 자동 compact 150k (CLAUDE.md:26). 이는 메인 세션 컨텍스트 증가 비용이 실제 관심사임을 보여줍니다.
- 연구(scout/researcher)는 서브에이전트 유지로 결정됐고, 코드 수정·분석·집필·배치·runner는 "세션 워커" 후보로 비교 실험 중입니다 (DECISIONS.md:25, :27). 아직 측정 결과는 없습니다 (미검증).

각 방식이 이기는 조건 (의견)
- 서브에이전트 승: 일회성·독립 작업, 탐색/검색이 큰 출력(결과 요약만 필요), 병렬 작업, 메인이 Haiku라 히스토리가 싼 경우, 캐시 TTL이 자주 끊기는 간헐적 사용.
- 메인 Opus/Sonnet 승: 같은 파일·결정 맥락을 연속 5-10턴 이상 이어가는 설계/디버깅 대화, 브리프로 압축하기 어려운 맥락(미묘한 사용자 정정 이력), 캐시가 살아 있어 읽기 단가가 적용되는 연속 사용. 브리프가 히스토리만큼 길어지면 서브에이전트 이점이 사라집니다.
- 혼합 지점: Opus 메인에서 서브에이전트 호출 시 Opus가 히스토리 재독 + 서브에이전트 비용을 이중 부담 (CLAUDE.md:9 "Opus delegating to Opus saves nothing"이 같은 취지).

## 권고
현행 유지(Haiku 메인 + Sonnet 분류 + Opus/Sonnet 서브에이전트 설계 검토)를 기본으로, 연속 작업줄기에 한해 세션 워커를 DECISIONS.md:27 실험으로 측정한 뒤 결정합니다. 추가로 metrics의 classify 비용과 에이전트별 토큰을 근거로 삼으세요.
주요 위험: (1) 매 프롬프트 Sonnet 분류 오버헤드가 소규모 프롬프트에서 Haiku 처리비보다 클 수 있음, (2) Sonnet이 size를 잘못 태깅하면 Haiku가 어려운 작업을 직접 수행해 재작업 비용 발생, (3) 위 비용 비교는 측정치 없는 추정입니다.
미해결 질문: classify 호출의 평균 비용 대비 Haiku 처리비 비율 (notes/metrics/*.jsonl에서 stage=classify 집계 필요, 이번에 집계하지 않음).
