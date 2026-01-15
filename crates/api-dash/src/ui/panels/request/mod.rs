//! Request editor panel
//!
//! The main request builder UI with URL input, method selector,
//! headers/params/body editors, and authentication configuration.

mod render;

#[cfg(test)]
mod tests;

use std::ops::Range;

use gpui::{
    div, prelude::*, px, ClipboardItem, Context, Entity, FocusHandle, IntoElement, KeyDownEvent,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render, Styled,
    Window,
};

use crate::ui::components::{render_text_view_with_max, find_word_start, find_word_end};
use crate::ui::components::code_editor::{CodeEditor, Language};
use crate::scripting::{ScriptEngine, ScriptContext, RequestData as ScriptRequestData, ResponseData as ScriptResponseData};

use super::explorer::ExplorerPanel;
use super::request_types::{ApiKeyLocation, AuthType, BodyType, EditTarget, FormField, FormFieldType, HttpMethod, KeyValuePair};
use super::request_utils::{base64_encode, status_text, url_decode, url_encode};
use super::response::{ResponseData, ResponsePanel};

use crate::chaining;
use crate::codegen::{self, CodegenRequest, Language as CodegenLanguage};
use http_parser::VariableExtraction;

/// Helper to render text with selection highlighting
fn render_text_view(
    text: &str,
    selection: &Range<usize>,
    is_focused: bool,
    font_size: f32,
    text_color: gpui::Hsla,
    placeholder: Option<&str>,
    placeholder_color: gpui::Hsla,
) -> gpui::AnyElement {
    render_text_view_with_max(text, selection, is_focused, font_size, text_color, placeholder, placeholder_color, None)
}

/// Convert character index to byte offset in a string
fn char_to_byte_offset(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(byte_offset, _)| byte_offset)
        .unwrap_or(text.len())
}

/// Convert byte offset to character index in a string
#[allow(dead_code)]
fn byte_to_char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset.min(text.len())]
        .chars()
        .count()
}

/// Request editor panel
pub struct RequestPanel {
    pub(super) active_tab: usize,
    pub(super) method: HttpMethod,
    pub(super) url: String,
    pub(super) url_selection: Range<usize>,
    pub(super) method_dropdown_open: bool,
    pub(super) url_focus: FocusHandle,
    pub(super) is_selecting: bool,
    pub(super) url_input_left: f32,
    pub(super) response_panel: Entity<ResponsePanel>,
    pub(super) loading: bool,
    pub(super) headers: Vec<KeyValuePair>,
    pub(super) params: Vec<KeyValuePair>,
    pub(super) form_data: Vec<FormField>,
    pub(super) body: String,
    pub(super) body_type: BodyType,
    pub(super) syncing_params: bool,
    pub(super) auth_type: AuthType,
    pub(super) bearer_token: String,
    pub(super) basic_username: String,
    pub(super) basic_password: String,
    pub(super) api_key_name: String,
    pub(super) api_key_value: String,
    pub(super) api_key_location: ApiKeyLocation,
    pub(super) active_edit: Option<EditTarget>,
    pub(super) edit_selection: Range<usize>,
    pub(super) edit_is_selecting: bool,
    pub(super) edit_input_left: f32,
    pub(super) url_undo_stack: Vec<(String, Range<usize>)>,
    pub(super) url_redo_stack: Vec<(String, Range<usize>)>,
    pub(super) edit_undo_stack: Vec<(EditTarget, String, Range<usize>)>,
    pub(super) edit_redo_stack: Vec<(EditTarget, String, Range<usize>)>,
    pub(super) skip_blur: bool,
    pub(super) edit_focus: FocusHandle,
    pub(super) body_focus: FocusHandle,
    pub(super) explorer_panel: Option<Entity<ExplorerPanel>>,
    // CodeEditor for JSON/Raw body
    pub(super) body_editor: Entity<CodeEditor>,
    // Script editors
    pub(super) pre_script: String,
    pub(super) post_script: String,
    pub(super) tests: String,
    pub(super) pre_script_editor: Entity<CodeEditor>,
    pub(super) post_script_editor: Entity<CodeEditor>,
    pub(super) tests_editor: Entity<CodeEditor>,
    /// Variable extractions from @set annotations
    pub(super) variable_extractions: Vec<VariableExtraction>,
    /// Code generation dropdown state
    pub(super) codegen_dropdown_open: bool,
    /// Generated code content
    pub(super) codegen_content: Option<String>,
    /// Selected code generation language
    pub(super) codegen_language: CodegenLanguage,
}

