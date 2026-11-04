//! Resident pipeline (Go resident.go): the prompter slot state machine and its reply rules.
//! The slots are parked child processes; this module holds the logic that does not need one.

use std::path::Path;
use std::time::Duration;

// The prompter's system prompt: the Go resident-prompter rules, copied so the engine does not depend on deprecated/.
pub const PROMPTER_RULE: &str = include_str!("rules/resident-prompter.md");

// How long the prompter may take to summarize (Go prompterTimeout). On timeout the input goes unchanged.
pub const PROMPTER_TIMEOUT: Duration = Duration::from_secs(60);

// How long the tui may take to translate the main reply (Go tuiTimeout). On timeout the main reply stays.
pub const TUI_TIMEOUT: Duration = Duration::from_secs(120);

// How long one coder task may run (Go residentCoderTimeout). On timeout the coder child is replaced.
pub const CODER_TIMEOUT: Duration = Duration::from_secs(30 * 60);

// Appended to every coder task so the reply ends with the lines summarize_reply keeps (Go coderReplyFormat).
pub const CODER_REPLY_FORMAT: &str = "\n\n[reply] End with `status: done|partial|fail` and one `changed: <path>: <what>` line per file you edited.";

// Appended to the main system prompt when the resident_coder tool is served (Go coderRule).
pub const CODER_RULE: &str = "For important files (main code, user documents) call the resident_coder tool: one resident haiku-coder, same session, long context. For lookups and small edits use Task haiku-scout / haiku-batch-worker.";

// Go summarizeReply: keeps the status and changed lines; a reply without them is cut to 300 chars.
pub fn summarize_reply(reply: &str) -> String {
    let keep: Vec<String> = reply
        .split('\n')
        .map(|ln| ln.trim().trim_start_matches("- ").trim())
        .filter(|t| t.starts_with("status:") || t.starts_with("changed:"))
        .map(str::to_string)
        .collect();
    if keep.is_empty() {
        return reply.trim().chars().take(300).collect();
    }
    keep.join("\n").chars().take(1200).collect()
}

// Go viaTUI prompt: the main return is the basis of the answer for the user.
pub fn tui_prompt(reply: &str) -> String {
    format!(
        "Opus main return follows. Use the relay field if present, else summary_ko (else the body), as the basis. Output only the answer for the user, in polite Korean (존댓말). Add no new facts.\n\n{reply}"
    )
}

// Go agentBody: the agent file under cwd, then under the home directory. Frontmatter is dropped.
pub fn agent_body(cwd: &Path, name: &str) -> Result<String, String> {
    let file = format!("{name}.md");
    let mut paths = vec![cwd.join(".claude").join("agents").join(&file)];
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(Path::new(&home).join(".claude").join("agents").join(&file));
    }
    for p in paths {
        if let Ok(s) = std::fs::read_to_string(&p) {
            let body = strip_frontmatter(&s);
            if !body.is_empty() {
                return Ok(body);
            }
        }
    }
    Err(format!("no agent file {name:?}"))
}

// Go stripFrontmatter: when the first line is `---`, drops the lines up to and including the second `---`.
pub fn strip_frontmatter(s: &str) -> String {
    let lines: Vec<&str> = s.split_inclusive('\n').collect();
    if lines.first().is_none_or(|l| l.trim() != "---") {
        return s.trim().to_string();
    }
    for i in 1..lines.len() {
        if lines[i].trim() == "---" {
            return lines[i + 1..].concat().trim().to_string();
        }
    }
    s.trim().to_string()
}

// Progress of one resident stage, shown as UiEvent::Stage. Failed carries the reason for the notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagePhase {
    Working,
    Done,
    Failed(String),
}

// What the main child receives for one input, and the stage to show (None: no stage line, Go viaPrompter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrompterOutcome {
    pub text: String,
    pub stage: Option<StagePhase>,
}

// The prompter slot: at most one summary is pending, holding the original input until its reply or timeout.
#[derive(Debug, Default)]
pub struct Prompter {
    pending: Option<String>,
}

