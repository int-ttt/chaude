//! Rotation deferral while background agents run (Go rotate_defer.go) and the background-agent lines of the
//! handoff note. Pure state: the actor counts its running background agents, calls hold() when a rotation is
//! due, and runs the held rotation once is_due() holds. Logging and the rotation itself stay with the actor.

use std::time::{Duration, Instant};

// Go rotateDeferDue: a rotation may proceed when nothing runs, deferral is off (timeout zero), or the hold
// started at `since` has lasted at least the timeout.
pub fn due(running: usize, since: Option<Instant>, now: Instant, timeout: Duration) -> bool {
    if running == 0 || timeout.is_zero() {
        return true;
    }
    since.is_some_and(|s| now.saturating_duration_since(s) >= timeout)
}

// What hold() decided. The caller logs the event (rotate_defer) and prints the notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// The rotation proceeds now. timed_out: a hold ended by the timeout (not by the agents finishing).
    Proceed { timed_out: bool },
    /// The rotation waits. first: this call started the hold.
    Held { first: bool },
}

// Go deferredRot and deferSince: the held rotation (reason) and when its hold started.
#[derive(Debug, Default)]
pub struct Deferral {
    since: Option<Instant>,
    reason: Option<String>,
}

impl Deferral {
    // Go deferRotate: holds a rotation while background agents run, for at most the timeout.
    // running is the count of unfinished background agents of the current session.
    pub fn hold(&mut self, running: usize, reason: &str, now: Instant, timeout: Duration) -> Hold {
        if due(running, self.since, now, timeout) {
            let timed_out = running > 0 && self.since.is_some();
            self.since = None;
            self.reason = None;
            return Hold::Proceed { timed_out };
        }
        let first = self.since.is_none();
        if first {
            self.since = Some(now);
        }
        self.reason = Some(reason.to_string());
        Hold::Held { first }
    }

    // Go deferredDue: a held rotation can run now (UI tick, no agent work).
    pub fn is_due(&self, running: usize, now: Instant, timeout: Duration) -> bool {
        self.reason.is_some() && due(running, self.since, now, timeout)
    }

    // Go runDeferredRotate: takes the held reason so the caller can run the rotation with it. Like Go, this
    // clears only the reason; the hold start is cleared by the next hold() call.
    pub fn take(&mut self) -> Option<String> {
        self.reason.take()
    }

    pub fn pending(&self) -> bool {
        self.reason.is_some()
    }
}

// What the handoff note lists about a delegated agent (Go agentRec: Task, ResultPath, Background).
#[derive(Debug, Clone, Copy)]
pub struct BgAgent<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub task: &'a str,
    pub started: Instant,
    pub background: bool,
    pub done: bool,
    // The result file has been written (a finished agent's result block).
    pub result_done: bool,
    pub result_path: &'a str,
}

// Go bgRunning count: background agents that have not finished.
pub fn running_count(agents: &[BgAgent]) -> usize {
    agents.iter().filter(|a| a.background && !a.done).count()
}