impl RequestPanel {
    pub fn new(cx: &mut Context<Self>, response_panel: Entity<ResponsePanel>) -> Self {
        let url = "https://httpbin.org/post".to_string();
        let url_len = url.len();
        let initial_body = "{\n  \"name\": \"API Dash\",\n  \"version\": \"0.1.0\"\n}";
        let body_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_content(initial_body)
                .with_language(Language::Json)
                .with_line_numbers(true)
        });
        let pre_script_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_language(Language::JavaScript)
                .with_line_numbers(true)
        });
        let post_script_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_language(Language::JavaScript)
                .with_line_numbers(true)
        });
        let tests_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_language(Language::JavaScript)
                .with_line_numbers(true)
        });
        Self {
            active_tab: 0,
            method: HttpMethod::Post,
            url,
            url_selection: url_len..url_len,
            method_dropdown_open: false,
            url_focus: cx.focus_handle(),
            is_selecting: false,
            url_input_left: 0.0,
            response_panel,
            loading: false,
            headers: vec![
                KeyValuePair {
                    key: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                    enabled: true,
                },
                KeyValuePair::default(),
            ],
            params: vec![KeyValuePair::default()],
            form_data: vec![FormField::default()],
            body: initial_body.to_string(),
            body_type: BodyType::Json,
            syncing_params: false,
            auth_type: AuthType::None,
            bearer_token: String::new(),
            basic_username: String::new(),
            basic_password: String::new(),
            api_key_name: String::new(),
            api_key_value: String::new(),
            api_key_location: ApiKeyLocation::Header,
            active_edit: None,
            edit_selection: 0..0,
            edit_is_selecting: false,
            edit_input_left: 0.0,
            url_undo_stack: Vec::new(),
            url_redo_stack: Vec::new(),
            edit_undo_stack: Vec::new(),
            edit_redo_stack: Vec::new(),
            skip_blur: false,
            edit_focus: cx.focus_handle(),
            body_focus: cx.focus_handle(),
            explorer_panel: None,
            body_editor,
            pre_script: String::new(),
            post_script: String::new(),
            tests: String::new(),
            pre_script_editor,
            post_script_editor,
            tests_editor,
            variable_extractions: Vec::new(),
            codegen_dropdown_open: false,
            codegen_content: None,
            codegen_language: CodegenLanguage::Curl,
        }
    }

    /// Set the explorer panel reference for environment variable substitution
    pub fn set_explorer_panel(&mut self, explorer_panel: Entity<ExplorerPanel>, cx: &mut Context<Self>) {
        self.explorer_panel = Some(explorer_panel);
        cx.notify();
    }

    fn cursor(&self) -> usize {
        self.url_selection.end
    }

    /// Set body content in the CodeEditor
    pub fn set_body_content(&mut self, content: &str, cx: &mut Context<Self>) {
        self.body_editor.update(cx, |editor, cx| {
            editor.set_content(content, cx);
        });
        self.body = content.to_string();
    }

    /// Set variable extractions from @set annotations
    pub fn set_variable_extractions(&mut self, extractions: Vec<VariableExtraction>, cx: &mut Context<Self>) {
        self.variable_extractions = extractions;
        cx.notify();
    }

    /// Load request data from a history entry
    pub fn load_from_history(
        &mut self,
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<String>,
        cx: &mut Context<Self>,
    ) {
        // Set method
        if let Some(m) = HttpMethod::from_str(&method) {
            self.method = m;
        }

        // Set URL
        self.url = url;
        let char_count = self.url.chars().count();
        self.url_selection = char_count..char_count;

        // Set headers
        self.headers = headers
            .into_iter()
            .map(|(key, value)| KeyValuePair {
                key,
                value,
                enabled: true,
            })
            .collect();
        // Always have at least one empty row
        if self.headers.is_empty() {
            self.headers.push(KeyValuePair::default());
        } else {
            self.headers.push(KeyValuePair::default());
        }

        // Set body
        if let Some(b) = body {
            self.set_body_content(&b, cx);
        }

        // Sync params from URL
        self.sync_params_from_url(cx);

        // Reset editing state
        self.active_edit = None;
        self.method_dropdown_open = false;

        // Clear variable extractions (will be set separately from file load if present)
        self.variable_extractions.clear();

        cx.notify();
    }

    fn set_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        cx.notify();
    }

    fn toggle_method_dropdown(&mut self, cx: &mut Context<Self>) {
        self.method_dropdown_open = !self.method_dropdown_open;
        cx.notify();
    }

    fn select_method(&mut self, method: HttpMethod, cx: &mut Context<Self>) {
        self.method = method;
        self.method_dropdown_open = false;
        cx.notify();
    }

    fn set_body_type(&mut self, body_type: BodyType, cx: &mut Context<Self>) {
        self.body_type = body_type;
        // Update Content-Type header based on body type
        let content_type = match body_type {
            BodyType::Json => "application/json",
            BodyType::Form => "application/x-www-form-urlencoded",
            BodyType::Raw => "text/plain",
        };
        // Update existing Content-Type header or add one
        if let Some(header) = self.headers.iter_mut().find(|h| h.key.eq_ignore_ascii_case("content-type")) {
            header.value = content_type.to_string();
        } else {
            self.headers.insert(0, KeyValuePair {
                key: "Content-Type".to_string(),
                value: content_type.to_string(),
                enabled: true,
            });
        }
        cx.notify();
    }

    fn toggle_header(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(header) = self.headers.get_mut(index) {
            header.enabled = !header.enabled;
            cx.notify();
        }
    }

    fn add_header(&mut self, cx: &mut Context<Self>) {
        self.headers.push(KeyValuePair::default());
        cx.notify();
    }

    fn remove_header(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.headers.len() && self.headers.len() > 1 {
            self.headers.remove(index);
            // Clear editing if removed row was being edited
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::HeaderKey(i) | EditTarget::HeaderValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::HeaderKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::HeaderKey(i - 1));
                    }
                    EditTarget::HeaderValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::HeaderValue(i - 1));
                    }
                    _ => {}
                }
            }
            cx.notify();
        }
    }

    fn toggle_param(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(param) = self.params.get_mut(index) {
            param.enabled = !param.enabled;
            self.sync_url_from_params(cx);
            cx.notify();
        }
    }

    fn add_param(&mut self, cx: &mut Context<Self>) {
        self.params.push(KeyValuePair::default());
        // Don't sync URL for empty params
        cx.notify();
    }

    fn remove_param(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.params.len() && self.params.len() > 1 {
            self.params.remove(index);
            // Clear editing if removed row was being edited
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::ParamKey(i) | EditTarget::ParamValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::ParamKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::ParamKey(i - 1));
                    }
                    EditTarget::ParamValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::ParamValue(i - 1));
                    }
                    _ => {}
                }
            }
            self.sync_url_from_params(cx);
            cx.notify();
        }
    }

    fn toggle_form_field(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(field) = self.form_data.get_mut(index) {
            field.enabled = !field.enabled;
            cx.notify();
        }
    }

    fn add_form_field(&mut self, cx: &mut Context<Self>) {
        self.form_data.push(FormField::default());
        cx.notify();
    }

    fn toggle_form_field_type(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(field) = self.form_data.get_mut(index) {
            field.field_type = match field.field_type {
                FormFieldType::Text => FormFieldType::File,
                FormFieldType::File => FormFieldType::Text,
            };
            // Clear file path when switching to text
            if field.field_type == FormFieldType::Text {
                field.file_path = None;
                field.value.clear();
            }
            cx.notify();
        }
    }

    fn select_form_file(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            if let Some(field) = self.form_data.get_mut(index) {
                field.value = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();
                field.file_path = Some(path);
                cx.notify();
            }
        }
    }

    fn remove_form_field(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.form_data.len() && self.form_data.len() > 1 {
            self.form_data.remove(index);
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::FormKey(i) | EditTarget::FormValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::FormKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::FormKey(i - 1));
                    }
                    EditTarget::FormValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::FormValue(i - 1));
                    }
                    _ => {}
                }
            }
            cx.notify();
        }
    }

    fn set_auth_type(&mut self, auth_type: AuthType, cx: &mut Context<Self>) {
        self.auth_type = auth_type;
        self.active_edit = None;
        cx.notify();
    }

    fn toggle_api_key_location(&mut self, cx: &mut Context<Self>) {
        self.api_key_location = match self.api_key_location {
            ApiKeyLocation::Header => ApiKeyLocation::QueryParam,
            ApiKeyLocation::QueryParam => ApiKeyLocation::Header,
        };
        cx.notify();
    }

    // ===== URL <-> Params Sync Methods =====

    /// Get the base URL without query string
    fn get_base_url(&self) -> &str {
        self.url.split('?').next().unwrap_or(&self.url)
    }

    /// Parse query params from URL and update params list
    fn sync_params_from_url(&mut self, cx: &mut Context<Self>) {
        if self.syncing_params {
            return;
        }
        self.syncing_params = true;

        // Find query string
        if let Some(query_start) = self.url.find('?') {
            let query_string = &self.url[query_start + 1..];
            let mut new_params: Vec<KeyValuePair> = Vec::new();

            for pair in query_string.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let mut parts = pair.splitn(2, '=');
                let key = url_decode(parts.next().unwrap_or(""));
                let value = url_decode(parts.next().unwrap_or(""));
                new_params.push(KeyValuePair {
                    key,
                    value,
                    enabled: true,
                });
            }

            // Always keep at least one empty param row
            if new_params.is_empty() {
                new_params.push(KeyValuePair::default());
            }

            self.params = new_params;
        } else {
            // No query string - reset to single empty param
            self.params = vec![KeyValuePair::default()];
        }

        self.syncing_params = false;
        cx.notify();
    }

    /// Build URL from base URL and params
    fn sync_url_from_params(&mut self, cx: &mut Context<Self>) {
        if self.syncing_params {
            return;
        }
        self.syncing_params = true;

        let base_url = self.get_base_url().to_string();

        // Build query string from enabled params with non-empty keys
        let query_parts: Vec<String> = self
            .params
            .iter()
            .filter(|p| p.enabled && !p.key.is_empty())
            .map(|p| {
                if p.value.is_empty() {
                    url_encode(&p.key)
                } else {
                    format!("{}={}", url_encode(&p.key), url_encode(&p.value))
                }
            })
            .collect();

        // Update URL
        let old_len = self.url.len();
        if query_parts.is_empty() {
            self.url = base_url;
        } else {
            self.url = format!("{}?{}", base_url, query_parts.join("&"));
        }

        // Adjust cursor if it was beyond the new URL length
        let new_len = self.url.len();
        if self.url_selection.start > new_len {
            self.url_selection.start = new_len;
        }
        if self.url_selection.end > new_len {
            self.url_selection.end = new_len;
        }

        self.syncing_params = false;
        if old_len != new_len {
            cx.notify();
        }
    }

    // ===== Unified Text Editing Methods =====

    /// Get reference to text for an edit target
    fn get_edit_text(&self, target: EditTarget) -> &str {
        match target {
            EditTarget::Url => &self.url,
            EditTarget::HeaderKey(i) => self.headers.get(i).map(|h| h.key.as_str()).unwrap_or(""),
            EditTarget::HeaderValue(i) => self.headers.get(i).map(|h| h.value.as_str()).unwrap_or(""),
            EditTarget::ParamKey(i) => self.params.get(i).map(|p| p.key.as_str()).unwrap_or(""),
            EditTarget::ParamValue(i) => self.params.get(i).map(|p| p.value.as_str()).unwrap_or(""),
            EditTarget::FormKey(i) => self.form_data.get(i).map(|f| f.key.as_str()).unwrap_or(""),
            EditTarget::FormValue(i) => self.form_data.get(i).map(|f| f.value.as_str()).unwrap_or(""),
            EditTarget::Body => &self.body,
            EditTarget::BearerToken => &self.bearer_token,
            EditTarget::BasicUsername => &self.basic_username,
            EditTarget::BasicPassword => &self.basic_password,
            EditTarget::ApiKeyName => &self.api_key_name,
            EditTarget::ApiKeyValue => &self.api_key_value,
        }
    }

    /// Get mutable reference to text for an edit target
    fn get_edit_text_mut(&mut self, target: EditTarget) -> Option<&mut String> {
        match target {
            EditTarget::Url => Some(&mut self.url),
            EditTarget::HeaderKey(i) => self.headers.get_mut(i).map(|h| &mut h.key),
            EditTarget::HeaderValue(i) => self.headers.get_mut(i).map(|h| &mut h.value),
            EditTarget::ParamKey(i) => self.params.get_mut(i).map(|p| &mut p.key),
            EditTarget::ParamValue(i) => self.params.get_mut(i).map(|p| &mut p.value),
            EditTarget::FormKey(i) => self.form_data.get_mut(i).map(|f| &mut f.key),
            EditTarget::FormValue(i) => self.form_data.get_mut(i).map(|f| &mut f.value),
            EditTarget::Body => Some(&mut self.body),
            EditTarget::BearerToken => Some(&mut self.bearer_token),
            EditTarget::BasicUsername => Some(&mut self.basic_username),
            EditTarget::BasicPassword => Some(&mut self.basic_password),
            EditTarget::ApiKeyName => Some(&mut self.api_key_name),
            EditTarget::ApiKeyValue => Some(&mut self.api_key_value),
        }
    }

    /// Start editing a field
    fn start_editing(&mut self, target: EditTarget, window: &mut Window, cx: &mut Context<Self>) {
        let text_len = self.get_edit_text(target).len();
        self.active_edit = Some(target);
        self.edit_selection = text_len..text_len; // Cursor at end
        self.edit_is_selecting = false;
        // Use body_focus for body editor, edit_focus for other fields
        if matches!(target, EditTarget::Body) {
            self.body_focus.focus(window, cx);
        } else {
            self.edit_focus.focus(window, cx);
        }
        cx.notify();
    }

    /// Stop editing
    fn stop_editing(&mut self, cx: &mut Context<Self>) {
        self.active_edit = None;
        self.edit_selection = 0..0;
        self.edit_is_selecting = false;
        cx.notify();
    }

    /// Calculate the window X position where text starts for a given edit target
    /// Used for click position calculation since GPUI provides window coordinates
    fn calc_input_left(&self, target: EditTarget) -> f32 {
        // Layout constants (from main_window.rs and render.rs)
        const SIDEBAR_WIDTH: f32 = 250.0;
        const BORDER: f32 = 1.0;
        const TAB_CONTENT_PADDING: f32 = 12.0;
        const ROW_PADDING_X: f32 = 2.0;
        const CHECKBOX_AND_GAP: f32 = 24.0; // 16px checkbox + 8px gap
        const INPUT_PADDING: f32 = 8.0;
        const KEY_INPUT_WIDTH: f32 = 150.0;
        const GAP: f32 = 8.0;

        // Base position for inputs in tab content (params, headers, form)
        let tab_base = SIDEBAR_WIDTH + BORDER + TAB_CONTENT_PADDING + ROW_PADDING_X;
        let key_input_left = tab_base + CHECKBOX_AND_GAP + INPUT_PADDING;
        let value_input_left = key_input_left + KEY_INPUT_WIDTH + GAP + INPUT_PADDING;

        // Auth tab has different layout
        const AUTH_BASE: f32 = SIDEBAR_WIDTH + BORDER + TAB_CONTENT_PADDING + 16.0; // card padding
        const AUTH_LABEL_WIDTH: f32 = 100.0; // approximate label width
        let auth_input_left = AUTH_BASE + AUTH_LABEL_WIDTH + INPUT_PADDING;

        match target {
            EditTarget::ParamKey(_) | EditTarget::HeaderKey(_) | EditTarget::FormKey(_) => {
                key_input_left
            }
            EditTarget::ParamValue(_) | EditTarget::HeaderValue(_) | EditTarget::FormValue(_) => {
                value_input_left
            }
            EditTarget::BearerToken => auth_input_left,
            EditTarget::BasicUsername | EditTarget::BasicPassword => auth_input_left + 50.0,
            EditTarget::ApiKeyName | EditTarget::ApiKeyValue => auth_input_left + 50.0,
            EditTarget::Url | EditTarget::Body => 0.0, // These use their own handling
        }
    }

    /// Get cursor position for current edit
    fn edit_cursor(&self) -> usize {
        self.edit_selection.end
    }

    /// Check if there's a selection in current edit
    fn edit_has_selection(&self) -> bool {
        self.edit_selection.start != self.edit_selection.end
    }

    /// Get selected text for current edit
    fn edit_selected_text(&self) -> String {
        if let Some(target) = self.active_edit {
            let text = self.get_edit_text(target);
            let start = self.edit_selection.start.min(self.edit_selection.end);
            let end = self.edit_selection.start.max(self.edit_selection.end);
            text[start..end].to_string()
        } else {
            String::new()
        }
    }

    /// Move cursor to position in current edit (position is char index)
    fn edit_move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            let char_count = self.get_edit_text(target).chars().count();
            let offset = offset.min(char_count);
            self.edit_selection = offset..offset;
            cx.notify();
        }
    }

    /// Extend selection to position (position is char index)
    fn edit_select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            let char_count = self.get_edit_text(target).chars().count();
            self.edit_selection.end = offset.min(char_count);
            cx.notify();
        }
    }

    /// Select all text in current edit
    fn edit_select_all(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            let char_count = self.get_edit_text(target).chars().count();
            self.edit_selection = 0..char_count;
            cx.notify();
        }
    }

    /// Delete selected text
    fn edit_delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.active_edit.is_some() && self.edit_has_selection() {
            self.save_edit_state();
            self.edit_delete_selection_no_save(cx);
        }
    }

    /// Delete edit selection without saving to undo (used internally)
    fn edit_delete_selection_no_save(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            if self.edit_has_selection() {
                let char_start = self.edit_selection.start.min(self.edit_selection.end);
                let char_end = self.edit_selection.start.max(self.edit_selection.end);
                if let Some(text) = self.get_edit_text_mut(target) {
                    // Convert character indices to byte offsets
                    let byte_start = char_to_byte_offset(text, char_start);
                    let byte_end = char_to_byte_offset(text, char_end);
                    text.replace_range(byte_start..byte_end, "");
                    self.edit_selection = char_start..char_start;
                    // Sync URL <-> params
                    self.sync_after_edit(target, cx);
                    cx.notify();
                }
            }
        }
    }

    /// Insert text at cursor (replacing selection if any)
    /// Save edit state to undo stack before making changes
    fn save_edit_state(&mut self) {
        if let Some(target) = self.active_edit {
            let text = self.get_edit_text(target).to_string();
            self.edit_undo_stack.push((target, text, self.edit_selection.clone()));
            if self.edit_undo_stack.len() > 100 {
                self.edit_undo_stack.remove(0);
            }
            self.edit_redo_stack.clear();
        }
    }

    /// Undo edit change
    fn edit_undo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_undo_stack.pop() {
            // Save current state to redo
            let current_text = self.get_edit_text(target).to_string();
            self.edit_redo_stack.push((target, current_text, self.edit_selection.clone()));

            // Restore previous state
            if let Some(field) = self.get_edit_text_mut(target) {
                *field = text;
            }
            self.edit_selection = selection;
            self.sync_after_edit(target, cx);
            cx.notify();
        }
    }

    /// Redo edit change
    fn edit_redo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_redo_stack.pop() {
            // Save current state to undo
            let current_text = self.get_edit_text(target).to_string();
            self.edit_undo_stack.push((target, current_text, self.edit_selection.clone()));

            // Restore redo state
            if let Some(field) = self.get_edit_text_mut(target) {
                *field = text;
            }
            self.edit_selection = selection;
            self.sync_after_edit(target, cx);
            cx.notify();
        }
    }

    fn edit_insert_text(&mut self, insert: &str, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            self.save_edit_state();
            self.edit_delete_selection_no_save(cx);
            let char_pos = self.edit_selection.start;
            if let Some(text) = self.get_edit_text_mut(target) {
                // Convert character index to byte offset for string operation
                let byte_pos = char_to_byte_offset(text, char_pos);
                text.insert_str(byte_pos, insert);
                // New position is after the inserted text (in character indices)
                let new_char_pos = char_pos + insert.chars().count();
                self.edit_selection = new_char_pos..new_char_pos;
                // Sync URL <-> params
                self.sync_after_edit(target, cx);
                cx.notify();
            }
        }
    }

    /// Sync URL and params after editing
    fn sync_after_edit(&mut self, target: EditTarget, cx: &mut Context<Self>) {
        match target {
            EditTarget::ParamKey(_) | EditTarget::ParamValue(_) => {
                self.sync_url_from_params(cx);
            }
            EditTarget::Url => {
                self.sync_params_from_url(cx);
            }
            _ => {}
        }
    }

    /// Calculate character index from x position (for single-line fields)
    /// Returns a character index (not byte offset)
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

    /// Handle mouse down for single-line edit fields
    /// Calculates click position based on the target's window position
    fn handle_edit_mouse_down(&mut self, event: &MouseDownEvent, target: EditTarget, char_width: f32, cx: &mut Context<Self>) {
        self.edit_is_selecting = true;
        // Calculate the input's text start position in window coordinates
        let input_left = self.calc_input_left(target);
        self.edit_input_left = input_left;
        let click_x = f32::from(event.position.x) - input_left;
        let index = self.edit_index_for_x(click_x.max(0.0), char_width);

        // Cycle: 1=cursor, 2=word, 3=all, 4+=cursor
        let effective_click = if event.click_count >= 4 { 1 } else { event.click_count };

        match effective_click {
            2 => {
                // Double-click: select word
                if let Some(target) = self.active_edit {
                    let text = self.get_edit_text(target);
                    let start = find_word_start(&text, index);
                    let end = find_word_end(&text, index);
                    self.edit_selection = start..end;
                    cx.notify();
                }
            }
            3 => {
                // Triple-click: select all
                self.edit_select_all(cx);
            }
            _ => {
                // Single click (or 4th+ click to deselect)
                if event.modifiers.shift {
                    self.edit_select_to(index, cx);
                } else {
                    self.edit_move_to(index, cx);
                }
            }
        }
    }

    /// Calculate cursor position after moving up one line
    fn body_cursor_up(&self) -> usize {
        let text = &self.body;
        let cursor = self.edit_cursor();

        if text.is_empty() || cursor == 0 {
            return 0;
        }

        // Find current line start
        let current_line_start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);

        // If already on first line, go to start
        if current_line_start == 0 {
            return 0;
        }

        // Column in current line
        let col = cursor - current_line_start;

        // Find previous line start (line before current)
        let prev_line_end = current_line_start - 1; // newline char position
        let prev_line_start = text[..prev_line_end].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let prev_line_len = prev_line_end - prev_line_start;

        // Move to same column in previous line (or end of line if shorter)
        prev_line_start + col.min(prev_line_len)
    }

    /// Calculate cursor position after moving down one line
    fn body_cursor_down(&self) -> usize {
        let text = &self.body;
        let cursor = self.edit_cursor();

        if text.is_empty() {
            return 0;
        }

        // Find current line start
        let current_line_start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);

        // Column in current line
        let col = cursor - current_line_start;

        // Find next line start
        let Some(newline_pos) = text[cursor..].find('\n') else {
            // No more lines, go to end
            return text.len();
        };

        let next_line_start = cursor + newline_pos + 1;

        // Find next line end
        let next_line_end = text[next_line_start..].find('\n')
            .map(|i| next_line_start + i)
            .unwrap_or(text.len());
        let next_line_len = next_line_end - next_line_start;

        // Move to same column in next line (or end of line if shorter)
        next_line_start + col.min(next_line_len)
    }

    /// Handle mouse move for single-line edit fields
    fn handle_edit_mouse_move(&mut self, event: &MouseMoveEvent, char_width: f32, cx: &mut Context<Self>) {
        if self.edit_is_selecting {
            let click_x = f32::from(event.position.x) - self.edit_input_left;
            let index = self.edit_index_for_x(click_x, char_width);
            self.edit_selection.end = index;
            cx.notify();
        }
    }

    /// Handle mouse up for edit fields
    fn handle_edit_mouse_up(&mut self, _event: &MouseUpEvent, _cx: &mut Context<Self>) {
        self.edit_is_selecting = false;
    }

    /// Unified key handler for all edit fields
    fn handle_edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(target) = self.active_edit else {
            return;
        };

        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let shift = event.keystroke.modifiers.shift;
        let is_body = matches!(target, EditTarget::Body);

        // Handle Ctrl shortcuts
        if ctrl {
            match key {
                "a" => {
                    self.edit_select_all(cx);
                    return;
                }
                "c" => {
                    if self.edit_has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(self.edit_selected_text()));
                    }
                    return;
                }
                "x" => {
                    if self.edit_has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(self.edit_selected_text()));
                        self.edit_delete_selection(cx);
                    }
                    return;
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard() {
                        if let Some(text) = item.text() {
                            let insert_text = if is_body {
                                text.to_string()
                            } else {
                                text.replace('\n', "")
                            };
                            self.edit_insert_text(&insert_text, cx);
                        }
                    }
                    return;
                }
                "z" => {
                    if shift {
                        // Ctrl+Shift+Z = Redo
                        self.edit_redo(cx);
                    } else {
                        // Ctrl+Z = Undo
                        self.edit_undo(cx);
                    }
                    return;
                }
                "y" => {
                    // Ctrl+Y = Redo (alternative)
                    self.edit_redo(cx);
                    return;
                }
                _ => {}
            }
        }

        match key {
            "left" => {
                if shift {
                    if self.edit_selection.end > 0 {
                        self.edit_selection.end -= 1;
                        cx.notify();
                    }
                } else if self.edit_has_selection() {
                    let start = self.edit_selection.start.min(self.edit_selection.end);
                    self.edit_move_to(start, cx);
                } else if self.edit_cursor() > 0 {
                    self.edit_move_to(self.edit_cursor() - 1, cx);
                }
            }
            "right" => {
                let char_count = self.get_edit_text(target).chars().count();
                if shift {
                    if self.edit_selection.end < char_count {
                        self.edit_selection.end += 1;
                        cx.notify();
                    }
                } else if self.edit_has_selection() {
                    let end = self.edit_selection.start.max(self.edit_selection.end);
                    self.edit_move_to(end, cx);
                } else if self.edit_cursor() < char_count {
                    self.edit_move_to(self.edit_cursor() + 1, cx);
                }
            }
            "home" => {
                if shift {
                    self.edit_selection.end = 0;
                    cx.notify();
                } else {
                    self.edit_move_to(0, cx);
                }
            }
            "end" => {
                let char_count = self.get_edit_text(target).chars().count();
                if shift {
                    self.edit_selection.end = char_count;
                    cx.notify();
                } else {
                    self.edit_move_to(char_count, cx);
                }
            }
            "up" => {
                if is_body {
                    let new_pos = self.body_cursor_up();
                    if shift {
                        self.edit_selection.end = new_pos;
                        cx.notify();
                    } else {
                        self.edit_move_to(new_pos, cx);
                    }
                }
            }
            "down" => {
                if is_body {
                    let new_pos = self.body_cursor_down();
                    if shift {
                        self.edit_selection.end = new_pos;
                        cx.notify();
                    } else {
                        self.edit_move_to(new_pos, cx);
                    }
                }
            }
            "backspace" => {
                if self.edit_has_selection() {
                    self.edit_delete_selection(cx);
                } else if self.edit_cursor() > 0 {
                    self.save_edit_state();
                    let char_pos = self.edit_cursor() - 1;
                    if let Some(text) = self.get_edit_text_mut(target) {
                        // Convert char position to byte offset and remove that char
                        let byte_pos = char_to_byte_offset(text, char_pos);
                        let next_byte_pos = char_to_byte_offset(text, char_pos + 1);
                        text.replace_range(byte_pos..next_byte_pos, "");
                        self.edit_selection = char_pos..char_pos;
                        self.sync_after_edit(target, cx);
                        cx.notify();
                    }
                }
            }
            "delete" => {
                let char_count = self.get_edit_text(target).chars().count();
                if self.edit_has_selection() {
                    self.edit_delete_selection(cx);
                } else {
                    let cursor = self.edit_cursor();
                    if cursor < char_count {
                        self.save_edit_state();
                        if let Some(text) = self.get_edit_text_mut(target) {
                            // Convert char position to byte offset and remove that char
                            let byte_pos = char_to_byte_offset(text, cursor);
                            let next_byte_pos = char_to_byte_offset(text, cursor + 1);
                            text.replace_range(byte_pos..next_byte_pos, "");
                            self.sync_after_edit(target, cx);
                            cx.notify();
                        }
                    }
                }
            }
            "escape" => {
                self.stop_editing(cx);
            }
            "enter" => {
                if is_body {
                    self.edit_insert_text("\n", cx);
                } else {
                    // Move to next field in kv pairs
                    self.move_to_next_field(cx);
                }
            }
            "tab" => {
                if is_body {
                    self.edit_insert_text("  ", cx);
                } else {
                    self.move_to_next_field(cx);
                }
            }
            _ => {
                // Handle printable characters
                if let Some(ch) = &event.keystroke.key_char {
                    self.edit_insert_text(ch, cx);
                }
            }
        }
    }

    /// Move to next field (for tab/enter in kv editors)
    fn move_to_next_field(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.active_edit else {
            return;
        };

        let next_target = match target {
            EditTarget::HeaderKey(i) => Some(EditTarget::HeaderValue(i)),
            EditTarget::HeaderValue(i) => {
                if i + 1 < self.headers.len() {
                    Some(EditTarget::HeaderKey(i + 1))
                } else {
                    None
                }
            }
            EditTarget::ParamKey(i) => Some(EditTarget::ParamValue(i)),
            EditTarget::ParamValue(i) => {
                if i + 1 < self.params.len() {
                    Some(EditTarget::ParamKey(i + 1))
                } else {
                    None
                }
            }
            EditTarget::FormKey(i) => Some(EditTarget::FormValue(i)),
            EditTarget::FormValue(i) => {
                if i + 1 < self.form_data.len() {
                    Some(EditTarget::FormKey(i + 1))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(next) = next_target {
            let text_len = self.get_edit_text(next).len();
            self.active_edit = Some(next);
            self.edit_selection = text_len..text_len;
            cx.notify();
        } else {
            self.stop_editing(cx);
        }
    }

    /// Save the current request to a .http file
    fn save_request(&mut self, cx: &mut Context<Self>) {
        // Generate .http file content
        let content = self.generate_http_content(cx);

        // Open save dialog
        let default_name = if self.url.is_empty() {
            "new-request.http".to_string()
        } else {
            // Extract path from URL for filename
            let url = &self.url;
            let name = url.split('/')
                .filter(|s| !s.is_empty() && !s.contains("://") && !s.contains('.'))
                .last()
                .unwrap_or("request");
            format!("{}.http", name)
        };

        let mut dialog = rfd::FileDialog::new()
            .set_title("Save Request")
            .set_file_name(&default_name)
            .add_filter("HTTP Request", &["http"]);

        if let Some(home) = dirs::home_dir() {
            dialog = dialog.set_directory(home);
        }

        if let Some(path) = dialog.save_file() {
            let path = if path.extension().map_or(true, |ext| ext != "http") {
                path.with_extension("http")
            } else {
                path
            };

            if let Err(e) = std::fs::write(&path, content) {
                eprintln!("Failed to save request: {}", e);
            }
        }
    }

    /// Generate .http file content from current request state
    fn generate_http_content(&self, cx: &Context<Self>) -> String {
        let mut lines = Vec::new();

        // Request name comment
        let name = if self.url.is_empty() {
            "New Request"
        } else {
            &self.url
        };
        lines.push(format!("### {}", name));
        lines.push(String::new());

        // Method and URL
        let method = format!("{:?}", self.method).to_uppercase();
        lines.push(format!("{} {}", method, self.url));

        // Headers
        for header in &self.headers {
            if header.enabled && !header.key.is_empty() {
                lines.push(format!("{}: {}", header.key, header.value));
            }
        }

        // Auth headers
        match self.auth_type {
            AuthType::None => {}
            AuthType::Bearer => {
                if !self.bearer_token.is_empty() {
                    lines.push(format!("Authorization: Bearer {}", self.bearer_token));
                }
            }
            AuthType::Basic => {
                if !self.basic_username.is_empty() || !self.basic_password.is_empty() {
                    // Base64 encoding for username:password
                    let credentials = format!("{}:{}", self.basic_username, self.basic_password);
                    let encoded = base64_encode(credentials.as_bytes());
                    lines.push(format!("Authorization: Basic {}", encoded));
                }
            }
            AuthType::ApiKey => {
                if !self.api_key_name.is_empty() && !self.api_key_value.is_empty() {
                    if self.api_key_location == ApiKeyLocation::Header {
                        lines.push(format!("{}: {}", self.api_key_name, self.api_key_value));
                    }
                    // Query params are added to URL, handled separately
                }
            }
        }

        // Body - get from CodeEditor
        let body_content = self.body_editor.read(cx).content().to_string();
        if !body_content.is_empty() {
            lines.push(String::new());
            lines.push(body_content);
        }

        lines.join("\n")
    }

    /// Toggle code generation dropdown
    pub(super) fn toggle_codegen_dropdown(&mut self, cx: &mut Context<Self>) {
        self.codegen_dropdown_open = !self.codegen_dropdown_open;
        cx.notify();
    }

    /// Generate code for current request using selected language
    pub(super) fn generate_code(&mut self, language: CodegenLanguage, cx: &mut Context<Self>) {
        // Build CodegenRequest from current state
        let mut headers: Vec<(String, String)> = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (h.key.clone(), h.value.clone()))
            .collect();

        // Add auth headers
        match self.auth_type {
            AuthType::Bearer if !self.bearer_token.is_empty() => {
                headers.push(("Authorization".to_string(), format!("Bearer {}", self.bearer_token)));
            }
            AuthType::Basic if !self.basic_username.is_empty() || !self.basic_password.is_empty() => {
                let credentials = format!("{}:{}", self.basic_username, self.basic_password);
                let encoded = base64_encode(credentials.as_bytes());
                headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
            }
            AuthType::ApiKey if !self.api_key_name.is_empty() && self.api_key_location == ApiKeyLocation::Header => {
                headers.push((self.api_key_name.clone(), self.api_key_value.clone()));
            }
            _ => {}
        }

        // Get body content
        let body = self.body_editor.read(cx).content().to_string();
        let body = if body.trim().is_empty() { None } else { Some(body) };

        let request = CodegenRequest {
            method: format!("{:?}", self.method).to_uppercase(),
            url: self.url.clone(),
            headers,
            body,
        };

        let code = codegen::generate(&request, language);
        self.codegen_language = language;
        self.codegen_content = Some(code);
        self.codegen_dropdown_open = false;
        cx.notify();
    }

    /// Close code modal
    pub(super) fn close_codegen_modal(&mut self, cx: &mut Context<Self>) {
        self.codegen_content = None;
        cx.notify();
    }

    /// Copy generated code to clipboard
    pub(super) fn copy_generated_code(&self, cx: &mut Context<Self>) {
        if let Some(code) = &self.codegen_content {
            cx.write_to_clipboard(ClipboardItem::new_string(code.clone()));
        }
    }

    fn send_request(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.url.is_empty() {
            return;
        }

        self.loading = true;
        cx.notify();

        // Get body content from CodeEditor
        let body_content = self.body_editor.read(cx).content().to_string();

        // Get script content from editors
        let pre_script = self.pre_script_editor.read(cx).content().to_string();
        let post_script = self.post_script_editor.read(cx).content().to_string();
        let tests_script = self.tests_editor.read(cx).content().to_string();

        // Set response panel to loading
        self.response_panel.update(cx, |panel, cx| {
            panel.set_loading(cx);
        });

        // Get environment state for variable substitution
        let env_state = self.explorer_panel.as_ref().map(|panel| {
            panel.read(cx).env_state().clone()
        });

        // Helper closure to substitute variables
        let substitute = |s: &str| -> String {
            if let Some(ref env) = env_state {
                env.substitute(s)
            } else {
                s.to_string()
            }
        };

        // Substitute variables in URL
        let url = substitute(&self.url);
        let method = self.method;
        let response_panel = self.response_panel.clone();
        let variable_extractions = self.variable_extractions.clone();
        let explorer_panel = self.explorer_panel.clone();

        // Substitute variables in headers
        let mut headers: Vec<(String, String)> = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();

        // Add authentication headers (with variable substitution)
        let auth_type = self.auth_type;
        let bearer_token = substitute(&self.bearer_token);
        let basic_username = substitute(&self.basic_username);
        let basic_password = substitute(&self.basic_password);
        let api_key_name = substitute(&self.api_key_name);
        let api_key_value = substitute(&self.api_key_value);
        let api_key_location = self.api_key_location;

        match auth_type {
            AuthType::None => {}
            AuthType::Bearer => {
                if !bearer_token.is_empty() {
                    headers.push(("Authorization".to_string(), format!("Bearer {}", bearer_token)));
                }
            }
            AuthType::Basic => {
                if !basic_username.is_empty() || !basic_password.is_empty() {
                    let credentials = format!("{}:{}", basic_username, basic_password);
                    let encoded = base64_encode(credentials.as_bytes());
                    headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
                }
            }
            AuthType::ApiKey => {
                if !api_key_name.is_empty() && !api_key_value.is_empty() {
                    if api_key_location == ApiKeyLocation::Header {
                        headers.push((api_key_name.clone(), api_key_value.clone()));
                    }
                    // Query param will be handled in URL
                }
            }
        }

        // Substitute variables in body and collect form data
        let has_files = self.body_type == BodyType::Form
            && self.form_data.iter().any(|f| f.enabled && f.field_type == FormFieldType::File && f.file_path.is_some());

        // Collect form fields for multipart (must be done before thread spawn)
        let form_fields: Vec<(String, String, Option<std::path::PathBuf>, bool)> = if self.body_type == BodyType::Form {
            self.form_data
                .iter()
                .filter(|f| f.enabled && !f.key.is_empty())
                .map(|f| (
                    substitute(&f.key),
                    substitute(&f.value),
                    f.file_path.clone(),
                    f.field_type == FormFieldType::File,
                ))
                .collect()
        } else {
            Vec::new()
        };

        let body = if matches!(method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch) {
            match self.body_type {
                BodyType::Form if !has_files => {
                    // URL-encode form data (no files)
                    let form_body: String = form_fields
                        .iter()
                        .filter(|(_, _, _, is_file)| !is_file)
                        .map(|(key, value, _, _)| {
                            format!("{}={}", url_encode(key), url_encode(value))
                        })
                        .collect::<Vec<_>>()
                        .join("&");
                    if form_body.is_empty() { None } else { Some(form_body) }
                }
                BodyType::Form => None, // Will use multipart
                _ => Some(substitute(&body_content)),
            }
        } else {
            None
        };

        // Run pre-script if present
        let (mut url, mut headers, mut body) = (url, headers, body);
        let env_vars: std::collections::HashMap<String, String> = env_state
            .as_ref()
            .and_then(|e| e.active())
            .map(|env| env.variables.clone())
            .unwrap_or_default();

        if !pre_script.trim().is_empty() {
            let engine = match ScriptEngine::new() {
                Ok(e) => e,
                Err(e) => {
                    self.loading = false;
                    self.response_panel.update(cx, |panel, cx| {
                        panel.set_error(format!("Script engine error: {}", e.message), cx);
                    });
                    cx.notify();
                    return;
                }
            };

            let script_request = ScriptRequestData::new(method.as_str(), &url)
                .with_headers(headers.clone())
                .with_body(body.clone().unwrap_or_default());
            let mut script_ctx = ScriptContext::new()
                .with_request(script_request)
                .with_env(env_vars.clone());

            match engine.run_pre_script(&pre_script, &mut script_ctx) {
                Ok(outcome) => {
                    if !outcome.success {
                        if let Some(error) = outcome.error {
                            self.loading = false;
                            self.response_panel.update(cx, |panel, cx| {
                                panel.set_error(format!("Pre-script error: {}", error.message), cx);
                            });
                            cx.notify();
                            return;
                        }
                    }
                    // Apply modifications
                    if let Some(new_url) = outcome.modified_request.url {
                        url = new_url;
                    }
                    for (name, value) in outcome.modified_request.headers_to_set {
                        // Remove existing header with same name (case-insensitive)
                        headers.retain(|(k, _)| !k.eq_ignore_ascii_case(&name));
                        headers.push((name, value));
                    }
                    for name in &outcome.modified_request.headers_to_remove {
                        headers.retain(|(k, _)| !k.eq_ignore_ascii_case(name));
                    }
                    if let Some(new_body) = outcome.modified_request.body {
                        body = Some(new_body);
                    }
                }
                Err(e) => {
                    self.loading = false;
                    self.response_panel.update(cx, |panel, cx| {
                        panel.set_error(format!("Pre-script error: {}", e.message), cx);
                    });
                    cx.notify();
                    return;
                }
            }
        }

        // Build final URL with API key as query param if needed
        let final_url = if auth_type == AuthType::ApiKey
            && api_key_location == ApiKeyLocation::QueryParam
            && !api_key_name.is_empty()
            && !api_key_value.is_empty()
        {
            if url.contains('?') {
                format!("{}&{}={}", url, api_key_name, api_key_value)
            } else {
                format!("{}?{}={}", url, api_key_name, api_key_value)
            }
        } else {
            url
        };

        // Add to history
        let history_id = cx.update_global::<super::history::RequestHistory, _>(|history, _| {
            history.add(
                method.as_str().to_string(),
                final_url.clone(),
                headers.clone(),
                body.clone(),
            )
        });

        // Spawn background thread for HTTP request (reqwest blocking)
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            // Run blocking HTTP in a thread
            let result = std::thread::spawn(move || {
                let start = std::time::Instant::now();

                let client = reqwest::blocking::Client::new();
                let req_method = match method {
                    HttpMethod::Get => reqwest::Method::GET,
                    HttpMethod::Post => reqwest::Method::POST,
                    HttpMethod::Put => reqwest::Method::PUT,
                    HttpMethod::Patch => reqwest::Method::PATCH,
                    HttpMethod::Delete => reqwest::Method::DELETE,
                };

                let mut req_builder = client.request(req_method, &final_url);

                // Add headers (skip Content-Type for multipart - reqwest sets it)
                for (key, value) in headers {
                    if has_files && key.eq_ignore_ascii_case("content-type") {
                        continue; // Let reqwest set multipart Content-Type with boundary
                    }
                    req_builder = req_builder.header(&key, &value);
                }

                // Add body for POST/PUT/PATCH
                if has_files && !form_fields.is_empty() {
                    // Build multipart form
                    let mut form = reqwest::blocking::multipart::Form::new();
                    for (key, value, file_path, is_file) in form_fields {
                        if is_file {
                            if let Some(path) = file_path {
                                if let Ok(part) = reqwest::blocking::multipart::Part::file(&path) {
                                    form = form.part(key, part);
                                }
                            }
                        } else {
                            form = form.text(key, value);
                        }
                    }
                    req_builder = req_builder.multipart(form);
                } else if let Some(body_content) = body {
                    req_builder = req_builder.body(body_content);
                }

                let result = req_builder.send();
                let elapsed = start.elapsed();

                match result {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        let status_text_str = status_text(status).to_string();
                        let headers: Vec<(String, String)> = response
                            .headers()
                            .iter()
                            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                            .collect();

                        let body = response.text().unwrap_or_default();
                        let size = body.len();

                        Ok(ResponseData {
                            status,
                            status_text: status_text_str,
                            headers,
                            body,
                            time: elapsed,
                            size,
                        })
                    }
                    Err(e) => Err(e.to_string()),
                }
            }).join();

            match result {
                Ok(Ok(data)) => {
                    let status = data.status;
                    let response_time = data.time;

                    // Run post-script and tests
                    let has_scripts = !post_script.trim().is_empty() || !tests_script.trim().is_empty();
                    let test_results = if has_scripts {
                        // Clone data for script execution
                        let script_status = data.status;
                        let script_status_text = data.status_text.clone();
                        let script_body = data.body.clone();
                        let script_headers = data.headers.clone();
                        let script_time = data.time.as_millis() as u64;
                        let script_size = data.size;

                        std::thread::spawn(move || {
                            let engine = match ScriptEngine::new() {
                                Ok(e) => e,
                                Err(_) => return Vec::new(),
                            };

                            let script_response = ScriptResponseData::new(script_status, &script_status_text, script_body)
                                .with_headers(script_headers)
                                .with_time(script_time)
                                .with_size(script_size);

                            let mut script_ctx = ScriptContext::new().with_env(env_vars);
                            script_ctx.set_response(script_response);

                            // Run post-script (ignore errors, just run)
                            if !post_script.trim().is_empty() {
                                let _ = engine.run_post_script(&post_script, &mut script_ctx);
                            }

                            // Run tests
                            if !tests_script.trim().is_empty() {
                                if let Ok(outcome) = engine.run_tests(&tests_script, &mut script_ctx) {
                                    return outcome.test_results;
                                }
                            }

                            Vec::new()
                        }).join().unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    // Run variable extractions from @set annotations
                    let extracted_vars: Vec<(String, String)> = if !variable_extractions.is_empty() {
                        chaining::extract_variables(&data.body, &variable_extractions)
                            .into_iter()
                            .filter(|r| r.success)
                            .map(|r| (r.name, r.value))
                            .collect()
                    } else {
                        Vec::new()
                    };

                    let _ = cx.update(|cx| {
                        // Update history with response
                        cx.update_global::<super::history::RequestHistory, _>(|history, _| {
                            history.update_response(history_id, status, response_time);
                        });

                        // Apply extracted variables to environment
                        if !extracted_vars.is_empty() {
                            if let Some(explorer) = &explorer_panel {
                                explorer.update(cx, |panel, cx| {
                                    for (name, value) in &extracted_vars {
                                        panel.set_env_variable(name, value, cx);
                                    }
                                });
                            }
                        }

                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(data, cx);
                            if !test_results.is_empty() {
                                panel.set_test_results(test_results, cx);
                            }
                        });
                    });
                }
                Ok(Err(e)) => {
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_error(e, cx);
                        });
                    });
                }
                Err(_) => {
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_error("Request thread panicked".to_string(), cx);
                        });
                    });
                }
            }

            let _ = cx.update(|cx| {
                let _ = this.update(cx, |this, cx| {
                    this.loading = false;
                    cx.notify();
                });
            });
        }).detach();
    }

    fn focus_url(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.url_focus.focus(window, cx);
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let char_count = self.url.chars().count();
        let offset = offset.min(char_count);
        self.url_selection = offset..offset;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let char_count = self.url.chars().count();
        let offset = offset.min(char_count);
        self.url_selection.end = offset;
        // Normalize range
        if self.url_selection.end < self.url_selection.start {
            self.url_selection = self.url_selection.end..self.url_selection.start;
        }
        cx.notify();
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        self.url_selection = 0..self.url.chars().count();
        cx.notify();
    }

    fn has_selection(&self) -> bool {
        self.url_selection.start != self.url_selection.end
    }

    fn selected_text(&self) -> String {
        let byte_start = char_to_byte_offset(&self.url, self.url_selection.start);
        let byte_end = char_to_byte_offset(&self.url, self.url_selection.end);
        self.url[byte_start..byte_end].to_string()
    }

    fn delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.has_selection() {
            self.save_url_state();
            self.delete_selection_no_save(cx);
        }
    }

    /// Delete selection without saving to undo (used internally)
    fn delete_selection_no_save(&mut self, cx: &mut Context<Self>) {
        if self.has_selection() {
            let char_start = self.url_selection.start.min(self.url_selection.end);
            let char_end = self.url_selection.start.max(self.url_selection.end);
            // Convert character indices to byte offsets
            let byte_start = char_to_byte_offset(&self.url, char_start);
            let byte_end = char_to_byte_offset(&self.url, char_end);
            self.url.replace_range(byte_start..byte_end, "");
            self.url_selection = char_start..char_start;
            self.sync_params_from_url(cx);
            cx.notify();
        }
    }

    /// Save URL state to undo stack before making changes
    fn save_url_state(&mut self) {
        self.url_undo_stack.push((self.url.clone(), self.url_selection.clone()));
        if self.url_undo_stack.len() > 100 {
            self.url_undo_stack.remove(0);
        }
        self.url_redo_stack.clear();
    }

    /// Undo URL change
    fn url_undo(&mut self, cx: &mut Context<Self>) {
        if let Some((text, selection)) = self.url_undo_stack.pop() {
            self.url_redo_stack.push((self.url.clone(), self.url_selection.clone()));
            self.url = text;
            self.url_selection = selection;
            self.sync_params_from_url(cx);
            cx.notify();
        }
    }

    /// Redo URL change
    fn url_redo(&mut self, cx: &mut Context<Self>) {
        if let Some((text, selection)) = self.url_redo_stack.pop() {
            self.url_undo_stack.push((self.url.clone(), self.url_selection.clone()));
            self.url = text;
            self.url_selection = selection;
            self.sync_params_from_url(cx);
            cx.notify();
        }
    }

    fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.save_url_state();
        self.delete_selection_no_save(cx);
        let char_pos = self.url_selection.start;
        // Convert character index to byte offset for string operation
        let byte_pos = char_to_byte_offset(&self.url, char_pos);
        self.url.insert_str(byte_pos, text);
        // New position is after the inserted text (in character indices)
        let new_char_pos = char_pos + text.chars().count();
        self.url_selection = new_char_pos..new_char_pos;
        self.sync_params_from_url(cx);
        cx.notify();
    }

    fn index_for_x(&self, x: f32) -> usize {
        // Approximate character position from x coordinate
        // ~7.8px per character at 13px font size
        let char_width: f32 = 7.8;
        if x <= 0.0 {
            0
        } else {
            let approx_char = (x / char_width) as usize;
            approx_char.min(self.url.chars().count())
        }
    }

    fn handle_url_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        self.is_selecting = true;
        let click_x = f32::from(event.position.x) - self.url_input_left;
        let index = self.index_for_x(click_x);

        // Cycle: 1=cursor, 2=word, 3=all, 4+=cursor
        let effective_click = if event.click_count >= 4 { 1 } else { event.click_count };

        match effective_click {
            2 => {
                // Double-click: select word
                let start = find_word_start(&self.url, index);
                let end = find_word_end(&self.url, index);
                self.url_selection = start..end;
                cx.notify();
            }
            3 => {
                // Triple-click: select all
                self.select_all(cx);
            }
            _ => {
                // Single click (or 4th+ click to deselect)
                if event.modifiers.shift {
                    self.select_to(index, cx);
                } else {
                    self.move_to(index, cx);
                }
            }
        }
    }

    fn handle_url_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.is_selecting {
            let click_x = f32::from(event.position.x) - self.url_input_left;
            let index = self.index_for_x(click_x);
            self.url_selection.end = index.min(self.url.chars().count());
            cx.notify();
        }
    }

    fn handle_url_mouse_up(&mut self, _event: &MouseUpEvent, _cx: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn handle_url_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let shift = event.keystroke.modifiers.shift;

        // Handle Ctrl shortcuts
        if ctrl {
            match key {
                "enter" => {
                    // Ctrl+Enter = Send request
                    self.send_request(cx);
                    return;
                }
                "a" => {
                    self.select_all(cx);
                    return;
                }
                "c" => {
                    if self.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            self.selected_text().to_string(),
                        ));
                    }
                    return;
                }
                "x" => {
                    if self.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            self.selected_text().to_string(),
                        ));
                        self.delete_selection(cx);
                    }
                    return;
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard() {
                        if let Some(text) = item.text() {
                            self.insert_text(&text.replace('\n', ""), cx);
                        }
                    }
                    return;
                }
                "z" => {
                    if shift {
                        // Ctrl+Shift+Z = Redo
                        self.url_redo(cx);
                    } else {
                        // Ctrl+Z = Undo
                        self.url_undo(cx);
                    }
                    return;
                }
                "y" => {
                    // Ctrl+Y = Redo (alternative)
                    self.url_redo(cx);
                    return;
                }
                _ => {}
            }
        }

        match key {
            "left" => {
                if shift {
                    if self.url_selection.end > 0 {
                        self.url_selection.end -= 1;
                        cx.notify();
                    }
                } else if self.has_selection() {
                    let start = self.url_selection.start.min(self.url_selection.end);
                    self.move_to(start, cx);
                } else if self.cursor() > 0 {
                    self.move_to(self.cursor() - 1, cx);
                }
            }
            "right" => {
                let char_count = self.url.chars().count();
                if shift {
                    if self.url_selection.end < char_count {
                        self.url_selection.end += 1;
                        cx.notify();
                    }
                } else if self.has_selection() {
                    let end = self.url_selection.start.max(self.url_selection.end);
                    self.move_to(end, cx);
                } else if self.cursor() < char_count {
                    self.move_to(self.cursor() + 1, cx);
                }
            }
            "home" => {
                if shift {
                    self.url_selection.end = 0;
                    cx.notify();
                } else {
                    self.move_to(0, cx);
                }
            }
            "end" => {
                let char_count = self.url.chars().count();
                if shift {
                    self.url_selection.end = char_count;
                    cx.notify();
                } else {
                    self.move_to(char_count, cx);
                }
            }
            "backspace" => {
                if self.has_selection() {
                    self.delete_selection(cx);
                } else if self.cursor() > 0 {
                    self.save_url_state();
                    let char_pos = self.cursor() - 1;
                    // Convert char position to byte offset and remove that char
                    let byte_pos = char_to_byte_offset(&self.url, char_pos);
                    let next_byte_pos = char_to_byte_offset(&self.url, char_pos + 1);
                    self.url.replace_range(byte_pos..next_byte_pos, "");
                    self.url_selection = char_pos..char_pos;
                    self.sync_params_from_url(cx);
                    cx.notify();
                }
            }
            "delete" => {
                let char_count = self.url.chars().count();
                if self.has_selection() {
                    self.delete_selection(cx);
                } else if self.cursor() < char_count {
                    self.save_url_state();
                    let cursor = self.cursor();
                    // Convert char position to byte offset and remove that char
                    let byte_pos = char_to_byte_offset(&self.url, cursor);
                    let next_byte_pos = char_to_byte_offset(&self.url, cursor + 1);
                    self.url.replace_range(byte_pos..next_byte_pos, "");
                    self.sync_params_from_url(cx);
                    cx.notify();
                }
            }
            "enter" => {
                self.send_request(cx);
            }
            _ => {
                // Handle printable characters
                if let Some(ch) = &event.keystroke.key_char {
                    self.insert_text(ch, cx);
                }
            }
        }
    }
}

