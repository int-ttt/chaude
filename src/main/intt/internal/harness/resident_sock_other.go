//go:build !windows

package harness

// sockBaseDir: "" makes os.MkdirTemp use the default temp dir.
func sockBaseDir() string { return "" }
