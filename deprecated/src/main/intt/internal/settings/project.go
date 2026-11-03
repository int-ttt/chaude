package settings

import (
	"embed"
	"errors"
	"io/fs"
	"os"
	"os/exec"
	"path"
	"path/filepath"

	"chaude/internal/osenv"
)

// guardHooks is the project settings written when a folder has no .claude/settings.json: the main-thread guard only
// (same matcher and command form as the repo's .claude/settings.json; posix form, windows converted by Windows()).
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
      }
    ]
  }
}
`

// guardFiles are the hook scripts the guard needs (main-thread-guard.py imports _lean.py).
var guardFiles = []string{"main-thread-guard.py", "_lean.py"}

// hookFS holds the package hook scripts (hooks/ is a copy of the repo's .claude/hooks manifest files, see
// TestEmbeddedHooksMatchRepo). The binary carries them, so an installed chaude needs no files next to it.
//
//go:embed hooks/*.py
var hookFS embed.FS

// EmbeddedHook returns the package copy of a hook script (name without folder, e.g. "ledger.py").
func EmbeddedHook(name string) ([]byte, bool) {
	b, err := hookFS.ReadFile(path.Join("hooks", name))
	if err != nil {
		return nil, false
	}
	return b, true
}

// EnsureProjectSettings makes a project runnable outside this repo: when <cwd>/.claude/settings.json does not exist,
// it writes the guard hook scripts from the embedded package copy into <cwd>/.claude/hooks (existing files are kept)
// and writes settings.json with the main-thread guard registered. An existing settings.json is never touched, and
// cwd == home is skipped (that is the user-global .claude). Returns true when settings.json was created.
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
	dst := filepath.Join(cwd, ".claude", "settings.json")
	if _, err := os.Lstat(dst); err == nil {
		return false, nil
	} else if !errors.Is(err, fs.ErrNotExist) {
		return false, err
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
	if err := writeNew(dst, txt); err != nil {
		return false, err
	}
	return true, nil
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
