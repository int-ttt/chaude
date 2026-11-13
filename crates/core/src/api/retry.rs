//! Retry and timeout handling on State: coder, tui and prompter timeouts, wait budgets, idle exit timers.

use super::*;
use crate::error::Error;

impl State {
    // The prompter gave up waiting. Its child is replaced, so late lines are dropped by epoch.
    pub(super) fn prompter_timeout(&mut self) {
        if let Some(mut p) = self.prompter_proc.take() {
            p.kill();
        }
        self.prompter_epoch += 1;
        if let Some(o) = self.prompter.timeout(self.cfg.prompter_timeout) {
            self.finish_prompter(o);
        }
    }

    // Time left before an idle extra coder expires (Go extraCoderTTL). None for the base coder, a busy
    // slot, or an empty slot.
    pub(super) fn coder_idle_left(&self, i: usize) -> Option<Duration> {
        let c = self.coders.get(i)?;
        if i == 0 || c.deadline.is_some() || c.proc.is_none() {
            return None;
        }
        let since = c.idle_since?;
        Some(self.cfg.coder_idle_ttl.saturating_sub(since.elapsed()))
    }

    // Time left before an idle base coder exits on its own (base_idle_exit_secs, 3570 s). Extra coders are
    // handled by coder_idle_left. None when disabled, busy or empty.
    pub(super) fn base_idle_exit_left(&self, i: usize) -> Option<Duration> {
        let c = self.coders.get(i)?;
        if i != 0 {
            return None; // extra coders expire through coder_idle_left
        }
        let secs = self.cfg.base_idle_exit_secs;
        if secs == 0 || c.deadline.is_some() || c.proc.is_none() {
            return None;
        }
        let since = c.idle_since?;
        Some(Duration::from_secs(secs).saturating_sub(since.elapsed()))
    }

    // S21: schedules another send of the held task after its backoff. False when the retry budget is spent,
    // so the caller reports the failure now.
    pub(super) fn coder_retry(&mut self, i: usize) -> bool {
        if self.coders[i].task.is_none() || self.coders[i].retries >= RETRY_MAX {
            return false;
        }
        let now = Instant::now();
        let started = *self.coders[i].retry_started.get_or_insert(now);
        let delay = RETRY_BACKOFF[self.coders[i].retries as usize];
        if now + delay > started + RETRY_TOTAL {
            return false;
        }
        self.coders[i].retries += 1;
        self.coders[i].retry_at = Some(now + delay);
        self.stage(&coder_role(i), StagePhase::Working);
        true
    }

    // S21: sends the held task again. A dead child is respawned with --resume on its session.
    pub(super) fn coder_resend(&mut self, i: usize) {
        self.coders[i].retry_at = None;
        let Some(text) = self.coders[i].task.clone() else {
            return;
        };
        if self.coders[i].proc.is_none() {
            let role = coder_role(i);
            if let Some(sid) = self.resident_sids.get(&role).cloned() {
                self.resume_pending.insert(role, sid);
            }
            if let Err(e) = self.spawn_coder(i) {
                self.finish_coder(i, Err(Error::Msg(format!("coder spawn failed: {e}"))));
                return;
            }
        }
        let wrote = match self.coders[i].proc.as_mut() {
            Some(p) => p.write_user(&text).map_err(Error::from),
            None => Err(Error::Msg("coder not running".to_string())),
        };
        if let Err(e) = wrote {
            self.drop_coder(i);
            self.finish_coder(i, Err(Error::Msg(format!("coder write failed: {e}"))));
        }
    }

    // The coder gave up waiting (Go residentCoderTimeout). The child is replaced.
    pub(super) fn coder_timeout(&mut self, i: usize) {
        self.drop_coder(i);
        self.finish_coder(i, Err(Error::Msg("coder timed out".to_string())));
    }

    // The tui gave up waiting. Its child is replaced, so late lines are dropped by epoch.
    pub(super) fn tui_timeout(&mut self) {
        if let Some(mut p) = self.tui_proc.take() {
            p.kill();
        }
        self.tui_epoch += 1;
        self.finish_tui(Err(Error::Msg(tf(
            Msg::TimedOut,
            &[&self.cfg.tui_timeout.as_secs_f64()],
        ))));
    }

    // Time left in the running tui wait. None when no wait is running.
    pub(super) fn tui_left(&self) -> Option<Duration> {
        self.tui_deadline
            .map(|t| t.saturating_duration_since(Instant::now()))
    }

