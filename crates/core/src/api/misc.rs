//! Rotation, drain, session and child lifecycle on State.

use super::*;

impl State {
    // Go shouldRotate at turn end: the context is at or past the threshold.
    pub(super) fn rotation_wanted(&self) -> bool {
        let thr = threshold_for(&self.model, self.cfg.rotate_threshold);
        self.cfg.rotate_threshold > 0
            && self.proc.is_some()
            && should_rotate(self.ctx_tokens, thr)
            && self.auto_rotate_allowed()
    }

    // Go maxAutoRotations and rotate-cooldown: a negative cap is unlimited, and a held rotation blocks automatic
    // context rotation until the cooldown has passed.
    pub(super) fn auto_rotate_allowed(&self) -> bool {
        let capped = self.cfg.max_auto_rotations >= 0
            && self.rotations >= self.cfg.max_auto_rotations as u64;
        let cooling = self
            .last_rot_fail
            .is_some_and(|t| t.elapsed() < self.cfg.rotate_cooldown);
        !capped && !cooling
    }

    // A held rotation may run now: still wanted, and no background agent runs or the defer timeout passed.
    // A held reply defers it too: the rotation waits for ReplyReleased.
    pub(super) fn deferred_due(&self) -> bool {
        self.held.is_none()
            && self.rotate_deferred.pending()
            && self.rotation_wanted()
            && self.rotate_deferred.is_due(
                self.agents.len(),
                Instant::now(),
                self.cfg.rotate_defer_timeout,
            )
    }

    // Drops a held rotation. hold(0) is due at once, so it ends the hold and its start (Deferral has no clear).
    pub(super) fn clear_deferral(&mut self) {
        let _ = self
            .rotate_deferred
            .hold(0, "", Instant::now(), Duration::ZERO);
    }

    // Go runDeferredRotation: the held handoff runs while the child is idle, so the turn is marked busy.
    pub(super) fn run_deferred_rotation(&mut self) {
        self.clear_deferral();
        self.set_busy(true);
        self.emit(UiEvent::Busy(true));
        if !self.start_handoff(self.ctx_reason()) {
            self.set_busy(false);
            self.emit(UiEvent::Busy(false));
        }
    }

    // Go idle rotation: the child has no API response for idle_deadline, so the handoff runs now.
    pub(super) fn run_idle_rotation(&mut self) {
        self.idle_fired_for = self.last_api;
        self.set_busy(true);
        self.emit(UiEvent::Busy(true));
        let reason = format!("idle {}s", self.idle_deadline().as_secs());
        if !self.start_handoff(reason) {
            self.set_busy(false);
            self.emit(UiEvent::Busy(false));
        }
    }

    // Reason shown for a context rotation: the context tokens against the threshold.
    pub(super) fn ctx_reason(&self) -> String {
        let thr = threshold_for(&self.model, self.cfg.rotate_threshold);
        format!("ctx {} >= {}", self.ctx_tokens, thr)
    }

    // Sends the hidden handoff prompt on the current child. Returns false when it could not be sent, or when a
    // pending snapshot was kept instead (the rotation then runs without a prompt; the caller goes on as after a failure).
    pub(super) fn start_handoff(&mut self, reason: String) -> bool {
        if !self.pending_snapshot.is_empty() {
            self.rotate_keep_snapshot(reason);
            return false;
        }
        self.start_handoff_as(reason, false)
    }

    // The handoff turn itself; the caller has already ruled out a pending snapshot.
    pub(super) fn start_handoff_as(&mut self, reason: String, clear: bool) -> bool {
        self.handoff = Some(Handoff {
            reason,
            attempts: 1,
            clear,
        });
        if self.send_turn(HANDOFF_PROMPT) {
            return true;
        }
        self.handoff = None;
        false
    }

