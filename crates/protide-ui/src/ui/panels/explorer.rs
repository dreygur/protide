//! File explorer panel - displays the workspace file tree, request history, and environment selector

use crate::ui::components::modal::ModalState;
use crate::ui::main_window::MainWindow;
use gpui::{
    ClipboardItem, Context, Entity, FocusHandle, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render,
    ScrollWheelEvent, SharedString, Styled, Subscription, WeakEntity, Window, canvas, deferred,
    div, prelude::*, px,
};
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use super::history::{HistoryEntry, RequestHistory};
use super::request::RequestPanel;
use crate::last_paths;
use crate::theme;
use crate::ui::components::icons::{
    ICON_ARROW_DOWN, ICON_CHEVRON_DOWN, ICON_CHEVRON_RIGHT, ICON_CHEVRON_UP, ICON_CLOSE,
    ICON_DELETE, ICON_EDIT, ICON_EXTERNAL, ICON_FILE, ICON_FOLDER, ICON_FOLDER_OPEN, ICON_INFO,
    ICON_MD, ICON_MENU, ICON_PLUS, ICON_REFRESH, ICON_SETTINGS, ICON_SM, ICON_TIMER, icon,
};
use crate::ui::components::{icon_btn, render_text_view_with_max_scrolled};
use protide_core::models::{Environment, EnvironmentState};

/// Represents an item in the collection tree (either a folder or a .http file)
#[derive(Clone, Debug)]
pub struct CollectionItem {
    /// Display name
    pub name: String,
    /// Full path to the item
    pub path: PathBuf,
    /// Whether this is a folder
    pub is_folder: bool,
    /// Children (for folders)
    pub children: Vec<CollectionItem>,
    /// HTTP method (for .http files, parsed from first line)
    pub method: Option<String>,
    /// Whether this folder is expanded
    pub expanded: bool,
}

/// Edit target for environment variable editor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnvEditTarget {
    VarKey(usize),
    VarValue(usize),
    NewEnvName,
}

/// File explorer panel showing the workspace tree, history, and environment selector
pub struct ExplorerPanel {
    /// Reference to request panel for loading history items
    request_panel: Option<Entity<RequestPanel>>,
    /// Whether history section is expanded
    history_expanded: bool,
    /// Whether collections section is expanded
    collections_expanded: bool,
    /// Current workspace root path
    workspace_path: Option<PathBuf>,
    /// Active workspace watcher (keeps watcher alive and provides event channel)
    workspace_watcher: Option<Arc<protide_core::workspace::Workspace>>,
    /// Collection items (folders and .http files)
    collection_items: Vec<CollectionItem>,
    /// Environment state
    env_state: EnvironmentState,
    /// Whether the environment dropdown is open
    env_dropdown_open: bool,
    /// Whether the environment editor panel is expanded
    env_editor_open: bool,
    /// Currently editing target
    active_edit: Option<EnvEditTarget>,
    /// Edit selection range
    edit_selection: Range<usize>,
    /// Focus handle for editing
    edit_focus: FocusHandle,
    /// Temporary new environment name
    new_env_name: String,
    /// Show new environment input
    show_new_env_input: bool,
    /// Undo stack for edit fields (target, text, selection)
    edit_undo_stack: Vec<(EnvEditTarget, String, Range<usize>)>,
    /// Redo stack for edit fields
    edit_redo_stack: Vec<(EnvEditTarget, String, Range<usize>)>,
    /// Whether mouse is currently selecting text
    edit_is_selecting: bool,
    /// Per-target text-start x (window coords) captured by canvas overlays
    edit_input_origins: std::collections::HashMap<EnvEditTarget, f32>,
    /// Per-target horizontal scroll offset (pixels) for focused text inputs
    edit_scroll_offsets: std::collections::HashMap<EnvEditTarget, f32>,
    /// Per-target input container width (pixels) for scroll computation
    edit_input_widths: std::collections::HashMap<EnvEditTarget, f32>,
    /// Subscription that clears active_edit when edit_focus loses focus
    _edit_blur_sub: Option<Subscription>,
    /// Currently selected collection item path
    selected_item: Option<PathBuf>,
    /// Context menu state (path, position)
    context_menu: Option<(PathBuf, Point<Pixels>)>,
    /// Item being renamed
    renaming_item: Option<PathBuf>,
    /// Rename input text
    rename_text: String,
    /// Width of KEY column in the env variable table
    env_col_key_w: f32,
    /// Active env column drag: (start_x, start_width)
    env_col_drag: Option<(f32, f32)>,
    /// Height of collections section when expanded
    collections_h: f32,
    /// Height of env editor when open
    env_h: f32,
    /// Active collections drag: (start_mouse_y, start_collections_h)
    drag_coll: Option<(f32, f32)>,
    /// Active env drag: (start_mouse_y, start_env_h)
    drag_env: Option<(f32, f32)>,
    /// Handle to main window for showing full-screen modals
    main_window: WeakEntity<MainWindow>,
    /// Panel bounds (origin + size) captured each frame via canvas
    panel_bounds: gpui::Bounds<Pixels>,
    /// Scroll offset (pixels) for the collections tree
    tree_scroll: f32,
}

impl ExplorerPanel {
    pub fn new(cx: &mut Context<Self>, main_window: WeakEntity<MainWindow>) -> Self {
        Self {
            request_panel: None,
            history_expanded: true,
            collections_expanded: true,
            workspace_path: None,
            workspace_watcher: None,
            collection_items: Vec::new(),
            env_state: EnvironmentState::new(),
            env_dropdown_open: false,
            env_editor_open: false,
            active_edit: None,
            edit_selection: 0..0,
            edit_focus: cx.focus_handle(),
            new_env_name: String::new(),
            show_new_env_input: false,
            edit_undo_stack: Vec::new(),
            edit_redo_stack: Vec::new(),
            edit_is_selecting: false,
            edit_input_origins: std::collections::HashMap::new(),
            edit_scroll_offsets: std::collections::HashMap::new(),
            edit_input_widths: std::collections::HashMap::new(),
            _edit_blur_sub: None,
            selected_item: None,
            context_menu: None,
            renaming_item: None,
            rename_text: String::new(),
            env_col_key_w: 90.0,
            env_col_drag: None,
            collections_h: crate::prefs::get_f32("explorer.collections_h", 220.0),
            env_h: crate::prefs::get_f32("explorer.env_h", 200.0),
            drag_coll: None,
            drag_env: None,
            main_window,
            panel_bounds: gpui::Bounds::default(),
            tree_scroll: 0.0,
        }
    }

    /// Set the request panel reference for loading history items
    pub fn set_request_panel(
        &mut self,
        request_panel: Entity<RequestPanel>,
        cx: &mut Context<Self>,
    ) {
        self.request_panel = Some(request_panel);
        cx.notify();
    }

    /// Get the current environment state for variable substitution
    pub fn env_state(&self) -> &EnvironmentState {
        &self.env_state
    }

    /// Set a variable in the active environment (for @set extraction)
    pub fn set_env_variable(&mut self, name: &str, value: &str, cx: &mut Context<Self>) {
        if let Some(env) = self.env_state.active_mut() {
            env.set(name, value);
            cx.notify();
        }
    }

    fn toggle_history(&mut self, cx: &mut Context<Self>) {
        self.history_expanded = !self.history_expanded;
        cx.notify();
    }

    fn load_history_item(&mut self, entry_id: u64, cx: &mut Context<Self>) {
        // Read the entry data from history
        let entry_data: Option<(String, String, Vec<(String, String)>, Option<String>)> =
            cx.read_global::<RequestHistory, _>(|history, _| {
                history.get(entry_id).map(|entry| {
                    (
                        entry.method.clone(),
                        entry.url.clone(),
                        entry.headers.clone(),
                        entry.body.clone(),
                    )
                })
            });

        if let Some((method, url, headers, body)) = entry_data {
            if let Some(request_panel) = &self.request_panel {
                request_panel.update(cx, |panel, cx| {
                    panel.load_from_history(method, url, headers, body, cx);
                });
            }
        }
    }

    fn get_history_entries(&self, cx: &Context<Self>) -> Vec<HistoryEntry> {
        cx.read_global::<RequestHistory, _>(|history, _| history.entries().to_vec())
    }

    fn toggle_env_dropdown(&mut self, cx: &mut Context<Self>) {
        self.env_dropdown_open = !self.env_dropdown_open;
        if self.env_dropdown_open {
            self.env_editor_open = false;
        }
        cx.notify();
    }

    fn toggle_env_editor(&mut self, cx: &mut Context<Self>) {
        self.env_editor_open = !self.env_editor_open;
        if self.env_editor_open {
            self.env_dropdown_open = false;
        }
        cx.notify();
    }

