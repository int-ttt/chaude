// Package scripts: helper CLIs (ports of scripts/*.py). One file per script.
package scripts

import (
	"fmt"
	"os"
)

// Names lists the script subcommands (for usage text).
const Names = "charcount | claude-history | compress-archive | export-history | extract-transcript | search-history | usage-report"

var registry = map[string]func(args []string) int{}

func register(name string, fn func(args []string) int) { registry[name] = fn }

// Has reports whether name is a script subcommand.
func Has(name string) bool { _, ok := registry[name]; return ok }

// Run executes one script and returns its exit code.
func Run(name string, args []string) int {
	fn, ok := registry[name]
	if !ok {
		fmt.Fprintf(os.Stderr, "chaude: unknown script %q\n", name)
		return 2
	}
	return fn(args)
}
