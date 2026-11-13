//! Routing on State: user text to the main child, haiku prompter, tui and coder workers; switch and resident handling.

use super::*;

impl State {
    pub(super) fn submit(&mut self, text: String) {
        // Go handle (repl.go:1080): pollDrains runs first, so finished drains queue their notices for this turn.
        self.reap_drains();
        // Go handle (repl.go:1081): the next user line hides the restored agents.
        if !self.restored_rows.is_empty() {
            self.restored_rows.clear();
            self.emit(UiEvent::AgentsRestored { rows: Vec::new() });
        }
        if self.proc.is_none() {
            self.emit(UiEvent::Error("claude is not running".to_string()));
            return;
        }
        // Go /clear and /rotate are session commands, not turns: they run here and never reach the child as text.
        let cmd = text.trim();
        if cmd == "/clear" || cmd == "/rotate" {
            if self.busy() || self.prompter.waiting() {
                self.emit(UiEvent::Error(format!("{cmd} needs an idle session")));
                return;
            }
            if cmd == "/clear" {
                self.clear_session();
            } else {
                self.manual_rotate();
            }
            return;
        }
        // Go /model NAME (tui.go runTUI): a session command too, handled as a rotation to the new model.
        if let Some(model) = cmd.strip_prefix("/model ").map(str::trim)
            && !model.is_empty()
        {
            if self.busy() || self.prompter.waiting() {
                self.emit(UiEvent::Error("/model needs an idle session".to_string()));
                return;
            }
            self.model_switch(model.to_string());
            return;
        }
        // /lang CODE (TUI and plain /lang, after the setting is saved and the process language set): a handoff so the
        // fresh main child starts with the new --settings language. Busy: the saved language applies at the next rotation.
        if let Some(code) = cmd.strip_prefix("/lang ").map(str::trim)
            && !code.is_empty()
        {
            if self.busy() || self.prompter.waiting() {
                self.emit(UiEvent::Error(
                    "language saved; the running session switches on the next /rotate".to_string(),
                ));
                return;
            }
            self.set_busy(true);
            self.emit(UiEvent::Busy(true));
            if !self.start_handoff(format!("lang {code}")) {
                self.set_busy(false);
                self.emit(UiEvent::Busy(false));
            }
            return;
        }
        if self.busy() || self.prompter.waiting() {
            self.queue.push_back(text);
            self.emit(UiEvent::Queued(self.queue.len()));
            self.emit_queue();
            return;
        }
        self.begin_turn(text);
    }

    // Sends text to the main child as a turn; the engine is busy only when it went out.
    pub(super) fn dispatch(&mut self, text: &str) {
        // Go routeOrch: mode f sends a workspace-tagged line to its haiku session and an untagged line to the main model.
        let routed = self.router.route(text, &self.cfg.model, Instant::now());
        match routed.target {
            orch::Target::Current => {}
            orch::Target::Main => {
                if let Some(m) = self.router.main_model().map(str::to_string)
                    && m != self.opts.model
                {
                    if self.start_switch_handoff(&m, &routed.text) {
                        return;
                    }
                    self.orch_child(m, None);
                }
            }
            orch::Target::Resume { session_id, .. } => {
                self.orch_child("haiku".to_string(), Some(session_id));
            }
            orch::Target::Fresh { .. } => self.orch_child("haiku".to_string(), None),
        }
        self.cur_task = routed.text.clone();
        if self.send_turn(&routed.text) {
            self.set_busy(true);
            self.emit(UiEvent::Busy(true));
        }
    }