    fn select_environment(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        self.env_state.set_active(index);
        self.env_dropdown_open = false;
        cx.notify();
    }

    fn add_variable(&mut self, cx: &mut Context<Self>) {
        if let Some(env) = self.env_state.active_mut() {
            let key = format!("var_{}", env.variables.len() + 1);
            env.set(key, "");
            cx.notify();
        }
    }

    fn remove_variable(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(env) = self.env_state.active_mut() {
            env.remove(key);
            cx.notify();
        }
    }

    fn start_new_env(&mut self, cx: &mut Context<Self>) {
        self.show_new_env_input = true;
        self.new_env_name = String::new();
        self.active_edit = Some(EnvEditTarget::NewEnvName);
        self.edit_selection = 0..0;
        cx.notify();
    }

    fn create_new_env(&mut self, cx: &mut Context<Self>) {
        if !self.new_env_name.trim().is_empty() {
            let env = Environment::new(self.new_env_name.trim());
            self.env_state.add_environment(env);
            // Select the new environment
            let new_index = self.env_state.environments.len() - 1;
            self.env_state.set_active(Some(new_index));
        }
        self.show_new_env_input = false;
        self.new_env_name.clear();
        self.active_edit = None;
        cx.notify();
    }

    fn cancel_new_env(&mut self, cx: &mut Context<Self>) {
        self.show_new_env_input = false;
        self.new_env_name.clear();
        self.active_edit = None;
        cx.notify();
    }

    /// Toggle collections section expanded/collapsed
    fn toggle_collections(&mut self, cx: &mut Context<Self>) {
        self.collections_expanded = !self.collections_expanded;
        cx.notify();
    }

    pub fn expand_section_collections(&mut self, cx: &mut Context<Self>) {
        self.collections_expanded = true;
        cx.notify();
    }

    pub fn expand_section_history(&mut self, cx: &mut Context<Self>) {
        self.history_expanded = true;
        cx.notify();
    }

    pub fn expand_section_env(&mut self, cx: &mut Context<Self>) {
        self.env_editor_open = true;
        cx.notify();
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

    /// Create a new .http file
    pub fn create_new_request(&mut self, cx: &mut Context<Self>) {
        // Default directory is workspace path or home
        let default_dir = self.workspace_path.clone().or_else(|| dirs::home_dir());

        let start_dir = last_paths::last_dir("new_request").or_else(|| last_paths::last_dir("save_request")).or(default_dir);
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
            // Ensure .http extension
            let path = if path.extension().is_none() || path.extension().unwrap() != "http" {
                path.with_extension("http")
            } else {
                path
            };

            // Create default content
            let content = "### New Request\n# @name new-request\n\nGET https://api.example.com\n";

            if fs::write(&path, content).is_ok() {
                // If parent is in workspace, refresh collection
                if let Some(workspace) = &self.workspace_path {
                    if path.starts_with(workspace) {
                        self.collection_items = self.scan_directory(workspace);
                    }
                }

                // Load the new file into request panel
                self.load_request_file(path, cx);
            }
        }
    }

    /// Load collection from a specific path
    pub fn load_collection_from_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path.is_dir() {
            self.workspace_path = Some(path.clone());
            self.collection_items = self.scan_directory(&path);
            // Start file watcher for auto-refresh
            match protide_core::workspace::Workspace::open(&path) {
                Ok(ws) => {
                    self.workspace_watcher = Some(Arc::new(ws));
                }
                Err(e) => {
                    eprintln!("File watcher failed: {}", e);
                }
            }
            cx.notify();
        }
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

    /// Import collection from Postman, cURL, or OpenAPI file
    pub fn import_collection(&mut self, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Import Collection")
            .add_filter(
                "All Supported",
                &["json", "yaml", "yml", "bru", "txt", "curl"],
            )
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
                        // Determine output directory
                        let output_dir = self
                            .workspace_path
                            .clone()
                            .or_else(|| file_path.parent().map(|p| p.to_path_buf()))
                            .unwrap_or_else(|| PathBuf::from("."));

                        // Create collection folder if we have a name
                        let collection_dir = if let Some(name) = &result.name {
                            let sanitized = sanitize_filename(name);
                            let dir = output_dir.join(&sanitized);
                            let _ = fs::create_dir_all(&dir);
                            dir
                        } else {
                            output_dir.clone()
                        };

                        // Write each request as a .http file
                        let mut created = 0;
                        for request in &result.requests {
                            let filename = request
                                .meta
                                .name
                                .as_ref()
                                .map(|n| sanitize_filename(n))
                                .unwrap_or_else(|| format!("request-{}", created + 1));

                            let filepath = collection_dir.join(format!("{}.http", filename));

                            if let Ok(content) = request_to_http_content(request) {
                                if fs::write(&filepath, content).is_ok() {
                                    created += 1;
                                }
                            }
                        }

                        // Reload collection if workspace is set
                        if self.workspace_path.is_some() {
                            self.collection_items = self.scan_directory(&output_dir);
                        } else {
                            self.load_collection_from_path(collection_dir, cx);
                        }

                        let mut msg = format!("Imported {} request(s).", created);
                        let modal_state = if !result.warnings.is_empty() {
                            msg.push_str(&format!(
                                "\n\n{} warning(s):\n{}",
                                result.warnings.len(),
                                result.warnings.join("\n")
                            ));
                            ModalState::warning("Import Complete", msg)
                        } else {
                            ModalState::info("Import Complete", msg)
                        };
                        if let Some(win) = self.main_window.upgrade() {
                            win.update(cx, |win, cx| win.show_modal(modal_state, cx));
                        }
                    }
                    Err(e) => {
                        if let Some(win) = self.main_window.upgrade() {
                            win.update(cx, |win, cx| {
                                win.show_modal(ModalState::error("Import Failed", e), cx)
                            });
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    /// Export the current collection as Markdown/HTML documentation.
    pub fn export_docs(&mut self, _cx: &mut Context<Self>) {
        let Some(workspace) = &self.workspace_path else {
            return;
        };

        let dialog = rfd::FileDialog::new()
            .set_title("Export API Documentation")
            .add_filter("Markdown", &["md"])
            .add_filter("HTML", &["html"])
            .set_file_name("api-docs.md");

        if let Some(save_path) = dialog.save_file() {
            let is_html = save_path.extension().and_then(|e| e.to_str()) == Some("html");
            let format = if is_html {
                protide_core::export::ExportFormat::Html
            } else {
                protide_core::export::ExportFormat::Markdown
            };

            let modal_state = match protide_core::export::export_collection(workspace, format) {
                Ok(content) => match std::fs::write(&save_path, &content) {
                    Ok(_) => ModalState::info(
                        "Export Complete",
                        format!("Documentation saved to {}", save_path.display()),
                    ),
                    Err(e) => {
                        ModalState::error("Export Failed", format!("Failed to write file: {}", e))
                    }
                },
                Err(e) => ModalState::error("Export Failed", e),
            };
            if let Some(win) = self.main_window.upgrade() {
                win.update(_cx, |win, cx| win.show_modal(modal_state, cx));
            }
        }
    }

    /// Recursively scan a directory for .http files and subdirectories
    fn scan_directory(&self, path: &PathBuf) -> Vec<CollectionItem> {
        let mut items = Vec::new();

        if let Ok(entries) = fs::read_dir(path) {
            let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            // Sort: folders first, then alphabetically
            entries.sort_by(|a, b| {
                let a_is_dir = a.path().is_dir();
                let b_is_dir = b.path().is_dir();
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            });

            for entry in entries {
                let entry_path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files/folders
                if name.starts_with('.') {
                    continue;
                }

                if entry_path.is_dir() {
                    let children = self.scan_directory(&entry_path);
                    // Only include folders that contain .http files (directly or in subfolders)
                    if !children.is_empty() || self.folder_has_http_files(&entry_path) {
                        items.push(CollectionItem {
                            name,
                            path: entry_path,
                            is_folder: true,
                            children,
                            method: None,
                            expanded: false,
                        });
                    }
                } else if entry_path.extension().is_some_and(|ext| ext == "http") {
                    let method = self.parse_method_from_file(&entry_path);
                    items.push(CollectionItem {
                        name,
                        path: entry_path,
                        is_folder: false,
                        children: Vec::new(),
                        method,
                        expanded: false,
                    });
                }
            }
        }

        items
    }

    /// Check if a folder contains any .http files
    fn folder_has_http_files(&self, path: &PathBuf) -> bool {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                if entry_path.is_file() && entry_path.extension().is_some_and(|ext| ext == "http") {
                    return true;
                }
            }
        }
        false
    }

    /// Parse the HTTP method from a .http file
    fn parse_method_from_file(&self, path: &PathBuf) -> Option<String> {
        if let Ok(content) = fs::read_to_string(path) {
            // Use http-parser to parse the file
            if let Ok(requests) = http_parser::parse(&content) {
                if let Some(req) = requests.first() {
                    return Some(format!("{:?}", req.method).to_uppercase());
                }
            }
        }
        None
    }

