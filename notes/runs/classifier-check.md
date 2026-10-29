# classifier-check (read-only)
date: 2026-10-29
conclusion: Sonnet classifier is wired in code; calls `claude -p --model sonnet` once per prompt when conductor is on.

## 1. classifier call and model
- model_id_used: "sonnet" (alias, not a full model ID) — internal/harness/classifier.go:13 `conductorModel = "sonnet"`
- call: internal/harness/classifier.go:43-44 `exec.Command("claude", "-p", in, "--model", conductorModel, ..., "--tools", "", "--no-session-persistence")`
- invoked from: internal/harness/classifier.go:66-75 `route()` -> `classify(...)`
- output parsed: classifier.go:21-37 `parseRoute` (model: haiku|sonnet|opus)

## 2. conductor entry conditions
- flag: internal/harness/repl.go:814 `fs.BoolVar(&a.Conductor, "conductor", false, ...)` (default false)
- field: internal/harness/repl.go:56 `Conductor bool`
- gate: internal/harness/repl.go:642-644 `if r.a.Conductor { text = r.route(line) }`
- bypass: classifier.go:67-69 acks (`isAck`) and `/`-prefixed lines skip the classifier
- runtime toggle: repl.go:866-876 `/conductor` and `/conductor-off`
- note: the flag is on the repl FlagSet (usage says repl is default with no command), so `chaude --conductor` is assumed to reach it; not run.

## 3. version vs HEAD
- HEAD: 890ac34 (master). Its diff touches only .github/workflows/release.yml and notes/DECISIONS.md, so no Go code change after 0.6.9.
- nearest tag: v0.6.9 (`git describe --tags`), commit 542b1e2 "0.6.9: inline category marker hidden and parsed". `git tag --points-at HEAD` is empty.
- source version: cmd/chaude/main.go:16 `var version = "0.6.9"`
- `chaude --version`: "chaude 0.6.9" (/Users/leekwangseok/.local/bin/chaude, symlink-free, built Oct 29 02:28)
- installed binary contains conductor strings (e.g. "classifier routes each prompt to haiku|sonnet|opus").
- sha256 differs from repo root `chaude` (e177cc50...) vs installed (35493a79...). Not compared further; likely build path/tooling difference.

## not verified
- Did not run chaude (would call the model). Runtime call of `claude -p --model sonnet` is code-confirmed only.
- Did not confirm the `--model sonnet` alias resolves to claude-sonnet-* on this machine.
- Root `chaude` file and installed binary are different builds; which one the user runs is not confirmed beyond `--version`.
