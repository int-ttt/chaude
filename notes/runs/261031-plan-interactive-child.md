# 계획: 구독 상태에서 claude -p 대신 인터랙티브 claude 클라이언트 사용 타당성

## 배경
- 사용자 관찰: 구독 로그인 상태에서 claude -p 호출이 일반 대화보다 사용량을 더 많이 소모하는 느낌.
- 현재 자식 세션은 `claude -p --input-format stream-json` (core.go:145)으로 실행됨.
- 인터랙티브 TUI는 ANSI 화면 출력만 내보내고 구조화 이벤트가 없음. 하네스 신호(권한 control_request can_use_tool, task-notification 감지, total_cost_usd/usage, --include-partial-messages 스트리밍)는 모두 stream-json 의존.

## 방법 후보
A. PTY 화면 스크래핑 (session_id/usage 불가, 화면 변경에 취약, 제외 권고)
B. PTY로 입력, 출력은 트랜스크립트 JSONL tail + Stop/PreToolUse 훅(--settings 주입) (스트리밍·비용 일부 불가, DECISIONS.md:26 훅은 main 세션만 현재 적용, 코드 경로 2개)
C. 현행 유지 + 보완 (--resume 연동)

## 미확인 항목
1. 구독 로그인 + -p 사용량이 구독 포함인지, 일반 대화와 실제로 다른지 (billing.md:18)
2. 트랜스크립트 JSONL 경로·기록 시점·usage 필드 존재 여부
3. PTY 인터랙티브에서 --settings 훅 실행과 사용자 설정 병합
4. PreToolUse 훅이 can_use_tool을 대체할 수 있는지
5. 유휴 상태에서 task-notification 전달 여부
6. bracketed paste, 슬래시, ESC, 신뢰/온보딩 대화상자 처리
7. -p 없이 --output-format 무시 여부
8. 5시간 사용률·/usage 보고가 두 모드에서 같은지
9. 같은 세션을 -p 살아 있는 상태에서 --resume해도 안전한지

## 상시 세션 구성 (2026-10-31 논의 반영)
- 기본 상시 활성 세션: chaude당 4개 = main(opus), tui(haiku), coder(haiku), prompter(haiku).
- coder 대기 세션 4개를 추가, 합계 최대 8개. (해석: 기본 4 + coder 대기 4 = 8. 미확정: "coder 풀 자체의 최대 8"일 수도 있어 사용자 확인 필요)
- prompter와 tui 분리: tui 세션 출력은 main(opus) 입력 경로에 연결하지 않는다. 목적: tui 출력이 opus로 유입될 가능성 차단. 구체 구현(어느 경로로 격리할지)은 미확인.
- coder pool 상한: 모든 쓰기를 개별 처리하지 않고, 유사 작업을 하나의 haiku 세션 요청으로 묶어 처리하는 방향으로 상한을 둔다. 묶음 기준(파일 종류·디렉터리·작업 유형)과 묶음 크기 한도는 미정.
- prompter 방식: 일회성 서브에이전트 대신, 매 프롬프트 출력 종료 후 /clear로 컨텍스트를 비우는 상시 세션 방식을 검토 대상으로 함.
  - /clear 방식 평가(단정 아님):
    - 세션 재시작 비용: 프로세스를 유지한 채 컨텍스트만 비우므로 프로세스 기동 비용은 없을 것으로 보임. 단 -p stream-json 모드에서 /clear가 슬래시 명령으로 처리되는지는 미확인.
    - 캐시 손실: 대화 내용 캐시는 의도적으로 사라짐(prompter는 무상태라 손실 영향 작음). 시스템 프롬프트·도구 정의 prefix 캐시가 /clear 후에도 재사용되는지, 일회성 서브에이전트 대비 실제 절감 여부는 미확인.
    - 상태 유실: 반복 루프 라운드 로그·진행 상태가 /clear로 사라짐. 필요한 상태는 파일(notes/runs)이나 입력 인자로 넘겨야 함.
    - 타이밍: 출력 종료(result 이벤트) 확인 후에만 /clear 전송해야 경합을 피함. 비동기 task-notification이 /clear 이후 도착할 때의 동작은 미확인.
    - 결론: 동작 가능성 있음, 그러나 위 미확인 항목 검증 전에는 채택 확정하지 않음.

## 수용 기준
- 위 배경·방법·미확인 목록이 모두 들어 있을 것.
- 상시 세션 구성(기본 4 + coder 대기 4, tui/prompter 분리, prompter /clear 평가, coder 묶음 상한)이 미확인 표시와 함께 들어 있을 것.
