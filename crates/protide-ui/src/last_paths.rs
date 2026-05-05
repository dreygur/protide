//! Persists the last-used directory for each file dialog across sessions.
//! Stored as a JSON map in ~/.config/protide/last_paths.json.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("protide").join("last_paths.json"))
}

fn load() -> HashMap<String, PathBuf> {
    let path = match config_path() { Some(p) => p, None => return HashMap::new() };
    let text = match std::fs::read_to_string(&path) { Ok(t) => t, Err(_) => return HashMap::new() };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save(map: &HashMap<String, PathBuf>) {
    let Some(path) = config_path() else { return };
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    if let Ok(text) = serde_json::to_string(map) { let _ = std::fs::write(path, text); }
}

/// Returns the last directory used for `key`, or `None` if never set.
pub fn last_dir(key: &str) -> Option<PathBuf> {
    load().remove(key)
}

/// Records the directory of `path` as the last used for `key`.
pub fn save_last_dir(key: &str, path: &Path) {
    let dir = if path.is_dir() {
        path.to_path_buf()
    } else if let Some(parent) = path.parent() {
        parent.to_path_buf()
    } else {
        return;
    };
    let mut map = load();
    map.insert(key.to_string(), dir);
    save(&map);
}
