//! Persistence on State: live session, agent, task and turn records, carried context and handoff snapshots.

use super::*;

impl State {
    // Appends a changed session id to the record folder's session log (/resume lists it). Only with a handoff path.
    pub(super) fn record_session(&self, role: &str, session_id: &str) {
        if let Some(path) = &self.cfg.handoff_path {
            let _ = handoff::append_session(path, role, session_id);
        }
    }

    // Rewrites the live marker with this run's ids and ctx, so a force kill leaves the latest state behind.
    // Only with a handoff path (the TUI sets one).
    pub(super) fn persist_live(&self) {
        if let Some(path) = &self.cfg.handoff_path {
            // No session id yet (startup, after /clear): write_live_marker writes nothing, never the shared name.
            let _ = handoff::write_live_marker(path, &self.sid, &self.exit_handoff());
        }
    }

    // This run's session ids for the handoff (Go handoffSnapshot). A resident counts only while its child runs.
    pub(super) fn exit_handoff(&self) -> ExitHandoff {
        let mut roles = vec![
            ("prompter".to_string(), self.prompter_proc.is_some()),
            ("tui".to_string(), self.tui_proc.is_some()),
        ];
        roles.extend(
            self.coders
                .iter()
                .enumerate()
                .map(|(i, c)| (coder_role(i), c.proc.is_some())),
        );
        let resident_sessions: BTreeMap<String, String> = roles
            .into_iter()
            .filter(|(_, alive)| *alive)
            .filter_map(|(role, _)| self.resident_sids.get(&role).map(|id| (role, id.clone())))
            .collect();
        let mut ctx_tokens: BTreeMap<String, u64> = self
            .role_ctx
            .iter()
            .filter(|(role, _)| resident_sessions.contains_key(*role))
            .map(|(role, tokens)| (role.clone(), *tokens))
            .collect();
        if !self.sid.is_empty() {
            ctx_tokens.insert(handoff::MAIN_ROLE.to_string(), self.ctx_tokens);
        }
        // Live children by role (Go child_pid), so a dead harness's leftovers can be found at the next start.
        // Go residentSlots: every resident runs haiku; the main child runs the session model.
        let child_models: BTreeMap<String, String> = self
            .proc
            .iter()
            .map(|_| (handoff::MAIN_ROLE.to_string(), self.model.clone()))
            .chain(
                self.prompter_proc
                    .iter()
                    .map(|_| ("prompter".to_string(), "haiku".to_string())),
            )
            .chain(
                self.tui_proc
                    .iter()
                    .map(|_| ("tui".to_string(), "haiku".to_string())),
            )
            .chain(self.coders.iter().enumerate().filter_map(|(i, c)| {
                c.proc
                    .as_ref()
                    .map(|_| (coder_role(i), "haiku".to_string()))
            }))
            .collect();
        let child_pids: BTreeMap<String, u32> = self
            .proc
            .iter()
            .map(|p| (handoff::MAIN_ROLE.to_string(), p.pid()))
            .chain(
                self.prompter_proc
                    .iter()
                    .map(|p| ("prompter".to_string(), p.pid())),
            )
            .chain(self.tui_proc.iter().map(|p| ("tui".to_string(), p.pid())))
            .chain(
                self.coders
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.proc.as_ref().map(|p| (coder_role(i), p.pid()))),
            )
            .collect();
        ExitHandoff {
            cwd: self
                .opts
                .cwd
                .as_deref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            main_session: self.sid.clone(),
            resident_sessions,
            owned_sessions: self.owned.clone(),
            ctx_tokens,
            owner_pid: std::process::id(),
            child_pids,
            child_models,
            ..Default::default()
        }
    }

    // The agent's SubagentHandback message (its final report) is kept for the done line. Both the current child and a
    // drained child can carry it (a rotation may leave the agent on the drained child).
    pub(super) fn record_handback(&mut self, raw: &Value) {
        let Some(msg) = handback_message(raw) else {
            return;
        };
        let Some(p) = raw
            .get("parent_tool_use_id")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
        else {
            return;
        };
        if let Some(info) = self.agent_info.get_mut(p) {
            info.handback = msg;
        }
    }

    // Go agentTracker persist: writes one delegated agent to the agents file (no-op without one).
    pub(super) fn persist_agent(&self, id: &str, status: &str) {
        let (Some(store), Some(info)) = (&self.agent_store, self.agent_info.get(id)) else {
            return;
        };
        let status = saved_status(status, info.failed);
        let mut s = SavedAgent::new(
            id,
            status,
            UNIX_EPOCH + Duration::from_secs(info.started_at),
        );
        // The agent's own model (its stream, else its input or type prefix); never the main session model.
        s.model = info.model.clone();
        s.name = info.session_name.clone();
        s.task = info.task.clone();
        s.prompt = info.prompt.clone();
        // Go persistLocked: the summary the restored agent view shows is the final report, else the last text line.
        s.summary = if info.handback.is_empty() {
            info.last_text.clone()
        } else {
            info.handback.clone()
        };
        s.background = info.background;
        s.agent_id = info.agent_id.clone();
        s.log = info.log.clone();
        // The session the agent runs in: /resume of that session lists it (AgentStore::load matches parent).
        s.parent = self.sid.clone();
        store.put(&[s]);
    }

    // The task list of this session: notes/tmp/long/tasks-<session>.json, read back on /resume.
    pub(super) fn persist_tasks(&self) {
        if self.sid.is_empty() {
            return;
        }
        let dir = crate::tmp_layout::long_dir(&self.engine_cwd());
        let rows: Vec<Value> = self
            .queued_tasks
            .iter()
            .map(|t| {
                json!({"id": t.id, "desc": t.desc, "status": task_status_name(t.status),
                    "tokens": t.tokens, "duration_ms": t.duration_ms})
            })
            .collect();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(
            dir.join(format!("tasks-{}.json", self.sid)),
            Value::Array(rows).to_string(),
        );
    }

    // /resume: the saved task list is shown again. Restored rows are display only, never pending.
    pub(super) fn load_tasks(&mut self, sid: &str) {
        if sid.is_empty()
            || sid
                .chars()
                .any(|c| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        {
            return;
        }
        let path =
            crate::tmp_layout::long_dir(&self.engine_cwd()).join(format!("tasks-{sid}.json"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };
        let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(&text) else {
            return;
        };
        for r in rows {
            let id = r["id"].as_str().unwrap_or_default().to_string();
            let desc = r["desc"].as_str().unwrap_or_default().to_string();
            let status = task_status_from_name(r["status"].as_str().unwrap_or_default());
            self.emit(UiEvent::TaskQueued {
                id: id.clone(),
                desc,
            });
            self.emit(UiEvent::TaskResolved {
                id: id.clone(),
                status,
            });
            if let Some(s) = saved_stats(&r) {
                self.emit(UiEvent::TaskStats {
                    id,
                    tokens: s.tokens,
                    duration_ms: s.duration_ms,
                });
            }
        }
    }

    pub(super) fn engine_cwd(&self) -> PathBuf {
        resolve_cwd(self.opts.cwd.clone())
    }

    // Record root: Config.record_dir when set (tests and the CLI override), else Go recordDir for cwd.
    pub(super) fn record_root(&self, cwd: &Path) -> PathBuf {
        self.cfg
            .record_dir
            .clone()
            .unwrap_or_else(|| record::dir_for(cwd))
    }

    pub(super) fn record_turn_save(&mut self, duration_ms: u64) {
        let cost_turn = (self.cost_usd - self.turn_cost_base).max(0.0);
        let mut fields = serde_json::Map::new();
        fields.insert("duration_ms".into(), json!(duration_ms));
        fields.insert("cost_turn_usd".into(), json!(cost_turn));
        fields.insert("cost_usd".into(), json!(self.cost_usd));
        fields.insert("ctx".into(), json!(self.ctx_tokens));
        fields.insert("delegations".into(), json!(self.delegations));
        self.record.last_measure = record_turn::measure_line(&fields);
        self.record.last_reply = self.turn_reply.clone();
        let Some(stamp) = local_stamp() else {
            return;
        };
        let cwd = self.engine_cwd();
        let root = self.record_root(&cwd);
        if let Err(e) = self.record.save(&root, &cwd, &self.turn_prompt, &stamp) {
            self.emit(UiEvent::Error(format!("record save failed: {e}")));
        }
    }

    // (input, output, cache) of the running turn: the result usage, else the sum of main message usage. None when zero.
    pub(super) fn turn_token_counts(&self) -> Option<(u64, u64, u64)> {
        if let Some(c) = self.result_usage.as_ref().and_then(Usage::turn_counts) {
            return Some(c);
        }
        sum_turn_counts(self.turn_msg_usage.values())
    }

    pub(super) fn log_turn(&mut self, duration_ms: u64, source: Option<&str>) {
        let end_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let cost_turn = (self.cost_usd - self.turn_cost_base).max(0.0);
        let mut row = json!({
            "stage": "turn",
            "start_ms": end_ms.saturating_sub(duration_ms),
            "end_ms": end_ms,
            "model": self.model,
            "ctx": self.ctx_tokens,
            "session": self.sid,
            "cost_usd": self.cost_usd,
            "cost_turn_usd": cost_turn,
            "duration_ms": duration_ms,
            "delegations": self.delegations,
        });
        if let Some(source) = source {
            row["source"] = json!(source);
        }
        // Go repl.go: result usage is summed over the turn's API calls; keys are written even when absent (zeros).
        let ru = serde_json::to_value(self.result_usage.take().unwrap_or_default())
            .unwrap_or(Value::Null);
        let usage = usage_fields(&ru);
        if let Some(fields) = usage.as_object() {
            for (k, v) in fields {
                row[format!("result_{k}")] = v.clone();
            }
        }
        if let Some(mu) = self.result_model_usage.take()
            && mu.as_object().is_some_and(|o| !o.is_empty())
        {
            row["result_model_usage"] = mu;
        }
        let _ = self.metrics.log(with_spend(row, &self.spend));
    }

    // Rotation with live agents (the previous main launched them): each live agent's full brief, and the previous
    // main's turns from the first live launch on, newest kept within CARRIED_TURN_CHARS. Empty when none is live.
    // Called before the old child settles its agents, so a killed child's agents are still listed.
    pub(super) fn carried_block(&self) -> String {
        let mut live: Vec<(&String, &AgentInfo)> = self
            .agents
            .keys()
            .filter_map(|id| self.agent_info.get(id).map(|i| (id, i)))
            .filter(|(_, i)| i.launch_epoch == self.epoch)
            .collect();
        if live.is_empty() {
            return String::new();
        }
        live.sort_by_key(|(_, i)| i.launch_seq);
        let mut block =
            String::from("## Agents still running from the previous session (launched by it)\n");
        for (id, i) in &live {
            let task = if i.task.is_empty() { &i.name } else { &i.task };
            block.push_str(&format!(
                "- {id} · {task} · launched at {} (unix secs)\n  brief:\n{}\n",
                i.started_at,
                cap_brief(&i.brief)
            ));
        }
        let turns = self.carried_turns(live[0].1.launch_seq);
        if !turns.is_empty() {
            block.push_str(
                "\n## Previous session turns (from the first live agent's launch, newest kept)\n",
            );
            block.push_str(&turns);
        }
        block
    }

    // The logged turns from turn number `from` on, as "User:" and "Assistant:" text, newest kept within
    // CARRIED_TURN_CHARS. The oldest included turn keeps its tail. Tool results are not in the log.
    pub(super) fn carried_turns(&self, from: u64) -> String {
        let mut left = CARRIED_TURN_CHARS;
        let mut parts: Vec<String> = Vec::new();
        for (seq, prompt, reply) in self.turn_log.iter().rev() {
            if *seq < from || left == 0 {
                break;
            }
            let text = tail(&format!("User: {prompt}\nAssistant: {reply}"), left);
            left -= text.chars().count();
            parts.push(text);
        }
        parts.reverse();
        parts.join("\n\n")
    }

    // Go buildSnapshot: preamble, handoff note, changed files, and the last turns, capped at snapshot_chars.
    pub(super) fn build_snapshot(&self, note: &str) -> String {
        let note = if note.is_empty() { "(empty)" } else { note };
        // Each header and its body are separate parts, joined with a blank line (Go buildSnapshot).
        let mut parts = vec![
            PREAMBLE.to_string(),
            "## Handoff note (written by previous session)".to_string(),
            note.to_string(),
        ];
        // Go bgHandoffLines: background agents still running or with a written result, oldest first.
        let bg: Vec<rotate_defer::BgAgent> = self
            .agent_info
            .iter()
            .map(|(id, i)| rotate_defer::BgAgent {
                id: id.as_str(),
                name: &i.name,
                task: "",
                started: i.started,
                background: i.background,
                done: i.done,
                result_done: i.result_done,
                // result_path is never set at launch; out_path is the file the prompt asked for.
                result_path: if i.result_path.is_empty() {
                    &i.out_path
                } else {
                    &i.result_path
                },
            })
            .collect();
        let lines = rotate_defer::bg_handoff_lines(&bg);
        if !lines.is_empty() {
            parts.push("## Background agents (running)".to_string());
            parts.push(lines.join("\n"));
        }
        if !self.files.is_empty() {
            let list: Vec<String> = self.files.iter().map(|f| format!("- {f}")).collect();
            parts.push("## Files changed".to_string());
            parts.push(list.join("\n"));
        }
        if !self.recent.is_empty() {
            let mut turns = vec!["## Last turns (verbatim, truncated)".to_string()];
            for (user, assistant) in &self.recent {
                turns.push(format!(
                    "User: {}\nAssistant: {}",
                    clip(user, TURN_CHARS),
                    clip(assistant, TURN_CHARS)
                ));
            }
            parts.push(turns.join("\n\n"));
        }
        clip(&parts.join("\n\n"), self.cfg.snapshot_chars)
    }
}