    // Go switchChild: the active main child is parked, then the child for the new model is reused live,
    // resumed from its registry row, or started new with the handoff snapshot in front of its first prompt.
    // resume keeps a live workspace session, None starts a fresh one.
    pub(super) fn orch_child(&mut self, model: String, resume: Option<String>) {
        let switched = model != self.model;
        if switched {
            self.park_active();
        }
        self.model = model.clone();
        self.opts.model = model.clone();
        // The previous pick_child row belongs to the child that is now parked or replaced.
        if self.picked_row.take().is_some() {
            self.emit_agent_rows();
        }
        if let Some(sid) = resume {
            self.resume_session(&sid);
            return;
        }
        let cwd = self.client_cwd();
        let key = child_key(&cwd, &model, "");
        let parked = match self.pick_for(&model) {
            ChildAction::UseLive => self.children.take(&key),
            ChildAction::Resume => {
                if let Some(r) = self.child_row(&cwd, &model) {
                    self.resume_session(&r.session_id);
                    self.picked_row = Some(picked_agent_row(&r.session_id, &model, now_secs()));
                    self.emit_agent_rows();
                }
                return;
            }
            ChildAction::StartNew => None,
        };
        match parked {
            Some(p) => self.restore_child(p),
            // A pending snapshot (import or kept rotation) stays for the new child (Go switchChild).
            None => {
                self.restart_child();
            }
        }
    }

    // Registry row for (cwd, model), as pickChild reads it.
    pub(super) fn child_row(&self, cwd: &str, model: &str) -> Option<RegRow> {
        self.clients
            .as_ref()
            .and_then(|reg| reg.get(cwd, model))
            .map(|r| RegRow {
                parked: r.state == PARKED,
                session_id: r.session_id,
                owner_pid: r.owner_pid,
                last_api: stamp_time(r.last_api),
                last_ctx: u64::try_from(r.last_ctx).unwrap_or(0),
            })
    }

    // Go pickChild for the (cwd, model) child: reuse live, resume the registry row, or start new.
    pub(super) fn pick_for(&self, model: &str) -> ChildAction {
        let cwd = self.client_cwd();
        let live = self.children.warm_info(&child_key(&cwd, model, ""));
        let row = self.child_row(&cwd, model);
        // Go pickChild compares against thresholdFor(target): Opus gets its own 200k limit.
        let limit = threshold_for(model, self.cfg.rotate_threshold);
        pick_child::pick_child(
            live,
            row.as_ref(),
            limit,
            SystemTime::now(),
            pick_warm_window(&self.cfg),
        )
    }

    // Go switchChild snapshot: a switch to a fresh child first runs the handoff turn on the running child. The
    // prompt is held in switch_to and sent to the new child when the handoff ends (finish_handoff).
    pub(super) fn start_switch_handoff(&mut self, model: &str, text: &str) -> bool {
        let pick = self.pick_for(model);
        let due = switch_needs_handoff(
            &self.pending_snapshot,
            self.recent.len(),
            self.proc.is_some(),
            pick,
            self.handoff.is_some(),
        );
        if !due {
            return false;
        }
        self.switch_to = Some((model.to_string(), text.to_string()));
        self.set_busy(true);
        self.emit(UiEvent::Busy(true));
        if self.start_handoff(format!("route {model}")) {
            return true;
        }
        self.switch_to = None;
        self.set_busy(false);
        self.emit(UiEvent::Busy(false));
        false
    }

    // Go switchChild after the snapshot: the old child is parked under its own model, the new child for the
    // target starts with the snapshot in front of the held prompt.
    pub(super) fn complete_switch(&mut self, model: String, text: String, snapshot: String) {
        self.park_active();
        self.model = model.clone();
        self.opts.model = model;
        self.restart_child();
        self.pending_snapshot = snapshot;
        self.cur_task = text.clone();
        if !self.send_turn(&text) {
            self.set_busy(false);
            self.emit(UiEvent::Busy(false));
        }
    }

