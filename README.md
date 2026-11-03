# chaude

[![Latest release](https://img.shields.io/github/v/release/int-ttt/chaude)](https://github.com/int-ttt/chaude/releases/latest)
[![Go 1.22+](https://img.shields.io/badge/Go-1.22%2B-00ADD8?logo=go&logoColor=white)](deprecated/src/main/intt/go.mod)
[![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)](#install)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**English** | [한국어](README.ko.md)

> **Deprecation notice:** The Go implementation under `deprecated/` is deprecated. It is still the binary shipped in releases. It is kept through the 5 releases after 0.10.0 and deleted in the 6th release after 0.10.0. The Rust workspace at the repo root is the successor and is still in progress.

## Effect

Ping-pong game benchmark (3 prompts), chaude vs plain Opus:

- **Cost: -42.7%** vs plain Opus (USD, mean of 2 runs, 2026-11-01).
- **Cost range: -37.5% to -50.0%** across effort settings (unset, low, medium; re-run 2026-11-02).
- **Total tokens: +73.0%** with Haiku included (2026-11-01). Cost still fell; cache reads dominate the cost.
- **Sample: 2 runs per method, 3 prompts each.** Small sample, so read these as a rough range.

Source: [docs/pingpong-bench.md](docs/pingpong-bench.md).

`chaude` wraps the `claude` CLI in a chat-style terminal screen. Run `chaude` with no arguments to open the REPL. When the context gets large, it writes a handoff note and continues the conversation in a new session.

## Features

- Chat-style REPL with a status line: context bar, rotation count, permission mode, running agents.
- Automatic session rotation with a handoff note, so long conversations continue.
- Opus main decides; Haiku agents do lookups, small edits, and checks.
- Four permission modes (`default`, `acceptEdits`, `plan`, `auto`), switched with `shift+tab`.
- Cost, 5-hour usage, and rotation count in the status line. `/usage` shows plan usage and limits.
- Starting a session creates `.claude/settings.json` with the guard hooks when the file is missing (0.9.14).
- Prompt history per project, saved in `.chaude/history.jsonl`. Search it with `chaude search-history`.
- Import Claude Code sessions with `/import`, `/resume claude`, or `chaude --from-claude`.
- Prebuilt binaries: macOS Apple Silicon (arm64), Linux (amd64, arm64), Windows (amd64; arm64 PCs run it through x64 emulation). Intel Macs are not supported. The installers check `checksums.txt` before installing.

## Install

### macOS / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | bash
chaude --version
```

Supported: macOS on Apple Silicon (arm64) and Linux (amd64, arm64). Intel Macs (darwin/amd64) are not supported and the installer exits with an error.

Installs to `/usr/local/bin/chaude`. The directory is created if missing. `sudo` is used only for the copy, and only when the directory is not writable. An existing `chaude` is kept as `chaude.<old version>.bak`.

To pin a version, set `CHAUDE_VERSION`:
```sh
curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | CHAUDE_VERSION=v0.9.14 bash
```

Needs `curl`, `tar`, `awk`, `sed`, `install`, `mktemp`, and `sha256sum` or `shasum`.

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/int-ttt/chaude/master/install.ps1 | iex
chaude --version
```

On Windows on ARM64, the installer downloads the amd64 build (x64 emulation) and prints a note.

Installs to `%LOCALAPPDATA%\Programs\chaude\chaude.exe`. The installer adds that folder to your user `PATH` once. Open a new terminal after installing.

Requirements: the Claude Code CLI (`claude`) must be on your `PATH`. The installer warns if it is missing. No token is needed. If GitHub rate-limits you (HTTP 403), set `GH_TOKEN` and run the installer again. The installer also reads `GITHUB_TOKEN` and `gh auth token`.

## Quick start

```sh
cd ~/my-project
chaude
```

Type a question or task in the input box at the bottom and press `Enter`. The default model is Opus and the default permission mode is `auto`. Change the mode with `shift+tab` or `/mode`.

## Usage

### REPL keys

| Key | Action |
|---|---|
| `Enter` | Send the input |
| `alt+enter` or `ctrl+j` | New line (a line ending with `\` + `Enter` also makes a new line) |
| `↑` / `↓` | Previous or next input. Up to 500 entries are kept |
| `tab` | Complete a slash command. Type `/` to open the list |
| `shift+tab` | Next permission mode |
| `esc` or `ctrl+c` (while answering) | Stop the answer |
| `esc` `esc` (empty input) | Open the rewind picker |
| `ctrl+c` (idle), `ctrl+d` | Quit |
| `ctrl+t` | Show the agent list (`esc` closes it) |
| `pgup` / `pgdown`, `ctrl+b` / `ctrl+f`, `shift+↑` / `shift+↓` | Scroll |
| `home` / `end` | Top or bottom (when the input is empty) |
| `y` or `Enter` / `n` or `esc` | Allow or deny a tool permission prompt |

Mouse wheel scrolling is on by default. Use `/mouse` to turn it off, so you can select text in the terminal.

### Permission modes

`shift+tab` cycles the modes in this order: `default` → `acceptEdits` → `plan` → `auto` → `default`.

| Mode | What it allows |
|---|---|
| `default` | Asks before each tool runs |
| `acceptEdits` | Runs file edit tools (Edit, Write, MultiEdit, NotebookEdit) without asking |
| `plan` | Runs read-only tools. Denies the rest |
| `auto` | Runs every tool without asking |

### Slash commands

| Command | What it does |
|---|---|
| `/help` | Show commands and keys |
| `/status` | Show model, context tokens, and session |
| `/model [name]` | Show or change the model |
| `/mode [name]` | Show or change the permission mode |
| `/clear` | Start a new session |
| `/rotate` | Hand off to a new session now |
| `/settings` | Open the settings screen. Values are saved on `esc` and apply at the next session start |
| `/resume [n or slug]` | List record topics, or show one. `/resume claude [query]` lists Claude Code sessions |
| `/import [ID or latest]` | Import a Claude Code session |
| `/view [ID prefix]` | Browse Claude Code sessions (`esc` goes back) |
| `/usage` | Show plan usage and limits |
| `/cost`, `/context`, `/compact`, `/config`, `/mcp`, `/agents` | Ask `claude` and show its output |
| `/init`, `/review`, `/doctor` | Run one turn (suggest CLAUDE.md, review changes, check the install) |
| `/mouse` | Turn mouse wheel scrolling on or off |
| `/version` | Show chaude and Claude Code versions |
| `/update` | Check for a newer release and replace the binary. Restart chaude to use it |
| `/exit`, `/quit` | Quit |

While an answer is running, only `/usage`, `/version`, `/help`, `/status`, `/mode`, `/mouse`, `/view`, and `/settings` can be used. Other commands work after the answer ends.

### Command line

| Command | What it does |
|---|---|
| `chaude [repl] [flags]` | Start the REPL (default when no command is given) |
| `chaude --from-claude ID\|latest\|PATH [--import-dry-run]` | Continue a Claude Code session. `--import-dry-run` only checks the size |
| `chaude metrics [--dir D] [--csv]` | Summarize per-turn metrics, or print CSV |
| `chaude search-history "<terms>"` | Search prompt history |
| `chaude gen-settings mac\|linux\|windows [--out FILE]` | Write a `.claude/settings.json` for an OS |
| `chaude clean [dir] [--yes]` | Remove the harness from a project. Dry run by default. `--yes` backs up and removes |
| `chaude update` | Check for a newer release and replace this binary |
| `chaude --version` | Print the version |

Main REPL flags (`chaude repl`): `--model` (default `opus`), `--cwd`, `--threshold` (default `80000`; Opus is fixed at `200000`), `--show-rotation`, `--metrics-dir`. Run `chaude repl -h` for all flags.

## Updating

- In the REPL, run `/update`. It checks the latest release and replaces the binary if a newer one exists. Restart chaude to use it.
- From a shell, run `chaude update`. It does the same check.

## How it works

Opus is the main session. It reads your request, decides the plan, and writes the final answer. It does not read or edit files itself. Hooks enforce this. Haiku agents do the actual work, such as searching, editing, and checking.

| Agent | Model | Used for |
|---|---|---|
| `opus-architect` | Opus | Design and judgment. Hands work to the `haiku-*` agents |
| `haiku-scout` | Haiku | Finding files and past conversations |
| `haiku-researcher` | Haiku | Web research with sources |
| `haiku-extractor` | Haiku | Facts, decisions, and open items from long conversations |
| `haiku-drafter` | Haiku | Writing or editing text |
| `haiku-batch-worker` | Haiku | Format conversion, bulk replace, splitting and merging |
| `haiku-coder` | Haiku | Small, scoped code and bug fixes |
| `haiku-checker` | Haiku | Read-only checks of the changes |
| `haiku-runner` | Haiku | Long jobs in the background |
| `haiku-prompter` | Haiku | Repeated or looped tasks (main thread only) |
| `haiku-tui` | Haiku | Turns agent results into a polite Korean reply |

The full list, tools, and routing rules are in [Models and agents](docs/guide/models-and-agents.md) (Korean).

## Documentation

The user guide is written in Korean.

- [Guide index](docs/guide/index.md)
- [REPL](docs/guide/repl.md): keys, status line, permission modes, slash commands, settings
- [CLI](docs/guide/cli.md): install, subcommands, flags, environment variables, helper scripts
- [Models and agents](docs/guide/models-and-agents.md)
- [Hooks and skills](docs/guide/hooks-and-skills.md)
- [Pingpong benchmark](docs/pingpong-bench.md)
- [Changelog](CHANGELOG.md)

## FAQ

**Q. What happens when the conversation gets long?**
A. When the context reaches the threshold, chaude moves to a new session with a handoff note. The `rot N` count in the status line shows how many times this happened. The threshold is 200,000 tokens for Opus. Other models use 80,000 by default, set with `--threshold`.

**Q. A background agent is still running when a rotation is due.**
A. The rotation waits for running agents for up to 10 minutes (`--rotate-defer-timeout`). After that, it rotates.

**Q. How much does it save?**
A. In one ping-pong game comparison, cost was -42.7% versus plain Opus (average of 2 runs). Two runs is a small sample, so read this as a rough range. See [docs/pingpong-bench.md](docs/pingpong-bench.md).

**Q. Can I continue a Claude Code conversation?**
A. Yes. Use `/import [ID|latest]` or `/resume claude`, or run `chaude --from-claude latest`. The import view shows the whole transcript, including thinking, tool calls and results, and subagent messages.
## Contributing

Build (Go 1.22 or later) and test from `deprecated/src/main/intt`:

```sh
cd deprecated/src/main/intt && go build -o ../../../../bin/chaude ./cmd/chaude && go test ./...
```

Issues: report bugs and request features on [GitHub Issues](https://github.com/int-ttt/chaude/issues).

Contribution guidelines: not published yet.

## License

MIT. See [LICENSE](LICENSE).
