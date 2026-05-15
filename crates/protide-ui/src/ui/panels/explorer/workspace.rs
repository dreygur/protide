use gpui::Context;
use super::*;
use super::util::{sanitize_filename, request_to_http_content};

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
            let draft = self.request_panel.as_ref()
                .map(|rp| rp.read(cx).capture_draft(cx));
            let entry = session.workspaces
                .entry(current.to_string_lossy().to_string())
                .or_default();
            entry.active_file      = self.selected_item.clone();
            entry.draft            = draft;
            entry.expanded_folders = self.collect_expanded();
            entry.active_env       = self.env_state.active().map(|e| e.name.clone());
        }

        session.current_workspace = Some(path.clone());
        let saved_entry = session.workspaces.get(&new_key).cloned();
        crate::session::save_bg(session);

        self.workspace_path    = Some(path.clone());
        self.collection_items  = self.scan_directory(&path);
        self.selected_item     = Option::None;

        match protide_core::workspace::Workspace::open(&path) {
            Ok(ws)  => {
                info!("Workspace loaded: {}", path.display());
                self.workspace_watcher = Some(Arc::new(ws));
            }
            Err(e)  => { error!("File watcher failed: {}", e); }
        }

        if let Some(entry) = saved_entry {
            self.restore_workspace_session(&entry, cx);
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
        self.workspace_path   = Some(path.clone());
        self.collection_items = self.scan_directory(&path);
        match protide_core::workspace::Workspace::open(&path) {
            Ok(ws)  => {
                info!("Workspace loaded: {}", path.display());
                self.workspace_watcher = Some(Arc::new(ws));
            }
            Err(e)  => { error!("Workspace watcher: {}", e); }
        }
        if let Some(entry) = saved_entry {
            self.restore_workspace_session(&entry, cx);
        }
        cx.notify();
    }

    /// Poll the file watcher and refresh the collection tree if changes detected.
    pub fn poll_workspace_changes(&mut self, cx: &mut Context<Self>) {
        let watcher = self.workspace_watcher.clone();
        let workspace = self.workspace_path.clone();
        if let (Some(ws), Some(root)) = (watcher, workspace) {
            let events = ws.poll_events();
            if events
                .iter()
                .any(|e| protide_core::workspace::is_relevant(e, &root))
            {
                self.collection_items = self.scan_directory(&root);
                cx.notify();
            }
        }
    }

    /// Refresh collections by rescanning the workspace directory
    pub fn refresh_collections(&mut self, cx: &mut Context<Self>) {
        if let Some(workspace) = &self.workspace_path {
            self.collection_items = self.scan_directory(workspace);
            cx.notify();
        }
    }

    /// Open folder dialog and load collection
    pub fn open_folder(&mut self, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new().set_title("Open Collection Folder");
        if let Some(dir) = last_paths::last_dir("open_folder") {
            dialog = dialog.set_directory(dir);
        }
        if let Some(folder) = dialog.pick_folder() {
            last_paths::save_last_dir("open_folder", &folder);
            self.load_collection_from_path(folder, cx);
        }
    }

    /// Load a .http file into the request panel
    pub(super) fn load_request_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if let Ok(content) = fs::read_to_string(&path)
            && let Ok(requests) = http_parser::parse(&content)
                && let Some(req) = requests.first()
                    && let Some(request_panel) = &self.request_panel {
                        let variable_extractions = req.meta.variable_extractions.clone();
                        let proto_path = req.meta.proto_path.clone().map(std::path::PathBuf::from);
                        let req = req.clone();

                        request_panel.update(cx, |panel, cx| {
                            panel.load_from_parsed_request(&req, cx);
                            if !variable_extractions.is_empty() {
                                panel.set_variable_extractions(variable_extractions, cx);
                            }
                            panel.current_file = Some(path.clone());
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
        let mut dialog = rfd::FileDialog::new()
            .set_title("Create New Request")
            .set_file_name("new-request.http")
            .add_filter("HTTP Request", &["http"]);
        if let Some(dir) = start_dir {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.save_file() {
            last_paths::save_last_dir("new_request", &path);
            last_paths::save_last_dir("save_request", &path);
            let path = if path.extension().is_none() || path.extension().unwrap() != "http" {
                path.with_extension("http")
            } else {
                path
            };
            let content = "### New Request\n# @name new-request\n\nGET https://api.example.com\n";
            match fs::write(&path, content) {
                Ok(_) => {
                    info!("Created: {}", path.display());
                    if let Some(workspace) = &self.workspace_path {
                        if path.starts_with(workspace) {
                            self.collection_items = self.scan_directory(workspace);
                        }
                    }
                    self.load_request_file(path, cx);
                }
                Err(e) => error!("Failed to create request {}: {}", path.display(), e),
            }
        }
    }

    pub(super) fn create_new_file_in_folder(&mut self, folder_path: PathBuf, cx: &mut Context<Self>) {
        let dialog = rfd::FileDialog::new()
            .set_title("Create New Request")
            .set_file_name("new-request.http")
            .set_directory(&folder_path)
            .add_filter("HTTP Request", &["http"]);
        if let Some(path) = dialog.save_file() {
            last_paths::save_last_dir("new_request", &path);
            last_paths::save_last_dir("save_request", &path);
            let path = if path.extension().map_or(true, |e| e != "http") {
                path.with_extension("http")
            } else {
                path
            };
            let content = "### New Request\n# @name new-request\n\nGET https://api.example.com\n";
            match fs::write(&path, content) {
                Ok(_) => {
                    info!("Created: {}", path.display());
                    if let Some(workspace) = &self.workspace_path {
                        if path.starts_with(workspace) {
                            self.collection_items = self.scan_directory(workspace);
                        }
                    }
                    self.load_request_file(path, cx);
                }
                Err(e) => error!("Failed to create request: {}", e),
            }
        }
    }

    pub fn import_collection(&mut self, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Import Collection")
            .add_filter("All Supported", &["json", "yaml", "yml", "bru", "txt", "curl"])
            .add_filter("Postman Collection", &["json"])
            .add_filter("OpenAPI/Swagger", &["json", "yaml", "yml"])
            .add_filter("Bruno Collection", &["bru"])
            .add_filter("cURL Command", &["txt", "curl"]);
        if let Some(dir) = last_paths::last_dir("import_collection") {
            dialog = dialog.set_directory(dir);
        }
        if let Some(file_path) = dialog.pick_file() {
            last_paths::save_last_dir("import_collection", &file_path);
            if let Ok(content) = fs::read_to_string(&file_path) {
                match protide_core::import::import(&content) {
                    Ok(result) => {
                        let output_dir = self.workspace_path.clone()
                            .or_else(|| file_path.parent().map(|p| p.to_path_buf()))
                            .unwrap_or_else(|| PathBuf::from("."));
                        let collection_dir = if let Some(name) = &result.name {
                            let dir = output_dir.join(sanitize_filename(name));
                            let _ = fs::create_dir_all(&dir);
                            dir
                        } else {
                            output_dir.clone()
                        };
                        let mut created = 0;
                        for request in &result.requests {
                            let filename = request.meta.name.as_ref()
                                .map(|n| sanitize_filename(n))
                                .unwrap_or_else(|| format!("request-{}", created + 1));
                            let filepath = collection_dir.join(format!("{}.http", filename));
                            match request_to_http_content(request) {
                                Ok(c) => match fs::write(&filepath, &c) {
                                    Ok(_) => created += 1,
                                    Err(e) => warn!("Failed to write {}: {}", filepath.display(), e),
                                },
                                Err(e) => warn!("Failed to convert '{}': {}", filename, e),
                            }
                        }
                        if self.workspace_path.is_some() {
                            self.collection_items = self.scan_directory(&output_dir);
                        } else {
                            self.load_collection_from_path(collection_dir, cx);
                        }
                        info!("Imported {} request(s)", created);
                        let mut msg = format!("Imported {} request(s).", created);
                        let modal_state = if !result.warnings.is_empty() {
                            msg.push_str(&format!("\n\n{} warning(s):\n{}", result.warnings.len(), result.warnings.join("\n")));
                            ModalState::warning("Import Complete", msg)
                        } else {
                            ModalState::info("Import Complete", msg)
                        };
                        if let Some(win) = self.main_window.upgrade() {
                            win.update(cx, |win, cx| win.show_modal(modal_state, cx));
                        }
                    }
                    Err(e) => {
                        error!("Import failed: {}", e);
                        if let Some(win) = self.main_window.upgrade() {
                            win.update(cx, |win, cx| win.show_modal(ModalState::error("Import Failed", e), cx));
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    pub fn export_docs(&mut self, cx: &mut Context<Self>) {
        let Some(workspace) = &self.workspace_path else { return; };
        let dialog = rfd::FileDialog::new()
            .set_title("Export API Documentation")
            .add_filter("Markdown", &["md"])
            .add_filter("HTML", &["html"])
            .set_file_name("api-docs.md");
        if let Some(save_path) = dialog.save_file() {
            let is_html = save_path.extension().and_then(|e| e.to_str()) == Some("html");
            let format = if is_html { protide_core::export::ExportFormat::Html } else { protide_core::export::ExportFormat::Markdown };
            let modal_state = match protide_core::export::export_collection(workspace, format) {
                Ok(content) => match std::fs::write(&save_path, &content) {
                    Ok(_) => {
                        info!("Exported docs to: {}", save_path.display());
                        ModalState::info("Export Complete", format!("Documentation saved to {}", save_path.display()))
                    }
                    Err(e) => ModalState::error("Export Failed", format!("Failed to write file: {}", e)),
                },
                Err(e) => ModalState::error("Export Failed", e),
            };
            if let Some(win) = self.main_window.upgrade() {
                win.update(cx, |win, cx| win.show_modal(modal_state, cx));
            }
        }
    }
}
