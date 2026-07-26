//! Test-only helpers shared across `protide-core` modules.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// A uniquely-named scratch directory under the system temp dir that is
/// removed when the guard drops.
///
/// Fixed temp paths are a real hazard here: several tests `remove_dir_all`
/// their scratch directory, so two tests (or two concurrent `cargo test`
/// processes, or a `cargo test` running while the app's own suite runs) that
/// share a name will delete each other's fixtures mid-run. The name mixes the
/// process id, a nanosecond timestamp and a process-local counter so it is
/// unique both across processes and across threads within one process.
/// Cleanup happens in `Drop`, so a panicking assertion still leaves no litter.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(prefix: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "{}_{}_{}_{}",
            prefix,
            std::process::id(),
            nanos,
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("failed to create test temp dir");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Write `contents` to `name` inside this directory and return the path.
    pub fn write(&self, name: &str, contents: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, contents).expect("failed to write test fixture");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
