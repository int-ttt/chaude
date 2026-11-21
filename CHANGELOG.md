# Changelog

## Unreleased

- clean: `chaude reset` now deletes notes/, record/ and logs/ too, and inside `.chaude/` keeps only the history files (`history*`); the rest (settings, lock, other files) is removed. `--with-notes` is accepted as a hidden no-op.
  - clean: `chaude reset`이 이제 notes/, record/, logs/도 삭제하고, `.chaude/`에서는 히스토리 파일(`history*`)만 남기고 나머지(설정, 잠금 파일 등)를 지웁니다. `--with-notes`는 숨겨진 무동작 별칭으로 계속 받습니다.
- hooks: projects set up by chaude (no `.claude/settings.json` yet, or merged into one) now also get the SessionStart operating index (`session-context.py`); the main-session block message names all blocked tools and the routing to use instead.
  - hooks: chaude가 설정한 프로젝트(`.claude/settings.json`이 없거나 병합되는 경우)에도 SessionStart 운영 색인(`session-context.py`)이 설치되며, 메인 세션 차단 메시지가 차단된 도구 전부와 대신 쓸 라우팅을 안내합니다.

## 0.13.2 - 2026-11-21

- clean: `chaude reset` fully resets the current folder (removes `.claude/`, the shipped scripts, the CLAUDE.md block and its recent-folders entry; asks first, `--yes`, `--dry-run`, `--with-notes` keeps notes/record/logs only when given).
  - clean: `chaude reset`이 현재 폴더를 완전히 초기화합니다(`.claude/`, 배포된 스크립트, CLAUDE.md 블록, 최근 폴더 항목을 제거하며 먼저 묻습니다. `--yes`, `--dry-run`, `--with-notes`는 주어질 때만 notes/record/logs를 지웁니다).
- tui: a colored span that wraps keeps its color on every row, including a color opened in the grey tool-result lead.
  - tui: 색이 들어간 줄이 줄바꿈되면 모든 줄에서 색이 유지됩니다. 회색 도구 결과 머리글에서 연 색도 포함합니다.
- hooks: at session start, with lean on, a static operating index (who handles what, routing, state files) comes first in the context; it is byte-identical every session so the prompt cache holds.
  - hooks: lean이 켜진 세션 시작 시 컨텍스트 맨 앞에 고정 운영 색인(역할 분담, 라우팅, 상태 파일)을 보여 줍니다. 매 세션 바이트가 같아 프롬프트 캐시가 유지됩니다.

## 0.13.0 - 2026-11-20

- gui: the HTML GUI (`chaude-gui`) is now included in the release archives, next to `chaude`. The installers install it when present.
  - gui: HTML GUI(`chaude-gui`)가 이제 릴리스 아카이브에 `chaude`와 함께 들어갑니다. 설치 스크립트는 있으면 함께 설치합니다.
- gui: a desktop completion notification when a turn ends while the GUI window is in the background.
  - gui: GUI 창이 백그라운드에 있을 때 턴이 끝나면 데스크톱 완료 알림을 보냅니다.

## 0.12.11 - 2026-11-17

- tui: an agent-done note no longer splits the reply into two blocks; it stays inside the same reply block.
  - tui: 에이전트 완료 알림이 답변 중간에 끼어도 답변이 두 블록으로 나뉘지 않고 한 블록으로 유지됩니다.
- core,tui,gui: the shipped hooks, agents and scripts refresh per project (stamp `<project>/.claude/.chaude-version`, not one global record), so every project launched after an update is refreshed, and a startup line says what changed (`hooks updated to 0.12.10: 4 hooks, 12 agents (_lean.py, …)`). A project with nothing to update shows no line.
  - core,tui,gui: 공유 훅·에이전트·스크립트가 프로젝트별로 갱신됩니다(전역 기록 하나가 아니라 `<프로젝트>/.claude/.chaude-version` 표식 사용). 업데이트 후 처음 여는 모든 프로젝트가 갱신되며, 시작 시 무엇이 바뀌었는지 한 줄로 알려 줍니다(`hooks updated to 0.12.10: 4 hooks, 12 agents (_lean.py, …)`). 바뀐 것이 없으면 줄을 표시하지 않습니다.

## 0.12.10 - 2026-11-17

- core: a session with no transcript is no longer resumed at startup (claude exits at once on `--resume` for a session that never ran a turn), which fixes "claude exited" on launch.
  - core: 전사(transcript)가 없는 세션은 시작할 때 이어 쓰지 않습니다(한 턴도 돌지 않은 세션에 `--resume`을 주면 claude가 바로 종료해 시작할 때 "claude exited"가 나왔습니다).
- core: when claude exits, the error shows its exit status and the last lines of its stderr (`claude exited (exit status: 1: ...)`) instead of a bare "claude exited".
  - core: claude가 종료하면 오류에 종료 상태와 stderr 마지막 줄이 함께 나옵니다(예: `claude exited (exit status: 1: ...)`). 이전에는 "claude exited"만 보였습니다.
- tui: a usage meter refresh no longer reopens a /usage window you closed.
  - tui: 사용량 미터 갱신이 닫아 둔 /usage 창을 다시 열지 않습니다.
- gui: the sidebar folder rows no longer list past sessions under them (no sub-rows and no "more" toggle), and opening a folder no longer reads its session files. Chat rows and folder rows still open as before.
  - gui: 사이드바의 폴더 행 아래에 지난 세션이 나열되지 않습니다(하위 항목과 "more" 토글 없음). 폴더를 열 때 세션 파일을 읽지 않습니다. 채팅 행과 폴더 행은 전과 같이 열립니다.
- gui: New chat makes its scratch dir, sidebar entry and recent record only when the first non-empty message is sent; an empty New chat just shows the centered composer. The open folder's past sessions list only the user's top-level sessions (sub-agent, empty and harness-sent ones such as rotation carry-overs, prompter summaries and handoff notes are left out), the newest 5 with a "more" toggle, each row showing its first prompt (truncated) and date.
  - gui: 새 채팅은 첫 메시지(공백 제외)를 보낼 때에만 임시 폴더, 사이드바 항목, 최근 기록을 만듭니다. 빈 새 채팅은 가운데 입력창만 보여 줍니다. 열린 폴더의 지난 세션은 사용자의 최상위 세션만 나열합니다(하위 에이전트, 빈 세션, 회전 이어쓰기·프롬프터 요약·인수인계 노트 같은 하네스가 보낸 세션은 제외). 최근 5개를 보여 주고 "more"로 펼치며, 각 행에는 첫 프롬프트(잘림)와 날짜가 보입니다.
- gui: the code session header shows the folder path (home shortened to ~, the full path on hover, a click copies it), and each sidebar folder row shows its path. The open folder lists its past sessions under its row, and a past chat row opens its scratch dir; clicking a past session loads its transcript into the chat and resumes it in the same folder or chat dir, so the mode follows.
  - gui: 코드 세션 헤더에 폴더 경로가 보입니다(홈은 ~로 줄이고, 마우스를 올리면 전체 경로, 누르면 복사). 사이드바의 폴더 행에도 경로가 보입니다. 열린 폴더 행 아래에는 그 폴더의 지난 세션이 나열되며, 지난 채팅 행을 누르면 그 채팅 임시 폴더가 열립니다. 지난 세션을 누르면 대화 내용을 보여 주고 같은 폴더(또는 채팅 임시 폴더)에서 이어서 시작하므로 모드도 따라갑니다. 새 bridge 명령 `open_session`(core `open_plan`, `past_sessions`)을 씁니다.
- gui: the sidebar foot shows the usage meter (session and week percentages with the session's reset time, from the same /usage numbers the TUI shows); a click opens the /usage window. It refreshes at start, every 5 minutes and after each turn, with at most one background /usage run at a time; `HARNESS_USAGE_POLL=0` turns the background runs off. The GUI palette is now claude-like: a warm off-white light theme, a warm dark gray dark theme, a terracotta accent, a rounded composer card, and a muted sidebar.
  - gui: 사이드바 아래쪽에 사용량이 항상 보입니다(세션과 주간 사용률, 세션 초기화 시각; TUI가 보여 주는 /usage 값과 같음). 누르면 /usage 창이 열립니다. 시작할 때, 5분마다, 턴이 끝날 때 갱신되며, 백그라운드 /usage 실행은 한 번에 하나만 돕니다. `HARNESS_USAGE_POLL=0`이면 끕니다. GUI 색상은 Claude 데스크톱과 비슷하게 바뀌었습니다: 따뜻한 밝은 배경, 따뜻한 어두운 회색 다크 테마, 테라코타 강조색, 둥근 입력 카드, 차분한 사이드바.
- gui: the GUI has a left sidebar (New chat, Open folder, the recent folders and the chat scratch dirs, and a collapse toggle); the start screen is a centered greeting with a composer, and typing there starts a New chat. Clicking a recent folder opens it; a chat row only shows for now (resuming a chat is not in core yet).
  - gui: GUI에 왼쪽 사이드바가 생겼습니다(새 채팅, 폴더 열기, 최근 폴더와 채팅 임시 폴더 목록, 접기 버튼). 시작 화면은 가운데 인사말과 입력창이며, 여기에 입력하면 새 채팅이 시작됩니다. 최근 폴더를 누르면 그 폴더가 열리고, 채팅 항목은 당분간 표시만 하며 채팅 이어 쓰기는 아직 core에 없습니다.
- gui: the GUI opens on a start screen with New chat, Open folder (a native folder dialog) and the recent folders. Open folder starts the full harness in that folder; New chat starts a chat with no file or shell tools and no harness agents, in a scratch dir under `~/.chaude/chat/`. The header shows the mode, the folder name, the model, the context and the rate; a finished reply sends an OS notification.
  - gui: GUI는 이제 시작 화면에서 새 채팅, 폴더 열기(네이티브 폴더 선택 창), 최근 폴더를 고릅니다. 폴더 열기는 그 폴더에서 전체 하네스를 시작하고, 새 채팅은 파일·셸 도구와 하네스 에이전트 없이 `~/.chaude/chat/` 아래 임시 폴더에서 채팅을 시작합니다. 헤더에는 모드, 폴더 이름, 모델, 컨텍스트, 속도 제한이 표시되며, 답변이 끝나면 OS 알림을 보냅니다.
- gui: the GUI shows the chat: a centered column with the user and assistant text as sanitized markdown, code blocks with a copy button, collapsible tool calls with their results, muted notes and stage rows, streaming text updated in place, a composer (Enter sends, Shift+Enter adds a line, Enter is ignored while an IME composition is open), an interrupt button while a turn is busy, and a permission card with Allow and Deny. Copy writes the OS clipboard. The page libraries (marked, DOMPurify, highlight.js) are vendored with their licenses in `crates/gui/web/vendor/LICENSES.md`. The reply after a turn with residents now shows in the chat (it was missing).
  - gui: GUI는 이제 대화를 보여 줍니다. 가운데 정렬된 열에 사용자와 어시스턴트 텍스트를 정제된 마크다운으로 표시하고, 코드 블록에는 복사 버튼이 있으며, 도구 호출은 결과와 함께 접을 수 있고, 메모와 단계 행은 흐리게 표시합니다. 스트리밍 텍스트는 제자리에서 갱신되며, 입력창은 Enter로 보내고 Shift+Enter로 줄을 바꾸며, IME 조합 중의 Enter는 무시합니다. 턴이 진행 중이면 중지 버튼이 보이고, 권한 카드에는 허용과 거부 버튼이 있습니다. 복사는 OS 클립보드에 씁니다. 페이지 라이브러리(marked, DOMPurify, highlight.js)는 라이선스와 함께 `crates/gui/web/vendor/LICENSES.md`에 포함했습니다. 레지던트 사용 시 턴의 응답이 이제 대화에 표시됩니다(이전에는 빠졌습니다).
- gui: the GUI is now one native window (tao + wry) that loads an embedded HTML page, replacing the egui window. The page and the Rust side exchange a ping both ways as the first bridge step; a second launch focuses the running window and exits. The egui GUI was never released; its code is removed.
  - gui: GUI는 이제 임베디드 HTML 페이지를 띄우는 네이티브 창 하나(tao + wry)이며, egui 창을 대체합니다. 페이지와 Rust가 첫 브리지 단계로 ping을 양방향으로 주고받으며, 두 번째 실행은 실행 중인 창을 앞으로 가져온 뒤 종료합니다. egui GUI는 출시된 적이 없어 그 코드를 제거했습니다.
- gui: a drag-select in the transcript copies the selected text when the mouse is released, and shows the `copied (N chars)` note for 2s on the status row, as the TUI does. The GUI copies through the native clipboard; the copy-failed note is not shown (the GUI does not see a clipboard error).
  - gui: 트랜스크립트에서 드래그로 고른 텍스트는 마우스를 놓을 때 복사되며, TUI처럼 상태 행에 `복사됨 (N자)` 안내를 2초 동안 표시합니다. GUI는 네이티브 클립보드로 복사하며, 복사 실패 안내는 표시하지 않습니다(GUI는 클립보드 오류를 알 수 없습니다).
- gui: `/view` level 2 scrolls the prompt body with up/down and k/j (one row), PageUp/PageDown (a window) and Home/End (the ends); left/right resets the scroll. The previous-session mode note (`previous session: ...`) shows at start and on each `/view`, as in the TUI. The GUI now starts the engine with the project handoff file (`notes/tmp/long`), so the previous run resumes or starts fresh the same way as in the TUI.
  - gui: `/view` 2단계에서 위/아래와 k/j는 한 줄, PageUp/PageDown은 한 화면, Home/End는 처음과 끝으로 본문을 스크롤하고, ←/→는 스크롤을 처음으로 되돌립니다. 이전 세션 모드 안내(`previous session: ...`)는 시작할 때와 `/view`마다 TUI처럼 표시됩니다. GUI는 이제 프로젝트 handoff 파일(`notes/tmp/long`)로 엔진을 시작하므로, 이전 실행을 TUI와 같은 방식으로 이어 가거나 새로 시작합니다.
- gui: `/import [id|latest|a,b]` reads the Claude Code transcript into the view as the TUI does (without the /resume digest) and installs it for the next prompt; the "not in the GUI yet" note for /import is gone.
  - gui: `/import [id|latest|a,b]`는 TUI처럼 Claude Code 기록을 트랜스크립트에 읽어 보여 주고(/resume의 요약 기록 없이), 다음 프롬프트를 위해 엔진에 설치합니다. /import의 "GUI에 아직 없음" 안내는 없어졌습니다.
- gui: Esc Esc on an empty input box (no turn busy) opens a rewind window over the prompts; Up/Down (or k/j) pick, Enter or a click rewinds: the transcript is cut before that prompt and its text goes back into the input box; Esc, q, Ctrl+C or X cancels.
  - gui: 입력 상자가 비어 있고 턴이 돌지 않을 때 Esc를 두 번 누르면 프롬프트 목록의 되감기 창이 열립니다. Up/Down(또는 k/j)으로 고르고 Enter 또는 클릭하면 그 프롬프트 앞에서 트랜스크립트를 자르고 프롬프트 문구를 입력 상자에 되돌립니다. Esc, q, Ctrl+C 또는 X로 취소합니다.
- gui: bare `/resume` opens a "Resume a project" window over the record folders of this project (Up/Down or k/j move, 1-9 select, Enter or a click loads the folder into the transcript like the TUI, Esc, q, Ctrl+C or X cancels). `/resume` with arguments that no record folder answers opens a "Resume a session" window over this project's Claude Code session groups (Up/Down or k/j move, 1-9 select, Enter or a click imports the group as `/resume claude` does, Esc, q, Ctrl+C or X cancels). A bare `/resume` after a killed run imports that killed session. The picker model and the import are the shared core ones.
  - gui: 인수 없이 `/resume`을 치면 이 프로젝트의 기록 폴더 목록 "Resume a project" 창이 열립니다(Up/Down 또는 k/j로 이동, 1-9로 고르기, Enter 또는 클릭하면 TUI처럼 그 폴더를 트랜스크립트에 불러옴, Esc, q, Ctrl+C 또는 X로 취소). 기록 폴더가 답하지 않는 인수를 붙인 `/resume`은 이 프로젝트의 Claude Code 세션 그룹 목록 "Resume a session" 창을 엽니다(Up/Down 또는 k/j로 이동, 1-9로 고르기, Enter 또는 클릭하면 `/resume claude`처럼 그 그룹을 가져옴, Esc, q, Ctrl+C 또는 X로 취소). 중단된 실행 뒤의 인수 없는 `/resume`은 중단된 세션을 가져옵니다. 목록 모델과 가져오기는 공용 코어를 씁니다.
- gui: /view opens the session browser window: the session list (level 1, with the active mark and sub-agents), enter or a click opens a session's prompts (level 2), left/right step through them, esc goes back, and esc, q or Ctrl+C closes. A /resume that switches the session reopens the list. The model is the shared core one, as in the TUI.
  - gui: /view는 세션 브라우저 창을 엽니다. 1단계는 세션 목록(활성 표시와 하위 에이전트 포함), Enter나 클릭으로 세션의 프롬프트를 열고(2단계), 좌/우로 넘기며, Esc로 목록에 돌아가고, Esc·q·Ctrl+C로 닫습니다. 세션이 바뀌면(/resume) 목록이 다시 열립니다. 모델은 TUI와 같은 코어 공용 모델입니다.
- gui: /usage, /lang and /settings open their own windows instead of "not in the GUI yet": /usage shows the one-shot run text with usage bars (Esc, q, Enter, Ctrl+C or X closes); /lang picks ko or en with 1/2, arrows or click, and `/lang CODE` saves and switches the UI; /settings edits the three project settings rows and saves on close. The picker models and the usage text are the shared core ones, as in the TUI.
  - gui: /usage, /lang, /settings가 GUI 미지원 안내 대신 자기 창을 엽니다. /usage 창은 한 번 실행한 결과 텍스트를 사용량 막대와 함께 보여주며(Esc, q, Enter, Ctrl+C 또는 X로 닫음), /lang은 1/2, 화살표 또는 클릭으로 ko·en을 고르고 `/lang CODE`는 저장 후 UI를 바꿉니다. /settings는 프로젝트 설정 세 행을 고치고 닫을 때 저장합니다. 창 모델과 사용량 문구는 TUI와 같이 공용 코어를 씁니다.
- gui: queued prompts show above the input box and are echoed once when the engine sends them; the task panel groups tasks by state (tokens, duration, timeout); foreground and background agent rows sit above the status rows, with a range line past four rows; a finished turn sends a desktop notification while the window is in the background; the session cost is back in the bottom status row. The rows come from the shared core `bar_text` helpers (`queue_rows`, `task_rows`, `agent_row_text`, `cost_text`).
  - gui: 대기 중 프롬프트가 입력 상자 위에 표시되고 엔진이 보낼 때 한 번 에코됩니다. 작업 패널은 상태별로 작업을 묶고(토큰, 소요 시간, 시간 초과), 전경·백그라운드 에이전트 행이 상태 행 위에 나오며 네 행을 넘으면 범위 줄이 붙습니다. 창이 백그라운드에 있을 때 턴이 끝나면 데스크톱 알림을 보내고, 하단 상태 행에 세션 비용이 다시 표시됩니다. 행 문구는 공용 코어 `bar_text` 도우미(`queue_rows`, `task_rows`, `agent_row_text`, `cost_text`)에서 옵니다.
- gui: slash commands run in the GUI from the same core dispatch as the TUI: /help, /status, /version, /mode, /exit, /doctor and /update run locally (doctor and update off the UI thread), and typing `/` opens a command popup that Tab completes. /import says "not in the GUI yet" for now.
  - gui: 슬래시 명령도 TUI와 같은 코어 디스패치를 써서 GUI에서 실행됩니다. /help, /status, /version, /mode, /exit, /doctor, /update는 로컬에서 돌고(doctor와 update는 UI 스레드 밖에서), `/`를 치면 명령 팝업이 뜨며 Tab으로 완성합니다. /import은 아직 GUI에 없다고 표시합니다.
- gui: the status bar matches the TUI (M/P/T/C row, spinner, rate window, agent segments with descriptions, busy row with "(esc to interrupt)", coder context line under the input, and a new-output marker); PageUp/PageDown scroll the transcript, and Home/End jump to its ends when the input is empty. The rows come from the shared core `bar_text` helpers (`busy_text`, `rate_status`, `top_text`, `bottom_text`, `coder_text`).
  - gui: 상태 표시줄이 TUI와 같습니다(M/P/T/C 행, 스피너, 요청 한도 창, 설명이 붙은 에이전트 구간, "(esc to interrupt)"가 붙은 바쁨 행, 입력 아래 코더 문맥 행, 새 출력 표시). PageUp/PageDown으로 트랜스크립트를 스크롤하고, 입력이 비어 있으면 Home/End로 양 끝으로 갑니다. 행 문구는 공용 코어 `bar_text` 도우미(`busy_text`, `rate_status`, `top_text`, `bottom_text`, `coder_text`)에서 옵니다.
- gui: tool rows carry their start and end times, tool results sit under their call (caveman returns hidden), resident stage rows open and close as the TUI does, and the rotation, orphan, budget, interrupted and mode notices use the shared core text; the GUI welcome banner and tool input summaries are drawn in the entries below.
  - gui: 도구 행에 시작·종료 시각이 붙고, 도구 결과는 해당 호출 아래에 놓입니다(캐비맨 반환은 숨김). 상주 단계 행은 TUI와 같이 열리고 닫히며, 회전·고아·예산·중단·모드 알림은 공용 코어 문구를 씁니다. GUI 환영 배너와 도구 입력 요약은 아래 항목에서 그립니다.
- gui: the welcome box is drawn at the top of the transcript, tool input updates the tool row's summary, and a finished agent writes its done line (a failed agent is marked failed), as the TUI does.
  - gui: 환영 상자가 트랜스크립트 맨 위에 그려지고, 도구 입력이 도구 행의 요약을 갱신하며, 끝난 에이전트는 TUI와 같이 완료 줄을 남깁니다(실패한 에이전트는 실패로 표시).
- gui: keys follow the TUI: Esc cancels a held reply's wait or interrupts a busy turn, Ctrl+C interrupts or quits, Ctrl+D quits, Enter/y allow and n/Esc deny a permission prompt, Ctrl+C denies and interrupts, and an answer that is not an interrupt is echoed as allowed or denied; Ctrl+A/E/W/U/K/J and Alt+Enter/Alt+Backspace edit the input through the core Editor. The Ctrl+C rule is the shared core `ctrl_c_action`.
  - gui: 키는 TUI와 같습니다. Esc는 보류 응답의 대기를 취소하거나 바쁜 턴을 중단하고, Ctrl+C는 중단하거나 종료하며, Ctrl+D는 종료합니다. 권한 요청은 Enter/y 허용, n/Esc 거부이고, Ctrl+C는 거부 후 중단합니다. 중단이 아닌 응답은 allowed 또는 denied로 표시합니다. Ctrl+A/E/W/U/K/J와 Alt+Enter, Alt+Backspace는 코어 Editor로 입력을 편집합니다. Ctrl+C 규칙은 공용 코어 `ctrl_c_action`입니다.
- agents: the haiku-test-lead live gate sends text and Enter as separate `tmux send-keys` calls (pause 1 s between), then polls `capture-pane` for up to 60 s for the reply; both agent copies stay identical.
  - agents: haiku-test-lead 라이브 게이트가 텍스트와 Enter를 별도의 `tmux send-keys` 호출로 보내고(사이 1초 대기), 응답을 `capture-pane`으로 최대 60초 폴링합니다. 두 에이전트 사본은 계속 동일합니다.
- gui: the transcript is the shared core `chaude_core::ui_state::Transcript` instead of a GUI-only list; the GUI pushes user, tool, notice, error and text rows through the same core helpers the TUI uses. Rendering is unchanged.
  - gui: 트랜스크립트가 GUI 전용 목록 대신 공용 코어 `chaude_core::ui_state::Transcript`입니다. GUI는 사용자, 도구, 알림, 오류, 텍스트 행을 TUI와 같은 코어 도우미로 쌓습니다. 화면 그리기는 그대로입니다.
- gui: the busy turn and the held-reply count (`bar.waiting`) now come from core `RunState` (`set_busy`, `held`) instead of GUI-only fields; a running turn keeps its start and the TUI behavior is unchanged.
  - gui: 진행 중 턴과 보류 응답 대기 수(`bar.waiting`)를 GUI 전용 필드 대신 코어 `RunState`(`set_busy`, `held`)에서 읽습니다. 이미 진행 중인 턴은 시작 시각을 유지하며 TUI 동작은 그대로입니다.
- gui: the input box's text, cursor and prompt history now live in core `chaude_core::editor::Editor` (the TUI's editor); egui still draws the box and handles IME and paste, and its edits are copied into the editor. Submitted text is trimmed like the TUI, and the GUI's own history copy was removed.
  - gui: 입력창의 텍스트, 커서, 프롬프트 기록이 이제 코어 `chaude_core::editor::Editor`(TUI의 편집기)에 있습니다. egui는 입력창을 그리고 한글 조합과 붙여넣기를 처리하며, 그 편집은 편집기로 옮겨집니다. 보낸 입력은 TUI처럼 앞뒤 공백을 제거하며, GUI의 기록 사본은 없앴습니다.
