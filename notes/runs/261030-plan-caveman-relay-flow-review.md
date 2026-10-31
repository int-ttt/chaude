# review: 261030-plan-caveman-relay-flow.md
date: 261030
reviewer: opus-architect
scope: 개선점만. 실행 없음. 원 계획서 수정 없음.
sources: 계획서 본문, notes/research/subagent-context-model.md:9-15, internal/harness/{core.go,children.go,orchws.go,switchchild.go,clients.go,repl.go} 코드 읽기(실행 검증 안 함)

## 1. 흐름별 판정 (계획서 판정 대비)
input_flow: 계획서 "조건부 유망" -> 동의하되 범위 축소. 압축 주체는 메인 Haiku로 고정(기존 계획 2절과 일치). opus 압축 옵션은 삭제. 기존 계획 조건 B·C와 중복이므로 별도 실험 없이 B·C에 흡수
input_tradeoff: 비용=Opus 입력 절감 작음(입력 단가 낮고 캐시 적중 시 더 작음) / 품질 위험=중(사실·경로·수치 탈락) / 난이도=하
read_flow: 계획서 "조건부 유망" -> 동의. 세 흐름 중 절감 대비 위험이 가장 낮으므로 1순위로 올림
read_tradeoff: 비용=Opus가 원본을 읽지 않아 입력 절감 큼 / 품질 위험=중(scout 누락을 Opus가 모름) / 난이도=하(현재 RETURN verbatim 규약 그대로 사용)
write_flow: 계획서 "비추천" -> "조건부 실현 가능"으로 수정 권고. 단, 반환을 요약만으로 두는 원안은 계속 비추천
write_tradeoff: 비용=Opus 출력 절감 큼(단가 최고 항목) / 품질 위험=상(요약만 반환 시 검증 불가) / 난이도=중(하네스 mode f 재사용 시) 또는 상(새 클라이언트 구현 시)

## 2. 쓰기 흐름 "별도 클라이언트 맥락 유지" 실현 가능성
via_subagent_normal: 불가. 일반 서브에이전트는 매 호출 새 컨텍스트로 시작(research:9). 현재 Agent 도구는 이전 에이전트 이어쓰기 불가로 명시됨. 따라서 Agent 경로로는 쓰기 에이전트 맥락이 호출 간 유지되지 않음
via_fork: 불가. fork는 부모 대화를 상속하지만(research:11) 모델도 부모를 따라감(Agent 도구 설명: fork는 항상 부모 모델 상속). Haiku로 내릴 수 없고, 부모 전체 맥락을 상속하므로 토큰 절감도 없음. fork 호출 방식 본문은 미확인(research:11 unverified)
via_frontmatter_model: 모델 지정은 가능(frontmatter 또는 호출 시 별칭, research:12-14). 다만 이는 모델 문제만 해결하고 맥락 유지 문제는 해결하지 않음
via_harness_mode_f: 가능, 이미 구현됨. 하네스가 claude -p 자식을 stream-json 입출력으로 띄우고 --resume <session_id>로 세션을 이어감(core.go:145-152). mode f에서 [ws:<id>] 태그 줄이 haiku로 라우팅되고 저장된 ws 세션이 있으면 --resume로 재개(orchws.go:78-101). 회전 기준 20턴 / 40k 토큰 / 유휴 270초(orchws.go:31-43), 인계 상한 1500(orchws.go:35)
conclusion: "별도 Claude Code 클라이언트"를 새로 만들 필요 없음. 쓰기 흐름은 mode f ws 경로를 쓰기 작업 단위로 재사용하는 안으로 다시 써야 함. 단발 작업은 기존 haiku-coder 일회성 호출 유지
doubt_1: 위 동작은 코드 읽기 기준. 실제 재사용/retire 동작은 실행 확인 안 됨
doubt_2: mode f는 --clients-file 설정 시에만 live 자식을 둠(repl.go:204-208). 기본 실행에서 켜져 있는지 확인 필요
doubt_3: Opus(Agent 도구만 보유)가 [ws:<id>] 줄을 직접 낼 수 없음. 라우팅 주체는 메인 세션/하네스여야 함. 계획서에 이 경계가 빠져 있음

## 3. 개선점 (우선순위 순)
imp_1 (쓰기 흐름 재작성): 3.3절을 "mode f ws 재사용"안으로 교체. 반환 필수 필드 = 변경 파일 경로 목록, git diff --stat, 검사 명령과 PASS/FAIL, ws id. "변경 요약과 방법 맥락만" 문구 삭제. risk_2(세션 수·캐시 미스)는 ws 회전 기준(270초 유휴 < 5분 캐시 TTL)으로 이미 제한되므로 근거와 함께 하향
imp_2 (읽기 흐름 상한): "줄 수 제한 없음"을 "문자 상한 + 근거 형식 고정"으로 교체. 예: RETURN verbatim <=4000자, 각 주장에 경로:행 필수, 미발견 항목 명시. 별도 "토큰 절감 번역" 단계는 삭제하고 scout가 처음부터 terse English key: value로 반환(중계 1단계 감소)
imp_3 (입력 흐름 주체 확정): 압축 주체 = 메인 Haiku 고정, opus 선택지 삭제. 3.1절은 기존 계획 조건 B·C 참조로 대체하고 중복 실행 금지 명시
imp_4 (제약 충돌 처리): 기존 계획 10절 "에이전트 호출은 opus-architect만"과 읽기·쓰기 흐름 충돌을 사용자 결정 항목으로 분리 표기. 개정안 문구 후보: "Opus가 직접 읽거나 쓰지 않는다. 읽기는 haiku-scout, 쓰기는 haiku-coder 또는 mode f ws"
imp_5 (측정 기준 누락): 5절 limit이 "측정 데이터 없음"으로 끝남. 채택 판정 지표를 notes/metrics/*.jsonl과 turnspend 기준으로 정의할 것. 최소: 흐름별 Opus 입력/출력 토큰, Haiku 토큰 합, 재작업 횟수, 근거 누락 건수(haiku-checker 표본 검사)
imp_6 (순서): 실행 순서를 읽기 -> 쓰기(mode f 재사용) -> 입력(B·C 흡수)으로 변경. 계획서 next_3 "쓰기 보류"는 mode f 확인 후 해제 가능으로 수정

## 4. 바꾸지 말 것
keep: 계획서 2절 충돌 기록, 4절 "출력 절감 효과가 가장 큼" 판단, 5절 "중계 단계 3개 이상이면 상쇄" 경고
