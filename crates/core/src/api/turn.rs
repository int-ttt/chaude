//! Turn lifecycle on State: begin a turn, finish it, start the next queued one.

use super::*;

impl State {
    // Whether a turn runs (Phase::Busy).
    pub(super) fn busy(&self) -> bool {
        matches!(self.phase, Phase::Busy { .. })
    }

    // Busy on or off. Going busy keeps a running turn's clock; going idle drops it.
    pub(super) fn set_busy(&mut self, on: bool) {
        self.phase = match (on, self.phase) {
            (true, Phase::Idle) => Phase::Busy { started: None },
            (true, busy) => busy,
            (false, _) => Phase::Idle,
        };
    }

    // The running turn's start, None while idle or before the clock starts.
    pub(super) fn turn_started(&self) -> Option<Instant> {
        match self.phase {
            Phase::Busy { started } => started,
            Phase::Idle => None,
        }
    }

    // Starts the turn clock as the send writes the user frame. The send is what makes the phase busy.
    pub(super) fn start_turn_clock(&mut self) {
        self.phase = Phase::Busy {
            started: Some(Instant::now()),
        };
    }

    // The turn ends its clock but stays busy until the next turn or the tui answer.
    pub(super) fn clear_turn_clock(&mut self) {
        if let Phase::Busy { started } = &mut self.phase {
            *started = None;
        }
    }

    // Go viaPrompter: with residents on, the prompt goes through the prompter first (S26: pure acks skip it).
    pub(super) fn begin_turn(&mut self, text: String) {
        if self.cfg.residents && !is_pure_ack(&text) {
            self.start_prompter(text);
        } else {
            self.dispatch(&text);
        }
    }

    // Finishes the running turn and starts the next queued one, if any.
    pub(super) fn end_turn(&mut self, duration_ms: u64) {
        if self.handoff.is_some() {
            self.finish_handoff();
            return;
        }
        // A child-initiated turn ended while the reply is held: its text is dropped, the hold goes on, and a release
        // deferred by this turn goes out now (the child is idle again).
        if self.held.is_some() {
            self.held_turn = false;
            self.release_held();
            return;
        }
        if self.interrupted_at.is_some() {
            self.emit(UiEvent::Interrupted);
        }
        if self.budget_hit {
            self.emit_budget_stop();
        }
        if self.hold_reply(duration_ms) {
            return;
        }
        self.finish_turn(duration_ms);
    }

    // The turn's end: stats, record and TurnEnd, then the tui answer or the next turn.
    pub(super) fn finish_turn(&mut self, duration_ms: u64) {
        if let Some((input, output, cache)) = self.turn_token_counts() {
            self.emit(UiEvent::TurnTokens {
                input,
                output,
                cache,
            });
        }
        self.emit(UiEvent::TurnStats {
            duration_ms,
            cost_turn_usd: (self.cost_usd - self.turn_cost_base).max(0.0),
            cost_usd: self.cost_usd,
            ctx: self.ctx_tokens,
            delegations: self.delegations,
        });
        let sid = (!self.sid.is_empty()).then(|| self.sid.clone());
        self.router.record(
            &self.turn_reply,
            sid.as_deref(),
            self.ctx_tokens,
            Instant::now(),
        );
        self.log_turn(duration_ms, None);
        self.clear_turn_clock();
        self.interrupted_at = None;
        self.open_tools.clear();
        self.stage("main", StagePhase::Done);
        self.emit(UiEvent::TurnEnd { duration_ms });
        self.record_turn_save(duration_ms);
        self.remember_turn();
        if self.cfg.residents && !self.turn_reply.trim().is_empty() {
            // The turn stays busy until the tui answers or fails; after_turn runs then.
            self.start_tui(self.turn_reply.clone());
            return;
        }
        self.after_turn();
    }

    // Rotation and the queued turn, once the reply is final (Go runs these after viaTUI).
    pub(super) fn after_turn(&mut self) {
        // Go repl.go: a category switch starts a new session before the context check, unless cooling down.
        let (cat, switched) = category_rotation(
            &self.turn_prompt,
            &self.turn_reply,
            &self.cat,
            self.cfg.no_category_rotate,
        );
        let prev = std::mem::replace(&mut self.cat, cat.clone());
        let cooled = self
            .last_rot_fail
            .is_none_or(|t| t.elapsed() >= self.cfg.rotate_cooldown);
        if switched
            && cooled
            && self.proc.is_some()
            && self.start_handoff(format!("category {prev} -> {cat}"))
        {
            return;
        }
        if self.rotation_wanted() {
            let reason = self.ctx_reason();
            let timeout = self.cfg.rotate_defer_timeout;
            let hold =
                self.rotate_deferred
                    .hold(self.agents.len(), &reason, Instant::now(), timeout);
            if matches!(hold, Hold::Proceed { .. }) && self.start_handoff(reason) {
                return;
            }
        } else {
            self.clear_deferral();
        }
        self.next_turn();
    }

    // Go doneMsg / mergePrompts: everything queued meanwhile goes out as ONE turn, joined by a blank line.
    pub(super) fn next_turn(&mut self) {
        if self.queue.is_empty() {
            self.set_busy(false);
            self.emit(UiEvent::Busy(false));
            return;
        }
        let next = self.queue.drain(..).collect::<Vec<_>>().join("\n\n");
        self.emit(UiEvent::Queued(0));
        self.emit_queue();
        if !self.send_turn(&next) {
            self.set_busy(false);
            self.emit(UiEvent::Busy(false));
        }
    }

    pub(super) fn remember_turn(&mut self) {
        let rc = extract_reco(&self.turn_reply);
        if !rc.is_empty() {
            self.last_reco = rc;
        }
        self.recent
            .push_back((self.turn_prompt.clone(), self.turn_reply.clone()));
        while self.recent.len() > self.cfg.tail_turns {
            self.recent.pop_front();
        }
        self.turn_seq += 1;
        self.turn_log.push_back((
            self.turn_seq,
            self.turn_prompt.clone(),
            self.turn_reply.clone(),
        ));
        while self.turn_log.len() > TURN_LOG_MAX {
            self.turn_log.pop_front();
        }
    }
}