impl Prompter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn waiting(&self) -> bool {
        self.pending.is_some()
    }

    // Go viaPrompter: starts a summary of input. Err while an earlier summary is still pending.
    pub fn start(&mut self, input: &str) -> Result<StagePhase, String> {
        if self.pending.is_some() {
            return Err("prompter busy".to_string());
        }
        self.pending = Some(input.to_string());
        Ok(StagePhase::Working)
    }

    // The prompter's reply for the pending input. None when nothing is pending.
    pub fn reply(&mut self, reply: &str) -> Option<PrompterOutcome> {
        let original = self.pending.take()?;
        if reply.is_empty() {
            return Some(failed(original, "prompter failed: empty reply"));
        }
        let (_, summary) = parse_prompter_reply(reply);
        if summary.is_empty() {
            return Some(PrompterOutcome {
                text: original,
                stage: None,
            });
        }
        Some(PrompterOutcome {
            text: format!("[prompter summary]\n{summary}\n\n[original]\n{original}"),
            stage: Some(StagePhase::Done),
        })
    }

    // The prompter failed for reason (child exited, could not spawn or write). None when nothing is pending.
    pub fn fail(&mut self, reason: &str) -> Option<PrompterOutcome> {
        let original = self.pending.take()?;
        Some(failed(original, reason))
    }

    // The wait exceeded PROMPTER_TIMEOUT. None when nothing is pending.
    pub fn timeout(&mut self) -> Option<PrompterOutcome> {
        let original = self.pending.take()?;
        Some(failed(
            original,
            &format!("prompter timed out after {}s", PROMPTER_TIMEOUT.as_secs()),
        ))
    }
}

fn failed(original: String, reason: &str) -> PrompterOutcome {
    PrompterOutcome {
        text: original,
        stage: Some(StagePhase::Failed(reason.to_string())),
    }
}

// Go parsePrompterReply: a first non-empty line NEED_ORIGINAL (flag set) or OK is dropped; any other reply is
// the summary whole. The summary is trimmed; empty when nothing is left.
pub fn parse_prompter_reply(reply: &str) -> (bool, String) {
    let s = reply.trim();
    let (head, rest) = s.split_once('\n').unwrap_or((s, ""));
    match head.trim() {
        "NEED_ORIGINAL" => (true, rest.trim().to_string()),
        "OK" => (false, rest.trim().to_string()),
        _ => (false, s.to_string()),
    }
}

const NEED_QUOTES: &[char] = &['`', '"', '“', '”', '「', '『'];
const NEED_WORDS: &[&str] = &[
    "아니",
    "근데",
    "말고",
    "정정",
    "다시 말",
    "번역",
    "translate",
    "문구",
    "표현",
    "말투",
    "어투",
    "wording",
    "문장",
    "그대로",
    "원문",
];
const PATH_EXTS: &[&str] = &[
    "go", "py", "md", "js", "ts", "tsx", "json", "yml", "yaml", "toml", "sh", "txt", "rs", "java",
    "c", "h", "cpp", "html", "css",
];

// Go needOriginal: true when input is too exact for a summary alone (quotes or code, many lines or long
// text, digits, file paths, a correction, or a translation or wording request). The path check is a hand port
// of Go's regex: a '/' between path characters, or a token named like a source file.
pub fn need_original(s: &str) -> bool {
    if s.chars().any(|c| NEED_QUOTES.contains(&c))
        || s.matches('\n').count() >= 3
        || s.chars().count() > 400
    {
        return true;
    }
    if s.chars().any(|c| c.is_ascii_digit()) || has_path(s) {
        return true;
    }
    let lower = s.to_lowercase();
    NEED_WORDS.iter().any(|w| lower.contains(w))
}

