# 261031 measure-plan-tasks (written before any run)
source: notes/deprecated/261030-plan-caveman-relay.md (L23-31 conditions, L66-83 scoring/verdict), notes/runs/261031-plan-io-design-refine.md, notes/runs/261031-measure-results.md
caps: $1 per run (--max-budget-usd 1), $10 total; stop when reached; record total_cost_usd per run from claude JSON output
runner cwd: empty temp dir (no repo files, no CLAUDE.md); opus gets facts inline only (no tools) -> evidence = E-items in the fact packet

## conditions
A: opus, plain prompt (task + fact packet), default style. Relay by Haiku not simulated; A text = final text.
B: haiku compresses (facts/constraints/question only, no modifiers, keep paths) -> opus with CAVEMAN_SUFFIX -> haiku translates to 한국어 존댓말 (no new facts, keep all E-IDs and [확실|추정|불확실] tags)
C: original prompt (no compress) + CAVEMAN_SUFFIX -> opus -> haiku translate (same rule as B)
CAVEMAN_SUFFIX: "출력 형식: 조사·접속어·인사 생략, key: value/명사구. 근거 행 `E<n>: <경로>:<행> / <수치>`, 주장 행 `<주장> [확실|추정|불확실] [근거: E<n>,...]`. 조건문 유지. 근거 누락 금지."
runs: 2 tasks x 3 conditions x 1 repeat = 6 opus runs (+ haiku compress/translate calls); repeat of A once more only if budget remains (baseline noise |A1-A2|)

## tasks (fact packets from existing measurements)
T1 rotation threshold: question "회전 임계를 150k 통일 / Haiku 80k·Opus 270k 중 무엇을 권고하는가? 최종 권고 1문장."
 facts: E1 DECISIONS.md:46 현 유효 150k 통일 / E2 DECISIONS.md:41 제안 80k(Haiku) 270k(Opus) 미결정 / E3 measure-results M7: 80k 회전 16회 성공, 오버슈트 중앙 7,818 최소 1,510 최대 21,631 / E4 270k 4회 시도 모두 실패(271,531, last_res=0) / E5 회전 후 첫 호출 cache_creation 중앙 29,778 (21,488-45,247) / E6 haiku ctx>100k 48/420 (11.4%), opus 0/14 / E7 plan R7 Opus 270k 회전 약 $2.16/회, 150k 약 $1.20/회 / E8 settings autoCompactWindow 150000, 회전 임계보다 커야 회전 발동
T2 resident child: question "쓰기 상주 child가 매번 새 생성보다 나은가, 현 데이터로 판단 가능한가? 최종 권고 1문장."
 facts: E1 plan :29 현 child는 상주 claude -p stream-json 세션 / E2 results M5: resident/fresh 식별 필드 없음, turn.spawns 1건뿐 / E3 haiku 첫 턴 cache_creation 중앙 27,492, 이후 2,941 / E4 비용/턴 중앙 첫 0.0087, 이후 0.0392 / E5 opus 첫 cc 26,382, 이후 3,322 / E6 background haiku 평균 $0.129 지연 중앙 3.1s, foreground $0.128 지연 55s / E7 haiku ctx>100k 48/420 (11.4%)

## scoring (per plan L66-83)
evidence items = file path, line, number, named source, quote = 1 item each. 근거 누락 수 = A의 근거 항목 중 최종 번역본에 없는 수 (A 추출을 B/C 보기 전에 수행). 역방향 근거 수 = 번역본에만 있는 항목.
조건부 표현 누락 = A의 불확실/조건 표현(추정, 미확인, ~라면, ~일 수 있음) 중 번역본에 없는 수; 뉘앙스 유지율 = 유지/전체.
결론 일치: 최종 권고 문장 1개 비교 -> 일치/부분/불일치.
환각·오역: 있음/없음.
설명 규칙 위반: 한국어, 존댓말, 최소 토큰 (CLAUDE.md 기준 3항).
Opus 총 출력(thinking+visible) 절감 = (A out - X out)/A out.
verdict: 유망(절감>=30% & 근거 누락 0 & 결론 불일치 0 & 스타일 통과) / 판단 불가(|A-B| <= |A1-A2| 또는 단가 미확인) / 기각(B가 A보다 비쌈 또는 근거 누락>=1). 임계 30% 변경 없음.
judge: 채점은 Agent 내 sonnet-reviewer 대신 haiku-checker 문자열 대조(E-ID, 수치) + 본 보고자 판독; 블라인드 라벨 X/Y 사용 불가(보고자가 조건 앎) -> 한계로 기록.
