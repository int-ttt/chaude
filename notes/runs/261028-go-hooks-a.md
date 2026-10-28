# go-hooks-a result
files: internal/hooks/style_adapter.go, internal/hooks/session_context.go (registered "style-adapter", "session-context")
verify: go vet ./internal/hooks clean; gofmt clean
difftest: notes/runs/261028-go-hooks-a-difftest.py (python original vs `harness hook <name>`, same temp project+HOME; compares exit code, stdout parsed as JSON/plain, resulting file tree)
result: 193/193 PASS

## cases (style-adapter)
mode switch (일반/노멀/lean off, 절약/린/lean on, flag removal, CLAUDE_LEAN=0 note), [normal] tag, was_off -> [lean] tag,
status ping (13 phrasings, no files, >6 files, CRLF, hidden file, 로그 tail), inbox (declarative stored, append, STYLE_INBOX=0, !/?, correction,
imperative incl. lookahead boundary, question, 2 dots, <8/>120, multiline, last_reply_asks ?/추천:/▶/다음 할 일:/plain/>300 chars/skip empty/missing transcript),
pending inbox delivery with frag/ack/long, paste (long, >20 newlines, <pasted_content, dup, +pending),
ack (38 words), retry (5), frag boundary (29/30 chars, question, correction), version notice (mismatch, already notified, older notified, same, no installed,
empty record, transcript-time fallback old/new/bad/missing, unicode sid, no session_id).
## cases (session-context)
source startup|resume|compact|clear|absent|bogus x {notes, empty, lean-off, CLAUDE_LEAN=0}; 3000-char truncation; resume upgrade via .lean-ver-<sid>, .lean_ver_last,
downgrade, tuple compare, garbage, transcript-time fallback, VERSION fallback; compact/startup no-upgrade; 3-day cleanup of logs/.*-*;
previous-session carry-over (filtering ping/meta/tool_result/<..., last 6, 200-char cut, newest other transcript, self excluded, non-dict line, cwd-fallback history_dir).

## intentional differences / doubts
- bad stdin JSON: python crashes (exit 1 traceback); Go exits 0 silently (plan: hooks never crash CC).
- python json.dumps spacing (", ", ": ") vs Go compact JSON: semantically identical, compared as parsed JSON.
- Go regexp has no lookahead: IMPERATIVE `(?=[\s.,!~]|$)` rewritten as consuming group; equivalent for boolean search. \s extended with \p{Z} etc. to match python unicode \s.
- history_dir ported privately (scHistoryDir) since internal/scripts has no exported one; Windows normcase approximated.
- helpers pyStrip/pyRstrip/pyIsSpace/pyReadLines/pySplitlines/truthy/lastRunes/firstRunes live in style_adapter.go (session_context.go depends on them): name clash risk with other hooks files.
- not covered: Windows, DST/timezone edge of naive timestamps, python non-dict transcript line crash in last_reply_asks (Go skips).
