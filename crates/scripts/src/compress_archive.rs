//! compress-archive (Go: compress_archive.go). Gzips .jsonl transcripts and removes the originals.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use flate2::{Compression, write::GzEncoder};

use crate::common::{fail, parse, usage};

/// Collects `.jsonl` files under `p` in sorted walk order, skipping hidden entries below the root.
fn walk(p: &Path, out: &mut Vec<PathBuf>) {
    let Ok(meta) = fs::symlink_metadata(p) else {
        return;
    };
    if meta.is_dir() {
        let Ok(rd) = fs::read_dir(p) else {
            return;
        };
        let mut children: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        children.sort();
        for child in children {
            let hidden = child
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'));
            if !hidden {
                walk(&child, out);
            }
        }
    } else if p.to_string_lossy().ends_with(".jsonl") {
        out.push(p.to_path_buf());
    }
}

fn gz_path(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".gz");
    PathBuf::from(s)
}

fn gzip_file(p: &Path) -> io::Result<()> {
    let mut src = fs::File::open(p)?;
    let dst = fs::File::create(gz_path(p))?;
    let mut zw = GzEncoder::new(dst, Compression::new(6));
    io::copy(&mut src, &mut zw)?;
    zw.finish()?;
    Ok(())
}

pub fn run(args: &[String]) -> i32 {
    let flags = match parse("compress-archive", &[], args) {
        Ok(f) => f,
        Err(rc) => return rc,
    };
    let dirs = flags.pos;
    if dirs.is_empty() {
        usage("compress-archive");
        return 2;
    }
    let mut before: u64 = 0;
    let mut after: u64 = 0;
    let mut n: u64 = 0;
    for d in &dirs {
        let mut files = Vec::new();
        walk(Path::new(d), &mut files);
        for p in files {
            if let Err(e) = gzip_file(&p) {
                return fail(&e.to_string());
            }
            before += fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            after += fs::metadata(gz_path(&p)).map(|m| m.len()).unwrap_or(0);
            let _ = fs::remove_file(&p);
            n += 1;
        }
    }
    if n > 0 {
        println!(
            "   압축: 기록 {n}개 {:.1}MB → {:.1}MB",
            before as f64 / 1e6,
            after as f64 / 1e6
        );
    } else {
        println!("   압축할 기록 없음: {}", dirs.join(", "));
    }
    0
}
