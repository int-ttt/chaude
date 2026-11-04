//! Dispatch decision for a (cwd, model) child (Go harness dispatch.go: pickChild; clients.go: isWarm).
//! The caller looks up the live child (ChildSet::warm_info) and the registry row for (cwd, model),
//! then asks what to do. Pure: no process or file access except the owner liveness probe.

use std::time::{Duration, SystemTime};

use crate::handoff::pid_alive;

/// What dispatch should do to reach a (cwd, model) child (Go childAction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildAction {
    /// No warm child anywhere: start a fresh claude -p.
    StartNew,
    /// A parked-live child in this process is still warm: reuse it.
    UseLive,
    /// A parked registry row is still warm: resume its session id.
    Resume,
}

/// The registry row fields pickChild reads (Go clientRec for (cwd, model)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegRow {
    /// State is `parked` (not `live` or `expired`).
    pub parked: bool,
    pub session_id: String,
    /// Harness process that owns the row. 0 = no owner.
    pub owner_pid: u32,
    pub last_api: SystemTime,
    pub last_ctx: u64,
}

/// Go isWarm for a live or parked child: idle for less than `deadline` and context below `limit`.
/// A last-API time after `now` counts as warm, as Go's negative Sub does.
pub fn is_warm(
    last_api: SystemTime,
    last_ctx: u64,
    limit: u64,
    now: SystemTime,
    deadline: Duration,
) -> bool {
    let fresh = match now.duration_since(last_api) {
        Ok(age) => age < deadline,
        Err(_) => true,
    };
    fresh && last_ctx < limit
}

/// Go pickChild. A live child (`live` = its last-API time and context) wins when warm. Otherwise a
/// registry row that is parked, has a session id, is warm, and is not owned by another live
/// process resumes. Everything else starts new.
pub fn pick_child(
    live: Option<(SystemTime, u64)>,
    row: Option<&RegRow>,
    limit: u64,
    now: SystemTime,
    deadline: Duration,
) -> ChildAction {
    if let Some((last_api, last_ctx)) = live
        && is_warm(last_api, last_ctx, limit, now, deadline)
    {
        return ChildAction::UseLive;
    }
    let Some(row) = row else {
        return ChildAction::StartNew;
    };
    if !row.parked || row.session_id.is_empty() {
        return ChildAction::StartNew;
    }
    let self_pid = std::process::id();
    if row.owner_pid != 0 && row.owner_pid != self_pid && pid_alive(row.owner_pid) {
        return ChildAction::StartNew; // another running chaude owns this row
    }
    if !is_warm(row.last_api, row.last_ctx, limit, now, deadline) {
        return ChildAction::StartNew;
    }
    ChildAction::Resume
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: u64 = 80_000;
    const DEADLINE: Duration = Duration::from_secs(3600);

    fn row(last_api: SystemTime, last_ctx: u64) -> RegRow {
        RegRow {
            parked: true,
            session_id: "s1".to_string(),
            owner_pid: 0,
            last_api,
            last_ctx,
        }
    }

    // Go TestPickChild (live part and registry part, one case per branch)
    #[test]
    fn pick_child_matches_go_cases() {
        let now = SystemTime::now();
        let ago = |s: u64| now - Duration::from_secs(s);

        assert_eq!(
            pick_child(None, None, LIMIT, now, DEADLINE),
            ChildAction::StartNew,
            "empty"
        );

        let live = Some((now, 100));
        assert_eq!(
            pick_child(live, None, LIMIT, now, DEADLINE),
            ChildAction::UseLive,
            "live warm"
        );
        assert_eq!(
            pick_child(Some((now, LIMIT + 1)), None, LIMIT, now, DEADLINE),
            ChildAction::StartNew,
            "live over limit"
        );

        assert_eq!(
            pick_child(None, Some(&row(now, 10)), LIMIT, now, DEADLINE),
            ChildAction::Resume,
            "parked warm"
        );
        assert_eq!(
            pick_child(None, Some(&row(ago(7200), 10)), LIMIT, now, DEADLINE),
            ChildAction::StartNew,
            "parked expired"
        );
        assert_eq!(
            pick_child(None, Some(&row(now, LIMIT)), LIMIT, now, DEADLINE),
            ChildAction::StartNew,
            "parked over limit"
        );
    }

    // Go TestPickChild: owned by another live process (Go uses the parent pid; pid 1 is always alive
    // and is not this process, which keeps the case without FFI)
    #[test]
    fn pick_child_refuses_row_owned_by_other_live_pid() {
        let now = SystemTime::now();
        let mut r = row(now, 10);
        r.owner_pid = 1;
        assert_eq!(
            pick_child(None, Some(&r), LIMIT, now, DEADLINE),
            ChildAction::StartNew
        );
    }

    #[test]
    fn pick_child_ignores_unparked_or_sessionless_rows() {
        let now = SystemTime::now();
        let mut r = row(now, 10);
        r.parked = false;
        assert_eq!(
            pick_child(None, Some(&r), LIMIT, now, DEADLINE),
            ChildAction::StartNew
        );
        let mut r = row(now, 10);
        r.session_id.clear();
        assert_eq!(
            pick_child(None, Some(&r), LIMIT, now, DEADLINE),
            ChildAction::StartNew
        );
    }

    // Go pickChild compares against thresholdFor(target): an Opus child at 150k stays warm, a Haiku child does not.
    #[test]
    fn pick_child_uses_threshold_for_model() {
        let now = SystemTime::now();
        let live = Some((now, 150_000));
        let opus = crate::rotate::threshold_for("claude-opus-5-5", LIMIT);
        let haiku = crate::rotate::threshold_for("haiku", LIMIT);
        assert_eq!(opus, 200_000);
        assert_eq!(
            pick_child(live, None, opus, now, DEADLINE),
            ChildAction::UseLive
        );
        assert_eq!(
            pick_child(live, None, haiku, now, DEADLINE),
            ChildAction::StartNew
        );
    }

    #[test]
    fn is_warm_treats_future_last_api_as_warm() {
        let now = SystemTime::now();
        assert!(is_warm(
            now + Duration::from_secs(5),
            0,
            LIMIT,
            now,
            DEADLINE
        ));
        assert!(
            !is_warm(now, 0, 0, now, DEADLINE),
            "ctx must be below limit"
        );
    }
}
