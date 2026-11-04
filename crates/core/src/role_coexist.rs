//! Go TestChildKeyRoleCoexist (harness/childkey_test.go). Test-only: the role rule lives in
//! child_set::child_key (an empty role gives the role-less key, a non-empty role adds a field).
//! This checks that the same model under two roles gets two distinct parked children.

use crate::child_set::{ChildSet, Close, Parked, child_key};
use crate::testutil::TempDir;
use std::time::SystemTime;

struct NoopClose;

impl Close for NoopClose {
    fn close(&mut self) {}
}

fn parked(model: &str, now: SystemTime) -> Parked<NoopClose> {
    Parked {
        sess: NoopClose,
        model: model.to_string(),
        last_api: now,
        last_ctx: 0,
    }
}

// Go TestChildKeyRoleCoexist
#[test]
fn same_model_with_role_and_without_role_coexist_when_parked() {
    let dir = TempDir::new("role-coexist");
    let cwd = dir.path().to_string_lossy().to_string();
    let plain = child_key(&cwd, "sonnet", "");
    let worker = child_key(&cwd, "sonnet", "worker");
    assert_ne!(plain, worker, "same model with different role shares a key");

    let set: ChildSet<NoopClose> = ChildSet::new();
    let now = SystemTime::now();
    set.park(&plain, parked("sonnet", now));
    set.park(&worker, parked("sonnet", now));
    assert_eq!(
        set.len(),
        2,
        "parked children must coexist (roles must not collide)"
    );
}
