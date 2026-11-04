//! /resume record picker (Go record_turn.go recordResume and recordPick, record_folder.go, the reader half of
//! record.go, and the tui.go rpick keys and draw). The record root holds one folder per project; each folder
//! holds *.md record files. The picker is a pure overlay over RecordProject rows; readers take the root path.

use std::cmp::Reverse;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use chaude_core::record_folder;

use super::bar::clip;
use super::render::{FG_OFF, GREY_ON, ORANGE_ON};
use super::sessview::{stamp, wheel_sel};

// Go rpick title (tui.go draw).
pub const TITLE: &str = "Resume a project  (↑/↓ wheel or 1-9 select · enter load · esc cancel)";
// Go recordResume list title (bare /resume).

// One project folder under the record root (Go recordProject).
#[derive(Clone, Debug, PartialEq)]
pub struct RecordProject {
    pub name: String,
    pub dir: PathBuf,
    pub files: usize,
    // Newest file mod time in the folder.
    pub last: SystemTime,
}

// One block parsed back from a record file (Go recordEntry).
#[derive(Clone, Debug, PartialEq)]
pub struct RecordEntry {
    pub num: usize,
    pub kind: String,
    pub ts: String,
    pub topic: String,
    pub text: String,
}

struct RecordFile {
    name: String,
    last: usize,
    modified: SystemTime,
}

struct Header {
    num: usize,
    kind: String,
    ts: String,
    topic: String,
}

// Go recordHeaderRe: `^<!-- rec #(\d+) (\w+) (\S+) cat=(.*) -->\n`, applied to one line without its newline.
fn parse_header(line: &str) -> Option<Header> {
    let rest = line.strip_prefix("<!-- rec #")?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let num = rest[..digits].parse().ok()?;
    let rest = rest[digits..].strip_prefix(' ')?;
    let kind_len = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
        .count();
    if kind_len == 0 {
        return None;
    }
    let (kind, rest) = rest.split_at(kind_len);
    let rest = rest.strip_prefix(' ')?;
    let ts_len = rest.find(' ')?;
    if ts_len == 0 {
        return None;
    }
    let (ts, rest) = rest.split_at(ts_len);
    let topic = rest.strip_prefix(" cat=")?.strip_suffix(" -->")?;
    Some(Header {
        num,
        kind: kind.to_string(),
        ts: ts.to_string(),
        topic: topic.to_string(),
    })
}

// Every header in data as (line start, body start, header). A header must end with a newline (Go regex).
fn scan_headers(data: &str) -> Vec<(usize, usize, Header)> {
    let mut out = Vec::new();
    let mut pos = 0;
    for line in data.split_inclusive('\n') {
        if let Some(h) = line.strip_suffix('\n').and_then(parse_header) {
            out.push((pos, pos + line.len(), h));
        }
        pos += line.len();
    }
    out
}

// Go recordUnescapeRe: a body line of backslashes then "<!-- rec #" loses one backslash.
fn unescape(seg: &str) -> String {
    let mut out = String::with_capacity(seg.len());
    for line in seg.split_inclusive('\n') {
        let trimmed = line.trim_start_matches('\\');
        if trimmed.len() < line.len() && trimmed.starts_with("<!-- rec #") {
            out.push_str(&line[1..]);
        } else {
            out.push_str(line);
        }
    }
    out
}

// Go parseRecordEntries: blocks in file order. Body = text after the heading line, minus one leading and
// one trailing blank line.
pub fn parse_entries(data: &str) -> Vec<RecordEntry> {
    let heads = scan_headers(data);
    heads
        .iter()
        .enumerate()
        .map(|(i, (_, body, h))| {
            let end = heads.get(i + 1).map_or(data.len(), |n| n.0);
            let mut seg = &data[*body..end];
            if let Some(j) = seg.find('\n') {
                seg = &seg[j + 1..];
            }
            let seg = seg.strip_prefix('\n').unwrap_or(seg);
            let seg = seg.strip_suffix("\n\n").unwrap_or(seg);
            RecordEntry {
                num: h.num,
                kind: h.kind.clone(),
                ts: h.ts.clone(),
                topic: h.topic.clone(),
                text: unescape(seg),
            }
        })
        .collect()
}

