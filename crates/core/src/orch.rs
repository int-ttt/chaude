//! Orchestration modes for the A/B test (Go orch.go, orchws.go): mode names, the default main model, the
//! system-prompt rule per mode, and mode f workspace routing. Pure state: the actor applies the routes.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::error::Error;

// Go applyOrchEnv: the caller exports the mode under this name for child processes and hooks.
pub const ENV_KEY: &str = "CHAUDE_ORCH";

// Go parseOrch: normalizes an --orch value; empty means the baseline m0.
pub fn parse(s: &str) -> Result<&'static str, Error> {
    match s.trim().to_lowercase().as_str() {
        "" | "m0" => Ok("m0"),
        "b" => Ok("b"),
        "c" => Ok("c"),
        "d" => Ok("d"),
        "e" => Ok("e"),
        "f" => Ok("f"),
        _ => Err(Error::Msg(format!(
            "unknown orch mode {s:?} (m0|b|c|d|e|f)"
        ))),
    }
}

// Go orchDefaultModel: the main model a mode expects. Empty keeps the --model default.
pub fn default_model(mode: &str) -> &'static str {
    match mode {
        "b" | "c" | "d" | "f" => "opus",
        "e" => "haiku",
        _ => "",
    }
}

const RULE_B: &str = "[orch B] Opus main decides inline and delegates directly to haiku-*. ALL file reads/writes go through haiku-scout/haiku-coder/haiku-batch-worker; no relay hop. One haiku-scout call per decision (batch all reads), RETURN cap 4000 chars. Merge would-be short spawns (<=3 msgs) into one brief. Reuse the same haiku-coder instance per workstream.";

const RULE_C_TAIL: &str = " [orch C] haiku-scout returns a structured digest (key: value, <=4000 chars); decide from the digest. After Haiku fails twice, Opus main decides itself.";

const RULE_D: &str = "[orch D] Write one task brief, then issue ALL Haiku calls (scout reads, coder edits on disjoint files, checker) in ONE message in parallel. No intermediate agent, no sequential ping-pong. Coder briefs must name disjoint file sets; sum of RETURN caps <= 8000 chars per round; max 2 rounds per task; second round only on FAIL.";

const RULE_E: &str = "[orch E] Haiku main does all work itself; no Opus agents except escalation after 2 Haiku failures (put ESCALATE in that brief). Spawning a reader is allowed only if the brief names absolute file paths and declares 'RETURN: <=N chars' with N <= 0.25 x total bytes of those files; otherwise the hook blocks it and you do the work inline with your own tools.";

const RULE_F: &str = "[orch F] Turns prefixed [ws:<id>] belong to one workstream; the harness routes them to the same live Haiku child, rotating at 40k tokens, 20 turns or idle >3570s with a <=1500-char handoff. Keep one workstream id per task.";

// Go orchRule: the system-prompt rule for a mode. Empty for m0 and unknown modes. C extends B.
pub fn rule(mode: &str) -> String {
    match mode {
        "b" => RULE_B.to_string(),
        "c" => format!("{RULE_B}{RULE_C_TAIL}"),
        "d" => RULE_D.to_string(),
        "e" => RULE_E.to_string(),
        "f" => RULE_F.to_string(),
        _ => String::new(),
    }
}

// Go wsRe limits: a workspace id is 1-32 of [A-Za-z0-9_-]; a session rotates at 20 turns, 40k context or 3570s idle.
pub const WS_MAX_TURNS: u32 = 20;
pub const WS_MAX_CTX: u64 = 40_000;
pub const WS_MAX_IDLE: Duration = Duration::from_secs(3570);
pub const WS_HANDOFF_MAX: usize = 1500;

// Go parseWS: a leading "[ws:<id>]" tag. Returns the id and the rest of the line after the tag and its
// trailing whitespace; None when the tag is missing or the id is not valid.
pub fn parse_ws(line: &str) -> Option<(&str, &str)> {
    let body = line.strip_prefix("[ws:")?;
    let end = body.find(']')?;
    let id = &body[..end];
    let valid = !id.is_empty()
        && id.len() <= 32
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if !valid {
        return None;
    }
    Some((
        id,
        body[end + 1..].trim_start_matches(|c: char| c.is_ascii_whitespace()),
    ))
}

// Go wsState: the per-workspace Haiku session record kept by mode f.
#[derive(Debug, Clone, PartialEq)]
pub struct WsState {
    pub session_id: String,
    pub turns: u32,
    pub last_used: Instant,
    pub ctx: u64,
    pub last_text: String,
}

impl WsState {
    pub fn new(now: Instant) -> Self {
        WsState {
            session_id: String::new(),
            turns: 0,
            last_used: now,
            ctx: 0,
            last_text: String::new(),
        }
    }
}

