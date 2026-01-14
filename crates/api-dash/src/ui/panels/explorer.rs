//! File explorer panel - displays the workspace file tree, request history, and environment selector

use gpui::{
    div, prelude::*, px, ClipboardItem, Context, Entity, FocusHandle, IntoElement, KeyDownEvent,
    MouseDownEvent, ParentElement, Render, SharedString, Styled, Window,
};
use std::ops::Range;

use crate::models::{Environment, EnvironmentState};
use crate::theme;
use super::history::{HistoryEntry, RequestHistory};
use super::request::RequestPanel;

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
}

impl ExplorerPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            request_panel: None,
            history_expanded: true,
            env_state: EnvironmentState::new(),
            env_dropdown_open: false,
            env_editor_open: false,
            active_edit: None,
            edit_selection: 0..0,
            edit_focus: cx.focus_handle(),
            new_env_name: String::new(),
            show_new_env_input: false,
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

    fn handle_edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
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
                    let start = self.edit_selection.start.min(self.edit_selection.end);
                    let end = self.edit_selection.start.max(self.edit_selection.end);
                    text.replace_range(start..end, "");
                    self.edit_selection = start..start;
                    self.set_edit_text(target, text);
                    cx.notify();
                } else if self.edit_selection.end > 0 {
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

    fn render_history_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let entries = self.get_history_entries(cx);

        div()
            .w_full()
            .flex()
            .flex_col()
            // History header
            .child(
                div()
                    .id("history-header")
                    .h(px(28.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .px(px(8.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_history(cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(if self.history_expanded { "▼" } else { "▶" })
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_secondary)
                                    .child("HISTORY")
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(format!("({})", entries.len()))
                            )
                    )
            )
            // History items
            .when(self.history_expanded, |el| {
                el.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
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
                                .h(px(26.0))
                                .flex()
                                .items_center()
                                .px(px(12.0))
                                .gap(px(8.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.bg_tertiary))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.load_history_item(entry_id, cx);
                                }))
                                // Method badge
                                .child(
                                    div()
                                        .min_w(px(42.0))
                                        .h(px(16.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(3.0))
                                        .bg(method_color.opacity(0.15))
                                        .child(
                                            div()
                                                .text_size(px(9.0))
                                                .font_weight(gpui::FontWeight::BOLD)
                                                .text_color(method_color)
                                                .child(method)
                                        )
                                )
                                // URL
                                .child(
                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_primary)
                                        .child(display_url)
                                )
                                // Status indicator
                                .when_some(status_color, |el, color| {
                                    el.child(
                                        div()
                                            .size(px(6.0))
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

        div()
            .w_full()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(theme.colors.border)
            // Environment selector header
            .child(
                div()
                    .h(px(36.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    // Left: env selector dropdown
                    .child(
                        div()
                            .id("env-selector")
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_env_dropdown(cx);
                            }))
                            .child(
                                div()
                                    .size(px(8.0))
                                    .rounded_full()
                                    .bg(if self.env_state.active_index.is_some() {
                                        theme.colors.status_success
                                    } else {
                                        theme.colors.text_muted
                                    })
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_primary)
                                    .child(active_env_name.clone())
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(if env_dropdown_open { "▲" } else { "▼" })
                            )
                    )
                    // Right: edit button
                    .child(
                        div()
                            .id("env-edit-btn")
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .text_color(if env_editor_open {
                                theme.colors.accent
                            } else {
                                theme.colors.text_secondary
                            })
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_env_editor(cx);
                            }))
                            .child(if env_editor_open { "Close" } else { "Edit" })
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
        let envs: Vec<(usize, String)> = self.env_state.environments
            .iter()
            .enumerate()
            .map(|(i, e)| (i, e.name.clone()))
            .collect();

        div()
            .id("env-dropdown")
            .w_full()
            .max_h(px(200.0))
            .overflow_scroll()
            .px(px(8.0))
            .pb(px(8.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            // "No Environment" option
            .child(
                div()
                    .id("env-option-none")
                    .w_full()
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .text_size(px(12.0))
                    .when(active_index.is_none(), |el| {
                        el.bg(theme.colors.bg_tertiary)
                            .text_color(theme.colors.text_primary)
                    })
                    .when(active_index.is_some(), |el| {
                        el.text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.select_environment(None, cx);
                    }))
                    .child("No Environment")
            )
            // Environment options
            .children(envs.into_iter().map(|(i, name)| {
                let is_active = active_index == Some(i);
                div()
                    .id(SharedString::from(format!("env-option-{}", i)))
                    .w_full()
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .text_size(px(12.0))
                    .when(is_active, |el| {
                        el.bg(theme.colors.bg_tertiary)
                            .text_color(theme.colors.text_primary)
                    })
                    .when(!is_active, |el| {
                        el.text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_environment(Some(i), cx);
                    }))
                    .child(name)
            }))
            // Add new environment button
            .child(
                div()
                    .id("env-add-btn")
                    .w_full()
                    .px(px(8.0))
                    .py(px(6.0))
                    .mt(px(4.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .text_size(px(12.0))
                    .text_color(theme.colors.accent)
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.env_dropdown_open = false;
                        this.env_editor_open = true;
                        this.start_new_env(cx);
                    }))
                    .child("+ New Environment")
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
            .px(px(8.0))
            .pb(px(8.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            // New environment input
            .when(show_new_env_input, |el| {
                el.child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
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
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .text_size(px(11.0))
                                .bg(theme.colors.accent)
                                .text_color(gpui::white())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.create_new_env(cx);
                                }))
                                .child("Create")
                        )
                        .child(
                            div()
                                .id("cancel-env-btn")
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(4.0))
                                .cursor_pointer()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_secondary)
                                .hover(|s| s.bg(theme.colors.bg_tertiary))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.cancel_new_env(cx);
                                }))
                                .child("Cancel")
                        )
                )
            })
            // Variables header
            .when(self.env_state.active_index.is_some(), |el| {
                el.child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(11.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.colors.text_secondary)
                                .child("VARIABLES")
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .id("add-var-btn")
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .text_size(px(11.0))
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
                                            .px(px(6.0))
                                            .py(px(2.0))
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.status_client_error)
                                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.delete_environment(idx, cx);
                                            }))
                                            .child("Delete Env")
                                    )
                                })
                        )
                )
            })
            // Variables list
            .children(vars.into_iter().map(|(i, key, value)| {
                let is_editing_key = self.active_edit == Some(EnvEditTarget::VarKey(i));
                let is_editing_value = self.active_edit == Some(EnvEditTarget::VarValue(i));
                let key_for_remove = key.clone();

                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
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
                            .text_color(theme.colors.text_muted)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).text_color(theme.colors.status_client_error))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_variable(&key_for_remove, cx);
                            }))
                            .child("×")
                    )
            }))
            // Empty state
            .when(!has_vars && self.env_state.active_index.is_some(), |el| {
                el.child(
                    div()
                        .w_full()
                        .py(px(8.0))
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child("No variables. Click '+ Add' to create one.")
                )
            })
            // No environment selected
            .when(self.env_state.active_index.is_none(), |el| {
                el.child(
                    div()
                        .w_full()
                        .py(px(8.0))
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child("Select an environment to edit variables.")
                )
            })
            // Usage hint
            .child(
                div()
                    .w_full()
                    .pt(px(8.0))
                    .text_size(px(11.0))
                    .text_color(theme.colors.text_muted)
                    .child("Use {{variable_name}} syntax in URL, headers, body, or auth fields.")
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
            .h(px(24.0))
            .px(px(6.0))
            .flex()
            .items_center()
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
    ) -> impl IntoElement {
        let theme = theme::current(cx);

        if text.is_empty() && !is_focused {
            return div()
                .text_color(theme.colors.text_muted)
                .child(placeholder)
                .into_any_element();
        }

        let sel_start = selection.start.min(selection.end).min(text.len());
        let sel_end = selection.start.max(selection.end).min(text.len());
        let has_sel = sel_start != sel_end;

        let before_sel = &text[..sel_start];
        let selected = &text[sel_start..sel_end];
        let after_sel = &text[sel_end..];

        div()
            .flex()
            .items_center()
            .text_color(theme.colors.text_primary)
            .child(before_sel.to_string())
            .when(has_sel, |el| {
                el.child(
                    div()
                        .bg(gpui::rgba(0x3366ff40))
                        .child(selected.to_string()),
                )
            })
            .when(!has_sel && is_focused, |el| {
                el.child(div().w(px(1.0)).h(px(12.0)).bg(theme.colors.text_primary))
            })
            .child(after_sel.to_string())
            .when(has_sel && is_focused, |el| {
                el.child(div().w(px(1.0)).h(px(12.0)).bg(theme.colors.text_primary))
            })
            .into_any_element()
    }
}

impl Render for ExplorerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .track_focus(&self.edit_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_edit_key(event, cx);
            }))
            // Header
            .child(
                div()
                    .h(px(32.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_secondary)
                            .child("EXPLORER")
                    )
            )
            // Content - History section
            .child(
                div()
                    .id("explorer-content")
                    .flex_1()
                    .overflow_scroll()
                    .py(px(4.0))
                    .child(self.render_history_section(cx))
            )
            // Environment section
            .child(self.render_environment_section(cx))
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
