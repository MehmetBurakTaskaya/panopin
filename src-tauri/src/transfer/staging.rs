//! The staging folder (`%APPDATA%\Panopin\staging`, D-042): files handed to other apps, such as
//! several copied images or a drag. Names never come from content: `Panopin-<date>-<n>.<ext>`.
//! The folder is emptied when Panopin starts and when it quits.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::Result;

pub struct Staging {
    dir: PathBuf,
}

/// One file to hand out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Staged {
    /// A copy of a stored file, keeping its format.
    Copy(PathBuf),
    /// Content made for this hand-out (an edited picture, a text, a shortcut) with its extension.
    Bytes(Vec<u8>, &'static str),
}

impl Staging {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn clean(&self) {
        if let Err(err) = fs::remove_dir_all(&self.dir) {
            if err.kind() != std::io::ErrorKind::NotFound {
                log::warn!("could not empty the staging folder: {err}");
            }
        }
    }

    /// Writes each item under a fresh neutral name and returns the paths in the same order.
    pub fn stage(&self, items: &[Staged]) -> Result<Vec<PathBuf>> {
        write_neutral(&self.dir, items)
    }
}

/// Writes items into `dir` as `Panopin-<date>-<n>.<ext>`, never overwriting a file that is there
/// already. Used for the staging folder and for exports.
pub fn write_neutral(dir: &Path, items: &[Staged]) -> Result<Vec<PathBuf>> {
    if items.is_empty() {
        return Ok(Vec::new());
    }
    fs::create_dir_all(dir)?;
    let date = local_date();
    let mut paths = Vec::with_capacity(items.len());
    let mut n = 0u32;
    for item in items {
        let ext = match item {
            Staged::Copy(source) => source
                .extension()
                .map(|ext| ext.to_string_lossy().into_owned())
                .unwrap_or_default(),
            Staged::Bytes(_, ext) => (*ext).to_owned(),
        };
        let path = loop {
            n += 1;
            let candidate = dir.join(format!("Panopin-{date}-{n:03}.{ext}"));
            if !candidate.exists() {
                break candidate;
            }
        };
        match item {
            Staged::Copy(source) => copy_atomically(source, &path)?,
            Staged::Bytes(bytes, _) => write_atomically(&path, bytes)?,
        }
        paths.push(path);
    }
    Ok(paths)
}

fn tmp_of(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}

/// A copy that appears under its final name only when complete.
fn copy_atomically(source: &Path, path: &Path) -> Result<()> {
    let tmp = tmp_of(path);
    fs::copy(source, &tmp)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = tmp_of(path);
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Today's date as `YYYY-MM-DD` in local time.
#[cfg(windows)]
fn local_date() -> String {
    // SAFETY: GetLocalTime only fills in the returned structure.
    let now = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    format!("{:04}-{:02}-{:02}", now.wYear, now.wMonth, now.wDay)
}

/// Today's date as `YYYY-MM-DD` (UTC until the macOS port adds local time).
#[cfg(not(windows))]
fn local_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_neutral_unique_and_cleaned() {
        let dir = tempfile::tempdir().unwrap();
        let staging = Staging::new(dir.path().join("staging"));
        let source = |name: &str| {
            let path = dir.path().join(name);
            fs::write(&path, name).unwrap();
            path
        };
        let (a, c) = (source("a.png"), source("c.jpg"));
        let first = staging
            .stage(&[Staged::Copy(a.clone()), Staged::Bytes(vec![1, 2], "png")])
            .unwrap();
        let second = staging.stage(&[Staged::Copy(c)]).unwrap();
        assert_eq!(fs::read(&first[1]).unwrap(), vec![1, 2]);
        assert_eq!(fs::read(&first[0]).unwrap(), fs::read(&a).unwrap());
        let names: Vec<String> = first
            .iter()
            .chain(&second)
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names[0].ends_with("-001.png") && names[1].ends_with("-002.png"));
        assert!(names[2].ends_with("-001.jpg"), "{names:?}");
        assert!(names
            .iter()
            .all(|n| n.starts_with("Panopin-") && n.len() == "Panopin-2026-10-29-001.png".len()));
        staging.clean();
        assert!(!dir.path().join("staging").exists());
    }
}
