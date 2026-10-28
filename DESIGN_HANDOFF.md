# 설계 세션 인계 — claude-opus-lean 패키지

새 세션 첫 프롬프트:
> `DESIGN_HANDOFF.md`만 읽고, 패키지 파일은 필요한 것만 열어. 이전 대화를 요약하거나 결정 사항을 다시 논의하지 말고 "다음 작업" 1번부터 진행해.

> 이 문서는 패키지를 **설계·개선하는 세션**의 인계입니다. 사용자는 기존 작업 세션을 이어가지 않고 **완전 초기화**할 예정이며, 그 절차는 `FRESH_START.md`(`/distill`)입니다.

## 목표
사용자의 Claude Code 사용 패턴(프롬프트 1,152개 분석)에 맞춰 **Opus 토큰을 최소화**하는 지침 패키지. 메인은 Opus, 실제 작업은 하위 에이전트(Haiku 중심).

## 사용자 정보·선호
- 존댓말 응답. macOS (`/Users/leekwangseok`), 구독 요금제 사용 추정.
- 주요 작업: 입시 서류·면접 준비(`Documents/univ`, 834턴 단일 세션), 음성 분석(`VocalAnalyze`, 장시간 작업·Colab/Lightning), 소규모 코드.
- 입력 스타일: 짧고 빠른 프롬프트(20자 미만 28%), 정정 18%("아니/근데/아"), 정보 문장 연속 입력, 같은 긴 글 반복 붙여넣기. **습관을 바꾸기보다 시스템을 맞추길 원함.**
- Haiku 5.5 성능을 신뢰함. Opus는 메인(듣기·판단)으로만.

