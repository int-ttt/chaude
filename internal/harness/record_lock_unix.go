//go:build !windows

package harness

import (
	"os"
	"path/filepath"
	"syscall"
)

// lockRecordDir takes an exclusive flock on <dir>/.record.lock. The returned func releases it.
func lockRecordDir(dir string) (func(), error) {
	f, err := os.OpenFile(filepath.Join(dir, recordLockName), os.O_CREATE|os.O_RDWR, 0o644)
	if err != nil {
		return nil, err
	}
	if err := syscall.Flock(int(f.Fd()), syscall.LOCK_EX); err != nil {
		f.Close()
		return nil, err
	}
	return func() {
		syscall.Flock(int(f.Fd()), syscall.LOCK_UN)
		f.Close()
	}, nil
}
