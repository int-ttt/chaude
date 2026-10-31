# scope: v0.8.2 write-block release (ESCALATE)
status: 구현 미완 / 릴리즈 미진행 (커밋·태그·리뷰·haiku 비교 모두 보류)
evidence (haiku-scout 확인, a1cedfc4):
- sonnet-analyst.md:4 tools: Write, Bash, WebSearch, WebFetch, Agent -> Write·Bash 남아있음 (Read/Grep/Glob만 없음)
- sonnet-reviewer.md:4 tools: Read, Grep, Glob, Bash -> 제거 안 됨
- opus-architect.md:4 tools: Agent -> 이미 OK
- hook matcher: settings.json:70-74 `Read|Bash` -> Write/Edit/Grep/Glob 미차단
- guard.py: Read/Grep/Glob 차단은 sonnet-analyst만; sonnet-/opus- Bash는 읽기/쓰기 패턴 매칭 시만 차단; Write/Edit 차단 없음
- guard.py 11행: `_s.reconfigure(encoding="utf-8")` (stdio 인코딩) -> 로직 아님; 사용자가 말한 "reconfigure 경고"는 별도 확인 필요(실행 시 경고 재현 안 됨/미확인)
- CLAUDE.md: "파일 읽기·쓰기(Bash 포함)는 Haiku 에이전트만" 규칙 없음 (10~11행 유사 문구만)
- CHANGELOG: 0.8.2 항목 없음 (최신 0.8.1)
- tag v0.8.2 없음; 변경 파일: guard.py, settings.json (미커밋)
open decision (요구 충돌):
- sonnet-reviewer에서 Read/Grep/Glob/Bash를 모두 빼면 검토 자체가 불가(검토 = 파일 읽기). 선택지:
  A) reviewer는 Read/Grep/Glob 유지 예외 (규칙을 "analyst/architect는 금지, reviewer는 읽기 전용 허용"으로 완화; Write/Edit/Bash쓰기만 차단)
  B) reviewer tools = Agent만, 읽기는 haiku-checker/scout에 위임 (규칙 일관, 검토 품질은 Haiku 요약에 의존)
  C) 규칙 그대로 두고 reviewer 삭제, haiku-checker가 검토 (사용자 4번 테스트와 연결, 단 "릴리즈 검토 권한 haiku 불부여"와 충돌)
- sonnet-analyst도 Write 제거 시 notes/runs 결과파일 작성 불가 -> haiku-drafter/batch-worker 위임 필요 (브리프 규약과 일치하나 흐름 변경)
- 훅 matcher를 Read|Write|Edit|Grep|Glob|Bash 로 확장 + sonnet-/opus- 전면 차단 시 이 에이전트 자신의 notes 작성도 막힘
recommendation: B (규칙 일관, 최소 변경). confidence: medium.
next if approved: haiku-coder로 tools 4개 파일/훅/matcher/CLAUDE.md/CHANGELOG 수정 -> py_compile, go build/vet/test -> sonnet-reviewer(또는 B면 reviewer 경유) -> 커밋(Co-Authored-By: Claude Haiku 5.5) -> v0.8.2 태그/push/gh release -> haiku-checker 동일 체크리스트 비교.
