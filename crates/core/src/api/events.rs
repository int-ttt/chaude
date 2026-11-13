//! Event handling on State: UiEvent emission, child and sub-agent stream lines, agent rows, queued notices.

use super::*;

impl State {
    pub(super) fn emit(&self, ev: UiEvent) {
        // The picked child's row follows the busy state, so the list updates on every Busy event.
        let busy = match &ev {
            UiEvent::Busy(b) if self.picked_row.is_some() => Some(*b),
            _ => None,
        };
        let _ = self.ui.send(ev);
        if let Some(b) = busy {
            self.emit_agent_rows_at(b);
        }
    }

    // Runs the one-shot on its own thread so the actor keeps streaming a busy turn.
    pub(super) fn usage(&self) {
        let bin = resolve_bin(&self.cfg.claude_bin);
        let cwd = self.engine_cwd();
        let ui = self.ui.clone();
        thread::spawn(move || {
            let ev = match proc::one_shot_usage(&bin, &cwd) {
                Ok(text) => UiEvent::Usage { text },
                Err(e) => UiEvent::Error(e.to_string()),
            };
            let _ = ui.send(ev);
        });
    }

    // Go pollUsage: after a turn ends, at most one background /usage poll per interval. Off when
    // HARNESS_USAGE_POLL=0 or metrics are off (Go returns before the throttle, so no claude runs).
    pub(super) fn poll_usage(&mut self) {
        if self.cfg.metrics_dir.is_none()
            || std::env::var("HARNESS_USAGE_POLL").as_deref() == Ok("0")
        {
            return;
        }
        let now = Instant::now();
        if !usage_poll_due(
            self.usage_poll_at,
            now,
            self.usage_polling.load(Ordering::SeqCst),
        ) {
            return;
        }
        self.usage_poll_at = Some(now);
        self.usage_polling.store(true, Ordering::SeqCst);
        let bin = resolve_bin(&self.cfg.claude_bin);
        let cwd = self.engine_cwd();
        let metrics = Arc::clone(&self.metrics);
        let polling = Arc::clone(&self.usage_polling);
        thread::spawn(move || {
            let text = proc::one_shot_usage(&bin, &cwd).map_err(|e| e.to_string());
            let _ = log_usage(&metrics, text.as_deref().map_err(|e| e.as_str()));
            polling.store(false, Ordering::SeqCst);
        });
    }

    // Records a session id this run started (Go noteOwn), once.
    pub(super) fn note_own(&mut self, id: &str) {
        if !self.owned.iter().any(|o| o == id) {
            self.owned.push(id.to_string());
        }
    }

    // Go /view: this run's session ids, sent whenever the main or a resident id changes.
    pub(super) fn emit_session_ids(&self) {
        self.emit(UiEvent::SessionIds {
            main: self.sid.clone(),
            residents: self.resident_sids.clone(),
            owned: self.owned.clone(),
        });
        self.persist_live();
    }

    // Tokens in the role's context window from its result usage (Go per-role ctx). Nothing without usage.
    pub(super) fn role_context(&mut self, role: &str, usage: Option<&Usage>) {
        if let Some(u) = usage {
            let tokens = u.context_tokens();
            self.role_ctx.insert(role.to_string(), tokens);
            self.emit(UiEvent::RoleContext {
                role: role.to_string(),
                tokens,
            });
            if role.starts_with("coder") {
                self.emit_coder_contexts();
            }
            self.persist_live();
        }
    }

    // The coder line: every live coder child in spawn order (slot index order), with its context tokens (0 until
    // its first result) and its rotation limit (the threshold its model rotates at, as the Go per-role limit).
    pub(super) fn emit_coder_contexts(&self) {
        let coders = self
            .coders
            .iter()
            .enumerate()
            .filter(|(_, c)| c.proc.is_some())
            .map(|(i, _)| {
                let role = coder_role(i);
                CoderContext {
                    slot: i + 1,
                    tokens: self.role_ctx.get(&role).copied().unwrap_or(0),
                    limit: threshold_for(&self.resident_model(&role), self.cfg.rotate_threshold),
                }
            })
            .collect();
        self.emit(UiEvent::CoderContexts { coders });
    }

    // Go estNext: planned cost of the next call. Warm while the last API response is inside the cache TTL
    // (Go idleDeadline, capped at 290 s for the 5m cache and 1 h for the 1h cache; no idle-rotate setting here).
    pub(super) fn estimate_next(&self) -> f64 {
        let ttl = effective_ttl(&self.ttl_seen, &self.cfg.cache_ttl);
        let window = if ttl == "1h" {
            Duration::from_secs(3600)
        } else {
            Duration::from_secs(290)
        };
        let warm = self.last_api.is_some_and(|t| t.elapsed() < window);
        budget::estimate(&self.model, self.ctx_tokens, warm, ttl)
    }

    pub(super) fn emit_budget_stop(&self) {
        if let Some(b) = &self.budget {
            self.emit(UiEvent::BudgetStop {
                spent_usd: b.spent(),
                cap_usd: b.cap(),
            });
        }
    }

    // Go runTurn assistant case: a main usage event whose in-flight estimate reaches the cap interrupts the turn once.
    pub(super) fn budget_mid_turn(&mut self, u: &Usage) {
        if self.budget_hit {
            return;
        }
        let est = budget::mid_turn_estimate(&self.model, u);
        let Some(b) = self.budget.as_mut() else {
            return;
        };
        if !b.over(est) {
            return;
        }
        self.budget_hit = true;
        if let Some(p) = self.proc.as_mut()
            && let Err(e) = p.write_interrupt()
        {
            self.emit(UiEvent::Error(format!("interrupt write failed: {e}")));
        }
        self.interrupted_at.get_or_insert_with(Instant::now);
    }