    // Go parkOld: the active main child stays alive under its (cwd, model) key and its registry row is parked.
    pub(super) fn park_active(&mut self) {
        let Some(proc) = self.proc.take() else {
            return;
        };
        let now = SystemTime::now();
        let last_api = self
            .last_api
            .and_then(|t| now.checked_sub(t.elapsed()))
            .unwrap_or(UNIX_EPOCH);
        let cwd = self.client_cwd();
        let key = child_key(&cwd, &self.model, "");
        let sid = self.sid.clone();
        let ctx = self.ctx_tokens;
        if let Some(reg) = &self.clients {
            reg.upsert(ClientRec {
                cwd: cwd.clone(),
                model: self.model.clone(),
                session_id: sid.clone(),
                state: PARKED.to_string(),
                last_api: now_stamp(),
                last_ctx: i64::try_from(ctx).unwrap_or(i64::MAX),
                owner_pid: std::process::id(),
                child_pid: proc.pid(),
                ..Default::default()
            });
        }
        let epoch = self.epoch;
        self.children.park(
            &key,
            Parked {
                sess: ParkedProc { proc, epoch, sid },
                model: self.model.clone(),
                last_api,
                last_ctx: ctx,
                busy: !self.agents.is_empty(),
            },
        );
    }

    // Go endAllParked("clear"): every parked child is ended and its registry row marked, so a /clear cannot bring
    // them back (their pre-clear session ids included).
    pub(super) fn end_all_parked(&mut self) {
        let cwd = self.client_cwd();
        for (_, mut p) in self.children.take_all() {
            p.sess.close();
            if let Some(reg) = &self.clients {
                reg.mark_ended(&cwd, &p.model, "clear", now_stamp());
            }
        }
    }

    // Go switchChild with actUseLive: the parked child becomes the active one again under its own epoch.
    pub(super) fn restore_child(&mut self, p: Parked<ParkedProc>) {
        let ParkedProc { proc, epoch, sid } = p.sess;
        let pid = proc.pid();
        self.proc = Some(proc);
        self.epoch = epoch;
        self.sid = sid.clone();
        self.ctx_tokens = p.last_ctx;
        self.last_api = SystemTime::now()
            .duration_since(p.last_api)
            .ok()
            .and_then(|age| Instant::now().checked_sub(age));
        self.open_tools.clear();
        self.interrupted_at = None;
        self.stray_results = 0;
        self.pending_perm.clear();
        self.opts.resume = None;
        self.upsert_live(pid, sid);
        // The restored child's id is now the main id (/status and /view read it from SessionIds).
        self.emit_session_ids();
        self.emit_status(self.cost_usd);
    }

    // Adds or removes an opus Agent and emits OpusBusy only when the running state changes.
    pub(super) fn opus_set(&mut self, id: &str, on: bool) {
        let before = !self.opus_agents.is_empty();
        if on {
            self.opus_agents.insert(id.to_string());
        } else {
            self.opus_agents.remove(id);
        }
        let after = !self.opus_agents.is_empty();
        if before != after {
            self.emit(UiEvent::OpusBusy(after));
        }
    }

    pub(super) fn resident_busy(&mut self, role: &str, busy: bool, prompt: &str) {
        let (prompt, started_at) = if busy {
            (
                prompt.lines().next().unwrap_or_default().to_string(),
                now_secs(),
            )
        } else {
            (String::new(), 0)
        };
        let model = if busy {
            self.resident_model(role)
        } else {
            String::new()
        };
        self.emit(UiEvent::ResidentBusy {
            role: role.to_string(),
            busy,
            model,
            prompt,
            started_at,
        });
    }

    // Sends text to the prompter. A spawn, start or write failure falls back to the raw prompt (Go).
    pub(super) fn start_prompter(&mut self, text: String) {
        if self.prompter_proc.is_none()
            && let Err(e) = self.spawn_prompter()
        {
            self.notice(prompter_notice(&tf(Msg::Error, &[&e])));
            self.dispatch(&text);
            return;
        }
        let phase = match self.prompter.start(&text) {
            Ok(phase) => phase,
            Err(e) => {
                self.notice(prompter_notice(&tf(Msg::Error, &[&e])));
                self.dispatch(&text);
                return;
            }
        };
        self.stage("prompter", phase);
        self.prompter_deadline = Some(Instant::now() + self.cfg.prompter_timeout);
        self.resident_busy("prompter", true, &text);
        let wrote = match self.prompter_proc.as_mut() {
            Some(p) => p.write_user(&text).map_err(|e| e.to_string()),
            None => Err("prompter not running".to_string()),
        };
        if let Err(e) = wrote {
            self.prompter_proc = None;
            if let Some(o) = self.prompter.fail(&format!("prompter write failed: {e}")) {
                self.finish_prompter(o);
            }
        }
    }

