# explore: cmd/chaude/main.go, internal/osenv/osenv.go, internal/settings/settings.go
date: 2026-10-29
status: done (read-only, no repo edits)

cmd/chaude/main.go (55 lines):
  role: single binary entry; dispatches os.Args.
  version: "0.8.0" (ldflags -X main.version overridable)
  no args / repl / --from-claude / --import-dry-run / leading "-" -> harness.Main(args)
  metrics [--dir D] [--csv] -> harness.MetricsMain (summarizes notes/metrics per-turn; counter defs NOT in these files)
  gen-settings <os> -> settings.Main
  --version|-v|version; -h|--help|help prints usage
  helper scripts via scripts.Has/scripts.Run; unknown cmd -> stderr + exit 2

internal/osenv/osenv.go (59 lines):
  role: only file that knows OS differences.
  IsWindows, IsMac (runtime.GOOS)
  Name() -> windows|mac|linux (default linux)
  ResolveBin(name) -> exec.LookPath full path (claude.cmd/.exe on Windows), else bare name
  StateDir():
    windows: %LOCALAPPDATA%\claude-harness (fallback ~/claude-harness)
    mac: ~/.claude-harness (compat)
    linux: $XDG_STATE_HOME/claude-harness or ~/.local/state/claude-harness

internal/settings/settings.go (291 lines):
  role: gen-settings; converts posix .claude/settings.json to OS-specific form.
  parse() / node tree: custom ordered parser + write()
  posixHook regex: ^python3 "\$CLAUDE_PROJECT_DIR/([^"]+\.py)"$
  winHook regex: ^\$\{CLAUDE_PROJECT_DIR\}/([^"]+\.py)$
  WindowsPython(): "py -3" if py launcher on PATH (windows only), else "python"
  Windows(src, py): rewrites hook command/args to interpreter + script;
    mirrors Bash(bin/chaude ...) allow entries as PowerShell(bin/chaude.exe ...)
  Main flags: --out FILE (default stdout), --python CMD (default WindowsPython()),
    --src FILE (default $CLAUDE_PROJECT_DIR/.claude/settings.json, else exe-relative ../.claude/settings.json)
    positional: windows|mac|linux (other -> usage, exit 2)
  mac/linux: text passed through unchanged; only windows is transformed
  output validated with json.Valid; error -> exit 1

doubts:
  - main.go usage text lists only --out for gen-settings; Main also accepts --src and --python.
  - no metric counters defined in these 3 files; metrics semantics need internal/harness MetricsMain (not read).
