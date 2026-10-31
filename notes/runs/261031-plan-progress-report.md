# 진행 보고 설계·오케스트레이션 계획 (261031)

status: 리뷰 반영 (261031-plan-progress-report-review.md), 실행 중
source: plan/caveman-structure.md (설계안 v7, 정본), 사용자 요청 (현재 지시, 원문 위치 미확인. notes/pastes/70ebcbbe.md는 서브에이전트 보고라 근거 아님)
scope: caveman 구조 구현 시작 전 진행 보고 경로 설계, 이전 요청(훅 재작성) 연결, S1..S6 실행 순서

## 1. 계획 유지 항목

- plan/caveman-structure.md:160 (변경 금지, 기록 전용):
  감시 대상 언어 규칙 (Q2 결정): 한국어 존댓말 검사는 haiku-tui 출력에만 적용. haiku-prompter 전달문(전달문 = 원문 + 요약, U8 = c)은 B형식 필수 키(task·paths·limits·return·lang) 검사 대상이고 존댓말 검사 제외. 검사 불가 항목은 그대로

## 2. 진행 보고 설계 (신규, 정본 미기재)

- 원천 파일: `logs/<job>.status` (haiku-runner.md:11 기존 규칙). 실행자가 1줄씩 append, job별 파일 분리, 공유 금지
- 줄 형식 (agent-read): `step: <noun phrase> / pct: <0-100>`. 본문 설명·결과 요약·한국어 문장 없음
- 실행자: haiku-scout(검색만) | haiku-runner(빌드·테스트·장시간 실행) | 소유 Haiku 에이전트(수정)
- 상한 (scout): Bash 명령 1회 ≤120s, 실행 1회 총 ≤10min. runner: 전경 명령 ≤120s, 더 긴 작업은 nohup + status 파일, 실행 1회 ≤10min 후 미완이면 반환
- 초과 시: status: partial, reason: time limit, done/remaining 목록을 notes/runs/에 저장하고 경로 반환 -> 상위가 분할 재위임
- 사용자 대면: haiku-tui(기존 파일 수정, 신설 아님)가 status 마지막 유효 줄을 Read -> 존댓말 1줄. 메인은 그 줄만 verbatim 게시
- 메인 중간 텍스트 없음. 예외: 사용자가 진행상황을 물을 때, 단계 완료 시 haiku-tui 줄 1개
- edge: 파일 없음/빈 파일 -> "아직 진행 기록이 없습니다." / pct 역행·100 초과 -> 마지막 유효값

## 3. 이전 요청 연결

- 훅 재작성 완료:
  - 기준선 커밋: 0ba338b (작업 트리 상태 저장)
  - 재작성 커밋: a916231 (haiku-tui 존댓말 검사, haiku-prompter 지시문 5키 검사 추가, 테스트를 tests/hooks로 통합)
  - 보고값: python -m unittest discover tests/hooks -> Ran 33, OK (보고 출처: notes/runs/261031-run-caveman-hook-rewrite.md, 재검증 안 함)
- 불필요 파일 삭제: 정본에 삭제 지정 파일 없음 -> 후보 없음. 삭제 전 사용자 확인 필요
- 다음 단계: S1 (아래 4절) 구현 시작

## 4. 실행 단계 (정본 §8 기준, S0..S5를 S1..S6으로 번호 재부여)

공통 규칙 (정본 §8 공통, §8 시작 절차):
- 단계마다 별도 worktree 브랜치, 별도 커밋
- worktree 기준점 = 적층: S(n) 브랜치는 S(n-1) 커밋에서 분기 (첫 단계만 커밋된 master HEAD, U6 = b)
- master 작업 트리 직접 적용 금지
- 되돌리기 = 해당 단계 커밋 revert
- 병합은 S1..S6 전부 끝난 뒤 한 번 (Q3)
- 훅은 log 모드 기본, 차단 없음 (C8)

공통 검증 명령 (사용자 지정 4종, 정본에 없음):
- go build ./...
- go vet ./...
- go test ./internal/harness/...
- python -m unittest discover tests/hooks