## 확정된 결정 (근거)
| 결정 | 근거 |
|---|---|
| 메인 Opus는 작업 도구 없음 — PreToolUse 훅이 `agent_id` 없는 Read/Write/Edit/Bash/Grep/Glob/Web 차단 | 비용 = 맥락 크기 × 요청 수. 위임으로 요청 8→2회, 도구 결과가 메인에 안 쌓임 |
| 기본 Haiku(`CLAUDE_CODE_SUBAGENT_MODEL=haiku`), Sonnet은 `sonnet-analyst`(종합·해석·복잡 코드·Haiku 2회 실패)·`sonnet-reviewer`(제출 전 검토)만 | 사용자 요청. Sonnet 캐시 읽기 단가 = Opus($0.20/M)라 메인 교체 이득 작음 |
| Opus 직접 답: 5줄 이내 또는 대화 맥락 필요한 판단. 30줄+ 긴 답은 sonnet-analyst가 파일로, Opus는 경로만 | Sonnet 콜드스타트(약 4~6센트) 때문에 짧은 답 위임은 손해. 본문 옮겨 적기는 Opus 출력 재과금 |
| Haiku 80k 교대 (`haiku-context-guard.py`, 75k부터 저장만, 80k 넘을 호출 차단, 이미지는 장당 1.7k로 추정, `CONTEXT-LIMIT` 인계 후 새 에이전트. 처음 50k였으나 사진 작업이 파일 크기 추정 때문에 멈춰서 상향) | 사용자 요청. Haiku는 100k 초과 시 단가 5배 |
| 메인: `effortLevel: low`, `autoCompactWindow: 300000`(사용자 결정: 압축 횟수 최소화. 대신 턴당 읽기·캐시 만료 비용 증가. 70% 설정은 제거), Sonnet 에이전트는 frontmatter로 100k 유지, 답 5줄, 지시서 5줄, 에이전트 반환 5줄 + 상세는 `notes/runs/` | 70k 이하에선 출력·생각이 턴 비용의 절반 |
| 입력 스타일 훅(`style-adapter.py`): 상태 질의 0토큰 표시, 정보 문장 INBOX 메모(0토큰), `[ack]/[retry]/[frag]/[paste]` 태그, 응답 끝 `▶ 기본:` 규칙 | 사용자 스타일 최적화 요청. 기록 재생 결과: 0토큰 16건, 태그 약 400건 |
| 정정 훅(`correction-guard.py`): 유형 A~G 한 줄 태그, 오해 2연속 시 체크리스트 | 정정 207건 분석 |
| 완전 초기화 = 기록 파일 오프라인 추출(`extract_transcript.py`, 7천 자 조각) → Haiku `haiku-extractor` 4조각씩 병렬 → Sonnet `sonnet-analyst` 병합 → `notes/` 5개(FACTS, DECISIONS, DELIVERABLES, OPEN, LESSONS) + `REVIEW.md` 번호 질문 | 사용자 요청. 500k 세션 재개(약 $4)보다 싸고, 압축 전 원문까지 읽음 |
| `/clear` 미사용: 자동 압축(메인 300k 창) + SubagentStop 훅이 `notes/LEDGER.md`에 결과 기록 + SessionStart(startup/resume/compact) 훅이 OPEN·LEDGER 요약 주입. 오래 쉰 뒤엔 `claude`(새 세션)가 `-c`보다 쌈 | 사용자 요청 (초기화 이후 `/clear` 쓰지 않음) |
| 예전 대화 검색: `scripts/search_history.py`(현재 `~/.claude/projects/<인코딩>/` + `~/claude-archive/<프로젝트명>-*`, 사용자·답변 본문만, 모든 단어 포함, 최신순)를 Haiku `haiku-scout`가 실행. `settings.json`에 Bash 허용 규칙 추가 | 사용자 요청. 압축으로 요약된 원문도 회수 가능 |
| 일반화: 에이전트·스킬·규칙·양식은 프로젝트 중립, 프로젝트 고유 내용은 각 프로젝트 `notes/`에만. `install.sh --main opus|sonnet`, 여러 프로젝트 일괄 `fresh_start.sh`(설치→추출→기록 보관, 메시지 0개면 보관 안 함, 실행 중 Claude 확인, bash 3.2 호환) | 사용자 요청 (모든 프로젝트에 적용) |
| Windows용 별도 패키지: `win/`(install.ps1, fresh_start.ps1, tools/common.ps1, tools/merge_settings.py) + `win/build_win.py`가 공통 파일에서 `claude-opus-lean-win` 생성. 훅은 exec form(`command`=설치 시 찾은 python.exe 절대경로, `args`=`-X utf8 ${CLAUDE_PROJECT_DIR}/.claude/hooks/X.py`)이라 Git Bash/PowerShell 무관. env `PYTHONUTF8=1`, .ps1은 UTF-8 BOM+CRLF, 에이전트의 `python scripts/`는 설치 시 `py -3` 등 실제 동작 명령으로 치환. 메인 차단·Haiku 상한에 PowerShell 도구 포함 | 사용자 요청 (Windows용은 따로) |
| 업그레이드: 설치 명령 재실행 시 공통 `tools/merge_settings.py`가 CLAUDE.md 정책 블록(`<!-- claude-lean:start/end -->`) 교체·뺀 설정 제거·빈 양식 줄 삭제·구버전 파일 백업 이동·버전 기록(`VERSION` → `.claude/claude-lean.version`) | 사용자 요청 (이미 적용된 상태 개선) |
| `/distill` 개선: 에이전트 수=ceil(조각/4)로 고르게 분배, 병합 시 빈 양식 줄 삭제, 공식 항목 수는 haiku-checker가 센 값, 에이전트 최종 메시지에 목록·파일 내용 금지 | 실사용(TextGame) 로그에서 메인 맥락 24k·개수 불일치 발견 |
| 압축 없는 리셋: 압축은 대화 전체 재전송이라 사용량 큼(공식 문서), Haiku로 압축 불가, 맥락만 비우는 방법은 `/clear`·압축뿐 → `/exit` 후 같은 터미널에서 `claude`(화면 기록 유지). `context-alert.py`가 150k(`LEAN_RESET_AT`)에서 안내(25k마다), `session-context.py`가 startup에 직전 세션 마지막 15개 메시지 주입, `history-export.py`(SessionEnd·PreCompact)가 `logs/history/*.txt` 저장. `idleCompaction`(200k+·54분 유휴) 기본값 유지 | 사용자 요청: 터미널 기록 유지 + 압축 비용 없음. md는 뷰어에서 느려 txt로 |
| 에이전트 이름에 모델 접두어(`haiku-*`, `sonnet-*`) — 화면·`/list-agents`에 모델 칸이 없어서. `agent-model-pin.py`(PreToolUse `Agent|Task`)가 호출 시 `model` 덮어쓰기를 지우고(파일의 `model:` 우선), 기본 제공 general-purpose/Explore/Plan/claude-code-guide는 haiku로(haiku·sonnet 지정은 유지), 사용자 에이전트는 그대로. `ledger.py`가 에이전트 기록의 `message.model`을 `[이름 · haiku 5.5]`로 남기고 접두어와 다르면 `⚠ 이름과 다름`. 업그레이드 시 예전 이름 파일은 description이 `Haiku.`/`Sonnet.`으로 시작할 때만 백업으로 이동 | 사용자 질문: 에이전트가 어떤 모델로 도는지 표시가 안 됨 |
| 일반 모드 스위치: `일반모드`/`절약모드` 입력(UserPromptSubmit block, 모델 호출 없음)이 `.claude/lean-off`를 만들고 지움. 세션 단위는 `CLAUDE_LEAN=0 claude`. 꺼져 있으면 메인 차단·Haiku 상한·모델 고정·정정·리셋 안내 훅이 멈추고 매 입력에 `[normal]` 태그, SessionStart는 일반 모드 한 줄만 주입. 다시 켜진 첫 입력에 `[lean]`. 대화 기록 저장·LEDGER는 계속. 판별은 공용 `_lean.py`. 전역 자동 적용은 하지 않음 | 사용자: 일반적인 사용을 해야 할 때도 있음 |
| 라우팅(Sonnet 우선, 빠른 넘김): 단순 → Haiku, 판단 필요 → `sonnet-analyst`가 파일 3개 이내로 범위 파악 후, 설계 후보 2개+ 근거 없음·요구 충돌·얽힌 파일 5개+·자기 검증 1회 실패면 구현 없이 `ESCALATE: <정리본>` → 메인이 그대로 `opus-architect`(opus, effort high). `opus로`·같은 정정 2회·이미 실패한 일은 바로 architect. architect는 작은 구현은 직접, 아니면 계획서 → 사용자 확인 → Sonnet 구현 → reviewer. Sonnet·Opus 에이전트에 `Agent` 도구를 주고 `agent-model-pin.py`가 하위 에이전트의 호출을 `haiku-*`로만 제한; 중첩 호출이 안 되는 버전이면 결과 파일 `## [haiku] steps`를 메인이 병렬 배분. (처음엔 Opus가 복잡도 신호를 세는 안이었으나, Opus는 파일을 못 보고 판단하므로 파일을 본 Sonnet이 판단하도록 변경) | 사용자 요청: Sonnet에서 오래 붙잡지 말고, Sonnet이 Haiku에 단순 작업 연계 |
| 답변 방식: 일반 Opus처럼 자연스러운 문장, 비유 없이 실제 방법을 쉽게(사용자 프롬프트 기록: "너무 쉬운 비유를 들지 말고 실제 방법을 풀어서"), 용어 풀이, 숫자의 기준 명시, 줄 수 제한 없음. 설명은 Opus가 직접, 보관 문서만 Sonnet 파일. 에이전트 반환은 쉬운 말 8줄 이내 | 사용자: 5줄 보고가 이해되지 않음, 예전 Opus 단독 때처럼 |
| 설치 선택사항: `tools/configure.py`(tkinter 창, 안 되면 터미널) → `CLAUDE_LEAN_OPTIONS` → `merge_settings.py`가 적용(모델, 답변 방식 줄 교체, STYLE_INBOX, LEAN_RESET_AT/알림 훅 제거, 기록 훅 제거, HAIKU_CONTEXT_CAP, SPAWN_DEPTH, 시작 모드) 후 `.claude/claude-lean.options.json` 저장, 다음 설치 때 재적용(시작 모드는 고를 때만). 릴리스: `.github/workflows/release.yml` + `tools/build_release.py`(태그=VERSION 검사). 대화 기록 `history/` | 사용자 요청: GitHub 공유, 릴리스, 선택사항을 설치 시 GUI로 |
| 플러그인: caveman·AgentOne·token-saver 비권장, RTK는 코드 프로젝트 선택, 측정 플러그인 권장 | 이미 겹치는 규칙, 존댓말 충돌, 외부 전송 위험 |

