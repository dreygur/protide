//! File explorer panel - displays the workspace file tree, request history, and environment selector

use log::{debug, error, info, warn};

use crate::components::modal::ModalState;
use crate::main_window::MainWindow;
use gpui::{
    ClipboardItem, Context, Entity, FocusHandle, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render,
    SharedString, Styled, Subscription, WeakEntity, Window, canvas, deferred,
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
use crate::components::icons::{
    ICON_ARROW_DOWN, ICON_CHEVRON_DOWN, ICON_CHEVRON_RIGHT, ICON_CHEVRON_UP, ICON_CLOSE,
    ICON_DELETE, ICON_EDIT, ICON_EXPORT, ICON_FILE, ICON_FOLDER, ICON_FOLDER_OPEN, ICON_INFO,
    ICON_LINK, ICON_MD, ICON_MENU, ICON_PLUS, ICON_REFRESH, ICON_SETTINGS, ICON_SM, ICON_TIMER,
    icon,
};
use crate::components::tooltip_text;
use http_parser::Protocol;
use crate::components::{ActionRow, ghost_action_btn, icon_btn, render_text_view_with_max_scrolled};
use protide_core::models::{Environment, EnvironmentState};

pub mod init;
pub mod workspace;
pub mod tree_nav;
pub mod tree_ops;
pub mod tree_scan;
pub mod env_input;
pub mod history;
pub mod env;
pub mod render;
pub mod render_tree;
pub mod render_tree_row;
pub mod render_history;
pub mod render_env;
pub mod render_env_editor;
pub mod render_inputs;
pub mod util;

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
    pub(super) request_panel: Option<Entity<RequestPanel>>,
    pub(super) history_expanded: bool,
    pub(super) collections_expanded: bool,
    pub(super) workspace_path: Option<PathBuf>,
    pub(super) workspace_watcher: Option<Arc<protide_core::workspace::Workspace>>,
    pub(super) collection_items: Vec<CollectionItem>,
    pub(super) env_state: EnvironmentState,
    pub(super) env_dropdown_open: bool,
    pub(super) env_editor_open: bool,
    pub(super) active_edit: Option<EnvEditTarget>,
    pub(super) edit_selection: Range<usize>,
    pub(super) edit_focus: FocusHandle,
    pub(super) new_env_name: String,
    pub(super) show_new_env_input: bool,
    pub(super) edit_undo_stack: Vec<(EnvEditTarget, String, Range<usize>)>,
    pub(super) edit_redo_stack: Vec<(EnvEditTarget, String, Range<usize>)>,
    pub(super) edit_is_selecting: bool,
    pub(super) edit_input_origins: std::collections::HashMap<EnvEditTarget, f32>,
    pub(super) edit_scroll_offsets: std::collections::HashMap<EnvEditTarget, f32>,
    pub(super) edit_input_widths: std::collections::HashMap<EnvEditTarget, f32>,
    pub(super) _edit_blur_sub: Option<Subscription>,
    pub(super) selected_item: Option<PathBuf>,
    pub(super) context_menu: Option<(PathBuf, Point<Pixels>)>,
    pub(super) renaming_item: Option<PathBuf>,
    pub(super) rename_text: String,
    pub(super) env_col_key_w: f32,
    pub(super) env_col_drag: Option<(f32, f32)>,
    pub(super) collections_h: f32,
    pub(super) env_h: f32,
    pub(super) drag_coll: Option<(f32, f32)>,
    pub(super) drag_env: Option<(f32, f32)>,
    pub(super) main_window: WeakEntity<MainWindow>,
    pub(super) panel_bounds: gpui::Bounds<Pixels>,
}

impl ExplorerPanel {
    /// Get the current environment state for variable substitution
    pub fn env_state(&self) -> &EnvironmentState {
        &self.env_state
    }

    /// Return the currently selected file path (if any).
    pub fn selected_item(&self) -> Option<&PathBuf> {
        self.selected_item.as_ref()
    }

    /// Return the open workspace path (if any).
    pub fn workspace_path(&self) -> Option<&PathBuf> {
        self.workspace_path.as_ref()
    }

    /// Collect all expanded folder paths from the collection tree.
    pub fn collect_expanded(&self) -> Vec<PathBuf> {
        fn gather(items: &[CollectionItem], out: &mut Vec<PathBuf>) {
            for item in items {
                if item.is_folder && item.expanded {
                    out.push(item.path.clone());
                    gather(&item.children, out);
                }
            }
        }
        let mut result = Vec::new();
        gather(&self.collection_items, &mut result);
        result
    }

    /// Mark the given folder paths as expanded (call after rescanning the tree).
    pub(super) fn apply_expanded(&mut self, paths: &[PathBuf]) {
        fn mark(items: &mut [CollectionItem], paths: &[PathBuf]) {
            for item in items.iter_mut() {
                if item.is_folder {
                    if paths.contains(&item.path) {
                        item.expanded = true;
                    }
                    mark(&mut item.children, paths);
                }
            }
        }
        mark(&mut self.collection_items, paths);
    }

    pub fn collection_items(&self) -> &[CollectionItem] {
        &self.collection_items
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

    /// Set a variable in the active environment (for @set extraction)
    pub fn set_env_variable(&mut self, name: &str, value: &str, cx: &mut Context<Self>) {
        if let Some(env) = self.env_state.active_mut() {
            env.set(name, value);
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