    /// Toggle a folder's expanded state
    fn toggle_collection_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        fn toggle_in_items(items: &mut [CollectionItem], target: &PathBuf) -> bool {
            for item in items.iter_mut() {
                if item.path == *target && item.is_folder {
                    item.expanded = !item.expanded;
                    return true;
                }
                if item.is_folder && toggle_in_items(&mut item.children, target) {
                    return true;
                }
            }
            false
        }

        toggle_in_items(&mut self.collection_items, &path);
        cx.notify();
    }

    /// Close the open project / workspace folder
    fn close_project(&mut self, cx: &mut Context<Self>) {
        self.workspace_path = None;
        self.collection_items.clear();
        self.collections_expanded = true;
        self.tree_scroll = 0.0;
        cx.notify();
    }

    /// Refresh collections by rescanning the workspace directory
    fn refresh_collections(&mut self, cx: &mut Context<Self>) {
        if let Some(workspace) = &self.workspace_path {
            self.collection_items = self.scan_directory(workspace);
            self.tree_scroll = 0.0;
            cx.notify();
        }
    }

    /// Load a .http file into the request panel
    fn load_request_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(requests) = http_parser::parse(&content) {
                if let Some(req) = requests.first() {
                    if let Some(request_panel) = &self.request_panel {
                        let method = req.method.as_str().to_string();
                        let url = req.url.clone();
                        let headers: Vec<(String, String)> = req
                            .headers
                            .iter()
                            .filter(|h| h.enabled)
                            .map(|h| (h.key.clone(), h.value.clone()))
                            .collect();
                        let body = req.body.clone();
                        let variable_extractions = req.meta.variable_extractions.clone();
                        let proto_path = req.meta.proto_path.clone().map(std::path::PathBuf::from);

                        request_panel.update(cx, |panel, cx| {
                            panel.load_from_history(method, url, headers, body, cx);
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
            }
        }
    }

    /// Delete a collection item — shows confirm modal in MainWindow.
    fn delete_collection_item(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let message = if path.is_dir() {
            format!("Delete folder '{}' and all its contents?", filename)
        } else {
            format!("Delete '{}'?", filename)
        };
        if let Some(win) = self.main_window.upgrade() {
            win.update(cx, |win, cx| {
                win.show_confirm_delete(ModalState::confirm("Confirm Delete", message), path, cx);
            });
        }
        self.context_menu = None;
        cx.notify();
    }

    /// Called by MainWindow when user confirms deletion.
    pub fn execute_delete(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let result = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        if result.is_ok() {
            self.refresh_collections(cx);
        }
    }

    /// Start renaming a collection item
    fn start_rename_item(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.rename_text = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.renaming_item = Some(path);
        self.context_menu = None;
        cx.notify();
    }

    /// Complete the rename operation
    fn complete_rename(&mut self, cx: &mut Context<Self>) {
        if let Some(old_path) = self.renaming_item.take() {
            let new_name = self.rename_text.trim();
            if !new_name.is_empty() {
                let old_name = old_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if new_name != old_name {
                    if let Some(parent) = old_path.parent() {
                        let new_path = parent.join(new_name);
                        if fs::rename(&old_path, &new_path).is_ok() {
                            self.refresh_collections(cx);
                        }
                    }
                }
            }
        }
        self.rename_text.clear();
        cx.notify();
    }

    /// Cancel the rename operation
    fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming_item = None;
        self.rename_text.clear();
        cx.notify();
    }

    /// Close context menu
    fn close_context_menu(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        cx.notify();
    }

    /// Render context menu for collection item
    fn render_context_menu(
        &self,
        path: PathBuf,
        position: Point<Pixels>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let path_for_rename = path.clone();
        let path_for_delete = path.clone();
        let is_folder = path.is_dir();

        const MENU_W: f32 = 140.0;
        const MENU_H: f32 = 64.0; // 2 items × 28px + 8px padding
        // event.position is in window coords; subtract panel origin to get panel-local coords
        let x = f32::from(position.x) - f32::from(self.panel_bounds.origin.x);
        let y = f32::from(position.y) - f32::from(self.panel_bounds.origin.y);
        let panel_w = f32::from(self.panel_bounds.size.width);
        let panel_h = f32::from(self.panel_bounds.size.height);
        // Flip left if would overflow right edge; flip up if would overflow bottom edge
        let left = if x + MENU_W > panel_w { px((x - MENU_W).max(0.0)) } else { px(x) };
        let top  = if y + MENU_H > panel_h { px((y - MENU_H).max(0.0)) } else { px(y) };

        div()
            .absolute()
            .left(left)
            .top(top)
            .w(px(MENU_W))
            .bg(theme.colors.bg_secondary)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .overflow_hidden()
            // Stop propagation so backdrop's on_mouse_down doesn't close menu before items fire
            .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                cx.stop_propagation();
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    // Rename option
                    .child(
                        div()
                            .id("context-menu-rename")
                            .w_full()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .gap(px(8.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                this.start_rename_item(path_for_rename.clone(), cx);
                            }))
                            .child(icon(ICON_EDIT, ICON_MD, theme.colors.text_muted))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_primary)
                                    .child("Rename"),
                            ),
                    )
                    // Delete option
                    .child(
                        div()
                            .id("context-menu-delete")
                            .w_full()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .gap(px(8.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.status_client_error.opacity(0.1)))
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                this.delete_collection_item(path_for_delete.clone(), cx);
                            }))
                            .child(icon(ICON_DELETE, ICON_MD, theme.colors.status_client_error))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.status_client_error)
                                    .child(if is_folder { "Delete Folder" } else { "Delete" }),
                            ),
                    ),
            )
    }

    fn delete_environment(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.env_state.environments.len() > 1 {
            self.env_state.remove_environment(index);
            cx.notify();
        }
    }

    fn start_editing(
        &mut self,
        target: EnvEditTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_edit = Some(target);
        let text_len = self.get_edit_text(target).chars().count();
        self.edit_selection = text_len..text_len;
        self.edit_focus.focus(window, cx);
        self._edit_blur_sub = Some(cx.on_blur(&self.edit_focus, window, |this, _, cx| {
            this.stop_editing(cx);
        }));
        cx.notify();
    }

    fn stop_editing(&mut self, cx: &mut Context<Self>) {
        self.active_edit = None;
        self.edit_selection = 0..0;
        self.edit_is_selecting = false;
        cx.notify();
    }

    /// Calculate character index from x position
    fn edit_index_for_x(&self, x: f32, char_width: f32) -> usize {
        if let Some(target) = self.active_edit {
            let char_count = self.get_edit_text(target).chars().count();
            if x <= 0.0 {
                0
            } else {
                let approx_char = (x / char_width) as usize;
                approx_char.min(char_count)
            }
        } else {
            0
        }
    }

    /// Handle mouse down for text input fields
    fn handle_edit_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        target: EnvEditTarget,
        char_width: f32,
        cx: &mut Context<Self>,
    ) {
        self.edit_is_selecting = true;
        let text_start_x = self.edit_input_origins.get(&target).copied().unwrap_or(0.0);
        let click_x = (f32::from(event.position.x) - text_start_x).max(0.0);
        let index = self.edit_index_for_x(click_x.max(0.0), char_width);

        // Cycle: 1=cursor, 2=word, 3=all, 4+=cursor
        let effective_click = if event.click_count >= 4 {
            1
        } else {
            event.click_count
        };

        match effective_click {
            2 => {
                // Double-click: select word
                let text = self.get_edit_text(target);
                let start = crate::ui::components::find_word_start(&text, index);
                let end = crate::ui::components::find_word_end(&text, index);
                self.edit_selection = start..end;
                cx.notify();
            }
            3 => {
                // Triple-click: select all
                let text_len = self.get_edit_text(target).chars().count();
                self.edit_selection = 0..text_len;
                cx.notify();
            }
            _ => {
                // Single click
                self.edit_selection = index..index;
                cx.notify();
            }
        }
    }

    /// Handle mouse move for text selection
    fn handle_edit_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        char_width: f32,
        cx: &mut Context<Self>,
    ) {
        if self.edit_is_selecting {
            let text_start_x = self
                .active_edit
                .and_then(|t| self.edit_input_origins.get(&t).copied())
                .unwrap_or(0.0);
            let click_x = f32::from(event.position.x) - text_start_x;
            let index = self.edit_index_for_x(click_x.max(0.0), char_width);
            self.edit_selection.end = index;
            cx.notify();
        }
    }

    /// Handle mouse up
    fn handle_edit_mouse_up(&mut self, _event: &MouseUpEvent, _cx: &mut Context<Self>) {
        self.edit_is_selecting = false;
    }

    fn get_edit_text(&self, target: EnvEditTarget) -> String {
        match target {
            EnvEditTarget::VarKey(i) => self
                .env_state
                .active()
                .and_then(|e| e.variables.keys().nth(i).cloned())
                .unwrap_or_default(),
            EnvEditTarget::VarValue(i) => self
                .env_state
                .active()
                .and_then(|e| e.variables.values().nth(i).cloned())
                .unwrap_or_default(),
            EnvEditTarget::NewEnvName => self.new_env_name.clone(),
        }
    }

    fn set_edit_text(&mut self, target: EnvEditTarget, text: String) {
        match target {
            EnvEditTarget::VarKey(i) => {
                if let Some(env) = self.env_state.active_mut() {
                    // Get old key and value
                    if let Some((old_key, value)) = env
                        .variables
                        .iter()
                        .nth(i)
                        .map(|(k, v)| (k.clone(), v.clone()))
                    {
                        env.variables.remove(&old_key);
                        if !text.is_empty() {
                            env.variables.insert(text, value);
                        }
                    }
                }
            }
            EnvEditTarget::VarValue(i) => {
                if let Some(env) = self.env_state.active_mut() {
                    if let Some(key) = env.variables.keys().nth(i).cloned() {
                        env.variables.insert(key, text);
                    }
                }
            }
            EnvEditTarget::NewEnvName => {
                self.new_env_name = text;
            }
        }
    }

    fn save_edit_state(&mut self) {
        if let Some(target) = self.active_edit {
            let text = self.get_edit_text(target);
            self.edit_undo_stack
                .push((target, text, self.edit_selection.clone()));
            if self.edit_undo_stack.len() > 100 {
                self.edit_undo_stack.remove(0);
            }
            self.edit_redo_stack.clear();
        }
    }

    fn edit_undo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_undo_stack.pop() {
            let current_text = self.get_edit_text(target);
            self.edit_redo_stack
                .push((target, current_text, self.edit_selection.clone()));
            self.set_edit_text(target, text);
            self.edit_selection = selection;
            cx.notify();
        }
    }

    fn edit_redo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_redo_stack.pop() {
            let current_text = self.get_edit_text(target);
            self.edit_undo_stack
                .push((target, current_text, self.edit_selection.clone()));
            self.set_edit_text(target, text);
            self.edit_selection = selection;
            cx.notify();
        }
    }

    fn handle_edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        // Handle rename mode first
        if self.renaming_item.is_some() {
            let key = event.keystroke.key.as_str();
            match key {
                "escape" => {
                    self.cancel_rename(cx);
                    return;
                }
                "enter" => {
                    self.complete_rename(cx);
                    return;
                }
                "backspace" => {
                    self.rename_text.pop();
                    cx.notify();
                    return;
                }
                _ if key.len() == 1 && !event.keystroke.modifiers.control => {
                    self.rename_text.push_str(key);
                    cx.notify();
                    return;
                }
                _ => return,
            }
        }

        let Some(target) = self.active_edit else {
            return;
        };

        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;

        if ctrl {
            match key {
                "a" => {
                    let text_len = self.get_edit_text(target).len();
                    self.edit_selection = 0..text_len;
                    cx.notify();
                    return;
                }
                "c" => {
                    if self.edit_selection.start != self.edit_selection.end {
                        let text = self.get_edit_text(target);
                        let start = self.edit_selection.start.min(self.edit_selection.end);
                        let end = self.edit_selection.start.max(self.edit_selection.end);
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            text[start..end].to_string(),
                        ));
                    }
                    return;
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard() {
                        if let Some(paste_text) = item.text() {
                            let paste_text = paste_text.replace('\n', "");
                            self.insert_text(target, &paste_text, cx);
                        }
                    }
                    return;
                }
                "z" => {
                    if event.keystroke.modifiers.shift {
                        self.edit_redo(cx);
                    } else {
                        self.edit_undo(cx);
                    }
                    return;
                }
                "y" => {
                    self.edit_redo(cx);
                    return;
                }
                _ => {}
            }
        }

        match key {
            "escape" => {
                if target == EnvEditTarget::NewEnvName {
                    self.cancel_new_env(cx);
                } else {
                    self.stop_editing(cx);
                }
            }
            "enter" => {
                if target == EnvEditTarget::NewEnvName {
                    self.create_new_env(cx);
                } else {
                    self.stop_editing(cx);
                }
            }
            "backspace" => {
                let mut text = self.get_edit_text(target);
                if self.edit_selection.start != self.edit_selection.end {
                    self.save_edit_state();
                    let start = self.edit_selection.start.min(self.edit_selection.end);
                    let end = self.edit_selection.start.max(self.edit_selection.end);
                    text.replace_range(start..end, "");
                    self.edit_selection = start..start;
                    self.set_edit_text(target, text);
                    self.update_env_scroll(target);
                    cx.notify();
                } else if self.edit_selection.end > 0 {
                    self.save_edit_state();
                    let pos = self.edit_selection.end - 1;
                    text.remove(pos);
                    self.edit_selection = pos..pos;
                    self.set_edit_text(target, text);
                    self.update_env_scroll(target);
                    cx.notify();
                }
            }
            "left" => {
                if self.edit_selection.end > 0 {
                    self.edit_selection.end -= 1;
                    if !event.keystroke.modifiers.shift {
                        self.edit_selection.start = self.edit_selection.end;
                    }
                    self.update_env_scroll(target);
                    cx.notify();
                }
            }
            "right" => {
                let text_len = self.get_edit_text(target).len();
                if self.edit_selection.end < text_len {
                    self.edit_selection.end += 1;
                    if !event.keystroke.modifiers.shift {
                        self.edit_selection.start = self.edit_selection.end;
                    }
                    self.update_env_scroll(target);
                    cx.notify();
                }
            }
            _ => {
                if let Some(ch) = &event.keystroke.key_char {
                    self.insert_text(target, ch, cx);
                }
            }
        }
    }

    fn insert_text(&mut self, target: EnvEditTarget, text: &str, cx: &mut Context<Self>) {
        self.save_edit_state();
        let mut current = self.get_edit_text(target);

        // Delete selection if any
        if self.edit_selection.start != self.edit_selection.end {
            let start = self.edit_selection.start.min(self.edit_selection.end);
            let end = self.edit_selection.start.max(self.edit_selection.end);
            current.replace_range(start..end, "");
            self.edit_selection = start..start;
        }

        let pos = self.edit_selection.end;
        current.insert_str(pos, text);
        let new_pos = pos + text.len();
        self.edit_selection = new_pos..new_pos;
        self.set_edit_text(target, current);
        self.update_env_scroll(target);
        cx.notify();
    }

    fn update_env_scroll(&mut self, target: EnvEditTarget) {
        let char_width = 11.0 * 0.6; // font_size 11 * 0.6 monospace ratio
        let padding = 6.0 * 2.0;     // px(6) each side
        let container_width = self.edit_input_widths.get(&target).copied().unwrap_or(200.0);
        let visible_width = (container_width - padding).max(40.0);
        let cursor_pos = self.edit_selection.end;
        let cursor_px = cursor_pos as f32 * char_width;
        let current_offset = self.edit_scroll_offsets.entry(target).or_insert(0.0);

        if cursor_px < *current_offset {
            *current_offset = cursor_px;
        } else if cursor_px > *current_offset + visible_width - char_width {
            *current_offset = cursor_px - visible_width + char_width;
        }
        if *current_offset < 0.0 {
            *current_offset = 0.0;
        }
    }

    fn count_visible_items(items: &[CollectionItem]) -> usize {
        items.iter().map(|item| {
            1 + if item.is_folder && item.expanded {
                Self::count_visible_items(&item.children)
            } else {
                0
            }
        }).sum()
    }

    /// Flatten collection items into a list with depth information for rendering
    fn flatten_collection_items(
        &self,
        items: &[CollectionItem],
        depth: usize,
    ) -> Vec<(CollectionItem, usize)> {
        let mut result = Vec::new();
        for item in items {
            result.push((item.clone(), depth));
            if item.is_folder && item.expanded {
                result.extend(self.flatten_collection_items(&item.children, depth + 1));
            }
        }
        result
    }

    fn render_collections_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let has_collections = !self.collection_items.is_empty();
        let collections_expanded = self.collections_expanded;
        let workspace_name = self
            .workspace_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Collections".to_string());

        // Flatten items for non-recursive rendering
        let flattened_items = if has_collections && collections_expanded {
            self.flatten_collection_items(&self.collection_items, 0)
        } else {
            Vec::new()
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(8.0))
            // Collections header
            .child(
                div()
                    .id("collections-header")
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .mx(px(4.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_collections(cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(if collections_expanded {
                                icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                            } else {
                                icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                            })
                            .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_muted))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child(workspace_name),
                            ),
                    )
                    // Action buttons grouped on the right
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.0))
                            .child(
                                icon_btn("refresh-collections-btn", ICON_REFRESH, cx)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.refresh_collections(cx);
                                    })),
                            )
                            .child(
                                icon_btn("open-folder-btn", ICON_FOLDER_OPEN, cx)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_folder(cx);
                                    })),
                            )
                            .child(
                                icon_btn("import-collection-btn", ICON_ARROW_DOWN, cx)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.import_collection(cx);
                                    })),
                            )
                            .when(self.workspace_path.is_some(), |el| {
                                el.child(
                                    icon_btn("export-docs-btn", ICON_EXTERNAL, cx)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.export_docs(cx);
                                        })),
                                )
                                .child(
                                    div()
                                        .id("close-project-btn")
                                        .size(px(22.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .text_color(theme.colors.text_muted)
                                        .hover(|s| {
                                            s.bg(theme.colors.bg_tertiary)
                                                .text_color(theme.colors.status_client_error)
                                        })
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.close_project(cx);
                                        }))
                                        .child(icon(ICON_CLOSE, ICON_MD, theme.colors.text_muted)),
                                )
                            }),
                    ),
            )
            // Content
            .when(collections_expanded, |el| {
                if has_collections {
                    const ROW_H: f32 = 28.0;
                    // header (32) + outer pt (8) + content pt (4)
                    let viewport_h = (self.collections_h - 44.0).max(0.0);
                    let indexed: Vec<(usize, CollectionItem, usize)> = flattened_items
                        .into_iter()
                        .enumerate()
                        .map(|(i, (item, depth))| (i, item, depth))
                        .collect();
                    let total_items = indexed.len();
                    let max_scroll = (total_items as f32 * ROW_H - viewport_h).max(0.0);
                    let scroll = self.tree_scroll.min(max_scroll);
                    let start_idx = (scroll / ROW_H).floor() as usize;
                    let visible_count = (viewport_h / ROW_H).ceil() as usize + 2;
                    let end_idx = (start_idx + visible_count).min(total_items);
                    let top_h = start_idx as f32 * ROW_H;
                    let bot_h = (total_items - end_idx) as f32 * ROW_H;

                    el.child(
                        div()
                            .id("collections-tree")
                            .w_full()
                            .flex()
                            .flex_col()
                            .px(px(4.0))
                            .pt(px(4.0))
                            .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
                                let vp = (this.collections_h - 44.0).max(0.0);
                                let n = Self::count_visible_items(&this.collection_items);
                                let max = (n as f32 * ROW_H - vp).max(0.0);
                                let delta = f32::from(event.delta.pixel_delta(px(ROW_H)).y);
                                this.tree_scroll = (this.tree_scroll - delta).clamp(0.0, max);
                                cx.notify();
                            }))
                            .when(top_h > 0.0, |el| el.child(div().w_full().h(px(top_h))))
                            .children(
                                indexed[start_idx..end_idx]
                                    .iter()
                                    .map(|(idx, item, depth)| {
                                        self.render_collection_item_row(item.clone(), *depth, *idx, cx)
                                    }),
                            )
                            .when(bot_h > 0.0, |el| el.child(div().w_full().h(px(bot_h)))),
                    )
                } else {
                    // Empty state for collections
                    el.child(
                        div()
                            .w_full()
                            .px(px(16.0))
                            .py(px(20.0))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .size(px(40.0))
                                    .bg(theme.colors.bg_tertiary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_muted)),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child("No collections"),
                            )
                            .child(
                                div()
                                    .id("open-folder-empty-btn")
                                    .mt(px(4.0))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .cursor_pointer()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.accent)
                                    .border_1()
                                    .border_color(theme.colors.accent.opacity(0.3))
                                    .hover(|s| s.bg(theme.colors.accent.opacity(0.1)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_folder(cx);
                                    }))
                                    .child("Open Folder"),
                            ),
                    )
                }
            })
    }

    /// Render a single collection item row (non-recursive)
    fn render_collection_item_row(
        &self,
        item: CollectionItem,
        depth: usize,
        idx: usize,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let indent = px((depth * 16 + 8) as f32);
        let path = item.path.clone();
        let path_for_right_click = item.path.clone();
        let path_for_select = item.path.clone();
        let is_folder = item.is_folder;
        let is_expanded = item.expanded;
        let display_name = item.name.trim_end_matches(".http").to_string();
        let method = item.method.clone();
        let is_selected = self.selected_item.as_ref() == Some(&item.path);
        let is_renaming = self.renaming_item.as_ref() == Some(&item.path);

        div()
            .id(SharedString::from(format!("collection-item-{}", idx)))
            .w_full()
            .h(px(28.0))
            .flex()
            .items_center()
            .pl(indent)
            .pr(px(8.0))
            .gap(px(6.0))
            .cursor_pointer()
            .when(is_selected, |el| el.bg(theme.colors.accent.opacity(0.1)))
            .when(!is_selected, |el| {
                el.hover(|s| s.bg(theme.colors.bg_tertiary))
            })
            // Left click - select and load/toggle
            .on_click({
                cx.listener(move |this, _, _, cx| {
                    this.selected_item = Some(path_for_select.clone());
                    if is_folder {
                        this.toggle_collection_folder(path.clone(), cx);
                    } else {
                        this.load_request_file(path.clone(), cx);
                    }
                })
            })
            // Right click - show context menu
            .on_mouse_down(MouseButton::Right, {
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.selected_item = Some(path_for_right_click.clone());
                    this.context_menu = Some((path_for_right_click.clone(), event.position));
                    cx.notify();
                })
            })
            // Expand/collapse icon for folders
            .when(is_folder, |el| {
                el.child(
                    div()
                        .w(px(10.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if is_expanded {
                            icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                        } else {
                            icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                        }),
                )
            })
            // Spacer for files to align with folders
            .when(!is_folder, |el| el.child(div().w(px(10.0))))
            // Icon
            .child(if is_folder {
                icon(ICON_FOLDER, ICON_MD, theme.colors.text_muted)
            } else {
                icon(ICON_FILE, ICON_MD, theme.colors.accent)
            })
            // Method badge (for .http files)
            .when_some(method, |el, method| {
                let method_color = theme.method_color(&method);
                el.child(
                    div()
                        .min_w(px(36.0))
                        .h(px(16.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(method_color.opacity(0.12))
                        .child(
                            div()
                                .text_size(px(9.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(method_color)
                                .child(method),
                        ),
                )
            })
            // Name (or rename input)
            .when(!is_renaming, |el| {
                el.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_primary)
                        .child(display_name),
                )
            })
            .when(is_renaming, |el| {
                let rename_text = self.rename_text.clone();
                el.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .h(px(28.0))
                        .px(px(4.0))
                        .flex()
                        .items_center()
                        .bg(theme.colors.bg_tertiary)
                        .border_1()
                        .border_color(theme.colors.accent)
                        .overflow_hidden()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_primary)
                        .child(rename_text),
                )
            })
    }

    fn render_history_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let entries = self.get_history_entries(cx);
        let has_entries = !entries.is_empty();
        let entry_count = entries.len();

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(4.0))
            // Divider
            .child(
                div()
                    .mx(px(12.0))
                    .h(px(1.0))
                    .bg(theme.colors.border.opacity(0.5)),
            )
            // History header
            .child(
                div()
                    .id("history-header")
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .cursor_pointer()
                    .mx(px(4.0))
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_history(cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(if self.history_expanded {
                                icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                            } else {
                                icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted)
                            })
                            .child(icon(ICON_TIMER, ICON_MD, theme.colors.text_muted))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("History"),
                            ),
                    )
                    // Count badge
                    .when(entry_count > 0, |el| {
                        el.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .bg(theme.colors.accent.opacity(0.15))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.accent)
                                .child(format!("{}", entry_count)),
                        )
                    }),
            )
            // History items
            .when(self.history_expanded, |el| {
                el.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .px(px(4.0))
                        .pt(px(4.0))
                        // Empty state
                        .when(!has_entries, |el| {
                            el.child(
                                div()
                                    .w_full()
                                    .px(px(16.0))
                                    .py(px(20.0))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .size(px(40.0))
                                            .bg(theme.colors.bg_tertiary)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(icon(
                                                ICON_FILE,
                                                ICON_MD,
                                                theme.colors.text_muted,
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.text_secondary)
                                            .child("No history yet"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("Requests will appear here after sending"),
                                    ),
                            )
                        })
                        .children(entries.into_iter().map(|entry| {
                            let entry_id = entry.id;
                            let method = entry.method.clone();
                            let display_url = entry.display_url();
                            let method_color = theme.method_color(&method);
                            let status = entry.status;
                            let status_color = status.map(|s| theme.status_color(s));

                            div()
                                .id(gpui::ElementId::Name(
                                    format!("history-{}", entry_id).into(),
                                ))
                                .w_full()
                                .h(px(32.0))
                                .flex()
                                .items_center()
                                .px(px(8.0))
                                .gap(px(10.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.bg_tertiary))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.load_history_item(entry_id, cx);
                                }))
                                // Method badge
                                .child(
                                    div()
                                        .min_w(px(44.0))
                                        .h(px(18.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .bg(method_color.opacity(0.12))
                                        .child(
                                            div()
                                                .text_size(px(10.0))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(method_color)
                                                .child(method),
                                        ),
                                )
                                // URL
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_primary)
                                        .child(display_url),
                                )
                                // Status indicator
                                .when_some(status_color, |el, color| {
                                    el.child(div().size(px(7.0)).bg(color))
                                })
                        })),
                )
            })
    }

    fn render_environment_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let active_env_name = self
            .env_state
            .active()
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "No Environment".to_string());
        let env_dropdown_open = self.env_dropdown_open;
        let env_editor_open = self.env_editor_open;
        let has_active_env = self.env_state.active_index.is_some();
        let var_count = self
            .env_state
            .active()
            .map(|e| e.variables.len())
            .unwrap_or(0);

        div()
            .w_full()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_tertiary.opacity(0.3))
            // Section header
            .child(
                div()
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(icon(ICON_SETTINGS, ICON_MD, theme.colors.text_muted))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("ENVIRONMENT"),
                            ),
                    )
                    // Variable count badge
                    .when(var_count > 0, |el| {
                        el.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .bg(theme.colors.status_success.opacity(0.12))
                                .text_size(px(9.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.status_success)
                                .child(format!("{} vars", var_count)),
                        )
                    }),
            )
            // Environment selector row
            .child(
                div()
                    .h(px(40.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    // Left: env selector dropdown button
                    .child(
                        div()
                            .id("env-selector")
                            .flex_1()
                            .h(px(32.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(10.0))
                            .cursor_pointer()
                            .bg(theme.colors.bg_primary)
                            .border_1()
                            .border_color(if env_dropdown_open {
                                theme.colors.accent
                            } else {
                                theme.colors.border
                            })
                            .hover(|s| s.border_color(theme.colors.text_muted))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_env_dropdown(cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    // Status indicator
                                    .child(div().size(px(6.0)).bg(if has_active_env {
                                        theme.colors.status_success
                                    } else {
                                        theme.colors.text_muted.opacity(0.5)
                                    }))
                                    // Env name
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(if has_active_env {
                                                theme.colors.text_primary
                                            } else {
                                                theme.colors.text_muted
                                            })
                                            .child(active_env_name.clone()),
                                    ),
                            )
                            // Chevron
                            .child(if env_dropdown_open {
                                icon(ICON_CHEVRON_UP, ICON_SM, theme.colors.text_muted)
                            } else {
                                icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted)
                            }),
                    )
                    // Right: edit button
                    .child(
                        div()
                            .id("env-edit-btn")
                            .ml(px(8.0))
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(12.0))
                            .when(env_editor_open, |el| {
                                el.bg(theme.colors.accent.opacity(0.12))
                                    .text_color(theme.colors.accent)
                            })
                            .when(!env_editor_open, |el| {
                                el.text_color(theme.colors.text_muted).hover(|s| {
                                    s.bg(theme.colors.bg_tertiary)
                                        .text_color(theme.colors.text_primary)
                                })
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_env_editor(cx);
                            }))
                            .child(if env_editor_open {
                                icon(ICON_EDIT, ICON_MD, theme.colors.accent)
                            } else {
                                icon(ICON_EDIT, ICON_MD, theme.colors.text_muted)
                            }),
                    ),
            )
            // Dropdown menu
            .when(env_dropdown_open, |el| {
                el.child(self.render_env_dropdown(cx))
            })
            // Editor panel
            .when(env_editor_open, |el| el.child(self.render_env_editor(cx)))
    }

    fn render_env_dropdown(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let active_index = self.env_state.active_index;
        let envs: Vec<(usize, String, usize)> = self
            .env_state
            .environments
            .iter()
            .enumerate()
            .map(|(i, e)| (i, e.name.clone(), e.variables.len()))
            .collect();

        div()
            .id("env-dropdown")
            .max_h(px(200.0))
            .overflow_scroll()
            .mx(px(12.0))
            .mb(px(8.0))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_primary)
            .overflow_hidden()
            .flex()
            .flex_col()
            // "No Environment" option
            .child(
                div()
                    .id("env-option-none")
                    .w_full()
                    .px(px(12.0))
                    .py(px(8.0))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .border_b_1()
                    .border_color(theme.colors.border.opacity(0.5))
                    .when(active_index.is_none(), |el| {
                        el.bg(theme.colors.accent.opacity(0.08))
                    })
                    .when(active_index.is_some(), |el| {
                        el.hover(|s| s.bg(theme.colors.bg_tertiary))
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.select_environment(None, cx);
                    }))
                    // Icon
                    .child(div().size(px(6.0)).bg(theme.colors.text_muted.opacity(0.4)))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child("No Environment"),
                    ),
            )
            // Environment options
            .children(
                envs.into_iter()
                    .enumerate()
                    .map(|(idx, (i, name, var_count))| {
                        let is_active = active_index == Some(i);
                        let is_last = idx == self.env_state.environments.len() - 1;
                        div()
                            .id(SharedString::from(format!("env-option-{}", i)))
                            .w_full()
                            .px(px(12.0))
                            .py(px(8.0))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .when(!is_last, |el| {
                                el.border_b_1()
                                    .border_color(theme.colors.border.opacity(0.5))
                            })
                            .when(is_active, |el| el.bg(theme.colors.accent.opacity(0.08)))
                            .when(!is_active, |el| {
                                el.hover(|s| s.bg(theme.colors.bg_tertiary))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_environment(Some(i), cx);
                            }))
                            // Left: icon + name
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(div().size(px(6.0)).bg(theme.colors.status_success))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(if is_active {
                                                gpui::FontWeight::MEDIUM
                                            } else {
                                                gpui::FontWeight::NORMAL
                                            })
                                            .text_color(if is_active {
                                                theme.colors.text_primary
                                            } else {
                                                theme.colors.text_secondary
                                            })
                                            .child(name),
                                    ),
                            )
                            // Right: var count
                            .when(var_count > 0, |el| {
                                el.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(theme.colors.text_muted)
                                        .child(format!("{} vars", var_count)),
                                )
                            })
                    }),
            )
            // Add new environment button
            .child(
                div()
                    .id("env-add-btn")
                    .w_full()
                    .px(px(12.0))
                    .py(px(8.0))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_tertiary.opacity(0.3))
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.env_dropdown_open = false;
                        this.env_editor_open = true;
                        this.start_new_env(cx);
                    }))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.accent)
                            .child("+"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.accent)
                            .child("New Environment"),
                    ),
            )
    }

    fn render_env_editor(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let show_new_env_input = self.show_new_env_input;
        let new_env_name = self.new_env_name.clone();
        let is_editing_new_env = self.active_edit == Some(EnvEditTarget::NewEnvName);
        let edit_selection = self.edit_selection.clone();

        let vars: Vec<(usize, String, String)> = self
            .env_state
            .active()
            .map(|e| {
                e.variables
                    .iter()
                    .enumerate()
                    .map(|(i, (k, v))| (i, k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let env_count = self.env_state.environments.len();
        let active_index = self.env_state.active_index;
        let has_vars = !vars.is_empty();

        div()
            .id("env-editor")
            .w_full()
            .h(px(self.env_h))
            .overflow_scroll()
            .px(px(12.0))
            .pb(px(12.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            // New environment input
            .when(show_new_env_input, |el| {
                el.child(
                    div()
                        .w_full()
                        .p(px(10.0))
                        .bg(theme.colors.bg_primary)
                        .border_1()
                        .border_color(theme.colors.accent.opacity(0.3))
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.text_secondary)
                                .child("Create New Environment"),
                        )
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(self.render_text_input(
                                    "new-env-name",
                                    EnvEditTarget::NewEnvName,
                                    &new_env_name,
                                    "Environment name",
                                    is_editing_new_env,
                                    if is_editing_new_env {
                                        edit_selection.clone()
                                    } else {
                                        0..0
                                    },
                                    cx,
                                ))
                                .child(
                                    div()
                                        .id("create-env-btn")
                                        .h(px(28.0))
                                        .px(px(10.0))
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .text_size(px(11.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .bg(theme.colors.accent)
                                        .text_color(theme.colors.bg_primary)
                                        .hover(|s| s.bg(theme.colors.accent.opacity(0.9)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.create_new_env(cx);
                                        }))
                                        .child("Create"),
                                )
                                .child(
                                    div()
                                        .id("cancel-env-btn")
                                        .h(px(28.0))
                                        .px(px(10.0))
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .text_size(px(11.0))
                                        .text_color(theme.colors.text_secondary)
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.cancel_new_env(cx);
                                        }))
                                        .child("Cancel"),
                                ),
                        ),
                )
            })
            // Variables section card
            .when(self.env_state.active_index.is_some(), |el| {
                el.child(
                    div()
                        .w_full()
                        .bg(theme.colors.bg_primary)
                        .border_1()
                        .border_color(theme.colors.border)
                        .overflow_hidden()
                        // Variables header
                        .child(
                            div()
                                .w_full()
                                .h(px(32.0))
                                .flex()
                                .items_center()
                                .justify_between()
                                .px(px(10.0))
                                .bg(theme.colors.bg_tertiary.opacity(0.3))
                                .border_b_1()
                                .border_color(theme.colors.border.opacity(0.5))
                                // Table headers
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.0))
                                        .bg(theme.colors.bg_secondary)
                                        .py(px(6.0))
                                        .child(
                                            div()
                                                .w(px(self.env_col_key_w))
                                                .text_size(px(10.0))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(theme.colors.text_muted)
                                                .child("KEY"),
                                        )
                                        .child(self.render_env_col_drag_handle(cx))
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_size(px(10.0))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(theme.colors.text_muted)
                                                .child("VALUE"),
                                        ),
                                )
                                // Trash icon to delete environment (only when >1 env exists)
                                .when(env_count > 1 && active_index.is_some(), |el| {
                                    let idx = active_index.unwrap();
                                    el.child(
                                        div()
                                            .id("delete-env-btn")
                                            .size(px(22.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .hover(|s| {
                                                s.bg(theme.colors.status_client_error.opacity(0.1))
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.delete_environment(idx, cx);
                                            }))
                                            .child(icon(
                                                ICON_DELETE,
                                                ICON_SM,
                                                theme.colors.text_muted.opacity(0.5),
                                            )),
                                    )
                                }),
                        )
                        // Variables list
                        .children(vars.into_iter().enumerate().map(|(idx, (i, key, value))| {
                            let is_editing_key = self.active_edit == Some(EnvEditTarget::VarKey(i));
                            let is_editing_value =
                                self.active_edit == Some(EnvEditTarget::VarValue(i));
                            let key_for_remove = key.clone();

                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .px(px(10.0))
                                .py(px(6.0))
                                .when(idx % 2 == 0, |el| {
                                    el.bg(theme.colors.bg_tertiary.opacity(0.2))
                                })
                                // Key input
                                .child(self.render_text_input_w(
                                    format!("var-key-{}", i),
                                    EnvEditTarget::VarKey(i),
                                    &key,
                                    "Key",
                                    is_editing_key,
                                    if is_editing_key {
                                        edit_selection.clone()
                                    } else {
                                        0..0
                                    },
                                    self.env_col_key_w,
                                    cx,
                                ))
                                .child(div().w(px(4.0)))
                                // Value input
                                .child(self.render_text_input(
                                    format!("var-value-{}", i),
                                    EnvEditTarget::VarValue(i),
                                    &value,
                                    "Value",
                                    is_editing_value,
                                    if is_editing_value {
                                        edit_selection.clone()
                                    } else {
                                        0..0
                                    },
                                    cx,
                                ))
                                // Remove button
                                .child(
                                    div()
                                        .id(SharedString::from(format!("var-remove-{}", i)))
                                        .size(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_muted.opacity(0.5))
                                        .hover(|s| {
                                            s.bg(theme.colors.status_client_error.opacity(0.1))
                                                .text_color(theme.colors.status_client_error)
                                        })
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.remove_variable(&key_for_remove, cx);
                                        }))
                                        .child(icon(
                                            ICON_CLOSE,
                                            ICON_SM,
                                            theme.colors.text_muted.opacity(0.5),
                                        )),
                                )
                        }))
                        // Ghost "+ Add variable" row at bottom of list
                        .when(has_vars, |el| {
                            el.child(
                                div()
                                    .id("add-var-ghost")
                                    .w_full()
                                    .h(px(32.0))
                                    .flex()
                                    .items_center()
                                    .px(px(10.0))
                                    .gap(px(6.0))
                                    .cursor_pointer()
                                    .border_t_1()
                                    .border_color(theme.colors.border.opacity(0.3))
                                    .hover(|s| s.bg(theme.colors.accent.opacity(0.06)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.add_variable(cx);
                                    }))
                                    .child(icon(
                                        ICON_PLUS,
                                        ICON_SM,
                                        theme.colors.accent.opacity(0.6),
                                    ))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.accent.opacity(0.6))
                                            .child("Add variable"),
                                    ),
                            )
                        })
                        // Empty state inside card
                        .when(!has_vars, |el| {
                            el.child(
                                div()
                                    .w_full()
                                    .py(px(16.0))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(18.0))
                                            .text_color(theme.colors.text_muted.opacity(0.5))
                                            .child("{ }"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("No variables defined"),
                                    )
                                    .child(
                                        div()
                                            .id("add-first-var-btn")
                                            .mt(px(4.0))
                                            .px(px(10.0))
                                            .py(px(4.0))
                                            .cursor_pointer()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.accent)
                                            .border_1()
                                            .border_color(theme.colors.accent.opacity(0.3))
                                            .hover(|s| s.bg(theme.colors.accent.opacity(0.1)))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.add_variable(cx);
                                            }))
                                            .child("+ Add Variable"),
                                    ),
                            )
                        }),
                )
            })
            // No environment selected state
            .when(
                self.env_state.active_index.is_none() && !show_new_env_input,
                |el| {
                    el.child(
                        div()
                            .w_full()
                            .py(px(16.0))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .size(px(36.0))
                                    .bg(theme.colors.bg_tertiary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icon(ICON_SETTINGS, ICON_MD, theme.colors.text_muted)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("Select an environment to edit"),
                            ),
                    )
                },
            )
            // Usage hint
            .child(
                div()
                    .w_full()
                    .mt(px(4.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .bg(theme.colors.bg_tertiary.opacity(0.5))
                    .flex()
                    .items_start()
                    .gap(px(6.0))
                    .child(icon(ICON_INFO, ICON_MD, theme.colors.text_muted))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Use {{var}} in URL, headers, body, or auth"),
                    ),
            )
    }

    fn render_env_col_drag_handle(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let start_w = self.env_col_key_w;
        div()
            .id("env-col-drag-handle")
            .w(px(4.0))
            .self_stretch()
            .cursor_col_resize()
            .bg(theme.colors.border.opacity(0.3))
            .hover(|s| s.bg(theme.colors.accent.opacity(0.5)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.env_col_drag = Some((f32::from(event.position.x), start_w));
                    cx.notify();
                }),
            )
    }

    fn render_text_input_w(
        &mut self,
        id: impl Into<SharedString>,
        target: EnvEditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        width: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(id.into())
            .w(px(width))
            .min_w(px(40.0))
            .h(px(28.0))
            .px(px(6.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
            .bg(theme.colors.bg_tertiary)
            .text_size(px(11.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, target, 6.6, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 6.6, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_edit_mouse_up(event, cx);
                }),
            )
            .child({
                let entity = cx.entity();
                canvas(
                    move |bounds, _, cx| {
                        let _ = entity.update(cx, |this, _| {
                            // canvas at padding edge; padding(6) to text content
                            this.edit_input_origins
                                .insert(target, f32::from(bounds.origin.x) + 6.0);
                            this.edit_input_widths
                                .insert(target, f32::from(bounds.size.width));
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full()
            })
            .child({
                // char_width = 11px * 0.6 = 6.6, padding = 2 * 6px = 12px
                let max_chars = (((width - 12.0) / 6.6).max(1.0)) as usize;
                let scroll = if is_editing { self.edit_scroll_offsets.get(&target).copied().unwrap_or(0.0) } else { 0.0 };
                render_text_view_with_max_scrolled(
                    &text,
                    &selection,
                    is_editing,
                    11.0,
                    theme.colors.text_primary,
                    Some(placeholder),
                    theme.colors.text_muted,
                    Some(max_chars),
                    theme.colors.accent.opacity(0.25),
                    scroll,
                )
            })
    }

    fn render_text_input(
        &mut self,
        id: impl Into<SharedString>,
        target: EnvEditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(id.into())
            .flex_1()
            .min_w(px(0.0))
            .h(px(28.0))
            .px(px(6.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
            .bg(theme.colors.bg_tertiary)
            .text_size(px(11.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.start_editing(target, window, cx);
                    // char_width for 11px font size
                    this.handle_edit_mouse_down(event, target, 6.6, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 6.6, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_edit_mouse_up(event, cx);
                }),
            )
            .child({
                let entity = cx.entity();
                canvas(
                    move |bounds, _, cx| {
                        let _ = entity.update(cx, |this, _| {
                            // canvas at padding edge; padding(6) to text content
                            this.edit_input_origins
                                .insert(target, f32::from(bounds.origin.x) + 6.0);
                            this.edit_input_widths
                                .insert(target, f32::from(bounds.size.width));
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full()
            })
            .child({
                let scroll = if is_editing { self.edit_scroll_offsets.get(&target).copied().unwrap_or(0.0) } else { 0.0 };
                self.render_text_content_scrolled(&text, placeholder, is_editing, selection, scroll, cx)
            })
    }

    fn render_text_content_scrolled(
        &self,
        text: &str,
        placeholder: &'static str,
        is_focused: bool,
        selection: Range<usize>,
        scroll_offset_x: f32,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        render_text_view_with_max_scrolled(
            text,
            &selection,
            is_focused,
            11.0,
            theme.colors.text_primary,
            Some(placeholder),
            theme.colors.text_muted,
            None,
            theme.colors.accent.opacity(0.25),
            scroll_offset_x,
        )
    }
}

impl Render for ExplorerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Poll file system changes on each render (cheap: just drains a channel)
        self.poll_workspace_changes(cx);
        let theme = theme::current(cx);

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_secondary)
            .track_focus(&self.edit_focus)
            .child({
                let entity = cx.entity();
                canvas(
                    move |bounds, _, cx| {
                        let _ = entity.update(cx, |this, _| {
                            this.panel_bounds = bounds;
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute().top_0().left_0().size_full()
            })
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_edit_key(event, cx);
            }))
            // Header
            .child(
                div()
                    .h(px(40.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(14.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .id("toggle-sidebar-btn")
                                    .size(px(22.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                                    .on_click({
                                        let main_window = self.main_window.clone();
                                        cx.listener(move |_this, _, _, cx| {
                                            if let Some(win) = main_window.upgrade() {
                                                win.update(cx, |w, cx| w.toggle_sidebar(cx));
                                            }
                                        })
                                    })
                                    .child(icon(ICON_MENU, ICON_MD, theme.colors.text_muted)),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("Explorer"),
                            ),
                    )
                    // New request button
                    .child(
                        div()
                            .id("new-request-btn")
                            .size(px(24.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(14.0))
                            .text_color(theme.colors.text_muted)
                            .hover(|s| {
                                s.bg(theme.colors.bg_tertiary)
                                    .text_color(theme.colors.text_primary)
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.create_new_request(cx);
                            }))
                            .child("+"),
                    ),
            )
            // Collections section (fixed height, resizable)
            .child(
                div()
                    .w_full()
                    .h(px(if self.collections_expanded { self.collections_h } else { 48.0 }))
                    .overflow_hidden()
                    .child(self.render_collections_section(cx)),
            )
            // Drag handle: collections / history
            .when(self.collections_expanded, |el| {
                el.child(
                    div()
                        .id("drag-handle-coll")
                        .w_full()
                        .h(px(4.0))
                        .cursor_row_resize()
                        .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, _, _| {
                                this.drag_coll =
                                    Some((f32::from(event.position.y), this.collections_h));
                            }),
                        ),
                )
            })
            // History section (flex_1, scrollable)
            .child(
                div()
                    .id("explorer-history-area")
                    .flex_1()
                    .w_full()
                    .overflow_scroll()
                    .child(self.render_history_section(cx)),
            )
            // Drag handle: history / environment
            .when(self.env_editor_open, |el| {
                el.child(
                    div()
                        .id("drag-handle-env")
                        .w_full()
                        .h(px(4.0))
                        .cursor_row_resize()
                        .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, _, _| {
                                this.drag_env = Some((f32::from(event.position.y), this.env_h));
                            }),
                        ),
                )
            })
            // Environment section
            .child(self.render_environment_section(cx))
            // Env column resize overlay
            .when(self.env_col_drag.is_some(), |el| {
                el.child(
                    deferred(
                        div()
                            .id("env-col-resize-overlay")
                            .absolute()
                            .top_0()
                            .left_0()
                            .w_full()
                            .h_full()
                            .cursor_col_resize()
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                                if let Some((start_x, start_w)) = this.env_col_drag {
                                    let delta = f32::from(event.position.x) - start_x;
                                    this.env_col_key_w = (start_w + delta).max(40.0).min(300.0);
                                    cx.notify();
                                }
                            }))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.env_col_drag = None;
                                    cx.notify();
                                }),
                            ),
                    )
                    .with_priority(1),
                )
            })
            // Collections drag overlay
            .when(self.drag_coll.is_some(), |el| {
                el.child(
                    deferred(
                        div()
                            .id("drag-coll-overlay")
                            .absolute()
                            .inset_0()
                            .cursor_row_resize()
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                                if let Some((start_y, start_h)) = this.drag_coll {
                                    let delta = f32::from(event.position.y) - start_y;
                                    this.collections_h = (start_h + delta).max(48.0).min(600.0);
                                    cx.notify();
                                }
                            }))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.drag_coll = None;
                                    crate::prefs::set_f32(
                                        "explorer.collections_h",
                                        this.collections_h,
                                    );
                                    cx.notify();
                                }),
                            ),
                    )
                    .with_priority(2),
                )
            })
            // Env drag overlay
            .when(self.drag_env.is_some(), |el| {
                el.child(
                    deferred(
                        div()
                            .id("drag-env-overlay")
                            .absolute()
                            .inset_0()
                            .cursor_row_resize()
                            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                                if let Some((start_y, start_h)) = this.drag_env {
                                    let delta = f32::from(event.position.y) - start_y;
                                    this.env_h = (start_h - delta).max(80.0).min(500.0);
                                    cx.notify();
                                }
                            }))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.drag_env = None;
                                    crate::prefs::set_f32("explorer.env_h", this.env_h);
                                    cx.notify();
                                }),
                            ),
                    )
                    .with_priority(2),
                )
            })
            // Context menu overlay
            .when_some(self.context_menu.clone(), |el, (path, position)| {
                el.child(
                    div()
                        .id("context-menu-backdrop")
                        .absolute()
                        .inset_0()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.close_context_menu(cx);
                            }),
                        )
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(|this, _, _, cx| {
                                this.close_context_menu(cx);
                            }),
                        ),
                )
                .child(self.render_context_menu(path, position, cx))
            })
    }
}