// Go recordLegacyRe `^\d{6}-.+-\d{3,}\.md$`: an old date/topic file name sitting directly in the root.
fn is_legacy(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".md") else {
        return false;
    };
    let digits = stem.bytes().rev().take_while(u8::is_ascii_digit).count();
    let head = &stem[..stem.len() - digits];
    let b = stem.as_bytes();
    digits >= 3
        && head.len() >= 9
        && head.ends_with('-')
        && b[..6].iter().all(u8::is_ascii_digit)
        && b[6] == b'-'
}

// Go listDir: directory entries sorted by name, empty when the directory does not exist.
fn sorted_entries(dir: &Path) -> Result<Vec<std::fs::DirEntry>, String> {
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    };
    let mut ents: Vec<std::fs::DirEntry> = rd.flatten().collect();
    ents.sort_by_key(|e| e.file_name());
    Ok(ents)
}

// Go migrateRecordRoot: moves legacy files out of root into one folder named after root's parent.
pub fn migrate_root(root: &Path) -> Result<(), String> {
    let legacy: Vec<String> = sorted_entries(root)?
        .iter()
        .filter(|e| e.file_type().is_ok_and(|t| !t.is_dir()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| is_legacy(n))
        .collect();
    if legacy.is_empty() {
        return Ok(());
    }
    let parent = root.parent().unwrap_or(Path::new("."));
    let dst = root.join(record_folder::folder_name(parent));
    std::fs::create_dir_all(&dst).map_err(|e| e.to_string())?;
    for name in legacy {
        std::fs::rename(root.join(&name), dst.join(&name)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Go listRecordFiles: the *.md files of dir in rec-number order of their last turn.
fn list_files(dir: &Path) -> Result<Vec<RecordFile>, String> {
    let mut out = Vec::new();
    for e in sorted_entries(dir)? {
        let name = e.file_name().to_string_lossy().into_owned();
        if e.file_type().is_ok_and(|t| t.is_dir()) || !name.ends_with(".md") {
            continue;
        }
        let path = dir.join(&name);
        let data = std::fs::read(&path).map_err(|e| e.to_string())?;
        let last = scan_headers(&String::from_utf8_lossy(&data))
            .last()
            .map_or(0, |h| h.2.num);
        let modified = e
            .metadata()
            .and_then(|m| m.modified())
            .map_err(|e| e.to_string())?;
        out.push(RecordFile {
            name,
            last,
            modified,
        });
    }
    out.sort_by_key(|f| f.last);
    Ok(out)
}

// Go listRecordProjects: the project folders under root, newest write first. Legacy files are migrated first.
// Folders with no record file are skipped; dot folders are hidden.
pub fn list_projects(root: &Path) -> Result<Vec<RecordProject>, String> {
    migrate_root(root)?;
    let mut out = Vec::new();
    for e in sorted_entries(root)? {
        let name = e.file_name().to_string_lossy().into_owned();
        if !e.file_type().is_ok_and(|t| t.is_dir()) || name.starts_with('.') {
            continue;
        }
        let dir = e.path();
        let files = list_files(&dir)?;
        if files.is_empty() {
            continue;
        }
        let last = files.iter().map(|f| f.modified).max().unwrap_or(UNIX_EPOCH);
        out.push(RecordProject {
            name,
            dir,
            files: files.len(),
            last,
        });
    }
    out.sort_by_key(|p| Reverse(p.last));
    Ok(out)
}

// Go loadRecordProject: every block of a folder as one history, in rec-number order.
pub fn load_project(dir: &Path) -> Result<Vec<RecordEntry>, String> {
    let mut out = Vec::new();
    for f in list_files(dir)? {
        let data = std::fs::read(dir.join(&f.name)).map_err(|e| e.to_string())?;
        out.extend(parse_entries(&String::from_utf8_lossy(&data)));
    }
    out.sort_by_key(|e| e.num);
    Ok(out)
}

// Go filterRecordProjects: folders whose name, or any block topic or text, contains q (case-insensitive).
pub fn filter_projects(projects: &[RecordProject], q: &str) -> Result<Vec<RecordProject>, String> {
    let q = q.to_lowercase();
    let mut out = Vec::new();
    for p in projects {
        if p.name.to_lowercase().contains(&q) {
            out.push(p.clone());
            continue;
        }
        let hit = load_project(&p.dir)?
            .iter()
            .any(|e| e.text.to_lowercase().contains(&q) || e.topic.to_lowercase().contains(&q));
        if hit {
            out.push(p.clone());
        }
    }
    Ok(out)
}

// Go recordPick: the folders the TUI picker offers. None means arg loads one folder directly (or is a
// number, or fails in resume). Empty arg gives every folder; a multi-hit query gives the hits.
pub fn pick(root: &Path, arg: &str) -> Result<Option<Vec<RecordProject>>, String> {
    let projects = list_projects(root)?;
    if projects.is_empty() {
        return Ok(None);
    }
    if arg.is_empty() {
        return Ok(Some(projects));
    }
    if arg.parse::<i64>().is_ok() {
        return Ok(None);
    }
    let matches = filter_projects(&projects, arg)?;
    Ok(if matches.len() < 2 {
        None
    } else {
        Some(matches)
    })
}

// The date part of a record project row (Go row date), used by core record_turn::resume.
pub fn list_date(t: SystemTime) -> String {
    stamp(t, true).chars().take(10).collect()
}

// Go recordProjectLines row text: "<name> · 파일 <n>개 · <yyyy-mm-dd>".
fn row_text(p: &RecordProject) -> String {
    let date: String = stamp(p.last, true).chars().take(10).collect();
    format!("{} · 파일 {}개 · {date}", p.name, p.files)
}

// Result of a key press on the record picker.
#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    // esc, q or ctrl+c: close the picker.
    Close,
    // enter: close the picker and load this folder (Go loadRecordPick).
    Load(RecordProject),
}

// Go rpick and rpickSel: the open picker and its cursor.
#[derive(Debug, Default)]
pub struct RecPicker {
    pub projects: Vec<RecordProject>,
    pub sel: usize,
}

impl RecPicker {
    pub fn new(projects: Vec<RecordProject>) -> Self {
        RecPicker { projects, sel: 0 }
    }

    // Go wheelSel on rpickSel (shared with the /view picker).
    pub fn wheel(&mut self, up: bool) {
        wheel_sel(&mut self.sel, self.projects.len(), up);
    }

    // Go rpick key branch. Up/k and down/j wrap; 1-9 move the cursor to that row; enter loads; esc/q/ctrl+c close.
    pub fn key(&mut self, key: KeyEvent) -> Action {
        let n = self.projects.len();
        let plain = key.modifiers == KeyModifiers::NONE;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') if plain => wheel_sel(&mut self.sel, n, true),
            KeyCode::Down | KeyCode::Char('j') if plain => wheel_sel(&mut self.sel, n, false),
            KeyCode::Esc | KeyCode::Char('q') if plain => return Action::Close,
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return Action::Close,
            KeyCode::Enter => {
                if let Some(p) = self.projects.get(self.sel) {
                    return Action::Load(p.clone());
                }
            }
            KeyCode::Char(c)
                if plain && ('1'..='9').contains(&c) && ((c as u8 - b'1') as usize) < n =>
            {
                self.sel = (c as u8 - b'1') as usize;
            }
            _ => {}
        }
        Action::None
    }

    // Go rpick draw: a title row, then rows from a window around sel, exactly hh rows total.
    pub fn lines(&self, width: usize, hh: usize) -> Vec<String> {
        if hh == 0 {
            return Vec::new();
        }
        let n = self.projects.len() as isize;
        let from = (self.sel as isize - (hh / 2) as isize)
            .min(n - (hh as isize - 2))
            .max(0) as usize;
        let mut l = vec![format!("{ORANGE_ON}{TITLE}{FG_OFF}")];
        for (i, p) in self.projects.iter().enumerate().skip(from) {
            if l.len() >= hh - 1 {
                break;
            }
            let t = format!("{:>2}) {}", i + 1, row_text(p));
            l.push(if i == self.sel {
                format!("{ORANGE_ON}{}{FG_OFF}", clip(&format!("> {t}"), width))
            } else {
                format!("{GREY_ON}{}{FG_OFF}", clip(&format!("  {t}"), width))
            });
        }
        l.resize(hh, String::new());
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST_TITLE: &str = "record projects (/resume <번호|이름|검색어> opens one):";

    // The core resume with the picker's date format.
    fn resume(
        root: &Path,
        arg: &str,
    ) -> Result<(Vec<String>, Vec<chaude_core::record::Block>), String> {
        chaude_core::record_turn::resume(root, arg, &list_date)
    }
    use std::time::Duration;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    // Writes one project folder with one record file holding a user "q" and an assistant answer.
    fn write_project(root: &Path, name: &str, answer: &str) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let body = format!(
            "<!-- rec #0001 user 2026-10-31T10:00 cat={name} -->\n## #0001 · 10:00 · {name}\n\nq\n\n\
             <!-- rec #0001 asst 2026-10-31T10:00 cat={name} -->\n## #0001 · 10:00 · {name}\n\n{answer}\n\n"
        );
        let f = dir.join("261031-100000-0001.md");
        std::fs::write(&f, body).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&f)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(3600))
            .unwrap();
    }

    fn fixture(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("chaude-records-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        write_project(&root, "alpha", "배포 방법");
        write_project(&root, "beta", "정규식 설명");
        write_project(&root, "gamma", "배포 체크리스트");
        root
    }

    // Go TestRecordPickCandidates: bare and multi-hit open the picker; single hit and number load directly.
    #[test]
    fn pick_candidates_follow_go() {
        let root = fixture("pick");
        assert_eq!(pick(&root, "").unwrap().map(|v| v.len()), Some(3), "bare");
        assert_eq!(
            pick(&root, "배포").unwrap().map(|v| v.len()),
            Some(2),
            "multi-hit"
        );
        assert_eq!(
            pick(&root, "정규식").unwrap(),
            None,
            "single hit loads directly"
        );
        assert_eq!(pick(&root, "2").unwrap(), None, "number loads directly");
        let _ = std::fs::remove_dir_all(&root);
    }

    // Go TestRecordPickerKeysWheelEnter: arrows and wheel move the cursor, enter loads that folder and closes,
    // esc cancels.
    #[test]
    fn picker_keys_wheel_enter_follow_go() {
        let root = fixture("keys");
        let ps = pick(&root, "").unwrap().unwrap();
        let mut p = RecPicker::new(ps.clone());
        assert_eq!(p.key(press(KeyCode::Down)), Action::None);
        assert_eq!(p.sel, 1, "down");
        p.key(press(KeyCode::Down));
        assert_eq!(p.sel, 2);
        p.key(press(KeyCode::Down));
        assert_eq!(p.sel, 0, "down wraps");
        p.wheel(false);
        assert_eq!(p.sel, 1, "wheel down");
        p.wheel(true);
        assert_eq!(p.sel, 0, "wheel up");
        p.key(press(KeyCode::Char('2')));
        assert_eq!(p.sel, 1, "digit selects row");
        p.key(press(KeyCode::Char('9')));
        assert_eq!(p.sel, 1, "digit past the list is ignored");
        p.key(press(KeyCode::Up));
        let Action::Load(chosen) = p.key(press(KeyCode::Enter)) else {
            panic!("enter must load the selected folder");
        };
        assert_eq!(chosen, ps[0]);
        let loaded = load_project(&chosen.dir).unwrap();
        assert_eq!(
            loaded.len(),
            2 * chosen.files,
            "every block of the chosen folder"
        );
        assert_eq!(RecPicker::new(ps).key(press(KeyCode::Esc)), Action::Close);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn picker_draws_window_with_title_and_padding() {
        let root = fixture("draw");
        let p = RecPicker::new(pick(&root, "").unwrap().unwrap());
        let rows = p.lines(80, 5);
        assert_eq!(rows.len(), 5);
        assert!(rows[0].contains("Resume a project"));
        assert!(rows[1].contains(" 1) alpha") || rows[1].contains("> ") || rows[1].contains("1) "));
        assert_eq!(rows[4], "", "padded to the body height");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resume_lists_then_rejects_bad_numbers() {
        let root = fixture("resume");
        let (lines, blocks) = resume(&root, "").unwrap();
        assert_eq!(lines[0], LIST_TITLE);
        assert_eq!(lines.len(), 4, "title plus three folders");
        assert!(blocks.is_empty());
        assert_eq!(
            resume(&root, "9").err().as_deref(),
            Some("no project 9 (1-3)")
        );
        assert_eq!(
            resume(&root, "nope").err().as_deref(),
            Some("no project matches \"nope\"")
        );
        let (_, one) = resume(&root, "정규식").unwrap();
        assert_eq!(one.len(), 2, "user and asst blocks");
        assert!(one.iter().any(|e| e.text == "정규식 설명"), "{one:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn parse_entries_unescapes_body_header_lines() {
        let data = "<!-- rec #0002 asst 2026-10-31T10:00 cat=t -->\n## #0002 · 10:00 · t\n\n\\<!-- rec #0009 x -->\nplain\n\n";
        let e = parse_entries(data);
        assert_eq!(e.len(), 1);
        assert_eq!(
            (e[0].num, e[0].kind.as_str(), e[0].topic.as_str()),
            (2, "asst", "t")
        );
        assert_eq!(e[0].text, "<!-- rec #0009 x -->\nplain");
    }
}
