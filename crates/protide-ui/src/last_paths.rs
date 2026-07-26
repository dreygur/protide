//! Persists the last-used directory for each file dialog across sessions.
//! Stored as a JSON map in ~/.config/protide/last_paths.json.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("protide").join("last_paths.json"))
}

/// Read the map at `file`. A missing, unreadable or malformed file yields an
/// empty map - never an error and never a panic.
fn load_from(file: &Path) -> HashMap<String, PathBuf> {
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(_) => return HashMap::new(),
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_to(file: &Path, map: &HashMap<String, PathBuf>) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(map) {
        let _ = std::fs::write(file, text);
    }
}

fn load() -> HashMap<String, PathBuf> {
    config_path().map(|p| load_from(&p)).unwrap_or_default()
}

fn save(map: &HashMap<String, PathBuf>) {
    let Some(path) = config_path() else { return };
    save_to(&path, map);
}

/// The directory to remember for `path`: itself when it is a directory,
/// otherwise its parent. `None` when neither yields a usable directory.
fn dir_of(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        return Some(path.to_path_buf());
    }
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

/// Returns the last directory used for `key`, or `None` if never set.
pub fn last_dir(key: &str) -> Option<PathBuf> {
    load().remove(key)
}

/// Records the directory of `path` as the last used for `key`.
pub fn save_last_dir(key: &str, path: &Path) {
    let Some(dir) = dir_of(path) else { return };
    let mut map = load();
    map.insert(key.to_string(), dir);
    save(&map);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn round_trips_through_disk() {
        let tmp = TempDir::new("last-paths");
        let file = tmp.join("last_paths.json");
        let mut map = HashMap::new();
        map.insert("open_folder".to_string(), PathBuf::from("/a/b"));
        save_to(&file, &map);
        assert_eq!(load_from(&file), map);
    }

    #[test]
    fn creates_missing_parent_directories() {
        let tmp = TempDir::new("last-paths-mkdir");
        let file = tmp.join("nested").join("deep").join("last_paths.json");
        save_to(&file, &HashMap::new());
        assert!(file.exists(), "save_to must create intermediate dirs");
    }

    #[test]
    fn missing_file_loads_empty() {
        let tmp = TempDir::new("last-paths-missing");
        assert!(load_from(&tmp.join("nope.json")).is_empty());
    }

    #[test]
    fn corrupt_file_loads_empty_without_panicking() {
        let tmp = TempDir::new("last-paths-corrupt");
        for (name, body) in [
            ("garbage.json", "not json at all"),
            ("truncated.json", "{\"open_folder\": \"/a/b"),
            ("empty.json", ""),
            ("wrong-schema.json", "[1, 2, 3]"),
            ("wrong-value.json", "{\"open_folder\": 42}"),
            ("nul.json", "\0\0\0"),
        ] {
            let file = tmp.write(name, body);
            assert!(load_from(&file).is_empty(), "{name} should load as empty");
        }
    }

    #[test]
    fn a_directory_is_remembered_as_itself() {
        let tmp = TempDir::new("last-paths-dir");
        assert_eq!(dir_of(tmp.path()), Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn a_file_is_remembered_as_its_parent() {
        let tmp = TempDir::new("last-paths-file");
        let file = tmp.write("req.http", "GET /");
        assert_eq!(dir_of(&file), Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn a_nonexistent_path_falls_back_to_its_parent() {
        let tmp = TempDir::new("last-paths-new");
        // A "Save as" target that has not been written yet still has a usable dir.
        assert_eq!(
            dir_of(&tmp.join("new.http")),
            Some(tmp.path().to_path_buf())
        );
    }

    // FIXED: `dir_of` used to hand back the *empty* path for a bare relative
    // filename, because `Path::new("x.http").parent()` is `Some("")`. That empty
    // path was then persisted and later fed to `AsyncFileDialog::set_directory`,
    // pointing the next dialog at nothing instead of falling back to $HOME.
    #[test]
    fn a_bare_filename_has_no_usable_directory() {
        assert_eq!(dir_of(Path::new("request.http")), None);
    }

    #[test]
    fn the_filesystem_root_has_no_parent() {
        assert_eq!(dir_of(Path::new("/")), Some(PathBuf::from("/")));
    }
}
