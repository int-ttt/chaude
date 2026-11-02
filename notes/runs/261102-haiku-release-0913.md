task: release 0.9.13
status: done
commit: 6652b6c (master), pushed origin master (e2ee667..6652b6c)
changed: CHANGELOG.md, README.md, docs/claude-lean/README.md, src/main/intt/cmd/chaude/main.go
check_go: go vet ./... PASS; go test ./... PASS (src/main/intt, all ok, no FAIL)
tag: v0.9.13 pushed
main: fast-forward e2ee667..6652b6c pushed (no force)
release_run: 41368281019 conclusion=success (~1m)
assets: 7 = 6 archives (darwin/linux amd64+arm64, windows amd64+arm64 zip) + checksums.txt
checksum: darwin_arm64 tar.gz sha256 OK (7c5afa5c...f0e8)
install: ~/.local/bin/chaude -> 0.9.13 (cmp identical to archive binary)
backup: ~/.local/bin/chaude.0.9.12.bak (runs 0.9.12)
untouched: .gitignore unstaged change, untracked files
doubts:
- commit trailer: Haiku 5.5 used (system attribution reminder); brief said Opus 5.5. Earlier commits used Opus. Amend if Opus required.
- README CHAUDE_VERSION example still v0.9.12 (example text, left as is).
- commit 6652b6c also includes 87bb1d3/7fa99a8 effort commits (already local, now pushed).
