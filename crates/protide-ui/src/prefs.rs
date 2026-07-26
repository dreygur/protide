//! Persistent UI state (panel sizes, collapse states) stored as JSON.
//! File: ~/.config/protide/prefs.json

use std::collections::HashMap;
use std::path::{Path, PathBuf};

type Prefs = HashMap<String, serde_json::Value>;

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("protide").join("prefs.json"))
}

/// Read the prefs at `file`. A missing, unreadable or malformed file yields an
/// empty map - never an error and never a panic.
fn load_from(file: &Path) -> Prefs {
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(_) => return Prefs::new(),
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_to(file: &Path, map: &Prefs) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(map) {
        let _ = std::fs::write(file, text);
    }
}

fn load() -> Prefs {
    config_path().map(|p| load_from(&p)).unwrap_or_default()
}

fn save(map: &Prefs) {
    let Some(path) = config_path() else { return };
    save_to(&path, map);
}

/// `key` as an `f32`, falling back to `default` when absent or not a number.
fn f32_in(map: &Prefs, key: &str, default: f32) -> f32 {
    map.get(key)
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or(default)
}

pub fn get_f32(key: &str, default: f32) -> f32 {
    f32_in(&load(), key, default)
}

pub fn set_f32(key: &str, value: f32) {
    let mut map = load();
    map.insert(key.to_string(), serde_json::Value::from(value as f64));
    save(&map);
}

/// `key` as a `String`, or `None` when absent or not a string.
fn str_in(map: &Prefs, key: &str) -> Option<String> {
    map.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

pub fn get_string(key: &str) -> Option<String> {
    str_in(&load(), key)
}

pub fn set_string(key: &str, value: &str) {
    let mut map = load();
    map.insert(key.to_string(), serde_json::Value::from(value));
    save(&map);
}

pub fn remove(key: &str) {
    let mut map = load();
    map.remove(key);
    save(&map);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use serde_json::Value;

    fn prefs(pairs: &[(&str, Value)]) -> Prefs {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn round_trips_through_disk() {
        let tmp = TempDir::new("prefs");
        let file = tmp.join("prefs.json");
        let map = prefs(&[
            ("sidebar_width", Value::from(240.5)),
            ("last_workspace", Value::from("/w")),
        ]);
        save_to(&file, &map);
        let back = load_from(&file);
        assert_eq!(f32_in(&back, "sidebar_width", 0.0), 240.5);
        assert_eq!(str_in(&back, "last_workspace").as_deref(), Some("/w"));
    }

    #[test]
    fn creates_missing_parent_directories() {
        let tmp = TempDir::new("prefs-mkdir");
        let file = tmp.join("a").join("b").join("prefs.json");
        save_to(&file, &Prefs::new());
        assert!(file.exists(), "save_to must create intermediate dirs");
    }

    #[test]
    fn missing_file_loads_empty() {
        let tmp = TempDir::new("prefs-missing");
        assert!(load_from(&tmp.join("nope.json")).is_empty());
    }

    #[test]
    fn a_directory_in_place_of_the_file_loads_empty() {
        // read_to_string on a directory is an Err, not a panic.
        let tmp = TempDir::new("prefs-isdir");
        assert!(load_from(tmp.path()).is_empty());
    }

    #[test]
    fn corrupt_file_loads_empty_without_panicking() {
        let tmp = TempDir::new("prefs-corrupt");
        for (name, body) in [
            ("garbage.json", "}{"),
            ("truncated.json", "{\"sidebar_width\": 24"),
            ("empty.json", ""),
            ("whitespace.json", "   \n\t "),
            ("wrong-schema.json", "\"just a string\""),
            ("array.json", "[1,2,3]"),
            ("null.json", "null"),
            ("invalid-utf8-ish.json", "{\"k\": \"\u{fffd}\"}\u{0}"),
        ] {
            let file = tmp.write(name, body);
            assert!(load_from(&file).is_empty(), "{name} should load as empty");
        }
    }

    #[test]
    fn a_corrupt_file_still_yields_the_caller_defaults() {
        let tmp = TempDir::new("prefs-corrupt-defaults");
        let file = tmp.write("prefs.json", "not json");
        let map = load_from(&file);
        assert_eq!(f32_in(&map, "sidebar_width", 260.0), 260.0);
        assert_eq!(str_in(&map, "last_workspace"), None);
    }

    #[test]
    fn wrong_value_types_fall_back_to_the_default() {
        let map = prefs(&[
            ("num", Value::from("not a number")),
            ("text", Value::from(7)),
            ("nul", Value::Null),
            ("obj", serde_json::json!({"nested": 1})),
        ]);
        assert_eq!(f32_in(&map, "num", 1.5), 1.5);
        assert_eq!(f32_in(&map, "nul", 1.5), 1.5);
        assert_eq!(f32_in(&map, "obj", 1.5), 1.5);
        assert_eq!(f32_in(&map, "absent", 1.5), 1.5);
        assert_eq!(str_in(&map, "text"), None);
        assert_eq!(str_in(&map, "nul"), None);
        assert_eq!(str_in(&map, "absent"), None);
    }

    #[test]
    fn non_finite_widths_never_reach_disk_as_invalid_json() {
        // serde_json cannot represent NaN/Infinity, so `Value::from` maps them to
        // null; the getter must then hand back the default rather than a NaN that
        // would silently poison every layout computation downstream.
        let tmp = TempDir::new("prefs-nan");
        let file = tmp.join("prefs.json");
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let map = prefs(&[("w", Value::from(bad as f64))]);
            save_to(&file, &map);
            let back = load_from(&file);
            assert_eq!(f32_in(&back, "w", 300.0), 300.0, "bad value {bad}");
        }
    }

    #[test]
    fn f64_values_are_narrowed_to_f32() {
        let map = prefs(&[("w", Value::from(1234.5678_f64))]);
        assert_eq!(f32_in(&map, "w", 0.0), 1234.5678_f64 as f32);
    }

    #[test]
    fn unicode_keys_and_values_survive_a_round_trip() {
        let tmp = TempDir::new("prefs-unicode");
        let file = tmp.join("prefs.json");
        let map = prefs(&[("路径 🗂", Value::from("/tmp/日本語/e\u{0301}mo🎉ji"))]);
        save_to(&file, &map);
        assert_eq!(
            str_in(&load_from(&file), "路径 🗂").as_deref(),
            Some("/tmp/日本語/e\u{0301}mo🎉ji")
        );
    }

    #[test]
    fn saving_replaces_rather_than_appends() {
        // A shorter payload must not leave the tail of the previous one behind.
        let tmp = TempDir::new("prefs-replace");
        let file = tmp.join("prefs.json");
        save_to(
            &file,
            &prefs(&[("a", Value::from(1.0)), ("b", Value::from(2.0))]),
        );
        save_to(&file, &prefs(&[("a", Value::from(1.0))]));
        let back = load_from(&file);
        assert_eq!(back.len(), 1, "stale keys must not survive a save");
        assert!(!back.contains_key("b"));
    }
}
