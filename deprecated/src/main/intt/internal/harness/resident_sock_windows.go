//go:build windows

package harness

import (
	"os"
	"syscall"
	"unsafe"
)

var getShortPathName = syscall.NewLazyDLL("kernel32.dll").NewProc("GetShortPathNameW")

// sockBaseDir returns the 8.3 short form of the temp dir: AF_UNIX paths are length-limited on Windows.
func sockBaseDir() string {
	dir := os.TempDir()
	p, err := syscall.UTF16PtrFromString(dir)
	if err != nil {
		return dir
	}
	buf := make([]uint16, 260)
	n, _, _ := getShortPathName.Call(uintptr(unsafe.Pointer(p)), uintptr(unsafe.Pointer(&buf[0])), uintptr(len(buf)))
	if n == 0 || n >= uintptr(len(buf)) {
		return dir
	}
	return syscall.UTF16ToString(buf[:n])
}
