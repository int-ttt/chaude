# review: notes/runs/261031-plan-progress-report.md (261031, opus-architect)
verdict: revise before S1. 3 open Q (5절) decided below; 6 concrete fixes
spec: plan/caveman-structure.md (정본, 수정 안 함)

## decisions
A (Q-A, 진행 줄 위치): (c) 둘 다, 역할 분리
- 원천 = 파일 `logs/<job>.status` (haiku-runner.md:11 기존 규칙 재사용, 새 경로 안 만듦). 실행자가 1줄씩 append
- 파일 줄 형식 (agent-read, C11): `step: <noun phrase> / pct: <0-100>` -> 2절 15행 `단계:/진행률:` 한글 키 교체
- 사용자 대면 = haiku-tui가 status 마지막 줄 Read -> 존댓말 1줄. 메인은 그 줄만 verbatim 게시
- why: 서브에이전트는 끝날 때만 부모에 반환 -> 실행 중 진행률은 파일 폴링 외 경로 없음. haiku-tui 도구 = Read 뿐
- 18행 보정: "메인 중간 텍스트 없음" 예외 = 사용자가 진행상황 물을 때 / 단계 완료 시 haiku-tui 줄 1개만
B (Q-B, scout 장시간): 빌드·테스트·장시간 실행 금지 -> haiku-runner 소유. 긴 검색만 허용
- 상한: Bash 명령 1회 ≤120s (기본 timeout), scout 실행 1회 총 ≤10min (DECISIONS.md:18 idle 10min과 맞춤)
- 초과 시 status: partial, reason: time limit, done/remaining 목록 notes/runs/ (haiku-scout.md:17 기존 규칙) -> 상위가 분할 재위임
- 14행 수정: 실행자 = haiku-scout(검색) | haiku-runner(빌드·테스트·장시간) | 소유 Haiku(수정)
C (Q-C): :160 유지 (사용자 지시). 5절 Q-C 삭제, 6절 정합성 메모로 이동
- 조정: :42/:126/:133 = 전달문 형식 규칙 (원문 verbatim + 요약, U8=c). :160 = 감시 훅 규칙. 형식은 바꾸지 않음
- :160 5키 검사는 log 전용(C8): 누락 기록만, 차단·재작성 요구 없음. prompter 5키 누락 로그는 "예상값", 위반 아님
- 5키를 전달문에 덧붙이지 않음 (= :249 기각안 (a) 원문+B형식 머리 부활 위험)
- 추가: :171-:174 감시 대상 목록에 haiku-prompter·haiku-tui 없음. 훅은 a916231부터 둘 다 검사 -> S2·S5 목록에 두 에이전트 유지 명시

## fixes (행 = 계획서 행)
1. 4·19행: notes/pastes/70ebcbbe.md 내용 = 훅 재작성 서브에이전트 보고 (사용자 원문 아님). 진행 보고 요청 근거 아님 -> "출처: 사용자 요청 (원문 위치 미확인)"으로 교체
2. 56행: haiku-tui 신설 -> 수정. .claude/agents/haiku-tui.md 이미 존재 (tools: Read). 추가할 것 = status 줄 -> 존댓말 1줄 변환 규칙
3. 34행 vs 37행: 병합 1회인데 기준점=HEAD면 S2가 S1 산출(CAVEMAN_FORMAT.md)을 못 봄 -> 적층 브랜치: S(n) worktree 기준점 = S(n-1) 커밋. 되돌리기 = 해당 커밋 revert 유지
4. 49행 S2 "검사 대상 S1 에이전트만" vs 현 훅 10종 (68행 미결): S2에서 축소하면 a916231의 tui·prompter 검사 소실 -> S2는 목록 축소 안 함, :171 범위 차이는 사용자 결정까지 보류
5. 50·51행 검증: 에이전트 파일 수정인데 go build/vet만 -> 각 단계에 python -m unittest discover tests/hooks 추가 (훅이 반환 형식 검사)
6. 신규 단계 S0.5 (S1 전, haiku-coder): haiku-runner.md·haiku-scout.md에 status 줄 형식, haiku-scout.md에 B 상한 추가. 검증: haiku-checker grep `step: .* / pct:` 3파일

## edge cases
- status 파일 없음/빈 파일 -> haiku-tui "아직 진행 기록이 없습니다" 1줄. pct 역행·100 초과 -> 마지막 유효값 사용
- 병렬 실행자 -> job별 파일 분리, 공유 금지

## must NOT change
- plan/caveman-structure.md (:160 포함), CLAUDE.md, 훅 모드(log), :227/:247 U8 문구 불일치는 6절 메모만 (정본 수정 안 함)

## open_q (사용자만)
- 진행 보고 요청 원문 위치 (70ebcbbe.md 아님)
- :171 감시 범위 2종 vs 현 10종 유지 여부
