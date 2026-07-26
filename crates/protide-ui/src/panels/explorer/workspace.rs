use super::*;
use gpui::Context;

impl ExplorerPanel {
    /// Load collection from a specific path (full workspace switch with session save)
    pub fn load_collection_from_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !path.is_dir() {
            return;
        }

        let new_key = path.to_string_lossy().to_string();
        let mut session = crate::session::load();

        // Persist state for the workspace we're leaving
        if let Some(ref current) = self.workspace_path.clone() {
            let draft = self
                .request_panel
                .as_ref()
                .map(|rp| rp.read(cx).capture_draft(cx));
            let entry = session
                .workspaces
                .entry(current.to_string_lossy().to_string())
                .or_default();
            entry.active_file = self.selected_item.clone();
            entry.draft = draft;
            entry.expanded_folders = self.collect_expanded();
            entry.active_env = self.env_state.active().map(|e| e.name.clone());
        }

        session.current_workspace = Some(path.clone());
        let saved_entry = session.workspaces.get(&new_key).cloned();
        crate::session::save_bg(session);

        self.workspace_path = Some(path.clone());
        self.collection_items = self.scan_directory(&path);
        self.selected_item = Option::None;

        match protide_core::workspace::Workspace::open(&path) {
            Ok(ws) => {
                info!("Workspace loaded: {}", path.display());
                self.workspace_watcher = Some(Arc::new(ws));
            }
            Err(e) => {
                error!("File watcher failed: {}", e);
            }
        }

        if let Some(entry) = saved_entry {
            self.restore_workspace_session(&entry, cx);
        }

        self.collections_expanded = true;
        if let Some(win) = self.main_window.upgrade() {
            win.update(cx, |w, cx| w.ensure_sidebar_visible(cx));
        }

