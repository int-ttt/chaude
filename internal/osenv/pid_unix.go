//go:build !windows

package osenv

import "syscall"

// PidAlive reports whether a process with pid exists (signal 0; EPERM means it exists but is not ours).
func PidAlive(pid int) bool {
	if pid <= 0 {
		return false
	}
	err := syscall.Kill(pid, 0)
	return err == nil || err == syscall.EPERM
}