## 문서로 확인한 사실 (code.claude.com/docs)
- 하위 에이전트 frontmatter: `model`, `effort`, `omitClaudeMd`, `autoCompactWindow`(100000~1000000, v2.1.296+), `tools`, `maxTurns` 등.
- 모델 결정 순서: 호출 시 지정 → frontmatter → `CLAUDE_CODE_SUBAGENT_MODEL` → 메인 모델. Explore는 메인 모델 사용.
- 스킬 `context: fork` + `agent:` → 하위 에이전트로 실행(대화 이력 없음). 스킬 `model`은 그 턴에만 적용.
- 훅 입력에 `agent_id`/`agent_type`(하위 에이전트일 때). PreToolUse `permissionDecision: deny`. UserPromptSubmit `decision: block` → 모델에 안 감, `reason`은 사용자에게만 표시, 프롬프트 교체 불가.
- 하위 에이전트 기록: `~/.claude/projects/{project}/{sessionId}/subagents/agent-{id}.jsonl`.
- `effortLevel`(low~xhigh), Opus 5.5 기본 medium, 5.5 계열은 생각을 끌 수 없음. `ultrathink`로 턴 단위 상향.
- env: `CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS`(기본 25000), `BASH_MAX_OUTPUT_LENGTH`(기본 30000), `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`(메인·하위 모두 적용).
- 단가(백만 토큰당, 입력/캐시 읽기/출력): Opus 5.5 $4/$0.20/$20, Sonnet 5.5 $2/$0.20/$10, Haiku 5.5 $0.10/(추정 $0.01)/$0.50(100k 초과 5배). 캐시 저장 1.25×(5분)/2×(1시간).

