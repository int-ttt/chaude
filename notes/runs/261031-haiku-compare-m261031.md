# m261031 final-text compare (A=opus baseline; B,C=haiku)
scope: T1 (회전 임계), T2 (상주 vs 신규). read-only. no edits.

## T1 근거 누락 (A items, 14 checked; present = number or E-ID in text)
- A_missing_in_B: 80% 비용차 (A), 175k+ 권장 (A) -> 2
- A_missing_in_C: 80% 비용차, 175k+ 권장, "Opus 150k 실측 미확인" caveat -> 3
- 역방향 B-only: 101.6k, 102k, +$0.96, 7,818(median), 1,510(min), 271,531, 21,488-45,247 -> 7
- 역방향 C-only: 7,818, 1,510, 271,531, 21,488-45,247, $0.96 saving, 270k 실패 원인 claim -> 5 (+1 claim)

## T1 hedges (strict terms: 추정|미확인|불확실|~라면|~일 수 있|판단 불가)
- A: 1 (주기라면)
- B: 8 (추정 4, 불확실 2, 라면 1, 일 수 있 1)
- C: 5 (추정 3, 불확실 2); +1 loose ("보임")
- 유지율 (A point "150k 발동 미검증" kept as hedge): B 1/1=100%, C 0/1=0%

## T1 recommendation vs A
- A: 혼합안: Haiku 80k / Opus 150k 유지 / 270k 기각 / autoCompactWindow >=175k
- B: 부분. Haiku 80k·270k 기각 일치; Opus를 "<150000"으로 유지(A: 150k) ; autoCompactWindow 상향 없음; 경계값 검증 후 DECISIONS:41 확정
- C: 일치(핵심). Haiku 80k / Opus 150k 유지 / 270k 재시도 전 autoCompactWindow >270k (A: >=175k, 목표값 상이)

## T1 환각/패킷외 flags
- B:25 "IDE에 열린 프로젝트..." environment claim, not in packet
- B:21 "101.6k/102k", "+$0.96": derived (21,631+80k; 2.16-1.20) OK
- C:18 "270k 실패 원인: 임계 > autoCompactWindow" causal, inferred from E4+E8, not stated as fact -> flag as 추정
- A:6 "약 175k": derived 150k+21,631 ~171.6k, rounded up -> note

## T2 근거 누락 (A items, 11 checked)
- A_missing_in_B: 1/9 ratio (B has 89% form), 100k 재시작 절충안 -> 2
- A_missing_in_C: 1/9 ratio (C has 89-90%) -> 1
- 역방향 B-only: 0.0392 switch threshold (E4 value reused)
- 역방향 C-only: 89-90% range, E3·E4 방향 불일치 claim, 첫 턴 과소집계 가설

## T2 hedges
- A: 3 (판단할 수 없 1, 가능성 2)
- B: 8 (판단 불가 1, 추정 2, 불확실 2, 라면 1, 가능성 1, 일 수 있 1)
- C: 11 (판단할 수 없 1, 추정 5, 불확실 2, 라면 2, 미확인 1)
- 유지율 (A points kept): 판단 불가 B✓ C✓; 컨텍스트 원인 가능성 B✓ C✓; 지연 차이 가능성 B✗ C✗ -> B 2/3=67%, C 2/3=67%

## T2 recommendation vs A
- A: 판단 불가; 상주 유지; resident/fresh 필드; A/B 측정; 100k 초과 시 재시작(잠정)
- B: 부분. 판단 불가·상주 유지·필드·측정 일치; 100k 잠정 재시작 없음; 전환 기준 0.0392 신규 도입
- C: 일치. 판단 불가·상주 유지·필드·spawn 기록·ctx 100k 재생성 A/B (100k는 A/B 팔, 잠정 아님)

## T2 환각/패킷외 flags
- B:12 "IDE 프로젝트로 열려 있지 않아" environment claim, not in packet
- C:24 "ctx 임계(예: 100k)" 100k not in packet (A:L9 also "100k" as example, not in packet)
- C:14 "약 89–90%": opus 87%, haiku 89% -> 90% not in packet
- A:9 / C:15 "cache_read·출력 증가": cache_read/출력 not in packet (E4 is cost only)
- T2 percentages 89%/87%, 4.5배 (0.0392/0.0087), 11.4% verified derived OK

## totals (per cond: 누락 / 권고 / 환각)
- T1-B: FAIL 누락 2 / FAIL 권고(부분) / FAIL 환각 1(IDE)
- T1-C: FAIL 누락 3 / PASS 권고 / FAIL 환각 1(270k 원인 추정)
- T2-B: FAIL 누락 2 / FAIL 권고(부분) / FAIL 환각 1(IDE)
- T2-C: FAIL 누락 1 / PASS 권고 / FAIL 환각 1(100k, 90%)
- PASS 2 of 12 checks (T1-C 권고, T2-C 권고)