    // Go residentSlots prompter: haiku with the minimal shape and the prompter rule. Kept alive across prompts.
    pub(super) fn spawn_prompter(&mut self) -> Result<(), String> {
        self.resident_sids.remove("prompter");
        let opts = SpawnOpts {
            model: self.resident_model("prompter"),
            resume: self.resume_pending.remove("prompter"),
            max_budget_usd: 0.0,
            shape: Shape {
                minimal: true,
                rule: PROMPTER_RULE.to_string(),
                ..Shape::default()
            },
            ..self.opts.clone()
        };
        self.prompter_epoch += 1;
        let epoch = self.prompter_epoch;
        let p = proc::spawn(&opts, self.self_tx.clone(), move |m| {
            Input::Prompter(epoch, m)
        })
        .map_err(|e| e.to_string())?;
        self.prompter_proc = Some(p);
        Ok(())
    }

    // The prompter's result line is its reply.
    pub(super) fn on_prompter_line(&mut self, line: &str) {
        if let Ok(Event::ControlRequest(c)) = parse_line(line)
            && c.request.subtype == "can_use_tool"
        {
            deny_parked(self.prompter_proc.as_mut(), &c.request_id);
            return;
        }
        let Ok(Event::Result(r)) = parse_line(line) else {
            return;
        };
        self.note_resident("prompter", r.session_id.clone());
        self.role_context("prompter", r.usage.as_ref());
        if let Some(o) = self.prompter.reply(&r.result.unwrap_or_default()) {
            self.finish_prompter(o);
        }
    }

    pub(super) fn on_prompter_eof(&mut self) {
        self.prompter_proc = None;
        if let Some(o) = self.prompter.fail("exited") {
            self.finish_prompter(o);
        }
    }

    // Ends the prompter wait: its stage row, then the outcome goes to the main child as a turn.
    pub(super) fn finish_prompter(&mut self, o: PrompterOutcome) {
        if self.prompter_deadline.take().is_some() {
            self.resident_busy("prompter", false, "");
        }
        match o.stage {
            Some(StagePhase::Failed(text)) => self.notice(text),
            Some(phase) => self.stage("prompter", phase),
            None => {}
        }
        self.dispatch(&o.text);
    }

    // Go viaTUI: the main reply goes to the tui child. Failures keep the main reply (the stage row says why).
    pub(super) fn start_tui(&mut self, reply: String) {
        if self.tui_proc.is_none()
            && let Err(e) = self.spawn_tui()
        {
            self.finish_tui(Err(tf(Msg::Error, &[&e])));
            return;
        }
        self.stage("tui", StagePhase::Working);
        self.tui_deadline = Some(Instant::now() + self.cfg.tui_timeout);
        self.resident_busy("tui", true, &reply);
        let wrote = match self.tui_proc.as_mut() {
            Some(p) => p
                .write_user(&tui_prompt(&reply, crate::i18n::current()))
                .map_err(|e| e.to_string()),
            None => Err("tui not running".to_string()),
        };
        if let Err(e) = wrote {
            self.tui_proc = None;
            self.finish_tui(Err(tf(Msg::Error, &[&format!("write failed: {e}")])));
        }
    }

