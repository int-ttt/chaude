# 261104 haiku release 0.10.1
status: released, Mac deploy BLOCKED
commit: a6b60ec release: 0.10.1 (pushed rust-port; includes fix 2b4bb2b)
tag: v0.10.1 -> a6b60ec (annotated, pushed)
workflow: release.yml run 41748183543 (push tag) -> completed success
assets: chaude_0.10.1_darwin_arm64.tar.gz, _linux_amd64.tar.gz, _linux_arm64.tar.gz, _windows_amd64.zip, checksums.txt
lock: Cargo.lock diff = 5 workspace member version lines only
checks: go build ./... PASS; go test ./cmd/... no test files; go vet ./... PASS (earlier, 2b4bb2b); workflow success
mac deploy:
- no 0.10.0 deploy record in notes/ or record/ (prep note only: scratch copy, no install)
- ~/.local/bin/chaude: absent now (older notes used it; install.sh default later moved to /usr/local/bin)
- live: /usr/local/bin/chaude, root:wheel 0.9.15 (Nov 3 21:48), not writable by leekwangseok
- running: pid 4465, 22607 (`/usr/local/bin/chaude mcp-resident ...`), pid 22547 `chaude`
- not done: overwrite needs sudo and would replace a binary in use; not killed per instruction
- exact cmd when allowed: sudo install -m 755 <extracted chaude> /usr/local/bin/chaude && /usr/local/bin/chaude --version (expect chaude 0.10.1)
doubts: deploy path ambiguous (no 0.10.0 deploy record); running resident processes use old binary until restarted
