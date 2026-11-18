use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Record that the editor now holds request `index` of `path`, so a save
    /// rewrites that request and no other.
    pub(in crate::panels) fn set_loaded_file(
        &mut self,
        path: std::path::PathBuf,
        index: usize,
        request: http_parser::Request,
        content: String,
    ) {
        self.current_file = Some(path);
        self.current_request_index = index;
        self.loaded_request = Some(request);
        self.disk_content = Some(content);
        self.external_change_pending = false;
    }

    /// Whether a watcher event saying `path` now holds `content` is a change
    /// made outside the app to the open file. The app's own saves leave the
    /// file equal to `disk_content`, so they never count.
    pub(in crate::panels) fn is_external_change(
        &self,
        path: &std::path::Path,
        content: Option<&str>,
    ) -> bool {
        self.current_file.as_deref() == Some(path) && self.disk_content.as_deref() != content
    }
}
