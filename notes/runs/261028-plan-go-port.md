goal: drop Python; whole project = one Go module, one binary bin/harness (bin/harness.exe on windows). user: "go로 전부 구현하고, 설치 코드에 go를 포함해"
superseded-part: hooks went back to Python (see 261028-go-port-result.md "## hooks back to python"); internal/hooks removed, hooks rows below are historical.
decision: single multi-call binary (not one binary per hook): one build, one artifact per OS, shared code once; OS diffs only in internal/osenv + settings generator + install scripts
rejected: per-hook binaries (10+ build outputs), bash/ps1 dual impl (logic duplicated in 2 langs), keep python hooks + go REPL (user asked for all Go)

layout (done by architect, do not restructure):
  go.mod                 module claude-harness, go 1.22, stdlib only (no third-party deps)
  cmd/harness/main.go    dispatch: `hook <name>` | `repl` | `gen-settings` | script names
  internal/osenv         IsWindows/IsMac/Name()/ResolveBin()/StateDir()
  internal/lean          Root() OffFlag() LeanOff() IsPing() ReadInput() Str() PrintJSON() ParseInterspersed(fs,args)
  internal/settings      gen-settings (posix settings.json -> windows exec form)
  internal/hooks         hooks.go: register(name, func(args []string) int) in each file's init(); Run(name,args)
  internal/scripts       scripts.go: same registry; Has(name); Run(name,args)
  internal/harness       harness.go: Main(args []string) int  (REPL)

port mapping (python file -> go file, command):
  .claude/hooks/<x>.py   -> internal/hooks/<x_with_underscores>.go, register("<x>")  run: bin/harness hook <x>
  .claude/hooks/_lean.py -> internal/lean (done)
  scripts/<x>.py         -> internal/scripts/<x>.go, register("<x with _ -> ->") e.g. search-history  run: bin/harness search-history
  scripts/claude_history.py -> internal/scripts/claude_history.go: exported HistoryDir(project string) string, ProjectsRoot() string + register("claude-history")
  scripts/gen_settings.py -> internal/settings (done)
  harness/{core,repl}.py -> internal/harness/{core,repl}.go; tests/test_acceptance.py -> internal/harness/acceptance_test.go (skip unless HARNESS_LIVE=1)
  spike/*.py             -> deleted (one-off Phase 0 probes; results kept in notes/spike-results.md)
  history-export hook    -> calls scripts.Run("export-history", []string{"--hook","--project",lean.Root()}) with recover(); always exit 0

porting rules:
  behavior identical: same stdout text, same exit codes, same files written (paths, content, encoding UTF-8, newline \n)
  JSON stdout: semantic equality is enough (key order/spacing/\u escapes may differ); use lean.PrintJSON
  argparse: same flag names/defaults; use flag.FlagSet + lean.ParseInterspersed (flags may follow positionals); `--x` and `--x=v` both work in Go flag
  regex: Go RE2 has no lookbehind/backrefs; rewrite equivalently. Python str len = runes (utf8.RuneCountInString); slicing by chars -> []rune
  hooks must never crash Claude Code: recover() panics -> exit 0; bad stdin JSON -> same as python (usually exit 0 silently)
  paths: filepath.Join everywhere; no shell-outs except to `claude` (ResolveBin) where python did
  no new deps; gofmt clean; go vet ./... clean

verification per part (python still present until architect removes it):
  build: go build -o /tmp-scratch/harness ./cmd/harness
  differential: same stdin/args/env (CLAUDE_PROJECT_DIR=<temp copy of fixture dir>) to python3 original and Go; diff stdout, exit code, written files. Cover each branch you can reach.
  record results in notes/runs/261028-go-<part>.md (cases run, pass/fail, known diffs)

must NOT change: cmd/harness/main.go, internal/lean, internal/osenv, internal/settings, .claude/settings.json, python originals (architect deletes after all parts pass), notes/FACTS.md, notes/DECISIONS.md
if you need a shared helper: put it in your own package file (unexported), don't edit internal/lean
