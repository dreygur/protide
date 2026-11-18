use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Record that the editor now holds request `index` of `path`, so a save
    /// rewrites that request and no other.
    pub(in crate::panels) fn set_loaded_file(
        &mut self,
        path: std::path::PathBuf,
        index: usize,
        request: http_parser::Request,
    ) {
        self.current_file = Some(path);
        self.current_request_index = index;
        self.loaded_request = Some(request);
        self.external_change_pending = false;
    }
}
