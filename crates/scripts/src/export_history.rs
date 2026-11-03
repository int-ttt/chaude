//! export-history (Go: export_history.go). Writes a project's transcripts as plain-text files.

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::claude_history::history_dir;
use crate::common::{
    abs_path, default_project, each_file_line, fail, fget, file_name_string, glob_dir, glob_names,
    parse, parse_line, py_split_lines, py_strip, rune_len, rune_slice, sha8, strip_ext,
    strip_noise, sval, text_of, truthy, ts_of, wrap_text,
};

fn wrap_all(t: &str, width: i64) -> String {
    let mut lines = py_split_lines(t);
    if lines.is_empty() {
        lines.push(String::new());
    }
    let mut out: Vec<String> = Vec::new();
    for l in lines {
        let mut w = wrap_text(&l, width);
        if w.is_empty() {
            w.push(String::new());
        }
        out.extend(w);
    }
    out.join("\n")
}

/// Exports one transcript; returns the written file names.
fn export_one(path: &Path, out_dir: &Path, width: i64, split_bytes: i64) -> Vec<String> {
    let base = file_name_string(path);
    let sid = rune_slice(strip_ext(&base), 0, 8);
    let mut blocks: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut first_ts = String::new();
    let _ = each_file_line(path, |line| {
        let Some(e) = parse_line(line) else { return };
        let kind = sval(fget(&e, "type"));
        if (kind != "user" && kind != "assistant") || truthy(fget(&e, "isMeta")) {
            return;
        }
        let mut t =
            py_strip(&strip_noise(&text_of(fget(fget(&e, "message"), "content")))).to_string();
        if t.is_empty() {
            return;
        }
        let ts = ts_of(&e);
        if first_ts.is_empty() {
            first_ts = ts.clone();
        }
        if kind == "user" && rune_len(&t) > 1500 {
            let h = sha8(&t);
            if !seen.insert(h.clone()) {
                t = format!("[이전과 같은 붙여넣기 #{h}]");
            }
        }
        let who = if truthy(fget(&e, "isCompactSummary")) {
            "압축 요약"
        } else if kind == "user" {
            "사용자"
        } else {
            "Claude"
        };
        let mark = if who == "사용자" { "====" } else { "----" };
        blocks.push(format!(
            "{mark} {ts}  {who} {mark}\n{}\n\n",
            wrap_all(&t, width)
        ));
    });
    if blocks.is_empty() {
        return Vec::new();
    }
    let digits: String = first_ts
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(12)
        .collect();
    let stamp = if digits.len() >= 12 {
        format!("{}-{}", &digits[..8], &digits[8..12])
    } else {
        digits
    };
    for old in glob_names(out_dir, &format!("*-{sid}-*.txt")) {
        if !old.starts_with('.') {
            let _ = fs::remove_file(out_dir.join(old));
        }
    }
    let mut parts: Vec<String> = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    let mut n: i64 = 0;
    for b in &blocks {
        let size = b.len() as i64;
        if !cur.is_empty() && n + size > split_bytes {
            parts.push(cur.concat());
            cur.clear();
            n = 0;
        }
        cur.push(b);
        n += size;
    }
    parts.push(cur.concat());
    let mut names = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let name = format!("{stamp}-{sid}-{:02}.txt", i + 1);
        let _ = fs::write(out_dir.join(&name), part);
        names.push(name);
    }
    names
}

fn write_index(out_dir: &Path) {
    let mut rows: Vec<String> = glob_names(out_dir, "*-*-*.txt")
        .into_iter()
        .filter(|b| !b.starts_with('.'))
        .collect();
    rows.sort();
    let s = format!(
        "대화 기록 (사용자 메시지와 Claude 답변만, 최신이 아래)\n\n{}\n",
        rows.join("\n")
    );
    let _ = fs::write(out_dir.join("INDEX.txt"), s);
}

pub fn run(args: &[String]) -> i32 {
    // --hook is a bool flag; lean has none, so it is stripped before parsing.
    let mut hook = false;
    let mut rest: Vec<String> = Vec::new();
    let mut ended = false;
    for a in args {
        if !ended {
            if a == "--" {
                ended = true;
            } else if a == "--hook" || a == "-hook" {
                hook = true;
                continue;
            }
        }
        rest.push(a.clone());
    }
    let default_proj = default_project();
    let flags = match parse(
        "export-history",
        &["project", "file", "out", "width", "split-kb"],
        &rest,
    ) {
        Ok(f) => f,
        Err(rc) => return rc,
    };
    let project = flags.str("project", &default_proj);
    let file = flags.str("file", "");
    let out = flags.str("out", "");
    let width = match flags.int("width", 120) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    let split_kb = match flags.int("split-kb", 500) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    if !flags.pos.is_empty() {
        eprintln!("export-history: unrecognized arguments");
        return 2;
    }

    let files: Vec<PathBuf> = if hook {
        let mut raw = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut raw);
        let data: Option<Value> = serde_json::from_slice::<Value>(&raw)
            .ok()
            .filter(|v| v.is_object());
        match data.as_ref().map(|v| sval(fget(v, "transcript_path"))) {
            Some(p) if !p.is_empty() => vec![PathBuf::from(p)],
            _ => Vec::new(),
        }
    } else if !file.is_empty() {
        vec![PathBuf::from(file)]
    } else {
        history_dir(&project)
            .map(|d| glob_dir(&d, ".jsonl"))
            .unwrap_or_default()
    };
    let out_dir = if out.is_empty() {
        abs_path(&project).join("logs").join("history")
    } else {
        PathBuf::from(&out)
    };
    if let Err(e) = fs::create_dir_all(&out_dir) {
        return fail(&e.to_string());
    }
    let mut total = 0usize;
    for p in &files {
        if p.exists() {
            total += export_one(p, &out_dir, width, split_kb * 1024).len();
        }
    }
    write_index(&out_dir);
    if !hook {
        println!(
            "세션 {}개 → 파일 {total}개 · {}",
            files.len(),
            out_dir.display()
        );
    }
    0
}
