// Single binary for the project: helper scripts, settings generator, REPL (Claude Code hooks stay Python in .claude/hooks/).
// Built by platform_setup/install.sh (mac/linux) or install.ps1 (windows) into bin/chaude[.exe].
package main

import (
	"fmt"
	"os"
	"strings"

	"chaude/internal/clean"
	"chaude/internal/harness"
	"chaude/internal/scripts"
	"chaude/internal/settings"
	"chaude/internal/update"
)

// version is set at release time: -ldflags "-X main.version=X.Y.Z".
var version = "0.10.2"

const usage = `usage: chaude <command> [args]
  repl [flags]           terminal REPL over claude -p stream-json with session rotation (default with no command)
  metrics [--dir D] [--csv]   summarize per-turn metrics (record/metrics)
  --from-claude ID|latest|PATH [--import-dry-run]   start a repl from a plain Claude Code session
  --version              print version
  gen-settings <os>      print/write OS-specific .claude/settings.json (mac|linux|windows) [--out FILE]
  clean [dir] [--yes]    remove the claude-lean harness from a project (dry-run; --yes backs up then removes)
  update                 check GitHub for a newer release and replace this binary (same as /update in the REPL)
  ` + scripts.Names + `   helper scripts (run with -h for flags)
`

func main() {
	harness.ChaudeVersion = version
	update.CleanupOld()
	if len(os.Args) < 2 {
		os.Exit(harness.Main(nil))
	}
	cmd, args := os.Args[1], os.Args[2:]
	switch cmd {
	case "repl":
		os.Exit(harness.Main(args))
	case "metrics":
		os.Exit(harness.MetricsMain(args))
	case "mcp-resident":
		os.Exit(harness.MCPResidentBridge(args))
	case "--from-claude", "--import-dry-run":
		os.Exit(harness.Main(os.Args[1:]))
	case "gen-settings":
		os.Exit(settings.Main(args))
	case "clean":
		os.Exit(clean.Main(args))
	case "update":
		os.Exit(update.Main(version, args))
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