fn has_path(s: &str) -> bool {
    let is_path = |c: char| c.is_alphanumeric() || matches!(c, '_' | '.' | '-' | '/');
    let chars: Vec<char> = s.chars().collect();
    let slash_path = chars.windows(3).any(|w| {
        w[1] == '/'
            && (w[0].is_alphanumeric() || w[0] == '_' || w[0] == '.' || w[0] == '-')
            && is_path(w[2])
    });
    if slash_path {
        return true;
    }
    s.split_whitespace().any(|tok| {
        let tok = tok.trim_matches(|c: char| !c.is_alphanumeric());
        match tok.rsplit_once('.') {
            Some((name, ext)) => {
                !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                    && PATH_EXTS.contains(&ext)
            }
            None => false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_keeps_status_and_changed_lines() {
        let reply = "working...\n- status: done\nchanged: a.rs: fix\n  - changed: b.rs: add\nbye";
        assert_eq!(
            summarize_reply(reply),
            "status: done\nchanged: a.rs: fix\nchanged: b.rs: add"
        );
    }

    #[test]
    fn summarize_without_status_cuts_to_300_chars() {
        let long = "x".repeat(400);
        assert_eq!(summarize_reply(&long).chars().count(), 300);
        assert_eq!(summarize_reply("  plain  "), "plain");
    }

    #[test]
    fn strip_frontmatter_drops_header_block() {
        let s = "---\nname: haiku-tui\n---\n\nBody line\n";
        assert_eq!(strip_frontmatter(s), "Body line");
        assert_eq!(strip_frontmatter("  plain  \n"), "plain");
        assert_eq!(strip_frontmatter("---\nno end\n"), "---\nno end");
    }

    #[test]
    fn tui_prompt_wraps_reply_after_instruction() {
        let p = tui_prompt("hello");
        assert!(p.starts_with("Opus main return follows."));
        assert!(p.ends_with("\n\nhello"));
    }

    #[test]
    fn summary_is_composed_with_original_and_stage_done() {
        let mut p = Prompter::new();
        assert_eq!(p.start("fix the bug").unwrap(), StagePhase::Working);
        assert!(p.waiting());
        let out = p.reply("OK\nUser wants the bug fixed.\n").unwrap();
        assert_eq!(
            out.text,
            "[prompter summary]\nUser wants the bug fixed.\n\n[original]\nfix the bug"
        );
        assert_eq!(out.stage, Some(StagePhase::Done));
        assert!(!p.waiting());
    }

    #[test]
    fn empty_summary_passes_input_through_without_stage() {
        let mut p = Prompter::new();
        p.start("hi").unwrap();
        let out = p.reply("OK\n").unwrap();
        assert_eq!(
            out,
            PrompterOutcome {
                text: "hi".into(),
                stage: None
            }
        );
    }

    #[test]
    fn empty_reply_fails_and_passes_input_through() {
        let mut p = Prompter::new();
        p.start("hi").unwrap();
        let out = p.reply("").unwrap();
        assert_eq!(out.text, "hi");
        assert!(matches!(out.stage, Some(StagePhase::Failed(_))));
    }

    #[test]
    fn timeout_fails_and_passes_input_through() {
        let mut p = Prompter::new();
        p.start("hi").unwrap();
        let out = p.timeout().unwrap();
        assert_eq!(out.text, "hi");
        assert_eq!(
            out.stage,
            Some(StagePhase::Failed("prompter timed out after 60s".into()))
        );
        assert!(p.timeout().is_none());
    }

    #[test]
    fn second_start_while_pending_is_refused() {
        let mut p = Prompter::new();
        p.start("one").unwrap();
        assert!(p.start("two").is_err());
        assert!(p.reply("OK\ndone").is_some());
        assert!(p.start("two").is_ok());
    }

    #[test]
    fn reply_with_nothing_pending_is_none() {
        assert!(Prompter::new().reply("OK\nx").is_none());
    }

    #[test]
    fn parse_reply_drops_status_head() {
        assert_eq!(
            parse_prompter_reply("NEED_ORIGINAL\n  s  "),
            (true, "s".into())
        );
        assert_eq!(parse_prompter_reply("OK\nx"), (false, "x".into()));
        assert_eq!(
            parse_prompter_reply("  plain text "),
            (false, "plain text".into())
        );
        assert_eq!(parse_prompter_reply("OK"), (false, "".into()));
    }

    #[test]
    fn fail_passes_input_through_with_reason() {
        let mut p = Prompter::new();
        p.start("go").unwrap();
        let out = p.fail("prompter exited").unwrap();
        assert_eq!(out.text, "go");
        assert_eq!(
            out.stage,
            Some(StagePhase::Failed("prompter exited".into()))
        );
        assert!(p.fail("again").is_none());
    }

    #[test]
    fn need_original_flags_exact_inputs() {
        assert!(need_original("rename `foo` to bar"));
        assert!(need_original("line\none\ntwo\nthree"));
        assert!(need_original("version 2 please"));
        assert!(need_original("see src/main.rs"));
        assert!(need_original("open config.json now"));
        assert!(need_original("이 문장 번역해줘"));
        assert!(!need_original("summarize what we did today"));
    }
}
