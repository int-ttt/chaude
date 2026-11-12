// Guards the hook copies: crates/core/src/settings/hooks/ is the source of truth,
// .claude/hooks/ must hold byte-identical copies of every *.py file.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn py_names(dir: &Path) -> BTreeSet<String> {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
    entries
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "py"))
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn hook_copies_are_byte_identical() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let src: PathBuf = root.join("crates/core/src/settings/hooks");
    let copy: PathBuf = root.join(".claude/hooks");

    let src_names = py_names(&src);
    let copy_names = py_names(&copy);

    let only_src: Vec<&String> = src_names.difference(&copy_names).collect();
    let only_copy: Vec<&String> = copy_names.difference(&src_names).collect();
    assert!(
        only_src.is_empty() && only_copy.is_empty(),
        "hook name sets differ: only in {} = {:?}; only in {} = {:?}",
        src.display(),
        only_src,
        copy.display(),
        only_copy
    );

    for name in &src_names {
        let a = fs::read(src.join(name)).unwrap_or_else(|e| panic!("read {name} (src): {e}"));
        let b = fs::read(copy.join(name)).unwrap_or_else(|e| panic!("read {name} (copy): {e}"));
        assert!(
            a == b,
            "hook copy differs from source: {name} ({} vs {} bytes)",
            a.len(),
            b.len()
        );
    }
}
