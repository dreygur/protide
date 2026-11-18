//! Per workspace state files kept under `<workspace>/.protide/`.

use serde::Serialize;
use serde::de::DeserializeOwned;
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

/// Write `value` as JSON to `path`, creating parent directories. The write goes
/// through a sibling temp file and a rename so a crash never leaves a truncated
/// file that would load as defaults and then overwrite the user's data.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
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

    #[test]
    fn missing_or_corrupt_files_load_as_none() {
        let tmp = TempDir::new("storage_corrupt");
        assert_eq!(load_json::<Vec<i32>>(&tmp.path().join("nope.json")), None);
        let bad = tmp.write("bad.json", b"{not json");
        assert_eq!(load_json::<Vec<i32>>(&bad), None);
    }
}
