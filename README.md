# chaude

A way to use Claude Code through a chat-format prompt that hands the routine work to Haiku agents.

[![Latest release](https://img.shields.io/github/v/release/int-ttt/chaude)](https://github.com/int-ttt/chaude/releases/latest)
[![Rust stable](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=white)](rust-toolchain.toml)
[![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)](#installation)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**English** | [한국어](README.ko.md)

## What is it

`chaude` wraps the `claude` CLI in a chat-style terminal screen. Opus is the main session. Haiku agents do lookups, small edits, and checks. When the context gets large, `chaude` writes a handoff note and continues the conversation in a new session.

## Features

- Chat-style REPL with a status line: context bar, rotation count, permission mode, running agents.
- Automatic session rotation with a handoff note, so long conversations continue.
- Opus main decides; Haiku agents do lookups, small edits, and checks.
- Four permission modes (`default`, `acceptEdits`, `plan`, `auto`), switched with `shift+tab`.
- Cost, 5-hour usage, and rotation count in the status line. `/usage` shows plan usage and limits.
- Prompt history per project, saved in `.chaude/history.jsonl`. Search it with `chaude search-history`.
- Import Claude Code sessions with `/import`, `/resume claude`, or `chaude --from-claude`.
- Starting a session creates `.claude/settings.json` with the guard hooks when the file is missing (0.9.14).
- The Bash hook `bash-py-guard.py` denies file edits made by inline Python or node (`open(..., 'w')`, `write_text`, `fs.writeFile*`). Read-only code and `python3 -m pytest` stay allowed (0.11.4).
- On Windows the guard hooks run `py -3` (or `python`), not `python3`, because the Microsoft Store `python3` stub made the guard fail open (0.11.7). Re-apply on an existing project: replace `python3` in `.claude/settings.json` with `py -3` or `python`, or regenerate it with `chaude gen-settings windows --out .claude/settings.json`.
- Sessions (main, prompter, TUI, base coder, extra coders) wait 1 hour before idle expiry. Regular agents run inside main and have no idle limit of their own.

## How it works

Opus is the main session. It reads your request, decides the plan, and writes the final answer. It does not read or edit files itself. Hooks enforce this. Haiku agents do the actual work, such as searching, editing, and checking.

| Agent | Model | Used for |
|---|---|---|
| `opus-architect` | Opus | Design and judgment. Hands work to the `haiku-*` agents |
| `haiku-scout` | Haiku | Finding files and past conversations |
| `haiku-researcher` | Haiku | Web research with sources |
| `haiku-extractor` | Haiku | Facts, decisions, and open items from long conversations |
| `haiku-drafter` | Haiku | Writing or editing text (default) |
| `sonnet-writer` | Sonnet | Story, game, and important writing |
| `haiku-batch-worker` | Haiku | Format conversion, bulk replace, splitting and merging |
| `haiku-coder` | Haiku | Small, scoped code and bug fixes |
| `haiku-checker` | Haiku | Read-only checks of the changes |
| `haiku-runner` | Haiku | Long jobs in the background |
| `haiku-prompter` | Haiku | Repeated or looped tasks (main thread only) |
| `haiku-tui` | Haiku | Turns agent results into a polite Korean reply |

The full list, tools, and routing rules are in [Models and agents](docs/guide/models-and-agents.md) (Korean).

**Context and rotation.** When the context reaches the threshold, `chaude` moves to a new session with a handoff note. The `rot N` count in the status line shows how many times this happened. The threshold is 200,000 tokens for Opus. Other models use 80,000 by default, set with `--threshold`. If background agents are still running when a rotation is due, it waits up to 10 minutes (`--rotate-defer-timeout`) and then rotates.

**Claude Code sessions.** You can continue a Claude Code conversation with `/import [ID|latest]` or `/resume claude`, or run `chaude --from-claude latest`. The import view shows the whole transcript, including thinking, tool calls and results, and subagent messages.

## Installation

Prebuilt binaries are published for macOS on Apple Silicon (arm64), Linux (amd64, arm64), and Windows (amd64; arm64 PCs run it through x64 emulation). Intel Macs are not supported. The installers check `checksums.txt` before installing anything.

You also need the Claude Code CLI (`claude`) on your `PATH`. The installers warn if it is missing.

### macOS / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | bash
chaude --version
```

Supported: macOS on Apple Silicon (arm64) and Linux (amd64, arm64). Intel Macs (darwin/amd64) exit with an error.

Installs to `~/.local/bin/chaude`, a folder you own, so no `sudo` is needed. The folder is created if missing. If `~/.local/bin` is not on your `PATH`, the installer adds `export PATH="$HOME/.local/bin:$PATH"` to your shell rc file (`~/.zshrc` for zsh, `~/.bashrc` or `~/.bash_profile` for bash) once. Open a new terminal after installing. An existing `chaude` is kept as `chaude.<old version>.bak`.

An older install at `/usr/local/bin/chaude` is not removed by the installer. If it exists, the installer prints the command to remove it (`sudo rm -f /usr/local/bin/chaude`). Until you remove it, a shell may still run the old copy first.

To pin a version, set `CHAUDE_VERSION`:

```sh
curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | CHAUDE_VERSION=v0.12.0 bash
```

Needs `curl`, `tar`, `awk`, `sed`, `install`, `mktemp`, and `sha256sum` or `shasum`.

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/int-ttt/chaude/master/install.ps1 | iex
chaude --version
```

On Windows on ARM64, the installer downloads the amd64 build (x64 emulation) and prints a note.

Installs to `%USERPROFILE%\.local\bin\chaude.exe`, a folder you own, so no admin rights are needed. The installer adds that folder to your user `PATH` once, only when it is missing. Open a new terminal after installing. An older install at `%LOCALAPPDATA%\Programs\chaude\chaude.exe` is not removed by the installer. If it exists, the installer prints how to remove it and its `PATH` entry.

No token is needed. If GitHub rate-limits you (HTTP 403), set `GH_TOKEN` and run the installer again. The installer also reads `GITHUB_TOKEN` and `gh auth token`.

### From source

You need Rust stable. `rust-toolchain.toml` pins the version. From the repo root:

```sh
cargo build --release -p chaude-cli
./target/release/chaude --version
```

The binary is `target/release/chaude`. Copy it to a folder on your `PATH`.

## Quick start

```sh
cd ~/my-project
chaude
```

Type a question or task in the input box at the bottom and press `Enter`. The default model is Opus. Change the permission mode with `shift+tab` or `/mode`.

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

Mouse wheel scrolling is on by default. Use `/mouse` to turn it off, so you can select text in the terminal. Dragging past the top or bottom row of the transcript keeps scrolling and extends the selection (0.11.4).

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
| `/view [ID prefix]` | Browse Claude Code sessions, with sub-agent names and SendMessage rows (`esc` goes back) |
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

Run `chaude repl -h` for all REPL flags.

### Updating

- In the REPL, run `/update`. It checks the latest release and replaces the binary if a newer one exists. Restart chaude to use it.
- From a shell, run `chaude update`. It does the same check.
- If the folder that holds the running binary is not writable (for example an old `/usr/local/bin` install), the update changes nothing and says so. Reinstall with the installer to `~/.local/bin`, which needs no `sudo`.

## Configuration

Settings you can change in the REPL are on the `/settings` screen. Values are saved on `esc` and apply at the next session start.

The project keeps its settings in `.claude/settings.json`. Starting a session creates it with the guard hooks when it is missing. `chaude gen-settings` writes one for a given OS.

Main REPL flags (`chaude repl`):

| Flag | Default | Meaning |
|---|---|---|
| `--model` | `opus` | Main model |
| `--cwd` | see `chaude repl -h` | Project folder |
| `--threshold` | `80000` | Rotation threshold for models other than Opus. Opus is fixed at `200000` |
| `--rotate-defer-timeout` | `10m` | How long a due rotation waits for background agents before it runs anyway |
| `--show-rotation` | see `chaude repl -h` | Print a one-line rotation marker |
| `--metrics-dir` | see `chaude repl -h` | Folder for per-turn metrics (JSONL) |

Prompt history is saved per project in `.chaude/history.jsonl`.

The installers read these environment variables:

| Variable | Meaning |
|---|---|
| `CHAUDE_VERSION` | Version to install (default: latest), for example `v0.11.2` |
| `CHAUDE_REPO` | GitHub repo to install from (default: `int-ttt/chaude`) |
| `GH_TOKEN`, `GITHUB_TOKEN` | Optional token for rate limits or private forks. `gh auth token` is used when neither is set |

## Language

The UI and the replies from hooks and agents use one language. Set it in `.chaude/settings.json`:

```json
{ "language": "auto" }
```

- `auto` (default) follows the OS locale. `ko` and `en` force a language.
- The `CHAUDE_LANG` environment variable overrides the setting (`CHAUDE_LANG=en chaude`).
- chaude exports the resolved value to the child Claude sessions it starts, so hooks and agents reply in the same language.

## Effect analysis

### Long project: cost savings (Rust port usage)

Real use of a long-running project during the Rust port: measured 2026-10-28 to 2026-11-05 14:00 UTC on the maintainer's machine: 229 main sessions, 2,039 subagent runs, 22,754 API requests. [scripts/rust_port_usage.py](scripts/rust_port_usage.py) reads the Claude Code transcripts, drops duplicate requests, and prices each request at its model's list price.

| | Actual (Opus, Sonnet, Haiku) | Est. Opus-only | Saved |
|---|---|---|---|
| Total tokens (input, cache write, cache read, output) | 771.8M | 446.0M | - |
| Approx. cost (USD) | 177.60 | 590.51 | 69.9% |

- Method: Opus-only tokens = actual tokens / R, where R = 1.73 is the ping-pong ratio (Opus + Haiku 1,362,928 / Opus-only 787,628 tokens, 2026-11-01). Cost = list-price cost of the same tokens at Opus rates (1,021.83) / R.
- Range: the 2026-11-02 effort re-runs give R from 1.37 to 1.62, so the saving would be 71.9% to 76.2%.
- Assumes the Opus-only run has the same input, cache, and output mix as the measured run. The ping-pong sample is small (2 runs per method).

### Short project: cost savings (ping-pong benchmark)

One small build (a ping-pong game in 3 prompts), chaude compared with plain Opus:

- **Cost: -42.7%** vs plain Opus (USD, mean of 2 runs, 2026-11-01).
- **Cost range: -37.5% to -50.0%** across effort settings (unset, low, medium; re-run 2026-11-02).
- **Total tokens: +73.0%** with Haiku included (2026-11-01). Cost still fell; cache reads dominate the cost.
- **Sample: 2 runs per method, 3 prompts each.** Small sample, so read these as a rough range.

Source: [docs/pingpong-bench.md](docs/pingpong-bench.md).

## Documentation

The user guide is written in Korean.

- [Guide index](docs/guide/index.md)
- [REPL](docs/guide/repl.md): keys, status line, permission modes, slash commands, settings
- [CLI](docs/guide/cli.md): install, subcommands, flags, environment variables, helper scripts
- [Models and agents](docs/guide/models-and-agents.md)
- [Hooks and skills](docs/guide/hooks-and-skills.md)
- [Pingpong benchmark](docs/pingpong-bench.md)
- [Changelog](CHANGELOG.md)
- [Rust port plan](plan/rust-port.md)

## Upgrading from Go

> **Go build removed:** The Go implementation under `deprecated/` was deleted in 0.11.3, the 6th release after 0.10.0. Release archives are built from the Rust workspace at the repo root (`crates/cli`, binary `chaude`) since 0.11.0, with the same asset names. The Go source is in git history at tag `v0.11.2`. The Rust version does not yet cover every Go feature; see `plan/rust-port.md`.


To upgrade, run the installer again or run `chaude update`. The installers now use `~/.local/bin` (macOS/Linux) and `%USERPROFILE%\.local\bin` (Windows). An install at `/usr/local/bin/chaude` or `%LOCALAPPDATA%\Programs\chaude` is not removed automatically; the installer prints how to remove it.

## Contributing

Build (Rust stable, pinned by `rust-toolchain.toml`) and test from the repo root:

```sh
cargo build --release -p chaude-cli   # binary: target/release/chaude
cargo test
```

The Go implementation was removed in 0.11.3. Its source is in git history at tag `v0.11.2`.

Issues: report bugs and request features on [GitHub Issues](https://github.com/int-ttt/chaude/issues).

Contribution guidelines: not published yet.

## License

MIT. See [LICENSE](LICENSE).
