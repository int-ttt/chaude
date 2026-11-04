package settings

import (
	"embed"
	"errors"
	"io/fs"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"strings"

	"chaude/internal/osenv"
)

// guardHooks is the project settings guard: the main-thread guard for work tools, and the same script for Agent|Task
// (the main thread may only call haiku-*, opus-architect, claude-code-guide). Same matchers and command form as the
// repo's .claude/settings.json; posix form, windows converted by Windows().
const guardHooks = `{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Read|Write|Edit|MultiEdit|NotebookEdit|Bash|PowerShell|Grep|Glob|WebSearch|WebFetch",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\"",
            "timeout": 5
          }
        ]
      },
      {
        "matcher": "Agent|Task",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\"",
            "timeout": 5
          }
        ]
      }
    ]
  }
}
`

// guardScript is the hook script name the merge looks for to tell that a group is already registered.
const guardScript = "main-thread-guard.py"

// guardFiles are the hook scripts the guard needs (main-thread-guard.py imports _lean.py).
var guardFiles = []string{"main-thread-guard.py", "_lean.py"}

// hookFS holds the package hook scripts (hooks/ is a copy of the repo's .claude/hooks manifest files, see
// TestEmbeddedHooksMatchRepo). The binary carries them, so an installed chaude needs no files next to it.
//
//go:embed hooks/*.py
var hookFS embed.FS

// agentFS holds the package copies of the repo's .claude/agents definitions (haiku-*.md and opus-architect.md, see
// TestEmbeddedAgentsMatchRepo). A first-opened folder gets them, so the main model can delegate to named agents.
//
//go:embed agents/*.md
var agentFS embed.FS

// EmbeddedHook returns the package copy of a hook script (name without folder, e.g. "ledger.py").
func EmbeddedHook(name string) ([]byte, bool) {
	b, err := hookFS.ReadFile(path.Join("hooks", name))
	if err != nil {
		return nil, false
	}
	return b, true
}

// EnsureProjectSettings makes a project runnable outside this repo: it writes the embedded agent definitions into
// <cwd>/.claude/agents and the guard hook scripts into <cwd>/.claude/hooks (existing files are kept, never
// overwritten), then creates <cwd>/.claude/settings.json or merges the guard into an existing one: user keys and
// hooks are kept, the guard groups are appended only when missing, so a second run changes nothing. cwd == home is
// skipped (that is the user-global .claude). Returns true when settings.json was created.
func EnsureProjectSettings(cwd, home string) (bool, error) {
	if cwd == "" {
		return false, nil
	}
	if home != "" {
		if a, errA := os.Stat(cwd); errA == nil {
			if b, errB := os.Stat(home); errB == nil && os.SameFile(a, b) {
				return false, nil
			}
		}
	}
	ents, err := agentFS.ReadDir("agents")
	if err != nil {
		return false, err
	}
	for _, e := range ents {
		b, err := agentFS.ReadFile(path.Join("agents", e.Name()))
		if err != nil {
			return false, err
		}
		if err := copyNew(b, filepath.Join(cwd, ".claude", "agents", e.Name())); err != nil {
			return false, err
		}
	}
	for _, f := range guardFiles {
		b, ok := EmbeddedHook(f)
		if !ok {
			return false, errors.New("chaude: embedded hook missing: " + f)
		}
		if err := copyNew(b, filepath.Join(cwd, ".claude", "hooks", f)); err != nil {
			return false, err
		}
	}
	txt := []byte(guardHooks)
	if osenv.IsWindows {
		var err error
		if txt, err = Windows(txt, hookPython()); err != nil {
			return false, err
		}
	}
	dst := filepath.Join(cwd, ".claude", "settings.json")
	old, err := os.ReadFile(dst)
	if errors.Is(err, fs.ErrNotExist) {
		if err := writeNew(dst, txt); err != nil {
			return false, err
		}
		return true, nil
	}
	if err != nil {
		return false, err
	}
	merged, changed, err := mergeGuard(old, txt)
	if err != nil || !changed {
		return false, err
	}
	return false, os.WriteFile(dst, merged, 0o644)
}

// mergeGuard adds the guard PreToolUse groups from guard to an existing settings text. Order and keys of old are kept;
// a group is added only when no group with the same matcher already runs the guard script. changed is false when
// nothing was added. Text that is not a JSON object (or whose hooks/PreToolUse have the wrong shape) is an error,
// and the caller leaves the file as it is.
func mergeGuard(old, guard []byte) ([]byte, bool, error) {
	cfg, err := parse(old)
	if err != nil {
		return nil, false, err
	}
	if cfg.kind != 'o' {
		return nil, false, errors.New("settings.json: top level is not an object")
	}
	g, err := parse(guard)
	if err != nil {
		return nil, false, err
	}
	gpre := g.get("hooks").get("PreToolUse")
	hooks := cfg.get("hooks")
	changed := false
	if hooks == nil {
		cfg.insertAfter("", "hooks", g.get("hooks"))
		changed = true
	} else if hooks.kind != 'o' {
		return nil, false, errors.New("settings.json: hooks is not an object")
	} else {
		pre := hooks.get("PreToolUse")
		if pre == nil {
			hooks.insertAfter("", "PreToolUse", gpre)
			changed = true
		} else if pre.kind != 'a' {
			return nil, false, errors.New("settings.json: hooks.PreToolUse is not an array")
		} else {
			for _, gg := range gpre.items {
				if hasGuardGroup(pre, gg.get("matcher").str()) {
					continue
				}
				pre.items = append(pre.items, gg)
				changed = true
			}
		}
	}
	if !changed {
		return old, false, nil
	}
	var b strings.Builder
	cfg.write(&b, "")
	b.WriteString("\n")
	return []byte(b.String()), true, nil
}

// hasGuardGroup reports whether a PreToolUse group with this matcher already runs the guard script.
func hasGuardGroup(pre *node, matcher string) bool {
	for _, grp := range pre.items {
		if grp.get("matcher").str() != matcher {
			continue
		}
		hs := grp.get("hooks")
		if hs == nil {
			continue
		}
		for _, h := range hs.items {
			if strings.Contains(hookText(h), guardScript) {
				return true
			}
		}
	}
	return false
}

// hookText is the command line of a hook entry (posix command, or the windows command + args).
func hookText(h *node) string {
	s := h.get("command").str()
	if a := h.get("args"); a != nil {
		for _, it := range a.items {
			s += " " + it.str()
		}
	}
	return s
}

// hookPython is the windows hook interpreter for the guard command: the first of python3, python, py -3 found on PATH
// (exec.LookPath), "python" when none is found.
func hookPython() string {
	for _, c := range []struct{ exe, cmd string }{{"python3", "python3"}, {"python", "python"}, {"py", "py -3"}} {
		if _, err := exec.LookPath(c.exe); err == nil {
			return c.cmd
		}
	}
	return "python"
}

// copyNew writes b to dst unless dst already exists.
func copyNew(b []byte, dst string) error {
	if _, err := os.Lstat(dst); err == nil {
		return nil
	}
	return writeNew(dst, b)
}

// writeNew creates dst (and its folder) with O_EXCL, so an existing file is never overwritten.
func writeNew(dst string, b []byte) error {
	if err := os.MkdirAll(filepath.Dir(dst), 0o755); err != nil {
		return err
	}
	f, err := os.OpenFile(dst, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o644)
	if err != nil {
		return err
	}
	if _, err := f.Write(b); err != nil {
		f.Close()
		return err
	}
	return f.Close()
}
