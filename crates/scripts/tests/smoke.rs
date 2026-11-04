use std::fs;
use std::path::{Path, PathBuf};

use chaude_scripts::{NAMES, has, run};

const ALL: [&str; 7] = [
    "charcount",
    "claude-history",
    "compress-archive",
    "export-history",
    "extract-transcript",
    "search-history",
    "usage-report",
];

/// Fresh empty directory under the system temp dir, unique per test name and process.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chaude-scripts-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn registry_lists_all_seven_scripts() {
    for name in ALL {
        assert!(has(name), "{name} not registered");
        assert!(NAMES.contains(name), "{name} missing from NAMES");
    }
    assert!(!has("repl"));
    assert_eq!(run("repl", &[]), 2);
}

#[test]
fn help_flag_exits_zero_for_every_script() {
    for name in ALL {
        assert_eq!(run(name, &args(&["-h"])), 0, "{name} -h");
    }
}

#[test]
fn compress_archive_without_dirs_is_usage_error() {
    assert_eq!(run("compress-archive", &[]), 2);
}

#[test]
fn compress_archive_gzips_jsonl_and_removes_original() {
    let dir = scratch("compress");
    let src = dir.join("session.jsonl");
    fs::write(&src, "{\"type\":\"user\"}\n".repeat(50)).unwrap();
    let code = run("compress-archive", &args(&[dir.to_str().unwrap()]));
    assert_eq!(code, 0);
    assert!(!src.exists(), "original jsonl should be removed");
    assert!(Path::new(&format!("{}.gz", src.display())).exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn extract_transcript_without_source_is_usage_error() {
    assert_eq!(run("extract-transcript", &[]), 2);
}

#[test]
fn extract_transcript_writes_index_for_one_jsonl() {
    let dir = scratch("extract");
    let src = dir.join("s.jsonl");
    fs::write(
        &src,
        "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"안녕하세요 test\"}}\n",
    )
    .unwrap();
    let out = dir.join("out");
    let code = run(
        "extract-transcript",
        &args(&[src.to_str().unwrap(), "--out", out.to_str().unwrap()]),
    );
    assert_eq!(code, 0);
    assert!(out.join("INDEX.md").exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn usage_report_on_empty_projects_dir_exits_zero() {
    let dir = scratch("usage");
    assert_eq!(run("usage-report", &args(&[dir.to_str().unwrap()])), 0);
    let _ = fs::remove_dir_all(&dir);
}
