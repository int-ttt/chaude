//go:build windows

package harness

import (
	"os"
	"path/filepath"

	"golang.org/x/sys/windows"
)

// lockRecordDir takes an exclusive LockFileEx lock on <dir>/.record.lock. The returned func releases it.
func lockRecordDir(dir string) (func(), error) {
	f, err := os.OpenFile(filepath.Join(dir, recordLockName), os.O_CREATE|os.O_RDWR, 0o644)
	if err != nil {
		return nil, err
	}
	ol := new(windows.Overlapped)
	if err := windows.LockFileEx(windows.Handle(f.Fd()), windows.LOCKFILE_EXCLUSIVE_LOCK, 0, 1, 0, ol); err != nil {
		f.Close()
		return nil, err
	}
	return func() {
		windows.UnlockFileEx(windows.Handle(f.Fd()), 0, 1, 0, ol)
		f.Close()
	}, nil
}
