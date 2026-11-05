// Record project folders (Go record_folder.go): one folder per checkout under the record root, the
// /resume project list, and the migration of old date/topic files that sit directly in the root.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::record::{self, Entry};

// One project folder under the record root, for the /resume list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub dir: PathBuf,
    pub files: usize,
    pub last: SystemTime, // newest file mod time in the folder
}

// Go folderName: the path's basename, sanitized as a slug, never hidden. "misc" when empty.
pub fn folder_name(path: &Path) -> String {
    let base = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = record::slugify(&base);
    let name = name.trim_start_matches('.');
    if name.is_empty() {
        "misc".to_string()
    } else {
        name.to_string()
    }
}

// Go recordProjectName: the folder name of the project at cwd.
pub fn project_name(cwd: &Path) -> String {
    folder_name(cwd)
}

// Go recordProjectDir: the folder of the project at cwd under the record root.
pub fn project_dir(root: &Path, cwd: &Path) -> PathBuf {
    root.join(project_name(cwd))
}

// Go recordLegacyRe `^\d{6}-.+-\d{3,}\.md$`: the old YYMMDD-<slug>-NNN.md names.
fn is_legacy(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".md") else {
        return false;
    };
    let b = stem.as_bytes();
    if b.len() < 6 + 1 + 1 + 1 + 3 || !b[..6].iter().all(u8::is_ascii_digit) || b[6] != b'-' {
        return false;
    }
    let rest = &stem[7..];
    let digits = rest.bytes().rev().take_while(u8::is_ascii_digit).count();
    // the trailing digit run must follow a '-' that has at least one char before it
    digits >= 3 && rest.len() - digits >= 2 && rest.as_bytes()[rest.len() - digits - 1] == b'-'
}

// Go migrateRecordRoot: moves the old files that sit directly in root into one project folder named
// after root's parent (the checkout that owns root). Files already in a folder are not touched.
pub fn migrate_root(root: &Path) -> io::Result<()> {
    let ents = match fs::read_dir(root) {
        Ok(ents) => ents,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let mut legacy = Vec::new();
    for ent in ents {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().into_owned();
        if !ent.file_type()?.is_dir() && is_legacy(&name) {
            legacy.push(name);
        }
    }
    if legacy.is_empty() {
        return Ok(());
    }
    let owner = root.parent().unwrap_or(root);
    let dst = root.join(folder_name(owner));
    fs::create_dir_all(&dst)?;
    for name in legacy {
        fs::rename(root.join(&name), dst.join(&name))?;
    }
    Ok(())
}

// Go pruneRecordFiles: deletes the .md records in project folders under root whose mtime is older than
// `days`. days <= 0 keeps everything. Emptied project folders are removed too. Returns the removed count.
pub fn prune_record_files(root: &Path, days: i64, now: SystemTime) -> io::Result<usize> {
    if days <= 0 {
        return Ok(0);
    }
    let Some(cutoff) = now.checked_sub(Duration::from_secs(days as u64 * 24 * 60 * 60)) else {
        return Ok(0);
    };
    let Ok(dirs) = fs::read_dir(root) else {
        return Ok(0);
    };
    let mut removed = 0;
    for d in dirs.flatten() {
        let name = d.file_name().to_string_lossy().into_owned();
        if !d.file_type().is_ok_and(|t| t.is_dir()) || name.starts_with('.') {
            continue;
        }
        let dir = root.join(&name);
        let Ok(files) = fs::read_dir(&dir) else {
            continue;
        };
        for f in files.flatten() {
            let fname = f.file_name().to_string_lossy().into_owned();
            if !fname.ends_with(".md") || !f.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let old = f
                .metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|m| m < cutoff);
            if old && fs::remove_file(dir.join(&fname)).is_ok() {
                removed += 1;
            }
        }
        if fs::read_dir(&dir).is_ok_and(|mut left| left.next().is_none()) {
            let _ = fs::remove_dir(&dir);
        }
    }
    Ok(removed)
}

