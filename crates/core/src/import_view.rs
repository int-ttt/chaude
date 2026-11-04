// /import view (Go importview.go): the whole imported transcript for display: user and assistant text,
// thinking, tool calls and results, and subagent (sidechain) messages placed right after the Agent call that
// spawned them. The view is display-only; the snapshot input (ClaudeImport.turns) stays text-only.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

// Limits for the /import view. Long tool results are cut with an explicit "(+N ...)" marker, never silently.
pub const VIEW_RESULT_LINES: usize = 20;
pub const VIEW_RESULT_CHARS: usize = 4000;
pub const VIEW_INPUT_CHARS: usize = 300;

// (user text, assistant text) for one turn.
pub type Turn = (String, String);

// One transcript line as a JSON object. Anything else is kept as {"type":"_unparsed","raw":line} (Go parseEvent).
pub fn parse_event(line: &str) -> Value {
    match serde_json::from_str::<Value>(line) {
        Ok(v) if v.is_object() => v,
        _ => serde_json::json!({"type": "_unparsed", "raw": line}),
    }
}

// Go str(): the string field, "" when absent or not a string.
pub(crate) fn sget<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

// Go bool field: false when absent.
pub(crate) fn bget(v: &Value, key: &str) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(false)
}

// Go asList: the array items, empty for anything else.
pub(crate) fn list(v: Option<&Value>) -> &[Value] {
    match v {
        Some(Value::Array(a)) => a.as_slice(),
        _ => &[],
    }
}

// Go resultText: a string as is, or the "text" fields of a block list joined by newlines.
pub(crate) fn result_text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| sget(x, "text"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

// Go jsonStr: compact JSON with keys sorted and no HTML escaping.
pub(crate) fn json_str(v: &Value) -> String {
    serde_json::to_string(&sorted(v)).unwrap_or_default()
}

fn sorted(v: &Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut e: Vec<(&String, &Value)> = m.iter().collect();
            e.sort_by(|a, b| a.0.cmp(b.0));
            Value::Object(e.into_iter().map(|(k, v)| (k.clone(), sorted(v))).collect())
        }
        Value::Array(a) => Value::Array(a.iter().map(sorted).collect()),
        _ => v.clone(),
    }
}

