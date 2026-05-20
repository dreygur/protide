use gpui::{Context, Window};
use super::*;

impl ExplorerPanel {
    pub(super) fn toggle_env_dropdown(&mut self, cx: &mut Context<Self>) {
        self.env_dropdown_open = !self.env_dropdown_open;
        if self.env_dropdown_open {
            self.env_editor_open = false;
        }
        cx.notify();
    }

    pub(super) fn toggle_env_editor(&mut self, cx: &mut Context<Self>) {
        self.env_editor_open = !self.env_editor_open;
        if self.env_editor_open {
            self.env_dropdown_open = false;
        }
        cx.notify();
    }

    pub(super) fn select_environment(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        self.env_state.set_active(index);
        self.env_dropdown_open = false;
        cx.notify();
    }

    pub(super) fn add_variable(&mut self, cx: &mut Context<Self>) {
        if let Some(env) = self.env_state.active_mut() {
            let key = format!("var_{}", env.variables.len() + 1);
            env.set(key, "");
            cx.notify();
        }
    }

    pub(super) fn remove_variable(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(env) = self.env_state.active_mut() {
            env.remove(key);
            cx.notify();
        }
    }

    pub(super) fn start_new_env(&mut self, cx: &mut Context<Self>) {
        self.show_new_env_input = true;
        self.new_env_name = String::new();
        self.active_edit = Some(EnvEditTarget::NewEnvName);
        self.edit_selection = 0..0;
        cx.notify();
    }

    pub(super) fn create_new_env(&mut self, cx: &mut Context<Self>) {
        if !self.new_env_name.trim().is_empty() {
            let env = Environment::new(self.new_env_name.trim());
            self.env_state.add_environment(env);
            let new_index = self.env_state.environments.len() - 1;
            self.env_state.set_active(Some(new_index));
        }
        self.show_new_env_input = false;
        self.new_env_name.clear();
        self.active_edit = None;
        cx.notify();
    }

    pub(super) fn cancel_new_env(&mut self, cx: &mut Context<Self>) {
        self.show_new_env_input = false;
        self.new_env_name.clear();
        self.active_edit = None;
        cx.notify();
    }

    pub(super) fn delete_environment(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.env_state.environments.len() > 1 {
            self.env_state.remove_environment(index);
            cx.notify();
        }
    }

    pub(super) fn start_editing(
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

    pub(super) fn stop_editing(&mut self, cx: &mut Context<Self>) {
        self.active_edit = None;
        self.edit_selection = 0..0;
        self.edit_is_selecting = false;
        cx.notify();
    }

    pub(super) fn get_edit_text(&self, target: EnvEditTarget) -> String {
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

    pub(super) fn set_edit_text(&mut self, target: EnvEditTarget, text: String) {
        match target {
            EnvEditTarget::VarKey(i) => {
                if let Some(env) = self.env_state.active_mut() {
                    if let Some((old_key, value)) = env
                        .variables
                        .iter()
                        .nth(i)
                        .map(|(k, v)| (k.clone(), v.clone()))
                    {
                        env.variables.shift_remove(&old_key);
                        if !text.is_empty() {
                            env.variables.insert(text, value);
                        }
                    }
                }
            }
            EnvEditTarget::VarValue(i) => {
                if let Some(env) = self.env_state.active_mut()
                    && let Some(key) = env.variables.keys().nth(i).cloned() {
                        env.variables.insert(key, text);
                    }
            }
            EnvEditTarget::NewEnvName => {
                self.new_env_name = text;
            }
        }
    }

    pub(super) fn save_edit_state(&mut self) {
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

    pub(super) fn edit_undo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_undo_stack.pop() {
            let current_text = self.get_edit_text(target);
            self.edit_redo_stack
                .push((target, current_text, self.edit_selection.clone()));
            self.set_edit_text(target, text);
            self.edit_selection = selection;
            cx.notify();
        }
    }

    pub(super) fn edit_redo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_redo_stack.pop() {
            let current_text = self.get_edit_text(target);
            self.edit_undo_stack
                .push((target, current_text, self.edit_selection.clone()));
            self.set_edit_text(target, text);
            self.edit_selection = selection;
            cx.notify();
        }
    }

    pub(super) fn update_env_scroll(&mut self, target: EnvEditTarget) {
        let char_width = 11.0 * 0.6;
        let padding = 6.0 * 2.0;
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

    pub(super) fn insert_text(&mut self, target: EnvEditTarget, text: &str, cx: &mut Context<Self>) {
        self.save_edit_state();
        let mut current = self.get_edit_text(target);

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

    pub(super) fn reorder_env_var(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if let Some(env) = self.env_state.active_mut() {
            let len = env.variables.len();
            if from < len && to < len {
                env.variables.move_index(from, to);
            }
        }
        cx.notify();
    }

    /// Calculate character index from x position
    pub(super) fn edit_index_for_x(&self, x: f32, char_width: f32) -> usize {
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

}