- agents: autocompact window 130000 so the 80K handoff runs first.
  - agents: 자동 압축 창을 130000으로 올려 80K 인계가 먼저 실행되도록 했습니다.
- agents: haiku-coder and haiku-batch-worker get a "Quiet output" rule: `-q` for cargo, `tail`/`head`/`-m` on long output, offset/limit reads, and re-run only the failing test, to cut repeated tool-output context.
  - agents: haiku-coder와 haiku-batch-worker에 "Quiet output" 규칙을 추가했습니다. cargo는 `-q`, 긴 출력은 `tail`/`head`/`-m`으로 자르고, 큰 파일은 offset/limit으로 읽고, 실패한 테스트만 다시 돌려 반복되는 도구 출력 맥락을 줄입니다.
- gui: the page and Rust exchange the bridge events and commands: page commands run the same core logic as the TUI (slash dispatch, the engine, the pickers), and the page only logs the events for now. The single-instance guard uses a per-user lock file and a local socket (a named pipe on Windows) for focus, instead of the fixed port 127.0.0.1:47613. `CHAUDE_GUI_SMOKE_SUBMIT=<text>` sends a submit 2 seconds after start (smoke runs only).
  - gui: 페이지와 Rust가 브리지 이벤트와 명령을 주고받습니다. 페이지 명령은 TUI와 같은 코어 로직(슬래시 분기, 엔진, 선택기)을 실행하며, 페이지는 지금은 이벤트를 기록만 합니다. 단일 실행 가드는 고정 포트 127.0.0.1:47613 대신 사용자별 잠금 파일과 포커스용 로컬 소켓(Windows는 named pipe)을 씁니다. `CHAUDE_GUI_SMOKE_SUBMIT=<text>`는 시작 2초 뒤 submit을 보냅니다(스모크 실행 전용).

## 0.12.9 - 2026-11-16
- tui: a streamed reply stays one block when a note or stage row comes in the middle of it. The unfinished line is no longer cut in two, so its rest no longer opens a second "●" block; the reply goes on under the same block with the "  " lead.
  - tui: 스트리밍 중인 답변 가운데에 메모나 단계 행이 와도 답변은 한 블록으로 남습니다. 끝나지 않은 줄이 둘로 잘리지 않아, 그 줄의 나머지가 두 번째 "●" 블록을 여는 일이 없습니다. 답변은 같은 블록에서 "  " 들여쓰기로 이어집니다.
- tui: an error row in the middle of a streamed reply no longer cuts the unfinished line in two either. The error row is drawn between the reply's lines, and the rest of that line completes the same line under the same "●" block.
  - tui: 스트리밍 중인 답변 가운데에 오류 행이 와도 끝나지 않은 줄이 둘로 잘리지 않습니다. 오류 행은 답변 줄 사이에 그려지고, 그 줄의 나머지는 같은 "●" 블록 안에서 원래 줄을 이어 완성합니다.
- core: the usage meter refreshes on its own in both the TUI and the GUI: a background /usage run every 5 minutes (a core timer checks every 5 seconds) and right after each finished turn. Only one run is in flight at a time, and a failed run keeps the last good numbers instead of showing "--".
  - core: 사용량 표시가 TUI와 GUI 모두에서 스스로 갱신됩니다. 백그라운드 /usage 실행이 5분마다(core 타이머가 5초마다 확인) 한 번씩 돌고, 턴이 끝난 직후에도 한 번 돕니다. 실행은 한 번에 하나만 진행되며, 실패하면 "--"로 바뀌지 않고 마지막으로 성공한 값을 그대로 둡니다.
- core: the usage meter shows at startup in both the TUI and the GUI. The last good /usage numbers are cached in `~/.chaude/usage.json` and shown at once; the background /usage run replaces them (about 5 seconds). The TUI status line shows session %, week %, and the session reset time.
  - core: 사용량이 TUI와 GUI 모두에서 시작할 때 바로 보입니다. 마지막으로 성공한 /usage 값을 `~/.chaude/usage.json`에 저장해 바로 보여 주며, 백그라운드 /usage 실행(약 5초)이 끝나면 새 값으로 바뀝니다. TUI 상태 행에는 세션 %, 주간 %, 세션 초기화 시각이 나옵니다.
- core: chat mode config flag (`Config.chat`) starts the claude child minimal (`--tools ""`, no MCP, no slash commands, no settings sources) with no residents and no extra coders; recent folders are kept in `~/.chaude/recent.json` (newest 10).
  - core: 채팅 모드 설정 플래그(`Config.chat`)는 claude 자식 프로세스를 최소 구성(`--tools ""`, MCP 없음, 슬래시 명령 없음, 설정 소스 없음)으로 시작하며 상주 에이전트와 추가 코더가 없습니다. 최근 폴더는 `~/.chaude/recent.json`에 최대 10개까지 저장합니다.
- core: the previous-session mode note (`session_mode_note`) moved from the TUI into shared `chaude_core::handoff`; the TUI output is unchanged.
  - core: 이전 세션 모드 안내(`session_mode_note`)를 TUI에서 공용 `chaude_core::handoff`로 옮겼습니다. TUI 출력은 그대로입니다.
- core: the /resume group lookup and the transcript reads for /resume and /import take the home (`group_pick::groups`, `import_claude::*_in`), so tests use a temp dir and never the real $HOME; the import install note is shared as `import_claude::INSTALL_NOTE`. The TUI output is unchanged.
  - core: /resume 그룹 조회와 /resume·/import의 기록 읽기가 홈 디렉터리를 인수로 받아(`group_pick::groups`, `import_claude::*_in`), 테스트는 임시 폴더를 쓰고 실제 $HOME을 읽지 않습니다. 가져오기 설치 안내는 `import_claude::INSTALL_NOTE`로 공유합니다. TUI 출력은 그대로입니다.
- core: the Esc Esc rewind model (prompt list, selection, cut and restored prompt text) moved from the TUI into shared `chaude_core::rewind`; the TUI calls it and its output is unchanged.
  - core: Esc Esc 되감기 모델(프롬프트 목록, 선택, 자르기와 복원할 프롬프트 문구)을 TUI에서 공용 `chaude_core::rewind`로 옮겼습니다. TUI는 이를 호출하며 출력은 그대로입니다.
- core: the /resume record picker model (project folder list, cursor, row text, and the folder a pick loads) moved from the TUI into shared `chaude_core::record_pick`; the TUI calls it and its output is unchanged. Folders with the same write time now list in name order, as the TUI already did.
  - core: /resume 기록 선택기 모델(프로젝트 폴더 목록, 커서, 행 문구, 고른 폴더를 여는 일)을 TUI에서 공용 `chaude_core::record_pick`으로 옮겼습니다. TUI는 이를 호출하며 출력은 그대로입니다. 수정 시각이 같은 폴더는 이름 순으로 나열합니다(TUI와 같은 순서).
- core: the /resume session group picker (the group list, cursor and import key), the killed-run group and the /resume claude import (`import_for_resume`, `import_blocks`) moved from the TUI into shared `chaude_core::group_pick` and `import_claude`; the TUI calls them and its output is unchanged.
  - core: /resume 세션 그룹 선택기(그룹 목록, 커서, 가져올 키), 중단된 실행의 그룹, /resume claude 가져오기(`import_for_resume`, `import_blocks`)를 TUI에서 공용 `chaude_core::group_pick`과 `import_claude`로 옮겼습니다. TUI는 이를 호출하며 출력은 그대로입니다.
- core: the queue, task and agent row text moved from the TUI into shared `bar_text` (`queue_rows`, `task_rows`, `agent_row_text`, `agent_state`); the TUI calls them and its output is unchanged.
  - core: 큐, 작업, 에이전트 행 문구를 TUI에서 공용 `bar_text`(`queue_rows`, `task_rows`, `agent_row_text`, `agent_state`)로 옮겼습니다. TUI는 이를 호출하며 출력은 그대로입니다.
- core: `slash::dispatch` returns `SlashOutcome` (renamed from `Dispatch`); plain text is `SlashOutcome::SendPrompt` instead of `None`. The TUI output is unchanged.
  - core: `slash::dispatch`는 `Dispatch`에서 이름이 바뀐 `SlashOutcome`을 돌려주며, 일반 입력은 `None` 대신 `SlashOutcome::SendPrompt`입니다. TUI 출력은 그대로입니다.
- core: the tool stamps and summaries, tool result rows, stage rows and the time helpers moved from the TUI into shared `chaude_core::ui_state` (`action_line`, `tool_line`, `tool_result_line`, `stage_text`, `tool_result_text`, `is_caveman_block`) and `chaude_core::localtime::clock_hhmmss`/`unix_now`; the TUI calls them and its output is unchanged (snapshots stay byte-identical).
  - core: 도구 시각과 요약, 도구 결과 행, 단계 행과 시각 도우미를 TUI에서 공용 `chaude_core::ui_state`(`action_line`, `tool_line`, `tool_result_line`, `stage_text`, `tool_result_text`, `is_caveman_block`)와 `chaude_core::localtime::clock_hhmmss`/`unix_now`로 옮겼습니다. TUI는 이를 호출하며 출력은 그대로입니다(스냅샷은 바이트 단위로 동일).
- core: the tool summary, the cut helper and the welcome box text moved into `chaude_core::ui_state` (`tool_summary`, `trunc_chars`, `welcome_lines`, `WELCOME_MODEL`, `WELCOME_THRESHOLD`); the TUI calls them and its output is unchanged (snapshots stay byte-identical).
  - core: 도구 요약, 자르기 도우미와 환영 상자 문구를 공용 `chaude_core::ui_state`(`tool_summary`, `trunc_chars`, `welcome_lines`, `WELCOME_MODEL`, `WELCOME_THRESHOLD`)로 옮겼습니다. TUI는 이를 호출하며 출력은 그대로입니다(스냅샷은 바이트 단위로 동일).
- tui: the transcript keeps an entry-to-first-row index, so looking up an entry's row no longer scans every row; it is kept in step on push, redraw, insert and rewind cut. The output is unchanged (snapshots stay byte-identical).
  - tui: 트랜스크립트가 항목→첫 행 인덱스를 유지하므로, 항목의 행을 찾을 때 모든 행을 훑지 않습니다. 쌓기, 다시 그리기, 삽입, 되감기 절단 때마다 함께 갱신합니다. 출력은 그대로입니다(스냅샷은 바이트 단위로 동일).
- tui: the transcript's row-keyed maps (tool, stage and resume rows, wide and linked rows, prompts) now hold core entry indexes; rewind, import and the record picker cut by entry and resolve rows through the row-to-entry map; the output is unchanged (snapshots stay byte-identical). Rewind also drops the resume rows, and clearing the transcript clears the tool, stage and resume rows.
  - tui: 트랜스크립트의 행 기준 맵(도구·단계·재개 행, 넓은·연결된 행, 프롬프트)이 이제 코어 항목 인덱스를 담습니다. 되감기, 가져오기, 기록 선택기는 항목 기준으로 자르고, 행은 행→항목 매핑으로 찾습니다. 출력은 그대로입니다(스냅샷은 바이트 단위로 동일). 되감기는 재개 행도 정리하고, 트랜스크립트를 비우면 도구·단계·재개 행도 함께 비웁니다.
