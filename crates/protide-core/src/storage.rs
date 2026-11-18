//! Per workspace state files kept under `<workspace>/.protide/`.

use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Directory inside a workspace that holds Protide's own state files.
pub const DATA_DIR: &str = ".protide";

/// Path of `name` inside the workspace's data directory.
pub fn data_file(workspace: &Path, name: &str) -> PathBuf {
    workspace.join(DATA_DIR).join(name)
}

/// Read JSON from `path`. A missing, unreadable or malformed file yields `None`.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// A workspace may come from someone else (a cloned repo), so a planted
/// symlink must not redirect our writes outside it.
fn refuse_symlink(path: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(io::Error::other(format!(
            "refusing to write through symlink {}",
            path.display()
        ))),
        _ => Ok(()),
    }
}

/// Create `path` exclusively, readable only by the owner on Unix since these
/// files can hold secrets.
fn create_private(path: &Path) -> io::Result<std::fs::File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// Write `value` as JSON to `path`, creating parent directories. The write goes
/// through a unique sibling temp file and a rename so a crash never leaves a
/// truncated file that would load as defaults and then overwrite the user's data.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("path has no parent"))?;
    refuse_symlink(dir)?;
    std::fs::create_dir_all(dir)?;
    refuse_symlink(path)?;
    let text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    let tmp = dir.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let result = create_private(&tmp)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .and_then(|()| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn round_trips_and_creates_the_data_dir() {
        let tmp = TempDir::new("storage_round_trip");
        let file = data_file(tmp.path(), "x.json");
        save_json(&file, &vec![1, 2, 3]).unwrap();
        assert_eq!(load_json::<Vec<i32>>(&file), Some(vec![1, 2, 3]));
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_data_dir_is_refused_and_its_target_untouched() {
        let tmp = TempDir::new("storage_symlink_dir");
        let elsewhere = tmp.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        let ws = tmp.path().join("ws");
        std::fs::create_dir(&ws).unwrap();
        std::os::unix::fs::symlink(&elsewhere, ws.join(DATA_DIR)).unwrap();

        assert!(save_json(&data_file(&ws, "x.json"), &1).is_err());
        assert_eq!(std::fs::read_dir(&elsewhere).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_target_file_is_refused_and_its_target_untouched() {
        let tmp = TempDir::new("storage_symlink_file");
        let victim = tmp.write("victim.txt", b"keep");
        let file = data_file(tmp.path(), "x.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&victim, &file).unwrap();

        assert!(save_json(&file, &1).is_err());
        assert_eq!(std::fs::read(&victim).unwrap(), b"keep");
    }

    #[cfg(unix)]
    #[test]
    fn saved_files_are_private_to_the_owner() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new("storage_mode");
        let file = data_file(tmp.path(), "x.json");
        save_json(&file, &1).unwrap();
        let mode = std::fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn missing_or_corrupt_files_load_as_none() {
        let tmp = TempDir::new("storage_corrupt");
        assert_eq!(load_json::<Vec<i32>>(&tmp.path().join("nope.json")), None);
        let bad = tmp.write("bad.json", b"{not json");
        assert_eq!(load_json::<Vec<i32>>(&bad), None);
    }
}