    // Ends the hidden handoff turn (Go makeSnapshotN): retries once when invalid, then rotates or holds.
    pub(super) fn finish_handoff(&mut self) {
        self.clear_turn_clock();
        self.interrupted_at = None;
        self.open_tools.clear();
        let Some(h) = self.handoff.take() else {
            return;
        };
        let note = self.turn_reply.trim().to_string();
        let problems = if self.cfg.no_handoff_validate {
            Vec::new()
        } else {
            validate_handoff(&note, &self.files)
        };
        let valid = problems.is_empty();
        if !valid && h.attempts < 2 {
            self.handoff = Some(Handoff {
                reason: h.reason.clone(),
                attempts: h.attempts + 1,
                clear: h.clear,
            });
            if self.send_turn(HANDOFF_PROMPT) {
                return;
            }
            self.handoff = None;
        }
        if !valid && note.is_empty() && self.recent.is_empty() {
            // Go: nothing to carry, so the rotation is held and the session keeps running.
            self.switch_to = None;
            self.last_rot_fail = Some(Instant::now());
            self.emit(UiEvent::Error(
                "context rotation held: handoff failed".to_string(),
            ));
            self.next_turn();
            return;
        }
        let body = if valid {
            note
        } else {
            format!(
                "(handoff note failed validation: {}; recent turns follow)",
                problems.join("; ")
            )
        };
        let snapshot = with_reco(&self.last_reco, &self.build_snapshot(&body));
        if h.clear {
            self.finish_clear(snapshot);
            self.next_turn();
            return;
        }
        if let Some((model, text)) = self.switch_to.take() {
            self.complete_switch(model, text, snapshot);
            return;
        }
        self.rotate_child(snapshot, h.reason, valid, h.attempts);
        self.next_turn();
    }

    // Closes the current child and starts a fresh one (no --resume). Returns the closed session id.
    pub(super) fn restart_child(&mut self) -> String {
        self.restart_child_with(false).0
    }

    // Returns the old session id and whether the old child was kept draining (Go rotate: drained).
    pub(super) fn restart_child_with(&mut self, drain: bool) -> (String, bool) {
        let old = self.sid.clone();
        let old_epoch = self.epoch;
        let mut drained = false;
        if let Some(mut p) = self.proc.take() {
            if drain {
                // Go drain.consume: prompts still pending on the old child are denied as it drains.
                for id in self.pending_perm.keys() {
                    let _ = p.write_permission(id, false, Value::Null, DRAIN_DENY);
                }
                self.start_drain(p, old_epoch, old.clone());
                drained = true;
            } else {
                p.kill();
                // The killed child took its running agents with it: they settle now (타임아웃), so the notice
                // queues for the new child and the tracker does not keep them forever.
                let ids: Vec<String> = self
                    .agents
                    .keys()
                    .filter(|id| !self.drains.iter().any(|d| d.started.contains(id)))
                    .cloned()
                    .collect();
                for id in ids {
                    self.settle_drained(&id, false);
                }
            }
        }
        self.next_epoch();
        self.rotations += 1;
        self.open_tools.clear();
        self.interrupted_at = None;
        self.stray_results = 0;
        self.pending_perm.clear();
        self.sid.clear();
        self.ctx_tokens = 0;
        self.last_api = None;
        self.opts.resume = None;
        let _ = self.start_child();
        (old, drained)
    }

    // Go drain: the old child keeps its running agents. Its agents are the ones still in self.agents. The oldest
    // drains beyond max_drain are closed.
    pub(super) fn start_drain(&mut self, proc: ClaudeProc, epoch: u64, session: String) {
        let mut started: Vec<String> = self.agents.keys().cloned().collect();
        started.sort();
        self.drains.push(Drain {
            proc,
            epoch,
            session,
            since: Instant::now(),
            started,
            interrupting: false,
        });
        while self.drains.len() > self.cfg.max_drain {
            let oldest = self.drains.remove(0);
            // Evicted before it finished: its agents are closed as timed out, same as a drain that ran out.
            self.close_drain(oldest, true);
        }
    }

    // Ends one drain: its child is killed, the entry dropped, and one "[agent done]" notice queued per agent
    // (Go pollDrains). timed_out marks every agent 타임아웃, as Go does.
    pub(super) fn end_drain(&mut self, epoch: u64, timed_out: bool) {
        if let Some(i) = self.drains.iter().position(|d| d.epoch == epoch) {
            let d = self.drains.remove(i);
            self.close_drain(d, timed_out);
        }
    }

    // The one finish path for a drain that is closed (end or eviction): its child is killed and each agent it
    // started settles once through settle_drained.
    pub(super) fn close_drain(&mut self, mut d: Drain, timed_out: bool) {
        d.proc.kill();
        for id in &d.started {
            let done = !timed_out && !self.agents.contains_key(id);
            self.settle_drained(id, done);
        }
    }