- tui: streamed assistant and agent text is pushed as core `Entry::Text` rows: the open entry grows in place as lines complete and is redrawn from its text; the output is unchanged (snapshots stay byte-identical, including a multi-chunk stream split mid-line).
  - tui: 스트리밍되는 어시스턴트·에이전트 텍스트를 코어 `Entry::Text` 행으로 쌓습니다. 열린 항목은 줄이 완성될 때마다 자라고 그 텍스트로 다시 그립니다. 출력은 그대로입니다(줄 중간에서 나뉜 여러 청크 스트림을 포함해 스냅샷은 바이트 단위로 동일).
- tui: notes, prompt echo, status and error lines and blank rows are pushed as core `Entry` rows, and the TUI keeps a row-to-entry map through inserts and rewind cuts; the output is unchanged (snapshots stay byte-identical).
  - tui: 알림, 프롬프트 에코, 상태·오류 행과 빈 줄을 코어 `Entry` 행으로 쌓고, 삽입과 되감기 절단 때도 행→항목 매핑을 유지합니다. 출력은 그대로입니다(스냅샷은 바이트 단위로 동일).
- tui: permission answers, the stage rows (working and done), the flush-time blank row, the /view error reset row, and recorded assistant blocks (imports and saved records) are pushed as core `Entry` rows (`Status`, `Notice`, `Blank`, `Text`); the output is unchanged (snapshots stay byte-identical).
  - tui: 권한 답변, 단계 행(진행 중·완료), 응답 종료 뒤 빈 줄, /view 오류 뒤 색 초기화 행, 기록된 어시스턴트 블록(가져오기와 저장 기록)을 코어 `Entry` 행(`Status`, `Notice`, `Blank`, `Text`)으로 쌓습니다. 출력은 그대로입니다(스냅샷은 바이트 단위로 동일).
- tui: tool rows, tool results, resume rows and budget stops are pushed as core `Entry` rows (`Tool` and `ToolResult` carry their start, end and arrival times); tool rows are rewritten through their entry; the output is unchanged (snapshots stay byte-identical).
  - tui: 도구 행, 도구 결과, 재개 행과 예산 중단을 코어 `Entry` 행으로 쌓습니다(`Tool`과 `ToolResult`에 시작·종료·도착 시각이 들어갑니다). 도구 행은 해당 항목을 통해 다시 씁니다. 출력은 그대로입니다(스냅샷은 바이트 단위로 동일).
- tui: the rendered transcript (prompt echo, streamed answer, tool rows and results, notes, mode, interrupt) has byte snapshot tests under `testdata/transcript`, the baseline before the transcript moves onto core `Entry` rows; the output is unchanged.
  - tui: 렌더링된 트랜스크립트(프롬프트 에코, 스트리밍 답변, 도구 행과 결과, 알림, 모드, 중단)에 바이트 스냅샷 테스트를 `testdata/transcript`에 추가했습니다. 트랜스크립트를 코어 `Entry` 행으로 옮기기 전의 기준선이며 출력은 그대로입니다.
- core: the shared transcript model covers every row kind the TUI pushes (user, assistant text, tool call, tool result, agent, notice, status, error, blank) as `chaude_core::ui_state::Entry`, with a `Transcript` list and push helpers; the TUI is unchanged and still draws its own rows, and the GUI keeps its behavior.
  - core: 공용 트랜스크립트 모델이 TUI가 쌓는 모든 행 종류(사용자, 어시스턴트 텍스트, 도구 호출, 도구 결과, 에이전트, 알림, 상태, 오류, 빈 줄)를 `chaude_core::ui_state::Entry`로 담고, `Transcript` 목록과 push 도우미를 제공합니다. TUI는 그대로이며 자체 행을 계속 그리고, GUI 동작도 같습니다.
- core: the GUI's transcript row type (`Entry`: user, text, tool, agent, notice, error) moved from the GUI to `chaude_core::ui_state::Entry`, so both front ends can embed it; no behavior change. The TUI transcript is still pre-rendered ANSI lines and stays in the TUI for now.
  - core: GUI 트랜스크립트 행 타입(`Entry`: 사용자, 텍스트, 도구, 에이전트, 알림, 오류)을 GUI에서 `chaude_core::ui_state::Entry`로 옮겨 두 프론트엔드가 함께 쓸 수 있게 했습니다. 동작은 그대로입니다. TUI 트랜스크립트는 아직 미리 렌더링된 ANSI 줄이므로 당분간 TUI에 남아 있습니다.
- tui: a repeated `Busy(true)` while a turn runs keeps its start time, as the GUI does; the TUI and GUI now share core `RunState::set_busy` (busy, turn start, last tool cleared on each turn start).
  - tui: 이미 진행 중인 턴에 `Busy(true)`가 다시 와도 시작 시각을 유지합니다(GUI와 동일). TUI와 GUI가 코어 `RunState::set_busy`(busy, 턴 시작 시각, 턴 시작 시 마지막 도구 초기화)를 함께 씁니다.
- core: engine progress (busy turn, running agents and their rows, background ids, opus and resident busy, restored rows, queue, Agent/Task list, held reply, rotations, last tool) moved from the TUI `App` into shared `chaude_core::ui_state::RunState`, with its own `apply` and unit tests. The TUI embeds it as `run` and the GUI's `GuiState` embeds it too; the GUI's own copies of busy, agent rows and task list were removed. The TUI output is unchanged. The GUI task list now follows the TUI rule that a new round clears the settled tasks (the GUI does not draw tasks yet).
  - core: 엔진 진행 상태(메인 턴 진행, 실행 중 에이전트와 행, 백그라운드 id, opus·resident 진행, 복원된 행, 대기열, Agent/Task 목록, 보류 응답, 회전, 마지막 도구)를 TUI `App`에서 공용 `chaude_core::ui_state::RunState`로 옮겼습니다. 자체 `apply`와 단위 테스트가 있습니다. TUI는 `run`으로 이를 포함하고, GUI의 `GuiState`도 포함합니다. GUI의 busy, 에이전트 행, 작업 목록 사본은 없앴습니다. TUI 출력은 그대로입니다. GUI 작업 목록은 새 라운드가 시작되면 끝난 작업을 지우는 TUI 규칙을 따릅니다(GUI는 아직 작업을 그리지 않습니다).
- core: the GUI bridge types (`chaude_core::gui_bridge`) are serde JSON: events Rust to page (`entry_push`, `entry_update`, `entry_cut`, `state`, `picker_open`, `picker_close`, `note`, `pong`) and commands page to Rust (`submit`, `interrupt`, `slash`, `picker_pick`, `permission_answer`, `copy`, `ping`), with snapshot tests per event and command. The core Mirror turns transcript and run changes into the events. The TUI output is unchanged.
  - core: GUI 브리지 타입(`chaude_core::gui_bridge`)은 serde JSON입니다. Rust에서 페이지로 가는 이벤트(`entry_push`, `entry_update`, `entry_cut`, `state`, `picker_open`, `picker_close`, `note`, `pong`)와 페이지에서 Rust로 가는 명령(`submit`, `interrupt`, `slash`, `picker_pick`, `permission_answer`, `copy`, `ping`)이며, 이벤트와 명령마다 스냅샷 테스트가 있습니다. 코어 Mirror가 트랜스크립트와 실행 상태의 변화를 이벤트로 바꿉니다. TUI 출력은 그대로입니다.

## 0.12.8 - 2026-11-15
- core: main session idle rotation, one per idle period: the 3570 s idle timer is the primary rotation for every context size (its handoff runs a model call while the 1 hour cache is still warm). Only when the timer did not rotate (app closed or restarted, machine asleep so the timer never fired), the next prompt rotates a main over 80K whose last API response is over 1 hour old, with a handoff built from the transcript (no model call). After a timer rotation the next-prompt rule does not rotate again. The 200K Opus threshold is unchanged.
  - core: 메인 세션 유휴 회전은 유휴 구간당 한 번: 3570 s 유휴 타이머가 모든 맥락 크기에서 먼저 회전합니다(1시간 캐시가 살아 있는 동안 인계 모델 호출). 타이머가 회전하지 못했을 때(앱 종료·재시작, 기기 절전으로 타이머 미발동)에만 다음 입력 시 마지막 API 응답이 1시간 넘었고 맥락이 80K를 넘는 메인을 트랜스크립트 인계로 회전합니다(모델 호출 없음). 타이머가 회전한 뒤에는 다음 입력 규칙이 다시 회전하지 않습니다. Opus 200K 기준은 그대로입니다.
- tui: first Enter submits: an Enter right after a single key (such as a Korean IME commit landing with the Enter) is no longer read as a pasted newline; a newline in the input needs the newline key combo or pasted text, and a run of quick keys before an Enter still counts as a paste.
  - tui: 첫 Enter로 바로 보냅니다. 키 하나 뒤에 바로 온 Enter(예: 한글 IME 조합 확정과 함께 온 Enter)는 붙여넣기 줄바꿈으로 보지 않습니다. 입력창의 줄바꿈은 줄바꿈 키 조합이나 붙여넣은 텍스트로만 생기며, 빠른 키 연속 뒤의 Enter는 여전히 붙여넣기로 봅니다.
- core: notes folder GC: `/notes-gc [--dry-run]` and chaude startup bundle notes/runs/ files older than 14 days into notes/archive/runs-YYYY-MM.md (startup runs only when notes/runs/ has over 200 files or notes/ is over 20 MB); originals are removed only after the bundle is written. Startup GC skips piped stdin and launches with a slash command, subcommand or prompt word, so a dry run never moves files. Only text files (valid UTF-8, no NUL byte) are bundled; binary files stay in place and are counted in the log line. Bundle entries use the header `<!-- notes-gc: notes/runs/<name> (mtime YYYY-MM-DD) -->`; the first version's `=== notes/runs/...` header is still recognised.
  - core: notes 폴더 GC: `/notes-gc [--dry-run]`와 chaude 시작 시 14일 넘은 notes/runs/ 파일을 notes/archive/runs-YYYY-MM.md로 묶습니다(시작 시에는 notes/runs/ 파일이 200개를 넘거나 notes/가 20 MB를 넘을 때만 실행). 원본은 묶음 파일을 쓴 뒤에만 지웁니다. 시작 시 GC는 파이프 입력이거나 슬래시 명령, 하위 명령, 프롬프트 인자와 함께 시작하면 건너뛰므로 드라이런은 파일을 옮기지 않습니다. 텍스트 파일(유효한 UTF-8, NUL 바이트 없음)만 묶고, 이진 파일은 제자리에 두고 로그 줄에 셉니다. 묶음 항목 헤더는 `<!-- notes-gc: notes/runs/<이름> (mtime YYYY-MM-DD) -->`이며, 첫 버전의 `=== notes/runs/...` 헤더도 계속 인식합니다.
- docs: README.md and README.ko.md state the pricing design: Haiku's lower rate at or under 100K, Haiku agents rotating at 80K, Opus rotating at 200K, and the turn cap of 60 for haiku-coder and haiku-batch-worker.
  - docs: README.md와 README.ko.md에 가격 설계를 적었습니다. Haiku 단가는 100K 이하에서 낮고, Haiku 에이전트는 80K에서 회전하며, Opus는 200K에서 회전하고, haiku-coder와 haiku-batch-worker의 턴 상한은 60입니다.
- agents: haiku-coder and haiku-batch-worker get `maxTurns: 60` in frontmatter (Claude Code ends the run at that turn) and a body rule to stop after ~60 tool calls or one commit and hand back state; the brief format (docs/claude-lean/CAVEMAN_FORMAT.md 3.1) now says one commit per brief.
  - agents: haiku-coder와 haiku-batch-worker에 frontmatter `maxTurns: 60`(Claude Code가 그 턴에서 실행을 끊음)과, 약 60 도구 호출 또는 커밋 1개 후 멈추고 상태를 넘기라는 본문 규칙을 추가했습니다. 브리프 형식(docs/claude-lean/CAVEMAN_FORMAT.md 3.1)에는 브리프당 커밋 1개 규칙이 추가되었습니다.
- release: `scripts/release-usage.py` writes token usage for one release cycle (previous tag to new tag) for this project only: cycle totals (main vs subagents, per model), a table per 5h block with the top 3 agents, a table per 7-day week, and the peak 5h block. Local transcripts only; the release procedure (notes/RELEASE.md) commits the result as notes/usage/<version>.md.
  - release: `scripts/release-usage.py`는 이 프로젝트만의 릴리스 주기(이전 태그부터 새 태그까지) 토큰 사용량을 씁니다. 주기 합계(메인과 서브에이전트, 모델별), 5시간 블록별 표(상위 에이전트 3개), 7일 주간 표, 최대 5시간 블록을 포함합니다. 로컬 기록만 사용하며, 릴리스 절차(notes/RELEASE.md)가 결과를 notes/usage/<버전>.md로 커밋합니다.
- core: the input line's buffer logic (cursor, word and line edits, wrapped rows, history browsing) and the display-width table moved from the TUI to `chaude_core::editor` and `chaude_core::width`; the TUI output is unchanged.
  - core: 입력줄의 버퍼 로직(커서, 단어와 줄 편집, 줄바꿈 행, 기록 탐색)과 표시 폭 표를 TUI에서 `chaude_core::editor`와 `chaude_core::width`로 옮겼습니다. TUI 출력은 그대로입니다.
- agents: a long job can run from a task file `notes/tasks/<ws>.md` (steps with title, files, gate, commit message, status todo|done|failed, hash, notes; `chaude_core::tasks` parses and updates it). haiku-coder and haiku-batch-worker do only the first todo step per brief, run its gate, commit, update that step, and hand back (plan/agent-scheduler.md, step 1).
  - agents: 긴 작업을 `notes/tasks/<ws>.md` 작업 파일의 단계(제목, 파일, 게이트, 커밋 메시지, 상태 todo|done|failed, 해시, 메모)로 돌릴 수 있습니다. `chaude_core::tasks`가 파일을 읽고 갱신합니다. haiku-coder와 haiku-batch-worker는 브리프당 첫 todo 단계만 수행하고, 게이트를 돌려 커밋한 뒤 그 단계를 갱신하고 상태를 넘깁니다(plan/agent-scheduler.md, 1단계).

## 0.12.7 - 2026-11-15
- ci: the Linux jobs (rust.yml, release.yml) install the system libraries chaude-gui needs (xkbcommon, GL, X11, Wayland) before building.
  - ci: Linux 작업(rust.yml, release.yml)이 chaude-gui에 필요한 시스템 라이브러리(xkbcommon, GL, X11, Wayland)를 빌드 전에 설치합니다.
- gui: adds the `egui_commonmark` (markdown) and `notify-rust` (desktop notification) dependencies for the GUI; no user-visible change yet.
  - gui: GUI용 `egui_commonmark`(마크다운)와 `notify-rust`(데스크톱 알림) 의존성을 추가합니다. 사용자에게 보이는 변화는 아직 없습니다.
- plan: GUI parity table (G3) added to plan/gui-parity.md: every TUI feature and slash command, with core readiness, GUI status, size, and phase.
  - plan: plan/gui-parity.md에 GUI 기능 대조표(G3)를 추가했습니다. TUI 기능과 슬래시 명령 전체를 core 준비 상태, GUI 상태, 크기, 단계와 함께 적었습니다.
- gui: the GUI starts the core engine and folds its events into a tested state reducer; the window shows a plain transcript for now, with no output pane or input yet.
  - gui: GUI가 core 엔진을 시작하고 이벤트를 테스트된 상태 reducer에 반영합니다. 창에는 지금 평문 기록만 보이고, 출력 창과 입력창은 아직 없습니다.
