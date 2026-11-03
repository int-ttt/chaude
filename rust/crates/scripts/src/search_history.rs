//! search-history (Go: search_history.go). Finds messages containing every search term.

use std::path::PathBuf;

use chaude_core::osenv;

use crate::claude_history::history_dir;
use crate::common::{
    abs_path, default_project, each_file_line, fail, fget, file_name_string, glob_dir, glob_names,
    lower_runes, parse, parse_line, py_fields, rune_len, rune_slice, strip_noise, sval, text_of,
    truthy, ts_of,
};

struct Hit {
    ts: String,
    sid: String,
    who: &'static str,
    snip: String,
}

pub fn run(args: &[String]) -> i32 {
    let default_proj = default_project();
    let flags = match parse(
        "search-history",
        &["project", "since", "who", "limit", "width"],
        args,
    ) {
        Ok(f) => f,
        Err(rc) => return rc,
    };
    let project = flags.str("project", &default_proj);
    let since = flags.str("since", "");
    let who = flags.str("who", "all");
    let limit = match flags.int("limit", 12) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    let width = match flags.int("width", 300) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    if flags.pos.len() != 1 {
        eprintln!(
            "usage: search-history \"<terms>\" [--project DIR] [--since YYYY-MM-DD] [--who user|claude|all] [--limit N]"
        );
        return 2;
    }
    if !matches!(who.as_str(), "user" | "claude" | "all") {
        eprintln!(
            "search-history: error: argument --who: invalid choice: '{who}' (choose from 'user', 'claude', 'all')"
        );
        return 2;
    }
    let raw_terms = flags.pos[0].clone();
    let proj = abs_path(&project);

    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(d) = history_dir(&proj.to_string_lossy()) {
        dirs.push(d);
    }
    let archive = osenv::home_dir().join("claude-archive");
    for name in glob_names(&archive, &format!("{}-*", file_name_string(&proj))) {
        dirs.push(archive.join(name));
    }
    let mut files: Vec<PathBuf> = Vec::new();
    for d in &dirs {
        files.extend(glob_dir(d, ".jsonl"));
        files.extend(glob_dir(d, ".jsonl.gz"));
    }
    if files.is_empty() {
        return fail(&format!("대화 기록 없음: {}", proj.display()));
    }

    let terms: Vec<String> = py_fields(&raw_terms).into_iter().map(lower_runes).collect();
    let all_in = |s: &str| terms.iter().all(|t| s.contains(t.as_str()));
    let mut hits: Vec<Hit> = Vec::new();
    let mut empty_terms = false;
    for path in &files {
        let sid = rune_slice(&file_name_string(path), 0, 8);
        let _ = each_file_line(path, |line| {
            if !all_in(&lower_runes(line)) {
                return;
            }
            let Some(e) = parse_line(line) else { return };
            let kind = sval(fget(&e, "type"));
            if (kind != "user" && kind != "assistant") || truthy(fget(&e, "isMeta")) {
                return;
            }
            if (who == "user" && kind != "user") || (who == "claude" && kind != "assistant") {
                return;
            }
            let ts = ts_of(&e);
            if !since.is_empty() && rune_slice(&ts, 0, 10) < since {
                return;
            }
            let t =
                py_fields(&strip_noise(&text_of(fget(fget(&e, "message"), "content")))).join(" ");
            let low = lower_runes(&t);
            if t.is_empty() || !all_in(&low) {
                return;
            }
            // Go panics here on an empty term list; reported as exit 2 instead.
            let Some(first) = terms.first() else {
                empty_terms = true;
                return;
            };
            let idx = low.find(first.as_str()).unwrap_or(0);
            let i = low[..idx].chars().count() as i64;
            let start = (i - width / 3).max(0);
            let n = rune_len(&t) as i64;
            let mut snip = String::new();
            if start > 0 {
                snip.push('…');
            }
            snip.push_str(&rune_slice(&t, start, start + width));
            if start + width < n {
                snip.push('…');
            }
            let label = if truthy(fget(&e, "isCompactSummary")) {
                "압축요약"
            } else if kind == "user" {
                "사용자"
            } else {
                "Claude"
            };
            hits.push(Hit {
                ts,
                sid: sid.clone(),
                who: label,
                snip,
            });
        });
    }
    if empty_terms {
        eprintln!("search-history: error: 검색어가 없습니다");
        return 2;
    }

    hits.sort_by(|a, b| b.ts.cmp(&a.ts));
    let len = hits.len() as i64;
    let shown = limit.min(len);
    println!(
        "'{raw_terms}' 일치 {}건 (최신순 {shown}건 표시, 세션 파일 {}개)",
        hits.len(),
        files.len()
    );
    let mut end = limit;
    if end < 0 {
        end = (len + end).max(0);
    }
    let end = end.min(len) as usize;
    for h in &hits[..end] {
        println!("- {} [{}] {}: {}", h.ts, h.sid, h.who, h.snip);
    }
    0
}
