# plan: router tier 판단 기준 조정안 (sonnet 사용 최소화, 짧은 컨텍스트 → haiku)
date: 2026-10-31
source: /Users/leekwangseok/PycharmProjects/claude-harness/CLAUDE.md (읽기 전용, 수정 없음)
status: draft (검토 전, 실행 없음)

## 1) 배경
- user 관찰: 짧은 컨텍스트에서 사용량이 예상보다 적게 드는 것으로 보임.
- 목표: sonnet 사용을 줄이고, 짧은 컨텍스트 작업은 haiku로 처리하도록 tier 판단 기준 조정안을 검토.
- 이 문서는 조정안 검토용이며, CLAUDE.md 및 다른 파일은 변경하지 않음.

## 2) 현재 기준 인용 (CLAUDE.md)
- Routing (line 9): "Routing: Sonnet = 검토·분석 (`sonnet-reviewer` review, `sonnet-analyst` analysis); Haiku = simple lookup/questions - "이거 뭐였는지 찾아" style search/query prompts → `haiku-scout` (effort high; `haiku-researcher` for web). Bounded/mechanical → a Haiku agent. ... multi-file edits that need deciding what to change → `sonnet-analyst`; ... Anything needing judgment (synthesis, design, multi-file code, long answers) → `sonnet-analyst`; ..."
- Chaude conductor (line 33): "Chaude conductor (Haiku-fixed main): ... the classifier is always Sonnet and only tags size. ... Large/judgment/design task (arrives with a `[routing: large ...]` hint) → workflow: ... Small tasks: Haiku does them via Haiku agents directly."
- 요약: 기준은 작업 유형 중심(검토·분석·판단 = Sonnet, 조회·기계적 = Haiku). 컨텍스트 길이 기준은 명시되어 있지 않음. classifier(Sonnet)는 size tag만 판정.

## 3) 쟁점
- 컨텍스트 임계값: 짧은 컨텍스트의 정의(토큰 수 또는 파일 수)가 없음. 임계값을 두면 기준 수치와 측정 방법을 정해야 함.
- 작업 유형 기준: "Anything needing judgment ... → sonnet-analyst"가 넓어서, 짧은 판단도 Sonnet으로 갈 수 있음. 판단의 깊이와 길이를 어떻게 나눌지 불명확.
- classifier 역할: classifier가 Sonnet으로 고정되어 size tag만 판정함. 짧은 작업을 classifier 단계에서 Haiku로 확정할지, classifier를 Haiku로 낮출지 결정 필요. classifier 호출 자체가 사용량을 늘릴 수 있음.
- 품질 리스크: haiku 처리 범위를 넓히면 검토(`sonnet-reviewer`) 누락 위험. 제출·출력 전 review 규칙과 충돌하지 않는지 확인 필요.

## 4) 후보 방안
- 방안 A (임계값 추가): 짧은 컨텍스트(예: 입력 N 토큰 이하, 파일 1개 이하) + 조회/단순 판단이면 haiku-scout 또는 haiku-coder로 처리. 길거나 다중 파일이면 기존 Sonnet 경로 유지. 장점: 규칙이 명확함. 단점: N 값 근거가 필요.
- 방안 B (작업 유형 세분화): Sonnet을 "깊은 분석/설계"로 한정하고, "짧은 검토·요약·단일 파일 판단"은 haiku-scout 요약 후 haiku 판단으로 처리. Routing 문장을 세 단계(조회 / 짧은 판단 / 깊은 분석)로 나눔. 장점: 컨텍스트 측정 불필요. 단점: 경계 사례 판정 기준이 주관적.
- 방안 C (classifier 축소): classifier를 Haiku로 낮추고 size tag 판정 기준을 명문화(예: 입력 길이 + 파일 수 + 키워드). Sonnet 호출 1회 절감. 단점: "classifier는 Sonnet" 원칙 변경이므로 사용자 승인 필요.
- 세 방안은 함께 쓸 수 있음(A 임계값 + B 유형 세분화 + C classifier 축소 여부 별도 결정).

## 5) 열린 질문
- 짧은 컨텍스트의 수치 기준은 무엇인가? (토큰, 파일 수, 줄 수 중 무엇을 쓸지)
- 사용량 감소 관찰의 근거(측정값)가 있는가? 현재는 체감 관찰만 기록됨. notes/metrics/ 데이터와 비교 필요.
- classifier를 Haiku로 낮추는 것이 "classifier는 항상 Sonnet" 원칙 변경으로 승인되는가?
- 짧은 판단을 haiku로 처리했을 때 제출 전 `sonnet-reviewer` 검수를 유지할지 여부.
- 적용 범위: chaude conductor 경로만인가, 일반 Routing 전체인가?