- gui: the GUI has an output pane (markdown, follows the tail unless scrolled up) and a multi-line input: Enter sends, a trailing `\` or Shift/Alt+Enter adds a newline, Up/Down recall prompt history for this session.
  - gui: GUI에 출력 창(마크다운, 스크롤을 올리지 않으면 끝을 따라감)과 여러 줄 입력창이 생겼습니다. Enter는 보내기, 끝의 `\` 또는 Shift/Alt+Enter는 줄바꿈, Up/Down은 이번 세션의 입력 기록을 불러옵니다.
- gui: the GUI shows a status bar (model, context %, tokens, cost, turn timer, agents, queue) and a y/n permission dialog; Shift+Tab cycles the permission mode.
  - gui: GUI에 상태 막대(모델, 컨텍스트 %, 토큰, 비용, 턴 타이머, 에이전트, 대기열)와 y/n 권한 대화상자가 생겼습니다. Shift+Tab은 권한 모드를 순환합니다.
- core: the prompt history file (`<cwd>/.chaude/history.jsonl`) moved from the TUI editor to `chaude_core::input_history`; the GUI now loads the same history at start and saves each prompt to it, so Up/Down recall survives a restart in both UIs. TUI behaviour is unchanged.
  - core: 프롬프트 기록 파일(`<cwd>/.chaude/history.jsonl`)을 TUI 편집기에서 `chaude_core::input_history`로 옮겼습니다. GUI도 시작할 때 같은 기록을 읽고 보낸 프롬프트를 저장하므로, 두 UI 모두 재시작 후에도 Up/Down 기록 불러오기가 유지됩니다. TUI 동작은 그대로입니다.
- core: the status bar text, agent rows and transcript notices (orphan, budget, rotated, new session, quiet) moved to `chaude_core::bar_text` with keys in core i18n; the GUI status bar and permission dialog use the same text. TUI output is unchanged in ko and en.
  - core: 상태 막대 문구, 에이전트 행, 기록 알림(고아 프로세스, 예산, 컨텍스트 회전, 새 세션, 무출력)을 core i18n 키와 함께 `chaude_core::bar_text`로 옮겼습니다. GUI 상태 막대와 권한 대화상자도 같은 문구를 씁니다. TUI 출력은 ko, en 모두 그대로입니다.
- core: the transcript notices (orphan, budget, rotated, new session, quiet), the status bar held and last-tokens lines, and the permission dialog strings now have Korean text in core i18n; English output is unchanged. The status bar text tests from the GUI (kfmt, status line with default mode hidden, idle activity line) are back in `chaude_core::bar_text`.
  - core: 기록 알림(고아 프로세스, 예산, 컨텍스트 교체, 새 세션, 무출력), 상태 막대의 보류와 마지막 토큰 줄, 권한 대화상자 문구에 core i18n의 한국어 문구를 붙였습니다. 영어 출력은 그대로입니다. GUI에 있던 상태 막대 테스트(kfmt, 기본 모드 숨김, 유휴 활동 줄)는 `chaude_core::bar_text`로 다시 옮겼습니다.
- tui: resident haiku-coder and TUI agents rotate after their reply once their context reaches 80K: the agent writes a handoff note, it is validated as for the main session, and the next call starts a fresh session with it. Main sessions are unchanged (Opus 200K, others 80K).
  - tui: Haiku 상주 coder와 TUI 에이전트는 응답이 끝난 뒤 맥락이 80K에 닿으면 회전합니다. 에이전트가 인계 노트를 쓰고, 메인과 같은 검증을 거친 뒤, 다음 호출이 그 노트로 새 세션을 시작합니다. 메인 세션은 그대로입니다(Opus 200K, 그 밖 80K).

## 0.12.6 - 2026-11-13
- agents: `haiku-runner` and `haiku-test-lead` note that on Windows `python3` is `py -3` (else `python`), the same interpreter choice the Windows hooks use.
  - agents: `haiku-runner`와 `haiku-test-lead`에 Windows에서는 `python3` 대신 `py -3`(없으면 `python`)를 쓴다고 적었습니다. Windows 훅이 고르는 인터프리터와 같습니다.
- release: the next release is 0.12.6; 0.13.0 waits until the GUI is implemented (plan S11).
  - release: 다음 릴리스는 0.12.6입니다. 0.13.0은 GUI 구현 후에 냅니다(계획 S11).
- installer: `scripts/squeeze-run.py` and `scripts/fixed-cost-audit.py` are carried in the chaude binary and written to `<cwd>/scripts` with the hooks (existing files kept; shipped copies overwritten on a version change, like the hooks), so the agents' `python3 scripts/squeeze-run.py` works in installed projects.
  - installer: `scripts/squeeze-run.py`와 `scripts/fixed-cost-audit.py`를 chaude 바이너리에 담아 훅과 함께 `<cwd>/scripts`에 씁니다(기존 파일은 유지, 배포본은 버전이 바뀌면 훅처럼 덮어씀). 설치된 프로젝트에서 에이전트의 `python3 scripts/squeeze-run.py`가 동작합니다.
- tools: new `scripts/fixed-cost-audit.py` estimates the fixed context a session pays before the first prompt (tokens = bytes / 4): CLAUDE.md files, agent and skill descriptions, the MEMORY.md index, and with `--hooks` the SessionStart hook output. Prints a table sorted by tokens with a total.
  - tools: `scripts/fixed-cost-audit.py`는 첫 프롬프트 전에 세션이 치르는 고정 컨텍스트를 추정합니다(토큰 = 바이트 / 4). CLAUDE.md 파일, 에이전트와 스킬 설명, MEMORY.md 색인, `--hooks`를 주면 SessionStart 훅 출력을 셉니다. 토큰 순 표와 합계를 출력합니다.
- tools: new `scripts/squeeze-run.py -- <cmd>` runs a command and prints the first 8 lines, error|warning|panic|FAIL lines with 2 lines of context, and the last 20 lines, with repeats collapsed as `(xN)`; the full output goes to `logs/runs/<ts>.log`, and the exit code is passed through. `haiku-runner` and `haiku-test-lead` wrap builds and tests with it.
  - tools: `scripts/squeeze-run.py -- <cmd>`는 명령을 실행해 앞 8줄, error|warning|panic|FAIL 줄과 앞뒤 2줄 문맥, 마지막 20줄만 출력합니다. 같은 줄 반복은 `(xN)`으로 묶고, 전체 출력은 `logs/runs/<ts>.log`에 저장하며, 종료 코드는 그대로 넘깁니다. `haiku-runner`와 `haiku-test-lead`는 빌드와 테스트를 이 도구로 감쌉니다.
- hooks: a new PreToolUse hook `haiku-read-guard.py` (Read, haiku-* subagents only) denies a full read of a file over 400 lines (use Grep or Read offset/limit) and a repeat full read of the same unchanged file in one session. Fails open. Registered in the installer template, not this repo's settings. `logs/hooks.jsonl` appends now take the shared hooks lock (`logs/.hooks.lock`), the same lock helper as the INBOX.
  - hooks: 새 PreToolUse 훅 `haiku-read-guard.py`(Read, haiku-* 서브에이전트만)는 400줄을 넘는 파일의 전체 읽기(Grep이나 Read offset/limit을 쓰라고 안내)와 같은 세션에서 변경 없는 같은 파일을 다시 전체 읽는 것을 막습니다. 오류가 나면 허용합니다. 이 저장소 설정이 아니라 설치 템플릿에 등록합니다. `logs/hooks.jsonl` 추가도 INBOX와 같은 잠금 도구로 공유 훅 잠금(`logs/.hooks.lock`)을 잡습니다.
- core: on Windows the `~/.chaude` state files (agents.json, clients.json) are now guarded by an exclusive cross-process lock (`LockFileEx` on `<path>.lock`), so two chaude processes no longer lose each other's rows. Before this the lock was a no-op off unix.
  - core: Windows에서 `~/.chaude` 상태 파일(agents.json, clients.json)에 프로세스 간 배타적 잠금(`<path>.lock`에 대한 `LockFileEx`)을 겁니다. 두 chaude 프로세스가 서로의 행을 잃지 않습니다. 이전에는 unix가 아닌 환경에서 잠금이 아무 일도 하지 않았습니다.
- tui: prompt history appends and the trim to the newest 500 entries share one cross-process lock (`.chaude/history.jsonl.lock`), so a line appended by another TUI during a trim is no longer lost.
  - tui: 프롬프트 기록 추가와 최신 500개로 줄이는 작업이 하나의 프로세스 간 잠금(`.chaude/history.jsonl.lock`)을 함께 씁니다. 트림 중에 다른 TUI가 추가한 줄이 더 이상 유실되지 않습니다.
- hooks: notes queued in `record/INBOX.md` from several sessions at once are no longer lost. Appends and rewrites take an exclusive lock (`record/.inbox.lock`), and a rewrite goes through a temp file and `os.replace`.
  - hooks: 여러 세션이 동시에 남긴 `record/INBOX.md` 메모가 더 이상 유실되지 않습니다. 추가와 다시 쓰기가 배타적 잠금(`record/.inbox.lock`)을 잡고, 다시 쓰기는 임시 파일과 `os.replace`를 거칩니다.
- core: a claude child process is killed and reaped whenever its owner drops (quit, panic unwind), and dropping the engine handle waits up to 2 s for the actor before detaching. A coder exit no longer blocks the actor for up to 5 s, so Interrupt is not held up (S14).
  - core: claude 자식 프로세스는 소유자가 드롭될 때(종료, 패닉 되감기) 항상 종료되고 회수됩니다. 엔진 핸들을 드롭하면 액터를 최대 2초 기다린 뒤 분리합니다. 코더 종료가 액터를 최대 5초 막지 않으므로 Interrupt가 지연되지 않습니다 (S14).
- core: the engine's busy flag and turn start time are one `Phase` value (idle, or busy with the turn clock). No behaviour change; a send from idle that is not a turn start (release_held) keeps the phase idle (S15).
  - core: 엔진의 busy 플래그와 턴 시작 시각이 하나의 `Phase` 값(유휴, 또는 턴 시계가 있는 바쁨)이 되었습니다. 동작 변화는 없으며, 유휴 상태에서 보낸 해제 전송(release_held)은 유휴 상태를 유지합니다 (S15).
- hooks: each hook run appends one line `{ts, hook, event, decision}` to `logs/hooks.jsonl` (D5). Logging is fail-safe: a missing or broken `_lean` or a log write error never changes a hook's output (live matrix item 5).
  - hooks: 훅이 실행될 때마다 `logs/hooks.jsonl`에 `{ts, hook, event, decision}` 한 줄을 추가합니다 (D5). 로깅은 실패해도 안전합니다. `_lean`이 없거나 깨져도, 로그 쓰기가 실패해도 훅의 출력은 바뀌지 않습니다 (라이브 매트릭스 5번).
- docs: CLAUDE.md Flow has a rule for a pure ack with no pending `추천:`: one-line reply, no delegation (live matrix D1).
  - docs: CLAUDE.md 흐름에 규칙을 추가했습니다. 보류 중인 `추천:`이 없는 순수 확인 응답은 한 줄로 답하고 위임하지 않습니다 (라이브 매트릭스 D1).
- hooks: with a Haiku main (`--model haiku`), main-thread-guard allows only `opus-architect` and `haiku-output` as agents; the main model is read from the main transcript's newest assistant entry (live matrix D2).
  - hooks: Haiku 메인(`--model haiku`)일 때 main-thread-guard는 에이전트로 `opus-architect`와 `haiku-output`만 허용합니다. 메인 모델은 메인 transcript의 가장 최근 assistant 항목에서 읽습니다 (라이브 매트릭스 D2).
- core: a fable main model is checked against the configured rotation threshold, not the fixed Opus 200k; a test locks this and the full-id form (`claude-fable-5-2`).
  - core: fable 메인 모델은 고정 Opus 200k가 아니라 설정된 회전 기준으로 봅니다. 테스트가 이것과 전체 id 형태(`claude-fable-5-2`)를 고정합니다.
- settings: the main model is one of `opus`, `sonnet`, `fable` (picker and `main_model`), passed to `claude --model` unchanged so the server resolves the latest version. A saved full id (e.g. `claude-sonnet-5-5`) migrates to its family alias on load and save; budget prices and rotation thresholds key on the family.
  - settings: 메인 모델은 `opus`, `sonnet`, `fable` 중 하나입니다 (설정 피커와 `main_model`). `claude --model`에 그대로 전달되어 서버가 최신 버전을 고릅니다. 저장된 전체 모델 이름(예: `claude-sonnet-5-5`)은 읽을 때와 저장할 때 같은 계열의 별칭으로 바뀝니다. 예산 단가와 세션 교체 기준은 계열 기준으로 봅니다.
- tui: the busy row and the welcome box show the model name the Claude server reports (stream-json init, then each reply's model), not a settings alias, so a fable main model no longer shows "opus".
  - tui: 바쁨 줄과 환영 상자는 settings 별칭 대신 Claude 서버가 보고한 모델 이름을 보여 줍니다 (stream-json init, 이후 응답의 모델). 그래서 fable 메인 모델이 "opus"로 보이지 않습니다.
- core: UI-neutral app state moved to `chaude_core::ui_state`: the `Action` enum, the status-bar data `BarInput`, and the prompt-history push rule. The TUI re-exports them, so behavior is unchanged (S4, first slice).
  - core: UI 중립 앱 상태를 `chaude_core::ui_state`로 옮겼습니다: `Action` 열거형, 상태 줄 데이터 `BarInput`, 프롬프트 기록 추가 규칙. TUI는 같은 이름으로 다시 내보내므로 동작은 바뀌지 않습니다 (S4, 첫 단계).
- tests: the test that switches the UI language (`/lang`) now holds one crate-wide lock, so it cannot leak a language change into another test running in parallel (S2).
  - tests: UI 언어를 바꾸는 테스트(`/lang`)는 크레이트 전체 잠금 하나를 잡습니다. 병렬로 도는 다른 테스트에 언어 변경이 새지 않습니다 (S2).
- tests: the two key-press wake tests in the TUI loop no longer use fixed sleeps; the key is sent at once and the sender stays open until the test has read it (S2).
  - tests: TUI 루프의 키 입력 깨우기 테스트 두 개는 고정 sleep을 쓰지 않습니다. 키를 바로 보내고, 테스트가 읽을 때까지 송신 측을 열어 둡니다 (S2).
- ci: the release build runs `cargo test --locked --workspace` on each target before building, so a failing test stops the tag release before any asset is published (S1).
  - ci: 릴리스 빌드는 각 타깃에서 빌드 전에 `cargo test --locked --workspace`를 실행합니다. 테스트가 실패하면 태그 릴리스가 중단되어 에셋이 게시되지 않습니다 (S1).
- docs: the TUI agent view and rewind picker headers no longer say they are unwired (both are wired into the TUI); their blanket dead-code allow is removed (S10).
  - docs: TUI 에이전트 보기와 되감기 선택기의 헤더가 더 이상 연결되지 않았다고 말하지 않습니다 (둘 다 TUI에 연결되어 있습니다). 전체 dead-code 허용도 제거했습니다 (S10).
- docs: the context-threshold values (80k, 100k, 150k, 200k, 400k) now live only in one table in the claude-lean README; CLAUDE.md, README, the repl guide, APPLY_GUIDE and the rust plan link to it instead of repeating the numbers (S28).
  - docs: 맥락 임계값(80k, 100k, 150k, 200k, 400k)은 claude-lean README의 표 하나에만 둡니다. CLAUDE.md, README, repl 가이드, APPLY_GUIDE, Rust 계획은 숫자를 반복하지 않고 이 표로 링크합니다 (S28).
- engine: a pure acknowledgement (`ㅇㅇ`, `ㅇㅋ`, `응`, `네`, `넵`, `좋아`, `계속`, `ok`, `okay`, `yes`, `thanks`, `감사합니다`) skips the prompter summary and goes straight to the main model. Any other input, including short ones like `go`, `hello`, `고쳐` or `다시`, still goes through the prompter (S26). `is_pure_ack` in api.rs covers it.
  - engine: 순수 확인 응답(`ㅇㅇ`, `ㅇㅋ`, `응`, `네`, `넵`, `좋아`, `계속`, `ok`, `okay`, `yes`, `thanks`, `감사합니다`)은 프롬프터 요약을 건너뛰고 메인 모델로 바로 갑니다. `go`, `hello`, `고쳐`, `다시`처럼 그 밖의 입력은 모두 프롬프터를 거칩니다 (S26). api.rs의 `is_pure_ack`이 이를 처리합니다.
- docs: one final-reply owner (haiku-output), one agent count (13), one main-model rule (Haiku main only with `--model haiku`) across README, guide, CLAUDE.md and APPLY_GUIDE (S30). `tests/hooks/test_doc_contradictions.py` guards it.
  - docs: 최종 답변 주체는 haiku-output 하나, 에이전트 수는 13개, Haiku 메인은 `--model haiku`로만 시작하는 규칙으로 README, 가이드, CLAUDE.md, APPLY_GUIDE를 맞췄습니다 (S30). `tests/hooks/test_doc_contradictions.py`가 이를 확인합니다.
- engine: a coder turn that fails with a 429 or overloaded API error, or whose child dies, is sent again with the same task up to 3 times with 2/4/8 s backoff and at most 60 s in total; "coder exited" is reported only after the retries end (S21). `coder_retries_429_twice_then_completes` covers it.
  - engine: 429 또는 overloaded API 오류, 또는 자식 프로세스 종료로 실패한 코더 작업은 같은 작업을 최대 3회, 2/4/8초 간격, 총 60초 이내로 다시 보냅니다. "coder exited"는 재시도가 끝난 뒤에만 보고합니다 (S21). `coder_retries_429_twice_then_completes`가 이를 확인합니다.
- tests: the hook language fallback matches the rule order (`/lang` setting > `CHAUDE_LANG` > OS locale, en when undetectable). `_lean.spawn_lang()` is the spawn-time version used by `style-adapter.py`, so a missing `CHAUDE_LANG` no longer defaults to ko. `tests/hooks/test_lang.py` controls the locale explicitly.
  - tests: 훅 언어 대체 순서를 규칙에 맞췄습니다 (`/lang` 설정 > `CHAUDE_LANG` > OS 로캘, 감지 안 되면 en). `_lean.spawn_lang()`이 시작 시점 언어를 맡고 `style-adapter.py`가 이를 씁니다. 그래서 `CHAUDE_LANG`이 없어도 ko가 기본이 아닙니다. `tests/hooks/test_lang.py`는 로캘을 직접 지정합니다.
- docs: stale context-alert and reset-notice references removed (automatic notice dropped in S20). APPLY_GUIDE and claude-lean README now say to reset by hand (`/exit`, then `claude`). The threshold table is the single source for the 80k/100k/150k/200k/400k values. `LEAN_RESET_AT` rows removed.
  - docs: 맥락 알림 관련 오래된 설명을 뺐습니다 (S20에서 자동 안내 제거). APPLY_GUIDE와 claude-lean README는 직접 리셋(`/exit` 후 `claude`)을 안내합니다. 임계값 표가 80k/100k/150k/200k/400k 값의 기준입니다. `LEAN_RESET_AT` 행을 뺐습니다.
- docs: plan/caveman-structure.md now describes the implemented design: summary_ko replaced by relay (S24), default language is the system language, and the S24/S25 known conflicts are marked done.
  - docs: plan/caveman-structure.md를 현재 구현 기준으로 고쳤습니다. summary_ko는 relay로 바뀌었고(S24), 기본 언어는 시스템 언어이며, S24/S25 해결 항목을 완료로 표시했습니다.
- caveman: guard hook warns by default and covers all 13 agents (S25). `CAVEMAN_GUARD` now defaults to `warn` (metrics row + additionalContext); `haiku-extractor` and `haiku-test-lead` join TARGETS. `log` and `off` still work.
  - caveman: 가드 훅은 기본값이 warn이고 13개 에이전트 전체를 봅니다 (S25). `CAVEMAN_GUARD` 기본값이 `warn`(메트릭 행 + additionalContext)으로 바뀌었고, `haiku-extractor`와 `haiku-test-lead`가 TARGETS에 추가됐습니다. `log`와 `off`는 그대로 동작합니다.
- caveman: drop summary_ko, relay only (S24). Every R-read, R-write and R-decide return ends with a 1-5 line English `relay:` field (docs/claude-lean/CAVEMAN_FORMAT.md 3.7). `summary_ko` is removed from the agents (both copies), the guard hook (every return now requires `relay`) and the resident prompt; haiku-output renders relay in the `lang:` language.
  - caveman: summary_ko 제거, relay만 사용 (S24). 모든 R-read, R-write, R-decide 반환은 끝에 1-5줄 영어 `relay:` 필드를 둡니다(docs/claude-lean/CAVEMAN_FORMAT.md 3.7). 에이전트(두 사본 모두), 가드 훅(모든 반환에 `relay` 필수), resident 프롬프트에서 `summary_ko`를 뺐고, `lang:` 언어 변환은 haiku-output이 합니다.
- docs: lang follows system language. The default is the system language (`/lang` setting > `CHAUDE_LANG` > OS locale; no locale means en), no longer ko. CLAUDE.md, haiku-output, README.md and README.ko.md now match.
  - docs: 언어는 시스템 언어를 따릅니다. 기본값은 시스템 언어입니다(`/lang` 설정 > `CHAUDE_LANG` > OS 로캘, 로캘이 없으면 en). 더 이상 ko가 기본이 아닙니다. CLAUDE.md, haiku-output, README.md, README.ko.md를 맞췄습니다.
- core: hook copy parity test (S6). `.claude/hooks/` must be byte-identical to `crates/core/src/settings/hooks/`; `cargo test -p chaude-core --test hook_copies` fails when they differ. Language order is now `/lang` (settings `language`) > `CHAUDE_LANG` > OS locale, in both Rust `i18n::resolve` and the Python `_lean.lang()`. With no locale set, the Python hooks now default to en, matching Rust (was ko).
  - core: 훅 복사본 동등성 테스트 (S6). `.claude/hooks/`는 `crates/core/src/settings/hooks/`와 바이트 단위로 같아야 하며, 다르면 `cargo test -p chaude-core --test hook_copies`가 실패합니다. 언어 우선순위는 이제 Rust `i18n::resolve`와 Python `_lean.lang()` 모두 `/lang`(설정 `language`) > `CHAUDE_LANG` > OS 로캘입니다. 로캘도 없으면 Python 훅은 이제 en을 씁니다(Rust와 맞춤, 예전에는 ko).
- hooks: drop context-alert.py (S20). The hook was inert (`LEAN_RESET_AT` 0 by default), so it is removed from the hook set, the package copy list and the docs. The ledger is one file, `record/LEDGER.md`; the old `notes/LEDGER.md` was appended into it and removed.
  - hooks: context-alert.py 삭제 (S20). 이 훅은 `LEAN_RESET_AT` 기본값 0이라 아무 일도 하지 않았으므로 훅 목록, 패키지 복사 목록, 문서에서 뺐습니다. 기록장은 `record/LEDGER.md` 한 파일이며, 예전 `notes/LEDGER.md`는 그 뒤에 이어 붙인 뒤 지웠습니다.
- core: per-session live handoff for all writers. The live marker is written only as `notes/tmp/long/.live-handoff-<sid>.json`, once the session id is known. Startup and `/clear` no longer write the shared `.live-handoff.json`, so two chaude instances in one project do not share it. Startup cleanup also skips a live peer's per-session marker.
  - core: 모든 쓰기 경로에서 세션별 live handoff. live 마커는 세션 ID를 알게 된 뒤에만 `notes/tmp/long/.live-handoff-<sid>.json`으로 씁니다. 시작 시와 `/clear` 때 공유 파일 `.live-handoff.json`을 더 이상 쓰지 않으므로, 같은 프로젝트의 두 chaude 인스턴스가 이 파일을 공유하지 않습니다. 시작 시 정리는 살아 있는 peer의 세션별 마커도 건너뜁니다.
- hooks: per-session lean toggle and inbox. 일반모드 / 절약모드 now switch only the current session (`logs/.lean-off-<sid>`, "off" or "on"), so one chaude instance no longer switches another. The project default comes from `.chaude/settings.json` `"lean": "on" | "off"`. The old `.claude/lean-off` is still read as the project default but is never written. `record/INBOX.md` stays one file per project, each line is tagged `[sid]`, and a session injects and removes only its own lines; untagged legacy lines are kept. `session-context.py` still reads the project default only until it passes `session_id` too.
  - hooks: 세션별 절약 모드 전환과 INBOX. 일반모드 / 절약모드는 이제 현재 세션만 바꿉니다(`logs/.lean-off-<sid>`, 값은 "off" 또는 "on"). 한 chaude 인스턴스가 다른 인스턴스를 바꾸지 않습니다. 프로젝트 기본값은 `.chaude/settings.json`의 `"lean": "on" | "off"`에서 옵니다. 예전 `.claude/lean-off`는 프로젝트 기본값으로 계속 읽지만 더 이상 쓰지 않습니다. `record/INBOX.md`는 프로젝트당 한 파일을 유지하며, 각 줄에 `[sid]` 태그를 달고 세션은 자기 줄만 주입하고 지웁니다. 태그가 없는 예전 줄은 그대로 둡니다. `session-context.py`는 `session_id`를 넘기도록 고치기 전까지 프로젝트 기본값만 봅니다.
- core: cross-process lock for ~/.chaude state. `agents.json` and `clients.json` are read, merged and written under an exclusive lock on `<file>.lock`, so two chaude processes no longer drop each other's rows. Writes still go through a tmp file and rename.
  - core: ~/.chaude 상태에 대한 프로세스 간 잠금. `agents.json`과 `clients.json`은 `<파일>.lock`에 건 배타 잠금 아래에서 읽고 합쳐 쓰므로, 두 chaude 프로세스가 서로의 행을 지우지 않습니다. 쓰기는 여전히 임시 파일과 rename으로 합니다.
- core: per-session handoff, safe tmp cleanup. Each session writes its own exit handoff (`notes/tmp/long/.exit-handoff-<sid>.json`), a resume reads the file of the session it resumes, and the old fixed `.exit-handoff.json` is read only as a fallback. Appends to `.sessions.jsonl` take a file lock. Startup cleanup skips entries of a live peer (a `test/` entry whose pid runs, or the live marker whose owner runs, plus anything touched in the last 15 minutes). The `/usage` run uses its own `test/usage-<pid>` dir and removes it after the run.
  - core: 세션별 handoff, 안전한 tmp 정리. 세션마다 자기 종료 handoff(`notes/tmp/long/.exit-handoff-<sid>.json`)를 쓰고, 재개할 때는 그 세션의 파일을 읽습니다. 예전의 고정 이름 `.exit-handoff.json`은 대체 경로로만 읽습니다. `.sessions.jsonl` 추가는 파일 잠금을 겁니다. 시작 시 정리는 살아 있는 다른 인스턴스의 항목(pid가 살아 있는 `test/` 항목, 주인이 살아 있는 live 마커, 최근 15분 안에 쓰인 항목)을 건너뜁니다. `/usage` 실행은 자기 `test/usage-<pid>` 디렉터리를 쓰고 실행 후 지웁니다.
- hooks: isolate session start context and lock ledger. SessionStart carries over only this instance's previous session (the sid chain in `notes/tmp/long/.sessions.jsonl`), never a transcript modified in the last 15 minutes, and injects nothing when unsure. The LEDGER and INBOX lines are gone from the SessionStart snapshot. SubagentStop writes `record/LEDGER.md` under an exclusive lock (`record/.ledger.lock`, msvcrt on Windows) with a temp-file rename, so parallel agents no longer drop entries.
  - hooks: 세션 시작 컨텍스트를 격리하고 ledger를 잠금 처리했습니다. SessionStart는 이 인스턴스의 직전 세션만 이어받습니다(`notes/tmp/long/.sessions.jsonl`의 세션 ID 체인 기준). 최근 15분 안에 수정된 트랜스크립트는 쓰지 않으며, 확실하지 않으면 아무것도 넣지 않습니다. LEDGER와 INBOX 줄은 SessionStart 스냅샷에서 빠졌습니다. SubagentStop은 `record/LEDGER.md`를 배타 잠금(`record/.ledger.lock`, Windows는 msvcrt)으로 보호하고 임시 파일 rename으로 씁니다. 병렬 에이전트의 기록이 더 이상 사라지지 않습니다.
- TUI: append-only input history. Each submitted prompt is appended as one line to `.chaude/history.jsonl`, so two TUIs in one project no longer overwrite each other's history. The file is trimmed to the newest 500 entries only when it grows past that, under `.chaude/history.jsonl.lock`. Up/Down recall is unchanged.
  - TUI: 입력 기록을 추가 전용으로 바꿨습니다. 제출한 프롬프트마다 `.chaude/history.jsonl`에 한 줄씩 덧붙이므로, 같은 프로젝트의 TUI 두 개가 서로의 기록을 덮어쓰지 않습니다. 파일이 500개를 넘을 때만 최신 500개로 줄이며, 이때는 `.chaude/history.jsonl.lock` 잠금 파일을 사용합니다. 위/아래 키로 기록을 불러오는 동작은 그대로입니다.
- TUI: submit only on explicit Enter, paste-safe. A paste with newlines stays in the input and is never submitted. In a terminal without bracketed paste, an Enter that arrives in a burst of keys (within 10 ms of the key before it, or with the next key already queued) becomes a newline; only a lone Enter submits.
  - TUI: 명시적 Enter에서만 전송되며 붙여넣기에 안전합니다. 줄바꿈이 든 붙여넣기는 입력창에 남고 전송되지 않습니다. 브라킷 붙여넣기를 지원하지 않는 터미널에서는 키 입력이 연속으로 들어온 Enter(앞 키로부터 10ms 이내, 또는 다음 키가 이미 대기 중)를 줄바꿈으로 처리하며, 단독 Enter만 전송합니다.
- Agents: `haiku-tui` is renamed `haiku-output` (user output owner for TUI and GUI). The embedded agent, the registry, the caveman hook targets and the docs use the new name. An old `haiku-tui.md` already in a project's `.claude/agents` is kept, so delete it by hand.
  - 에이전트: `haiku-tui`의 이름을 `haiku-output`으로 바꿨습니다(TUI와 GUI의 사용자 출력 담당). 내장 에이전트, 등록 목록, caveman 훅 대상, 문서 모두 새 이름을 씁니다. 프로젝트의 `.claude/agents`에 남아 있던 `haiku-tui.md`는 자동으로 지워지지 않으니 직접 삭제하세요.
- GUI: new `chaude-gui` app (eframe) that opens an empty window. Skeleton only; no chat features yet.
  - GUI: eframe 기반 `chaude-gui` 앱을 추가했습니다. 지금은 빈 창만 열리며 채팅 기능은 아직 없습니다.
- Internal: slash commands, doctor checks and the settings data model moved into chaude-core, so the GUI can share them. TUI behaviour is unchanged.
  - 내부 변경: 슬래시 명령, doctor 점검, 설정 데이터 모델을 chaude-core로 옮겨 GUI가 함께 쓸 수 있게 했습니다. TUI 동작은 그대로입니다.
- Docs: the improvement plan adds algorithm findings (N-T) and steps (S17-S23).
  - 문서: 개선 계획에 알고리즘 점검 결과(N-T)와 단계(S17-S23)를 추가했습니다.
- Docs: caveman-structure is now the current design only (58 lines); the old design history is archived in plan/archive/caveman-structure-history.md.
  - 문서: caveman-structure는 현재 설계만 남겼습니다(58줄). 이전 설계 기록은 plan/archive/caveman-structure-history.md에 보관했습니다.
- Docs: the improvement plan adds user field feedback (section 1a), an operating-model review (Finding V) and steps S27-S30; S26 is rewritten to the user's chosen direction.
  - 문서: 개선 계획에 사용자 현장 평가(1a절), 운영 모델 점검(V), 단계 S27-S30을 추가했습니다. S26은 사용자가 고른 방향으로 다시 썼습니다.
- Docs: the improvement plan adds an execution order (section 3a) that groups steps S1-S30 into five phases, plus a list of steps that need a user decision.
  - 문서: 개선 계획에 실행 순서(3a절)를 추가했습니다. 단계 S1-S30을 5개 단계로 묶었고, 사용자 결정이 필요한 단계 목록도 넣었습니다.
- Correction guard: task notifications and wrapper text (`<system-reminder>`, `<task-notification>`, `<command-…>`) no longer count as corrections, and the `말고` and `…라고` patterns only match at sentence start.
  - 정정 감지 훅: 작업 알림과 래퍼 텍스트(`<system-reminder>`, `<task-notification>`, `<command-…>`)는 정정으로 세지 않으며, `말고`와 `…라고` 패턴은 문장 시작에서만 맞습니다.
- Hooks: the paste hook measures only the text after `[original]` and no longer saves task notifications or wrapper text as pastes, on both hook copies.
  - 훅: 붙여넣기 훅은 `[original]` 뒤의 텍스트만 크기로 재며, 작업 알림과 래퍼 텍스트는 붙여넣기로 저장하지 않습니다. 두 훅 사본 모두 적용했습니다.
- Docs: stale caveman-structure section refs in CLAUDE.md and docs/claude-lean/CAVEMAN_FORMAT.md now point to the current section names (S29).
- install: the macOS installer retries the release read and the asset downloads up to 3 times (waits 2 s, then 4 s); docs/guide/cli.md gives a CI-artifact fallback whose checksum is checked against the release job log (S9).
  - install: macOS 설치 스크립트는 릴리스 조회와 에셋 다운로드를 최대 3번(2초, 4초 대기) 다시 시도합니다. `docs/guide/cli.md`에 CI 아티팩트 대체 설치 방법을 추가했습니다. 체크섬은 릴리스 작업 로그와 맞춰 확인합니다 (S9).
  - 문서: CLAUDE.md와 docs/claude-lean/CAVEMAN_FORMAT.md의 오래된 caveman-structure 절 참조를 현재 절 이름으로 고쳤습니다(S29).
- repo: the unused `tests/e2e-rebuild` Go scratch directory is deleted and its `.gitignore` entries are removed; `tests/chaude-soak` is kept (S16).
  - 저장소: 쓰지 않는 Go 임시 디렉터리 `tests/e2e-rebuild`를 삭제하고 `.gitignore` 항목을 뺐습니다. `tests/chaude-soak`는 그대로 둡니다 (S16).
- core: the `api` module's unit tests moved from `api.rs` into `api/tests.rs` (S5, first step). No code or public API changed.
  - core: `api` 모듈의 단위 테스트를 `api.rs`에서 `api/tests.rs`로 옮겼습니다 (S5, 첫 단계). 코드와 공개 API는 바뀌지 않았습니다.
- core: the turn lifecycle methods (`begin_turn`, `end_turn`, `finish_turn`, `after_turn`, `next_turn`, `remember_turn`) moved from `api.rs` to `api/turn.rs`, with no logic change (S5, turn module).
  - core: 턴 수명 주기 메서드(`begin_turn`, `end_turn`, `finish_turn`, `after_turn`, `next_turn`, `remember_turn`)를 `api.rs`에서 `api/turn.rs`로 옮겼습니다. 로직은 바뀌지 않았습니다 (S5, 턴 모듈).
- core: the retry and timeout methods (prompter, coder idle, wait budget, idle exit timers) moved from `api.rs` to `api/retry.rs`, with no logic change (S5, retry module).
  - core: 재시도와 시간 초과 메서드(프롬프터, 코더 유휴, 대기 예산, 유휴 종료 타이머)를 `api.rs`에서 `api/retry.rs`로 옮겼습니다. 로직은 바뀌지 않았습니다 (S5, 재시도 모듈).
- core: the UI event methods (UiEvent emission, child and sub-agent stream lines, agent rows, queued notices) moved from `api.rs` to `api/events.rs`, with no logic change (S5, events module).
  - core: UI 이벤트 메서드(UiEvent 송출, 자식과 하위 에이전트 스트림 줄, 에이전트 행, 대기 알림)를 `api.rs`에서 `api/events.rs`로 옮겼습니다. 로직은 바뀌지 않았습니다 (S5, 이벤트 모듈).
- core: the submit and routing methods (user text to the main child, prompter, tui and coder workers, switch and resident handling) moved from `api.rs` to `api/route.rs`, with no logic change (S5, route module).
  - core: 제출과 라우팅 메서드(사용자 입력을 메인 자식, 프롬프터, TUI와 코더 워커로 보내기, 전환과 상주 처리)를 `api.rs`에서 `api/route.rs`로 옮겼습니다. 로직은 바뀌지 않았습니다 (S5, 라우트 모듈).
- core: the persistence methods (session, live marker, agent, task and turn records, carried context, handoff snapshots) moved from `api.rs` to `api/persist.rs`, with no logic change (S5, persist module).
  - core: 저장 메서드(세션, 라이브 표식, 에이전트, 작업과 턴 기록, 이어받을 맥락, handoff 스냅샷)를 `api.rs`에서 `api/persist.rs`로 옮겼습니다. 로직은 바뀌지 않았습니다 (S5, 저장 모듈).
- core: the remaining `api.rs` methods (rotation, drain, session and child lifecycle) moved to `api/misc.rs`, with no logic change. This completes the `api` split (S5, misc module).
  - core: 남은 `api.rs` 메서드(회전, 드레인, 세션과 자식 수명 주기)를 `api/misc.rs`로 옮겼습니다. 로직은 바뀌지 않았습니다. 이것으로 `api` 분할이 끝납니다 (S5, 기타 모듈).
- core: the core error type `chaude_core::error::Error` (`Msg` or `Io`) starts replacing `Result<_, String>`; `import_claude` is the first module on it. `home_dir` is one shared function (`osenv::try_home_dir`); the cli copy now delegates to it (S8, first slice).
  - core: 핵심 오류 타입 `chaude_core::error::Error`(`Msg` 또는 `Io`)가 `Result<_, String>`을 대체하기 시작했습니다. 첫 모듈은 `import_claude`입니다. `home_dir`은 하나의 함수(`osenv::try_home_dir`)로 모았고, cli 쪽 복사본은 이것을 씁니다 (S8, 첫 단계).
- core: `osenv::try_home_dir` reads HOME first, then USERPROFILE, on Windows only; on mac and linux it reads HOME only (no USERPROFILE fallback). Unit tests lock both orders on any host.
  - core: `osenv::try_home_dir`는 Windows에서만 HOME을 먼저, 그다음 USERPROFILE을 읽습니다. mac과 linux에서는 HOME만 읽고 USERPROFILE로 넘어가지 않습니다. 단위 테스트가 두 순서를 어느 호스트에서든 고정합니다.
- core: `child_set`, `settings`, `mcp`, `resident` and `proc` return `chaude_core::error::Error` instead of `Result<_, String>`; the API glue (`api/misc.rs`, `api/events.rs`) converts at the boundary. `api.rs` keeps `String` until after S15 (S8, rest of core).
  - core: `child_set`, `settings`, `mcp`, `resident`, `proc`은 `Result<_, String>` 대신 `chaude_core::error::Error`를 반환합니다. API 연결부(`api/misc.rs`, `api/events.rs`)가 경계에서 변환합니다. `api.rs`는 S15 이후까지 `String`을 유지합니다 (S8, core 나머지).
- core: `orch::parse` and `cache_ttl::parse_cache_ttl` return `chaude_core::error::Error` instead of `String`; the cli maps the error to text at the one call site, so the messages are unchanged (S8, sweep).
  - core: `orch::parse`와 `cache_ttl::parse_cache_ttl`은 `String` 대신 `chaude_core::error::Error`를 돌려줍니다. cli는 호출 한 곳에서 문구로 바꾸므로 메시지는 그대로입니다 (S8, 일괄 정리).
- core: `resume_note::write` returns `chaude_core::error::Error`; its callers already ignore the message or take it as the digest hint, so nothing else changes (S8, sweep).
  - core: `resume_note::write`는 `chaude_core::error::Error`를 돌려줍니다. 호출하는 쪽은 이미 메시지를 버리거나 다이제스트 힌트로만 쓰므로 나머지는 바뀌지 않습니다 (S8, 일괄 정리).
- core: `record_turn::resume` returns `chaude_core::error::Error`; the cli turns it into text where it shows the message, so the messages are unchanged (S8, sweep).
  - core: `record_turn::resume`는 `chaude_core::error::Error`를 돌려줍니다. cli는 메시지를 보여주는 곳에서 문구로 바꾸므로 메시지는 그대로입니다 (S8, 일괄 정리).

- core: the coder and tui reply paths in `api/route.rs` and `api/retry.rs` (spawn, write, finish) return `chaude_core::error::Error`; the public `try_start`, `resolve_max_workers` and `CoderCaller::call` and the MCP waiter still return `String`, so the API is unchanged and messages are the same (S8, api sweep).
  - core: `api/route.rs`와 `api/retry.rs`의 코더·tui 응답 경로(생성, 쓰기, 종료)는 `chaude_core::error::Error`를 돌려줍니다. 공개 `try_start`, `resolve_max_workers`, `CoderCaller::call`과 MCP 대기 채널은 여전히 `String`을 씁니다. 그래서 공개 API와 메시지는 그대로입니다 (S8, api 일괄 정리).
- core: a coder task that runs past its timeout (`CODER_TIMEOUT`, 30 min by default) replies with a partial result: `status: partial`, a reason, one `changed:` line per file the coder wrote or edited, then the tail of its text. The limit is `Config.coder_timeout` (no settings-file key yet). The default is unchanged (S22).
  - core: 코더 작업이 제한 시간(`CODER_TIMEOUT`, 기본 30분)을 넘기면 부분 결과로 답합니다: `status: partial`, 사유, 쓰거나 수정한 파일마다 `changed:` 한 줄, 그리고 코더 텍스트의 끝부분. 제한 시간은 `Config.coder_timeout`에서 설정합니다(설정 파일 키는 아직 없습니다). 기본값은 바뀌지 않습니다 (S22).

## 0.12.5 - 2026-11-12
- Token usage: the idle bar row shows the last turn's input, output and cache tokens, and each Task/Agent row in the Tasks list shows its tokens and duration (for example "12.3k tok · 45s") when the tool result reports them.
  - 토큰 사용량: 대기 상태의 바 줄에 마지막 턴의 입력·출력·캐시 토큰을 표시하고, 작업 목록의 Task/Agent 행에는 도구 결과가 알려주는 토큰과 소요 시간(예: "12.3k 토큰 · 45s")을 표시합니다.
- Resuming a session (`/resume` or the startup resume) restores the saved Tasks list, including each agent's tokens and duration.
  - 세션을 재개하면(`/resume` 또는 시작 시 자동 재개) 저장된 작업 목록을 에이전트별 토큰과 소요 시간을 포함해 복원합니다.

## 0.12.4 - 2026-11-12
- CI: the release workflow retries apt for the arm64 cross linker and falls back to archive.ubuntu.com when azure.archive.ubuntu.com does not resolve.
  - CI: arm64 크로스 링커 단계의 apt를 재시도하고, azure.archive.ubuntu.com 이 해석되지 않으면 archive.ubuntu.com 으로 전환합니다.
- 0.12.3 was tagged but not published; its changes ship in 0.12.4.
  - 0.12.3은 태그만 만들어지고 배포되지 않았습니다. 0.12.3의 변경 사항은 0.12.4에 포함됩니다.
- `chaude doctor` and `/doctor`: check old chaude binaries (below 0.12.0), PATH order, install lock (macOS uchg), project hook wiring, Claude login, and repo target/ size. Each check prints OK, WARN or FAIL with a fix hint.
  - `chaude doctor`과 `/doctor`: 오래된 chaude(0.12.0 미만), PATH 순서, 설치본 잠금(macOS uchg), 프로젝트 훅 연결, Claude 로그인, 저장소 target/ 크기를 점검합니다. 각 항목은 OK, WARN, FAIL 중 하나와 조치 안내를 보여줍니다.
- `/doctor` runs the checks in the background, shows "checking…" at once, and posts the result when done. The install version is read from the binary bytes without running it, so 0.12.x shows.
  - `/doctor`는 점검을 백그라운드에서 실행하고 바로 "점검 중…"을 표시한 뒤 끝나면 결과를 보여줍니다. 설치 버전은 바이너리를 실행하지 않고 바이트에서 읽으므로 0.12.x가 표시됩니다.
- `/lang`: switch the language between English and Korean. The language also applies to hook messages and child agent arguments.
  - `/lang`: 영어와 한국어를 전환합니다. 언어 설정은 훅 메시지와 하위 에이전트 인자에도 적용됩니다.
- Agents: new `haiku-test-lead` agent. Tests and gates are routed to it.
  - 에이전트: 새 `haiku-test-lead` 에이전트를 추가했습니다. 테스트와 게이트 작업이 이 에이전트로 전달됩니다.
- Agent replies are queued and shown in a Tasks list.
  - 에이전트 응답은 큐에 쌓이고 Tasks 목록에 표시됩니다.
- Plan text before a tool call is shown first, above its tool row.
  - 도구 호출 전 계획 문구가 도구 행보다 먼저 표시됩니다.
- Scratch layout: `notes/tmp/long` and `notes/tmp/test` are auto-cleaned at startup.
  - 임시 폴더 구조: `notes/tmp/long`과 `notes/tmp/test`는 시작할 때 자동으로 정리됩니다.
- Agent list is cleared on a fresh launch and restored by `/resume`.
  - 에이전트 목록: 새로 실행하면 비워지고 `/resume`으로 복원됩니다.
- Markdown tables render with borders. The input box's last cell is left blank.
  - 마크다운 표는 테두리와 함께 그립니다. 입력창의 마지막 칸은 비워 둡니다.
- Startup cleanup never deletes the `~/.local/bin` install. `mcp-resident` skips cleanup.
  - 시작 정리는 `~/.local/bin` 설치본을 지우지 않습니다. `mcp-resident`는 정리를 건너뜁니다.
- Repo: commit flow rule and a pre-commit guard.
  - 저장소: 커밋 절차 규칙과 pre-commit 가드를 추가했습니다.

## 0.12.3 - 2026-11-11
- `chaude doctor` and `/doctor`: check old chaude binaries (below 0.12.0), PATH order, install lock (macOS uchg), project hook wiring, Claude login, and repo target/ size. Each check prints OK, WARN or FAIL with a fix hint.
  - `chaude doctor`과 `/doctor`: 오래된 chaude(0.12.0 미만), PATH 순서, 설치본 잠금(macOS uchg), 프로젝트 훅 연결, Claude 로그인, 저장소 target/ 크기를 점검합니다. 각 항목은 OK, WARN, FAIL 중 하나와 조치 안내를 보여줍니다.
- `/doctor` runs the checks in the background, shows "checking…" at once, and posts the result when done. The install version is read from the binary bytes without running it, so 0.12.x shows.
  - `/doctor`는 점검을 백그라운드에서 실행하고 바로 "점검 중…"을 표시한 뒤 끝나면 결과를 보여줍니다. 설치 버전은 바이너리를 실행하지 않고 바이트에서 읽으므로 0.12.x가 표시됩니다.
- `/lang`: switch the language between English and Korean. The language also applies to hook messages and child agent arguments.
  - `/lang`: 영어와 한국어를 전환합니다. 언어 설정은 훅 메시지와 하위 에이전트 인자에도 적용됩니다.
- Agents: new `haiku-test-lead` agent. Tests and gates are routed to it.
  - 에이전트: 새 `haiku-test-lead` 에이전트를 추가했습니다. 테스트와 게이트 작업이 이 에이전트로 전달됩니다.
- Agent replies are queued and shown in a Tasks list.
  - 에이전트 응답은 큐에 쌓이고 Tasks 목록에 표시됩니다.
- Plan text before a tool call is shown first, above its tool row.
  - 도구 호출 전 계획 문구가 도구 행보다 먼저 표시됩니다.
- Scratch layout: `notes/tmp/long` and `notes/tmp/test` are auto-cleaned at startup.
  - 임시 폴더 구조: `notes/tmp/long`과 `notes/tmp/test`는 시작할 때 자동으로 정리됩니다.
- Agent list is cleared on a fresh launch and restored by `/resume`.
  - 에이전트 목록: 새로 실행하면 비워지고 `/resume`으로 복원됩니다.
- Markdown tables render with borders. The input box's last cell is left blank.
  - 마크다운 표는 테두리와 함께 그립니다. 입력창의 마지막 칸은 비워 둡니다.
- Startup cleanup never deletes the `~/.local/bin` install. `mcp-resident` skips cleanup.
  - 시작 정리는 `~/.local/bin` 설치본을 지우지 않습니다. `mcp-resident`는 정리를 건너뜁니다.
- Repo: commit flow rule and a pre-commit guard.
  - 저장소: 커밋 절차 규칙과 pre-commit 가드를 추가했습니다.

## 0.12.2 - 2026-11-11
- Agent list: a fresh launch starts with an empty agent list. `/resume` restores the agents of that session only.
  - 에이전트 목록: 새로 실행하면 에이전트 목록이 비어 있습니다. `/resume`은 그 세션의 에이전트만 복원합니다.
- Plan text before a tool call appears right away, above its tool row.
  - 도구 호출 전 계획 문구: 도구 행 위에 바로 표시됩니다.
- Scratch and exit data moved under `notes/tmp/`: `notes/tmp/long` keeps 30 days (resume digests, rotation handoffs), `notes/tmp/test` keeps 24 hours (MCP config and socket dirs, `/usage` run dir). Old files are cleaned at startup. Existing `notes/tmp/*.md` files move into `long/`.
  - 임시 데이터 위치: 임시 데이터가 `notes/tmp/` 아래로 옮겨졌습니다. `notes/tmp/long`은 30일(재개 요약, 교체 인계), `notes/tmp/test`는 24시간(MCP 설정과 소켓 폴더, `/usage` 실행 폴더)을 보관합니다. 시작할 때 오래된 파일을 지웁니다. 기존 `notes/tmp/*.md` 파일은 `long/`으로 옮깁니다.
- MCP sockets use a short system temp path, so long project paths no longer hit the socket path limit.
  - MCP 소켓: 시스템 임시 폴더의 짧은 경로를 씁니다. 긴 프로젝트 경로에서도 소켓 경로 길이 제한에 걸리지 않습니다.
- Tests no longer change `HOME`.
  - 테스트: 더 이상 `HOME`을 바꾸지 않습니다.

## 0.12.1 - 2026-11-11
- Markdown tables render as bordered tables in the TUI.
  - 마크다운 표: TUI에서 테두리가 있는 표로 그립니다.
- Bare `/resume` after a killed run imports the saved state automatically and writes a work note to `notes/tmp`.
  - `/resume` 자동 가져오기: 강제 종료된 실행 뒤 인자 없이 `/resume`을 입력하면 저장된 상태를 자동으로 가져오고 `notes/tmp`에 작업 노트를 씁니다.
- Upgrade: 0.11.x deletes `~/.local/bin/chaude` on start. If an old `/usr/local/bin/chaude` still exists, remove it with `sudo rm -f /usr/local/bin/chaude` so the shell runs the new copy.
  - 업그레이드: 0.11.x는 시작할 때 `~/.local/bin/chaude`를 지웁니다. 예전 `/usr/local/bin/chaude`가 남아 있으면 `sudo rm -f /usr/local/bin/chaude`로 지워야 셸이 새 실행 파일을 먼저 실행합니다.

## 0.12.0 - 2026-11-11
- Language: a new `i18n` setting and the `CHAUDE_LANG` environment variable pick English or Korean. The TUI and the hook messages follow it.
  - 언어: 새 `i18n` 설정과 `CHAUDE_LANG` 환경 변수로 영어 또는 한국어를 고릅니다. TUI와 훅 메시지가 이 설정을 따릅니다.
- Installer: `~/.local/bin` is the install directory and needs no sudo. The installer adds it to PATH once.
  - 설치: 설치 위치는 `~/.local/bin`이며 sudo가 필요 없습니다. 설치 스크립트가 PATH에 한 번만 추가합니다.
- Windows: times show local time.
  - Windows: 시각을 로컬 시간으로 표시합니다.
- Agent tool lines are green.
  - 에이전트 도구 줄이 녹색으로 표시됩니다.
- Link color: fixed a drag that changed the link color.
  - 링크 색: 드래그할 때 링크 색이 바뀌던 문제를 고쳤습니다.
- Input box: the last cell of the input box is left blank.
  - 입력창: 입력창의 마지막 칸을 비워 둡니다.
- Quit is instant. `/resume` notes are kept in `notes/tmp`.
  - 종료가 즉시 이루어집니다. `/resume` 노트는 `notes/tmp`에 보관합니다.
- Hooks: fixed the English status line and the imperative hook message.
  - 훅: 영어 상태 줄과 명령형 훅 메시지를 고쳤습니다.

## 0.11.11 - 2026-11-10
- Rotation: agent completion keeps going to the current main session. Before, the old main stopped after its last child and carried its history over on rotation.
  - 로테이션 뒤에도 에이전트 완료 알림이 현재 메인에 전달됩니다. 이전 메인은 마지막 자식 에이전트가 끝나면 종료되고, 진행 중인 작업 내역은 새 메인에 미리 실립니다.
- TUI: every done mark reaches the main session.
  - TUI에 표시된 모든 완료가 메인에도 전달됩니다.
- Agents: the agent rows, the agent pane, and the agent output show HH:MM:SS times.
  - 에이전트 목록, 에이전트 창, 출력에 HH:MM:SS 시각이 표시됩니다.
- Tool results: each tool result appears under its own tool call.
  - 도구 결과가 자기 호출 바로 아래에 표시됩니다.
- Restored agents: the restored agent list keeps each agent's status, model, start time, and duration.
  - 다시 켜도 에이전트 목록의 상태, 모델, 시작 시각, 걸린 시간이 유지되고, 끝난 에이전트도 목록에 남습니다.
- Agent list: finished agents stay in the list.
- Agent model: a running agent's row shows the model from its own stream once it reports, so it matches its done row (`haiku 5.5`, not `haiku`).
- Resident coder: a usage line `C1 x/80k` shows under the input.
  - 입력창 아래에 상시 코더 사용량(C1 x/80k)이 표시됩니다.

## 0.11.10 - 2026-11-09
- Agents: each finished agent is announced once, with its own result path. A resumed agent is announced again on its next finish. A glob pattern in a prompt (`notes/runs/x-*.md`) is no longer taken as the result path.
  - 에이전트: 끝난 에이전트마다 자기 결과 경로와 함께 한 번만 알립니다. 다시 이어 받은 에이전트는 다음에 끝날 때 다시 알립니다. 프롬프트의 glob 패턴(`notes/runs/x-*.md`)은 더 이상 결과 경로로 쓰지 않습니다.
- Agent list: a running agent with no output or tool activity for 10 minutes shows "possibly stalled" with its silent minutes. This is a display mark only.
  - 에이전트 목록: 실행 중인 에이전트가 10분 동안 출력이나 도구 활동이 없으면 조용했던 분 수와 함께 "멈췄을 수 있음"으로 표시합니다. 표시일 뿐입니다.
- Agent list: a child session picked with pick_child shows as a row while it runs, and as done when idle.
  - 에이전트 목록: `pick_child`로 고른 자식 세션은 실행 중에는 행으로 보이고, 유휴 상태일 때는 완료로 보입니다.
- Idle: every session (main, prompter, TUI, base coder, extra coders) idles out after 3570 seconds. An observed 5-minute cache no longer shortens the main session's idle rotation. Extra coders used 290 seconds before.
  - 유휴 시간: 모든 세션(메인, prompter, TUI, 기본 coder, 추가 coder)은 3570초 동안 쉬면 만료됩니다. 관찰된 5분 캐시는 더 이상 메인 세션의 유휴 교체를 앞당기지 않습니다. 추가 coder는 전에는 290초였습니다.
- Settings: the extra coder idle label is read from the same constant as the idle timeout.
  - 설정: 추가 coder 유휴 표시 문구는 유휴 시간 제한과 같은 상수에서 읽습니다.
- `resident_coder`: a blocked, partial, or failed reply keeps its `reason:` line in the MCP text.
  - `resident_coder`: 차단됨, 부분 완료, 실패한 응답은 MCP 텍스트에 `reason:` 줄을 그대로 남깁니다.
- Hooks: the Bash guard denies an unbounded `until`/`while` wait loop with no alarm, timeout, or counter bound. The shipped hook file is updated.
  - 훅: Bash 가드는 알람, 시간 제한, 카운터 한도 없이 끝나지 않는 `until`/`while` 대기 루프를 거부합니다. 기본 제공 훅 파일도 갱신했습니다.

## 0.11.9 - 2026-11-08
- Idle timeouts: base sessions (main, prompter, TUI, base coder) expire after 1 hour idle. Extra coders and agents expire after 290 seconds, matching the 5-minute prompt cache.
  - 유휴 시간 제한: 기본 세션(메인, prompter, TUI, 기본 coder)은 1시간 동안 쉬면 만료됩니다. 추가 coder와 에이전트는 5분 프롬프트 캐시에 맞춰 290초 후 만료됩니다.
- Hooks and agents: shipped hook and agent files are rewritten when the chaude version changes, so stale copies are replaced. Same-version edits are kept.
  - 훅과 에이전트: chaude 버전이 바뀌면 기본 제공 훅과 에이전트 파일을 다시 씁니다. 같은 버전에서 바꾼 내용은 유지됩니다.
- Windows: a finished agent's state write retries the rename briefly when the target file is busy, so the done record is not lost.
  - Windows: 대상 파일이 사용 중이면 상태 파일 이름 바꾸기를 잠시 재시도하여, 완료 기록이 사라지지 않게 합니다.

## 0.11.8 - 2026-11-07
- Agents: a finished agent is hidden from the agent list, and it comes back when `SendMessage` resumes it.
  - 에이전트: 끝난 에이전트는 에이전트 목록에서 숨기며, `SendMessage`로 재개하면 다시 나타납니다.
- Handback: every turn end sends a reply to main. A failed coder turn sends its reason as `오류: ...` instead of silence.
  - 핸드백: 턴이 끝날 때마다 메인에 응답을 보냅니다. 실패한 coder 턴은 침묵 대신 이유를 `오류: ...`로 보냅니다.
- Idle agents: a resident agent that stays idle for `agent_idle_exit_secs` (default 600 seconds) exits on its own. The next task or `SendMessage` resumes it on the same session.
  - 유휴 에이전트: 상주 에이전트가 `agent_idle_exit_secs`(기본값 600초) 동안 쉬면 스스로 종료합니다. 다음 작업이나 `SendMessage`가 같은 세션으로 다시 이어 갑니다.
- Quit: children get 3 seconds to flush their transcripts before they are killed.
  - 종료: 자식 프로세스는 강제 종료 전에 대화 기록을 저장할 수 있도록 3초를 받습니다.
- Hooks: the guard hooks are regenerated when the chaude version changes, so an upgrade does not need a manual re-apply.
  - 훅: chaude 버전이 바뀌면 가드 훅을 다시 만들므로, 업그레이드 후 수동으로 다시 적용할 필요가 없습니다.
- TUI: the input cursor is hidden unless the input pane has focus.
  - TUI: 입력 창에 포커스가 없으면 입력 커서를 숨깁니다.
- TUI: the agent pane clamps scrolling for the mouse wheel and keys, so it stops at the top and bottom of the content.
  - TUI: 에이전트 화면은 마우스 휠과 키 스크롤을 범위 안으로 제한하여, 내용의 맨 위와 맨 아래에서 멈춥니다.
- Docs: the README "Releasing" section is removed. Release steps are not part of the public README.
  - 문서: README의 "Releasing"(릴리스) 섹션을 삭제했습니다. 릴리스 절차는 공개 README에 두지 않습니다.

## 0.11.7 - 2026-11-07
- Permissions: the engine and the status bar start in the configured start mode (`perm_mode`, default `auto`), and non-TUI runs use it too. Default behavior is unchanged.
  - 권한: 엔진과 상태 막대는 설정된 시작 모드(`perm_mode`, 기본값 `auto`)로 시작하며, TUI가 아닌 실행에도 같은 모드를 씁니다. 기본 동작은 그대로입니다.
- Resume: the `/resume` picker no longer caps the list at 30 groups, so older sessions (including Go-era ones) are reachable by scrolling.
  - 재개: `/resume` 선택 목록이 30개 그룹으로 잘리지 않으므로, 오래된 세션(Go 시절 세션 포함)도 스크롤로 찾을 수 있습니다.
- Windows: the guard hooks run `py -3` (or `python`) instead of `python3`. The Microsoft Store `python3` stub exits without running the hook, which made the main-thread guard fail open. Re-apply: an existing Windows `.claude/settings.json` still has `python3`; replace it with `py -3` or `python`, or regenerate it with `chaude gen-settings windows --out .claude/settings.json`.
  - Windows: 가드 훅이 `python3` 대신 `py -3`(또는 `python`)으로 실행됩니다. Microsoft Store의 `python3` 스텁은 훅을 실행하지 않고 끝나서, 메인 스레드 가드가 열린 상태(fail open)가 되었습니다. 재적용: 기존 Windows `.claude/settings.json`에는 아직 `python3`이 남아 있으므로, `py -3` 또는 `python`으로 바꾸거나 `chaude gen-settings windows --out .claude/settings.json`으로 다시 만드세요.

## 0.11.6 - 2026-11-06
- Permissions: the harness starts in `auto` mode, as the Go harness did, so tool calls no longer prompt on start. The TUI bar shows the same mode the engine runs.
  - 권한: 하네스가 Go 하네스와 같이 `auto` 모드로 시작하므로, 시작 직후 도구 호출에서 더 이상 권한 확인을 묻지 않습니다. TUI 막대는 엔진이 실제로 쓰는 모드를 표시합니다.
- Resume: a session started from an unset cwd is saved under the project folder name, not `misc`, and `/clear` marks use the absolute cwd, so `/resume` finds them.
  - 재개: cwd가 지정되지 않은 세션은 `misc`가 아니라 프로젝트 폴더 이름으로 저장되며, `/clear` 표시도 절대 cwd로 기록하므로 `/resume`이 찾을 수 있습니다.
- Tests: the flaky `child_pid_recorded_on_park` test no longer reads a shared pid list that parallel fake spawns could interleave (test-only change).
  - 테스트: 불안정하던 `child_pid_recorded_on_park` 테스트가 병렬 가짜 spawn이 섞을 수 있는 공유 pid 목록을 더 이상 읽지 않습니다 (테스트만 변경).

## 0.11.5 - 2026-11-06
- Engine: on Windows, a running agent's pid is checked with `OpenProcess` and `GetExitCodeProcess`, so a live agent is no longer marked dead.
  - 엔진: Windows에서 실행 중인 에이전트의 pid를 `OpenProcess`와 `GetExitCodeProcess`로 확인하여, 살아 있는 에이전트를 죽은 것으로 표시하지 않습니다.
- Agents: a finished agent keeps its last text as its summary, and the restored agent view shows it as `(restored)` after a restart.
  - 에이전트: 끝난 에이전트는 마지막 텍스트를 요약으로 보관하며, 재시작 후 복원된 에이전트 화면에 `(restored)`로 표시합니다.
- Windows: test fixtures are checked out with LF line endings (`.gitattributes`), and three unix-only tests are gated with `cfg(unix)`, so Windows clippy and tests pass.
  - Windows: 테스트 픽스처를 LF 줄바꿈으로 체크아웃하도록 `.gitattributes`를 두고, Unix 전용 테스트 3개를 `cfg(unix)`로 제한했습니다. 그래서 Windows clippy와 테스트가 통과합니다.
- Tests: the engine test fakes write one `role_args.<pid>.log` per process, so parallel fake spawns no longer race on a shared log (test-only change).
  - 테스트: 엔진 테스트의 가짜 프로세스가 프로세스마다 `role_args.<pid>.log`를 씁니다. 그래서 동시에 뜬 가짜 spawn이 공유 로그에서 경쟁하지 않습니다 (테스트만 변경).
- TUI: a busy resident (prompter, tui, coder) row shows its own model (`haiku`) instead of the main session model (`opus`); a row without a model falls back to `haiku`, never the main model.
  - TUI: 사용 중인 상주 역할(prompter, tui, coder) 행은 메인 세션 모델(`opus`) 대신 자기 모델(`haiku`)을 표시합니다. 모델이 없는 행은 메인 모델이 아니라 `haiku`로 대체합니다.

## 0.11.4 - 2026-11-06
- Hooks: new `bash-py-guard.py` (Bash matcher) denies file edits made by inline Python or node (`open(..., 'w')`, `write_text`, `os.replace`, `shutil.copy`, `fs.writeFile*`). Read-only Python and `python3 -m pytest` stay allowed. Fresh projects and existing `settings.json` get it too.
  - 훅: 새 `bash-py-guard.py`(Bash 매처)가 인라인 Python이나 node로 하는 파일 편집(`open(..., 'w')`, `write_text`, `os.replace`, `shutil.copy`, `fs.writeFile*`)을 거부합니다. 읽기 전용 Python과 `python3 -m pytest`는 그대로 허용합니다. 새 프로젝트와 기존 `settings.json` 모두 이 훅을 받습니다.
- Agents: `sonnet-drafter` is renamed to `sonnet-writer` (agent file, hooks, CLAUDE.md, docs). Released sections keep the old name.
  - 에이전트: `sonnet-drafter`의 이름을 `sonnet-writer`로 바꿨습니다(에이전트 파일, 훅, CLAUDE.md, 문서). 릴리스된 섹션은 이전 이름을 그대로 둡니다.
- TUI: dragging the mouse past the top or bottom row of the transcript keeps scrolling and extends the selection (1 row per 50 ms at the edge, 3 rows past the pane). It stops at the content top or tail, when the pointer moves back inside, and on release.

## 0.11.3 - 2026-11-06
- Release: the Go implementation under `deprecated/` is deleted, as planned (6th release after 0.10.0). Its CI jobs and release job are removed, and `platform_setup/install.sh` now builds the Rust `chaude`. The Go source stays in git history at tag `v0.11.2`.
  - 릴리스: 계획대로 `deprecated/` 아래의 Go 구현을 삭제했습니다(0.10.0 이후 6번째 릴리스). Go용 CI 작업과 릴리스 작업을 제거했고, `platform_setup/install.sh`는 이제 Rust `chaude`를 빌드합니다. Go 소스는 태그 `v0.11.2`의 git 이력에 남아 있습니다.
- TUI: a SendMessage row stays open until the resumed agent ends. A queued message opens the same row, and the row colours are fixed.
  - TUI: SendMessage 행은 재개된 에이전트가 끝날 때까지 열려 있습니다. 대기 중인 메시지도 같은 행을 열며, 행 색상을 고쳤습니다.
- TUI: `/view` scrolls correctly and shows the agent names.
  - TUI: `/view`가 올바르게 스크롤되고 에이전트 이름을 표시합니다.
- Engine: a reply from an idle agent (handback) is shown at once instead of waiting for a turn.
  - 엔진: 유휴 상태 에이전트의 응답(핸드백)을 턴을 기다리지 않고 바로 표시합니다.

## 0.11.2 - 2026-11-06
- Docs: README effect analysis is updated (long vs short project), with the ping-pong ratio, and the line-3 wording is revised (EN and KO).
  - 문서: README(EN/KO)의 사용 효과 분석을 갱신했습니다(긴 프로젝트 vs 짧은 프로젝트, 핑퐁 비율). 3번째 줄 문구도 다듬었습니다.
- Agents: `sonnet-drafter` is added for story, game and important writing. `haiku-drafter` stays the default.
  - 에이전트: 스토리·게임·중요 문서 작성용 `sonnet-drafter`를 추가했습니다. 기본값은 `haiku-drafter`입니다.
- TUI: wrapped agent rows keep the `│` bar on every line, including the last one.
  - TUI: 줄이 넘어간 에이전트 행은 마지막 줄을 포함해 모든 줄에 `│` 막대를 유지합니다.

## 0.11.1 - 2026-11-06
- TUI: agents resumed with SendMessage are shown back in the agent list, and running rows are yellow.
  - TUI: SendMessage로 재개된 에이전트를 에이전트 목록에 다시 표시하고, 실행 중인 행은 노란색으로 표시합니다.
- TUI: timed action rows, and queued input is no longer echoed. A `/view` crash is fixed.
  - TUI: 시간이 표시되는 작업 행을 추가하고, 대기 중인 입력은 다시 출력하지 않습니다. `/view` 충돌을 고쳤습니다.
- Docs: README install section and feature order reordered (EN and KO), with a usage effect note.
  - 문서: README(EN/KO) 설치 섹션과 기능 순서를 재배치하고 사용 효과 설명을 추가했습니다.

## 0.11.0 - 2026-11-05
- Release: the release archives are now built from the Rust workspace (`crates/cli`, binary `chaude`) for the same four targets: darwin/arm64, linux/amd64, linux/arm64, windows/amd64. Asset names (`chaude_<version>_<os>_<arch>.tar.gz`, windows `.zip`) and `checksums.txt` are unchanged, so `install.sh`, `install.ps1` and `chaude update` keep working.
  - 릴리스: 릴리스 아카이브를 같은 네 대상(darwin/arm64, linux/amd64, linux/arm64, windows/amd64)에 대해 이제 Rust 워크스페이스(`crates/cli`, 바이너리 `chaude`)에서 빌드합니다. 에셋 이름(`chaude_<version>_<os>_<arch>.tar.gz`, windows `.zip`)과 `checksums.txt`는 그대로이므로 `install.sh`, `install.ps1`, `chaude update`는 계속 동작합니다.
- Release: the Go build under `deprecated/` is no longer published. Its release job is disabled, not deleted.
  - 릴리스: `deprecated/` 아래의 Go 빌드는 더 이상 배포하지 않습니다. 릴리스 작업은 삭제하지 않고 비활성화했습니다.
- Parity: the Rust build does not yet cover every Go feature. The open phases are listed in `plan/rust-port.md`.
  - 기능 동등성: Rust 빌드는 아직 Go의 모든 기능을 다루지 않습니다. 남은 단계는 `plan/rust-port.md`에 나와 있습니다.

## 0.10.2 - 2026-11-04
- Project setup: when chaude opens a folder, it writes the agent definitions (`haiku-*.md`, `opus-architect.md`) into `<cwd>/.claude/agents`. Existing files are never overwritten.
- Project setup: if `.claude/settings.json` already exists, the main-thread guard is merged into it instead of being skipped. Existing keys and hooks are kept, and a second run changes nothing.
- Guard: from the main thread, `Agent`/`Task` calls are allowed only for `haiku-*`, `opus-architect` and `claude-code-guide`. The default general-purpose agent is blocked.

## 0.10.1 - 2026-11-04
- Resume: a background agent resumed with SendMessage (`resumedAgentId`) goes back to the active agent list. The launch `agentId` is kept and saved with the agent store, so restored agents can be matched after a restart.
- /view: session rows and the prompt header show the agent name (for example `coder`, `prompter`) instead of the session id. Sessions from an earlier run still show their transcript title.
- /view: the prompt text scrolls with up/down, pgup/pgdn, home/end and the mouse wheel. Left/right step through prompts, esc goes back to the list.

## 0.10.0 - 2026-11-03
- Repo layout: the Go implementation moved to `deprecated/src/main/intt/`. The Rust workspace is at the repo root (`crates/core`, `crates/cli`; `crates/gui` is excluded until it is built). The Rust prototype moved to `deprecated/prototype/chaude-rs/`. E2E tests moved to `tests/e2e/`, `job_monitor.py` to `scripts/`.
- Rust workspace: in progress (core, CLI and TUI skeleton). The shipped binary is still the Go build.
- Deprecation: `deprecated/` (Go) is kept through the 5 releases after 0.10.0 and deleted in the 6th release after 0.10.0. Notes are in `deprecated/README.md`.
- Release: `release-dispatch` and `scripts/bump-version.sh` now use the `deprecated/` paths and also bump the Cargo workspace version.

## 0.9.15
- 설정: `gen-settings`의 출력 폴더(`--out`)가 없으면 자동으로 만듭니다. `.claude` 폴더면 `hooks/`도 함께 만듭니다.
- 턴 대기: 응답을 기다리는 고정 300초 제한을 없앴습니다. 300초가 지나면 턴이 중단되어 뒤늦게 온 Agent 결과가 버려지던 문제를 고쳤습니다. 이제 열린 tool_use가 있으면 조용히 기다리고, 열린 tool_use가 없이 `--idle-notice`(기본 30분) 동안 조용하면 안내를 한 번 출력한 뒤 계속 기다립니다. 중단은 Esc, Ctrl+C, SIGINT로만 합니다.

## 0.9.14
- 설정: 세션 시작 시 `.claude/settings.json`이 없으면 가드 훅이 들어간 파일을 자동으로 만듭니다.
- 빌드 대상 축소: 릴리즈 빌드를 darwin/arm64, linux/amd64, linux/arm64, windows/amd64 4개로 줄였습니다. Intel Mac(darwin/amd64)은 더 이상 빌드하지 않아 설치와 `chaude update`가 오류로 안내합니다. windows/arm64 설치와 업데이트는 windows/amd64 빌드를 받습니다(x64 에뮬레이션).
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.14로 갱신

## 0.9.13
- 메인 Opus 기본 effort를 medium으로 바꿨습니다 (`설정: 메인 Opus 기본 effort를 medium으로 변경`).
- 효율: 핑퐁 벤치 effort 재측정(unset/low/medium, 2회 평균)을 추가했습니다. 비용은 unset 0.861 / low 0.689 / medium 0.786 USD(plain 1.379 대비 -37.5% / -50.0% / -43.0%)입니다. 이번 측정에서 low가 가장 쌌고 medium이 그 다음입니다. 표본이 2회라 순서를 확정할 수 없습니다. 상세는 `docs/pingpong-bench.md`입니다.
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.13으로 갱신

## 0.9.12
- 상주 세션: 꺼졌거나 없는 prompter·tui·coder 슬롯을 요청할 때 한 번 다시 띄운 뒤 재시도합니다. 다시 띄우지 못하면 입력을 원문 그대로 메인에 보냅니다.
- 문서: README와 APPLY_GUIDE에서 `sonnet-reviewer` 참조를 지웠습니다. 이 항목의 CHANGELOG 기록은 그대로 둡니다.
- 설치: 비공개 저장소용 토큰 헤더 안내를 README에 추가했습니다 (`curl -fsSL -H "Authorization: Bearer $GH_TOKEN" .../install.sh | bash`). 설치 스크립트 토큰 순서는 `GH_TOKEN`, `GITHUB_TOKEN`, `gh auth token`입니다.
- 설치: `install.sh`와 `install.ps1`은 릴리즈 바이너리만 받습니다. 소스 빌드 절차는 README "개발자용" 절로 옮겼습니다.
- 소스: Go 코드 위치를 `src/main/intt`로 고쳐 적었습니다 (루트 `go.work`가 이 모듈을 가리킵니다).
- 설정: `.gitattributes`로 `*.sh`, `*.py` 줄바꿈을 LF로 고정합니다.
- 문서: 효율 수치는 비용(-42.7%)만 적고 토큰 절감으로 표현하지 않습니다. `docs/pingpong-bench.md` 요약에서 토큰 차이 줄을 뺐습니다. 원시 수치 표는 그대로입니다.
- 문서: 스킬 `correction`에서 없는 `sonnet-analyst` 참조를 지웠습니다.
- 에이전트: `sonnet-analyst` 참조를 모두 없앴습니다. 판단·병합은 메인 Opus가 정하고 파일 작업은 `haiku-batch-worker`가 맡습니다. 추출은 `haiku-extractor`가 합니다. `/distill` 병합 단계와 설명 단계, `FRESH_START`, `APPLY_GUIDE`, `README`를 고쳤습니다.
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.12로 갱신

## 0.9.11
- `chaude clean [dir]`: 프로젝트에서 claude-lean 하네스를 제거합니다. 기본은 dry-run으로 제거 대상만 보여 주고 아무것도 바꾸지 않습니다.
- `--yes`: 바뀔 파일을 `.chaude-backup-<YYYYMMDD-HHMMSS>/`에 먼저 복사한 뒤 제거합니다.
- 제거 범위: `.claude/{agents,hooks,skills}` 설치 파일, CLAUDE.md의 claude-lean 정책 블록, settings.json의 패키지 훅 항목. `notes/`, `record/`, 매니페스트 밖 파일, settings.json의 사용자 키·사용자 훅은 건드리지 않습니다.
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.11로 갱신

## 0.9.10
- auto 권한 모드 기본: 권한 모드를 지정하지 않으면 auto로 시작합니다.
- 드래그 선택: 출력 영역을 드래그해 선택하면 놓을 때 자동으로 클립보드에 복사합니다.
- 링크·경로 색상: 출력의 링크와 파일 경로를 구분되는 색으로 표시합니다.
- /usage 전체 화면: 사용량 보기가 전체 화면으로 열리고 스크롤할 수 있습니다.
- Windows 회전 기호: 회전 표시 기호의 색을 Windows 터미널에서 맞게 고쳤습니다.
- gen-settings: 기본 경로를 바로잡았습니다.
- charcount: BOM과 CRLF 줄바꿈을 같은 방식으로 처리합니다.
- claude-history·compress-archive 등: `-h` 도움말과 빈 출력 동작을 고쳤습니다.
- extract-transcript: 경로 구분자를 OS에 맞게 처리합니다.
- CI: macOS·Linux·Windows 3개 OS에서 vet·test·빌드·스모크를 돌립니다.
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.10으로 갱신

## 0.9.9
- Windows·Linux 지원: 릴리즈 빌드가 darwin/linux/windows × amd64·arm64 6개 플랫폼 아카이브와 checksums.txt를 만듭니다. `install.sh`가 Linux를 지원합니다.
- Windows 대응: resume 경로 인코딩, claude 실행 경로 해석, 상주 소켓(Windows 전용 구현 분리)
- 효율 테스트: 핑퐁 게임 plain 대비 harness 비용 -42.7%(2회 평균, 전체 토큰 +73%, Haiku 포함). 결과와 주의점은 README의 "효율 테스트" 섹션, 장기 테스트 계획은 `docs/long-session-bench-plan.md`(미실행)
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.9로 갱신

## 0.9.8
- 프롬프트 기록: 입력 기록을 프로젝트별 `<프로젝트>/.chaude/history.jsonl`에 저장(최대 500개), 다시 시작해도 유지. 위 방향키로 이전 입력을 불러옵니다(슬래시 완성 목록이 열려 있으면 위아래는 목록 이동, 목록 맨 위에서 위 키를 누르면 이전 입력으로 넘어감)
- 회전 보류: 백그라운드 에이전트가 실행 중이면 컨텍스트 회전을 보류(최대 10분, `--rotate-defer-timeout`, 0이면 바로 회전). 시간이 지나면 회전 후 `rotate_defer` 기록 남김
- 에이전트 결과 보존: 백그라운드 에이전트가 끝나면 결과를 `notes/runs/<YYMMDD>-agent-<id>.md`에 덧붙여 저장
- 상주 세션은 main, prompter, tui, coder 4개로 고정(0.9.7 이전과 동일, 설정으로 바꾸지 않음)
- `/settings`: `coder_slots`를 `extra_coders`(추가 coder 수, 0-8, 기본 4)로 변경. 추가 coder(coder2...)는 호출 시 생성되고 270초 유휴 후 종료. 예전 `coder_slots` 키도 읽음
- TUI: 입력창 타이핑은 출력 화면 스크롤 위치를 바꾸지 않음(프롬프트 전송이나 맨 아래에서의 출력 갱신만 따라감)
- haiku-coder: 최종 결과 요약을 `notes/runs/`에 먼저 기록한 뒤 반환
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.8로 갱신

## 0.9.7
- `/settings`: 설정을 프로젝트 폴더별 `<프로젝트>/.chaude/settings.json`에 저장(전역 `~/.chaude/settings.json` 폐지). 파일이 없으면 기본값 사용, 프로젝트마다 독립
- 메인 모델 전체 이름: `main_model`(opus/sonnet/fable)을 `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-fable-5-1`로 CLI에 전달, 예산 단가 계산도 전체 이름 인식(이전에는 0으로 계산)
- `/resume`: 검색어 없이 또는 결과가 여럿이면 기록 폴더 선택창(↑/↓·휠·Enter·esc)
- `/view`: 이 chaude 실행에서 만든 세션만 표시, 종료 시 `.exit-handoff.json`에 세션 ID 저장해 다음 시작 때 이어받음
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.7로 갱신

## 0.9.6
- `/resume` 프로젝트 폴더: 현재 폴더뿐 아니라 프로젝트 폴더별로 Claude Code 세션을 묶어 보여 주고, 검색어(query)로 걸러 이어받기
- `/version`: chaude 버전과 Claude Code 버전을 함께 표시
- 바쁠 때 안전한 슬래시 명령은 응답 대기 중에도 바로 실행(`internal/harness/busyslash_test.go`)
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.6으로 갱신

## 0.9.5
- P/T 최소 세션: prompter, tui는 도구·MCP·스킬·설정 없이 짧은 자체 규칙으로만 실행(`Minimal`, `--system-prompt`)
- 코더 도구 제한: coder는 Read,Edit,Write,Bash,Grep,Glob만 사용, MCP는 엄격 모드(`--strict-mcp-config`), 슬래시 명령 끔
- main 세션 MCP 엄격 모드 적용
- prompter 요약 표기: `[prompter summary]`(caveman 영문), 원문은 필요할 때만 `[original]`로 덧붙임. 예전 `[prompter summary EN]`·`[original input]` 제거
- CLAUDE.md 분량 절반 축소(약 8.8k → 4.4k 바이트)
- opus-architect는 Haiku 메인 릴레이 대상만 담당, Opus 메인은 haiku-*에 직접 위임(`opus로` 강제 경로 제거)
- 프로젝트 설정: serena, pyright-lsp 플러그인 비활성화(.claude/settings.json)
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.5로 갱신

## 0.9.4
- 상주 세션 구성 고정: main(Opus 메인), prompter, tui(haiku-tui), coder 4개 슬롯으로 고정
- 입력·응답 흐름 정리: 입력은 prompter를 거쳐 main으로, 응답은 haiku-tui를 거쳐 pane으로 전달
- 에이전트 규칙을 슬롯별로 적용
- 단계별(stage) 진행 줄 표시
- 제거: `--resident` 옵션, 코더 C1~C4 라우팅(coder-spare-1..4), `resident_coders_status` MCP 도구
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md를 0.9.4로 갱신

## 0.9.3
- 상주 세션 표시: 상주(resident) 자식은 명령 처리 중(생각중)일 때만 실행 목록에 표시, 유휴 상태는 숨김(internal/harness/resident.go)
- prompter 영문 요약 중계: 입력을 Opus 메인에 넘기기 전 상주 prompter가 영문 요약을 만들고 원문과 함께 전달(viaPrompter, 실패·시간 초과 시 원문 그대로)
- MCP 도구 추가: resident_coder(상주 코더에 작업 위임, 반환은 변경 요약만), resident_coders_status(코더 상태 조회)
- 코더 라우팅 C0~C4: 컨텍스트 판단으로 기본 코더(C0)와 spare 슬롯(C1~C4) 선택, 단순 조회·소규모 수정은 Task haiku 에이전트 유지(resident_coder.go)
- 상태 줄 P/T/C 컨텍스트 바: 상주 prompter(P), tui(T), coder(C0·Ci)의 사용량으로 계산
- /view 세션 보기: 세션 목록을 입력창 아래에 표시, /view <id-prefix>로 해당 세션 프롬프트 열람(sessview.go)
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md를 0.9.3으로 갱신

## 0.9.2
- 입력 키: opt+delete(Alt+Backspace) 단어 삭제(DeleteWord, 한글 포함), cmd+delete(ctrl+u) 줄 앞까지 삭제(KillToStart)
- 테스트 추가: KillToStart, DeleteWord 구두점/한글, Alt+Backspace·Ctrl+U

## 0.9.1
- status 줄 변경: 역할별 컨텍스트 바(M, P, T, C0, 실행 중 코더 C1..C4) 표시(internal/harness/drain.go)
- TUI 에이전트 라벨 수정: 접두 이름(haiku-scout, opus-architect) 기준으로 라벨 표시
- /view 명령 추가(TUI 전용, Claude Code 세션 대화 보기: /view [id-prefix])
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md를 0.9.1로 갱신

## 0.9.0-r1
- record 기록 기능 추가(internal/harness/record*.go)
- 에이전트 반환 템플릿에 3.6 status·reason 머리 2줄 추가, 반환 규칙 정리(I9)
- 진행 보고 규칙 추가: status 줄 형식(step/pct), scout 상한 120s·10분, haiku-tui 존댓말 1줄 렌더
- caveman-format 훅 재작성: haiku-tui 존댓말·haiku-prompter 브리프 검사 추가, 테스트 tests/hooks로 통합
- haiku-prompter 브리프·반환 형식 보완, 훅 재작성 전 기준선 스냅샷 커밋
- deprecated/ 정리: 이전 노트·가져오기 자료를 deprecated/ 아래로 이동
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md를 0.9.0-r1로 갱신

## 0.9.0
- caveman 형식 정본 docs/claude-lean/CAVEMAN_FORMAT.md 신설 (S0)
- haiku-scout·haiku-researcher R-read 템플릿, 감시 훅 caveman-format.py(PostToolUse Agent|Task, 기본 log, 비차단) 등록 (S1)
- haiku-coder·batch-worker·runner·checker·drafter R-write 템플릿 (S2)
- opus-architect·sonnet-analyst·sonnet-reviewer B형식 브리프 + R-decide 템플릿 (S3)
- 감시 대상 12개 에이전트로 확대 (S4)
- CLAUDE.md:4 존댓말 범위를 사용자 노출 텍스트로 한정, O2 예외줄 제거 (S5)
- haiku-tui 신설(사용자 대면 출력), haiku-prompter 전달문 = 원문 + 요약 (U8, U9, U10)
- U3·U4·U7 구현 조사로 결정: matcher Agent|Task, mode f 기본 활성, lang: ko는 tool_input.prompt로 확인

## 0.8.2
- sonnet/opus(비-haiku) 서브에이전트의 파일·셸 직접 접근을 훅으로 전면 차단; 모든 읽기/쓰기는 haiku 에이전트 경유
- haiku-context-guard 매처 확장: Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell
- sonnet/opus 서브에이전트는 haiku-* 에이전트만 호출 가능 (agent-model-pin)
- sonnet-analyst/sonnet-reviewer/opus-architect tools: Agent 전용
- haiku 에이전트: `RETURN: verbatim <=N chars` 지정 시 원문 발췌 반환 허용 (max 6000)
- 훅 Pyright 경고 수정(reconfigure getattr, 모든 훅)
- 탈출구: 환경변수 CLAUDE_HARNESS_ALLOW_SONNET_IO=1 (기본 off)
- cmd/chaude version 0.8.2

## 0.8.1
- resume 수정: 작업 알림(`<task-notification>`)을 사용자 이벤트에서 처리해 에이전트 추적이 끝나도록 함(onUserEvent).
- 에이전트 패널(입력창 아래) 마우스 휠 스크롤.
- 최종 결과가 비어 있으면 result를 TUI 출력창에 표시(파일 경로만 안내하지 않음).
- sonnet-analyst 직접 파일 읽기/수정 차단(Read/Grep/Glob/Edit 도구 제거, haiku-scout/haiku-coder 위임).
- opus-architect 도구를 Agent 하나로 제한.
- 라우터(분류) 기본 모델을 sonnet에서 haiku로 변경. `CHAUDE_ROUTER_MODEL=sonnet`으로 되돌릴 수 있습니다.

## 0.8.0
- 모델별 세션 유지: 같은 작업 디렉터리에서 모델마다 살아 있는 세션을 따로 두고, 한 번에 하나씩 작업합니다. 같은 모델의 워커 전환은 기존 경로를 그대로 씁니다.
- 유휴 3599초 후 핸드오프 후 종료: 마지막 API 호출 후 3599초가 지난 세션은 핸드오프 후 닫습니다(`--idle-rotate`, TUI에서만 동작).
- 캐시 TTL 관측: 응답 usage에서 캐시 TTL을 감지하며, 5m 캐시가 감지되면 유휴 기준을 290초로 줄입니다(`--cache-ttl`).
- 분류 size tag 기본값을 large로 변경.
- 상태줄에 5h 사용량 표시.
- 설계 파일 경로 기록(U8b): opus 계획 파일 쓰기 시 DesignPath를 기록하고 핸드오프 스냅샷에 덧붙입니다.
- `/clear` 시 파킹된 자식 세션도 종료(U5c): `/clear`와 `/model`이 파킹된 자식 세션을 함께 닫습니다.
- 예산 기능(U9, 기본 꺼짐): `--task-budget-usd`(기본 0)로 켜며, 0이면 예산 로직을 쓰지 않습니다. 켜면 `--max-budget-usd`를 자식 프로세스에 전달합니다.
- 수정(BUG-1): 파킹되는 세션마다 자식 PID를 기록합니다(`ChildPID`).
- 수정(BUG-2): 재조정(reconcile)이 파킹된 세션까지 확인하며, 자식 PID가 살아 있으면 `orphan-alive`로 만료 처리하고 알림만 표시합니다.

## 0.6.6
- 분류 기반 세션 회전: 메인 세션이 답변 끝에 `[cat: X]`(같은 분류) 또는 `[cat-switch: X]`(새 분류) 표시를 붙이고, 전환이면 그 턴 뒤 핸드오프로 새 세션으로 넘어갑니다. 짧은 응답(ㅇㅇ 등)은 전환하지 않습니다. 끄기: `--no-category-rotate`. 메트릭 `stage: category`.

## 0.6.5
- REPL 기능: 에이전트 실행 상태 표시(상태줄), 이전 프롬프트로 시점 이동(Esc Esc), ↑/↓ 프롬프트 기록, 권한 모드(`/mode`, Shift+Tab: default/acceptEdits/plan/auto).
- 수정: 턴 사이에 백그라운드 서브에이전트가 보내는 권한 요청이 응답되지 않아 멈추던 문제(유휴 stdout 리더 추가). 백그라운드 작업의 늦은 `result`를 새 턴의 결과로 오인하지 않도록 방어.

## 0.6.4
- 인자 없이 `chaude`를 실행하면 REPL이 바로 시작합니다(`chaude repl`과 동일). `-`로 시작하는 REPL 옵션(예: `chaude --threshold 200000`)도 REPL로 전달됩니다. `-h/--help`는 사용법 출력.

## 0.6.3
- REPL에서 `/resume`(세션 목록 후 번호 또는 ↑↓+Enter로 선택)와 `/import [id|latest]` 추가. 현재 디렉터리의 Claude Code 세션을 날짜·턴 수·첫/마지막 프롬프트와 함께 최신순으로 보여주고 실행 중인 세션에 이어받습니다.

## 0.6.2
- claude-lean 병합(96765d3)으로 딸려 들어온 옛 파일 28개 제거. `.gitignore`가 `cmd/chaude`를 무시하던 문제 수정.
## 0.6.1
- `chaude --from-claude <id|latest|path>`: 일반 Claude Code 세션 기록을 읽어 새 chaude 세션으로 이어받음(마지막 `추천:` 원문 보존, 최근 대화 포함). `--import-dry-run`은 claude를 띄우지 않고 크기만 출력.
- 턴 로그에 비용·소요시간·회전 수·위임 수 추가, `chaude metrics [--csv]`로 요약/CSV 출력.

## 0.6.0
- claude-lean에서 이름을 `chaude`로 변경 (0.5.3 → 0.6.0). claude-lean의 연장선이며 저장소도 int-ttt/chaude로 변경.
- macOS Apple Silicon(darwin/arm64) 바이너리와 `install.sh`(gh 로그인 시 비공개 저장소도 가능) 추가.
- 이전 내역: `docs/claude-lean/CHANGELOG.md`.