    pub(super) fn interrupt(&mut self, src: InterruptSource) {
        // A held reply has no child turn to interrupt: the interrupt ends the wait and the held turn ends.
        if self.held.is_some() {
            self.cancel_wait();
            return;
        }
        let busy = self.busy;
        let mut sent = false;
        // While the tui translates, the main child is idle: there is nothing to interrupt.
        if busy
            && self.tui_deadline.is_none()
            && let Some(p) = self.proc.as_mut()
        {
            match p.write_interrupt() {
                Ok(_) => sent = true,
                Err(e) => self.emit(UiEvent::Error(format!("interrupt write failed: {e}"))),
            }
            self.interrupted_at.get_or_insert_with(Instant::now);
        }
        self.log_interrupt(src, busy, sent);
    }

    pub(super) fn log_interrupt(&self, src: InterruptSource, busy: bool, sent: bool) {
        let Some(path) = &self.cfg.log_path else {
            return;
        };
        let line = format!(
            "{} interrupt source={} busy={busy} sent={sent}\n",
            now_secs(),
            src.as_str()
        );
        let res = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(line.as_bytes()));
        if let Err(e) = res {
            self.emit(UiEvent::Error(format!("interrupt log failed: {e}")));
        }
    }

    pub(super) fn on_eof(&mut self) {
        self.open_tools.clear();
        self.interrupted_at = None;
        self.stray_results = 0;
        self.handoff = None;
        self.emit(UiEvent::Error("claude exited".to_string()));
        if let Some(mut p) = self.proc.take() {
            p.kill();
        }
        self.queue.clear();
        self.emit_queue();
        if self.busy {
            self.busy = false;
            self.turn_started = None;
            self.emit(UiEvent::Busy(false));
        }
    }

    pub(super) fn on_line(&mut self, line: &str) {
        self.last_activity = Instant::now();
        if let Some(t) = observe_ttl(line, &self.ttl_seen) {
            self.ttl_seen = t;
        }
        if let Ok(raw) = serde_json::from_str::<Value>(line) {
            self.spend.on_event(&raw);
            self.on_sub_agent_line(&raw);
        }
        match parse_line(line) {
            Ok(ev) => self.on_event(ev),
            Err(_) => self.log_unparsed(line),
        }
    }

    // Go repl.go _unparsed -> Transcript.add("unparsed"): trace only, the turn is not affected. Write errors are ignored.
    pub(super) fn log_unparsed(&self, line: &str) {
        self.log_transcript("unparsed", json!({"raw": line}));
    }

    // Go Transcript.add: one JSONL row {"t", "kind", ...fields} appended to the transcript. Write errors are ignored.
    pub(super) fn log_transcript(&self, kind: &str, fields: Value) {
        let Some(path) = &self.cfg.transcript else {
            return;
        };
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let mut row = json!({"t": t, "kind": kind});
        if let (Some(m), Value::Object(f)) = (row.as_object_mut(), fields) {
            m.extend(f);
        }
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(format!("{row}\n").as_bytes()));
    }

    pub(super) fn on_event(&mut self, ev: Event) {
        match ev {
            Event::System(s) => {
                self.on_system_task(&s);
                if let Some(id) = s.session_id {
                    self.note_own(&id);
                    let prev = std::mem::replace(&mut self.sid, id.clone());
                    if prev != id {
                        self.record_session(handoff::MAIN_ROLE, &id);
                        // A rotation clears sid first, so a non-empty previous id here means a /clear.
                        if !prev.is_empty() {
                            self.emit(UiEvent::NewSession { old_session: prev });
                        }
                        self.emit_session_ids();
                    }
                }
                if let Some(m) = s.model {
                    self.model = m;
                    self.emit_status(self.cost_usd);
                }
            }
            Event::StreamEvent(w) => {
                let role = match w.parent_tool_use_id {
                    Some(id) => Role::Agent(id),
                    None => Role::Main,
                };
                if matches!(role, Role::Main) {
                    self.note_held_turn();
                }
                match w.event {
                    StreamInner::ContentBlockDelta {
                        delta: Delta::TextDelta { text },
                    } => {
                        if matches!(role, Role::Main) && !self.busy {
                            self.idle_started();
                        }
                        // The raw reply keeps the [cat: X] marker for category_rotation; the screen gets it filtered.
                        let shown = if matches!(role, Role::Main) {
                            self.turn_reply.push_str(&text);
                            self.mark.feed(&text)
                        } else {
                            text
                        };
                        // With residents the pane shows only the tui answer (Go runTurn routed): hold main text back.
                        // An idle reply (agent handback) has no tui wait to drain it, so it shows at once (Go idleText).
                        if self.cfg.residents && self.busy && matches!(role, Role::Main) {
                            self.routed_text.push_str(&shown);
                        } else if self.handoff.is_none() && !shown.is_empty() {
                            self.emit(UiEvent::TextDelta { role, text: shown });
                        }
                    }
                    StreamInner::MessageStop if self.handoff.is_none() => {
                        if matches!(role, Role::Main) {
                            let rest = self.mark.flush();
                            if self.cfg.residents && self.busy {
                                self.routed_text.push_str(&rest);
                            } else if !rest.is_empty() {
                                self.emit(UiEvent::TextDelta {
                                    role: role.clone(),
                                    text: rest,
                                });
                            }
                        }
                        self.emit(UiEvent::MessageDone { role })
                    }
                    _ => {}
                }
            }
            Event::Assistant(env) => {
                // Subagent messages are not surfaced beyond their text in E1; they only refresh their agent's activity.
                if let Some(pid) = &env.parent_tool_use_id {
                    self.note_agent_activity(pid);
                    return;
                }
                self.note_held_turn();
                if !self.busy {
                    self.idle_started();
                }
                // The server-reported model of the reply: the label follows it when it differs from the init model.
                if let Some(m) = env
                    .message
                    .model
                    .clone()
                    .filter(|m| !m.is_empty() && *m != self.model)
                {
                    self.model = m;
                    self.emit_status(self.cost_usd);
                }
                if let (Some(id), Some(u)) = (&env.message.id, &env.message.usage) {
                    self.turn_msg_usage
                        .entry(id.clone())
                        .or_default()
                        .max_with(u);
                }
                if let Some(u) = &env.message.usage {
                    self.ctx_tokens = u.context_tokens();
                    self.persist_live();
                    self.last_api = Some(Instant::now());
                    self.budget_mid_turn(u);
                    self.emit_status(self.cost_usd);
                }
                if let Some(Content::Blocks(blocks)) = env.message.content {
                    if blocks.iter().any(|b| matches!(b, Block::ToolUse { .. })) {
                        self.release_routed_text();
                    }
                    for b in blocks {
                        if let Block::ToolUse { id, name, input } = b {
                            self.on_tool_use(id, name, input.unwrap_or(Value::Null));
                        }
                    }
                }
            }
            Event::User(env) => {
                if let Some(pid) = &env.parent_tool_use_id {
                    self.note_agent_activity(pid);
                    return;
                }
                self.note_held_turn();
                if let Some(Content::Text(text)) = &env.message.content {
                    self.on_user_notice(text);
                }
                if let Some(Content::Blocks(blocks)) = env.message.content {
                    // The top-level tool_use_result belongs to the single tool result of its message.
                    let single = blocks
                        .iter()
                        .filter(|b| matches!(b, Block::ToolResult { .. }))
                        .count()
                        == 1;
                    let stats = env.tool_use_result.as_ref().and_then(tool_stats);
                    for b in blocks {
                        match b {
                            Block::Text { text } => self.on_user_notice(&text),
                            Block::ToolResult {
                                tool_use_id,
                                content,
                                is_error,
                            } => {
                                if let (true, Some(s)) = (single, stats) {
                                    self.record_task_stats(&tool_use_id, s);
                                }
                                self.on_tool_result(tool_use_id, content, is_error == Some(true))
                            }
                            _ => {}
                        }
                    }
                }
            }
            Event::Result(r) => {
                if self.stray_results > 0 {
                    self.stray_results -= 1;
                    return;
                }
                self.result_usage = r.usage.clone();
                self.result_model_usage = r.model_usage.clone();
                if !self.busy {
                    self.idle_result();
                    return;
                }
                if let Some(c) = r.total_cost_usd {
                    self.cost_usd = c;
                    if let Some(b) = self.budget.as_mut() {
                        b.charge(CHILD_PROC, c);
                    }
                }
                if r.is_error == Some(true) {
                    let msg = r
                        .result
                        .or(r.subtype)
                        .unwrap_or_else(|| "turn failed".to_string());
                    self.emit(UiEvent::Error(msg));
                }
                let duration_ms = r
                    .duration_ms
                    .unwrap_or_else(|| elapsed_ms(self.turn_started));
                self.emit_status(self.cost_usd);
                if let Some(b) = &self.budget {
                    self.emit(UiEvent::BudgetStatus {
                        spent_usd: b.spent(),
                        cap_usd: b.cap(),
                    });
                }
                self.poll_usage();
                self.end_turn(duration_ms);
            }
            Event::ControlRequest(c) => {
                if c.request.subtype != "can_use_tool" {
                    return;
                }
                let tool = c.request.tool_name.unwrap_or_default();
                let input = c.request.input.unwrap_or(Value::Null);
                match mode::decide(self.mode, &tool) {
                    Verdict::Allow => self.respond_permission(&c.request_id, true, input, ""),
                    Verdict::Deny(msg) => {
                        self.respond_permission(&c.request_id, false, Value::Null, msg)
                    }
                    // Go askPerm: with a UI the ask goes to the UI whether or not a turn runs. Without one
                    // (plain mode) an idle ask is denied with the Go note.
                    Verdict::Ask if self.cfg.auto_allow => {
                        self.respond_permission(&c.request_id, true, input, "")
                    }
                    Verdict::Ask if self.cfg.no_ui && !self.busy => {
                        let note = format!(
                            "[permission denied while idle] {tool} {} (use /mode auto or acceptEdits)",
                            crate::import_view::trunc(&input.to_string(), 120)
                        );
                        self.emit(UiEvent::Notice { text: note });
                        self.respond_permission(
                            &c.request_id,
                            false,
                            Value::Null,
                            "denied: no interactive prompt available between turns",
                        );
                    }
                    Verdict::Ask => {
                        let summary = tool_summary(&input);
                        self.pending_perm.insert(c.request_id.clone(), input);
                        self.emit(UiEvent::PermissionRequest {
                            id: c.request_id,
                            tool,
                            summary,
                        });
                    }
                }
            }
            Event::RateLimitEvent(ev) => {
                if let Some(info) = ev.rate_limit_info {
                    self.rate_log.set_rate(&info, &self.metrics);
                    self.emit(UiEvent::RateLimit { info });
                }
            }
            Event::Unknown => {}
        }
    }

    // Go /mode. Empty name reports the current mode (Go modeLine with no args). Unknown names are reported
    // with the Go message and the mode is unchanged.
    pub(super) fn set_mode(&mut self, name: &str) {
        let next = if name.trim().is_empty() {
            Some(self.mode)
        } else if name.trim() == "next" {
            Some(self.mode.next())
        } else {
            PermMode::parse(name)
        };
        match next {
            Some(m) => {
                self.mode = m;
                self.emit(UiEvent::ModeChanged {
                    mode: m.as_str().to_string(),
                });
            }
            None => self.emit(UiEvent::Error(format!(
                "unknown mode {name} (default | acceptEdits | plan | auto)"
            ))),
        }
    }

    // Go RespondPermission for a pending UI prompt. An id that is not pending is reported, not sent.
    pub(super) fn answer_permission(&mut self, id: &str, allow: bool) {
        match self.pending_perm.remove(id) {
            Some(input) => self.respond_permission(id, allow, input, ""),
            None => self.emit(UiEvent::Error(format!(
                "no pending permission request {id}"
            ))),
        }
    }

    pub(super) fn respond_permission(&mut self, id: &str, allow: bool, input: Value, msg: &str) {
        let res = match self.proc.as_mut() {
            Some(p) => p.write_permission(id, allow, input, msg),
            None => Ok(()),
        };
        if let Err(e) = res {
            self.emit(UiEvent::Error(format!("permission reply failed: {e}")));
        }
    }

    pub(super) fn on_tool_use(&mut self, id: String, name: String, input: Value) {
        if !self.tools.insert(id.clone()) {
            return;
        }
        // Go runTurn: files written by Write/Edit/NotebookEdit are listed in the handoff snapshot.
        if let Some(fp) = input.get("file_path").and_then(Value::as_str)
            && matches!(name.as_str(), "Write" | "Edit" | "NotebookEdit")
            && !fp.is_empty()
            && !self.files.iter().any(|f| f == fp)
        {
            self.files.push(fp.to_string());
        }
        // Go repl.go:743: an edit by a Haiku session that has a session id joins the worker pool.
        if self.model == "haiku"
            && !self.sid.is_empty()
            && matches!(name.as_str(), "Write" | "Edit" | "NotebookEdit")
            && let Some(fp) = input.get("file_path").and_then(Value::as_str)
        {
            self.pool.note_edit(&self.sid, &self.cur_task, fp);
        }
        self.open_tools.insert(id.clone());
        // Go repl.go:721: an Agent call's "· agent <label> 호출" row comes before its tool line.
        if name == "Agent" || name == "Task" {
            let label = subagent_label(&input);
            self.stage(&agent_slot(&label), StagePhase::Working);
            self.agent_types.insert(id.clone(), label);
        }
        let summary = tool_summary(&input);
        self.emit(UiEvent::ToolStart {
            id: id.clone(),
            name: name.clone(),
            summary,
        });
        self.emit(UiEvent::ToolInput {
            id: id.clone(),
            input: input.clone(),
        });
        if name == "Agent" || name == "Task" {
            self.delegations += 1;
            let description = ["description", "subagent_type", "prompt"]
                .iter()
                .find_map(|k| {
                    input
                        .get(*k)
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                })
                .map(|s| clip(s, 80))
                .unwrap_or_default();
            // Go drain.go opusRunning: Name is `name`, else `subagent_type`.
            let agent_name = ["name", "subagent_type"].iter().find_map(|k| {
                input
                    .get(*k)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
            });
            if input.get("model").and_then(Value::as_str) == Some("opus")
                || agent_name.is_some_and(|n| n.starts_with("opus-"))
            {
                self.opus_set(&id, true);
            }
            self.agents.insert(id.clone(), Instant::now());
            self.agent_labels.insert(id.clone(), description.clone());
            self.queue_task(id.clone(), description.clone());
            let started_at = now_secs();
            self.agent_info.insert(
                id.clone(),
                AgentInfo {
                    name: description.clone(),
                    // No result file is written for an agent, so none is advertised.
                    result_path: String::new(),
                    background: input
                        .get("run_in_background")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    started: Instant::now(),
                    done: false,
                    result_done: false,
                    out_path: input
                        .get("prompt")
                        .and_then(Value::as_str)
                        .map(out_path_in)
                        .unwrap_or_default(),
                    model: input
                        .get("model")
                        .and_then(Value::as_str)
                        .filter(|m| !m.is_empty())
                        .map(str::to_string)
                        .or_else(|| agent_type_model(agent_name.unwrap_or_default()))
                        .unwrap_or_default(),
                    task: input
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    prompt: input
                        .get("prompt")
                        .and_then(Value::as_str)
                        .and_then(|p| p.lines().next())
                        .unwrap_or_default()
                        .to_string(),
                    brief: input
                        .get("prompt")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    launch_epoch: self.epoch,
                    launch_seq: self.turn_seq + 1,
                    session_name: ["name", "subagent_type"]
                        .iter()
                        .find_map(|k| {
                            input
                                .get(*k)
                                .and_then(Value::as_str)
                                .filter(|s| !s.is_empty())
                        })
                        .unwrap_or_default()
                        .to_string(),
                    agent_id: String::new(),
                    started_at,
                    last_active: started_at,
                    log: Vec::new(),
                    last_text: String::new(),
                    handback: String::new(),
                    report: String::new(),
                    failed: false,
                },
            );
            self.emit(UiEvent::AgentStart {
                id: id.clone(),
                description,
                started_at,
            });
            self.persist_agent(&id, STO_RUNNING);
            self.emit_agent_rows();
        }
    }

    pub(super) fn on_tool_result(&mut self, id: String, content: Option<Value>, is_error: bool) {
        self.open_tools.remove(&id);
        let text = result_text(content.as_ref());
        // SendMessage to a background agent resumes it: it runs again under its original tool id.
        // The SendMessage result is JSON: {"success":true,"message":"Resuming agent <short>","resumedAgentId":"<full>"}.
        // A queued message ("Message queued for delivery to <id>", no resumedAgentId) opens the same row:
        // the agent is still running and its next notice closes it.
        let resume = serde_json::from_str::<Value>(&text)
            .ok()
            .filter(|v| v["success"] != false)
            .and_then(|v| {
                v["resumedAgentId"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| {
                        v["message"]
                            .as_str()
                            .filter(|m| m.starts_with("Message queued for delivery to"))
                            .and_then(|_| v["pin"]["id"].as_str().map(str::to_string))
                    })
            });
        // agent_info stays after the agent ends, so a resume uses its start label, then the agent id.
        // An agent launched outside this process (earlier run or a sub-agent) has no launch id here: its resumed id keys the row.
        let known = resume.map(|aid| {
            let tool = self
                .agent_ids
                .get(&aid)
                .cloned()
                .unwrap_or_else(|| aid.clone());
            (tool, aid)
        });
        if let Some((tool, aid)) = known {
            let description = self
                .agent_info
                .get(&tool)
                .map(|i| i.name.clone())
                .or_else(|| self.agent_labels.get(&tool).cloned())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| format!("agent {}", aid.chars().take(8).collect::<String>()));
            // A queued message to a still-running agent keeps its running start time.
            self.agents.entry(tool.clone()).or_insert_with(Instant::now);
            self.resume_ids.insert(id.clone(), tool.clone());
            // The resumed agent runs in the current child now. An older drain must not settle it when it closes
            // (that wrote a 타임아웃 notice and ended the live agent while it kept working).
            for d in self.drains.iter_mut() {
                d.started.retain(|s| s != &tool);
            }
            // A resumed agent's next finish is announced again.
            self.announced.remove(&tool);
            if let Some(info) = self.agent_info.get_mut(&tool) {
                info.background = true;
                info.done = false;
            }
            self.persist_agent(&tool, STO_RUNNING);
            self.emit_tool_result(id.clone(), text);
            // The SendMessage call's row stays open as the resumed agent's row: no ToolEnd for it here.
            self.emit(UiEvent::AgentResumed {
                id: tool,
                description,
                started_at: now_secs(),
                send_id: id,
            });
            self.emit_agent_rows();
            return;
        }
        self.emit(UiEvent::ToolEnd {
            id: id.clone(),
            ok: !is_error,
        });
        if is_error && let Some(info) = self.agent_info.get_mut(&id) {
            info.failed = true;
        }
        // A handback launch result has "agentId: <id>" without the async mark: map it too, so a later SendMessage resolves to this row.
        let launched_aid =
            after_marker(&text, "agentId: ").filter(|_| self.agent_info.contains_key(&id));
        if let Some(aid) = launched_aid {
            self.agent_ids.insert(aid.clone(), id.clone());
            if let Some(info) = self.agent_info.get_mut(&id) {
                info.agent_id = aid;
            }
        }
        // A background agent keeps running after its launch result; it ends later, which E1 does not track.
        if text.contains(ASYNC_AGENT_MARK) {
            if let Some(info) = self.agent_info.get_mut(&id) {
                info.background = true;
            }
            self.settle_task(&id, TaskStatus::Running);
            self.emit_tool_result(id.clone(), text);
            self.emit(UiEvent::AgentBackground { id });
            self.emit_agent_rows();
            return;
        }
        if self.agents.contains_key(&id) {
            self.finish_agent(id.clone());
            self.emit_tool_result(id, text);
        } else {
            self.emit_tool_result(id.clone(), text);
            self.finish_agent(id);
        }
    }

    pub(super) fn emit_tool_result(&self, id: String, text: String) {
        self.emit(UiEvent::ToolResult { id, text });
    }

    // Ends a delegated agent: AgentEnd, and the deferred rotation runs if this was the last agent.
    // Go agentTracker rows: every delegated agent in start order, for the panel and background lines.
    // Output or tool activity from a delegated agent's own messages refreshes its last_active in place. The panel
    // re-reads the shared rows on its tick, so no event is sent per message.
    pub(super) fn note_agent_activity(&mut self, id: &str) {
        let now = now_secs();
        let Some(info) = self.agent_info.get_mut(id) else {
            return;
        };
        info.last_active = now;
        let mut rows = self.agent_rows.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(row) = rows.iter_mut().find(|r| r.id == id) {
            row.last_active = now;
        }
    }

    /// A new run for an agent that already ended (resumed) shows its row as running again.
    pub(super) fn reopen_agent(&mut self, tool: String) {
        self.agents.entry(tool.clone()).or_insert_with(Instant::now);
        for d in self.drains.iter_mut() {
            d.started.retain(|s| s != &tool);
        }
        self.announced.remove(&tool);
        if let Some(info) = self.agent_info.get_mut(&tool) {
            info.background = true;
            info.done = false;
        }
        self.persist_agent(&tool, STO_RUNNING);
        self.emit_agent_rows();
    }

    pub(super) fn emit_agent_rows(&self) {
        self.emit_agent_rows_at(self.busy);
    }

    // busy: the state the picked child's row shows (running while busy, done otherwise).
    pub(super) fn emit_agent_rows_at(&self, busy: bool) {
        let mut rows: Vec<(Instant, AgentRow)> = self
            .agent_info
            .iter()
            .map(|(id, i)| {
                (
                    i.started,
                    AgentRow {
                        id: id.clone(),
                        task: i.task.clone(),
                        prompt: i.prompt.clone(),
                        name: i.session_name.clone(),
                        model: i.model.clone(),
                        background: i.background,
                        done: i.done,
                        failed: i.failed,
                        interrupted: false,
                        agent_id: i.agent_id.clone(),
                        result_path: i.result_path.clone(),
                        started_at: i.started_at,
                        last_active: i.last_active,
                    },
                )
            })
            .collect();
        rows.sort_by_key(|(started, _)| *started);
        let rows: Vec<AgentRow> = rows.into_iter().map(|(_, r)| r).collect();
        let rows = with_picked(rows, self.picked_row.as_ref(), busy);
        *self.agent_rows.lock().unwrap_or_else(|e| e.into_inner()) = rows.clone();
        self.emit(UiEvent::AgentRows { rows });
    }

    // Go restoreAgents (agentstore.go:221) for /resume: the stored agents of the imported session are listed
    // below the running rows until the next user line. A fresh start restores none; the records stay on disk.
    pub(super) fn restore_agents_for(&mut self, sid: &str) {
        let saved = self
            .agent_store
            .as_ref()
            .map(|s| s.load(sid))
            .unwrap_or_default();
        let (rows, events) = restored_events(&saved);
        self.restored_rows = rows;
        for ev in events {
            self.emit(ev);
        }
    }

    // Go onSubEvent (repl.go:426, 686): a delegated agent's text and tool lines go to its agent-view log.
    pub(super) fn on_sub_agent_line(&mut self, raw: &Value) {
        self.record_handback(raw);
        let Some(p) = raw
            .get("parent_tool_use_id")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
        else {
            return;
        };
        let Some(info) = self.agent_info.get_mut(p) else {
            return;
        };
        // The agent's own model is the one its stream reports; it overrides the input or type-prefix guess.
        // A change re-emits the panel rows, so the running row shows the model its done row will.
        let model = stream_model(raw);
        let model_changed = model.as_ref().is_some_and(|m| *m != info.model);
        if let Some(m) = model {
            info.model = m;
        }
        let (lines, tool_ids): (Vec<String>, Vec<Option<String>>) =
            sub_event_entries(raw).into_iter().unzip();
        if lines.is_empty() {
            if model_changed {
                self.emit_agent_rows();
            }
            return;
        }
        // Go lastSummary: the newest plain text line of this batch, if any, is the agent's last text.
        let last = last_summary(&lines);
        if !last.is_empty() {
            info.last_text = last;
        }
        // Kept with this arrival second, so a restored agent view shows when each line came.
        let at = now_secs();
        info.log.extend(lines.iter().map(|text| SavedLine {
            at,
            text: text.clone(),
        }));
        let excess = info.log.len().saturating_sub(MAX_AGENT_LOG);
        info.log.drain(..excess);
        self.emit(UiEvent::AgentLog {
            id: p.to_string(),
            lines,
            tool_ids,
        });
        if model_changed {
            self.emit_agent_rows();
        }
    }

    // Queued prompt texts, oldest first (Go m.queue), sent with each queue change.
    pub(super) fn emit_queue(&self) {
        self.emit(UiEvent::QueueTexts(self.queue.iter().cloned().collect()));
    }

    // Go resident.go notice(): a failed prompter or tui pass, sent only while residents run.
    pub(super) fn notice(&mut self, text: String) {
        if self.cfg.residents {
            self.emit(UiEvent::Notice { text });
        }
    }

    // Go spawnSlot: a missing agent file shows the notice and the slot starts a default session (no rule).
    pub(super) fn agent_rule(&mut self, cwd: &Path, name: &str) -> String {
        match agent_body(cwd, name, self.cfg.home.as_deref()) {
            Ok(body) => body,
            Err(e) => {
                self.notice(tf(Msg::DefaultSession, &[&e]));
                String::new()
            }
        }
    }

    // Go repl.go stage rows ("· main 처리중", "· agent X 호출"): sent only while residents run.
    pub(super) fn stage(&mut self, slot: &str, phase: StagePhase) {
        if self.cfg.residents {
            self.emit(UiEvent::Stage {
                slot: slot.to_string(),
                phase,
            });
        }
    }

    pub(super) fn finish_agent(&mut self, id: String) {
        let failed = self.agent_info.get(&id).is_some_and(|i| i.failed);
        self.settle_task(
            &id,
            if failed {
                TaskStatus::Failed
            } else {
                TaskStatus::Done
            },
        );
        self.pending_end.remove(&id);
        self.owned_tasks.retain(|_, parent| *parent != id);
        self.agent_task_ids.retain(|_, tool| *tool != id);
        self.opus_set(&id, false);
        if let Some(t0) = self.agents.remove(&id) {
            let label = self
                .agent_types
                .get(&id)
                .cloned()
                .unwrap_or_else(|| "agent".to_string());
            self.stage(&agent_slot(&label), StagePhase::Done);
            let (name, summary, model) = self
                .agent_info
                .get_mut(&id)
                .map(|i| {
                    i.done = true;
                    (i.session_name.clone(), i.done_what(), i.model.clone())
                })
                .unwrap_or_default();
            self.persist_agent(&id, STO_DONE);
            self.emit(UiEvent::AgentEnd {
                id,
                elapsed_ms: t0.elapsed().as_millis() as u64,
                model,
                name,
                summary,
            });
            self.emit_agent_rows();
        }
        // The last background agent finished while the child is idle: the deferred rotation runs now.
        if !self.busy && self.deferred_due() {
            self.run_deferred_rotation();
        }
    }

    // System task events (stream-json). An owned task (owned_by_subagent) keeps its agent open: its completion
    // ends the agent once no owned task of it runs. An agent-level completion (tool_use_id of a tracked agent)
    // goes through complete_agent. Shared by the main stream and the drained children.
    pub(super) fn on_system_task(&mut self, s: &SystemEvent) {
        match s.subtype.as_deref() {
            Some("task_started") => {
                if !s.owned_by_subagent {
                    self.seen_task_started = true;
                    if let (Some(tid), Some(tool), Some("local_agent")) =
                        (&s.task_id, &s.tool_use_id, s.task_type.as_deref())
                    {
                        self.agent_task_ids.insert(tid.clone(), tool.clone());
                        // A resumed run replaces the previous run's handback, text, and report, and cancels a pending end.
                        if let Some(info) = self.agent_info.get_mut(tool) {
                            info.handback.clear();
                            info.last_text.clear();
                            info.report.clear();
                            self.pending_end.remove(tool);
                        }
                        // A run that starts for an agent already ended (resumed) reopens its row.
                        if !self.agents.contains_key(tool) && self.agent_info.contains_key(tool) {
                            self.reopen_agent(tool.clone());
                        }
                    }
                } else if let (Some(tid), Some(parent)) = (&s.task_id, &s.parent_task_id) {
                    // Live claude names the parent by the agent's task id; the engine keys agents by tool_use id.
                    let parent = self
                        .agent_task_ids
                        .get(parent)
                        .cloned()
                        .unwrap_or(parent.clone());
                    self.owned_tasks.insert(tid.clone(), parent);
                }
            }
            Some(sub @ ("task_updated" | "task_notification")) => {
                let done = if sub == "task_updated" {
                    s.patch
                        .as_ref()
                        .and_then(|p| p["status"].as_str())
                        .is_some_and(|st| st == "completed" || st == "killed")
                } else {
                    matches!(s.status.as_deref(), Some("completed" | "stopped"))
                };
                if !done {
                    return;
                }
                if let Some(parent) = s.task_id.as_ref().and_then(|t| self.owned_tasks.remove(t)) {
                    // The last owned task gone restarts a pending agent's grace timer; it does not finish the agent.
                    // The agent ends on its own completion, or on grace expiry when no resumed run cancels the pending end.
                    if self.pending_end.contains_key(&parent) && !self.owned_live(&parent) {
                        self.pending_end.insert(parent, Instant::now());
                    }
                } else if sub == "task_notification"
                    && let Some(id) = s.tool_use_id.clone()
                {
                    let id = self.resume_ids.get(&id).cloned().unwrap_or(id);
                    // The agent's own report becomes the body of its "[agent done]" notice.
                    if let Some(text) = s.handback_report.as_ref().and_then(|r| r["text"].as_str())
                        && let Some(info) = self.agent_info.get_mut(&id)
                    {
                        info.report = text.to_string();
                    }
                    self.complete_agent(id);
                }
            }
            _ => {}
        }
    }

    // An agent's completion. It ends now, unless one of its owned tasks still runs: then it is pending until that
    // task completes or agent_end_grace passes (expire_pending_ends). A drained agent settles through its drain.
    pub(super) fn complete_agent(&mut self, id: String) {
        let id = self.resume_ids.get(&id).cloned().unwrap_or(id);
        if !self.agents.contains_key(&id) {
            self.pending_end.remove(&id);
            return;
        }
        if self.owned_tasks.values().any(|p| *p == id) {
            self.pending_end.entry(id).or_insert_with(Instant::now);
            return;
        }
        if self.drains.iter().any(|d| d.started.contains(&id)) {
            self.settle_drained(&id, true);
        } else {
            // A report of a task that already timed out is not a new reply: its notice waits for the next prompt.
            let late = self
                .queued_tasks
                .iter()
                .any(|t| t.id == id && t.status == TaskStatus::Timeout);
            self.queue_main_notice(&id);
            self.finish_agent(id);
            if !late {
                self.send_notice_turn();
            }
        }
    }

    // A background agent that finished on the main child: its "[agent done]" notice with the final text is queued
    // once per run, unless the native task_notification already carried that text to main. The native body counts
    // as delivered only when this run's SubagentHandback equals the notification's handback_report.
    pub(super) fn queue_main_notice(&mut self, id: &str) {
        let Some(info) = self.agent_info.get(id) else {
            return;
        };
        if !info.background || (!info.handback.is_empty() && info.report == info.handback) {
            return;
        }
        if !announce_once(&mut self.announced, id) {
            return;
        }
        let body = if info.handback.is_empty() {
            info.last_text.clone()
        } else {
            info.handback.clone()
        };
        self.notices.push(drain_done_notice(
            id,
            &info.task,
            &info.out_path,
            true,
            &self.old_brief(info),
            &body,
        ));
    }

    // The brief of an agent the previous main launched (a rotation came after its launch); empty otherwise.
    pub(super) fn old_brief(&self, info: &AgentInfo) -> String {
        if info.launch_epoch == self.epoch {
            String::new()
        } else {
            info.brief.clone()
        }
    }

    // Queued notices go out as a turn of their own while the main child is idle. A child-initiated turn in flight
    // answers its own notification, so the notices wait for the next user message instead (no second reply).
    pub(super) fn send_notice_turn(&mut self) {
        if self.notices.is_empty()
            || self.busy
            || self.hold_notice_turn
            || self.idle_turn.active
            || self.prompter.waiting()
            || self.proc.is_none()
        {
            return;
        }
        self.busy = true;
        self.emit(UiEvent::Busy(true));
        if !self.send_turn("") {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
        }
    }

    // Whether an owned task of this agent still runs.
    pub(super) fn owned_live(&self, id: &str) -> bool {
        self.owned_tasks.values().any(|p| p == id)
    }

    // A user text (string content or text block) that carries a notice (Go onUserEvent). While no turn runs,
    // a seen notice is recorded for the idle turn (Go idleEvent sawNotif).
    pub(super) fn on_user_notice(&mut self, text: &str) {
        let idle = !self.busy;
        self.hold_notice_turn = idle;
        let found = self.on_task_notification(text);
        self.hold_notice_turn = false;
        if found && idle {
            self.idle_turn.notif = true;
        }
    }

    // A <task-notification> names the background agents that finished (Go drain.go onNotification).
    // Returns whether it names a tracked agent, finished or not.
    pub(super) fn on_task_notification(&mut self, text: &str) -> bool {
        let mut found = false;
        for id in task_notification_ids(text) {
            let id = self.resume_ids.get(&id).cloned().unwrap_or(id);
            if self.agent_info.contains_key(&id) {
                found = true;
            }
            self.complete_agent(id);
        }
        found
    }

    // Go repl.go idleFinish: a child-initiated turn ends. Its reply with no visible completion notice means
    // the oldest background agent finished (Go drain.go finishOldestBackground).
    pub(super) fn idle_started(&mut self) {
        self.idle_turn.active = true;
        self.idle_turn.t0.get_or_insert_with(Instant::now);
    }

    // Go idleFinish: a child-initiated reply ends the oldest background agent unless a notice already named it,
    // and is logged as a background turn (metrics stage=turn, source=background; repl.go:518-575).
    pub(super) fn idle_result(&mut self) {
        let idle = std::mem::take(&mut self.idle_turn);
        // With a system task_started seen, the agents end on their own events: the reply-ends-oldest fallback is off.
        if idle.active && !idle.notif && !self.seen_task_started {
            self.finish_oldest_background();
        }
        if idle.active {
            let duration_ms = idle.t0.map_or(0, |t| t.elapsed().as_millis() as u64);
            self.log_turn(duration_ms, Some("background"));
        }
    }

    // Go drain.go finishOldestBackground: the longest-running background agent ends.
    pub(super) fn finish_oldest_background(&mut self) {
        let oldest = self
            .agents
            .iter()
            .filter(|(id, _)| self.agent_info.get(*id).is_some_and(|i| i.background))
            .min_by_key(|(_, t0)| **t0)
            .map(|(id, _)| id.clone());
        if let Some(id) = oldest {
            self.finish_agent(id);
        }
    }

    // Go repl.go turn record (stage "turn"). Keys the Rust engine does not produce yet (draining, rotations)
    // are left out until their groups land. Write errors are ignored, as in Go.
    // Go recordSave: the turn that just ended goes to its project folder under the record root (slash
    // commands and empty replies are skipped by RecordState). A failed write is reported as an Error.
    // Go clearMark: a /clear splits /resume groups at this time, in the record dir of cwd.
    pub(super) fn clear_mark(&self) {
        let cwd = self.engine_cwd();
        let root = self.record_root(&cwd);
        let cwd_text = cwd.to_string_lossy();
        if let Err(e) = resume_group::clear_mark_now(&root, &cwd_text) {
            self.emit(UiEvent::Error(format!("clear mark failed: {e}")));
        }
    }

    // Go recordProjectName: the cwd is absolute (os.Getwd when unset). A "." fallback would name the
    // record folder "misc" and key /clear marks differently from the absolute cwd /resume reads.
    // Pending main Agent/Task calls: the reply is held while any runs.
    pub(super) fn running_tasks(&self) -> usize {
        self.queued_tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .count()
    }

    // Registers a main Agent/Task call (on_tool_use). Its settle comes from its result, hand-back, notice or timeout.
    pub(super) fn queue_task(&mut self, id: String, desc: String) {
        if self.queued_tasks.iter().any(|t| t.id == id) {
            return;
        }
        self.queued_tasks.push(QueuedTask {
            id: id.clone(),
            desc: desc.clone(),
            status: TaskStatus::Running,
            since: Instant::now(),
            tokens: None,
            duration_ms: None,
        });
        self.emit(UiEvent::TaskQueued { id, desc });
        self.persist_tasks();
    }

    // Stores a task's agent totals from its tool result and tells the TUI. Unknown ids are ignored.
    pub(super) fn record_task_stats(&mut self, id: &str, s: ToolStats) {
        let Some(t) = self.queued_tasks.iter_mut().find(|t| t.id == id) else {
            return;
        };
        t.tokens = s.tokens;
        t.duration_ms = s.duration_ms;
        self.emit(UiEvent::TaskStats {
            id: id.to_string(),
            tokens: s.tokens,
            duration_ms: s.duration_ms,
        });
        self.persist_tasks();
    }

    // Settles a queued task. Running only acknowledges an async launch and keeps the task pending.
    pub(super) fn settle_task(&mut self, id: &str, status: TaskStatus) {
        let Some(t) = self.queued_tasks.iter_mut().find(|t| t.id == id) else {
            return;
        };
        if t.status != TaskStatus::Running {
            return;
        }
        if status != TaskStatus::Running {
            t.status = status;
            self.persist_tasks();
        }
        self.emit(UiEvent::TaskResolved {
            id: id.to_string(),
            status,
        });
        self.release_held();
    }

    // Main output while a reply is held means a child-initiated turn started (its own hand-back notice).
    pub(super) fn note_held_turn(&mut self) {
        if self.held.is_some() {
            self.held_turn = true;
        }
    }

    // Ends the hold once no queued task runs: one continuation input carries the results to main.
    // A child turn still running defers the release to its end (end_turn), so no input goes in mid-turn.
    pub(super) fn release_held(&mut self) {
        if self.held.is_none() || self.running_tasks() > 0 || self.held_turn {
            return;
        }
        self.held = None;
        self.emit(UiEvent::ReplyReleased);
        let rows: Vec<String> = self
            .queued_tasks
            .iter()
            .map(|t| format!("- {} ({}): {}", t.id, t.desc, task_status_name(t.status)))
            .collect();
        let text = format!(
            "The background agents you started have settled:\n{}\nContinue with the reply to the user.",
            rows.join("\n")
        );
        self.continuation = true;
        if !self.send_turn(&text) {
            self.busy = false;
            self.emit(UiEvent::Busy(false));
        }
    }

    // Main's turn ended with queued tasks still running: its final text is held (ReplyHeld), no TurnEnd yet.
    pub(super) fn hold_reply(&mut self, duration_ms: u64) -> bool {
        let pending = self.running_tasks();
        if !self.cfg.agent_wait
            || pending == 0
            || self.interrupted_at.is_some()
            || self.budget_hit
            || self.handoff.is_some()
        {
            return false;
        }
        self.held = Some((self.turn_reply.clone(), duration_ms));
        self.emit(UiEvent::ReplyHeld { pending });
        true
    }

    // Abandons the wait of a held reply: the running tasks settle as Timeout, then the held text ends the turn.
    pub(super) fn cancel_wait(&mut self) {
        let Some((reply, duration_ms)) = self.held.take() else {
            return;
        };
        // The held text is the reply; text of a child turn that ran during the hold is not.
        self.turn_reply = reply;
        self.held_turn = false;
        let ids: Vec<String> = self
            .queued_tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .map(|t| t.id.clone())
            .collect();
        for id in ids {
            self.settle_task(&id, TaskStatus::Timeout);
        }
        self.finish_turn(duration_ms);
    }

    pub(super) fn emit_status(&self, cost_usd: f64) {
        let window = self.cfg.context_window.max(1);
        let pct = u32::try_from(self.ctx_tokens.saturating_mul(100) / window).unwrap_or(u32::MAX);
        self.emit(UiEvent::Status {
            model: self.model.clone(),
            tokens: self.ctx_tokens,
            cost_usd,
            context_pct: pct,
        });
    }
}