| 단계 | 내용 (정본) | 실행자 | 입력 | 출력 | 검증 |
|---|---|---|---|---|---|
| S1 | S0 정본: docs/claude-lean/CAVEMAN_FORMAT.md에 3.1~3.6 형식 정의 | haiku-coder | 정본 plan §3 | CAVEMAN_FORMAT.md, 커밋 1 | go build ./...; python -m unittest discover tests/hooks |
| S2 | S1 읽기 경로: haiku-scout, haiku-researcher에 R-read 템플릿 + O2 예외줄; 감시 훅 log 등록 (감시 목록 축소 안 함: a916231 기존 10종 유지) | haiku-coder | S1 산출, plan §5 O2, §6 | 에이전트 파일 2개 수정, 훅 설정 1곳, 커밋 2 | go vet ./...; python -m unittest discover tests/hooks |
| S3 | S2 쓰기 경로: haiku-coder, haiku-batch-worker, haiku-runner, haiku-checker, haiku-drafter에 R-write 템플릿 | haiku-coder | S1 산출 | 에이전트 파일 5개 수정, 커밋 3 | go build ./...; go vet ./...; python -m unittest discover tests/hooks |
| S4 | S3 지휘 경로: opus-architect에 B형식 작성 규칙 + R-decide 템플릿 | haiku-coder | S1 산출 | opus-architect 파일 수정, 커밋 4 | go build ./...; go test ./internal/harness/...; python -m unittest discover tests/hooks |
| S5 | S4 감시 확대: 감시 훅 검사 대상에 S2·S3 에이전트 추가 (설정 1곳) | haiku-coder | S2, S3 산출 | 훅 설정 수정, 커밋 5 | python -m unittest discover tests/hooks |
| S6 | S5 통합: CLAUDE.md:4 범위 한정(O1), O2 예외줄 제거 (전역 CLAUDE.md 수정은 사용자 승인 후) | haiku-coder (수정), haiku-runner (검증 실행) | S1..S5 산출, 사용자 승인 | CLAUDE.md 수정 (승인 시), 커밋 6 | 정본 §10 정합성 확인 6항 + 4종 검증 명령 |

보조 작업 (정본 외, 진행 보고용):
- haiku-tui 에이전트 파일 수정 (.claude/agents/haiku-tui.md 이미 있음, tools: Read): status 줄 -> 존댓말 1줄 변환 규칙 추가. 실행자 haiku-coder (S0.5)
- 실행 중 검증: haiku-runner가 명령 실행, 결과는 R-write check 필드에 PASS|FAIL로 기록. haiku-checker는 정본 §10 항목 대조

## 5. 열린 질문 (사용자 답 필요)

- A 결정: 진행 줄 = logs/<job>.status (step/pct), haiku-tui가 존댓말 1줄로 렌더 (2절)
- B 결정: scout는 검색만, 빌드·장시간 실행은 runner. 명령 120s, 실행 10min, 초과 시 partial + 남은 목록 notes/runs/
- C 결정: :160 유지, 기록 전용(log, C8). prompter 5키 누락 로그는 예상값, 위반 아님. 전달문 형식(:42/:126/:133) 안 바꿈
- :171 범위 결정: 훅 기존 10종 감시 유지 (회귀 없음)
- 남은 질문 (사용자): 불필요 파일 삭제 후보 확인 (정본 지정 없음)

## 6. 정합성 메모

- 정본 §10 확인 항목 중 "모든 반환 템플릿 맨 앞 status·reason 2줄"은 S2~S4 완료 후 확인
- 정본 :171 감시 대상 (haiku-scout, haiku-researcher 한정)과 훅 기존 10종 감시 범위 차이: 훅 재작성 보고에서 제기됨, 미결
- 측정·비용 재계산: 본 계획 범위 밖 (정본 §0 C9, §13)

## 7. 실행 상태 점검 (261031, haiku-scout)

- S1: done (CAVEMAN_FORMAT.md:5-67, 3.1~3.6)
- S2: R-read 템플릿 done (haiku-scout.md:21-32, haiku-researcher.md:15-29). O2 예외줄 없음: CLAUDE.md:4에 O1 이미 적용 -> O2 불필요 (S6에서 제거 대상), 추가 안 함. 훅 log 등록 done (settings.json:82-87, CAVEMAN_GUARD 기본 log)
- S3: done (R-write 5종). S4: done (opus-architect.md:26, :35-44)
- S5: done (caveman-format.py:12 TARGETS 10종). S6: CLAUDE.md:4 O1 적용됨, O2 없음 -> done
- 남은 구현: S0.5 진행 보고 (haiku-runner.md, haiku-scout.md, haiku-tui.md)
