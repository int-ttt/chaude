//go:build windows

package harness

import "os"

// pidAlive reports whether a process exists. On Windows os.FindProcess fails for a missing pid.
func pidAlive(pid int) bool {
	if pid <= 0 {
		return false
	}
	_, err := os.FindProcess(pid)
	return err == nil
}
