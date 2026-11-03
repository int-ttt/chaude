//! extract-transcript (Go: extract_transcript.go). Splits a conversation into chunk files.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::claude_history::{history_dir, projects_root};
use crate::common::{
    abs_path, clean, comma_int, each_file_line, fail, fget, file_name_string, glob_dir, norm_case,
    parse, parse_line, py_strip, rune_len, rune_slice, sha8, strip_ext, strip_noise, sval, text_of,
    to_slash, truthy, ts_of,
};

struct Entry {
    ts: String,
    sid: String,
    label: &'static str,
    text: String,
}

/// Keeps the head and tail of `s` when it is longer than `n` characters (Go `clip`).
fn clip(s: &str, n: i64) -> String {
    let r: Vec<char> = s.chars().collect();
    let n = n.max(0) as usize;
    if r.len() <= n {
        return s.to_string();
    }
    let head = (n * 5 / 8).min(r.len());
    let tail_n = n * 3 / 8;
    // Go keeps the whole text as the tail when tail_n is 0; mirrored here.
    let tail: String = if tail_n > 0 {
        r[r.len() - tail_n..].iter().collect()
    } else {
        r.iter().collect()
    };
    let head_s: String = r[..head].iter().collect();
    format!("{head_s}\n[…{}자 생략…]\n{tail}", r.len() - n)
}

pub fn run(args: &[String]) -> i32 {
    let flags = match parse("extract-transcript", &["out", "chunk", "reply-chars"], args) {
        Ok(f) => f,
        Err(rc) => return rc,
    };
    let out = flags.str("out", "notes/distill");
    let chunk_n = match flags.int("chunk", 7000) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    let reply_chars = match flags.int("reply-chars", 800) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    if flags.pos.len() != 1 {
        eprintln!("usage: extract-transcript SOURCE [--out DIR] [--chunk N] [--reply-chars N]");
        return 2;
    }
    let src = abs_path(&flags.pos[0]);
    let src_s = src.to_string_lossy().into_owned();
    let files: Vec<PathBuf> = if src_s.ends_with(".jsonl") {
        vec![src.clone()]
    } else {
        let parent = src.parent().unwrap_or(Path::new(""));
        let dir = if norm_case(parent) == norm_case(&projects_root()) {
            Some(src.clone())
        } else {
            history_dir(&src_s)
        };
        dir.map(|d| glob_dir(&d, ".jsonl")).unwrap_or_default()
    };
    if files.is_empty() {
        return fail(&format!(
            "대화 기록을 찾지 못했습니다: {}\n~/.claude/projects 아래 폴더를 직접 지정해 보세요.",
            to_slash(&src)
        ));
    }

    let mut entries: Vec<Entry> = Vec::new();
    let mut seen_paste: HashSet<String> = HashSet::new();
    for path in &files {
        let sid = rune_slice(strip_ext(&file_name_string(path)), 0, 8);
        let res = each_file_line(path, |line| {
            let Some(e) = parse_line(line) else { return };
            let kind = sval(fget(&e, "type"));
            if (kind != "user" && kind != "assistant") || truthy(fget(&e, "isMeta")) {
                return;
            }
            let t =
                py_strip(&strip_noise(&text_of(fget(fget(&e, "message"), "content")))).to_string();
            if t.is_empty() {
                return;
            }
            let ts = ts_of(&e);
            let (label, text) = if kind == "user" && truthy(fget(&e, "isCompactSummary")) {
                ("압축요약", clip(&t, 4000))
            } else if kind == "user" {
                if rune_len(&t) > 1500 {
                    let h = sha8(&t);
                    if seen_paste.contains(&h) {
                        ("사용자", format!("[이전과 같은 붙여넣기 #{h} 반복]"))
                    } else {
                        seen_paste.insert(h.clone());
                        ("사용자", format!("[붙여넣기 #{h}]\n{}", clip(&t, 3000)))
                    }
                } else {
                    ("사용자", t)
                }
            } else {
                ("Claude", clip(&t, reply_chars))
            };
            entries.push(Entry {
                ts,
                sid: sid.clone(),
                label,
                text,
            });
        });
        if let Err(e) = res {
            return fail(&e.to_string());
        }
    }
    entries.sort_by(|a, b| a.ts.cmp(&b.ts));

    let out_dir = clean(Path::new(&out));
    if let Err(e) = fs::create_dir_all(out_dir.join("chunks")) {
        return fail(&e.to_string());
    }
    let mut chunks: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_len: i64 = 0;
    for en in &entries {
        let block = format!("### {} [{}] {}\n{}\n\n", en.ts, en.sid, en.label, en.text);
        let bl = rune_len(&block) as i64;
        if cur_len > 0 && cur_len + bl > chunk_n {
            chunks.push(std::mem::take(&mut cur));
            cur_len = 0;
        }
        cur.push_str(&block);
        cur_len += bl;
    }
    if cur_len > 0 {
        chunks.push(cur);
    }
    let mut total: i64 = 0;
    for (i, c) in chunks.iter().enumerate() {
        let _ = fs::write(out_dir.join("chunks").join(format!("{:03}.md", i + 1)), c);
        total += rune_len(c) as i64;
    }
    let (first, last) = match (entries.first(), entries.last()) {
        (Some(f), Some(l)) => (f.ts.as_str(), l.ts.as_str()),
        _ => ("-", "-"),
    };
    let par = chunks.len().div_ceil(4).max(1);
    let idx = format!(
        "# 추출 인덱스\n- 원본: {}개 세션, 메시지 {}개\n- 조각: {}개, 총 {}자\n- 기간: {first} ~ {last}\n- 권장 배분: haiku-extractor 1개당 조각 4개 → {par}개 병렬\n",
        files.len(),
        entries.len(),
        chunks.len(),
        comma_int(total)
    );
    let index = out_dir.join("INDEX.md");
    let _ = fs::write(&index, idx);
    println!(
        "세션 {}개 · 메시지 {}개 → 조각 {}개 ({}자) · {}",
        files.len(),
        entries.len(),
        chunks.len(),
        comma_int(total),
        to_slash(&index)
    );
    0
}