// Go trunc: the first n runes.
pub(crate) fn trunc(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

// Go clipLines: keeps the first VIEW_RESULT_LINES lines, then clip()s the result.
pub fn clip_lines(body: &str) -> String {
    let lines: Vec<&str> = body.split('\n').collect();
    let joined = if lines.len() > VIEW_RESULT_LINES {
        let extra = lines.len() - VIEW_RESULT_LINES;
        let mut v: Vec<String> = lines[..VIEW_RESULT_LINES]
            .iter()
            .map(|s| s.to_string())
            .collect();
        v.push(format!("… (+{extra} lines)"));
        v.join("\n")
    } else {
        body.to_string()
    };
    clip(&joined)
}

// Go clip: cuts s to VIEW_RESULT_CHARS runes and says how many were dropped.
pub fn clip(s: &str) -> String {
    let n = s.chars().count();
    if n <= VIEW_RESULT_CHARS {
        return s.to_string();
    }
    let head = trunc(s, VIEW_RESULT_CHARS);
    format!("{head} … (+{} chars)", n - VIEW_RESULT_CHARS)
}

// Builds the view turns of one transcript. Subagent transcripts are rendered inline (see subagent).
#[derive(Default)]
pub struct ViewBuilder {
    turns: Vec<Turn>,
    user: String,
    asst: Vec<String>,
    // Agent tool_use id -> subagent transcript path (from subagents/*.meta.json).
    subs: BTreeMap<String, PathBuf>,
    sub_done: BTreeSet<PathBuf>,
    // uuids of sidechain events stored inside the main file (avoid rendering them twice).
    inline_uuid: BTreeSet<String>,
    // Subagent transcripts rendered so far.
    pub n_sub: usize,
}

impl ViewBuilder {
    // Maps Agent tool_use ids to <session>/subagents/agent-*.jsonl through their meta.json files.
    pub fn new(path: &Path) -> Self {
        let mut vb = ViewBuilder::default();
        let s = path.to_string_lossy();
        let base = s.strip_suffix(".jsonl").unwrap_or(&s);
        let dir = PathBuf::from(base).join("subagents");
        let mut metas: Vec<PathBuf> = fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().ends_with(".meta.json"))
            .collect();
        metas.sort();
        for mp in metas {
            let Ok(text) = fs::read_to_string(&mp) else {
                continue;
            };
            let Ok(meta) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let id = sget(&meta, "toolUseId");
            if id.is_empty() {
                continue;
            }
            let ms = mp.to_string_lossy();
            let stem = ms.strip_suffix(".meta.json").unwrap_or(&ms);
            vb.subs
                .insert(id.to_string(), PathBuf::from(format!("{stem}.jsonl")));
        }
        vb
    }

    // Renders one main-file event. Meta (injected context) and non user/assistant events are skipped.
    pub fn consume(&mut self, ev: &Value) {
        let t = sget(ev, "type");
        if (t != "user" && t != "assistant") || bget(ev, "isMeta") {
            return;
        }
        let side = bget(ev, "isSidechain");
        let prefix = if side {
            self.inline_uuid.insert(sget(ev, "uuid").to_string());
            "┆ "
        } else {
            ""
        };
        let msg = ev.get("message").unwrap_or(&Value::Null);
        self.event(t, msg, side, prefix);
    }

    fn event(&mut self, role: &str, msg: &Value, side: bool, prefix: &str) {
        match msg.get("content") {
            Some(Value::String(c)) => self.text(role, c, side, prefix),
            Some(Value::Array(blocks)) => {
                for b in blocks {
                    match sget(b, "type") {
                        "text" => self.text(role, sget(b, "text"), side, prefix),
                        "thinking" => {
                            let tx = sget(b, "thinking").trim(); // signature-only blocks have no text
                            if !tx.is_empty() {
                                let body = clip(tx);
                                self.lines(
                                    &format!("{prefix}[thinking] "),
                                    &body,
                                    &format!("{prefix}  "),
                                );
                            }
                        }
                        "tool_use" => {
                            let input = b.get("input").unwrap_or(&Value::Null);
                            self.add(format!(
                                "{prefix}▸ {} {}",
                                sget(b, "name"),
                                trunc(&json_str(input), VIEW_INPUT_CHARS)
                            ));
                            if let Some(p) = self
                                .subs
                                .get(sget(b, "id"))
                                .cloned()
                                .filter(|p| !self.sub_done.contains(p))
                            {
                                self.subagent(&p);
                            }
                        }
                        "tool_result" => self.result(b, prefix),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    // A main-file user prompt starts a new turn; other user text is a note; assistant text is appended.
    fn text(&mut self, role: &str, s: &str, side: bool, prefix: &str) {
        let tx = s.trim();
        if tx.is_empty() {
            return;
        }
        if role == "user" && !side && !tx.starts_with('<') {
            self.flush();
            self.user = tx.to_string();
        } else if role == "user" && side {
            self.lines(&format!("{prefix}task: "), tx, &format!("{prefix}  "));
        } else if role == "user" {
            self.lines(&format!("{prefix}· "), tx, &format!("{prefix}  "));
        } else {
            self.lines(prefix, tx, prefix);
        }
    }

    fn result(&mut self, b: &Value, prefix: &str) {
        let mark = if b.get("is_error").and_then(Value::as_bool) == Some(true) {
            "← error: "
        } else {
            "← "
        };
        let text = result_text(b.get("content"));
        let body = text.trim();
        if body.is_empty() {
            self.add(format!("{prefix}{mark}(empty)"));
            return;
        }
        self.lines(
            &format!("{prefix}{mark}"),
            &clip_lines(body),
            &format!("{prefix}  "),
        );
    }

    // Renders one subagent transcript with the sidechain prefix. A file whose first event is already stored
    // inside the main transcript is skipped, so its messages do not appear twice.
    fn subagent(&mut self, path: &Path) {
        self.sub_done.insert(path.to_path_buf());
        let Ok(f) = File::open(path) else {
            return;
        };
        let mut rd = BufReader::with_capacity(1 << 20, f);
        let mut buf = Vec::new();
        let mut first = true;
        loop {
            buf.clear();
            match rd.read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let line = String::from_utf8_lossy(&buf);
            if line.trim().is_empty() {
                continue;
            }
            let ev = parse_event(&line);
            if first && self.inline_uuid.contains(sget(&ev, "uuid")) {
                return;
            }
            first = false;
            self.consume(&ev);
        }
        self.n_sub += 1;
    }

    // Renders subagents never linked to a tool call (background runs), flushes the open turn, and returns the view.
    pub fn finish(&mut self) -> Vec<Turn> {
        let mut rest: Vec<PathBuf> = self
            .subs
            .values()
            .filter(|p| !self.sub_done.contains(*p))
            .cloned()
            .collect();
        rest.sort();
        rest.dedup();
        for p in rest {
            self.subagent(&p);
        }
        self.flush();
        std::mem::take(&mut self.turns)
    }

    fn flush(&mut self) {
        if self.user.is_empty() && self.asst.is_empty() {
            return;
        }
        let u = if self.user.is_empty() {
            "(session continued)".to_string()
        } else {
            std::mem::take(&mut self.user)
        };
        self.turns.push((u, self.asst.join("\n")));
        self.asst.clear();
    }

    fn add(&mut self, s: String) {
        self.asst.push(s.trim_end_matches(' ').to_string());
    }

    // Appends text line by line: the first line gets first, the others rest.
    fn lines(&mut self, first: &str, text: &str, rest: &str) {
        for (i, l) in text.split('\n').enumerate() {
            let p = if i == 0 { first } else { rest };
            self.add(format!("{p}{l}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use serde_json::json;

    fn view_of(lines: &[Value], path: &Path) -> Vec<Turn> {
        let mut vb = ViewBuilder::new(path);
        for ev in lines {
            vb.consume(ev);
        }
        vb.finish()
    }

    #[test]
    fn unparsable_line_becomes_unparsed_event() {
        let ev = parse_event("not json");
        assert_eq!(sget(&ev, "type"), "_unparsed");
        assert_eq!(sget(&ev, "raw"), "not json");
        assert_eq!(sget(&parse_event("[1]"), "type"), "_unparsed");
    }

    #[test]
    fn json_str_sorts_keys_and_skips_html_escape() {
        let v = json!({"b": "<x>&", "a": [1, {"d": 1, "c": 2}]});
        assert_eq!(json_str(&v), r#"{"a":[1,{"c":2,"d":1}],"b":"<x>&"}"#);
    }

    #[test]
    fn meta_and_non_message_events_are_skipped() {
        let tmp = TempDir::new("view-meta");
        let p = tmp.path().join("s.jsonl");
        let turns = view_of(
            &[
                json!({"type": "user", "isMeta": true, "message": {"content": "injected"}}),
                json!({"type": "summary", "summary": "x"}),
                json!({"type": "user", "message": {"content": "hello"}}),
                json!({"type": "assistant", "message": {"content": [{"type": "text", "text": "hi"}]}}),
            ],
            &p,
        );
        assert_eq!(turns, vec![("hello".to_string(), "hi".to_string())]);
    }

    #[test]
    fn tool_use_result_and_thinking_are_rendered_in_order() {
        let tmp = TempDir::new("view-tools");
        let p = tmp.path().join("s.jsonl");
        let turns = view_of(
            &[
                json!({"type": "user", "message": {"content": "scan"}}),
                json!({"type": "assistant", "message": {"content": [
                    {"type": "thinking", "thinking": "plan it", "signature": "x"},
                    {"type": "tool_use", "id": "t1", "name": "Bash", "input": {"command": "ls", "b": 1}}
                ]}}),
                json!({"type": "user", "message": {"content": [
                    {"type": "tool_result", "tool_use_id": "t1", "is_error": true, "content": "boom"}
                ]}}),
            ],
            &p,
        );
        assert_eq!(turns.len(), 1);
        assert_eq!(
            turns[0].1,
            "[thinking] plan it\n▸ Bash {\"b\":1,\"command\":\"ls\"}\n← error: boom"
        );
    }

    #[test]
    fn long_result_is_cut_with_line_and_char_markers() {
        let body: String = (0..25).map(|i| format!("l{i}\n")).collect();
        let cut = clip_lines(body.trim_end());
        assert!(cut.ends_with("… (+5 lines)"), "{cut}");
        assert_eq!(cut.lines().count(), VIEW_RESULT_LINES + 1);

        let wide = "é".repeat(VIEW_RESULT_CHARS + 3);
        let c = clip(&wide);
        assert!(c.ends_with(" … (+3 chars)"), "{c}");
        assert_eq!(
            c.chars().count(),
            VIEW_RESULT_CHARS + " … (+3 chars)".chars().count()
        );
    }

    // Port of the Go view test: a subagent linked by tool_use id renders right after the Agent call.
    #[test]
    fn subagent_renders_after_its_agent_call() {
        let tmp = TempDir::new("view-sub");
        let session = tmp.path().join("s");
        let sub_dir = session.join("subagents");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(
            sub_dir.join("agent-a.meta.json"),
            r#"{"toolUseId":"toolu_1"}"#,
        )
        .unwrap();
        fs::write(
            sub_dir.join("agent-a.jsonl"),
            concat!(
                r#"{"type":"user","isSidechain":true,"uuid":"s1","message":{"content":"find files"}}"#,
                "\n",
                r#"{"type":"assistant","isSidechain":true,"uuid":"s2","message":{"content":[{"type":"text","text":"scan done"}]}}"#,
                "\n"
            ),
        )
        .unwrap();
        let p = tmp.path().join("s.jsonl");
        let mut vb = ViewBuilder::new(&p);
        vb.consume(&json!({"type": "user", "uuid": "m0", "message": {"content": "go"}}));
        vb.consume(
            &json!({"type": "assistant", "uuid": "m1", "message": {"content": [
                {"type": "tool_use", "id": "toolu_1", "name": "Agent", "input": {"prompt": "x"}},
                {"type": "tool_result", "content": "ok"}
            ]}}),
        );
        let turns = vb.finish();
        assert_eq!(vb.n_sub, 1);
        assert_eq!(turns.len(), 1);
        let text = &turns[0].1;
        let agent = text.find("▸ Agent").expect("agent call");
        let task = text.find("┆ task: find files").expect("subagent task");
        let done = text.find("┆ scan done").expect("subagent reply");
        assert!(agent < task && task < done, "{text}");
        assert!(!text.contains("subagents"), "no meta leakage: {text}");
    }
}