    // Go residentSlots tui: haiku with the minimal shape and the haiku-output agent body. Kept alive across turns.
    pub(super) fn spawn_tui(&mut self) -> Result<(), String> {
        self.resident_sids.remove("tui");
        let cwd = self.opts.cwd.clone().unwrap_or_else(|| ".".into());
        let rule = self.agent_rule(&cwd, "haiku-output");
        let opts = SpawnOpts {
            model: self.resident_model("tui"),
            resume: self.resume_pending.remove("tui"),
            max_budget_usd: 0.0,
            shape: Shape {
                minimal: true,
                rule,
                ..Shape::default()
            },
            ..self.opts.clone()
        };
        self.tui_epoch += 1;
        let epoch = self.tui_epoch;
        let p = proc::spawn(&opts, self.self_tx.clone(), move |m| Input::Tui(epoch, m))
            .map_err(|e| e.to_string())?;
        self.tui_proc = Some(p);
        Ok(())
    }

    // Go runResidentCoder: the first idle coder takes the task (spawned if not running). Busy coders are skipped.
    // waiter (MCP calls only) gets the reply or the failure reason.
    pub(super) fn start_coder(
        &mut self,
        task: String,
        waiter: Option<Sender<Result<String, String>>>,
    ) {
        let task = task.trim().to_string();
        if task.is_empty() {
            self.emit(UiEvent::Error("task is required".to_string()));
            answer(waiter, Err("task is required".to_string()));
            return;
        }
        let extra = self.cfg.extra_coders.min(MAX_EXTRA_CODERS);
        let idle = (0..=extra).find(|&i| self.coders.get(i).is_none_or(|c| c.deadline.is_none()));
        let Some(i) = idle else {
            self.stage(
                "coder",
                StagePhase::Failed("all resident coders are busy".into()),
            );
            answer(waiter, Err("all resident coders are busy".to_string()));
            return;
        };
        let role = coder_role(i);
        if self.coders.len() <= i {
            self.coders.resize_with(i + 1, Coder::default);
        }
        if self.coders[i].proc.is_none()
            && let Err(e) = self.spawn_coder(i)
        {
            self.stage(
                &role,
                StagePhase::Failed(format!("coder spawn failed: {e}")),
            );
            answer(waiter, Err(format!("coder spawn failed: {e}")));
            return;
        }
        self.stage(&role, StagePhase::Working);
        self.coders[i].deadline = Some(Instant::now() + CODER_TIMEOUT);
        self.resident_busy(&role, true, &task);
        self.coders[i].idle_since = None;
        self.coders[i].waiter = waiter;
        let text = format!("{task}{CODER_REPLY_FORMAT}");
        self.coders[i].task = Some(text.clone());
        let wrote = match self.coders[i].proc.as_mut() {
            Some(p) => p.write_user(&text).map_err(|e| e.to_string()),
            None => Err("coder not running".to_string()),
        };
        if let Err(e) = wrote {
            self.drop_coder(i);
            self.finish_coder(i, Err(format!("coder write failed: {e}")));
        }
    }

    // Go spawnResident: after the main child starts, the prompter, tui and base coder start and stay parked,
    // in slot-table order (main is already up). A failed spawn is left to its first use, which reports it.
    pub(super) fn spawn_residents(&mut self) {
        if self.coders.is_empty() {
            self.coders.resize_with(1, Coder::default);
        }
        let roles: Vec<&'static str> = self.resident_slots.iter().map(|(r, _)| *r).collect();
        for role in roles {
            let _ = match role {
                "prompter" => self.spawn_prompter(),
                "tui" => self.spawn_tui(),
                "coder" => self.spawn_coder(0),
                _ => Ok(()),
            };
        }
    }

    // Go residentSlots model of a role. A role outside the table (coder2..) is haiku, as Go coderSlot.
    pub(super) fn resident_model(&self, role: &str) -> String {
        self.resident_slots
            .iter()
            .find(|(r, _)| *r == role)
            .map_or_else(|| "haiku".to_string(), |(_, m)| m.clone())
    }

