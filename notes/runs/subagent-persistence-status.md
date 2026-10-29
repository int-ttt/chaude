# 서브에이전트 지속화(persistence) 현황 조사 (코드 수정 없음, 2026-10-29)

## 결론
- 디스크 저장/복원 형태의 서브에이전트 지속화는 미구현입니다. 코드 근거 없음, 전용 노트 없음(notes/runs에 subagent/persist 이름 파일 없음; notes/OPEN.md 파일도 존재하지 않음).
- 존재하는 것은 "프로세스 내 메모리 유지"뿐입니다: rotate 시 옛 세션을 drain으로 살려 두는 방식.
- 듀얼 메인(Sonnet/Opus) 구현은 아직 시작되지 않았습니다(dual-main-sessions.md는 "기본 방식으로는 부적합" 판단 문서이며 코드 변경 없음). 사용자 확정은 이 문서의 판단과 다르므로 확정 방향으로 재설계 필요.

## 1. 이미 있는 것 (증거)
- internal/harness/drain.go: agentRec/agentTracker(메모리 map, mu 잠금). onToolUse/onToolResult/onNotification으로 Agent 위임을 추적. `type drain` = rotate 후 옛 child 프로세스를 에이전트 종료까지 유지(timeout 기본 60분, repl.go:827), consume이 stdout을 비우고 권한 요청을 자동 거부("session is draining").
- repl.go:536 `rotate`, :581 옛 세션+tracker를 r.draining에 push, MaxDrain 기본 3(:826) 초과 시 가장 오래된 것 Close. :624 closeDrains 주석: "harness exit; unfinished agents stay only in the transcript" 즉 하네스 종료 시 미완료 에이전트는 추적 상태 소실(Claude transcript에만 남음).
- agentview.go(미커밋, untracked): 에이전트 패널/뷰 UI(snapshot, sortedAll). 읽기 전용 표시이며 저장 없음.
- resume.go: Claude Code transcript(~/.claude/projects/*.jsonl)를 읽어 대화 turn을 가져오는 /resume. 에이전트 상태 복원 코드 없음(grep: resume.go, importclaude.go에 tracker/agents 참조 없음).
- 테스트: drain_test.go TestAgentTrackerAndDrain, TestDrainConsumeDenies (메모리 동작만). resumeshift_test.go TestRotateKeepsPendingSnapshot(스냅샷 유지만). agentview_test.go는 UI 테스트. 지속화 테스트 없음.

## 2. 없는 것
- agentTracker 직렬화/디스크 저장, 재시작 후 복원, 에이전트 ID로 resume, 새 세션에서 이전 에이전트 결과 수신.
- 이전 조사(notes/runs/261028-longsession-improvements.md 53행)의 미확인 항목 4개(새 session_id에서 이전 에이전트 ID resume 가능?, 부모 child Close 시 백그라운드 에이전트 생존?, 완료 알림 귀속, stream-json 식별)가 이후 검증된 기록을 찾지 못했습니다(추측: 미검증 상태).
- 261028-plan-sonnet-conductor.md 102행: 워커를 하네스 소유 독립 session_id로 두면 --resume으로 지속 가능하다는 [추정] 설계만 있고 미구현.

## 3. 듀얼 메인 세션 구현에 필요한 변경 범위 (dual-main-sessions.md 5절 + 이번 확인, 일부 추측)
| 파일 | 변경 |
|---|---|
| repl.go | Repl 단일 상태(sess, tracker, lastCtx, pendingSnapshot, files, draining)를 세션별 구조체 2개로 분리; rotate를 활성 포인터 전환으로; 입력 라우팅 |
| classifier.go | pickModel/route 의미를 "어느 세션으로 보낼지"로 변경(현재 cur가 opus/빈 값이면 항상 opus, 31~39행) |
| drain.go | tracker/drain을 세션별로 소유; 에이전트 지속화를 넣는다면 agentRec 직렬화 |
| core.go | Session 수명/ID 관리 2개, 모델별 spawn (추측: 세부는 미확인) |
| resume.go / importclaude.go | 두 세션 ID 저장·복원, 에이전트 상태 복원(신규) |
| tui.go / agentview.go | 활성 세션 표시, 패널에 세션 태그, 두 tracker 합산 |
| *_test.go | TestMainRouteSwitchPolicy 등 재작성, 지속화 테스트 신규 |
- 신규 필요: 지속 저장소(예: notes 또는 ~/.claude 외부 상태 파일) 포맷 결정, 세션 간 델타 요약 주입, TTL 관리.
- 5개 이상 파일이 얽히므로 설계 결정(저장 형식, 에이전트를 하네스 소유 독립 session으로 둘지) 선행 필요.

## 4. 불확실점
- Claude CLI가 새 프로세스에서 이전 서브에이전트를 resume 할 수 있는지, 부모 종료 시 백그라운드 에이전트 생존 여부: 미검증.
- git diff 미커밋 변경(repl.go, drain.go, resume.go 등)에 지속화가 숨어 있는지는 grep(persist/지속/restore/rehydrate)로 확인했으며 해당 없음. diff 전체를 줄 단위로 읽지는 않았습니다.
