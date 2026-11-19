//! Which scripts the user has agreed to run.
//!
//! A request file can come from anyone (an imported collection, a cloned repo,
//! a synced folder), and its scripts run on send. A script runs only once the
//! user has trusted its exact text, recorded as a content hash. The store lives
//! in the app config dir, never in the workspace, so a shared workspace cannot
//! pre trust its own scripts.

use crate::storage::{load_json, save_json};
use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;

/// File name of the store inside the app config dir.
const TRUST_FILE: &str = "trusted_scripts.json";

fn script_hash(script: &str) -> String {
    blake3::hash(script.as_bytes()).to_hex().to_string()
}

/// Hashes of trusted scripts, persisted to `path` when there is one.
#[derive(Clone, Default)]
pub struct TrustStore {
    path: Option<PathBuf>,
    hashes: BTreeSet<String>,
}

impl TrustStore {
    /// Load the store at `path`. A missing or corrupt file trusts nothing,
    /// and no path keeps trust in memory only.
    pub fn load(path: Option<PathBuf>) -> Self {
        let hashes = path.as_deref().and_then(load_json).unwrap_or_default();
        Self { path, hashes }
    }

    /// The store in the app config dir.
    pub fn app_default() -> Self {
        Self::load(dirs::config_dir().map(|dir| dir.join("protide").join(TRUST_FILE)))
    }

    /// A blank script never runs, so it needs no trust.
    pub fn is_trusted(&self, script: &str) -> bool {
        script.trim().is_empty() || self.hashes.contains(&script_hash(script))
    }

    /// The scripts among `scripts` the user has not trusted yet.
    pub fn untrusted<'a>(&self, scripts: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        scripts
            .into_iter()
            .filter(|script| !self.is_trusted(script))
            .collect()
    }

    /// Trust `script` and persist the store. Trust holds in memory even when
    /// the write fails.
    pub fn trust(&mut self, script: &str) -> io::Result<()> {
        if self.is_trusted(script) {
            return Ok(());
        }
        self.hashes.insert(script_hash(script));
        match &self.path {
            Some(path) => save_json(path, &self.hashes),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    const SCRIPT: &str = "env.set('token', 'x');";

    #[test]
    fn trust_survives_a_reload_from_the_same_file() {
        let tmp = TempDir::new("trust_round_trip");
        let path = tmp.path().join(TRUST_FILE);
        let mut store = TrustStore::load(Some(path.clone()));
        assert!(!store.is_trusted(SCRIPT));
        store.trust(SCRIPT).unwrap();

        let reloaded = TrustStore::load(Some(path));
        assert!(reloaded.is_trusted(SCRIPT));
    }

    #[test]
    fn trust_is_keyed_by_the_exact_text_and_stores_only_its_hash() {
        let tmp = TempDir::new("trust_hash");
        let path = tmp.path().join(TRUST_FILE);
        let mut store = TrustStore::load(Some(path.clone()));
        store.trust(SCRIPT).unwrap();

        let edited = "env.set('token', 'y');";
        assert_eq!(store.untrusted([SCRIPT, edited]), vec![edited]);
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains(&script_hash(SCRIPT)));
        assert!(!saved.contains("env.set"), "script text leaked: {saved}");
    }

    #[test]
    fn blank_scripts_need_no_trust() {
        let store = TrustStore::default();
        assert!(store.untrusted(["", "  \n"]).is_empty());
    }

    #[test]
    fn a_corrupt_file_trusts_nothing() {
        let tmp = TempDir::new("trust_corrupt");
        let path = tmp.write(TRUST_FILE, b"{not json");
        assert!(!TrustStore::load(Some(path)).is_trusted(SCRIPT));
    }

    #[test]
    fn a_store_without_a_path_trusts_in_memory() {
        let mut store = TrustStore::load(None);
        store.trust(SCRIPT).unwrap();
        assert!(store.is_trusted(SCRIPT));
    }
}
