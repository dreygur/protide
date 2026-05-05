//! Persistent UI state (panel sizes, collapse states) stored as JSON.
//! File: ~/.config/protide/prefs.json

use std::collections::HashMap;
use std::path::PathBuf;

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("protide").join("prefs.json"))
}

fn load() -> HashMap<String, serde_json::Value> {
    let path = match config_path() { Some(p) => p, None => return HashMap::new() };
    let text = match std::fs::read_to_string(&path) { Ok(t) => t, Err(_) => return HashMap::new() };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save(map: &HashMap<String, serde_json::Value>) {
    let Some(path) = config_path() else { return };
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    if let Ok(text) = serde_json::to_string(map) { let _ = std::fs::write(path, text); }
}

pub fn get_f32(key: &str, default: f32) -> f32 {
    load().get(key).and_then(|v| v.as_f64()).map(|v| v as f32).unwrap_or(default)
}

pub fn set_f32(key: &str, value: f32) {
    let mut map = load();
    map.insert(key.to_string(), serde_json::Value::from(value as f64));
    save(&map);
}
