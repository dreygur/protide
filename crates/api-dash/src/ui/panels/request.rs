//! Request editor panel

use std::ops::Range;

use gpui::{
    div, prelude::*, px, ClipboardItem, Context, Entity, FocusHandle, IntoElement, KeyDownEvent,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render, SharedString, Styled,
    Window,
};

use crate::theme;
use crate::ui::components::{render_text_view, find_word_start, find_word_end, is_word_char};
use super::explorer::ExplorerPanel;
use super::response::{ResponseData, ResponsePanel};

/// A key-value pair for headers, params, etc.
#[derive(Clone, Default)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

/// Identifies which text field is currently being edited
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EditTarget {
    Url,
    HeaderKey(usize),
    HeaderValue(usize),
    ParamKey(usize),
    ParamValue(usize),
    Body,
    BearerToken,
    BasicUsername,
    BasicPassword,
    ApiKeyName,
    ApiKeyValue,
}

/// Authentication type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AuthType {
    #[default]
    None,
    Bearer,
    Basic,
    ApiKey,
}

/// Where to send the API key
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ApiKeyLocation {
    #[default]
    Header,
    QueryParam,
}

/// HTTP methods
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "PATCH" => Some(HttpMethod::Patch),
            "DELETE" => Some(HttpMethod::Delete),
            _ => None,
        }
    }

    pub fn all() -> &'static [HttpMethod] {
        &[
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Patch,
            HttpMethod::Delete,
        ]
    }
}

// Word boundary functions imported from crate::ui::components

/// Request editor panel
pub struct RequestPanel {
    /// Active tab index for params/headers/body/auth
    active_tab: usize,
    /// Selected HTTP method
    method: HttpMethod,
    /// URL text
    url: String,
    /// Selection range (start..end), when empty cursor is at end
    url_selection: Range<usize>,
    /// Whether method dropdown is open
    method_dropdown_open: bool,
    /// Focus handle for URL input
    url_focus: FocusHandle,
    /// Whether mouse is selecting (for URL)
    is_selecting: bool,
    /// URL input element left offset for click calculation
    url_input_left: f32,
    /// Reference to response panel for updating results
    response_panel: Entity<ResponsePanel>,
    /// Whether a request is in flight
    loading: bool,
    /// Request headers
    headers: Vec<KeyValuePair>,
    /// Query parameters
    params: Vec<KeyValuePair>,
    /// Request body (JSON/raw text)
    body: String,
    /// Flag to prevent infinite URL<->params sync loops
    syncing_params: bool,
    /// Authentication type
    auth_type: AuthType,
    /// Bearer token
    bearer_token: String,
    /// Basic auth username
    basic_username: String,
    /// Basic auth password
    basic_password: String,
    /// API key header/param name
    api_key_name: String,
    /// API key value
    api_key_value: String,
    /// Where to send API key (header or query param)
    api_key_location: ApiKeyLocation,
    /// Currently active edit target (what field is being edited)
    active_edit: Option<EditTarget>,
    /// Selection range for active edit
    edit_selection: Range<usize>,
    /// Whether mouse is selecting in edit field
    edit_is_selecting: bool,
    /// Edit input left offset for click calculation
    edit_input_left: f32,
    /// Focus handle for editable fields
    edit_focus: FocusHandle,
    /// Reference to explorer panel for environment variable substitution
    explorer_panel: Option<Entity<ExplorerPanel>>,
}

impl RequestPanel {
    pub fn new(cx: &mut Context<Self>, response_panel: Entity<ResponsePanel>) -> Self {
        let url = "https://httpbin.org/post".to_string();
        let url_len = url.len();
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
            body: "{\n  \"name\": \"API Dash\",\n  \"version\": \"0.1.0\"\n}".to_string(),
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
            edit_focus: cx.focus_handle(),
            explorer_panel: None,
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
        self.url_selection = self.url.len()..self.url.len();

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
            self.body = b;
        }

        // Sync params from URL
        self.sync_params_from_url(cx);

