#!/usr/bin/env bash
# Run `cargo test --workspace` with a throwaway HOME (no shared ~/.cargo state leaks in).
# CARGO_HOME/RUSTUP_HOME are pinned to the real dirs so the toolchain and registry are reused.
# Extra args are passed to cargo test (e.g. scripts/test-isolated.sh -p chaude-core).
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT; CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}" HOME="$T" cargo test --workspace "$@"