    // Settles one drained or killed agent exactly once: it leaves every drain's started list, its "[agent done]"
    // notice is queued (done or 타임아웃), and it leaves the tracker so its status is terminal.
    pub(super) fn settle_drained(&mut self, id: &str, done: bool) {
        let owners: Vec<u64> = self
            .drains
            .iter()
            .filter(|d| d.started.iter().any(|s| s == id))
            .map(|d| d.epoch)
            .collect();
        for d in self.drains.iter_mut() {
            d.started.retain(|s| s != id);
        }
        let (task, out, brief, report) = self
            .agent_info
            .get(id)
            .map(|i| {
                (
                    i.task.clone(),
                    i.out_path.clone(),
                    self.old_brief(i),
                    i.report.clone(),
                )
            })
            .unwrap_or_default();
        if announce_once(&mut self.announced, id) {
            self.notices
                .push(drain_done_notice(id, &task, &out, done, &brief, &report));
        }
        if self.agents.contains_key(id) {
            self.finish_agent(id.to_string());
        }
        // A drain whose last owned child just completed has nothing left to wait for: the old main is ended now
        // (killed and reaped), so a turn it would start on its own never reaches the TUI.
        if done {
            for epoch in owners {
                if self
                    .drains
                    .iter()
                    .any(|d| d.epoch == epoch && d.started.is_empty())
                {
                    self.end_drain(epoch, false);
                }
            }
        }
        // The drained child consumed the completion: an idle current main gets the notice as its own turn now.
        // A timed-out drain keeps the notice for the next user message (Go).
        if done
            && !self.notices.is_empty()
            && !self.busy()
            && !self.prompter.waiting()
            && self.proc.is_some()
        {
            self.set_busy(true);
            self.emit(UiEvent::Busy(true));
            if !self.send_turn("") {
                self.set_busy(false);
                self.emit(UiEvent::Busy(false));
            }
        }
    }

    // Go drain.consume: a drained child's permission prompts are denied and every line is logged as drain_deny or
    // drain_event (transcript rows, Go r.tr.add). They never finish the shared tracker's agents. Eof only returns,
    // as Go consume does on stdout close; the drain ends in reap_drains (Go pollDrains).
    pub(super) fn on_drain_msg(&mut self, epoch: u64, msg: ProcMsg) {
        let ProcMsg::Line(line) = msg else {
            return;
        };
        let Some(i) = self.drains.iter().position(|d| d.epoch == epoch) else {
            return;
        };
        let kind = match drain_permission_id(&line) {
            Some(id) => {
                let _ = self.drains[i]
                    .proc
                    .write_permission(&id, false, Value::Null, DRAIN_DENY);
                "drain_deny"
            }
            None => "drain_event",
        };
        let raw = serde_json::from_str::<Value>(&line).ok();
        // A turn the drained child starts on its own (a native task notification) is not caused by chaude: it is
        // interrupted once, and its output is discarded up to its result. The agent-done handling still runs below.
        if raw.as_ref().is_some_and(Self::drain_turn_started) && !self.drains[i].interrupting {
            self.drains[i].interrupting = true;
            let _ = self.drains[i].proc.write_interrupt();
        }
        let discard = self.drains[i].interrupting;
        if raw.as_ref().is_some_and(|v| v["type"] == "result") {
            self.drains[i].interrupting = false;
        }
        if !discard {
            let ty = raw
                .as_ref()
                .and_then(|v| v["type"].as_str().map(str::to_string))
                .unwrap_or_default();
            let session = self.drains[i].session.clone();
            self.log_transcript(kind, json!({"type": ty, "session": session}));
            if let Some(v) = &raw {
                self.record_handback(v);
            }
        }
        self.drain_completed(epoch, &line);
    }

    // A turn start on a drained child: an assistant message or message_start of the main thread (no parent tool use).
    // Subagent lines carry parent_tool_use_id and do not count.
    pub(super) fn drain_turn_started(v: &Value) -> bool {
        let top = v["parent_tool_use_id"]
            .as_str()
            .is_none_or(|p| p.is_empty());
        top && (v["type"] == "assistant"
            || (v["type"] == "stream_event" && v["event"]["type"] == "message_start"))
    }

    // A drained agent's <task-notification> ends it at once: the agent leaves the tracker (terminal, AgentEnd) and its
    // "[agent done]" notice is queued now, once. end_drain then skips it. The notice goes out with the next user prompt
    // to the current main, so a rotation in between does not lose it.
    pub(super) fn drain_completed(&mut self, epoch: u64, line: &str) {
        if !self.drains.iter().any(|d| d.epoch == epoch) {
            return;
        }
        match parse_line(line) {
            Ok(Event::System(s)) => self.on_system_task(&s),
            _ => {
                self.on_task_notification(line);
            }
        }
    }

