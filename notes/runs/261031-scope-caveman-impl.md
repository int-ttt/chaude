# scope: caveman 전체 구현 + v0.9.0 (ESCALATE)
reason: 5+ interlocked files; 보류 결정 3건; 병합 절차 충돌; 버전 불일치
evidence (plan/caveman-structure.md 8절, 14절):
- S0 docs/claude-lean/CAVEMAN_FORMAT.md 신설 (3.1~3.6 정본)
- S1 haiku-scout, haiku-researcher 템플릿 + 감시 훅 log 모드 등록
- S2 haiku-coder, batch-worker, runner, checker, drafter R-write 템플릿
- S3 opus-architect, sonnet-* B형식 + R-decide
- S4 훅 검사 대상 확대 (훅 설정 1곳)
- S5 CLAUDE.md:4 범위 한정, O2 예외줄 제거
- 신설: haiku-tui (U9=a); haiku-prompter 루프 유지 + 전달 역할 추가 (U10=b), 전달문 원문+요약 (U8=c)
open decisions:
1. U3/U4/U7 보류 (PostToolUse matcher 이름, mode f, lang:ko 훅 가시성) -> 구현 중 조사 필요; 설계 갈림
2. U8 해석("사용자 대면 출력은 haiku-tui") 미확인
3. 병합 규칙: 계획서는 worktree 브랜치 + S0~S5 완료 후 master 병합, 커밋된 HEAD 기준(U6=b). master에 미커밋 변경 다수 -> 사용자의 "전부 구현"과 충돌 가능; 현 작업트리 직접 수정 여부 결정 필요
4. 버전: cmd/chaude/main.go:16 version="0.8.1", CHANGELOG 최상단 0.8.2(하네스 전용). v0.9.0 범위(Go 변수+CHANGELOG) 확인 필요
options: (a) S0~S2만 먼저 구현 후 테스트 (b) 보류 3건을 먼저 조사 후 전체 구현 (c) 사용자가 3건 결정 후 진행
recommend: (b) 후 단계별(S0→S5) 구현, 단계마다 haiku-runner로 go test + 훅 검사
