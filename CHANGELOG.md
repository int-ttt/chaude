# Changelog

## Unreleased
- core: a claude child process is killed and reaped whenever its owner drops (quit, panic unwind), and dropping the engine handle waits up to 2 s for the actor before detaching. A coder exit no longer blocks the actor for up to 5 s, so Interrupt is not held up (S14).
  - core: claude 자식 프로세스는 소유자가 드롭될 때(종료, 패닉 되감기) 항상 종료되고 회수됩니다. 엔진 핸들을 드롭하면 액터를 최대 2초 기다린 뒤 분리합니다. 코더 종료가 액터를 최대 5초 막지 않으므로 Interrupt가 지연되지 않습니다 (S14).
- core: the engine's busy flag and turn start time are one `Phase` value (idle, or busy with the turn clock). No intended behaviour change; a send that starts while idle now makes the phase busy with its clock (S15, first slice).
  - core: 엔진의 busy 플래그와 턴 시작 시각이 하나의 `Phase` 값(유휴, 또는 턴 시계가 있는 바쁨)이 되었습니다. 의도한 동작 변화는 없으며, 유휴 상태에서 시작된 전송은 시계와 함께 바쁨 상태가 됩니다 (S15, 첫 단계).
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