    // Go pollDrains: finished drains are closed (done when the shared tracker is empty, timed out at drain_timeout).
    // Runs at the start of each user submit and on quiet, as Go polls at handle().
    // self.agents is the engine's tracker of running background agents (Go d.tr.running()).
    pub(super) fn reap_drains(&mut self) {
        let ages: Vec<(u64, Duration)> = self
            .drains
            .iter()
            .map(|d| (d.epoch, d.since.elapsed()))
            .collect();
        let finished = finished_drains(&ages, self.agents.len(), self.cfg.drain_timeout);
        for (epoch, timed_out) in finished {
            self.end_drain(epoch, timed_out);
        }
    }

    // Takes the next epoch from the monotonic counter, so a restored parked child keeps a unique tag.
    pub(super) fn next_epoch(&mut self) {
        self.epoch_seq += 1;
        self.epoch = self.epoch_seq;
    }

    // Go rotate with a pending snapshot (snapshot_kept): the child has not received it yet, so a handoff from
    // it would be empty and would replace the snapshot. The child restarts and the snapshot stays for its next prompt.
    pub(super) fn rotate_keep_snapshot(&mut self, reason: String) {
        let old = self.restart_child();
        let _ = self.metrics.log(json!({
            "stage": "rotate",
            "reason": reason,
            "old_session": old,
            "snapshot_kept": true,
            "outcome": "snapshot_kept",
        }));
        self.emit(UiEvent::RotatedFrom { old_session: old });
        self.emit(UiEvent::Rotated { reason });
        self.emit_status(self.cost_usd);
    }

    // Go /rotate (repl.go rotate("manual")): the same handoff as a context rotation, reason "manual".
    pub(super) fn manual_rotate(&mut self) {
        self.set_busy(true);
        self.emit(UiEvent::Busy(true));
        if !self.start_handoff("manual".to_string()) {
            self.set_busy(false);
            self.emit(UiEvent::Busy(false));
        }
    }

    // Go /clear in the TUI (clearWithHandoff): the snapshot is taken first when there is a conversation to carry.
    // A pending snapshot is dropped, as in Go (clear() resets pendingSnapshot before the new one is set).
    pub(super) fn clear_session(&mut self) {
        self.set_busy(true);
        self.emit(UiEvent::Busy(true));
        if self.recent.is_empty() || !self.pending_snapshot.is_empty() {
            self.finish_clear(String::new());
            self.next_turn();
            return;
        }
        if !self.start_handoff_as("clear".to_string(), true) {
            self.finish_clear(String::new());
            self.next_turn();
        }
    }

    // Go clear(): the old session ends, the record splits at this time, and the fresh child gets the snapshot
    // (empty when none) with its next prompt.
    pub(super) fn finish_clear(&mut self, snapshot: String) {
        self.clear_mark();
        self.clear_client_row();
        self.end_all_parked();
        self.record.split = true;
        self.restart_child();
        self.recent.clear();
        self.files.clear();
        self.pending_snapshot = snapshot;
    }

    // Closes the current child and starts a fresh one that gets the snapshot with its next prompt.
    pub(super) fn rotate_child(
        &mut self,
        snapshot: String,
        reason: String,
        valid: bool,
        attempts: u32,
    ) {
        // Built before restart_child_with: a killed child's agents leave self.agents when they settle.
        let carried = self.carried_block();
        let drain = drain_wanted(self.cfg.max_drain, self.agents.len());
        let (old, drained) = self.restart_child_with(drain);
        self.pending_snapshot = if carried.is_empty() {
            snapshot
        } else {
            format!("{snapshot}\n\n{carried}")
        };
        let _ = self.metrics.log(json!({
            "stage": "rotate",
            "reason": reason,
            "old_session": old,
            "handoff_valid": valid,
            "attempts": attempts,
            "drained": drained,
            "outcome": if drained { "drained" } else { "closed" },
        }));
        self.emit(UiEvent::RotatedFrom { old_session: old });
        self.emit(UiEvent::Rotated { reason });
        self.emit_status(self.cost_usd);
    }

