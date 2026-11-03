package settings

import (
	"errors"
	"fmt"
	"io"
	"io/fs"
	"os"
	"os/exec"
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

// warnOut receives the one-line warning when no hook source is found (tests replace it).
var warnOut io.Writer = os.Stderr

// HookSource returns the folder that holds the guard hook scripts for the running binary, "" when none is found.
// Candidates: <exe>/../.claude/hooks (bin/ layout, the repo build), then <exe>/.claude/hooks (flat layout).
// The release installers (install.sh, install.ps1) put only the chaude binary in place, so an installed chaude
// usually finds nothing here.
func HookSource() string {
	exe, err := os.Executable()
	if err != nil {
		return ""
	}
	if r, err := filepath.EvalSymlinks(exe); err == nil {
		exe = r
	}
	return hookSource(exe)
}

func hookSource(exe string) string {
	dir := filepath.Dir(exe)
	for _, d := range []string{filepath.Join(dir, "..", ".claude", "hooks"), filepath.Join(dir, ".claude", "hooks")} {
		if hasGuardFiles(d) {
			return d
		}
	}
	return ""
}

// hasGuardFiles reports whether every guard hook script exists as a file in dir.
func hasGuardFiles(dir string) bool {
	if dir == "" {
		return false
	}
	for _, f := range guardFiles {
		if st, err := os.Stat(filepath.Join(dir, f)); err != nil || st.IsDir() {
			return false
		}
	}
	return true
}

// EnsureProjectSettings makes a project runnable outside this repo: when <cwd>/.claude/settings.json does not exist,
// it copies the guard hook scripts from srcDir (a .claude/hooks folder, see HookSource) into <cwd>/.claude/hooks
// (existing files are kept) and writes settings.json with the main-thread guard registered. An existing settings.json
// is never touched, and cwd == home is skipped (that is the user-global .claude). When srcDir has no guard scripts,
// one warning line goes to stderr and nothing is written. Returns true when settings.json was created.
func EnsureProjectSettings(cwd, home, srcDir string) (bool, error) {
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
	if !hasGuardFiles(srcDir) {
		fmt.Fprintln(warnOut, "chaude: hook scripts not found next to chaude (main-thread guard skipped, .claude/settings.json not created in "+cwd+")")
		return false, nil
	}
	for _, f := range guardFiles {
		if err := copyNew(filepath.Join(srcDir, f), filepath.Join(cwd, ".claude", "hooks", f)); err != nil {
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

// copyNew copies src to dst unless dst already exists.
func copyNew(src, dst string) error {
	b, err := os.ReadFile(src)
	if err != nil {
		return err
	}
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
