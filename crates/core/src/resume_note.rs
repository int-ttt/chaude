//! Resume digest: a markdown file under notes/tmp/long/ built from a stored session (user prompts, final replies,
//! agent rows, the last recommendation, changed files). /resume writes it and puts its path into the first
//! input of the resumed main. The source is the Claude transcript, so a session that ended by SIGKILL (a
//! partial last line) reads the same way as a clean one.

use std::fs;
use std::path::{Path, PathBuf};

use crate::agent_store::{AgentStore, SavedAgent};
use crate::import_claude::{ClaudeImport, session_stem};
use crate::osenv::home_dir;
use crate::tmp_layout;

// Upper bound on the digest, in chars. The newest turns are kept when it does not fit.
pub const RESUME_CAP: usize = 12_000;
const PROMPT_CHARS: usize = 600;
const REPLY_CHARS: usize = 1_500;
const AGENT_TASK_CHARS: usize = 120;

// Keeps at most n chars from the start, ending with "..." when cut (n >= 3).
fn head(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let kept: String = s.chars().take(n - 3).collect();
    format!("{kept}...")
}

// Keeps at most n chars from the end, starting with "..." when cut (n >= 3).
fn tail(s: &str, n: usize) -> String {
    let count = s.chars().count();
    if count <= n {
        return s.to_string();
    }
    let kept: String = s.chars().skip(count - (n - 3)).collect();
    format!("...{kept}")
}

fn agent_line(a: &SavedAgent) -> String {
    let kind = if a.name.is_empty() {
        a.model.as_str()
    } else {
        a.name.as_str()
    };
    let task = head(&a.task.replace('\n', " "), AGENT_TASK_CHARS);
    let mut line = format!("- [{}] {kind}: {task}", a.status);
    if !a.out_path.is_empty() {
        line.push_str(&format!(" -> {}", a.out_path));
    }
    line
}

// The digest text, at most cap chars. Header, agents, last recommendation and changed files always come first;
// turns fill the rest from the newest backwards.
pub fn build(imp: &ClaudeImport, agents: &[SavedAgent], cap: usize) -> String {
    let mut fixed = format!("# Resumed session {}\n", session_stem(&imp.path));
    if !agents.is_empty() {
        fixed.push_str("\n## Agents\n");
        for a in agents {
            fixed.push_str(&agent_line(a));
            fixed.push('\n');
        }
    }
    if !imp.reco.is_empty() {
        fixed.push_str(&format!("\n## Last recommendation\n{}\n", imp.reco));
    }
    if !imp.files.is_empty() {
        fixed.push_str("\n## Changed files\n");
        for f in &imp.files {
            fixed.push_str(&format!("- {f}\n"));
        }
    }
    let turns_head = "\n## Turns (newest kept)\n";
    let budget = cap.saturating_sub(fixed.chars().count() + turns_head.chars().count());
    let mut used = 0;
    let mut kept: Vec<String> = Vec::new();
    for (user, reply) in imp.turns.iter().rev() {
        let block = format!(
            "\n### User\n{}\n\n### Reply\n{}\n",
            head(user.trim(), PROMPT_CHARS),
            tail(reply.trim(), REPLY_CHARS)
        );
        let n = block.chars().count();
        if used + n > budget {
            break;
        }
        used += n;
        kept.push(block);
    }
    let mut out = fixed;
    if !kept.is_empty() {
        out.push_str(turns_head);
        out.extend(kept.iter().rev().map(String::as_str));
    }
    tail(&out, cap)
}

// The engine agent store: ~/.chaude/agents.json (api.rs Config default, agents_file).
pub fn agents_path() -> PathBuf {
    home_dir().join(".chaude").join("agents.json")
}

// Writes cwd/notes/tmp/long/<newest session id>.md and returns its path. Agent rows come from the parent sessions
// in sids (every session of a group). Errors are text for the caller.
pub fn write(cwd: &Path, imp: &ClaudeImport, sids: &[String]) -> Result<PathBuf, String> {
    let store = AgentStore::new(&agents_path());
    let mut agents: Vec<SavedAgent> = Vec::new();
    for sid in sids.iter().filter(|s| !s.is_empty()) {
        agents.extend(store.load(sid));
    }
    agents.sort_by(|a, b| a.started.cmp(&b.started).then_with(|| a.id.cmp(&b.id)));
    agents.dedup_by(|a, b| a.id == b.id);
    let md = build(imp, &agents, RESUME_CAP);
    let dir = tmp_layout::long_dir(cwd);
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{}.md", session_stem(&imp.path)));
    fs::write(&path, md).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

// The first-input line that points the resumed main at the digest (model-facing, so English).
pub fn hint(path: &Path) -> String {
    format!(
        "Read {} first: it digests the previous session. Continue that prior work.",
        path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_claude::read_claude_transcript;
    use std::time::SystemTime;

    fn turn_import(n: usize) -> ClaudeImport {
        let turns = (0..n)
            .map(|i| {
                (
                    format!("U{i} {}", "p".repeat(300)),
                    format!("R{i} {}", "r".repeat(900)),
                )
            })
            .collect();
        ClaudeImport {
            path: PathBuf::from("/x/sess-1.jsonl"),
            turns,
            ..Default::default()
        }
    }

    #[test]
    fn build_caps_and_keeps_newest_turns() {
        let imp = turn_import(60);
        let md = build(&imp, &[], 4_000);
        assert!(md.chars().count() <= 4_000, "{}", md.chars().count());
        assert!(md.contains("U59 "), "newest prompt missing");
        assert!(md.contains("R59 "), "newest reply missing");
        assert!(!md.contains("U0 "), "oldest prompt should be dropped");
        assert!(md.starts_with("# Resumed session sess-1"));
    }

    #[test]
    fn build_keeps_fixed_sections_under_cap() {
        let mut imp = turn_import(3);
        imp.reco = "추천: 테스트를 실행하세요.".to_string();
        imp.files = vec!["/w/a.rs".to_string()];
        let mut a = SavedAgent::new("ag1", "done", SystemTime::UNIX_EPOCH);
        a.name = "haiku-coder".to_string();
        a.task = "fix\nthe bug".to_string();
        a.out_path = "/w/out.md".to_string();
        let md = build(&imp, &[a], 600);
        assert!(md.chars().count() <= 600);
        assert!(md.contains("- [done] haiku-coder: fix the bug -> /w/out.md"));
        assert!(md.contains("추천: 테스트를 실행하세요."));
        assert!(md.contains("- /w/a.rs"));
    }

    #[test]
    fn build_reads_session_killed_mid_line() {
        let path =
            std::env::temp_dir().join(format!("resume-note-{}-killed.jsonl", std::process::id()));
        let body = concat!(
            r#"{"type":"user","message":{"role":"user","content":"first prompt"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"first reply"}]}}"#,
            "\n",
            r#"{"type":"user","message":{"role":"user","content":"second pro"#,
        );
        fs::write(&path, body).unwrap();
        let imp = read_claude_transcript(&path);
        let md = imp.as_ref().map(|i| build(i, &[], RESUME_CAP));
        let _ = fs::remove_file(&path);
        let md = md.expect("killed transcript must still read");
        assert!(md.contains("first prompt"), "{md}");
        assert!(md.contains("first reply"), "{md}");
    }
}
