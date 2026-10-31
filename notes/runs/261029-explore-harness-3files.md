# explore: internal/harness history.go, importclaude.go, localcmd.go (read-only)

## history.go
purpose: up/down prompt recall (shell style) state + rewind turn picker. Pure state, no I/O.
types: inputHistory{items []string; idx int; draft string}; rewindState{open bool; turns []int; sel int}
const: maxHistory = 200
inputHistory.Add(s): skip "" and consecutive dup; keep last 200; idx=len(items), draft="".
inputHistory.Prev(cur): first step saves cur as draft; false if empty or at oldest.
inputHistory.Next(): past newest returns (draft, true).
inputHistory.Browsing(): idx < len(items).
rewindState.Open(blocks) bool: turns = indices of kind=="user" blocks with non-empty text not starting with "/"; sel = last; open iff any.
rewindState.Move(d): sel wraps modulo len(turns). Cancel(): open=false.
rewindState.Confirm(blocks) (cut, text, ok): cut = turns[sel]; text = blocks[cut].text; caller keeps blocks[:cut].
counts: maxHistory = stored prompts (not bytes). turns = real user prompts (slash commands excluded), not all user blocks.

## importclaude.go
purpose: import plain Claude Code .jsonl transcript into REPL; dry-run report.
types: claudeImport{Path string; Turns [][2]string (user, assistant joined "\n"); Files []string; Reco string}
resolveClaudeTranscript(arg, cwd): arg containing ".jsonl" or separator used as path; else ~/.claude/projects/<cwd with "/" "." "_" -> "-">/; "latest" = newest mtime *.jsonl; else <id>.jsonl.
readClaudeTranscript(path): streams with bufio 1<<20 (1 MiB) buffer; skips isSidechain and isMeta; user turn = string content not starting "<", or first text block when curUser empty; assistant text blocks trimmed, joined "\n"; Files = unique file_path of Write/Edit/NotebookEdit (Bash not counted); Reco = last extractReco hit across whole transcript; error if zero turns.
importFromClaude() (exit, done): dry-run prints path, turns, files, snapshot_chars (runes), snapshot_bytes, reco_found, reco_preserved, reco_bytes; exit 1 if !preserved; else (0, true) dry / (0, false) continue REPL with pendingSnapshot.
applyImport(imp, path) (snap, preserved): sets r.tr.turns, r.files, r.lastReco; builds snapshot with note; restoreAgents(sessionID); metrics stage="import".
consts: buffer 1<<20 only.
counts: turns = flushed user-turn entries (not raw events; prompt with no assistant text still counts). files = unique edited paths. reco_bytes = bytes of last 추천 block.
doubt: comment says assistant "final text blocks only" but code appends every non-empty assistant text block in the turn.

## localcmd.go
purpose: plain REPL local commands (/help, /status) answered without sending to child.
localCmd(r, line) (string, bool): first whitespace field only; "/help" -> localHelp(); "/status" -> r.localStatus(); args ignored; else ok=false.
localHelp(): "%-8s %s" per slashCmds entry (name, desc), joined "\n".
localStatus(): "model %s · ctx %s/%s · rotations %d · session %s"; session = trunc(ID, 8), or "-" if sess nil or ID empty.
consts: none (literal "/help", "/status").
counts: ctx = r.lastCtx (comma-formatted); threshold = r.effectiveThreshold(); rotations = r.rotations; session = first 8 chars of ID.
doubts: lastCtx unit and effectiveThreshold logic, slashCmds table are defined outside these files (not read).
