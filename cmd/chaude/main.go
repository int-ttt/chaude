// Single binary for the project: helper scripts, settings generator, REPL (Claude Code hooks stay Python in .claude/hooks/).
// Built by platform_setup/install.sh (mac/linux) or install.ps1 (windows) into bin/chaude[.exe].
package main

import (
	"fmt"
	"os"

	"chaude/internal/harness"
	"chaude/internal/scripts"
	"chaude/internal/settings"
)

// version is set at release time: -ldflags "-X main.version=X.Y.Z".
var version = "0.6.3"

const usage = `usage: chaude <command> [args]
  repl [flags]           terminal REPL over claude -p stream-json with session rotation
  metrics [--dir D] [--csv]   summarize per-turn metrics (notes/metrics)
  --from-claude ID|latest|PATH [--import-dry-run]   start a repl from a plain Claude Code session
  --version              print version
  gen-settings <os>      print/write OS-specific .claude/settings.json (mac|linux|windows) [--out FILE]
  ` + scripts.Names + `   helper scripts (run with -h for flags)
`

func main() {
	if len(os.Args) < 2 {
		fmt.Fprint(os.Stderr, usage)
		os.Exit(2)
	}
	cmd, args := os.Args[1], os.Args[2:]
	switch cmd {
	case "repl":
		os.Exit(harness.Main(args))
	case "metrics":
		os.Exit(harness.MetricsMain(args))
	case "--from-claude", "--import-dry-run":
		os.Exit(harness.Main(os.Args[1:]))
	case "gen-settings":
		os.Exit(settings.Main(args))
	case "--version", "-v", "version":
		fmt.Println("chaude " + version)
	case "-h", "--help", "help":
		fmt.Print(usage)
	default:
		if scripts.Has(cmd) {
			os.Exit(scripts.Run(cmd, args))
		}
		fmt.Fprintf(os.Stderr, "unknown command %q\n%s", cmd, usage)
		os.Exit(2)
	}
}
