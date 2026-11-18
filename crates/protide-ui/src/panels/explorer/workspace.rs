use super::*;
use gpui::Context;

/// The text of the .http file at `path` and its request `index`, if the
/// file reads and parses.
pub(super) fn read_request(
    path: &std::path::Path,
    index: usize,
) -> Option<(String, http_parser::Request)> {
    let content = fs::read_to_string(path).ok()?;
    let request = http_parser::parse(&content).ok()?.into_iter().nth(index)?;
    Some((content, request))
}

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
            let rp = self.request_panel.as_ref().map(|rp| rp.read(cx));
            let draft = rp.map(|rp| rp.capture_draft(cx));
            let location = rp.and_then(|rp| rp.loaded_location());
            let entry = session
                .workspaces
                .entry(current.to_string_lossy().to_string())
                .or_default();
            entry.active_request = location.as_ref().map_or(0, |l| l.1);
            entry.active_file = location.map(|l| l.0);
            entry.draft = draft;
            entry.expanded_folders = self.collect_expanded();
            entry.active_env = self.env_state.active().map(|e| e.name.clone());
        }

        session.current_workspace = Some(path.clone());
        let saved_entry = session.workspaces.get(&new_key).cloned();
        crate::session::save_bg(session);

        self.workspace_path = Some(path.clone());
        self.env_state = EnvironmentState::load(&path);
        cx.set_global(RequestHistory::load(&path));
        self.collection_items = self.scan_directory(&path);
        self.selected_item = Option::None;

        match protide_core::workspace::Workspace::open(&path) {
            Ok(ws) => {
                info!("Workspace loaded: {}", path.display());
                self.workspace_watcher = Some(Rc::new(ws));
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
        self.env_state = EnvironmentState::load(&path);
        cx.set_global(RequestHistory::load(&path));
        self.collection_items = self.scan_directory(&path);
        match protide_core::workspace::Workspace::open(&path) {
            Ok(ws) => {
                info!("Workspace loaded: {}", path.display());
                self.workspace_watcher = Some(Rc::new(ws));
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
                            let content = fs::read_to_string(p).ok();
                            if let Some(request_panel) = &self.request_panel
                                && request_panel
                                    .read(cx)
                                    .is_external_change(p, content.as_deref())
                            {
                                request_panel.update(cx, |panel, cx| {
                                    panel.external_change_pending = true;
                                    cx.notify();
                                });
                            }
                            if let Some(content) = content {
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
                        if self.sync_skip_paths.remove(p) {
                            continue;
                        }
                        let p = p.clone();
                        let root = root.clone();
                        if let Some(win) = self.main_window.upgrade() {
                            win.update(cx, |win, _| {
                                win.broadcast_workspace_file(&root, &p, String::new(), true)
                            });
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

    /// Load request `index` of a .http file into the request panel
    pub fn load_request_file(&mut self, path: PathBuf, index: usize, cx: &mut Context<Self>) {
        if let Some((content, req)) = read_request(&path, index)
            && let Some(request_panel) = &self.request_panel
        {
            let variable_extractions = req.meta.variable_extractions.clone();
            let proto_path = req.meta.proto_path.clone().map(std::path::PathBuf::from);

            request_panel.update(cx, |panel, cx| {
                panel.load_from_parsed_request(&req, cx);
                if !variable_extractions.is_empty() {
                    panel.set_variable_extractions(variable_extractions, cx);
                }
                panel.set_loaded_file(path.clone(), index, req, content);
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
}