    // Relaunches the main child on an existing session with --resume (EngineHandle::resume_session).
    // The caller names the session, so resume_mode does not apply here. Idle only, like a fresh start.
    // Go applyImport: the imported turns, files and recommendation become this session's history, and the
    // snapshot built from them goes in front of the next prompt (send_turn). Turns beyond tail_turns are dropped.
    pub(super) fn install_import(&mut self, imp: ClaudeImport) {
        if self.busy() || self.prompter.waiting() {
            self.emit(UiEvent::Error(
                "cannot import while a turn is running".to_string(),
            ));
            return;
        }
        let stem = imp
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let skip = imp.turns.len().saturating_sub(self.cfg.tail_turns);
        self.recent = imp.turns.into_iter().skip(skip).collect();
        self.files = imp.files;
        self.last_reco = imp.reco;
        self.restore_agents_for(&stem);
        let mut note = import_note(&stem);
        if let Some(p) = &imp.note_path {
            note = format!("{note}\n\n{}", resume_note::hint(p));
        }
        self.pending_snapshot = with_reco(&self.last_reco, &self.build_snapshot(&note));
    }

    pub(super) fn resume_session(&mut self, session_id: &str) {
        if session_id.is_empty() {
            self.emit(UiEvent::Error("resume needs a session id".to_string()));
            return;
        }
        if self.busy() || self.prompter.waiting() {
            self.emit(UiEvent::Error(
                "cannot resume while a turn is running".to_string(),
            ));
            return;
        }
        let old = self.sid.clone();
        if let Some(mut p) = self.proc.take() {
            p.kill();
        }
        self.next_epoch();
        self.open_tools.clear();
        self.interrupted_at = None;
        self.stray_results = 0;
        self.pending_perm.clear();
        self.queued_tasks.clear();
        self.sid.clear();
        self.ctx_tokens = 0;
        self.last_api = None;
        self.opts.resume = Some(session_id.to_string());
        self.load_tasks(session_id);
        let _ = self.start_child();
        self.emit(UiEvent::SessionResumed {
            old_session: old,
            session_id: session_id.to_string(),
        });
        self.emit_status(self.cost_usd);
    }

    // Spawns the child for the current generation; its lines are tagged so a replaced child is ignored.
    // Go residentMCPConfig: started once, only with residents on. None leaves the main child without the coder tool.
    pub(super) fn mcp_config(&mut self) -> Option<PathBuf> {
        if !self.cfg.residents {
            return None;
        }
        if self.mcp_cfg.is_none() {
            let caller = CoderCaller {
                tx: self.self_tx.clone(),
            };
            let cwd = self.engine_cwd();
            self.mcp_cfg = mcp::start_owned(
                &cwd,
                Arc::new(move |task: &str| caller.call(task).map_err(crate::error::Error::Msg)),
            )
            .ok();
        }
        self.mcp_cfg.as_ref().map(|c| c.path().to_path_buf())
    }

    // Returns the spawn error text; the UiEvent::Error is sent either way when the spawn fails.
    pub(super) fn start_child(&mut self) -> Result<(), String> {
        self.seen_task_started = false;
        let mut opts = self.opts.clone();
        if let Some(path) = self.mcp_config() {
            opts.shape = Shape {
                strict_mcp: true,
                mcp_config: Some(path),
                rule: join_rules(&[&main_rule(&self.cfg), CODER_RULE]),
                ..Shape::default()
            };
        }
        let epoch = self.epoch;
        match proc::spawn(&opts, self.self_tx.clone(), move |m| Input::Proc(epoch, m)) {
            Ok(p) => {
                self.upsert_live(p.pid(), opts.resume.clone().unwrap_or_default());
                self.proc = Some(p);
                Ok(())
            }
            Err(e) => {
                self.emit(UiEvent::Error(format!("claude spawn failed: {e}")));
                Err(e.to_string())
            }
        }
    }

    // Go switchchild.go:263: the live row for this harness's main child, owned by this process.
    pub(super) fn upsert_live(&self, child_pid: u32, session_id: String) {
        let Some(reg) = &self.clients else {
            return;
        };
        reg.upsert(ClientRec {
            cwd: self.client_cwd(),
            model: self.model.clone(),
            session_id,
            state: LIVE.to_string(),
            last_api: now_stamp(),
            owner_pid: std::process::id(),
            child_pid,
            ..Default::default()
        });
    }

    // Go repl.go clear(): the old row ends with "clear", then the new session gets a fresh live row.
    pub(super) fn clear_client_row(&self) {
        let Some(reg) = &self.clients else {
            return;
        };
        reg.mark_ended(&self.client_cwd(), &self.model, "clear", now_stamp());
        let child_pid = self.proc.as_ref().map_or(0, |p| p.pid());
        self.upsert_live(child_pid, String::new());
    }

    // Go r.a.Cwd as text: the absolute engine cwd (current dir when the engine has no cwd option).
    pub(super) fn client_cwd(&self) -> String {
        self.engine_cwd().to_string_lossy().into_owned()
    }
}
