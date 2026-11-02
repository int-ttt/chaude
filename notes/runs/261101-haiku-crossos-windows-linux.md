# 261101 haiku crossos windows-linux
status: done (record_turn item deviated, see doubts)
changed:
- internal/harness/resume.go: claudeProjectDir uses local nonAlnum regex ([^A-Za-z0-9] -> "-")
- internal/harness/version.go, usagepoll.go, tui.go: claude exec via osenv.ResolveBin("claude")
- internal/harness/resident_mcp.go: listenMCP unix, fallback tcp 127.0.0.1:0 ("tcp:<addr>" arg); bridge dials tcp when prefix; CloseWrite via interface
- internal/harness/resident_sock_windows.go (new): sockBaseDir = GetShortPathNameW(os.TempDir())
- internal/harness/resident_sock_other.go (new): sockBaseDir = ""
- tools/job_monitor.py: win_pid_alive (OpenProcess/GetExitCodeProcess ctypes); pid_alive nt branch; proc_etime returns "" on nt
- scripts/build-release.sh: default 6 TARGETS; windows .exe in .zip; checksums over tar.gz+zip
- .github/workflows/release.yml: gh release upload adds dist/*.zip
- install.sh: darwin/linux x amd64/arm64; windows -> platform_setup\install.ps1; sha256 fallback; rc by SHELL
- README.md: install section mac/linux/windows; release targets line
- NOT changed: internal/harness/record_turn.go (see doubts)
checks:
- gofmt -l . -> clean
- go vet ./... -> PASS
- go test ./... -> PASS (internal/harness ok)
- go build ./... GOOS={linux,windows,darwin} GOARCH={amd64,arm64} -> 6/6 PASS
- GOOS=windows go vet ./internal/harness/ -> PASS
- sh -n install.sh, sh -n scripts/build-release.sh -> PASS
- build-release.sh in temp copy TARGETS="windows/amd64 linux/arm64" -> zip(chaude.exe) + tar.gz + checksums PASS
- python3 -I job_monitor.py --pid $$ --once -> PASS (posix path); win branch parse only
doubts:
- record_turn.go:14 filepath.IsAbs("/usage") is false on Windows -> slash cmds would be recorded; test TestRecordSkip expects "/resume" true. kept HasPrefix "/" (already OS-neutral).
- tcp fallback binds loopback, any local process can connect to MCP socket (applied on all OS, not only windows).
- win_pid_alive untested on real Windows.
- install.sh rc for bash on mac now .bash_profile (was .zshrc only).
- release.yml gh upload glob dist/*.zip fails if no windows target.
