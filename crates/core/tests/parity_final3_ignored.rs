// Parity stubs for the Go test body parts that the partial rows in plan/rust-test-parity.md still lack
// (final3 workstream). Each test is #[ignore]d with its reason and panics when run with --ignored, so it
// cannot pass by accident. Covered rows were removed and point at their active tests. The remaining
// core items need a test seam. cli items are covered by #[cfg(test)] mods in crates/cli/src/tui (private access);
// the cli rows left here are the ones those mods do not reach yet.
// The covered halves of these rows are real tests in parity_final3.rs, parity_final3_resident.rs and
// parity_p12_harness.rs. The MCP spawn-once and created-once rows are converted in parity_final3_resident.rs.
