//! Test-only helpers shared across `protide-ui` panel tests.

use gpui::TestAppContext;
use std::path::{Path, PathBuf};

/// Install the globals every rendered panel reads from.
///
/// `gpui_component::Theme` and `crate::theme` are process-globals on the
/// `TestAppContext`, so each `#[gpui::test]` must seed them before building a
/// window or `render()` panics on a missing global.
pub fn init_theme(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(gpui_component::Theme::default());
        crate::theme::init(cx);
    });
}

/// A uniquely-named scratch directory that deletes itself on drop.
///
/// The name is derived from the pid, a monotonic counter and the wall clock so
/// concurrently-running tests (and concurrent `cargo test` invocations) never
/// collide on a fixed path.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "protide-ui-test-{tag}-{}-{nanos}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Path to `name` inside this directory (the file need not exist).
    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Write `contents` to `name` inside this directory and return its path.
    pub fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.join(name);
        std::fs::write(&path, contents).expect("write temp file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
