//go:build !windows

package harness

import "syscall"

// pidAlive reports whether a process exists (signal 0).
func pidAlive(pid int) bool {
	if pid <= 0 {
		return false
	}
	err := syscall.Kill(pid, 0)
	return err == nil || err == syscall.EPERM
}
