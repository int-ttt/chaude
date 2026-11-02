task: CI test workflow + trailer fix
status: done
run: https://github.com/int-ttt/chaude/actions/runs/41298649279 (success)
commits:
  3f10dc2: 226e870 rewritten, trailer Haiku 5.5 -> Opus 5.5
  f66d565: f345493 rebased only (parent change)
  9a63eab: .github/workflows/test.yml, .gitattributes, testdata/smoke/*
  88e773f: test fixes (Windows/macOS)
per-os (round 2): ubuntu PASS, windows PASS, macos PASS
bugs fixed:
  resident_coder.go:98: "/x" rooted path joined under cwd on Windows (allowed /etc/hosts) -> treat leading / or \ as outside
  dispatch_test.go: pid 1 assumption -> os.Getppid()
  sessview_test.go: Windows home needs USERPROFILE
  macOS: go1.22.12 harness.test dyld "missing LC_UUID" abort -> CI uses go 1.23 on macOS only (root cause not confirmed; local 1.23.5 OK)
rounds: 2 of 3 used
unresolved: macOS toolchain deviation from go.mod (go-version-file) is macOS-only; revisit when go1.22 fixed
