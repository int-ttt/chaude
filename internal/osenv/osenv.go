// Package osenv is the only place that knows OS differences. Everything else stays OS-neutral.
package osenv

import (
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
)

var (
	IsWindows = runtime.GOOS == "windows"
	IsMac     = runtime.GOOS == "darwin"
)

// Name is "windows", "mac" or "linux".
func Name() string {
	switch {
	case IsWindows:
		return "windows"
	case IsMac:
		return "mac"
	}
	return "linux"
}

// ResolveBin returns the full path (finds claude.cmd/.exe on Windows); falls back to the bare name.
func ResolveBin(name string) string {
	if p, err := exec.LookPath(name); err == nil {
		return p
	}
	return name
}

func home() string {
	h, err := os.UserHomeDir()
	if err != nil {
		return "."
	}
	return h
}

// StateDir: windows %LOCALAPPDATA%; linux XDG state; mac unchanged ~/.claude-harness (compat).
func StateDir() string {
	h := home()
	if IsWindows {
		if d := os.Getenv("LOCALAPPDATA"); d != "" {
			return filepath.Join(d, "claude-harness")
		}
		return filepath.Join(h, "claude-harness")
	}
	if IsMac {
		return filepath.Join(h, ".claude-harness")
	}
	if d := os.Getenv("XDG_STATE_HOME"); d != "" {
		return filepath.Join(d, "claude-harness")
	}
	return filepath.Join(h, ".local", "state", "claude-harness")
}
