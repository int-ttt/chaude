//! Checksum verification and single-binary extraction (Go update.verify / sha256File / extract*).

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::Error;

/// Checks the sha256 of `file` against the line for `name` in the checksums file ("<hex>  <name>" or "<hex> *<name>").
pub fn verify(file: &Path, name: &str, sums_path: &Path) -> Result<(), Error> {
    let sums = fs::read_to_string(sums_path).map_err(|e| Error::Failed(e.to_string()))?;
    let mut want = String::new();
    for line in sums.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() >= 2 && f[1].strip_prefix('*').unwrap_or(f[1]) == name {
            want = f[0].to_lowercase();
            break;
        }
    }
    if want.is_empty() {
        return Err(Error::Failed(format!(
            "{name} is not listed in checksums.txt"
        )));
    }
    let got = sha256_file(file).map_err(|e| Error::Failed(e.to_string()))?;
    if got != want {
        return Err(Error::Failed(format!("checksum mismatch for {name}")));
    }
    Ok(())
}

/// Lowercase hex sha256 of a file, streamed.
pub fn sha256_file(file: &Path) -> io::Result<String> {
    let mut f = File::open(file)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Writes the entry named `bin` from `archive` to `dst`. Only the base name is matched; nothing else is written.
pub fn extract(archive: &Path, bin: &str, dst: &Path) -> Result<(), Error> {
    let is_zip = archive.to_string_lossy().ends_with(".zip");
    if is_zip {
        extract_zip(archive, bin, dst)
    } else {
        extract_tar_gz(archive, bin, dst)
    }
}

fn extract_tar_gz(archive: &Path, bin: &str, dst: &Path) -> Result<(), Error> {
    let f = File::open(archive).map_err(io_err)?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut ar = tar::Archive::new(gz);
    for entry in ar.entries().map_err(io_err)? {
        let mut entry = entry.map_err(io_err)?;
        // Go checks Typeflag == TypeReg: directories, links and devices never match.
        if entry.header().entry_type() != tar::EntryType::Regular {
            continue;
        }
        let name = entry.path().map_err(io_err)?.to_string_lossy().into_owned();
        if base_name(&name) == bin {
            return write_executable(dst, &mut entry);
        }
    }
    Err(Error::Failed(format!("{bin} not found in archive")))
}

fn extract_zip(archive: &Path, bin: &str, dst: &Path) -> Result<(), Error> {
    let f = File::open(archive).map_err(io_err)?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| Error::Failed(e.to_string()))?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| Error::Failed(e.to_string()))?;
        let name = file
            .name()
            .map_err(|e| Error::Failed(e.to_string()))?
            .into_owned();
        if file.is_dir() || base_name(&name) != bin {
            continue;
        }
        return write_executable(dst, &mut file);
    }
    Err(Error::Failed(format!("{bin} not found in archive")))
}

/// Go path.Base for archive member names: last '/' element, ignoring trailing slashes.
fn base_name(name: &str) -> &str {
    let t = name.trim_end_matches('/');
    t.rsplit('/').next().unwrap_or(t)
}

/// Creates `dst` (mode 0755 on unix) and copies `src` into it.
pub fn write_executable(dst: &Path, src: &mut dyn Read) -> Result<(), Error> {
    let mut opts = fs::OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o755);
    }
    let mut out = opts.open(dst).map_err(io_err)?;
    io::copy(src, &mut out).map_err(io_err)?;
    Ok(())
}

fn io_err(e: io::Error) -> Error {
    Error::Failed(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::base_name;

    #[test]
    fn base_name_matches_go_path_base() {
        assert_eq!(base_name("chaude"), "chaude");
        assert_eq!(base_name("./dist/chaude"), "chaude");
        assert_eq!(base_name("dist/chaude/"), "chaude");
    }
}
