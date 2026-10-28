# 구현 = 테스트 표본 (계획 7-8 단계 1-2)

## 변경 (internal/harness)
- metrics.go (+test): notes/metrics/<날짜>.jsonl 한 줄씩 기록(ts, flags, usage 4종, ctx). `--metrics-dir`(빈 값=끔).
- handoff.go (+test): extractReco(마지막 `추천:` 줄~끝, 절단 없음), validateHandoff(길이 200자, 7섹션, 변경 파일 기본명 언급), withReco, recoPreserved(바이트 포함 검사).
- drain.go (+test): 에이전트 추적(Agent/Task tool_use, 브리프의 notes/runs/*.md 경로 추출), 파일 2초 무변화 완료 감지, 타임아웃.
- repl.go: threshold 기본 270000, `>=`, 매 답변 후 판정; rotate에서 검증 실패 시 1회 재시도 후 결정적 스냅샷 폴백, 노트·턴 없으면 보류+쿨다운(`--rotate-cooldown` 2분); 추천 블록을 새 세션 첫 입력 앞에 붙임; 에이전트 실행 중이면 이전 세션을 draining으로 유지(`--max-drain 3`, `--drain-timeout 60m`), 완료 시 `[agent done]` 줄을 다음 입력 앞에 주입; rotate/turn 메트릭(handoff_valid, reco_preserved, rotate_at_tokens). `--no-handoff-validate`로 안전장치 끔.
- 번역 단계 제외. `--max-auto-rotations` 플래그는 acceptance_test 때문에 유지(기본 무제한).

## 검증
go build ./... OK, go test ./... OK. 실제 claude(haiku) 1턴으로 메트릭 파일 생성 확인(notes/metrics/*.jsonl, ctx 20,571). 커밋 없음.

## 표본 수치 (notes/runs/261029-impl-as-test.jsonl)
- 위임 2건 모두 haiku-coder, 재시도 0, 내 수정 0: metrics 14,018 tok/27.6s/도구 10회; handoff 16,861 tok/37.3s/도구 6회.
- 직접 구현(drain+repl 연결)은 위임 안 함(repl.go와 얽힘); 토큰/시간은 측정 불가(null). 표본 n=2는 결론용으로 부족.

## 미검증
- rotate 경로 end-to-end(270k 도달, 검증 실패 폴백, drain 진행)는 가짜 claude 없이 못 돌려 단위 테스트만 있음.
- 백그라운드 에이전트의 tool_result에 경로/완료 이벤트가 오는지(계획 3-1 확인 필요 a-d), 드레인 중 이전 child의 control_request 자동 거부와 stdout 소비는 미구현(채널 버퍼 256 초과 시 막힐 수 있음).
- 변경 파일 검증은 git status 대신 r.files 사용. 드레인 중 TUI 상태줄 "에이전트 N개 대기" 미구현. 4:50 무입력 정리는 범위 밖.

## 추가 (승인 후)
- drain.consume: 이전 child stdout 소비 + control_request 자동 거부(단위 테스트). closeDrains: 종료 시 정리. 완료 감지는 2회 폴링 대신 파일 mtime 2초 경과로 변경(1차 e2e에서 종료 시점에야 닫히는 결함 발견).
- 턴 메트릭 ctx 키 덮어쓰기 버그 수정(result 합계는 result_* 키).
- 위임 규칙 내장: 자식 claude에 `--append-system-prompt`로 delegationRule 전달(`--no-delegation-rule`로 끔). haiku-coder 위임으로 구현.
- 전환 e2e(threshold 20000, haiku): 3회 전환, handoff_valid 3/3, reco_preserved 3/3, 그중 1회는 백그라운드 에이전트 실행 중 drained -> 파일 완료 후 drain timed_out=false로 종료. 
- 위임 표본 추가: 11,286 tok/18.7s(재시도 0), 17,888 tok/33.2s(재시도 0). 위임 4건 평균 약 15k tok, 29s.
- 한계: 에이전트 `[agent done]` 통지가 다음 사용자 입력에 붙는 경로는 e2e에서 확인 못 함(드레인이 끝난 시점 이후 입력 없음). 규칙이 가리키는 haiku-coder는 프로젝트 .claude/agents에 없음(전역 에이전트).
