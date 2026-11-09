// Parity with Go harness group 3 (see plan/rust-test-parity-p3.md).
// Only public core API is used here; behaviour that needs crate-private items is mapped in the plan file instead.

use std::time::Duration;

use chaude_core::api::Config;

// Go: resident_extra_test.go TestExtraCoderPoolFromSetting (defaults part). The Go 270 s extra-coder expiry is withdrawn: 3570 s.
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
    assert_eq!(cfg.coder_idle_ttl, Duration::from_secs(3570));
    assert_eq!(cfg.interrupt_grace, Duration::from_secs(20));
    assert_eq!(cfg.inactivity_timeout, Duration::from_secs(300));
    assert_eq!(cfg.rotate_defer_timeout, Duration::from_secs(600));
}

// User rule: main, base coder, extra coders idle out at 3570 s. Regular agents have no idle limit of their own:
// they run inside main and own no Claude session.
#[test]
fn idle_defaults_are_3570_for_every_session() {
    let cfg = Config::default();
    assert_eq!(cfg.idle_rotate, Duration::from_secs(3570));
    assert_eq!(cfg.base_idle_exit_secs, 3570);
    assert_eq!(cfg.coder_idle_ttl, Duration::from_secs(3570));
}
