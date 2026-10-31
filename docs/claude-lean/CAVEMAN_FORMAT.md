# Caveman format (canonical)

source: plan/caveman-structure.md 3.1~3.6 (v7)

## 3.1 브리프 B형식 (하향)

```
task: <one-line verb phrase>
paths: <absolute path>[:<line range>], ...
ctx: E<n>: <path>:<line> / <number or quote gist>   (no inheritance, C6)
limits: <paths not to edit>, <forbidden actions>
return: R-read | R-write | R-decide, <cap (chars or lines)>
lang: caveman | ko   (ko = summary_ko required, rule_4)
```

- ctx의 E<n> 근거 행 형식은 이전 정의를 재사용
- 사용자 원문·한국어 값은 ctx 안에서도 verbatim

## 3.2 읽기 반환 R-read

```
claim: <path>:<line> | <fact or verbatim quote> [sure|likely|unsure]
searched: <paths/patterns looked at>
not_found: <looked, confirmed absent>
not_checked: <ranges not looked at>
truncated: yes|no ; omitted: <omitted items or ->
summary_ko: <required only when lang: ko, 1-5 lines>
```

- not_found와 not_checked는 합치지 않음 ("없음"과 "안 봄" 구분, read-review.md:15)
- 반환 상한에 걸리면 truncated: yes와 omitted 목록을 반드시 씀 (C12)
- 브리프·R-read 값은 영어만 씀 (claim·reason·searched·not_found·not_checked). 한국어는 원문 인용 안에서만. 출력창 한국어화(요약 + 항목 번역)는 haiku-tui가 맡음 (plan/caveman-structure.md 2.1, 261101)

## 3.3 쓰기 반환 R-write

```
changed: <경로 목록>
diffstat: <git diff --stat 요지>
check: <명령> -> PASS|FAIL
ws: <작업 공간 id | none>
summary_ko: <lang: ko일 때 필수, 1~5줄>
```

- 필드 목록은 flow.md:58-61 정의를 그대로 씀

## 3.4 결정 반환 R-decide

```
decision: <한 줄>
why: <근거 E<n> 참조>
rejected: <버린 안: 이유>
risks: <항목>
open_q: <사용자만 답할 수 있는 질문 | ->
plan_path: <경로 | ->
summary_ko: <lang: ko일 때 필수, 1~5줄, L0 전달용>
```

## 3.5 caveman 허용·금지 범위

- 허용: 조사·접속어·인사·머리말 생략, key: value, 명사구, 영어 키
- 금지: 한정어 삭제(C3), 수치의 단위·대상 생략, 경로·행 생략, 확실도 표시 삭제, not_found/not_checked 병합, 한국어 값 번역·축약
- 빈 필드는 키를 지우지 않고 "-"로 둠 (필드 누락과 "해당 없음" 구분)

## 3.6 공통 상태 필드 (3.2~3.4 각 블록 맨 앞에 붙는 공통 머리 2줄)

```
status: done | partial | fail | blocked
reason: <one line when partial/fail/blocked, else ->
```

- blocked: 훅·권한 거부로 도구 사용이 막힌 경우. 거부 메시지를 verbatim 인용하고 우회 없이 반환
- fail: check FAIL 또는 작업 불가. 다시 시도한 횟수를 reason에 적음
- 메인은 status가 done이 아니면 사용자 답변에 그 상태를 한국어로 반드시 적음
