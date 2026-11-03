# deprecated/

Deprecated code. Nothing here is the successor to build on.

## Contents

- `src/main/intt/` — the Go implementation of chaude (module `intt`, `go.work` at this level). Still the binary shipped in GitHub Releases.
- `prototype/chaude-rs/` — the retired Rust stream-json spike. Own `[workspace]` root; not part of the repo-root Cargo workspace.
- `imports/`, `notes/`, `scripts/`, `tests/`, `bin/`, `chaude` — Go-era leftovers kept as they were.

## Policy

deprecated/ (Go) is kept through the 5 releases after 0.10.0; delete in the 6th release after 0.10.0.

The Go binary keeps shipping until the Rust port reaches parity (see `plan/rust-port.md`, phase 6).

## Commands

```sh
cd deprecated/src/main/intt && go vet ./... && go test ./...   # CI: .github/workflows/test.yml
scripts/build-release.sh 0.10.0                                  # from repo root; writes dist/
```

Dev build into `bin/chaude` from repo root: `platform_setup/install.sh` (mac/linux) or `platform_setup/install.ps1` (windows).
