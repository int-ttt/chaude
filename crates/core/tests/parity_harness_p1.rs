// Parity with Go harness/dispatch_test.go: TestMaxWorkersDefault, TestMaxWorkersEnvOverride,
// TestMaxWorkersRejectZero. Uses only the public api::resolve_max_workers (no crate-internal items).

use chaude_core::api::resolve_max_workers;

#[test]
fn max_workers_env_override_and_flag_precedence() {
    // Go: env "5" -> 5; flag beats env.
    assert_eq!(resolve_max_workers(None, "5", 0), Ok(5));
    assert_eq!(resolve_max_workers(Some(2), "5", 0), Ok(2));
}

#[test]
fn max_workers_rejects_zero_from_flag_and_env() {
    // Go: flag 0 -> error; env "0" -> error.
    assert!(resolve_max_workers(Some(0), "", 0).is_err());
    assert!(resolve_max_workers(None, "0", 0).is_err());
}

#[test]
fn max_workers_default_is_three() {
    assert_eq!(resolve_max_workers(None, "", 0), Ok(3));
}