// Go listRecordProjects: the project folders under root (old files migrated first), most recently
// written first. Hidden folders and folders without a record file are skipped.
pub fn list_projects(root: &Path) -> io::Result<Vec<Project>> {
    migrate_root(root)?;
    let ents = match fs::read_dir(root) {
        Ok(ents) => ents,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    for ent in ents {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().into_owned();
        if !ent.file_type()?.is_dir() || name.starts_with('.') {
            continue;
        }
        let dir = root.join(&name);
        let files = record::list_files(&dir)?;
        if files.is_empty() {
            continue;
        }
        let mut last = SystemTime::UNIX_EPOCH;
        for f in &files {
            let mod_time = fs::metadata(dir.join(&f.name))?.modified()?;
            last = last.max(mod_time);
        }
        out.push(Project {
            name,
            dir,
            files: files.len(),
            last,
        });
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.last)); // stable: ties keep read_dir order
    Ok(out)
}

// Go filterRecordProjects: keeps the folders whose name, or any block topic or text, contains q
// (case-insensitive).
pub fn filter_projects(projects: &[Project], q: &str) -> io::Result<Vec<Project>> {
    let q = q.to_lowercase();
    let mut out = Vec::new();
    for p in projects {
        if p.name.to_lowercase().contains(&q) {
            out.push(p.clone());
            continue;
        }
        let entries = load_project(&p.dir)?;
        let hit = entries
            .iter()
            .any(|e| e.text.to_lowercase().contains(&q) || e.topic.to_lowercase().contains(&q));
        if hit {
            out.push(p.clone());
        }
    }
    Ok(out)
}

// Go loadRecordProject: every block of a project folder, all its files as one history, in rec-number
// order (ties keep file order).
pub fn load_project(dir: &Path) -> io::Result<Vec<Entry>> {
    let mut out = Vec::new();
    for f in record::list_files(dir)? {
        let data = fs::read(dir.join(&f.name))?;
        out.extend(record::parse_entries(&String::from_utf8_lossy(&data)));
    }
    out.sort_by_key(|e| e.num);
    Ok(out)
}

