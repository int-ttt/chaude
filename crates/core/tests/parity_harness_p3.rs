// Parity with Go harness group 3 (see plan/rust-test-parity-p3.md).
// Only public core API is used here; behaviour that needs crate-private items is mapped in the plan file instead.

use std::time::Duration;

use chaude_core::api::Config;

// Go: resident_extra_test.go TestExtraCoderExpiresAt270s, TestExtraCoderPoolFromSetting (defaults part).
// The idle TTL and the default pool size are the public Config defaults the Go values map to.
#[test]
fn default_extra_pool_and_idle_ttl_match_go() {
    // No engine starts here, so the state paths under this dir are never created.
    let dir = std::env::temp_dir().join(format!("chaude-p3-defaults-{}", std::process::id()));
    let cfg = Config {
        agents_file: Some(dir.join("agents.json")),
        clients_file: Some(dir.join("clients.json")),
        ..Config::default()
    };
    assert!(cfg.residents);
    assert_eq!(cfg.extra_coders, 4);
    assert_eq!(cfg.coder_idle_ttl, Duration::from_secs(270));
    assert_eq!(cfg.interrupt_grace, Duration::from_secs(20));
    assert_eq!(cfg.inactivity_timeout, Duration::from_secs(300));
    assert_eq!(cfg.rotate_defer_timeout, Duration::from_secs(600));
}
