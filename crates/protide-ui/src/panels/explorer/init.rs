use super::workspace::read_request;
use super::*;
use gpui::{Context, Entity};

impl ExplorerPanel {
    pub fn new(cx: &mut Context<Self>, main_window: WeakEntity<MainWindow>) -> Self {
        Self {
            request_panel: None,
            history_expanded: false,
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
            edit_undo_stack: std::collections::VecDeque::new(),
            edit_redo_stack: std::collections::VecDeque::new(),
            edit_is_selecting: false,
            edit_input_origins: std::collections::HashMap::new(),
            edit_scroll_offsets: std::collections::HashMap::new(),
            edit_input_widths: std::collections::HashMap::new(),
            _edit_blur_sub: None,
            selected_item: None,
            selected_request: None,
            context_menu: None,
            renaming_item: None,
            rename_text: String::new(),
            rename_selection: 0..0,
            rename_is_selecting: false,
            rename_input_origin: 0.0,
            env_col_key_w: 90.0,
            env_col_drag: None,
            env_row_drag: None,
            env_row_drag_over: None,
            collections_h: crate::prefs::get_f32("explorer.collections_h", 220.0),
            env_h: crate::prefs::get_f32("explorer.env_h", 320.0),
            drag_coll: None,
            drag_env: None,
            main_window,
            panel_bounds: gpui::Bounds::default(),
            sync_skip_paths: std::collections::HashSet::new(),
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

    /// Restore the session entry for a given workspace (call after scanning).
    pub fn restore_workspace_session(
        &mut self,
        entry: &crate::session::WorkspaceEntry,
        cx: &mut Context<Self>,
    ) {
        self.apply_expanded(&entry.expanded_folders);

        // Re-select active environment
        if let Some(ref env_name) = entry.active_env
            && let Some(idx) = self
                .env_state
                .environments
                .iter()
                .position(|e| &e.name == env_name)
        {
            self.env_state.set_active(Some(idx));
        }

        // Re-open the last active file
        if let Some(ref file) = entry.active_file
            && file.exists()
        {
            let index = entry.active_request;
            let multi = read_request(file, 1).is_some();
            self.selected_item = Some(file.clone());
            self.selected_request = multi.then_some(index);
            if let Some(ref draft) = entry.draft
                && let Some(ref rp) = self.request_panel
            {
                let draft = draft.clone();
                let file = file.clone();
                let request = read_request(&file, index);
                rp.update(cx, |panel, cx| {
                    panel.restore_from_draft(&draft, cx);
                    // Without the request on disk a save must not guess
                    // which block to rewrite; it asks for a file instead.
                    if let Some((content, request)) = request {
                        panel.set_loaded_file(file, index, request, content, cx);
                    }
                });
            }
        }
    }
}
