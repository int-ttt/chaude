# 261031-plan-caveman-c 설계 검토 (개선안만)

target: notes/runs/261031-plan-caveman-c.md (201행)
reviewer: opus-architect, 2026-10-31
mode: 검토만, 실행 안 함
format: 번호 | 근거 경로:행 | 수정 제안

## (a) 선택지 C 범위의 빠짐

1. plan:37-38 / internal/harness/routerprompt_test.go:13, classifier_model_test.go:8 | D3·D4 사유가 서로 뒤바뀜. D3=라우터 프롬프트 SHA 테스트(sonnetPromptSHA256), D4=라우터 모델 env 테스트(TestRouterModelDefault)로 고친다.
2. plan:60 / internal/harness/tui.go:938-940, :1161 | tui.go는 171행 도움말만 있음. /mode conductor 분기 938-940과 1161 case를 §4.1과 P2 위임 항목에 추가한다.
3. internal/harness/resumeshift_test.go:63 | 주석 "conductor route"가 목록에 없음. §4.3에 주석 정리(또는 허용 목록 등록)를 추가한다.
4. internal/harness/budget.go:42-43 | chargeOneShot 주석에 classifier가 있음. D1 삭제 후 호출처가 /usage만 남는지 확인하는 단계와 주석 수정을 P2에 추가한다.
5. .claude/skills/distill/SKILL.md:14, .claude/skills/correction/SKILL.md:8 | 스킬 2개가 목록에 없음. distill의 "one sonnet-analyst" 병합 담당을 haiku-batch-worker(기계적 병합) 또는 opus-architect로 바꾸는 행을 §4.2에 추가한다.
6. .claude/agents/haiku-coder.md (sonnet 1건) | 목록에 없음. 해당 행을 확인해 §4.2에 추가한다.
7. README.md:24,35 | 회전 임계값 80000/270000과 sonnet 표기가 남아 있고, 이미 stale 상태(DECISIONS.md:46 = 150K). README 수정을 범위 안에 넣고 150K 기준으로 고친다.
8. CHANGELOG.md:14,16, docs/claude-lean/CHANGELOG.md:10 | 기록성 문서라 수정하지 않음을 명시하고 §9 허용 목록에 넣는다.
9. plan/interactive-child-resident-sessions.md:16 | classifier.go:258-285를 참조함. "D1 보관본 참조"로 주석을 달거나 허용 목록에 넣는다.
10. tests/hooks/test_orch_gate.py:78-85 | plan:96·199의 "확인 필요"는 이미 확인됨. test_4a/4b가 sonnet-analyst payload를 쓰므로 Q-B 결과에 맞게 opus-architect payload로 다시 쓰는 작업을 P3에 명시한다(검증 전에 수정).
11. .claude/hooks/main-thread-guard.py:32 (plan:78) | "final review -> sonnet-reviewer"를 opus-architect로만 바꾸면 E gate와 충돌하고 비용이 크다. "final review -> haiku-checker(근거 수집) + 판정 필요 시 opus-architect"로 바꾼다.
12. notes/DECISIONS.md, notes/LESSONS.md:5 | 선택지 C 결정이 notes에 기록되지 않는다. 마지막 단계(P8)에 DECISIONS 신규 항목 [routing] 261031과 LESSONS:5 대체 메모 추가를 넣는다(기존 행은 수정하지 않음).
13. plan:91 / internal/harness/repl.go:27-28 | opusSonnetThreshold 제거는 Q-D=(a)일 때만 해당. "Q-D 조건부"로 표시한다.

## (b) opus 다중 haiku 대체의 비용·품질