        // Reset editing state
        self.active_edit = None;
        self.method_dropdown_open = false;

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
        self.edit_focus.focus(window, cx);
        cx.notify();
    }

    /// Stop editing
    fn stop_editing(&mut self, cx: &mut Context<Self>) {
        self.active_edit = None;
        self.edit_selection = 0..0;
        self.edit_is_selecting = false;
        cx.notify();
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

    /// Move cursor to position in current edit
    fn edit_move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            let text_len = self.get_edit_text(target).len();
            let offset = offset.min(text_len);
            self.edit_selection = offset..offset;
            cx.notify();
        }
    }

    /// Extend selection to position
    fn edit_select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            let text_len = self.get_edit_text(target).len();
            self.edit_selection.end = offset.min(text_len);
            cx.notify();
        }
    }

    /// Select all text in current edit
    fn edit_select_all(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            let text_len = self.get_edit_text(target).len();
            self.edit_selection = 0..text_len;
            cx.notify();
        }
    }

    /// Delete selected text
    fn edit_delete_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            if self.edit_has_selection() {
                let start = self.edit_selection.start.min(self.edit_selection.end);
                let end = self.edit_selection.start.max(self.edit_selection.end);
                if let Some(text) = self.get_edit_text_mut(target) {
                    text.replace_range(start..end, "");
                    self.edit_selection = start..start;
                    // Sync URL <-> params
                    self.sync_after_edit(target, cx);
                    cx.notify();
                }
            }
        }
    }

    /// Insert text at cursor (replacing selection if any)
    fn edit_insert_text(&mut self, insert: &str, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            self.edit_delete_selection(cx);
            let pos = self.edit_selection.start;
            if let Some(text) = self.get_edit_text_mut(target) {
                text.insert_str(pos, insert);
                let new_pos = pos + insert.len();
                self.edit_selection = new_pos..new_pos;
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

    /// Calculate character index from x position
    fn edit_index_for_x(&self, x: f32, char_width: f32) -> usize {
        if let Some(target) = self.active_edit {
            let text_len = self.get_edit_text(target).len();
            if x <= 0.0 {
                0
            } else {
                let approx_char = (x / char_width) as usize;
                approx_char.min(text_len)
            }
        } else {
            0
        }
    }

    /// Handle mouse down for edit fields
    fn handle_edit_mouse_down(&mut self, event: &MouseDownEvent, input_left: f32, char_width: f32, cx: &mut Context<Self>) {
        self.edit_is_selecting = true;
        self.edit_input_left = input_left;
        let click_x = f32::from(event.position.x) - input_left;
        let index = self.edit_index_for_x(click_x, char_width);

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

    /// Handle mouse move for edit fields
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
                let text_len = self.get_edit_text(target).len();
                if shift {
                    if self.edit_selection.end < text_len {
                        self.edit_selection.end += 1;
                        cx.notify();
                    }
                } else if self.edit_has_selection() {
                    let end = self.edit_selection.start.max(self.edit_selection.end);
                    self.edit_move_to(end, cx);
                } else if self.edit_cursor() < text_len {
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
                let text_len = self.get_edit_text(target).len();
                if shift {
                    self.edit_selection.end = text_len;
                    cx.notify();
                } else {
                    self.edit_move_to(text_len, cx);
                }
            }
            "backspace" => {
                if self.edit_has_selection() {
                    self.edit_delete_selection(cx);
                } else if self.edit_cursor() > 0 {
                    let pos = self.edit_cursor() - 1;
                    if let Some(text) = self.get_edit_text_mut(target) {
                        text.remove(pos);
                        self.edit_selection = pos..pos;
                        self.sync_after_edit(target, cx);
                        cx.notify();
                    }
                }
            }
            "delete" => {
                let text_len = self.get_edit_text(target).len();
                if self.edit_has_selection() {
                    self.edit_delete_selection(cx);
                } else {
                    let cursor = self.edit_cursor();
                    if cursor < text_len {
                        if let Some(text) = self.get_edit_text_mut(target) {
                            text.remove(cursor);
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

    fn send_request(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.url.is_empty() {
            return;
        }

        self.loading = true;
        cx.notify();

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

        // Substitute variables in body
        let body = if matches!(method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch) {
            Some(substitute(&self.body))
        } else {
            None
        };

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

                // Add headers
                for (key, value) in headers {
                    req_builder = req_builder.header(&key, &value);
                }

                // Add body for POST/PUT/PATCH
                if let Some(body_content) = body {
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
                    let _ = cx.update(|cx| {
                        // Update history with response
                        cx.update_global::<super::history::RequestHistory, _>(|history, _| {
                            history.update_response(history_id, status, response_time);
                        });
                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(data, cx);
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
        let offset = offset.min(self.url.len());
        self.url_selection = offset..offset;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = offset.min(self.url.len());
        self.url_selection.end = offset;
        // Normalize range
        if self.url_selection.end < self.url_selection.start {
            self.url_selection = self.url_selection.end..self.url_selection.start;
        }
        cx.notify();
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        self.url_selection = 0..self.url.len();
        cx.notify();
    }

    fn has_selection(&self) -> bool {
        self.url_selection.start != self.url_selection.end
    }

    fn selected_text(&self) -> &str {
        &self.url[self.url_selection.clone()]
    }

    fn delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.has_selection() {
            let start = self.url_selection.start;
            self.url.replace_range(self.url_selection.clone(), "");
            self.url_selection = start..start;
            self.sync_params_from_url(cx);
            cx.notify();
        }
    }

    fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.delete_selection(cx);
        let pos = self.url_selection.start;
        self.url.insert_str(pos, text);
        let new_pos = pos + text.len();
        self.url_selection = new_pos..new_pos;
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
            approx_char.min(self.url.len())
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
            self.url_selection.end = index.min(self.url.len());
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
                if shift {
                    if self.url_selection.end < self.url.len() {
                        self.url_selection.end += 1;
                        cx.notify();
                    }
                } else if self.has_selection() {
                    let end = self.url_selection.start.max(self.url_selection.end);
                    self.move_to(end, cx);
                } else if self.cursor() < self.url.len() {
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
                if shift {
                    self.url_selection.end = self.url.len();
                    cx.notify();
                } else {
                    self.move_to(self.url.len(), cx);
                }
            }
            "backspace" => {
                if self.has_selection() {
                    self.delete_selection(cx);
                } else if self.cursor() > 0 {
                    let pos = self.cursor() - 1;
                    self.url.remove(pos);
                    self.url_selection = pos..pos;
                    self.sync_params_from_url(cx);
                    cx.notify();
                }
            }
            "delete" => {
                if self.has_selection() {
                    self.delete_selection(cx);
                } else if self.cursor() < self.url.len() {
                    self.url.remove(self.cursor());
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
        div()
            .size_full()
            .flex()
            .flex_col()
            // URL bar
            .child(self.render_url_bar(window, cx))
            // Tabs
            .child(self.render_tabs(cx))
            // Tab content
            .child(
                div()
                    .flex_1()
                    .p(px(12.0))
                    .overflow_hidden()
                    .child(self.render_tab_content(cx)),
            )
    }
}

impl RequestPanel {
    fn render_url_bar(&mut self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let method = self.method;
        let method_color = theme.method_color(method.as_str());
        let is_url_focused = self.url_focus.is_focused(window);

        // Calculate approximate left offset of URL input
        // Sidebar: 250px + method area + padding
        self.url_input_left = 293.0;

        div()
            .h(px(48.0))
            .w_full()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .border_b_1()
            .border_color(theme.colors.border)
            // Method selector with dropdown
            .child(
                div()
                    .relative()
                    .child(
                        div()
                            .id("method-selector")
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .bg(theme.colors.bg_tertiary)
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(method_color)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_elevated))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_method_dropdown(cx);
                            }))
                            .child(method.as_str()),
                    )
                    .when(self.method_dropdown_open, |el| {
                        el.child(self.render_method_dropdown(cx))
                    }),
            )
            // URL input with selection support
            .child(
                div()
                    .id("url-input")
                    .flex_1()
                    .h(px(32.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .rounded(px(4.0))
                    .border_1()
                    .when(is_url_focused, |el| {
                        el.border_color(theme.colors.border_focused)
                    })
                    .when(!is_url_focused, |el| el.border_color(theme.colors.border))
                    .bg(theme.colors.bg_tertiary)
                    .cursor_text()
                    .track_focus(&self.url_focus)
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            this.focus_url(window, cx);
                            this.handle_url_mouse_down(event, cx);
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                        this.handle_url_mouse_move(event, cx);
                    }))
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &MouseUpEvent, _, cx| {
                            this.handle_url_mouse_up(event, cx);
                        }),
                    )
                    .on_mouse_up_out(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &MouseUpEvent, _, cx| {
                            this.handle_url_mouse_up(event, cx);
                        }),
                    )
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        this.handle_url_key(event, cx);
                    }))
                    .child(self.render_url_text(is_url_focused, cx)),
            )
            // Send button
            .child({
                let is_loading = self.loading;
                div()
                    .id("send-button")
                    .px(px(16.0))
                    .py(px(8.0))
                    .rounded(px(4.0))
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(gpui::white())
                    .when(is_loading, |el| {
                        el.bg(theme.colors.text_muted)
                            .cursor_default()
                    })
                    .when(!is_loading, |el| {
                        el.bg(theme.colors.accent)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.colors.accent_hover))
                            .active(|style| style.opacity(0.8))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.send_request(cx);
                            }))
                    })
                    .child(if is_loading { "Sending..." } else { "Send" })
            })
    }

    fn render_url_text(&self, is_focused: bool, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        render_text_view(
            &self.url,
            &self.url_selection,
            is_focused,
            13.0,
            theme.colors.text_primary,
            Some("Enter request URL..."),
            theme.colors.text_muted,
        )
    }

    fn render_method_dropdown(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .absolute()
            .top(px(36.0))
            .left_0()
            .min_w(px(100.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .children(HttpMethod::all().iter().map(|&m| {
                let method_color = theme.method_color(m.as_str());
                let is_selected = m == self.method;

                div()
                    .id(SharedString::from(format!("method-{}", m.as_str())))
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(method_color)
                    .cursor_pointer()
                    .when(is_selected, |el| el.bg(theme.colors.bg_tertiary))
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_method(m, cx);
                    }))
                    .child(m.as_str())
            }))
    }

    fn render_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let tabs = ["Params", "Headers", "Body", "Auth"];
        let active_tab = self.active_tab;

        div()
            .h(px(36.0))
            .w_full()
            .flex()
            .items_center()
            .px(px(12.0))
            .gap(px(4.0))
            .border_b_1()
            .border_color(theme.colors.border)
            .children(tabs.iter().enumerate().map(|(i, tab)| {
                let is_active = i == active_tab;
                div()
                    .id(SharedString::from(format!("tab-{}", i)))
                    .px(px(12.0))
                    .py(px(8.0))
                    .text_size(px(13.0))
                    .cursor_pointer()
                    .when(is_active, |el| {
                        el.text_color(theme.colors.text_primary)
                            .border_b_2()
                            .border_color(theme.colors.accent)
                    })
                    .when(!is_active, |el| {
                        el.text_color(theme.colors.text_secondary)
                            .hover(|style| style.text_color(theme.colors.text_primary))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_tab(i, cx);
                    }))
                    .child(*tab)
            }))
    }

    fn render_tab_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.active_tab {
            0 => self.render_params_tab(cx),
            1 => self.render_headers_tab(cx),
            2 => self.render_body_tab(cx),
            3 => self.render_auth_tab(cx),
            _ => div().into_any_element(),
        }
    }

    fn render_params_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let params_len = self.params.len();
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();

        // Collect param data first to avoid borrow issues
        let params_data: Vec<_> = self.params.iter().enumerate().map(|(i, param)| {
            (i, param.enabled, param.key.clone(), param.value.clone())
        }).collect();

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .track_focus(&self.edit_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_edit_key(event, cx);
            }));

        // Params list
        for (i, is_enabled, key, value) in params_data {
            let can_remove = params_len > 1;
            let is_editing_key = active_edit == Some(EditTarget::ParamKey(i));
            let is_editing_value = active_edit == Some(EditTarget::ParamValue(i));

            container = container.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    // Checkbox
                    .child(
                        div()
                            .id(SharedString::from(format!("param-checkbox-{}", i)))
                            .size(px(16.0))
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(theme.colors.border)
                            .cursor_pointer()
                            .when(is_enabled, |el| el.bg(theme.colors.accent))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(is_enabled, |el| {
                                el.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(gpui::white())
                                        .child("✓")
                                )
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_param(i, cx);
                            }))
                    )
                    // Key input
                    .child(
                        self.render_kv_input(
                            format!("param-key-{}", i),
                            EditTarget::ParamKey(i),
                            &key,
                            "Key",
                            is_editing_key,
                            if is_editing_key { edit_selection.clone() } else { 0..0 },
                            px(150.0),
                            cx,
                        )
                    )
                    // Value input
                    .child(
                        self.render_kv_input_flex(
                            format!("param-value-{}", i),
                            EditTarget::ParamValue(i),
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
                            .id(SharedString::from(format!("param-remove-{}", i)))
                            .size(px(24.0))
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .when(can_remove, |el| {
                                el.hover(|s| s.bg(theme.colors.bg_tertiary))
                                    .text_color(theme.colors.text_muted)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.remove_param(i, cx);
                                    }))
                            })
                            .when(!can_remove, |el| el.text_color(theme.colors.border))
                            .child("×")
                    )
            );
        }

        // Add param button
        container = container.child(
            div()
                .id("add-param-btn")
                .pt(px(4.0))
                .flex()
                .items_center()
                .gap(px(4.0))
                .cursor_pointer()
                .text_size(px(12.0))
                .text_color(theme.colors.accent)
                .hover(|s| s.text_color(theme.colors.text_primary))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.add_param(cx);
                }))
                .child("+")
                .child("Add Parameter")
        );

        container.into_any_element()
    }

    fn render_headers_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let headers_len = self.headers.len();
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();

        // Collect header data first to avoid borrow issues
        let headers_data: Vec<_> = self.headers.iter().enumerate().map(|(i, header)| {
            (i, header.enabled, header.key.clone(), header.value.clone())
        }).collect();

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .track_focus(&self.edit_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_edit_key(event, cx);
            }));

        // Headers list
        for (i, is_enabled, key, value) in headers_data {
            let can_remove = headers_len > 1;
            let is_editing_key = active_edit == Some(EditTarget::HeaderKey(i));
            let is_editing_value = active_edit == Some(EditTarget::HeaderValue(i));

            container = container.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    // Checkbox
                    .child(
                        div()
                            .id(SharedString::from(format!("header-checkbox-{}", i)))
                            .size(px(16.0))
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(theme.colors.border)
                            .cursor_pointer()
                            .when(is_enabled, |el| el.bg(theme.colors.accent))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(is_enabled, |el| {
                                el.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(gpui::white())
                                        .child("✓")
                                )
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_header(i, cx);
                            }))
                    )
                    // Key input
                    .child(
                        self.render_kv_input(
                            format!("header-key-{}", i),
                            EditTarget::HeaderKey(i),
                            &key,
                            "Key",
                            is_editing_key,
                            if is_editing_key { edit_selection.clone() } else { 0..0 },
                            px(150.0),
                            cx,
                        )
                    )
                    // Value input
                    .child(
                        self.render_kv_input_flex(
                            format!("header-value-{}", i),
                            EditTarget::HeaderValue(i),
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
                            .id(SharedString::from(format!("header-remove-{}", i)))
                            .size(px(24.0))
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .when(can_remove, |el| {
                                el.hover(|s| s.bg(theme.colors.bg_tertiary))
                                    .text_color(theme.colors.text_muted)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.remove_header(i, cx);
                                    }))
                            })
                            .when(!can_remove, |el| el.text_color(theme.colors.border))
                            .child("×")
                    )
            );
        }

        // Add header button
        container = container.child(
            div()
                .id("add-header-btn")
                .pt(px(4.0))
                .flex()
                .items_center()
                .gap(px(4.0))
                .cursor_pointer()
                .text_size(px(12.0))
                .text_color(theme.colors.accent)
                .hover(|s| s.text_color(theme.colors.text_primary))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.add_header(cx);
                }))
                .child("+")
                .child("Add Header")
        );

        container.into_any_element()
    }

    fn render_body_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let is_editing_body = self.active_edit == Some(EditTarget::Body);
        let edit_selection = self.edit_selection.clone();
        let body = self.body.clone();

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Body type selector
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.colors.accent)
                            .text_size(px(12.0))
                            .text_color(gpui::white())
                            .child("JSON")
                    )
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .text_size(px(12.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Raw")
                    )
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .text_size(px(12.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Form")
                    )
            )
            // Body editor
            .child(
                div()
                    .id("body-editor")
                    .flex_1()
                    .w_full()
                    .p(px(12.0))
                    .rounded(px(4.0))
                    .border_1()
                    .cursor_text()
                    .track_focus(&self.edit_focus)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        this.handle_edit_key(event, cx);
                    }))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            this.start_editing(EditTarget::Body, window, cx);
                            this.handle_edit_mouse_down(event, 12.0, 7.2, cx);
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                        this.handle_edit_mouse_move(event, 7.2, cx);
                    }))
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &MouseUpEvent, _, cx| {
                            this.handle_edit_mouse_up(event, cx);
                        }),
                    )
                    .when(is_editing_body, |el| el.border_color(theme.colors.accent))
                    .when(!is_editing_body, |el| el.border_color(theme.colors.border))
                    .bg(theme.colors.bg_tertiary)
                    .child(self.render_body_text(&body, is_editing_body, edit_selection, cx))
            )
            .into_any_element()
    }

    /// Render text with cursor/selection for body editor
    fn render_body_text(
        &self,
        text: &str,
        is_focused: bool,
        selection: Range<usize>,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        // Use shared render helper, wrapped in monospace font
        div()
            .font_family("monospace")
            .child(render_text_view(
                text,
                &selection,
                is_focused,
                12.0,
                theme.colors.text_primary,
                Some("Enter request body..."),
                theme.colors.text_muted,
            ))
            .into_any_element()
    }

    /// Render a key-value input field with fixed width
    fn render_kv_input(
        &mut self,
        id: String,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        width: gpui::Pixels,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(SharedString::from(id))
            .w(width)
            .h(px(28.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .rounded(px(4.0))
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
            .bg(theme.colors.bg_tertiary)
            .text_size(px(12.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, 8.0, 7.2, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 7.2, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_edit_mouse_up(event, cx);
                }),
            )
            .child(self.render_kv_text(&text, placeholder, is_editing, selection, cx))
    }

    /// Render a key-value input field with flex width
    fn render_kv_input_flex(
        &mut self,
        id: String,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(SharedString::from(id))
            .flex_1()
            .h(px(28.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .rounded(px(4.0))
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
            .bg(theme.colors.bg_tertiary)
            .text_size(px(12.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, 8.0, 7.2, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 7.2, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_edit_mouse_up(event, cx);
                }),
            )
            .child(self.render_kv_text(&text, placeholder, is_editing, selection, cx))
    }

    /// Render text with cursor/selection for kv inputs
    fn render_kv_text(
        &self,
        text: &str,
        placeholder: &'static str,
        is_focused: bool,
        selection: Range<usize>,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        render_text_view(
            text,
            &selection,
            is_focused,
            12.0,
            theme.colors.text_primary,
            Some(placeholder),
            theme.colors.text_muted,
        )
    }

    fn render_auth_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let auth_type = self.auth_type;
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();

        let auth_types = [
            (AuthType::None, "None"),
            (AuthType::Bearer, "Bearer Token"),
            (AuthType::Basic, "Basic Auth"),
            (AuthType::ApiKey, "API Key"),
        ];

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .track_focus(&self.edit_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_edit_key(event, cx);
            }))
            // Auth type selector
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .children(auth_types.iter().map(|(at, label)| {
                        let is_selected = *at == auth_type;
                        let at = *at;
                        div()
                            .id(SharedString::from(format!("auth-type-{:?}", at)))
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .text_size(px(12.0))
                            .when(is_selected, |el| {
                                el.bg(theme.colors.bg_tertiary)
                                    .border_1()
                                    .border_color(theme.colors.accent)
                                    .text_color(theme.colors.text_primary)
                            })
                            .when(!is_selected, |el| {
                                el.text_color(theme.colors.text_secondary)
                                    .hover(|s| s.text_color(theme.colors.text_primary))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_auth_type(at, cx);
                            }))
                            .child(*label)
                    }))
            )
            // Auth type specific content
            .child(self.render_auth_content(auth_type, active_edit, edit_selection, cx))
            .into_any_element()
    }

    fn render_auth_content(
        &mut self,
        auth_type: AuthType,
        active_edit: Option<EditTarget>,
        edit_selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);

        match auth_type {
            AuthType::None => {
                div()
                    .pt(px(8.0))
                    .text_size(px(12.0))
                    .text_color(theme.colors.text_muted)
                    .child("No authentication. The request will be sent without any auth headers.")
                    .into_any_element()
            }
            AuthType::Bearer => {
                let token = self.bearer_token.clone();
                let is_editing = active_edit == Some(EditTarget::BearerToken);

                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Token")
                    )
                    .child(
                        self.render_auth_input(
                            "bearer-token",
                            EditTarget::BearerToken,
                            &token,
                            "Enter bearer token...",
                            is_editing,
                            if is_editing { edit_selection } else { 0..0 },
                            cx,
                        )
                    )
                    .child(
                        div()
                            .pt(px(4.0))
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child("The token will be sent as: Authorization: Bearer <token>")
                    )
                    .into_any_element()
            }
            AuthType::Basic => {
                let username = self.basic_username.clone();
                let password = self.basic_password.clone();
                let is_editing_user = active_edit == Some(EditTarget::BasicUsername);
                let is_editing_pass = active_edit == Some(EditTarget::BasicPassword);

                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    // Username field
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child("Username")
                            )
                            .child(
                                self.render_auth_input(
                                    "basic-username",
                                    EditTarget::BasicUsername,
                                    &username,
                                    "Enter username...",
                                    is_editing_user,
                                    if is_editing_user { edit_selection.clone() } else { 0..0 },
                                    cx,
                                )
                            )
                    )
                    // Password field
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child("Password")
                            )
                            .child(
                                self.render_auth_input(
                                    "basic-password",
                                    EditTarget::BasicPassword,
                                    &password,
                                    "Enter password...",
                                    is_editing_pass,
                                    if is_editing_pass { edit_selection } else { 0..0 },
                                    cx,
                                )
                            )
                    )
                    .child(
                        div()
                            .pt(px(4.0))
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .child("Credentials will be Base64 encoded and sent as: Authorization: Basic <encoded>")
                    )
                    .into_any_element()
            }
            AuthType::ApiKey => {
                let key_name = self.api_key_name.clone();
                let key_value = self.api_key_value.clone();
                let location = self.api_key_location;
                let is_editing_name = active_edit == Some(EditTarget::ApiKeyName);
                let is_editing_value = active_edit == Some(EditTarget::ApiKeyValue);

                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    // Key name field
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child("Key Name")
                            )
                            .child(
                                self.render_auth_input(
                                    "api-key-name",
                                    EditTarget::ApiKeyName,
                                    &key_name,
                                    "e.g., X-API-Key",
                                    is_editing_name,
                                    if is_editing_name { edit_selection.clone() } else { 0..0 },
                                    cx,
                                )
                            )
                    )
                    // Key value field
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child("Key Value")
                            )
                            .child(
                                self.render_auth_input(
                                    "api-key-value",
                                    EditTarget::ApiKeyValue,
                                    &key_value,
                                    "Enter API key value...",
                                    is_editing_value,
                                    if is_editing_value { edit_selection } else { 0..0 },
                                    cx,
                                )
                            )
                    )
                    // Location selector
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child("Add to")
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .id("api-key-header")
                                            .px(px(12.0))
                                            .py(px(6.0))
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .text_size(px(12.0))
                                            .when(location == ApiKeyLocation::Header, |el| {
                                                el.bg(theme.colors.bg_tertiary)
                                                    .border_1()
                                                    .border_color(theme.colors.accent)
                                                    .text_color(theme.colors.text_primary)
                                            })
                                            .when(location != ApiKeyLocation::Header, |el| {
                                                el.text_color(theme.colors.text_secondary)
                                                    .hover(|s| s.text_color(theme.colors.text_primary))
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if this.api_key_location != ApiKeyLocation::Header {
                                                    this.toggle_api_key_location(cx);
                                                }
                                            }))
                                            .child("Header")
                                    )
                                    .child(
                                        div()
                                            .id("api-key-query")
                                            .px(px(12.0))
                                            .py(px(6.0))
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .text_size(px(12.0))
                                            .when(location == ApiKeyLocation::QueryParam, |el| {
                                                el.bg(theme.colors.bg_tertiary)
                                                    .border_1()
                                                    .border_color(theme.colors.accent)
                                                    .text_color(theme.colors.text_primary)
                                            })
                                            .when(location != ApiKeyLocation::QueryParam, |el| {
                                                el.text_color(theme.colors.text_secondary)
                                                    .hover(|s| s.text_color(theme.colors.text_primary))
                                            })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if this.api_key_location != ApiKeyLocation::QueryParam {
                                                    this.toggle_api_key_location(cx);
                                                }
                                            }))
                                            .child("Query Param")
                                    )
                            )
                    )
                    .into_any_element()
            }
        }
    }

    fn render_auth_input(
        &mut self,
        id: &str,
        target: EditTarget,
        text: &str,
        placeholder: &'static str,
        is_editing: bool,
        selection: Range<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let text = text.to_string();

        div()
            .id(SharedString::from(id.to_string()))
            .w_full()
            .max_w(px(400.0))
            .h(px(32.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .rounded(px(4.0))
            .border_1()
            .cursor_text()
            .when(is_editing, |el| el.border_color(theme.colors.accent))
            .when(!is_editing, |el| el.border_color(theme.colors.border))
            .bg(theme.colors.bg_tertiary)
            .text_size(px(12.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.start_editing(target, window, cx);
                    this.handle_edit_mouse_down(event, 12.0, 7.2, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_edit_mouse_move(event, 7.2, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_edit_mouse_up(event, cx);
                }),
            )
            .child(self.render_kv_text(&text, placeholder, is_editing, selection, cx))
    }
}

fn status_text(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
}

/// Simple URL encoding for query params
fn url_encode(input: &str) -> String {
    let mut result = String::new();
    for c in input.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            ' ' => result.push('+'),
            _ => {
                for b in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    result
}

/// Simple URL decoding for query params
fn url_decode(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '+' => result.push(' '),
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if hex.len() == 2 {
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        result.push(byte as char);
                    } else {
                        result.push('%');
                        result.push_str(&hex);
                    }
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            }
            _ => result.push(c),
        }
    }
    result
}