## 검증 상태
- **시험 완료(스크립트 단독):** 6개 훅 전부, `search_history.py`, `charcount.py`, `usage_report.py`·`extract_transcript.py`(가짜 기록), `install.sh`·`fresh_start.sh`(리눅스, 기존 설정 병합·재실행·여러 프로젝트·기록 없음·폴더 없음), 기존 프롬프트 1,152개 재생 분류.
- **미검증:** 실제 Claude Code 세션에서의 동작(훅 등록·차단·태그 반응, SessionStart matcher `startup|resume|compact` 값), `install.sh`의 macOS 실행, 실제 대화 기록 형식에서의 `extract_transcript.py`(필드명 가정: type/message/content/timestamp/isCompactSummary), `/distill` 전체 흐름, 실사용 토큰 절감 수치(현재는 단가표 기반 추정: 턴당 약 $0.96 → $0.08).
- **Windows 미검증:** PowerShell 스크립트(install.ps1, fresh_start.ps1, common.ps1)는 이 환경에서 실행 불가(PowerShell 설치가 권한 정책으로 차단)라 실행 시험 못 함. `merge_settings.py`와 빌드는 시험 완료. exec form 훅(`args`)은 최신 Claude Code 필요.
- **미검증(2026.10.28):** `agent-model-pin.py`의 `updatedInput`(호출 입력 변경)이 실제 세션에서 적용되는지, SubagentStop 입력의 `agent_transcript_path` 필드명(없으면 `agent_id`로 기록 파일 검색).
- **시험 완료(실제 Claude Code 2.1.312, `claude -p`):** 중첩 호출 — 공식 문서상 기본 3단계까지 허용, `tools:`에 `Agent` 필요, 깊이는 `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`(패키지 2). 환경 변수가 1이어도 settings env가 우선해 sonnet-analyst → haiku-scout 성공, general-purpose 호출은 `agent-model-pin.py`가 차단, PreToolUse는 하위 에이전트 안에서도 동작. LEDGER가 중첩 에이전트 모델도 기록(기록 지연 대비 재시도 + 설정 파일 대체).
- **알려진 한계:** INBOX 분류는 정규식 기반이라 오분류 가능(`!`로 우회, `STYLE_INBOX=0`으로 끔). 컨텍스트 가드의 도구 결과 크기는 추정치.