        cx.notify();
    }

    /// Called on startup to seed the workspace without triggering the "save old state" path.
    pub fn init_workspace(
        &mut self,
        path: PathBuf,
        saved_entry: Option<crate::session::WorkspaceEntry>,
        cx: &mut Context<Self>,
    ) {
        if !path.is_dir() {
            return;
        }
        self.workspace_path = Some(path.clone());
        self.collection_items = self.scan_directory(&path);
        match protide_core::workspace::Workspace::open(&path) {
            Ok(ws) => {
                info!("Workspace loaded: {}", path.display());
                self.workspace_watcher = Some(Arc::new(ws));
            }
            Err(e) => {
                error!("Workspace watcher: {}", e);
            }
        }
        if let Some(entry) = saved_entry {
            self.restore_workspace_session(&entry, cx);
        }
        cx.notify();
    }

    /// Poll the file watcher and refresh the collection tree if changes detected.
    pub fn poll_workspace_changes(&mut self, cx: &mut Context<Self>) {
        use protide_core::workspace::WorkspaceEvent;
        let watcher = self.workspace_watcher.clone();
        let workspace = self.workspace_path.clone();
        if let (Some(ws), Some(root)) = (watcher, workspace) {
            let events = ws.poll_events();
            let relevant: Vec<_> = events
                .into_iter()
                .filter(|e| protide_core::workspace::is_relevant(e, &root))
                .collect();
            if relevant.is_empty() {
                return;
            }
            for event in &relevant {
                match event {
                    WorkspaceEvent::FileCreated(p) | WorkspaceEvent::FileModified(p) => {
                        if !self.sync_skip_paths.remove(p) {
                            if let Some(request_panel) = &self.request_panel {
                                let is_open_file = request_panel.read(cx).current_file.as_deref()
                                    == Some(p.as_path());
                                if is_open_file {
                                    request_panel.update(cx, |panel, cx| {
                                        panel.external_change_pending = true;
                                        cx.notify();
                                    });
                                }
                            }
                            if let Ok(content) = fs::read_to_string(p) {
                                let p = p.clone();
                                let root = root.clone();
                                if let Some(win) = self.main_window.upgrade() {
                                    win.update(cx, |win, _| {
                                        win.broadcast_workspace_file(&root, &p, content, false)
                                    });
                                }
                            }
                        }
                    }
                    WorkspaceEvent::FileDeleted(p) => {
                        if !self.sync_skip_paths.remove(p) {
                            let p = p.clone();
                            let root = root.clone();
                            if let Some(win) = self.main_window.upgrade() {
                                win.update(cx, |win, _| {
                                    win.broadcast_workspace_file(&root, &p, String::new(), true)
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
            let expanded = self.collect_expanded();
            self.collection_items = self.scan_directory(&root);
            self.apply_expanded(&expanded);
            cx.notify();
        }
    }

    /// Refresh collections by rescanning the workspace directory
    pub fn refresh_collections(&mut self, cx: &mut Context<Self>) {
        if let Some(workspace) = self.workspace_path.clone() {
            let expanded = self.collect_expanded();
            self.collection_items = self.scan_directory(&workspace);
            self.apply_expanded(&expanded);
            cx.notify();
        }
    }

    /// Open folder dialog and load collection
    pub fn open_folder(&mut self, cx: &mut Context<Self>) {
        file_dialog::prompt(
            cx,
            Pick::Folder,
            |d| {
                let d = d.set_title("Open Collection Folder");
                match last_paths::last_dir("open_folder") {
                    Some(dir) => d.set_directory(dir),
                    None => d,
                }
            },
            |this, folder, cx| {
                last_paths::save_last_dir("open_folder", &folder);
                this.load_collection_from_path(folder, cx);
            },
        );
    }

    /// Load a .http file into the request panel
    pub fn load_request_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if let Ok(content) = fs::read_to_string(&path)
            && let Ok(requests) = http_parser::parse(&content)
            && let Some(req) = requests.first()
            && let Some(request_panel) = &self.request_panel
        {
            let variable_extractions = req.meta.variable_extractions.clone();
            let proto_path = req.meta.proto_path.clone().map(std::path::PathBuf::from);
            let req = req.clone();

            request_panel.update(cx, |panel, cx| {
                panel.load_from_parsed_request(&req, cx);
                if !variable_extractions.is_empty() {
                    panel.set_variable_extractions(variable_extractions, cx);
                }
                panel.current_file = Some(path.clone());
                panel.external_change_pending = false;
                if let Some(pp) = proto_path {
                    panel.load_grpc_proto_from_path(pp, cx);
                }
            });
        }
    }

    /// Close the open project / workspace folder
    pub(super) fn close_project(&mut self, cx: &mut Context<Self>) {
        self.workspace_path = None;
        self.collection_items.clear();
        self.collections_expanded = true;
        cx.notify();
    }

    pub fn create_new_request(&mut self, cx: &mut Context<Self>) {
        let default_dir = self.workspace_path.clone().or_else(dirs::home_dir);
        let start_dir = last_paths::last_dir("new_request")
            .or_else(|| last_paths::last_dir("save_request"))
            .or(default_dir);
        file_dialog::prompt(
            cx,
            Pick::Save,
            move |d| {
                let d = d
                    .set_title("Create New Request")
                    .set_file_name("new-request.http")
                    .add_filter("HTTP Request", &["http"]);
                match start_dir {
                    Some(dir) => d.set_directory(dir),
                    None => d,
                }
            },
            |this, path, cx| {
                last_paths::save_last_dir("new_request", &path);
                last_paths::save_last_dir("save_request", &path);
                let path = if path.extension().is_none() || path.extension().unwrap() != "http" {
                    path.with_extension("http")
                } else {
                    path
                };
                let content =
                    "### New Request\n# @name new-request\n\nGET https://api.example.com\n";
                match fs::write(&path, content) {
                    Ok(_) => {
                        info!("Created: {}", path.display());
                        if let Some(workspace) = &this.workspace_path {
                            if path.starts_with(workspace) {
                                this.collection_items = this.scan_directory(workspace);
                            }
                        }
                        this.load_request_file(path, cx);
                    }
                    Err(e) => error!("Failed to create request {}: {}", path.display(), e),
                }
            },
        );
    }

    pub(super) fn clone_file(&mut self, source_path: PathBuf, cx: &mut Context<Self>) {
        let content = match fs::read_to_string(&source_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read {}: {}", source_path.display(), e);
                return;
            }
        };
        let parent = match source_path.parent() {
            Some(p) => p.to_path_buf(),
            None => return,
        };
        let stem = source_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "request".to_string());
        let mut new_path = parent.join(format!("{}-copy.http", stem));
        let mut n = 2usize;
        while new_path.exists() {
            new_path = parent.join(format!("{}-copy-{}.http", stem, n));
            n += 1;
        }
        match fs::write(&new_path, &content) {
            Ok(_) => {
                info!("Cloned: {}", new_path.display());
                if let Some(workspace) = &self.workspace_path {
                    if new_path.starts_with(workspace) {
                        self.collection_items = self.scan_directory(workspace);
                    }
                }
                self.load_request_file(new_path, cx);
            }
            Err(e) => {
                error!("Failed to clone: {}", e);
                cx.notify();
            }
        }
    }

    pub(super) fn create_new_file_in_folder(
        &mut self,
        folder_path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        file_dialog::prompt(
            cx,
            Pick::Save,
            move |d| {
                d.set_title("Create New Request")
                    .set_file_name("new-request.http")
                    .set_directory(&folder_path)
                    .add_filter("HTTP Request", &["http"])
            },
            |this, path, cx| {
                last_paths::save_last_dir("new_request", &path);
                last_paths::save_last_dir("save_request", &path);
                let path = if path.extension().map_or(true, |e| e != "http") {
                    path.with_extension("http")
                } else {
                    path
                };
                let content =
                    "### New Request\n# @name new-request\n\nGET https://api.example.com\n";
                match fs::write(&path, content) {
                    Ok(_) => {
                        info!("Created: {}", path.display());
                        if let Some(workspace) = &this.workspace_path {
                            if path.starts_with(workspace) {
                                this.collection_items = this.scan_directory(workspace);
                            }
                        }
                        this.load_request_file(path, cx);
                    }
                    Err(e) => error!("Failed to create request: {}", e),
                }
            },
        );
    }
}
