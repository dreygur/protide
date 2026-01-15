//! File explorer panel - displays the workspace file tree, request history, and environment selector

use gpui::{
    div, prelude::*, px, ClipboardItem, Context, Entity, FocusHandle, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Point, Pixels, Render, SharedString, Styled, Window,
};
use std::fs;
use std::ops::Range;
use std::path::PathBuf;

use crate::models::{Environment, EnvironmentState};
use crate::theme;
use crate::ui::components::render_text_view_with_max;
use super::history::{HistoryEntry, RequestHistory};
use super::request::RequestPanel;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    /// Currently selected collection item path
    selected_item: Option<PathBuf>,
    /// Context menu state (path, position)
    context_menu: Option<(PathBuf, Point<Pixels>)>,
    /// Item being renamed
    renaming_item: Option<PathBuf>,
    /// Rename input text
    rename_text: String,
}

impl ExplorerPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            request_panel: None,
            history_expanded: true,
            collections_expanded: true,
            workspace_path: None,
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
            selected_item: None,
            context_menu: None,
            renaming_item: None,
            rename_text: String::new(),
        }
    }

    /// Set the request panel reference for loading history items
    pub fn set_request_panel(&mut self, request_panel: Entity<RequestPanel>, cx: &mut Context<Self>) {
        self.request_panel = Some(request_panel);
        cx.notify();
    }

    /// Get the current environment state for variable substitution
    pub fn env_state(&self) -> &EnvironmentState {
        &self.env_state
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

    /// Open folder dialog and load collection
    pub fn open_folder(&mut self, cx: &mut Context<Self>) {
        // Use blocking file dialog (runs on main thread)
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Open Collection Folder")
            .pick_folder()
        {
            self.load_collection_from_path(folder, cx);
        }
    }

    /// Create a new .http file
    pub fn create_new_request(&mut self, cx: &mut Context<Self>) {
        // Default directory is workspace path or home
        let default_dir = self.workspace_path.clone()
            .or_else(|| dirs::home_dir());

        let mut dialog = rfd::FileDialog::new()
            .set_title("Create New Request")
            .set_file_name("new-request.http")
            .add_filter("HTTP Request", &["http"]);

        if let Some(dir) = default_dir {
            dialog = dialog.set_directory(dir);
        }

        if let Some(path) = dialog.save_file() {
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
            cx.notify();
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

    /// Refresh collections by rescanning the workspace directory
    fn refresh_collections(&mut self, cx: &mut Context<Self>) {
        if let Some(workspace) = &self.workspace_path {
            self.collection_items = self.scan_directory(workspace);
            cx.notify();
        }
    }

    /// Load a .http file into the request panel
    fn load_request_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(requests) = http_parser::parse(&content) {
                if let Some(req) = requests.first() {
                    if let Some(request_panel) = &self.request_panel {
                        let method = format!("{:?}", req.method).to_uppercase();
                        let url = req.url.clone();
                        let headers: Vec<(String, String)> = req.headers
                            .iter()
                            .filter(|h| h.enabled)
                            .map(|h| (h.key.clone(), h.value.clone()))
                            .collect();
                        let body = req.body.clone();

                        request_panel.update(cx, |panel, cx| {
                            panel.load_from_history(method, url, headers, body, cx);
                        });
                    }
                }
            }
        }
    }

    /// Delete a collection item (file or folder) with confirmation
    fn delete_collection_item(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let filename = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let is_folder = path.is_dir();
        let message = if is_folder {
            format!("Delete folder '{}' and all its contents?", filename)
        } else {
            format!("Delete '{}'?", filename)
        };

        let confirmed = rfd::MessageDialog::new()
            .set_title("Confirm Delete")
            .set_description(&message)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show() == rfd::MessageDialogResult::Yes;

        if confirmed {
            let result = if is_folder {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            };

            if result.is_ok() {
                self.refresh_collections(cx);
            }
        }

        self.context_menu = None;
        cx.notify();
    }

    /// Start renaming a collection item
    fn start_rename_item(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.rename_text = path.file_name()
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
                let old_name = old_path.file_name()
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
    fn render_context_menu(&self, path: PathBuf, position: Point<Pixels>, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let path_for_rename = path.clone();
        let path_for_delete = path.clone();
        let is_folder = path.is_dir();

        div()
            .absolute()
            .left(position.x)
            .top(position.y)
            .w(px(120.0))
            .bg(theme.colors.bg_secondary)
            .border_1()
            .border_color(theme.colors.border)
            .rounded(px(6.0))
            .shadow_lg()
            .overflow_hidden()
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
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.start_rename_item(path_for_rename.clone(), cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("✏️")
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_primary)
                                    .child("Rename")
                            )
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
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.delete_collection_item(path_for_delete.clone(), cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.status_client_error)
                                    .child("🗑️")
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.status_client_error)
                                    .child(if is_folder { "Delete Folder" } else { "Delete" })
                            )
                    )
            )
    }

    fn delete_environment(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.env_state.environments.len() > 1 {
            self.env_state.remove_environment(index);
            cx.notify();
        }
    }

    fn start_editing(&mut self, target: EnvEditTarget, window: &mut Window, cx: &mut Context<Self>) {
        self.active_edit = Some(target);
        let text_len = self.get_edit_text(target).len();
        self.edit_selection = text_len..text_len;
        self.edit_focus.focus(window, cx);
        cx.notify();
    }

    fn stop_editing(&mut self, cx: &mut Context<Self>) {
        self.active_edit = None;
        self.edit_selection = 0..0;
        cx.notify();
    }

    fn get_edit_text(&self, target: EnvEditTarget) -> String {
        match target {
            EnvEditTarget::VarKey(i) => {
                self.env_state.active()
                    .and_then(|e| e.variables.keys().nth(i).cloned())
                    .unwrap_or_default()
            }
            EnvEditTarget::VarValue(i) => {
                self.env_state.active()
                    .and_then(|e| e.variables.values().nth(i).cloned())
                    .unwrap_or_default()
            }
            EnvEditTarget::NewEnvName => self.new_env_name.clone(),
        }
    }

    fn set_edit_text(&mut self, target: EnvEditTarget, text: String) {
        match target {
            EnvEditTarget::VarKey(i) => {
                if let Some(env) = self.env_state.active_mut() {
                    // Get old key and value
                    if let Some((old_key, value)) = env.variables.iter().nth(i).map(|(k, v)| (k.clone(), v.clone())) {
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
            self.edit_undo_stack.push((target, text, self.edit_selection.clone()));
            if self.edit_undo_stack.len() > 100 {
                self.edit_undo_stack.remove(0);
            }
            self.edit_redo_stack.clear();
        }
    }

    fn edit_undo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_undo_stack.pop() {
            let current_text = self.get_edit_text(target);
            self.edit_redo_stack.push((target, current_text, self.edit_selection.clone()));
            self.set_edit_text(target, text);
            self.edit_selection = selection;
            cx.notify();
        }
    }

    fn edit_redo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_redo_stack.pop() {
            let current_text = self.get_edit_text(target);
            self.edit_undo_stack.push((target, current_text, self.edit_selection.clone()));
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
                        cx.write_to_clipboard(ClipboardItem::new_string(text[start..end].to_string()));
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
                    cx.notify();
                } else if self.edit_selection.end > 0 {
                    self.save_edit_state();
                    let pos = self.edit_selection.end - 1;
                    text.remove(pos);
                    self.edit_selection = pos..pos;
                    self.set_edit_text(target, text);
                    cx.notify();
                }
            }
            "left" => {
                if self.edit_selection.end > 0 {
                    self.edit_selection.end -= 1;
                    if !event.keystroke.modifiers.shift {
                        self.edit_selection.start = self.edit_selection.end;
                    }
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
        cx.notify();
    }

    /// Flatten collection items into a list with depth information for rendering
    fn flatten_collection_items(&self, items: &[CollectionItem], depth: usize) -> Vec<(CollectionItem, usize)> {
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
        let workspace_name = self.workspace_path.as_ref()
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
                    .rounded(px(4.0))
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_collections(cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(if collections_expanded { "▾" } else { "▸" })
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("📁")
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child(workspace_name)
                            )
                    )
                    // Refresh button
                    .child(
                        div()
                            .id("refresh-collections-btn")
                            .size(px(22.0))
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).text_color(theme.colors.text_primary))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh_collections(cx);
                            }))
                            .child("↻")
                    )
                    // Open folder button
                    .child(
                        div()
                            .id("open-folder-btn")
                            .size(px(22.0))
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).text_color(theme.colors.text_primary))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_folder(cx);
                            }))
                            .child("📂")
                    )
            )
            // Content
            .when(collections_expanded, |el| {
                if has_collections {
                    el.child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .px(px(4.0))
                            .pt(px(4.0))
                            .children(flattened_items.into_iter().enumerate().map(|(idx, (item, depth))| {
                                self.render_collection_item_row(item, depth, idx, cx)
                            }))
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
                                    .rounded(px(8.0))
                                    .bg(theme.colors.bg_tertiary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(18.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("📁")
                                    )
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child("No collections")
                            )
                            .child(
                                div()
                                    .id("open-folder-empty-btn")
                                    .mt(px(4.0))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
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
                                    .child("Open Folder")
                            )
                    )
                }
            })
    }

    /// Render a single collection item row (non-recursive)
    fn render_collection_item_row(&self, item: CollectionItem, depth: usize, idx: usize, cx: &Context<Self>) -> impl IntoElement {
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
            .rounded(px(4.0))
            .when(is_selected, |el| el.bg(theme.colors.accent.opacity(0.1)))
            .when(!is_selected, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary)))
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
                        .text_size(px(10.0))
                        .text_color(theme.colors.text_muted)
                        .w(px(10.0))
                        .child(if is_expanded { "▾" } else { "▸" })
                )
            })
            // Spacer for files to align with folders
            .when(!is_folder, |el| {
                el.child(div().w(px(10.0)))
            })
            // Icon
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(if is_folder { theme.colors.text_muted } else { theme.colors.accent })
                    .child(if is_folder { "📁" } else { "📄" })
            )
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
                        .rounded(px(3.0))
                        .bg(method_color.opacity(0.12))
                        .child(
                            div()
                                .text_size(px(9.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(method_color)
                                .child(method)
                        )
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
                        .child(display_name)
                )
            })
            .when(is_renaming, |el| {
                let rename_text = self.rename_text.clone();
                el.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .h(px(20.0))
                        .px(px(4.0))
                        .flex()
                        .items_center()
                        .bg(theme.colors.bg_tertiary)
                        .border_1()
                        .border_color(theme.colors.accent)
                        .rounded(px(3.0))
                        .overflow_hidden()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_primary)
                        .child(rename_text)
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
                    .bg(theme.colors.border.opacity(0.5))
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
                    .mt(px(8.0))
                    .cursor_pointer()
                    .rounded(px(4.0))
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
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(if self.history_expanded { "▾" } else { "▸" })
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("⏱")
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_secondary)
                                    .child("History")
                            )
                    )
                    // Count badge
                    .when(entry_count > 0, |el| {
                        el.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(10.0))
                                .bg(theme.colors.accent.opacity(0.15))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.accent)
                                .child(format!("{}", entry_count))
                        )
                    })
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
                                    .py(px(16.0))
                                    .flex()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("No requests yet")
                                    )
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
                                .id(gpui::ElementId::Name(format!("history-{}", entry_id).into()))
                                .w_full()
                                .h(px(30.0))
                                .flex()
                                .items_center()
                                .px(px(8.0))
                                .gap(px(10.0))
                                .cursor_pointer()
                                .rounded(px(4.0))
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
                                        .rounded(px(4.0))
                                        .bg(method_color.opacity(0.12))
                                        .child(
                                            div()
                                                .text_size(px(10.0))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(method_color)
                                                .child(method)
                                        )
                                )
                                // URL
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .overflow_hidden()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_primary)
                                        .child(display_url)
                                )
                                // Status indicator
                                .when_some(status_color, |el, color| {
                                    el.child(
                                        div()
                                            .size(px(7.0))
                                            .rounded_full()
                                            .bg(color)
                                    )
                                })
                        }))
                )
            })
    }

    fn render_environment_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let active_env_name = self.env_state.active()
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "No Environment".to_string());
        let env_dropdown_open = self.env_dropdown_open;
        let env_editor_open = self.env_editor_open;
        let has_active_env = self.env_state.active_index.is_some();
        let var_count = self.env_state.active()
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
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("⚙")
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_secondary)
                                    .child("ENVIRONMENT")
                            )
                    )
                    // Variable count badge
                    .when(var_count > 0, |el| {
                        el.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(8.0))
                                .bg(theme.colors.status_success.opacity(0.12))
                                .text_size(px(9.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.colors.status_success)
                                .child(format!("{} vars", var_count))
                        )
                    })
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
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(10.0))
                            .rounded(px(6.0))
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
                                    .child(
                                        div()
                                            .size(px(6.0))
                                            .rounded_full()
                                            .bg(if has_active_env {
                                                theme.colors.status_success
                                            } else {
                                                theme.colors.text_muted.opacity(0.5)
                                            })
                                    )
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
                                            .child(active_env_name.clone())
                                    )
                            )
                            // Chevron
                            .child(
                                div()
                                    .text_size(px(9.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(if env_dropdown_open { "▲" } else { "▼" })
                            )
                    )
                    // Right: edit button
                    .child(
                        div()
                            .id("env-edit-btn")
                            .ml(px(8.0))
                            .size(px(28.0))
                            .rounded(px(6.0))
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
                                el.text_color(theme.colors.text_muted)
                                    .hover(|s| s.bg(theme.colors.bg_tertiary).text_color(theme.colors.text_primary))
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_env_editor(cx);
                            }))
                            .child("✎")
                    )
            )
            // Dropdown menu
            .when(env_dropdown_open, |el| {
                el.child(self.render_env_dropdown(cx))
            })
            // Editor panel
            .when(env_editor_open, |el| {
                el.child(self.render_env_editor(cx))
            })
    }

    fn render_env_dropdown(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let active_index = self.env_state.active_index;
        let envs: Vec<(usize, String, usize)> = self.env_state.environments
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
            .rounded(px(8.0))
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
                    .child(
                        div()
                            .size(px(6.0))
                            .rounded_full()
                            .bg(theme.colors.text_muted.opacity(0.4))
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child("No Environment")
                    )
            )
            // Environment options
            .children(envs.into_iter().enumerate().map(|(idx, (i, name, var_count))| {
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
                    .when(!is_last, |el| el.border_b_1().border_color(theme.colors.border.opacity(0.5)))
                    .when(is_active, |el| {
                        el.bg(theme.colors.accent.opacity(0.08))
                    })
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
                            .child(
                                div()
                                    .size(px(6.0))
                                    .rounded_full()
                                    .bg(theme.colors.status_success)
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(if is_active { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                                    .text_color(if is_active { theme.colors.text_primary } else { theme.colors.text_secondary })
                                    .child(name)
                            )
                    )
                    // Right: var count
                    .when(var_count > 0, |el| {
                        el.child(
                            div()
                                .text_size(px(10.0))
                                .text_color(theme.colors.text_muted)
                                .child(format!("{} vars", var_count))
                        )
                    })
            }))
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
                            .child("+")
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.accent)
                            .child("New Environment")
                    )
            )
    }

    fn render_env_editor(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let show_new_env_input = self.show_new_env_input;
        let new_env_name = self.new_env_name.clone();
        let is_editing_new_env = self.active_edit == Some(EnvEditTarget::NewEnvName);
        let edit_selection = self.edit_selection.clone();

        let vars: Vec<(usize, String, String)> = self.env_state.active()
            .map(|e| e.variables.iter()
                .enumerate()
                .map(|(i, (k, v))| (i, k.clone(), v.clone()))
                .collect())
            .unwrap_or_default();

        let env_count = self.env_state.environments.len();
        let active_index = self.env_state.active_index;
        let has_vars = !vars.is_empty();

        div()
            .id("env-editor")
            .w_full()
            .max_h(px(300.0))
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
                        .rounded(px(8.0))
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
                                .child("Create New Environment")
                        )
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    self.render_text_input(
                                        "new-env-name",
                                        EnvEditTarget::NewEnvName,
                                        &new_env_name,
                                        "Environment name",
                                        is_editing_new_env,
                                        if is_editing_new_env { edit_selection.clone() } else { 0..0 },
                                        cx,
                                    )
                                )
                                .child(
                                    div()
                                        .id("create-env-btn")
                                        .px(px(10.0))
                                        .py(px(5.0))
                                        .rounded(px(6.0))
                                        .cursor_pointer()
                                        .text_size(px(11.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .bg(theme.colors.accent)
                                        .text_color(gpui::white())
                                        .hover(|s| s.bg(theme.colors.accent.opacity(0.9)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.create_new_env(cx);
                                        }))
                                        .child("Create")
                                )
                                .child(
                                    div()
                                        .id("cancel-env-btn")
                                        .px(px(10.0))
                                        .py(px(5.0))
                                        .rounded(px(6.0))
                                        .cursor_pointer()
                                        .text_size(px(11.0))
                                        .text_color(theme.colors.text_secondary)
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.cancel_new_env(cx);
                                        }))
                                        .child("Cancel")
                                )
                        )
                )
            })
            // Variables section card
            .when(self.env_state.active_index.is_some(), |el| {
                el.child(
                    div()
                        .w_full()
                        .rounded(px(8.0))
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
                                        .child(
                                            div()
                                                .w(px(90.0))
                                                .text_size(px(9.0))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(theme.colors.text_muted)
                                                .child("KEY")
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_size(px(9.0))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(theme.colors.text_muted)
                                                .child("VALUE")
                                        )
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            div()
                                                .id("add-var-btn")
                                                .px(px(8.0))
                                                .py(px(3.0))
                                                .rounded(px(4.0))
                                                .cursor_pointer()
                                                .text_size(px(10.0))
                                                .text_color(theme.colors.accent)
                                                .hover(|s| s.bg(theme.colors.bg_tertiary))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.add_variable(cx);
                                                }))
                                                .child("+ Add")
                                        )
                                        // Delete environment button
                                        .when(env_count > 1 && active_index.is_some(), |el| {
                                            let idx = active_index.unwrap();
                                            el.child(
                                                div()
                                                    .id("delete-env-btn")
                                                    .px(px(8.0))
                                                    .py(px(3.0))
                                                    .rounded(px(4.0))
                                                    .cursor_pointer()
                                                    .text_size(px(10.0))
                                                    .text_color(theme.colors.status_client_error.opacity(0.8))
                                                    .hover(|s| s.bg(theme.colors.status_client_error.opacity(0.1)).text_color(theme.colors.status_client_error))
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.delete_environment(idx, cx);
                                                    }))
                                                    .child("Delete")
                                            )
                                        })
                                )
                        )
                        // Variables list
                        .children(vars.into_iter().enumerate().map(|(idx, (i, key, value))| {
                            let is_editing_key = self.active_edit == Some(EnvEditTarget::VarKey(i));
                            let is_editing_value = self.active_edit == Some(EnvEditTarget::VarValue(i));
                            let key_for_remove = key.clone();

                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .px(px(10.0))
                                .py(px(6.0))
                                .when(idx % 2 == 0, |el| el.bg(theme.colors.bg_tertiary.opacity(0.2)))
                                // Key input
                                .child(
                                    self.render_text_input(
                                        format!("var-key-{}", i),
                                        EnvEditTarget::VarKey(i),
                                        &key,
                                        "Key",
                                        is_editing_key,
                                        if is_editing_key { edit_selection.clone() } else { 0..0 },
                                        cx,
                                    )
                                )
                                // Value input
                                .child(
                                    self.render_text_input(
                                        format!("var-value-{}", i),
                                        EnvEditTarget::VarValue(i),
                                        &value,
                                        "Value",
                                        is_editing_value,
                                        if is_editing_value { edit_selection.clone() } else { 0..0 },
                                        cx,
                                    )
                                )
                                // Remove button
                                .child(
                                    div()
                                        .id(SharedString::from(format!("var-remove-{}", i)))
                                        .size(px(20.0))
                                        .rounded(px(4.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_muted.opacity(0.5))
                                        .hover(|s| s.bg(theme.colors.status_client_error.opacity(0.1)).text_color(theme.colors.status_client_error))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.remove_variable(&key_for_remove, cx);
                                        }))
                                        .child("×")
                                )
                        }))
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
                                            .child("{ }")
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("No variables defined")
                                    )
                                    .child(
                                        div()
                                            .id("add-first-var-btn")
                                            .mt(px(4.0))
                                            .px(px(10.0))
                                            .py(px(4.0))
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.accent)
                                            .border_1()
                                            .border_color(theme.colors.accent.opacity(0.3))
                                            .hover(|s| s.bg(theme.colors.accent.opacity(0.1)))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.add_variable(cx);
                                            }))
                                            .child("+ Add Variable")
                                    )
                            )
                        })
                )
            })
            // No environment selected state
            .when(self.env_state.active_index.is_none() && !show_new_env_input, |el| {
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
                                .rounded(px(8.0))
                                .bg(theme.colors.bg_tertiary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .text_size(px(16.0))
                                        .text_color(theme.colors.text_muted)
                                        .child("⚙")
                                )
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted)
                                .child("Select an environment to edit")
                        )
                )
            })
            // Usage hint
            .child(
                div()
                    .w_full()
                    .mt(px(4.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(theme.colors.bg_tertiary.opacity(0.5))
                    .flex()
                    .items_start()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("💡")
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Use {{var}} in URL, headers, body, or auth")
                    )
            )
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
            .h(px(24.0))
            .px(px(6.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .rounded(px(4.0))
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
            .bg(theme.colors.bg_tertiary)
            .text_size(px(11.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event: &MouseDownEvent, window, cx| {
                    this.start_editing(target, window, cx);
                }),
            )
            .child(self.render_text_content(&text, placeholder, is_editing, selection, cx))
    }

    fn render_text_content(
        &self,
        text: &str,
        placeholder: &'static str,
        is_focused: bool,
        selection: Range<usize>,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        // ~15 chars for narrow env variable inputs
        render_text_view_with_max(
            text,
            &selection,
            is_focused,
            11.0,
            theme.colors.text_primary,
            Some(placeholder),
            theme.colors.text_muted,
            Some(15),
        )
    }
}

impl Render for ExplorerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_secondary)
            .track_focus(&self.edit_focus)
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
                                    .text_size(px(14.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("☰")
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("Explorer")
                            )
                    )
                    // New request button
                    .child(
                        div()
                            .id("new-request-btn")
                            .size(px(24.0))
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(14.0))
                            .text_color(theme.colors.text_muted)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).text_color(theme.colors.text_primary))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.create_new_request(cx);
                            }))
                            .child("+")
                    )
            )
            // Content
            .child(
                div()
                    .id("explorer-content")
                    .flex_1()
                    .overflow_scroll()
                    // Collections section
                    .child(self.render_collections_section(cx))
                    // History section
                    .child(self.render_history_section(cx))
            )
            // Environment section
            .child(self.render_environment_section(cx))
            // Context menu overlay
            .when_some(self.context_menu.clone(), |el, (path, position)| {
                el.child(
                    // Backdrop to catch clicks outside
                    div()
                        .id("context-menu-backdrop")
                        .absolute()
                        .inset_0()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.close_context_menu(cx);
                        }))
                        .on_mouse_down(MouseButton::Right, cx.listener(|this, _, _, cx| {
                            this.close_context_menu(cx);
                        }))
                )
                .child(self.render_context_menu(path, position, cx))
            })
    }
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
