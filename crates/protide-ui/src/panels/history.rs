//! Request history storage and management, persisted per workspace in
//! `<workspace>/.protide/history.json`.

use log::warn;
use protide_core::storage::{data_file, load_json, save_json};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Maximum number of history entries to keep
pub const MAX_HISTORY_ENTRIES: usize = 200;

const HISTORY_FILE: &str = "history.json";

/// A single history entry storing request/response data. The request is kept
/// as written ({{vars}} unresolved) because it is written to disk.
#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unique ID for this entry
    pub id: u64,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request URL
    pub url: String,
    /// Request headers
    pub headers: Vec<(String, String)>,
    /// Request body (if any)
    pub body: Option<String>,
    /// Response status code (if completed)
    pub status: Option<u16>,
    /// Response time
    pub response_time: Option<Duration>,
}

impl HistoryEntry {
    /// Create a new history entry
    pub fn new(
        id: u64,
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<String>,
    ) -> Self {
        Self {
            id,
            method,
            url,
            headers,
            body,
            status: None,
            response_time: None,
        }
    }

    /// Get a truncated URL for display
    pub fn display_url(&self) -> String {
        const MAX_LEN: usize = 40;
        if self.url.chars().count() <= MAX_LEN {
            self.url.clone()
        } else {
            let head: String = self.url.chars().take(MAX_LEN - 3).collect();
            format!("{head}...")
        }
    }

    /// Status code and duration, e.g. "200 150ms", once a response arrived.
    pub fn summary(&self) -> Option<String> {
        let status = self.status?;
        Some(match self.response_time {
            Some(time) => format!("{status} {}ms", time.as_millis()),
            None => status.to_string(),
        })
    }
}

/// Request history storage
#[derive(Default)]
pub struct RequestHistory {
    entries: Vec<HistoryEntry>,
    next_id: u64,
    /// Where to persist; `None` when no workspace is open.
    file: Option<PathBuf>,
}

impl RequestHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load the workspace's history. A missing or corrupt file starts empty.
    pub fn load(workspace: &Path) -> Self {
        let file = data_file(workspace, HISTORY_FILE);
        let mut entries: Vec<HistoryEntry> = load_json(&file).unwrap_or_default();
        entries.truncate(MAX_HISTORY_ENTRIES);
        let next_id = entries.iter().map(|e| e.id + 1).max().unwrap_or(0);
        Self {
            entries,
            next_id,
            file: Some(file),
        }
    }

    fn persist(&self) {
        if let Some(file) = &self.file
            && let Err(e) = save_json(file, &self.entries)
        {
            warn!("Failed to save history to {}: {}", file.display(), e);
        }
    }

    /// Add a new entry to history
    pub fn add(
        &mut self,
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<String>,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let entry = HistoryEntry::new(id, method, url, headers, body);
        self.entries.insert(0, entry);

        // Trim to max size
        if self.entries.len() > MAX_HISTORY_ENTRIES {
            self.entries.truncate(MAX_HISTORY_ENTRIES);
        }
        self.persist();

        id
    }

    /// Update entry with response data
    pub fn update_response(&mut self, id: u64, status: u16, response_time: Duration) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.status = Some(status);
            entry.response_time = Some(response_time);
            self.persist();
        }
    }

    /// Get all entries
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Get entry by id
    pub fn get(&self, id: u64) -> Option<&HistoryEntry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

impl gpui::Global for RequestHistory {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    // HistoryEntry tests
    #[test]
    fn test_history_entry_new() {
        let entry = HistoryEntry::new(
            1,
            "GET".to_string(),
            "https://api.example.com".to_string(),
            vec![],
            None,
        );
        assert_eq!(entry.id, 1);
        assert_eq!(entry.method, "GET");
        assert_eq!(entry.url, "https://api.example.com");
        assert!(entry.headers.is_empty());
        assert!(entry.body.is_none());
        assert!(entry.status.is_none());
        assert!(entry.response_time.is_none());
    }

    #[test]
    fn test_history_entry_display_url_short() {
        let entry = HistoryEntry::new(
            1,
            "GET".to_string(),
            "https://api.example.com".to_string(),
            vec![],
            None,
        );
        assert_eq!(entry.display_url(), "https://api.example.com");
    }

    #[test]
    fn test_history_entry_display_url_long_truncated() {
        let long_url =
            "https://api.example.com/very/long/path/that/exceeds/the/maximum/display/length";
        let entry = HistoryEntry::new(1, "GET".to_string(), long_url.to_string(), vec![], None);
        let display = entry.display_url();
        assert!(display.len() <= 40);
        assert!(display.ends_with("..."));
    }

    #[test]
    fn test_history_entry_display_url_exact_max_len() {
        // Create URL exactly 40 chars long
        let url = "https://api.example.com/path/exact/12345";
        assert_eq!(url.len(), 40);
        let entry = HistoryEntry::new(1, "GET".to_string(), url.to_string(), vec![], None);
        assert_eq!(entry.display_url(), url);
    }