/// Sanitize a string to be used as a filename
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Convert a Request to .http file content
fn request_to_http_content(request: &http_parser::Request) -> Result<String, String> {
    let mut content = String::new();

    // Add name annotation if present
    if let Some(name) = &request.meta.name {
        content.push_str(&format!("# @name {}\n", name));
    }

    // Add description annotation if present
    if let Some(desc) = &request.meta.description {
        content.push_str(&format!(
            "# @description {}\n",
            desc.lines().next().unwrap_or("")
        ));
    }

    // Add blank line if we have annotations
    if !content.is_empty() {
        content.push('\n');
    }

    // Add request line
    content.push_str(&format!("{} {}\n", request.method.as_str(), request.url));

    // Add headers
    for header in &request.headers {
        if header.enabled {
            content.push_str(&format!("{}: {}\n", header.key, header.value));
        }
    }

    // Add body if present
    if let Some(body) = &request.body {
        content.push_str("\n");
        content.push_str(body);
        if !body.ends_with('\n') {
            content.push('\n');
        }
    }

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    // EnvEditTarget tests
    #[test]
    fn test_env_edit_target_var_key() {
        let target = EnvEditTarget::VarKey(0);
        assert_eq!(target, EnvEditTarget::VarKey(0));
        assert_ne!(target, EnvEditTarget::VarKey(1));
        assert_ne!(target, EnvEditTarget::VarValue(0));
    }

    #[test]
    fn test_env_edit_target_var_value() {
        let target = EnvEditTarget::VarValue(5);
        assert_eq!(target, EnvEditTarget::VarValue(5));
        assert_ne!(target, EnvEditTarget::VarValue(0));
        assert_ne!(target, EnvEditTarget::VarKey(5));
    }

    #[test]
    fn test_env_edit_target_new_env_name() {
        let target = EnvEditTarget::NewEnvName;
        assert_eq!(target, EnvEditTarget::NewEnvName);
        assert_ne!(target, EnvEditTarget::VarKey(0));
        assert_ne!(target, EnvEditTarget::VarValue(0));
    }

    #[test]
    fn test_env_edit_target_clone() {
        let target = EnvEditTarget::VarKey(3);
        let cloned = target;
        assert_eq!(target, cloned);
    }
}