14. plan:111 / budget.go:94 vs internal/scripts/usage_report.go:18 | sonnet 가격 필드 구조가 다르다({2,2.50,4,0.10,10} vs {2.0,4.0,0.20,10.0}). 측정 전에 두 표의 열 의미를 맞추는 단계를 Q-C에 넣는다. 맞추지 않으면 비교가 무효다.
15. plan:114, 193 | "구현 후 측정"(U1=a)만 있고 기준선이 없다. P0에 notes/metrics/*.jsonl·LEDGER.md에서 sonnet-analyst·sonnet-reviewer의 호출당 토큰·비용(최근 7일)을 뽑아 기준선으로 저장하는 단계를 추가한다.
16. plan:113 | 병합 단계의 opus 입력은 하위 반환 합계다. opus-architect.md에 "haiku 병렬 호출 ≤4개/과제, 반환 각 ≤4000자, 응답 ≤20회" 상한을 넣는다.
17. plan:119 (위험 1) | 완화책이 없다. 머지·릴리즈 검토는 "설계한 opus와 다른 새 opus-architect 호출(diff+계획서만 입력) + haiku-checker 체크리스트"로 정하고 LESSONS.md:5의 메인 디스패치 규칙과 맞춘다.
18. plan:107-135 | sonnet-analyst가 맡던 복잡도 1 작업(긴 설명, 비교)까지 opus로 가면 소형 과제 비용이 크게 늘어난다. 복잡도 1은 메인 -> haiku-drafter/haiku-scout 직행, opus는 복잡도 2 이상으로 하는 라우팅 규칙을 Q-E의 CLAUDE.md 개정안에 넣는다.
19. plan:141-142 (Q-B) | 기본 제안이 없다. (b) "opus-architect는 brief에 return: R-decide가 있으면 E gate 예외"를 기본으로 제안하고, test_orch_gate에 해당 예외 테스트 1개를 추가한다.
20. plan:120 (위험 2) | E<n> 누락을 감시에만 의존한다. P3 검증에 caveman-format.py로 opus-architect 반환 샘플 1건을 점검하는 단위 테스트를 추가한다.

## (c) 실행 순서와 검증 명령

21. plan:7, 160 (P1 보관) | 치명적. .go 원본을 notes/archive/261031/에 그대로 복사하면 같은 모듈(go.mod 루트) 안의 패키지가 되어 `go test ./...`가 컴파일 실패한다. 파일명에 `.go.orig` 접미사를 붙이거나 `_`로 시작하는 디렉터리(Go가 무시함, 예: notes/archive/261031/_go/)에 보관한다.
22. plan:161, 166 / multiproc_s_test.go:371-378 | classifyFn 스텁 제거가 P4에 있는데 D1 삭제(P2) 직후 P2 검증에서 컴파일이 실패한다. 스텁 제거를 P2로 옮긴다.
23. plan:163, 168 | P3(sonnet 에이전트 삭제)와 P5(CLAUDE.md 개정) 사이에 CLAUDE.md가 없는 에이전트로 라우팅한다. Q-E 승인을 P3의 선행 조건으로 두고 P3와 P5를 같은 커밋으로 묶는다.
24. plan:154 | "단계마다 별도 worktree"는 단계들이 서로 의존하므로 맞지 않는다. 단일 worktree 브랜치(caveman-c)에서 단계별 커밋, 실패하면 해당 커밋 revert로 바꾼다.
25. plan:137-150, 152-171 | Q와 단계의 연결이 없다. 차단표를 추가한다: P2←Q-A, P3←Q-A·Q-B·Q-E, P4←Q-D, P5←Q-E, P4 Go 가격행←Q-C, 전체←Q-F.
26. plan:181, 200 | 러너는 확인됨(unittest, pytest 아님). 명령을 `python3 -m unittest discover -s .claude/hooks/tests -p 'test_*.py'` 와 `python3 tests/hooks/test_orch_gate.py` 로 확정한다.
27. plan:180 / tests/e2e-rebuild/go.mod | 루트에서 실행하는 `go test ./...`는 별도 모듈인 tests/e2e-rebuild를 포함하지 않는다. 포함할지 제외할지 명시하고, `go vet ./...`를 추가한다.
28. plan:183 | grep 패턴에 conductor가 빠져 있다(P6 plan:170에는 있음). 제외 디렉터리에 notes/archive·notes/runs·notes/metrics·imports를 추가하고, 허용 목록을 파일로 고정한다: scripts/usage_report.py·usage_sync.py·claude_usage_import.py(classify 오탐), samples/probe_misc.jsonl, internal/harness/testdata/bg_turn_live.jsonl, CHANGELOG 2개, notes/*.md, CLAUDE_HARNESS_ALLOW_SONNET_IO.
29. plan:158-159 | P0에서 haiku-scout와 haiku-checker가 테스트를 중복 실행한다. 테스트는 checker만, scout는 grep 전수 목록만 맡긴다.

## (d) 릴리즈 범위 분리

30. plan:16 | 범위 밖 선언은 적절하다. 다만 이번 변경은 호환성을 깨는 변경이다(-conductor 플래그, /mode conductor, sonnet 에이전트 2개 삭제, Q-D(a)이면 /model sonnet). 이를 notes/runs/261031-plan-release-v082-v090.md의 미결 항목으로 넘기는 한 줄을 §1에 추가한다.
31. DECISIONS.md:34 (release.yml이 커밋 목록으로 노트 생성) | 단계 커밋 메시지 접두사를 "caveman-c P<n>: <한국어 요지>"로 고정해 다음 릴리즈 노트를 그대로 읽을 수 있게 한다.
32. docs/claude-lean/APPLY_GUIDE.md:24, FRESH_START.md:34,45 (--main sonnet) | Q-D(a)이면 사용자 안내가 깨진다. 이 문서 수정은 Q-D와 묶고, 기록성 문서와 구분해 "안내 문서 = 수정 대상"으로 분류한다.
33. git 상태 (v0.9.0 태그는 이미 master에 있음, notes 미커밋) | 작업 기준을 "v0.9.0 이후 커밋된 HEAD, 태그 재생성 금지"로 명시하고, 미커밋 notes 변경은 P0 전에 별도 커밋하거나 제외한다고 적는다.

## 요약

count: 33 (a 13, b 7, c 9, d 4)
critical: 21 (archive .go가 go test를 깨뜨림), 22 (classifyFn 순서), 23 (에이전트 삭제와 CLAUDE.md 사이 공백)