// Go recordProjectLines: folder rows for /resume. Numbered rows can be picked with /resume <number>.
// date formats a mod time as local "yyyy-mm-dd" (the crate has no time-zone dependency).
pub fn project_lines(
    title: &str,
    ps: &[Project],
    numbered: bool,
    date: &dyn Fn(SystemTime) -> String,
) -> Vec<String> {
    let mut lines = vec![title.to_string()];
    for (i, p) in ps.iter().enumerate() {
        let prefix = if numbered {
            format!("{:>2}. ", i + 1)
        } else {
            "  - ".to_string()
        };
        lines.push(format!(
            "{prefix}{} · 파일 {}개 · {}",
            p.name,
            p.files,
            date(p.last)
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Block, Stamp, render};
    use crate::testutil::TempDir;

    fn stamp() -> Stamp {
        Stamp::new(2026, 11, 4, 9, 5, 7)
    }

    fn write_turn(dir: &Path, name: &str, num: u64, topic: &str, text: &str) {
        fs::create_dir_all(dir).unwrap();
        let turn = [Block {
            kind: "asst".to_string(),
            text: text.to_string(),
        }];
        fs::write(dir.join(name), render(num, &stamp(), topic, &turn)).unwrap();
    }

    fn set_mtime(path: &Path, secs: u64) {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(t)
            .unwrap();
    }

    #[test]
    fn project_name_is_the_slugged_basename() {
        assert_eq!(project_name(Path::new("/src/My Project")), "My-Project");
        assert_eq!(project_name(Path::new("/src/.hidden")), "hidden");
        assert_eq!(project_name(Path::new("/src/...")), "misc");
        assert_eq!(
            project_dir(Path::new("/r/record"), Path::new("/src/app")),
            PathBuf::from("/r/record/app")
        );
    }

    #[test]
    fn legacy_names_match_the_old_pattern_only() {
        assert!(is_legacy("261104-tui-001.md"));
        assert!(is_legacy("261104-a-b-1234.md"));
        assert!(!is_legacy("261104--001.md"));
        assert!(!is_legacy("261104-tui-01.md"));
        assert!(!is_legacy("26110-tui-001.md"));
        assert!(!is_legacy("261104-tui-001.txt"));
        assert!(!is_legacy("app-261104-tui-001.md"));
    }

    #[test]
    fn migrate_root_moves_legacy_files_into_owner_folder() {
        let tmp = TempDir::new("rf-migrate");
        let root = tmp.path().join("myrepo").join("record");
        write_turn(&root, "261104-tui-001.md", 1, "tui", "a");
        write_turn(&root.join("kept"), "261104-x-001.md", 2, "x", "b");
        migrate_root(&root).unwrap();
        assert!(root.join("myrepo").join("261104-tui-001.md").is_file());
        assert!(!root.join("261104-tui-001.md").exists());
        assert!(root.join("kept").join("261104-x-001.md").is_file());
    }

    #[test]
    fn listing_skips_hidden_and_empty_folders_newest_first() {
        let tmp = TempDir::new("rf-list");
        let root = tmp.path().join("record");
        write_turn(&root.join("old"), "261104-a-001.md", 1, "a", "one");
        write_turn(&root.join("new"), "261104-b-001.md", 1, "b", "two");
        write_turn(&root.join("new"), "261104-b-002.md", 2, "b", "three");
        write_turn(&root.join(".hidden"), "261104-h-001.md", 1, "h", "x");
        fs::create_dir_all(root.join("empty")).unwrap();
        set_mtime(&root.join("old").join("261104-a-001.md"), 1_000);
        set_mtime(&root.join("new").join("261104-b-001.md"), 2_000);
        set_mtime(&root.join("new").join("261104-b-002.md"), 3_000);

        let ps = list_projects(&root).unwrap();
        let names: Vec<&str> = ps.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["new", "old"]);
        assert_eq!(ps[0].files, 2);
        assert_eq!(
            ps[0].last,
            SystemTime::UNIX_EPOCH + Duration::from_secs(3_000)
        );
        assert_eq!(ps[1].files, 1);
    }

    #[test]
    fn listing_a_missing_root_is_empty() {
        let tmp = TempDir::new("rf-missing");
        assert!(list_projects(&tmp.path().join("none")).unwrap().is_empty());
    }

    #[test]
    fn filter_matches_name_topic_or_text_case_insensitively() {
        let tmp = TempDir::new("rf-filter");
        let root = tmp.path().join("record");
        write_turn(&root.join("alpha"), "261104-a-001.md", 1, "Parser", "plain");
        write_turn(
            &root.join("beta"),
            "261104-b-001.md",
            1,
            "misc",
            "Needle here",
        );
        write_turn(&root.join("gamma"), "261104-g-001.md", 1, "misc", "nothing");
        let ps = list_projects(&root).unwrap();

        let by_name = filter_projects(&ps, "ALP").unwrap();
        assert_eq!(
            by_name.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["alpha"]
        );
        let by_topic = filter_projects(&ps, "parse").unwrap();
        assert_eq!(
            by_topic.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["alpha"]
        );
        let by_text = filter_projects(&ps, "needle").unwrap();
        assert_eq!(
            by_text.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["beta"]
        );
        assert!(filter_projects(&ps, "zzz").unwrap().is_empty());
    }

    #[test]
    fn load_project_gives_all_files_in_rec_order() {
        let tmp = TempDir::new("rf-load");
        let dir = tmp.path().join("p");
        write_turn(&dir, "261104-b-002.md", 2, "b", "second");
        write_turn(&dir, "261104-a-001.md", 1, "a", "first");
        write_turn(&dir, "261104-c-003.md", 3, "c", "third");
        let entries = load_project(&dir).unwrap();
        let nums: Vec<u64> = entries.iter().map(|e| e.num).collect();
        assert_eq!(nums, [1, 2, 3]);
        assert_eq!(entries[0].text, "first");
    }

    #[test]
    fn project_lines_numbered_and_plain() {
        let p = Project {
            name: "app".to_string(),
            dir: PathBuf::from("/r/app"),
            files: 2,
            last: SystemTime::UNIX_EPOCH,
        };
        let date = |_: SystemTime| "2026-11-04".to_string();
        let numbered = project_lines("title", std::slice::from_ref(&p), true, &date);
        assert_eq!(numbered[0], "title");
        assert_eq!(numbered[1], " 1. app · 파일 2개 · 2026-11-04");
        let plain = project_lines("title", std::slice::from_ref(&p), false, &date);
        assert_eq!(plain[1], "  - app · 파일 2개 · 2026-11-04");
    }
}
