// Record turns (Go record_turn.go): which prompts are saved, the per-turn measurement line, the topic of
// the turn, saving the turn into its project folder, and /resume answers.

use std::io;
use std::path::Path;
use std::time::SystemTime;

use serde_json::{Map, Value};

use crate::record::{self, Block, Stamp};
use crate::record_folder::{self, Project};

// Go recordSkip: prompts whose answer is never saved are slash commands and empty prompts.
pub fn skip(prompt: &str) -> bool {
    match prompt.split_whitespace().next() {
        None => true,
        Some(first) => first.starts_with('/'),
    }
}

// Go measureKeys: the fixed measurement keys, in output order. result_* keys follow, sorted.
const MEASURE_KEYS: [&str; 6] = [
    "duration_ms",
    "cost_turn_usd",
    "cost_usd",
    "ctx",
    "rotations",
    "delegations",
];

// Go %.4g for a float64: 4 significant digits, %e form outside [1e-4, 1e4), trailing zeros dropped.
fn fmt_g4(v: f64) -> String {
    if v == 0.0 {
        return if v.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    if !v.is_finite() {
        return v.to_string();
    }
    let sci = format!("{v:.3e}"); // rounded to 4 significant digits; exponent after rounding
    let (mant, exp) = sci.split_once('e').expect("exponent form");
    let exp: i32 = exp.parse().expect("exponent");
    if !(-4..4).contains(&exp) {
        let sign = if exp < 0 { '-' } else { '+' };
        return format!("{}e{sign}{:02}", trim_zeros(mant), exp.abs());
    }
    let dec = (3 - exp).max(0) as usize;
    trim_zeros(&format!("{v:.dec$}"))
}

fn trim_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

// Go measureLine: "[측정] k=v ..." over the fixed keys then the result_* keys, numeric values only.
// It returns "" when no key is present.
pub fn measure_line(fields: &Map<String, Value>) -> String {
    let mut extra: Vec<&String> = fields.keys().filter(|k| k.starts_with("result_")).collect();
    extra.sort();
    let keys = MEASURE_KEYS
        .iter()
        .copied()
        .map(str::to_string)
        .chain(extra.into_iter().cloned());
    let mut parts = Vec::new();
    for k in keys {
        let s = match fields.get(&k) {
            Some(Value::Number(n)) if n.is_f64() => fmt_g4(n.as_f64().unwrap_or_default()),
            Some(Value::Number(n)) => n.to_string(),
            _ => continue,
        };
        parts.push(format!("{k}={s}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("[측정] {}", parts.join(" "))
    }
}

// The per-REPL record state: the reply of the turn that just ended, the measurement line of that turn,
// the topic carried over between turns, and whether the next turn starts a new file (after /clear).
#[derive(Debug, Default)]
pub struct RecordState {
    pub last_reply: String,
    pub last_measure: String,
    pub topic: String,
    pub split: bool,
}

impl RecordState {
    // Go recordTopicFor: takes the reply of the turn that just ended (cleared here). It returns the topic the
    // answer is saved under and the reply without [cat:] markers. topic is "" when the turn is not saved.
    pub fn topic_for(&mut self, prompt: &str) -> (String, String) {
        let reply = std::mem::take(&mut self.last_reply);
        if skip(prompt) || reply.is_empty() {
            return (String::new(), String::new());
        }
        self.topic = record::topic_of(&reply, &self.topic);
        let text = record::strip_cat_marks(&reply).trim().to_string();
        (self.topic.clone(), text)
    }

    // Go recordSave: saves the turn that just ended under its topic, unless skipped. The measurement is
    // cleared whether or not the turn is saved. Root is the record root; cwd picks the project folder.
    pub fn save(&mut self, root: &Path, cwd: &Path, prompt: &str, stamp: &Stamp) -> io::Result<()> {
        let measure = std::mem::take(&mut self.last_measure);
        let (topic, mut text) = self.topic_for(prompt);
        if topic.is_empty() {
            return Ok(());
        }
        if !measure.is_empty() {
            text.push_str("\n\n");
            text.push_str(&measure);
        }
        record_folder::migrate_root(root)?;
        let turn = [
            Block {
                kind: "user".to_string(),
                text: prompt.to_string(),
            },
            Block {
                kind: "asst".to_string(),
                text,
            },
        ];
        let dir = record_folder::project_dir(root, cwd);
        record::append_turn(&dir, stamp, &topic, &turn, self.split)?;
        self.split = false;
        Ok(())
    }
}

// Go recordResume: answers /resume <arg> over the record root (one folder per project). An empty arg lists
// the project folders. A number picks that folder of the list; otherwise arg filters the folders by name or
// content: one match loads all its files as one history (in time order), several match as a list.
// Returns (lines, blocks). date formats a folder's mod time as local "yyyy-mm-dd".
pub fn resume(
    root: &Path,
    arg: &str,
    date: &dyn Fn(SystemTime) -> String,
) -> Result<(Vec<String>, Vec<Block>), String> {
    let projects = record_folder::list_projects(root).map_err(|e| e.to_string())?;
    if projects.is_empty() {
        return Err(format!("no record in {}", root.display()));
    }
    if arg.is_empty() {
        let title = "record projects (/resume <번호|이름|검색어> opens one):";
        return Ok((
            record_folder::project_lines(title, &projects, true, date),
            Vec::new(),
        ));
    }
    let picked: Project = match arg.parse::<i64>() {
        Ok(n) => {
            if n < 1 || n as usize > projects.len() {
                return Err(format!("no project {n} (1-{})", projects.len()));
            }
            projects[n as usize - 1].clone()
        }
        Err(_) => {
            let matches =
                record_folder::filter_projects(&projects, arg).map_err(|e| e.to_string())?;
            match matches.len() {
                0 => return Err(format!("no project matches {arg:?}")),
                1 => matches[0].clone(),
                _ => {
                    let title =
                        format!("record projects matching {arg} (/resume <이름> opens one):");
                    return Ok((
                        record_folder::project_lines(&title, &matches, false, date),
                        Vec::new(),
                    ));
                }
            }
        }
    };
    let entries = record_folder::load_project(&picked.dir).map_err(|e| e.to_string())?;
    let blocks = entries
        .into_iter()
        .map(|e| Block {
            kind: e.kind,
            text: e.text,
        })
        .collect();
    Ok((Vec::new(), blocks))
}

// Go recordPick: the project folders the TUI /resume picker offers: every folder for an empty arg, the hits
// for a multi-hit query. An empty result means arg loads one folder directly (or is a number, or fails in
// resume).
pub fn pick(root: &Path, arg: &str) -> io::Result<Vec<Project>> {
    let projects = record_folder::list_projects(root)?;
    if projects.is_empty() {
        return Ok(Vec::new());
    }
    if arg.is_empty() {
        return Ok(projects);
    }
    if arg.parse::<i64>().is_ok() {
        return Ok(Vec::new());
    }
    let matches = record_folder::filter_projects(&projects, arg)?;
    if matches.len() < 2 {
        return Ok(Vec::new());
    }
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn skip_slash_commands_and_empty_prompts() {
        assert!(!skip("분석 a"));
        assert!(!skip("측정"));
        assert!(skip("/resume"));
        assert!(skip(""));
        assert!(skip("   "));
        assert!(!skip("hello"));
    }

    #[test]
    fn measure_line_fixed_then_result_keys() {
        let v = serde_json::json!({
            "stage": "turn",
            "start_ms": 1,
            "duration_ms": 1200,
            "cost_usd": 0.01234,
            "result_model_usage": {},
            "result_input_tokens": 10,
        });
        let fields = v.as_object().unwrap();
        assert_eq!(
            measure_line(fields),
            "[측정] duration_ms=1200 cost_usd=0.01234 result_input_tokens=10"
        );
        assert_eq!(measure_line(&Map::new()), "");
    }

    #[test]
    fn fmt_g4_matches_go_percent_g() {
        assert_eq!(fmt_g4(0.01234), "0.01234");
        assert_eq!(fmt_g4(123.456), "123.5");
        assert_eq!(fmt_g4(1_500_000.0), "1.5e+06");
        assert_eq!(fmt_g4(1_234_567.0), "1.235e+06");
        assert_eq!(fmt_g4(100.0), "100");
        assert_eq!(fmt_g4(9.99996), "10");
        assert_eq!(fmt_g4(0.00001), "1e-05");
        assert_eq!(fmt_g4(0.5), "0.5");
    }

    #[test]
    fn topic_carries_over_and_markers_are_stripped() {
        let mut st = RecordState::default();
        st.last_reply = "answer [cat: parser]\nmore".to_string();
        let (topic, text) = st.topic_for("분석 a");
        assert_eq!(topic, "parser");
        assert_eq!(text, "answer\nmore");

        st.last_reply = "no mark here".to_string();
        let (topic, _) = st.topic_for("다음");
        assert_eq!(topic, "parser");
        assert_eq!(st.last_reply, "");

        st.last_reply = "switch [cat-switch: ui]".to_string();
        assert_eq!(st.topic_for("/resume").0, "", "skipped turn saves nothing");
        st.last_reply = "switch [cat-switch: ui]".to_string();
        assert_eq!(st.topic_for("다음").0, "ui");
    }

    #[test]
    fn save_writes_turn_with_measure_and_clears_split_on_success() {
        let tmp = TempDir::new("rt-save");
        let root = tmp.path().join("repo").join("record");
        let cwd = tmp.path().join("repo");
        let stamp = Stamp::new(2026, 11, 4, 9, 5, 7);
        let mut st = RecordState {
            last_reply: "done [cat: core]".to_string(),
            last_measure: "[측정] duration_ms=5".to_string(),
            split: true,
            ..Default::default()
        };
        st.save(&root, &cwd, "작업", &stamp).unwrap();
        assert!(!st.split);
        assert_eq!(st.last_measure, "");
        let dir = record_folder::project_dir(&root, &cwd);
        let entries = record_folder::load_project(&dir).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, "user");
        assert_eq!(entries[1].text, "done\n\n[측정] duration_ms=5");
        assert_eq!(entries[1].topic, "core");

        // a skipped turn leaves the folder alone and still clears the measurement
        st.last_measure = "[측정] x=1".to_string();
        st.last_reply = "ignored".to_string();
        st.save(&root, &cwd, "/clear", &stamp).unwrap();
        assert_eq!(st.last_measure, "");
        assert_eq!(record_folder::load_project(&dir).unwrap().len(), 2);
    }

    #[test]
    fn resume_lists_picks_and_reports_errors() {
        let tmp = TempDir::new("rt-resume");
        let root = tmp.path().join("record");
        let date = |_: SystemTime| "2026-11-04".to_string();
        let err = resume(&root, "", &date).unwrap_err();
        assert_eq!(err, format!("no record in {}", root.display()));

        let stamp = Stamp::new(2026, 11, 4, 9, 5, 7);
        let turn = |t: &str| {
            vec![Block {
                kind: "asst".to_string(),
                text: t.to_string(),
            }]
        };
        record::append_turn(&root.join("alpha"), &stamp, "a", &turn("first"), false).unwrap();
        record::append_turn(&root.join("beta"), &stamp, "b", &turn("needle"), false).unwrap();

        let (lines, blocks) = resume(&root, "", &date).unwrap();
        assert_eq!(lines.len(), 3);
        assert!(lines[1].starts_with(" 1. "));
        assert!(blocks.is_empty());

        assert_eq!(resume(&root, "9", &date).unwrap_err(), "no project 9 (1-2)");
        assert_eq!(
            resume(&root, "zzz", &date).unwrap_err(),
            "no project matches \"zzz\""
        );

        let (_, blocks) = resume(&root, "needle", &date).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "needle");

        assert_eq!(pick(&root, "").unwrap().len(), 2);
        assert!(pick(&root, "1").unwrap().is_empty());
        assert!(pick(&root, "needle").unwrap().is_empty());
    }
}