    // Time left in the running prompter wait. None when no wait is running.
    pub(super) fn prompter_left(&self) -> Option<Duration> {
        self.prompter_deadline
            .map(|t| t.saturating_duration_since(Instant::now()))
    }

    // Pending ends past agent_end_grace: the agent ends. A pending agent with a live owned task is not expired: its
    // owned tasks stay tracked, and the last owned completion ends it (on_system_task).
    pub(super) fn expire_pending_ends(&mut self) {
        let grace = self.cfg.agent_end_grace;
        let due: Vec<String> = self
            .pending_end
            .iter()
            .filter(|(id, t0)| t0.elapsed() >= grace && !self.owned_live(id))
            .map(|(id, _)| id.clone())
            .collect();
        for id in due {
            self.complete_agent(id);
        }
    }

    // Time until the nearest pending end expires. None when none is pending. Pending agents with a live owned task
    // are left out: they do not expire, so their remaining time would be zero and spin the wait.
    pub(super) fn pending_end_left(&self) -> Option<Duration> {
        self.pending_end
            .iter()
            .filter(|(id, _)| !self.owned_live(id))
            .map(|(_, t0)| self.cfg.agent_end_grace.saturating_sub(t0.elapsed()))
            .min()
    }

    // How long the actor may block before the inactivity timer must be checked. None means block until input.
    // A coder wait can run during a busy main turn, so its deadline is only added to the main timers.
    pub(super) fn wait_budget(&self) -> Option<Duration> {
        // A busy coder wakes at its deadline; an idle extra coder wakes at its expiry.
        let coders = self
            .coders
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c.deadline {
                Some(t) => {
                    let left = t.saturating_duration_since(Instant::now());
                    Some(match c.retry_at {
                        Some(r) => left.min(r.saturating_duration_since(Instant::now())),
                        None => left,
                    })
                }
                None => [self.coder_idle_left(i), self.base_idle_exit_left(i)]
                    .into_iter()
                    .flatten()
                    .min(),
            })
            .min();
        // A draining child wakes at its drain timeout.
        let drains = self
            .drains
            .iter()
            .map(|d| self.cfg.drain_timeout.saturating_sub(d.since.elapsed()))
            .min();
        // A pending agent end wakes at its grace.
        let own = [coders, drains, self.pending_end_left(), self.task_left()]
            .into_iter()
            .flatten()
            .min()
            .map(|d| d.max(Duration::from_millis(1)));
        match (self.main_wait_budget(), own) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    // Idle, or any tool_use without its result, pauses the timer.
    pub(super) fn main_wait_budget(&self) -> Option<Duration> {
        // A prompter wait only starts while idle, so it is the only timer that matters until it ends.
        if let Some(left) = self.prompter_left() {
            return Some(left.max(Duration::from_millis(1)));
        }
        // A tui wait runs on a busy turn with no open tool: its own deadline is the only timer.
        if let Some(left) = self.tui_left() {
            return Some(left.max(Duration::from_millis(1)));
        }
        if !self.busy() {
            // Idle with a held rotation: poll it, or wake for the idle rotation, whichever is nearer.
            let poll =
                (self.rotate_deferred.pending() && self.rotation_wanted()).then_some(DEFER_POLL);
            let left = [poll, self.idle_left()].into_iter().flatten().min();
            return left.map(|left| left.max(Duration::from_millis(1)));
        }
        if let Some(t0) = self.interrupted_at {
            let left = self.cfg.interrupt_grace.saturating_sub(t0.elapsed());
            return Some(left.max(Duration::from_millis(1)));
        }
        if !self.open_tools.is_empty() {
            return None;
        }
        let left = self
            .cfg
            .inactivity_timeout
            .saturating_sub(self.last_activity.elapsed());
        Some(left.max(Duration::from_millis(1)))
    }