    // Go residentSlots coder: haiku with the haiku-coder agent body, the coder tools, strict MCP and no slash commands.
    pub(super) fn spawn_coder(&mut self, i: usize) -> Result<(), String> {
        self.resident_sids.remove(&coder_role(i));
        let cwd = self.opts.cwd.clone().unwrap_or_else(|| ".".into());
        let rule = self.agent_rule(&cwd, "haiku-coder");
        let opts = SpawnOpts {
            model: self.resident_model(&coder_role(i)),
            resume: self.resume_pending.remove(&coder_role(i)),
            max_budget_usd: 0.0,
            shape: Shape {
                minimal: false,
                tools: CODER_TOOLS.to_string(),
                strict_mcp: true,
                no_slash: true,
                rule,
                mcp_config: None,
            },
            ..self.opts.clone()
        };
        self.coder_epoch += 1;
        let epoch = self.coder_epoch;
        self.coders[i].epoch = epoch;
        let p = proc::spawn(&opts, self.self_tx.clone(), move |m| {
            Input::Coder(i, epoch, m)
        })
        .map_err(|e| e.to_string())?;
        self.coders[i].proc = Some(p);
        // The idle timers run from spawn: the base coder starts at boot, before any task.
        self.coders[i].idle_since = Some(Instant::now());
        self.emit_coder_contexts();
        Ok(())
    }

    // A message is current only while its slot still has the child that sent it.
    pub(super) fn coder_matches(&self, i: usize, epoch: u64) -> bool {
        self.coders.get(i).is_some_and(|c| c.epoch == epoch)
    }

    // Kills the slot's child and bumps its generation, so its late lines are dropped.
    pub(super) fn drop_coder(&mut self, i: usize) {
        if let Some(mut p) = self.coders[i].proc.take() {
            p.kill();
        }
        self.coder_epoch += 1;
        self.coders[i].epoch = self.coder_epoch;
        self.coders[i].idle_since = None;
        self.emit_coder_contexts();
    }

    // Keeps the session id a resident result line carries, for its role; owned too.
    pub(super) fn note_resident(&mut self, role: &str, session_id: Option<String>) {
        if let Some(id) = session_id.filter(|id| !id.is_empty()) {
            self.note_own(&id);
            if self.resident_sids.get(role) != Some(&id) {
                self.record_session(role, &id);
            }
            self.resident_sids.insert(role.to_string(), id);
            self.emit_session_ids();
        }
    }

    // Ends an idle resident child with ClaudeProc::finish. Its session id stays in resume_pending, so the next
    // task respawns it with --resume on the same session.
    pub(super) fn exit_idle_coder(&mut self, i: usize) {
        if let Some(p) = self.coders[i].proc.take() {
            // The stdin close and the wait run off the actor thread, so an Interrupt is not held up by a 5 s exit.
            thread::spawn(move || {
                let mut p = p;
                p.finish(CODER_EXIT_WAIT);
            });
        }
        let role = coder_role(i);
        if let Some(sid) = self.resident_sids.get(&role).cloned() {
            self.resume_pending.insert(role, sid);
        }
        self.coder_epoch += 1;
        self.coders[i].epoch = self.coder_epoch;
        self.coders[i].idle_since = None;
        self.emit_coder_contexts();
    }

    // The coder's result line is its reply. An empty reply is a failure, as for the tui.
    pub(super) fn on_coder_line(&mut self, i: usize, line: &str) {
        if let Ok(Event::ControlRequest(c)) = parse_line(line)
            && c.request.subtype == "can_use_tool"
        {
            self.coder_permission(i, c);
            return;
        }
        if self.coders[i].deadline.is_none() {
            return;
        }
        let Ok(Event::Result(r)) = parse_line(line) else {
            return;
        };
        self.note_resident(&coder_role(i), r.session_id.clone());
        self.role_context(&coder_role(i), r.usage.as_ref());
        let raw = r.result.unwrap_or_default();
        if r.is_error == Some(true) && retryable_api_error(&raw) {
            if !self.coder_retry(i) {
                self.finish_coder(i, Err(raw));
            }
            return;
        }
        let text = summarize_reply(&raw);
        if text.is_empty() {
            self.finish_coder(i, Err("coder empty reply".to_string()));
        } else {
            self.finish_coder(i, Ok(text));
        }
    }