impl Render for RequestPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Reset skip_blur flag at start of each render
        self.skip_blur = false;

        div()
            .id("request-panel")
            .size_full()
            .flex()
            .flex_col()
            .relative()
            .track_focus(&self.body_focus)
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                // Route key events based on active_edit
                if this.active_edit.is_some() {
                    this.handle_edit_key(event, cx);
                }
            }))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                // Only clear focus if an input wasn't clicked
                if !this.skip_blur && this.active_edit.is_some() {
                    this.active_edit = None;
                    cx.notify();
                }
                // Close dropdown when clicking outside
                if this.method_dropdown_open {
                    this.method_dropdown_open = false;
                    cx.notify();
                }
            }))
            // URL bar
            .child(self.render_url_bar(window, cx))
            // Tabs
            .child(self.render_tabs(cx))
            // Tab content
            .child(
                div()
                    .id("tab-content")
                    .flex_1()
                    .p(px(12.0))
                    .overflow_scroll()
                    .child(self.render_tab_content(cx)),
            )
            // Floating dropdown overlay (rendered last to be on top)
            .when(self.method_dropdown_open, |el| {
                el.child(self.render_method_dropdown_overlay(cx))
            })
            // Code generation dropdown overlay
            .when(self.codegen_dropdown_open, |el| {
                el.child(self.render_codegen_dropdown_overlay(cx))
            })
            // Code generation modal
            .when(self.codegen_content.is_some(), |el| {
                el.child(self.render_codegen_modal(cx))
            })
    }
}


