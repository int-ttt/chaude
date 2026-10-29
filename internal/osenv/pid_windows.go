//go:build windows

package osenv

import "golang.org/x/sys/windows"

// PidAlive reports whether a process with pid exists and is still running (no signals on Windows:
// open the process with query rights and read its exit code).
func PidAlive(pid int) bool {
	if pid <= 0 {
		return false
	}
	h, err := windows.OpenProcess(windows.PROCESS_QUERY_LIMITED_INFORMATION, false, uint32(pid))
	if err != nil {
		return false
	}
	defer windows.CloseHandle(h)
	var code uint32
	if err := windows.GetExitCodeProcess(h, &code); err != nil {
		return false
	}
	return code == 259 // STILL_ACTIVE
}