// Go wsNeedsRotate: a resumable session (non-empty session id) must be replaced by a fresh one.
pub fn needs_rotate(st: Option<&WsState>, now: Instant) -> bool {
    match st {
        Some(s) if !s.session_id.is_empty() => {
            s.turns >= WS_MAX_TURNS
                || s.ctx >= WS_MAX_CTX
                || now.saturating_duration_since(s.last_used) > WS_MAX_IDLE
        }
        _ => false,
    }
}

// Go wsHandoff: the last reply of a rotated workspace, carried into its fresh session (cap in chars, prefix included).
pub fn handoff_text(id: &str, st: Option<&WsState>) -> String {
    let text = st.map_or("", |s| s.last_text.as_str());
    format!("[ws:{id} handoff] {text}")
        .chars()
        .take(WS_HANDOFF_MAX)
        .collect()
}

// Where a routed line goes. The caller applies it; the router only decides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Mode is not f: the line goes to the current child unchanged.
    Current,
    /// Untagged line in f: the main model takes it. Switch to main_model() first when the active model differs.
    Main,
    /// Tagged line whose workspace session is still live: resume session_id on haiku.
    Resume { id: String, session_id: String },
    /// Tagged line that needs a fresh haiku session for workspace id (new, or rotated).
    Fresh { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routed {
    pub target: Target,
    // The text to send: the line without its tag, prefixed by the handoff when a rotated session had a reply.
    pub text: String,
}

// Go Repl orch state for mode f: main model, workspace sessions, and the workspace of the current turn.
#[derive(Debug, Default)]
pub struct Router {
    enabled: bool,
    main: Option<String>,
    ws: HashMap<String, WsState>,
    cur: Option<String>,
}

impl Router {
    // Only mode f routes; other modes keep every line on the current child.
    pub fn new(mode: &str) -> Self {
        Router {
            enabled: mode == "f",
            ..Router::default()
        }
    }

    // The main model: the configured model at the first routed line (Go orchMain).
    pub fn main_model(&self) -> Option<&str> {
        self.main.as_deref()
    }

    // Go routeOrch. model is the configured --model; now is the time of the line.
    pub fn route(&mut self, line: &str, model: &str, now: Instant) -> Routed {
        if !self.enabled {
            return Routed {
                target: Target::Current,
                text: line.to_string(),
            };
        }
        if self.main.is_none() {
            self.main = Some(model.to_string());
        }
        let Some((id, rest)) = parse_ws(line) else {
            self.cur = None;
            return Routed {
                target: Target::Main,
                text: line.to_string(),
            };
        };
        let id = id.to_string();
        self.cur = Some(id.clone());
        let prev = self.ws.get(&id);
        if let Some(st) = prev.filter(|s| !s.session_id.is_empty() && !needs_rotate(Some(*s), now))
        {
            let target = Target::Resume {
                id,
                session_id: st.session_id.clone(),
            };
            return Routed {
                target,
                text: rest.to_string(),
            };
        }
        let handoff = prev
            .filter(|s| !s.session_id.is_empty())
            .map(|s| handoff_text(&id, Some(s)));
        let text = match handoff {
            Some(h) => format!("{h}\n\n{rest}"),
            None => rest.to_string(),
        };
        self.ws.insert(id.clone(), WsState::new(now));
        Routed {
            target: Target::Fresh { id },
            text,
        }
    }

    // Go wsRecord: stores the finished turn for the current workspace. session_id is None when no session is
    // live (the stored id stays); ctx is the last context size.
    pub fn record(&mut self, reply: &str, session_id: Option<&str>, ctx: u64, now: Instant) {
        if !self.enabled {
            return;
        }
        let Some(id) = self.cur.clone() else {
            return;
        };
        let st = self.ws.entry(id).or_insert_with(|| WsState::new(now));
        if let Some(sid) = session_id {
            st.session_id = sid.to_string();
        }
        st.turns += 1;
        st.last_used = now;
        st.ctx = ctx;
        st.last_text = reply.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mode_normalizes_and_rejects() {
        assert_eq!(parse("").ok(), Some("m0"));
        assert_eq!(parse("  ").ok(), Some("m0"));
        assert_eq!(parse("m0").ok(), Some("m0"));
        assert_eq!(parse("B").ok(), Some("b"));
        assert_eq!(parse(" c ").ok(), Some("c"));
        assert_eq!(parse("F").ok(), Some("f"));
        assert!(parse("x").is_err());
        assert!(parse("m1").is_err());
    }

    #[test]
    fn default_model_per_mode() {
        let cases = [
            ("m0", ""),
            ("b", "opus"),
            ("c", "opus"),
            ("d", "opus"),
            ("e", "haiku"),
            ("f", "opus"),
            ("", ""),
        ];
        for (mode, want) in cases {
            assert_eq!(default_model(mode), want, "mode {mode:?}");
        }
    }

    #[test]
    fn rule_per_mode() {
        assert_eq!(rule("m0"), "");
        for mode in ["b", "c", "d", "e", "f"] {
            assert!(!rule(mode).is_empty(), "mode {mode}");
        }
        assert!(rule("c").starts_with(RULE_B));
        for mode in ["b", "c", "d", "f"] {
            assert!(
                !rule(mode).contains("opus-architect"),
                "mode {mode} names opus-architect"
            );
        }
    }

    #[test]
    fn parse_ws_cases() {
        assert_eq!(
            parse_ws("[ws:abc-1_X] hello there"),
            Some(("abc-1_X", "hello there"))
        );
        assert_eq!(parse_ws("plain line"), None);
        assert_eq!(parse_ws("[ws:]x"), None);
        assert_eq!(parse_ws(&format!("[ws:{}] x", "a".repeat(33))), None);
        assert_eq!(
            parse_ws(&format!("[ws:{}] x", "a".repeat(32))).map(|p| p.0.len()),
            Some(32)
        );
        assert_eq!(parse_ws("[ws:bad.id] x"), None);
        assert_eq!(parse_ws("x [ws:abc] y"), None);
    }

    #[test]
    fn needs_rotate_cases() {
        let now = Instant::now();
        assert!(!needs_rotate(None, now));
        let mut st = WsState::new(now);
        st.turns = 99;
        assert!(
            !needs_rotate(Some(&st), now),
            "state without session id rotates"
        );
        st.session_id = "s".into();
        st.turns = 19;
        assert!(!needs_rotate(Some(&st), now));
        st.turns = 20;
        assert!(needs_rotate(Some(&st), now));
        st.turns = 0;
        st.ctx = 39_999;
        assert!(!needs_rotate(Some(&st), now));
        st.ctx = 40_000;
        assert!(needs_rotate(Some(&st), now));
        st.ctx = 0;
        assert!(!needs_rotate(Some(&st), now + Duration::from_secs(3570)));
        assert!(needs_rotate(Some(&st), now + Duration::from_secs(3571)));
        assert_eq!(WS_MAX_IDLE, Duration::from_secs(3570));
    }

    #[test]
    fn handoff_is_rune_capped_with_prefix() {
        let long = "한글".repeat(2500);
        let h = handoff_text(
            "w1",
            Some(&WsState {
                last_text: long,
                ..WsState::new(Instant::now())
            }),
        );
        assert_eq!(h.chars().count(), WS_HANDOFF_MAX);
        assert!(h.starts_with("[ws:w1 handoff] "));
        let short = WsState {
            last_text: "짧음".into(),
            ..WsState::new(Instant::now())
        };
        assert_eq!(handoff_text("w1", Some(&short)), "[ws:w1 handoff] 짧음");
        assert_eq!(handoff_text("w1", None), "[ws:w1 handoff] ");
    }

    #[test]
    fn route_non_f_mode_keeps_line() {
        let mut r = Router::new("b");
        let line = "[ws:x] 파일 읽어줘";
        let got = r.route(line, "opus", Instant::now());
        assert_eq!(
            got,
            Routed {
                target: Target::Current,
                text: line.into()
            }
        );
        assert_eq!(r.main_model(), None);
        r.record("reply", Some("s1"), 10, Instant::now());
        assert!(r.ws.is_empty());
    }

    #[test]
    fn route_f_mode_new_resume_rotate() {
        let t0 = Instant::now();
        let mut r = Router::new("f");
        let got = r.route("untagged", "opus", t0);
        assert_eq!(
            got,
            Routed {
                target: Target::Main,
                text: "untagged".into()
            }
        );
        assert_eq!(r.main_model(), Some("opus"));

        let got = r.route("[ws:w1] first", "opus", t0);
        assert_eq!(
            got,
            Routed {
                target: Target::Fresh { id: "w1".into() },
                text: "first".into()
            }
        );
        r.record("reply one", Some("sess-1"), 5, t0);

        let got = r.route("[ws:w1] second", "opus", t0);
        assert_eq!(
            got,
            Routed {
                target: Target::Resume {
                    id: "w1".into(),
                    session_id: "sess-1".into()
                },
                text: "second".into()
            }
        );
        r.record("reply two", None, 6, t0);
        assert_eq!(r.ws["w1"].session_id, "sess-1");
        assert_eq!(r.ws["w1"].turns, 2);

        // idle past 3570s: rotate, carrying the last reply as handoff
        let got = r.route("[ws:w1] third", "opus", t0 + Duration::from_secs(3600));
        assert_eq!(got.target, Target::Fresh { id: "w1".into() });
        assert_eq!(got.text, "[ws:w1 handoff] reply two\n\nthird");
        assert!(r.ws["w1"].session_id.is_empty());
    }
}