    // Inactivity notice. Informational only: the turn is never interrupted, and the clock restarts so the
    // notice repeats once per quiet period rather than on every wakeup.
    pub(super) fn on_quiet(&mut self) {
        self.reap_drains();
        self.expire_pending_ends();
        self.expire_tasks();
        if self.prompter_left().is_some_and(|d| d.is_zero()) {
            self.prompter_timeout();
        }
        if self.tui_left().is_some_and(|d| d.is_zero()) {
            self.tui_timeout();
        }
        for i in 0..self.coders.len() {
            if self.coders[i].deadline.is_some_and(|t| t <= Instant::now()) {
                self.coder_timeout(i);
            }
            if self.coders[i].retry_at.is_some_and(|t| t <= Instant::now()) {
                self.coder_resend(i);
            }
            if self.coder_idle_left(i).is_some_and(|d| d.is_zero()) {
                self.drop_coder(i);
            }
            if self.base_idle_exit_left(i).is_some_and(|d| d.is_zero()) {
                self.exit_idle_coder(i);
            }
        }
        // A running tui wait is not an inactivity case: the main child is idle.
        if self.tui_deadline.is_some() {
            return;
        }
        if !self.busy() {
            // A deferred rotation waits out a prompter wait: it would send its handoff while the prompter runs.
            if !self.prompter.waiting() && self.deferred_due() {
                self.run_deferred_rotation();
            } else if self.idle_left().is_some_and(|d| d.is_zero()) {
                self.run_idle_rotation();
            }
            return;
        }
        if let Some(t0) = self.interrupted_at {
            if t0.elapsed() >= self.cfg.interrupt_grace {
                self.abandon_turn();
            }
            return;
        }
        if !self.open_tools.is_empty() {
            return;
        }
        let quiet = self.last_activity.elapsed();
        if quiet < self.cfg.inactivity_timeout {
            return;
        }
        self.last_activity = Instant::now();
        self.emit(UiEvent::Error(format!(
            "still running: no output for {:.1}s (not interrupted)",
            quiet.as_secs_f64()
        )));
    }

    // Interrupted turn gave no result within the grace period (Go: finishForeground). Foreground agents still
    // open are ended here; background agents keep running. Then the turn is ended as if it had finished.
    pub(super) fn abandon_turn(&mut self) {
        let ids: Vec<String> = self
            .agents
            .keys()
            .filter(|id| !self.agent_info.get(*id).is_some_and(|i| i.background))
            .cloned()
            .collect();
        for id in ids {
            self.opus_set(&id, false);
            // The agent is gone: its pending end and owned tasks must not outlive it.
            self.pending_end.remove(&id);
            self.owned_tasks.retain(|_, parent| *parent != id);
            self.agent_task_ids.retain(|_, tool| *tool != id);
            if let Some(t0) = self.agents.remove(&id) {
                let (model, name, summary) = self
                    .agent_info
                    .get(&id)
                    .map(|i| (i.model.clone(), i.session_name.clone(), i.done_what()))
                    .unwrap_or_default();
                self.persist_agent(&id, STO_DONE);
                self.emit(UiEvent::AgentEnd {
                    id,
                    elapsed_ms: t0.elapsed().as_millis() as u64,
                    model,
                    name,
                    summary,
                });
            }
        }
        self.emit_agent_rows();
        let duration_ms = elapsed_ms(self.turn_started());
        // The abandoned turn's prompts can no longer be answered by the child; drop them.
        self.pending_perm.clear();
        self.stray_results += 1;
        self.end_turn(duration_ms);
    }

    // Per-task wait: a queued task still running past agent_wait_timeout settles as Timeout.
    pub(super) fn expire_tasks(&mut self) {
        let limit = self.cfg.agent_wait_timeout;
        let late: Vec<String> = self
            .queued_tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running && t.since.elapsed() >= limit)
            .map(|t| t.id.clone())
            .collect();
        for id in late {
            self.settle_task(&id, TaskStatus::Timeout);
        }
    }

    // Time until the nearest queued task times out (wakes on_quiet).
    pub(super) fn task_left(&self) -> Option<Duration> {
        let limit = self.cfg.agent_wait_timeout;
        self.queued_tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .map(|t| limit.saturating_sub(t.since.elapsed()))
            .min()
    }

    // Go idleDeadline: the main session's idle rotation waits idle_rotate (3570 s by default) whatever TTL the
    // cache showed. The 290 s cache window is for warm checks only (see pick_child). Zero = off.
    pub(super) fn idle_deadline(&self) -> Duration {
        main_idle_rotation(&self.cfg)
    }

    // Time left before the idle rotation fires. None when it cannot fire now: off, no child, no API response
    // yet, a prompter wait, or already fired for this response (so the wait does not spin at zero).
    pub(super) fn idle_left(&self) -> Option<Duration> {
        let deadline = self.idle_deadline();
        let t0 = self.last_api?;
        if deadline.is_zero()
            || self.proc.is_none()
            || self.prompter.waiting()
            || self.idle_fired_for == Some(t0)
        {
            return None;
        }
        Some(deadline.saturating_sub(t0.elapsed()))
    }
}