    // RequestHistory tests
    #[test]
    fn test_request_history_new() {
        let history = RequestHistory::new();
        assert!(history.entries().is_empty());
    }

    #[test]
    fn test_request_history_add() {
        let mut history = RequestHistory::new();
        let id = history.add(
            "GET".to_string(),
            "https://api.example.com".to_string(),
            vec![],
            None,
        );
        assert_eq!(id, 0);
        assert_eq!(history.entries().len(), 1);
        assert_eq!(history.entries()[0].method, "GET");
    }

    #[test]
    fn test_request_history_add_multiple() {
        let mut history = RequestHistory::new();
        let id1 = history.add(
            "GET".to_string(),
            "https://example.com/1".to_string(),
            vec![],
            None,
        );
        let id2 = history.add(
            "POST".to_string(),
            "https://example.com/2".to_string(),
            vec![],
            None,
        );
        let id3 = history.add(
            "PUT".to_string(),
            "https://example.com/3".to_string(),
            vec![],
            None,
        );

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);
        assert_eq!(history.entries().len(), 3);
        // Most recent first
        assert_eq!(history.entries()[0].method, "PUT");
        assert_eq!(history.entries()[1].method, "POST");
        assert_eq!(history.entries()[2].method, "GET");
    }

    #[test]
    fn test_request_history_get() {
        let mut history = RequestHistory::new();
        let id = history.add(
            "GET".to_string(),
            "https://example.com".to_string(),
            vec![],
            None,
        );

        let entry = history.get(id);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().url, "https://example.com");

        let not_found = history.get(999);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_request_history_update_response() {
        let mut history = RequestHistory::new();
        let id = history.add(
            "GET".to_string(),
            "https://example.com".to_string(),
            vec![],
            None,
        );

        history.update_response(id, 200, Duration::from_millis(150));

        let entry = history.get(id).unwrap();
        assert_eq!(entry.status, Some(200));
        assert_eq!(entry.response_time, Some(Duration::from_millis(150)));
    }

    #[test]
    fn test_request_history_max_entries() {
        let mut history = RequestHistory::new();

        // Add more than MAX_HISTORY_ENTRIES
        for i in 0..(MAX_HISTORY_ENTRIES + 10) {
            history.add(
                "GET".to_string(),
                format!("https://example.com/{}", i),
                vec![],
                None,
            );
        }

        // Should be capped at MAX_HISTORY_ENTRIES
        assert_eq!(history.entries().len(), MAX_HISTORY_ENTRIES);
        // Most recent should be last added
        assert!(
            history.entries()[0]
                .url
                .ends_with(&format!("{}", MAX_HISTORY_ENTRIES + 9))
        );
    }

    #[test]
    fn history_round_trips_through_the_workspace() {
        let tmp = TempDir::new("history-round-trip");
        let mut history = RequestHistory::load(tmp.path());
        let first = history.add(
            "GET".into(),
            "{{base_url}}/a".into(),
            vec![("X-Key".into(), "{{api_key}}".into())],
            None,
        );
        history.update_response(first, 404, Duration::from_millis(12));
        history.add(
            "POST".into(),
            "{{base_url}}/b".into(),
            vec![],
            Some("{}".into()),
        );

        let mut back = RequestHistory::load(tmp.path());
        assert_eq!(back.entries().len(), 2);
        assert_eq!(back.entries()[0].url, "{{base_url}}/b");
        let old = back.get(first).unwrap();
        assert_eq!(
            old.headers,
            [("X-Key".to_string(), "{{api_key}}".to_string())]
        );
        assert_eq!(old.summary().as_deref(), Some("404 12ms"));
        assert_ne!(back.add("GET".into(), "x".into(), vec![], None), first);
    }

    #[test]
    fn a_saved_history_is_capped_and_a_corrupt_one_starts_empty() {
        let tmp = TempDir::new("history-cap");
        let mut history = RequestHistory::load(tmp.path());
        for i in 0..MAX_HISTORY_ENTRIES + 5 {
            history.add("GET".into(), format!("/{i}"), vec![], None);
        }
        assert_eq!(
            RequestHistory::load(tmp.path()).entries().len(),
            MAX_HISTORY_ENTRIES
        );

        let file = protide_core::storage::data_file(tmp.path(), HISTORY_FILE);
        std::fs::write(&file, "not json").unwrap();
        assert!(RequestHistory::load(tmp.path()).entries().is_empty());
    }

    #[test]
    fn summary_shows_status_and_duration() {
        let mut history = RequestHistory::new();
        let id = history.add("GET".into(), "/".into(), vec![], None);
        assert_eq!(history.get(id).unwrap().summary(), None);
        history.update_response(id, 200, Duration::from_millis(150));
        assert_eq!(
            history.get(id).unwrap().summary().as_deref(),
            Some("200 150ms")
        );
    }

    #[test]
    fn display_url_truncates_on_char_boundaries() {
        let url = format!("https://例え.jp/{}", "日本語".repeat(20));
        let entry = HistoryEntry::new(1, "GET".into(), url, vec![], None);
        assert!(entry.display_url().ends_with("..."));
    }
}