    // Go parkedIdleEvent: a resident coder's can_use_tool is approved only while it is busy (coder_may_use);
    // every other request is denied with the Go parked note.
    pub(super) fn coder_permission(&mut self, i: usize, c: crate::event::ControlRequest) {
        let tool = c.request.tool_name.unwrap_or_default();
        let input = c.request.input.unwrap_or(Value::Null);
        let busy = self.coders[i].deadline.is_some();
        let allow = coder_may_use(busy, self.opts.cwd.as_deref(), &tool, &input);
        let res = match self.coders[i].proc.as_mut() {
            Some(p) if allow => p.write_permission(&c.request_id, true, input, ""),
            Some(p) => p.write_permission(
                &c.request_id,
                false,
                Value::Null,
                "denied: session parked (another model is active)",
            ),
            None => Ok(()),
        };
        if let Err(e) = res {
            self.emit(UiEvent::Error(format!("permission reply failed: {e}")));
        }
    }

    pub(super) fn on_coder_eof(&mut self, i: usize) {
        self.coders[i].proc = None;
        self.emit_coder_contexts();
        if self.coders[i].deadline.is_some() && !self.coder_retry(i) {
            self.finish_coder(i, Err("coder exited".to_string()));
        }
    }

    // Ends a coder task: its stage row, then the reply (or the failure reason) goes to the UI.
    pub(super) fn finish_coder(&mut self, i: usize, out: Result<String, String>) {
        self.coders[i].task = None;
        self.coders[i].retries = 0;
        self.coders[i].retry_started = None;
        self.coders[i].retry_at = None;
        if self.coders[i].deadline.take().is_some() {
            self.resident_busy(&coder_role(i), false, "");
        }
        self.coders[i].idle_since = Some(Instant::now());
        let waiter = self.coders[i].waiter.take();
        let role = coder_role(i);
        answer(waiter, out.clone());
        // Every turn end is a handback: a failed task still sends its reason as the reply, so main never sees silence.
        match out {
            Ok(text) => {
                self.stage(&role, StagePhase::Done);
                self.emit(UiEvent::CoderReply { role, text });
            }
            Err(reason) => {
                self.emit(UiEvent::CoderReply {
                    role: role.clone(),
                    text: tf(Msg::Error, &[&reason]),
                });
                self.stage(&role, StagePhase::Failed(reason));
            }
        }
    }

    // The tui's result line is its answer. An empty answer is a failure (Go empty reply).
    pub(super) fn on_tui_line(&mut self, line: &str) {
        if let Ok(Event::ControlRequest(c)) = parse_line(line)
            && c.request.subtype == "can_use_tool"
        {
            deny_parked(self.tui_proc.as_mut(), &c.request_id);
            return;
        }
        if self.tui_deadline.is_none() {
            return;
        }
        let Ok(Event::Result(r)) = parse_line(line) else {
            return;
        };
        self.note_resident("tui", r.session_id.clone());
        self.role_context("tui", r.usage.as_ref());
        let text = r.result.unwrap_or_default().trim().to_string();
        if text.is_empty() {
            self.finish_tui(Err(t(Msg::EmptyResponse).to_string()));
        } else {
            self.finish_tui(Ok(text));
        }
    }

    pub(super) fn on_tui_eof(&mut self) {
        self.tui_proc = None;
        if self.tui_deadline.is_some() {
            self.finish_tui(Err(tf(Msg::Error, &[&"exited"])));
        }
    }

    // With residents the main text waits for the tui answer. Text written before a tool call is the work plan: it is
    // shown when that tool call arrives, so it stays above the tools and the final reply still comes from the tui.
    pub(super) fn release_routed_text(&mut self) {
        if self.cfg.residents && self.busy() && self.handoff.is_none() {
            let text = std::mem::take(&mut self.routed_text);
            if !text.is_empty() {
                self.emit(UiEvent::TextDelta {
                    role: Role::Main,
                    text,
                });
                self.emit(UiEvent::MessageDone { role: Role::Main });
            }
        }
    }

