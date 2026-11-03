//! usage-report (Go: usage_report.go). Token totals and estimated cost per model family and session.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::common::{
    comma_str, each_file_line, expand_user, fget, file_name_string, glob_dir, parse, parse_line,
    rune_len, rune_slice, strip_ext, sval, truthy,
};

const FAMILIES: [&str; 3] = ["opus", "sonnet", "haiku"];

/// USD per million tokens: input, cache write, cache read, output.
fn price(f: &str) -> [f64; 4] {
    match f {
        "sonnet" => [2.0, 4.0, 0.20, 10.0],
        "haiku" => [0.10, 0.20, 0.01, 0.50],
        _ => [4.0, 8.0, 0.20, 20.0],
    }
}

#[derive(Default, Clone, Copy)]
struct Agg {
    req: i64,
    inp: i64,
    cw: i64,
    cr: i64,
    out: i64,
    peak: i64,
}

/// Per-family totals in first-seen order (Python dict order in the Go original).
#[derive(Default)]
struct Usage {
    items: Vec<(&'static str, Agg)>,
}

impl Usage {
    fn get(&mut self, f: &'static str) -> &mut Agg {
        let i = match self.items.iter().position(|(k, _)| *k == f) {
            Some(i) => i,
            None => {
                self.items.push((f, Agg::default()));
                self.items.len() - 1
            }
        };
        &mut self.items[i].1
    }
}

fn family(model: &str) -> &'static str {
    let m = model.to_lowercase();
    FAMILIES
        .into_iter()
        .find(|f| m.contains(*f))
        .unwrap_or("opus")
}

fn num(v: &Value) -> i64 {
    v.as_f64().map_or(0, |f| f as i64)
}

fn scan_usage(path: &Path, ag: &mut Usage, seen: &mut HashSet<String>) {
    let _ = each_file_line(path, |line| {
        let Some(e) = parse_line(line) else { return };
        let msg = fget(&e, "message");
        let u = fget(msg, "usage");
        if sval(fget(&e, "type")) != "assistant" || !truthy(u) {
            return;
        }
        let mut mid = sval(fget(msg, "id")).to_string();
        if mid.is_empty() {
            mid = sval(fget(&e, "uuid")).to_string();
        }
        if !seen.insert(mid) {
            return;
        }
        let a = ag.get(family(sval(fget(msg, "model"))));
        let inp = num(fget(u, "input_tokens"));
        let cw = num(fget(u, "cache_creation_input_tokens"));
        let cr = num(fget(u, "cache_read_input_tokens"));
        let out = num(fget(u, "output_tokens"));
        a.req += 1;
        a.inp += inp;
        a.cw += cw;
        a.cr += cr;
        a.out += out;
        let s = inp + cw + cr;
        if s > a.peak {
            a.peak = s;
        }
    });
}

fn cost(f: &str, a: &Agg) -> f64 {
    let p = price(f);
    let mut s = a.inp as f64 * p[0];
    s += a.cw as f64 * p[1];
    s += a.cr as f64 * p[2];
    s += a.out as f64 * p[3];
    s / 1e6
}

struct Row {
    total: f64,
    proj: String,
    sid: String,
    ag: Usage,
}

fn pad_r(s: &str, n: usize) -> String {
    let k = rune_len(s);
    if k < n {
        format!("{s}{}", " ".repeat(n - k))
    } else {
        s.to_string()
    }
}

fn last_runes(s: &str, n: usize) -> String {
    let c: Vec<char> = s.chars().collect();
    if c.len() > n {
        c[c.len() - n..].iter().collect()
    } else {
        s.to_string()
    }
}

pub fn run(args: &[String]) -> i32 {
    let flags = match parse("usage-report", &["top"], args) {
        Ok(f) => f,
        Err(rc) => return rc,
    };
    let top = match flags.int("top", 15) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    if flags.pos.len() > 1 {
        eprintln!("usage: usage-report [PROJECTS_DIR] [--top N]");
        return 2;
    }
    let root_s = flags
        .pos
        .first()
        .cloned()
        .unwrap_or_else(|| expand_user("~/.claude/projects"));
    let root = Path::new(&root_s);

    let mut proj_dirs: Vec<String> = match fs::read_dir(root) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    proj_dirs.sort();
    let mut rows: Vec<Row> = Vec::new();
    for d in proj_dirs {
        if d.starts_with('.') {
            continue;
        }
        let pdir = root.join(&d);
        for main_f in glob_dir(&pdir, ".jsonl") {
            let sid = strip_ext(&file_name_string(&main_f)).to_string();
            let mut ag = Usage::default();
            let mut seen = HashSet::new();
            scan_usage(&main_f, &mut ag, &mut seen);
            for sub in glob_dir(&pdir.join(&sid).join("subagents"), ".jsonl") {
                scan_usage(&sub, &mut ag, &mut seen);
            }
            if ag.items.is_empty() {
                continue;
            }
            let total: f64 = ag.items.iter().map(|(f, a)| cost(f, a)).sum();
            rows.push(Row {
                total,
                proj: d.clone(),
                sid,
                ag,
            });
        }
    }
    rows.sort_by(|a, b| {
        b.total
            .partial_cmp(&a.total)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.proj.cmp(&a.proj))
            .then_with(|| b.sid.cmp(&a.sid))
    });

    let mut grand = Usage::default();
    for r in &rows {
        for (f, a) in &r.ag.items {
            let g = grand.get(f);
            g.req += a.req;
            g.inp += a.inp;
            g.cw += a.cw;
            g.cr += a.cr;
            g.out += a.out;
            g.peak = g.peak.max(a.peak);
        }
    }
    let mega = |n: i64| format!("{:.1}M", n as f64 / 1e6);
    println!("== 전체 (모델별) ==");
    for (f, a) in &grand.items {
        let c = cost(f, a);
        let p = price(f);
        let mut share = String::new();
        if c != 0.0 {
            let vals = [
                ("읽기(캐시)", a.cr as f64 * p[2]),
                ("저장(캐시)", a.cw as f64 * p[1]),
                ("입력", a.inp as f64 * p[0]),
                ("출력·생각", a.out as f64 * p[3]),
            ];
            share = vals
                .iter()
                .map(|(k, v)| format!("{k} {:.0}%", v / 1e6 / c * 100.0))
                .collect::<Vec<_>>()
                .join(", ");
        }
        println!(
            "{} 요청 {:>6}  캐시읽기 {}  캐시저장 {}  출력 {}  최대맥락 {}k  ≈ ${}  ({})",
            pad_r(f, 7),
            a.req,
            mega(a.cr),
            mega(a.cw),
            mega(a.out),
            a.peak / 1000,
            comma_str(&format!("{c:.2}")),
            share
        );
    }

    println!("\n== 비용 상위 세션 {top}개 ==");
    let len = rows.len() as i64;
    let mut n = top;
    if n < 0 {
        n = (len + n).max(0);
    }
    let n = n.min(len) as usize;
    for r in &rows[..n] {
        let (mut req, mut cr, mut peak) = (0i64, 0i64, 0i64);
        for (_, a) in &r.ag.items {
            req += a.req;
            cr += a.cr;
            peak = peak.max(a.peak);
        }
        let den = req.max(1);
        println!(
            "${:>8.2}  요청 {:>5}  최대맥락 {:>4}k  요청당 평균읽기 {:>5.0}k  {} / {}",
            r.total,
            req,
            peak / 1000,
            cr as f64 / den as f64 / 1000.0,
            last_runes(&r.proj, 40),
            rune_slice(&r.sid, 0, 8)
        );
    }
    0
}