## 파일 지도
| 파일 | 역할 |
|---|---|
| `CLAUDE.md` | 메인 정책 (약 3.1KB) |
| `.claude/settings.json` | 모델·effort·압축·env·훅 10개 |
| `.claude/agents/*.md` | `haiku-*` 8개(haiku-extractor 포함), `sonnet-*` 2개, `opus-architect` 1개. 이름 앞부분 = 모델 |
| `.claude/hooks/main-thread-guard.py` | 메인 도구 차단 |
| `.claude/hooks/haiku-context-guard.py` | Haiku 80k 교대 |
| `.claude/hooks/agent-model-pin.py` | 에이전트 모델 고정 (호출 시 덮어쓰기 제거, 기본 제공 에이전트는 Haiku) |
| `.claude/hooks/_lean.py` | 일반 모드 판별 (훅 공용) |
| `.claude/hooks/style-adapter.py` | 입력 스타일 최적화 |
| `.claude/hooks/correction-guard.py` | 정정 유형 태그 |
| `.claude/hooks/ledger.py`, `session-context.py` | 에이전트 결과·실제 모델 자동 기록, 압축 직후·새 세션에 상황·직전 대화 주입 |
| `.claude/hooks/context-alert.py`, `history-export.py`, `scripts/export_history.py` | 150k 리셋 안내, 대화 기록 txt 저장 |
| `.claude/skills/{status,check,fact,research}` | Haiku fork 바로가기 |
| `.claude/skills/{correction,migrate-session,distill}` | 정정 처리, 세션 이어가기(대안), 완전 초기화 |
| `scripts/charcount.py`, `scripts/usage_report.py`, `scripts/search_history.py` | 글자 수, 실사용량 집계, 예전 대화 검색 |
| `install.sh`, `fresh_start.sh`, `APPLY_GUIDE.md` | 설치·여러 프로젝트 일괄 초기화·적용 (Mac) |
| `win/` , `win/build_win.py` | Windows 패키지 원본과 빌드 (`python3 win/build_win.py` → `../claude-opus-lean-win`, Mac 표현이 남으면 실패) |
| `scripts/claude_history.py` | 프로젝트의 대화 기록 폴더 찾기 (폴더명 규칙 → 실패 시 기록 안의 `cwd`로) |
| `FRESH_START.md`, `.claude/skills/distill`, `.claude/agents/haiku-extractor.md`, `scripts/extract_transcript.py` | 완전 초기화 (권장) |
| `MIGRATE_NOW_PROMPT.md`, `templates/` | 세션 이어가기 (대안) |
| `ANALYSIS.md`, `README.md` | 분석·개요 |

## 다음 작업
1. 사용자가 `FRESH_START.md` 2단계 추출 결과(INDEX.md)와 `/distill` 결과를 보여 주면 추출·병합 품질 조정.
2. 사용자가 `univ`에 설치 후 받은 결과(`/agents`, `/hooks`, 첫 몇 턴 반응)를 보고 훅·규칙 수정.
3. `usage_report.py` 적용 전/후 결과를 받아 실제 절감량 확인, 추정치 갱신.
4. INBOX·`[frag]` 오분류 사례를 받아 `style-adapter.py` 정규식 조정.
5. Windows에서 첫 설치 결과(`install.ps1` 출력, `/hooks`, 한글 프롬프트에 대한 훅 반응)를 받아 수정.
6. 실제 세션에서 모델 고정 확인: `notes/LEDGER.md`의 `[이름 · 모델]`이 접두어와 같은지, `⚠`가 나오면 `agent-model-pin.py` 출력 형식 수정.
