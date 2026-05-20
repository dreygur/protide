//! Stash operations.

use super::GitRepo;

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}

impl GitRepo {
    /// List all stash entries.
    pub fn stash_list(&mut self) -> Result<Vec<StashEntry>, String> {
        let mut entries = Vec::new();
        self.repo.stash_foreach(|idx, msg, _oid| {
            entries.push(StashEntry { index: idx, message: msg.to_string() });
            true
        }).map_err(|e| format!("Stash list failed: {}", e))?;
        Ok(entries)
    }

    /// Push current working tree changes onto the stash.
    pub fn stash_push(&mut self, message: &str) -> Result<(), String> {
        let sig = self.repo.signature()
            .map_err(|e| format!("Signature error: {}", e))?;
        let msg = if message.is_empty() { None } else { Some(message) };
        self.repo.stash_save(&sig, msg.unwrap_or("WIP"), None)
            .map(|_| ())
            .map_err(|e| format!("Stash push failed: {}", e))
    }

    /// Apply and drop the stash entry at `index`.
    pub fn stash_pop(&mut self, index: usize) -> Result<(), String> {
        self.repo.stash_apply(index, None)
            .map_err(|e| format!("Stash apply failed: {}", e))?;
        self.repo.stash_drop(index)
            .map_err(|e| format!("Stash drop failed: {}", e))
    }

    /// Drop a stash entry without applying it.
    pub fn stash_drop(&mut self, index: usize) -> Result<(), String> {
        self.repo.stash_drop(index)
            .map_err(|e| format!("Stash drop failed: {}", e))
    }
}