    // Ends the tui wait: the answer (or its failure row), then the turn is over.
    pub(super) fn finish_tui(&mut self, out: Result<String, String>) {
        if self.tui_deadline.take().is_some() {
            self.resident_busy("tui", false, "");
        }
        let raw = std::mem::take(&mut self.routed_text);
        match out {
            Ok(text) => {
                self.stage("tui", StagePhase::Done);
                self.emit(UiEvent::TuiAnswer { text });
            }
            Err(detail) => {
                // Go viaTUI: on failure the raw main reply is shown, then the notice.
                if !raw.is_empty() {
                    self.emit(UiEvent::TextDelta {
                        role: Role::Main,
                        text: raw,
                    });
                    self.emit(UiEvent::MessageDone { role: Role::Main });
                }
                self.notice(tui_notice(&detail));
            }
        }
        self.after_turn();
    }

    // Writes one user frame and starts the turn clock. Returns false if the write failed.
    pub(super) fn send_turn(&mut self, text: &str) -> bool {
        // A release continuation keeps the task list; any other prompt starts a fresh one.
        if !std::mem::take(&mut self.continuation) && !self.queued_tasks.is_empty() {
            self.queued_tasks.clear();
            self.persist_tasks();
        }
        // Go handle/runTurn: each prompt starts a fresh ledger, and is refused when its next call would exceed the cap.
        if let Some(b) = self.budget.as_mut() {
            b.reset();
        }
        self.budget_hit = false;
        let est = self.estimate_next();
        if self.budget.as_ref().is_some_and(|b| !b.allow(est)) {
            self.emit_budget_stop();
            return false;
        }
        let Some(p) = self.proc.as_mut() else {
            return false;
        };
        // Go runTurn: queued notices go in front of the prompt; the pending snapshot of a rotated session goes in
        // front of the first prompt sent to it.
        let body = with_notices(&std::mem::take(&mut self.notices), text);
        let frame = if self.pending_snapshot.is_empty() {
            body
        } else {
            format!(
                "{}\n\n## Current user message\n{}",
                self.pending_snapshot, body
            )
        };
        match p.write_user(&frame) {
            Ok(()) => {
                self.pending_snapshot.clear();
                self.turn_prompt = text.to_string();
                self.turn_reply.clear();
                self.routed_text.clear();
                self.mark = MarkerFilter::default();
                self.start_turn_clock();
                self.stage("main", StagePhase::Working);
                self.last_activity = Instant::now();
                self.turn_cost_base = self.cost_usd;
                self.delegations = 0;
                self.spend = TurnSpend::new();
                self.turn_msg_usage.clear();
                true
            }
            Err(e) => {
                self.emit(UiEvent::Error(format!("write to claude failed: {e}")));
                false
            }
        }
    }

    // Go /model NAME (repl.go runTUI): the parked child for NAME is ended, the session model changes, and a rotation
    // runs with reason "model NAME". The handoff runs on the current child; the fresh child starts on NAME.
    pub(super) fn model_switch(&mut self, model: String) {
        self.end_parked(&model);
        self.model = model.clone();
        self.opts.model = model.clone();
        self.set_busy(true);
        self.emit(UiEvent::Busy(true));
        if !self.start_handoff(format!("model {model}")) {
            self.set_busy(false);
            self.emit(UiEvent::Busy(false));
        }
    }

    // Go endParked(key, reason): the parked child under (cwd, model) is closed and its registry row marked ended.
    pub(super) fn end_parked(&mut self, model: &str) {
        let cwd = self.client_cwd();
        if let Some(mut p) = self.children.take(&child_key(&cwd, model, "")) {
            p.sess.close();
            if let Some(reg) = &self.clients {
                reg.mark_ended(&cwd, model, "model-switch", now_stamp());
            }
        }
    }
}
