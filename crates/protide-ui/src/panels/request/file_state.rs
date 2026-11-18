use super::super::request_utils::split_grpc_url;
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Re-render the explorer so its dirty marker follows this panel. Deferred
    /// because it is also called while this panel renders.
    fn notify_explorer(&self, cx: &mut Context<Self>) {
        if let Some(explorer) = self.explorer_panel.clone() {
            cx.defer(move |cx| explorer.update(cx, |_, cx| cx.notify()));
        }
    }

    /// Record that the editor now holds request `index` of `path`, as loaded
    /// or just saved, so a save rewrites that request and no other.
    pub(in crate::panels) fn set_loaded_file(
        &mut self,
        path: std::path::PathBuf,
        index: usize,
        request: http_parser::Request,
        content: String,
        cx: &mut Context<Self>,
    ) {
        self.current_file = Some(path);
        self.current_request_index = index;
        self.loaded_request = Some(request);
        self.disk_content = Some(content);
        self.external_change_pending = false;
        self.clean_snapshot = None;
        self.dirty = false;
        self.notify_explorer(cx);
    }

    /// Detach the editor from its file, for a load that comes from
    /// somewhere else: a save then asks for a file instead of rewriting
    /// the request that was open before.
    pub(super) fn clear_loaded_file(&mut self, cx: &mut Context<Self>) {
        self.current_file = None;
        self.current_request_index = 0;
        self.loaded_request = None;
        self.disk_content = None;
        self.external_change_pending = false;
        self.clean_snapshot = None;
        self.dirty = false;
        self.notify_explorer(cx);
    }

    /// Load request `index` of `path` into the editor, with `content` the
    /// file text it was parsed from.
    pub(in crate::panels) fn load_file_request(
        &mut self,
        path: std::path::PathBuf,
        index: usize,
        request: http_parser::Request,
        content: String,
        cx: &mut Context<Self>,
    ) {
        self.load_from_parsed_request(&request, cx);
        if !request.meta.variable_extractions.is_empty() {
            self.set_variable_extractions(request.meta.variable_extractions.clone(), cx);
        }
        let proto_path = request.meta.proto_path.clone();
        let method = split_grpc_url(&request.url).1.map(str::to_string);
        self.set_loaded_file(path, index, request, content, cx);
        if let Some(proto_path) = proto_path {
            self.load_grpc_proto_from_path(proto_path.into(), cx);
        }
        // Select the method the file names, not the proto's first one or the
        // previous request's, so a save writes the same URL back.
        self.grpc_method = method.and_then(|name| {
            self.grpc_methods
                .iter()
                .find(|m| m.full_name == name)
                .cloned()
        });
        if let Some(m) = &self.grpc_method {
            self.grpc_service = m.full_name.split('/').next().map(str::to_string);
        }
    }

    /// Called from render, after queued editor contents have landed. The
    /// first call after a load records what a save would write; later calls
    /// compare against it, so an edit that is undone, such as a protocol
    /// switched and back, is clean again. A binary body never comes from a
    /// file, and reading it each frame would be costly, so it simply counts
    /// as an edit.
    pub(super) fn refresh_dirty(&mut self, cx: &mut Context<Self>) {
        if self.current_file.is_none() {
            return;
        }
        let dirty = self.body_type == BodyType::Binary || {
            let now = self.build_request(cx).to_http();
            match &self.clean_snapshot {
                Some(clean) => *clean != now,
                None => {
                    self.clean_snapshot = Some(now);
                    false
                }
            }
        };
        if dirty != self.dirty {
            self.dirty = dirty;
            self.notify_explorer(cx);
        }
    }

    /// The loaded file and request index while they hold unsaved edits.
    pub fn dirty_location(&self) -> Option<(std::path::PathBuf, usize)> {
        self.loaded_location().filter(|_| self.dirty)
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