/// Simple base64 encoding for Basic auth
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Unit Tests for HTTP Methods =====

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Put.as_str(), "PUT");
        assert_eq!(HttpMethod::Patch.as_str(), "PATCH");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
    }

    #[test]
    fn test_http_method_from_str() {
        assert_eq!(HttpMethod::from_str("GET"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("get"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("POST"), Some(HttpMethod::Post));
        assert_eq!(HttpMethod::from_str("PUT"), Some(HttpMethod::Put));
        assert_eq!(HttpMethod::from_str("PATCH"), Some(HttpMethod::Patch));
        assert_eq!(HttpMethod::from_str("DELETE"), Some(HttpMethod::Delete));
        assert_eq!(HttpMethod::from_str("INVALID"), None);
    }

    #[test]
    fn test_http_method_all() {
        let methods = HttpMethod::all();
        assert_eq!(methods.len(), 5);
        assert!(methods.contains(&HttpMethod::Get));
        assert!(methods.contains(&HttpMethod::Post));
        assert!(methods.contains(&HttpMethod::Put));
        assert!(methods.contains(&HttpMethod::Patch));
        assert!(methods.contains(&HttpMethod::Delete));
    }

    // ===== Unit Tests for Word Boundary Functions =====

    #[test]
    fn test_is_word_char() {
        assert!(is_word_char('a'));
        assert!(is_word_char('Z'));
        assert!(is_word_char('5'));
        assert!(is_word_char('_'));
        assert!(!is_word_char(' '));
        assert!(!is_word_char('.'));
        assert!(!is_word_char('/'));
        assert!(!is_word_char(':'));
    }

    #[test]
    fn test_find_word_start_simple() {
        let text = "hello world";
        assert_eq!(find_word_start(text, 0), 0);
        assert_eq!(find_word_start(text, 3), 0);
        assert_eq!(find_word_start(text, 5), 0); // end of "hello"
        assert_eq!(find_word_start(text, 6), 6); // space -> finds "world"
        assert_eq!(find_word_start(text, 8), 6); // middle of "world"
    }

    #[test]
    fn test_find_word_end_simple() {
        let text = "hello world";
        assert_eq!(find_word_end(text, 0), 5);
        assert_eq!(find_word_end(text, 3), 5);
        assert_eq!(find_word_end(text, 5), 11); // at space, skips to next word end
        assert_eq!(find_word_end(text, 6), 11);
        assert_eq!(find_word_end(text, 8), 11);
    }

    #[test]
    fn test_find_word_boundaries_url() {
        let text = "https://api.example.com/users";
        // "https" is a word
        assert_eq!(find_word_start(text, 2), 0);
        assert_eq!(find_word_end(text, 2), 5);
        // "api" is a word
        assert_eq!(find_word_start(text, 9), 8);
        assert_eq!(find_word_end(text, 9), 11);
        // "users" is a word
        assert_eq!(find_word_start(text, 27), 24);
        assert_eq!(find_word_end(text, 27), 29);
    }

    #[test]
    fn test_find_word_boundaries_empty() {
        assert_eq!(find_word_start("", 0), 0);
        assert_eq!(find_word_end("", 0), 0);
    }

    #[test]
    fn test_find_word_boundaries_single_word() {
        let text = "hello";
        assert_eq!(find_word_start(text, 0), 0);
        assert_eq!(find_word_start(text, 2), 0);
        assert_eq!(find_word_start(text, 5), 0);
        assert_eq!(find_word_end(text, 0), 5);
        assert_eq!(find_word_end(text, 2), 5);
        assert_eq!(find_word_end(text, 5), 5);
    }

    #[test]
    fn test_find_word_with_underscore() {
        let text = "hello_world test";
        // "hello_world" is treated as one word (underscore is word char)
        assert_eq!(find_word_start(text, 5), 0);
        assert_eq!(find_word_end(text, 5), 11);
    }

    // ===== Unit Tests for URL Encoding/Decoding =====
    // Note: Uses application/x-www-form-urlencoded style (+ for spaces)

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello"), "hello");
        assert_eq!(url_encode("hello world"), "hello+world"); // + for spaces
        assert_eq!(url_encode("key=value"), "key%3Dvalue");
        assert_eq!(url_encode("a&b"), "a%26b");
        assert_eq!(url_encode("100%"), "100%25");
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("hello"), "hello");
        assert_eq!(url_decode("hello+world"), "hello world"); // + decoded to space
        assert_eq!(url_decode("hello%20world"), "hello world"); // %20 also works
        assert_eq!(url_decode("key%3Dvalue"), "key=value");
        assert_eq!(url_decode("a%26b"), "a&b");
        assert_eq!(url_decode("100%25"), "100%");
    }

    #[test]
    fn test_url_encode_decode_roundtrip() {
        let test_cases = vec![
            "simple",
            "with spaces",
            "special=chars&here",
            "numbers123",
        ];
        for original in test_cases {
            let encoded = url_encode(original);
            let decoded = url_decode(&encoded);
            assert_eq!(decoded, original, "Roundtrip failed for: {}", original);
        }
    }

    #[test]
    fn test_url_encode_special_chars() {
        assert_eq!(url_encode("?"), "%3F");
        assert_eq!(url_encode("#"), "%23");
        assert_eq!(url_encode("/"), "%2F");
        assert_eq!(url_encode(":"), "%3A");
        assert_eq!(url_encode("+"), "%2B");
    }

    #[test]
    fn test_url_decode_invalid() {
        // Incomplete percent encoding should be handled gracefully
        assert_eq!(url_decode("%"), "%");
        assert_eq!(url_decode("%2"), "%2");
        assert_eq!(url_decode("%GG"), "%GG"); // Invalid hex
    }

    // ===== Unit Tests for Base64 Encoding =====

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_base64_encode_basic_auth() {
        // Test basic auth style encoding (username:password)
        assert_eq!(base64_encode(b"user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64_encode(b"admin:secret123"), "YWRtaW46c2VjcmV0MTIz");
    }

    // ===== Unit Tests for Data Types =====

    #[test]
    fn test_key_value_pair_default() {
        let pair = KeyValuePair::default();
        assert_eq!(pair.key, "");
        assert_eq!(pair.value, "");
        assert!(!pair.enabled);
    }

    #[test]
    fn test_key_value_pair_creation() {
        let pair = KeyValuePair {
            key: "Content-Type".to_string(),
            value: "application/json".to_string(),
            enabled: true,
        };
        assert_eq!(pair.key, "Content-Type");
        assert_eq!(pair.value, "application/json");
        assert!(pair.enabled);
    }

    #[test]
    fn test_auth_type_default() {
        let auth = AuthType::default();
        assert_eq!(auth, AuthType::None);
    }

    #[test]
    fn test_auth_type_variants() {
        assert_ne!(AuthType::None, AuthType::Bearer);
        assert_ne!(AuthType::Bearer, AuthType::Basic);
        assert_ne!(AuthType::Basic, AuthType::ApiKey);
    }

    #[test]
    fn test_api_key_location_default() {
        let location = ApiKeyLocation::default();
        assert_eq!(location, ApiKeyLocation::Header);
    }

    #[test]
    fn test_api_key_location_variants() {
        assert_ne!(ApiKeyLocation::Header, ApiKeyLocation::QueryParam);
    }

    // ===== Unit Tests for Edit Target =====

    #[test]
    fn test_edit_target_equality() {
        assert_eq!(EditTarget::Body, EditTarget::Body);
        assert_eq!(EditTarget::HeaderKey(0), EditTarget::HeaderKey(0));
        assert_ne!(EditTarget::HeaderKey(0), EditTarget::HeaderKey(1));
        assert_ne!(EditTarget::HeaderKey(0), EditTarget::HeaderValue(0));
    }

    #[test]
    fn test_edit_target_param_indices() {
        let target1 = EditTarget::ParamKey(5);
        let target2 = EditTarget::ParamValue(5);
        assert_ne!(target1, target2);

        if let EditTarget::ParamKey(idx) = target1 {
            assert_eq!(idx, 5);
        } else {
            panic!("Expected ParamKey");
        }
    }

    // ===== Integration-like Tests (testing logic without GPUI) =====

    /// Test URL query string parsing logic
    #[test]
    fn test_parse_query_string() {
        // Simulate the logic from sync_params_from_url
        let url = "https://api.example.com/users?name=john&age=30&active=true";

        let query_start = url.find('?').unwrap();
        let query_string = &url[query_start + 1..];

        let params: Vec<KeyValuePair> = query_string
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = url_decode(parts.next().unwrap_or(""));
                let value = url_decode(parts.next().unwrap_or(""));
                KeyValuePair {
                    key,
                    value,
                    enabled: true,
                }
            })
            .collect();

        assert_eq!(params.len(), 3);
        assert_eq!(params[0].key, "name");
        assert_eq!(params[0].value, "john");
        assert_eq!(params[1].key, "age");
        assert_eq!(params[1].value, "30");
        assert_eq!(params[2].key, "active");
        assert_eq!(params[2].value, "true");
    }

    /// Test URL building from params logic
    #[test]
    fn test_build_query_string() {
        // Simulate the logic from sync_url_from_params
        let base_url = "https://api.example.com/search";
        let params = vec![
            KeyValuePair {
                key: "q".to_string(),
                value: "rust programming".to_string(),
                enabled: true,
            },
            KeyValuePair {
                key: "limit".to_string(),
                value: "10".to_string(),
                enabled: true,
            },
            KeyValuePair {
                key: "debug".to_string(),
                value: "true".to_string(),
                enabled: false, // Disabled, should not appear in URL
            },
        ];

        let query_parts: Vec<String> = params
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

        let url = if query_parts.is_empty() {
            base_url.to_string()
        } else {
            format!("{}?{}", base_url, query_parts.join("&"))
        };

        assert!(url.contains("q=rust+programming")); // + for spaces
        assert!(url.contains("limit=10"));
        assert!(!url.contains("debug")); // Disabled param excluded
    }

    /// Test empty query handling
    #[test]
    fn test_empty_query_string() {
        let url = "https://api.example.com/users";
        assert!(url.find('?').is_none());
    }

    /// Test key-only params (no value)
    #[test]
    fn test_key_only_param() {
        let param = KeyValuePair {
            key: "verbose".to_string(),
            value: "".to_string(),
            enabled: true,
        };

        let encoded = if param.value.is_empty() {
            url_encode(&param.key)
        } else {
            format!("{}={}", url_encode(&param.key), url_encode(&param.value))
        };

        assert_eq!(encoded, "verbose");
    }
}
