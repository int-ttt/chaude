# 리뷰: 261031-plan-record-output.md (opus 리뷰 요약)

- 분석·측정 턴도 record에 저장 (출력창 사본). 분석 결과 파일 생성 금지는 에이전트 프롬프트·CLAUDE.md 규칙으로 따로 둔다.
- S0 추가: `[cat-switch:`·`[cat:` 신호가 TUI에 도착하는 지점을 grep으로 먼저 확인. 없으면 `/topic <이름>` 명령 또는 첫 사용자 입력 앞 20자 슬러그.
- 저장 단위: 턴 시작 시 turnStartIdx=len(blocks) 기록, doneMsg에서 blocks[turnStartIdx:]의 user·asst·note 저장; tool·result는 한 줄 요약. 한 턴 = O_APPEND 한 번.
- 로드된 블록 재기록 금지 (block.loaded 또는 로드 후 turnStartIdx=len(blocks)). 테스트 포함.
- 파일 형식 왕복 파싱: 블록마다 `<!-- rec #0042 asst 2026-10-31T14:05 cat=<주제> -->` 주석과 `## #0042 · 14:05 · <주제>` 헤더.
- index.jsonl 삭제. 번호 기준은 파일 헤더: 시작 시 최신 record 파일의 마지막 rec 번호 +1.
- 분할 빈칸: 날짜=턴 종료 로컬 날짜; A-B-A 전환 시 새 NNN 파일; 파일당 200KB 또는 답변 100개 상한 시 NNN 증가; 슬러그는 한글 유지, 공백과 /:*? → -, 30자, 빈 값은 misc.
- 테스트 주입: 시계·디렉터리. 케이스: 자정 경계, A-B-A, 한글 슬러그, 왕복 파싱, 로드 블록 재기록 금지, 재시작 후 번호 이어짐.
- D7: `/resume claude [인자]` 형식, 도움말 문구, resumeshift_test 수정 명시.
- 저장 위치: git-common-dir 상위 폴더 고정 또는 HARNESS_RECORD_DIR 재정의; 쓰기 flock. rotation·자식 교체 때 기록은 부모 TUI 한 곳에서만.
- /resume은 화면 표시만, 모델 컨텍스트에 넣지 않음. 일괄 로드 시 최근 N블록 상한.
- 비밀값이 평문으로 남을 위험 명시.
- Q1: 이번 변경 제외, CLAUDE.md에 "새 기록은 record/, notes/는 읽기 전용 이전 대상" 명시. 이전 목록: session-context.py:114-154, ledger.py, style-adapter.py:7,187, importclaude.go:156, drain.go:13, switchchild.go:247, metrics·extract_transcript 기본 경로.
- Q2: record/를 .gitignore에 추가 권고 (사용자 최종 확인 필요).
- Q3: 접두어 대신 첫 토큰이 {분석, 측정} 정확 일치 + 수동 /norec 토글.
