//! Workspace files shared between peers: one CRDT entry per file.

use super::*;

const FILE_ID_CONTEXT: &str = "protide workspace file id v1";

/// One entry id per relative path, the same on every peer, so last write
/// wins orders all versions of a file instead of each change standing alone.
fn file_id(relative_path: &str) -> uuid::Uuid {
    let hash = blake3::derive_key(FILE_ID_CONTEXT, relative_path.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    uuid::Uuid::from_bytes(bytes)
}

impl SyncEngine {
    /// Broadcast a workspace file change (create/modify/delete) to all P2P
    /// peers. Returns `None` when the file already holds what the store has,
    /// which is how a file written from a peer's change avoids echoing back.
    pub fn broadcast_workspace_file(
        &mut self,
        workspace_root: &std::path::Path,
        file_path: &std::path::Path,
        content: String,
        deleted: bool,
    ) -> Option<types::CrdtEntry> {
        let rel = file_path.strip_prefix(workspace_root).unwrap_or(file_path);
        let rel = rel.to_string_lossy();
        let payload = serde_json::json!({
            "path": rel,
            "content": content,
            "deleted": deleted,
        })
        .to_string();

        let id = file_id(&rel);
        if self.store.get(&id).is_some_and(|e| e.data == payload) {
            return None;
        }
        self.update_local_change(id, DataType::WorkspaceFile, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn node() -> SyncEngine {
        SyncEngine::new(SyncConfig::default())
    }

    fn write(engine: &mut SyncEngine, content: &str) -> Option<types::CrdtEntry> {
        engine.broadcast_workspace_file(
            Path::new("/ws"),
            Path::new("/ws/api/users.http"),
            content.to_string(),
            false,
        )
    }

    /// REGRESSION: every change made a new random entry, so last write
    /// wins never compared two versions of the same file.
    #[test]
    fn every_change_to_a_file_updates_one_entry() {
        let mut a = node();
        let first = write(&mut a, "v1").unwrap();
        let second = write(&mut a, "v2").unwrap();
        assert_eq!(first.id, second.id);
        assert!(second.timestamp > first.timestamp);
    }

    /// REGRESSION: an old copy arriving late overwrote a newer edit.
    #[test]
    fn a_late_old_copy_does_not_overwrite_a_newer_edit() {
        let (mut a, mut b) = (node(), node());
        let v1 = write(&mut a, "v1").unwrap();
        b.store.merge_remote(v1.clone());
        let v2 = write(&mut b, "v2").unwrap();

        assert!(matches!(a.store.merge_remote(v2), MergeResult::Accepted(_)));
        assert!(matches!(a.store.merge_remote(v1), MergeResult::Stale));
        assert!(a.store.get(&first_id(&a)).unwrap().data.contains("v2"));
    }

    /// REGRESSION: writing a received file fired the watcher, which sent the
    /// same content back as a new change and raced real edits.
    #[test]
    fn content_already_synced_is_not_sent_again() {
        let (mut a, mut b) = (node(), node());
        let v1 = write(&mut a, "v1").unwrap();
        b.store.merge_remote(v1);
        assert!(write(&mut b, "v1").is_none());
        assert!(write(&mut b, "v2").is_some());
    }

    fn first_id(engine: &SyncEngine) -> uuid::Uuid {
        engine.store.all_entries().first().unwrap().id
    }
}
