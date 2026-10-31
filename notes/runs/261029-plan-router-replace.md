# 계획 rev2: 라우터 교체 (Sonnet -> Haiku, Sonnet 라우터 deprecated)

요청 원문: "결과 다듬기, 라우터 교체, haiku 최적화 진행, sonnet 라우터는 deprecated 처리."
기반: 261029-plan-router-haiku-policy.md, 리뷰 반영본(261029-plan-router-replace-review.md 15항 중 채택분)
개발셋 30개: 261029-haiku-router-30-samples-list.md (라벨은 /tmp/router3t/run.py EXPECTED, 호출 전 확정)

## 제약
- 커밋/릴리스 없음. API 키 없음(cleanEnv, 구독 인증; 비용은 보고 단가이지 청구액 아님).
- CLAUDE.md, notes/DECISIONS.md 수정 금지(승인 없음). 수정안은 결과 노트에만 기재.
- 기본 라우터 모델은 사용자 승인 전까지 sonnet 유지. 게이트 통과해도 S6 전환은 하지 않고 "승인 대기"로 보고 (리뷰 13).
- 라이브 총비용(튜닝 포함) $3 상한. 도달 = 실패(sonnet 유지). 튜닝/평가 비용 별도 집계.

## 단계
- S0 (구현 잔여; 대부분 기존 코드에 있음): routerModel()/classifyPromptSonnet/Haiku/routerPrompt 존재 확인. 추가: 메트릭 router_prompt, Sonnet 프롬프트 sha256 고정 테스트, 프롬프트 덤프 테스트 헬퍼(CHAUDE_DUMP_PROMPTS=dir일 때만), env 테스트("SONNET " 포함). 기본값 sonnet 유지.
- S1 현황 조사: 위 코드로 확인 완료. 수정 대상: classifier.go 주석(S7), repl.go:60 주석. 비대상: CLAUDE.md, DECISIONS.md.
- S2 Haiku 프롬프트 튜닝: 개발셋 30개만. 기존 규칙 의미 유지(표현 변경과 "어느 규칙에도 안 맞으면 sonnet" 추가까지). 초안 최대 5회, 개발셋 문장/키워드를 예시로 넣지 않음(티어당 예시 1개 이하). 프롬프트는 덤프 파일에서 러너가 읽음(수동 복사 금지). 동결 sha256 기록. 현행 프롬프트 해시도 기록.
- S3 평가셋 150개: 동결 후 새 haiku-batch-worker 인스턴스가 티어 정의만 보고 작성(프롬프트 문안 비공개). 티어당 50개, 비-haiku 100개, 개발셋과 겹침 0(정규화 완전일치 검사), 경계 사례는 sonnet 라벨+표시+10% 이하, 머리말에 동결 sha256, 라벨은 호출 전 고정.
- S4 라이브(순서 고정 a->b->c, 팔 단위 누적비용 기록, 러너 total_cost_usd 합산): (a) Sonnet+현행 x1, (b) Haiku+새 x3, (c) Haiku+현행 x3. 150프롬프트, 총 1,050콜 예상 약 $1.0 (Sonnet $0.0045/콜, Haiku $0.0003/콜 실측 기반).
- S5 게이트 판정(오류 = 3회 평균, 누수는 한 번이라도 새면 1건):
  - 누수(비-haiku 프롬프트 -> haiku) 0, 고유 프롬프트 기준, 비-haiku 표본 >= 100. Sonnet 기준선 누수도 보고.
  - 총 오류 <= Sonnet + 3건
  - 티어별 오류 <= Sonnet 해당 티어 + 2건
  - opus->sonnet <= Sonnet 기준선 + 2건
  - 일관성: 150프롬프트 중 3회 모두 같은 티어 >= 90%
  - tier 누락 0, 파싱 실패 0(재시도 후)
  - haiku 실행 task 접미('reply: Korean 존댓말...') 바이트 일치 100%
  - 비용 <= $3
  - 판정 규칙: (b)가 (c)보다 낫지 않고 (c)가 통과하면 (c)(현행 프롬프트)로 전환 대상 변경.
  - 게이트 실패 시 재튜닝 금지(재튜닝은 새 평가셋 필요).
- S6 기본값 전환: 수행하지 않음(승인 대기). 통과 시 필요한 변경과 롤백 검증 계획만 기록.
- S7 deprecated 표기: classifyPromptSonnet 바이트 유지(해시 테스트), 주석/문서 문자열만, 허용값 sonnet 유지. repl.go:60 주석을 실제 동작에 맞게 정정. 기본값이 sonnet이므로 "default to be replaced after approval"로 표기.
- S8 검증: go build/vet/test, CHAUDE_LIVE 미설정 시 라이브 스킵, 판정표를 notes/runs/에 저장, sonnet-reviewer로 최종 diff 검토.

## 담당
S0/S7: haiku-coder. S2: 설계는 본 에이전트, 프롬프트 반영은 haiku-coder. S3: haiku-batch-worker(독립). S4/S8: haiku-runner. S5: 본 에이전트. 완료 전 sonnet-reviewer.

## 열린 질문(사용자)
- S6 전환과 CLAUDE.md:32 / DECISIONS.md 수정 승인 문구.
- 평가셋 경계 사례 라벨을 호출 전 사용자가 확인할지.
