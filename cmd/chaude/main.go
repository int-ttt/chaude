// Single binary for the project: REPL, project installer, settings generator, helper scripts
// (Claude Code hooks stay Python in .claude/hooks/ and are embedded for `chaude install`).
// Build: go build -o bin/chaude ./cmd/chaude   (windows: -o bin/chaude.exe); releases: scripts/build-release.sh.
package main

import (
	"fmt"
	"os"
	"strings"

	"chaude/internal/harness"
	"chaude/internal/install"
	"chaude/internal/launch"
	"chaude/internal/scripts"
	"chaude/internal/settings"
)

// version is set at release time: -ldflags "-X main.version=X.Y.Z".
var version = "0.8.1"

const usage = `usage: chaude <command> [args]
  repl [flags]           terminal REPL over claude -p stream-json with session rotation (default with no command)
  metrics [--dir D] [--csv]   summarize per-turn metrics (notes/metrics)
  --from-claude ID|latest|PATH [--import-dry-run]   start a repl from a plain Claude Code session
  --version              print version
  install <project> [--main opus|sonnet|fable|haiku] [--python CMD] [--lean-off] [--os windows|mac|linux]
                         install/upgrade the package (agents, hooks, skills, settings, CLAUDE.md policy, bin/chaude) in a project
  open [project] [repl flags]   install there if needed (asks), then start the REPL in that folder
  context-menu install|remove|status   windows: "chaude로 열기" in the folder right-click menu (HKCU)
  gen-settings <os>      print/write OS-specific .claude/settings.json (mac|linux|windows) [--out FILE]
  ` + scripts.Names + `   helper scripts (run with -h for flags)
`

func main() {
	if len(os.Args) < 2 {
		os.Exit(harness.Main(nil))
	}
	cmd, args := os.Args[1], os.Args[2:]
	switch cmd {
	case "repl":
		os.Exit(harness.Main(args))
	case "metrics":
		os.Exit(harness.MetricsMain(args))
	case "--from-claude", "--import-dry-run":
		os.Exit(harness.Main(os.Args[1:]))
	case "install":
		os.Exit(install.Main(args, version))
	case "open":
		os.Exit(launch.Open(args, version))
	case "context-menu":
		os.Exit(launch.ContextMenu(args))
	case "gen-settings":
		os.Exit(settings.Main(args))
	case "--version", "-v", "version":
		fmt.Println("chaude " + version)
	case "-h", "--help", "help":
		fmt.Print(usage)
	default:
		if strings.HasPrefix(cmd, "-") {
			os.Exit(harness.Main(os.Args[1:]))
		}
		if scripts.Has(cmd) {
			os.Exit(scripts.Run(cmd, args))
		}
		fmt.Fprintf(os.Stderr, "unknown command %q\n%s", cmd, usage)
		os.Exit(2)
	}
}
