// Package lean holds helpers shared by the Go scripts and settings generator
// (Claude Code hooks are Python: .claude/hooks/*.py with .claude/hooks/_lean.py).
package lean

import "flag"

// ParseInterspersed parses like Python argparse: flags may appear before or after positionals.
// Returns the positionals in order.
func ParseInterspersed(fs *flag.FlagSet, args []string) ([]string, error) {
	var pos []string
	for {
		if err := fs.Parse(args); err != nil {
			return nil, err
		}
		rest := fs.Args()
		if len(rest) == 0 {
			return pos, nil
		}
		if rest[0] == "--" {
			return append(pos, rest[1:]...), nil
		}
		pos = append(pos, rest[0])
		args = rest[1:]
	}
}