// Go bgHandoffLines: background agents with a result file, running or finished-with-result, oldest first.
// Each line names the agent, its state, and its result file path.
pub fn bg_handoff_lines(agents: &[BgAgent]) -> Vec<String> {
    let mut listed: Vec<&BgAgent> = agents
        .iter()
        .filter(|a| a.background && !a.result_path.is_empty() && (!a.done || a.result_done))
        .collect();
    listed.sort_by(|a, b| a.started.cmp(&b.started).then_with(|| a.id.cmp(b.id)));
    listed
        .into_iter()
        .map(|a| {
            let state = if a.done { "done" } else { "running" };
            let name = if a.name.is_empty() { a.task } else { a.name };
            format!(
                "- {name} ({}): {state}, result file: {}",
                a.id, a.result_path
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN10: Duration = Duration::from_secs(600);

    #[test]
    fn due_cases() {
        let now = Instant::now();
        let min = |m: u64| now + Duration::from_secs(60 * m);
        assert!(due(0, None, now, MIN10), "nothing running");
        assert!(due(2, None, now, Duration::ZERO), "deferral off");
        assert!(!due(1, None, now, MIN10), "running, not yet held");
        assert!(!due(1, Some(now), min(9), MIN10), "running, held 9m");
        assert!(due(1, Some(now), min(10), MIN10), "running, held 10m");
        assert!(due(0, Some(now), min(1), MIN10), "finished while held");
    }

    #[test]
    fn held_while_agent_runs_then_due_at_timeout() {
        let t0 = Instant::now();
        let mut d = Deferral::default();
        assert_eq!(
            d.hold(1, "ctx 9 >= 8", t0, MIN10),
            Hold::Held { first: true }
        );
        assert_eq!(
            d.hold(1, "ctx 9 >= 8", t0 + Duration::from_secs(60), MIN10),
            Hold::Held { first: false }
        );
        assert!(d.pending());
        assert!(!d.is_due(1, t0, MIN10));
        assert!(d.is_due(1, t0 + MIN10, MIN10));
    }

    #[test]
    fn released_when_agent_finishes() {
        let t0 = Instant::now();
        let mut d = Deferral::default();
        d.hold(1, "manual", t0, MIN10);
        assert!(d.is_due(0, t0, MIN10), "due after the agent finished");
        assert_eq!(d.take().as_deref(), Some("manual"));
        assert!(!d.pending());
        assert_eq!(
            d.hold(0, "manual", t0, MIN10),
            Hold::Proceed { timed_out: false }
        );
        assert!(d.since.is_none() && d.reason.is_none());
    }

    #[test]
    fn timeout_proceeds_and_reports_it() {
        let t0 = Instant::now();
        let mut d = Deferral::default();
        d.hold(1, "ctx 9 >= 8", t0, MIN10);
        let later = t0 + Duration::from_secs(660);
        assert_eq!(
            d.hold(1, "ctx 9 >= 8", later, MIN10),
            Hold::Proceed { timed_out: true }
        );
        assert!(!d.pending());
        assert!(d.since.is_none());
    }

    #[test]
    fn deferral_off_never_holds() {
        let t0 = Instant::now();
        let mut d = Deferral::default();
        assert_eq!(
            d.hold(3, "manual", t0, Duration::ZERO),
            Hold::Proceed { timed_out: false }
        );
        assert!(!d.pending());
    }

    fn agent<'a>(id: &'a str, started: Instant) -> BgAgent<'a> {
        BgAgent {
            id,
            name: "haiku-coder",
            task: "write notes",
            started,
            background: true,
            done: false,
            result_done: false,
            result_path: "notes/runs/261101-agent-x.md",
        }
    }

    #[test]
    fn handoff_lines_list_running_and_finished_with_result() {
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_secs(5);
        let bg1 = BgAgent {
            done: true,
            result_done: true,
            ..agent("bg1", t0)
        };
        let bg2 = BgAgent {
            name: "",
            result_path: "notes/runs/261101-agent-bg2.md",
            ..agent("bg2", t1)
        };
        let fg = BgAgent {
            background: false,
            ..agent("fg", t0)
        };
        let no_file = BgAgent {
            result_path: "",
            ..agent("nofile", t0)
        };
        let finished_unwritten = BgAgent {
            done: true,
            result_done: false,
            ..agent("bg3", t0)
        };
        let lines = bg_handoff_lines(&[bg2, fg, bg1, no_file, finished_unwritten]);
        assert_eq!(
            lines,
            vec![
                "- haiku-coder (bg1): done, result file: notes/runs/261101-agent-x.md".to_string(),
                "- write notes (bg2): running, result file: notes/runs/261101-agent-bg2.md"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn running_count_skips_finished_and_foreground() {
        let t0 = Instant::now();
        let agents = [
            agent("a", t0),
            BgAgent {
                done: true,
                ..agent("b", t0)
            },
            BgAgent {
                background: false,
                ..agent("c", t0)
            },
        ];
        assert_eq!(running_count(&agents), 1);
    }
}
