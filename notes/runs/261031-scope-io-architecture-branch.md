# scope: io-architecture plan -> branch interactive-child-compare (261031)
status: ESCALATE, nothing implemented
main_plan: notes/runs/261031-plan-io-architecture-review.md
conflict: plan says 쓰기 별도 인터랙티브 클라이언트 비권장 (현 child = 상주 `claude -p --input-format stream-json` 이미 맥락 유지; TUI 전환 시 비용·권한·알림 신호 상실, 261030-plan-interactive-child-review.md:32). Branch name/goal = interactive-child compare. 
open_decision: A) branch를 비교 실험(측정)으로 한정 / B) 계획 권장 구조대로 현행 유지+지시문 최적화만 / C) 인터랙티브 child 그대로 구현
## plan-derived work items (no conflict)
1. 읽기: scout/researcher 기본 상한 4000자 유지, "줄 제한 없음" 미채택; 요청 시 확장
2. 쓰기: Opus -> 정확한 사양 -> 상주 stream-json haiku-coder child(100k 전 회전) / 일회성 일반 에이전트; 회신은 변경 파일·방식·검사 결과만
3. 출력: Opus 영어 key: value, 최종 사용자 답만 메인 Haiku 존댓말
4. 입력: 압축 홉 없음, 대용량은 파일 경로
5. 측정 6항목(caveman B·C, Opus 5회 비용 구성, 한/영 토큰비, 읽기 상한 비교, 상주 vs 신규 child, 홉 지연) 먼저
## files likely affected
agents/*.md (Return 규칙), CLAUDE.md, internal/harness/livechild.go, repl.go, orch*.go (interactive child 관련), notes/FACTS.md
## unverified
branch 내 uncommitted 변경 다수(livechild.go, orch.go 등) 상태; 인터랙티브 child 구현 범위 미확인
