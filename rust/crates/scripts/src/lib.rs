//! Maintenance scripts ported from the Go `internal/scripts` package.

pub mod charcount;
pub mod claude_history;
pub mod common;
pub mod compress_archive;
pub mod export_history;
pub mod extract_transcript;
pub mod search_history;
pub mod usage_report;

pub const NAMES: &str = "charcount | claude-history | compress-archive | export-history | extract-transcript | search-history | usage-report";

pub fn has(name: &str) -> bool {
    matches!(
        name,
        "charcount"
            | "claude-history"
            | "compress-archive"
            | "export-history"
            | "extract-transcript"
            | "search-history"
            | "usage-report"
    )
}

pub fn run(name: &str, args: &[String]) -> i32 {
    match name {
        "charcount" => charcount::run(args),
        "claude-history" => claude_history::run(args),
        "compress-archive" => compress_archive::run(args),
        "export-history" => export_history::run(args),
        "extract-transcript" => extract_transcript::run(args),
        "search-history" => search_history::run(args),
        "usage-report" => usage_report::run(args),
        _ => {
            eprintln!("chaude: unknown script {name:?}");
            2
        }
    }
}
