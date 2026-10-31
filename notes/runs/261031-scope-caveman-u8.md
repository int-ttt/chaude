# scope: caveman U8 implementation (haiku-prompter 전달문 = 원문 + 요약)
status: ESCALATE (nothing implemented)
source: plan/caveman-structure.md v7 (commit 529da82), sections 8 (:170-181), 10 (:191-197), rule_6 :42, table :135, U8 :249-251

## found
- 8절 S0~S5 touch: docs/claude-lean/CAVEMAN_FORMAT.md (new), haiku-scout, haiku-researcher, haiku-coder, haiku-batch-worker, haiku-runner, haiku-checker, haiku-drafter, opus-architect, sonnet-* (2), 감시 훅 설정, CLAUDE.md, haiku-tui.md (new, U9=a), haiku-prompter.md => well over 5 interlocked files
- U8 itself = one row (haiku-prompter 반환/전달: 원문 verbatim + 요약 줄); but its format depends on S0 정본 (3.1~3.6 템플릿) which is not yet written
- plan has no executable test plan; only 10절 정적 정합성 확인 5항목 (템플릿 문자열 일치, 언어 규칙 충돌 0, 훅 log 기본값·차단 경로 없음, drafter/L0 한국어 유지, 템플릿 맨 앞 status·reason 2줄)
- notes/runs/261030-plan-caveman-relay.md not read (scout excerpt only covered the design)

## conflicts / open decisions
1. 시작 절차 U6=b: worktree는 커밋된 HEAD 기준. 그런데 .claude/agents/haiku-prompter.md 는 untracked, 에이전트·훅 다수가 미커밋 수정 상태 => worktree에는 haiku-prompter.md가 없고 수정 전 버전이 기준이 됨. 어떤 기준으로 구현할지 결정 필요 (먼저 현재 변경을 커밋할지 / 그대로 HEAD 기준으로 진행할지)
2. 단계별 별도 worktree 브랜치·커밋, master 직접 적용 금지(C8), 병합은 S0~S5 완료 후(Q3). U8만 구현하면 S0~S5 중 어느 단계 브랜치에 넣을지 불명 (haiku-prompter는 S2 목록에 없음; 전달 역할은 U10=b 추가분)
3. "테스트"가 정적 확인뿐: 실행 가능한 테스트 없음 => 어떤 검증을 PASS/FAIL 기준으로 삼을지 확인 필요
4. U3/U4/U7 보류 유지 (설계 변경 없음)

## options
A. U8만 선행: haiku-prompter.md 에 전달문 형식(원문 verbatim + 요약 줄) 한 블록 추가 + CAVEMAN_FORMAT.md 초안 최소 작성 (S0 부분 + S2 일부). 요구: 위 1·2번 결정
B. S0~S5 전체 순차 구현 (단계별 worktree 브랜치): 큰 작업, opus-architect 또는 단계별 sonnet-analyst 위임 권장
C. 먼저 현재 미커밋 변경(haiku-prompter.md 포함)을 커밋한 뒤 A 또는 B 진행 (U6=b 와 충돌 없이 기준 확보)

## next
user decision on 1·2·3 (option A/B/C), then proceed
