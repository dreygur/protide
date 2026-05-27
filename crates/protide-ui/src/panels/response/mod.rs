//! Response viewer panel

use std::time::Duration;

use gpui::{
    canvas, deferred, div, prelude::*, px, uniform_list, Bounds, ClipboardItem, Context, Entity,
    IntoElement, MouseButton, MouseDownEvent, Pixels, Point, Render, ScrollHandle,
    SharedString, Styled, UniformListScrollHandle, WeakEntity, Window,
};
use gpui_component::scroll::ScrollableElement;

pub(super) const GUTTER_W: f32 = 44.0;
pub(super) const INDENT_W: f32 = 16.0;
pub(super) const CHEVRON_W: f32 = 16.0;
pub(super) const ROW_H: f32 = 20.0;
pub(super) const ROW_FONT: f32 = 12.5;
pub(super) const GUTTER_FONT: f32 = 11.0;
// Header table layout constants (response headers tab)
pub(super) const HDR_LABEL_ROW_H: f32 = 22.0; // NAME/VALUE column header: py(6)*2 + font 10px
pub(super) const HDR_ROW_H: f32 = 28.0;       // data row: py(8)*2 + font 12px
pub(super) const HDR_SPACER_W: f32 = 4.0;     // div().w(px(4.0)) spacer between columns
pub(super) const HDR_PADDING: f32 = 12.0;     // px(12.0) left padding in value column
pub(super) const HDR_CHAR_W: f32 = 7.2;       // JetBrains Mono 12px ≈ font_size × 0.6
pub(super) const JSON_CHAR_W: f32 = 7.5;      // JetBrains Mono 12.5px ≈ font_size × 0.6
// Strings longer than COLLAPSE_CHARS get a "show more" toggle in wrap mode.
pub(super) const COLLAPSE_CHARS: usize = 300;
// Responses with more rows than this fall back to uniform_list (no wrapping).
pub(super) const WRAP_MODE_MAX_ROWS: usize = 2000;

use log::{debug, warn};
use protide_core::chaining;
use protide_core::scripting::results::TestResult;
use crate::theme;
use crate::components::selectable_text::{
    selectable_text_element, selection_changed, render_selectable_json_value, SelectionRange,
};
use crate::components::icons::{
    icon, ICON_SM, ICON_MD, ICON_CLOSE, ICON_CHECK, ICON_CIRCLE_CHECK,
    ICON_ARROW_DOWN, ICON_COPY, ICON_GLOBE, ICON_CHEVRON_DOWN, ICON_CHEVRON_RIGHT,
};
use gpui_component::input::{Input, InputState};

pub mod types;
pub mod json;
pub mod render_json_row;
pub mod render_json;
pub mod render;
pub mod render_content;
pub mod render_header;
pub mod render_body;
pub mod render_html_preview;
pub mod render_headers;
pub mod render_cookies;
pub mod render_tests;
pub mod render_extract;
pub mod render_util;

pub use types::*;
pub use types::format_size;
pub use json::{PrimVal, RowKind, JsonCtxMenu, JsonRow};

/// Response viewer panel
pub struct ResponsePanel {
    /// Active tab index for body/headers/cookies/tests/extract
    pub(super) active_tab: usize,
    /// Response data (None if no request sent yet)
    pub(super) response: Option<ResponseData>,
    /// Loading state
    pub(super) loading: bool,
    /// Error message
    pub(super) error: Option<String>,
    /// Copy feedback state (shows "Copied!" briefly)
    pub(super) copy_feedback: Option<CopyFeedback>,
    /// Editor for viewing response body
    pub(super) body_viewer: Entity<InputState>,
    /// Pending (body_text, language) to apply to body_viewer on next render (needs &mut Window)
    pub(super) body_pending: Option<(String, String)>,
    /// Which sub-view is active in the Body tab: Pretty / Raw / Preview
    pub(super) body_view_mode: BodyViewMode,
    /// Unmodified response body, used by the Raw view and Preview renderer
    pub(super) raw_body: String,
    /// Pretty-printed body text (same language as body_viewer was last set to)
    pub(super) formatted_body: String,
    /// Language id of the formatted body ("json", "xml", "html", "")
    pub(super) formatted_lang: String,
    /// Parsed JSON value for tree rendering (Some when body is valid JSON)
    pub(super) json_value: Option<serde_json::Value>,
    /// Set of collapsed JSON paths (using "/" as separator, root = "")
    pub(super) json_tree_collapsed: std::collections::HashSet<String>,
    /// Flat pre-computed row list for the JSON tree (rebuilt on every collapse change)
    pub(super) json_rows: Vec<JsonRow>,
    /// Scroll position for the JSON tree uniform_list (perf mode, >2000 rows)
    pub(super) json_scroll_handle: UniformListScrollHandle,
    /// Scroll position for the JSON tree div (wrap mode, ≤2000 rows)
    pub(super) json_scroll_handle_div: ScrollHandle,
    /// Scroll handle for the response-content area (drives the custom scrollbar)
    pub(super) content_scroll_handle: ScrollHandle,
    /// Row indices (0-based) of long strings the user has expanded via "show more"
    pub(super) expanded_strings: std::collections::HashSet<usize>,
    /// Test results from script execution
    pub(super) test_results: Vec<TestResult>,
    /// JSONPath expression input for extraction
    pub(super) jsonpath_input: Entity<InputState>,
    /// Result of JSONPath extraction
    pub(super) extraction_result: Option<Result<String, String>>,
    /// Read-only editor for displaying extracted value with syntax highlighting
    pub(super) extraction_editor: Entity<InputState>,
    /// Column widths for resizable tables
    pub(super) resp_header_col1_w: f32,   // response headers: NAME column
    pub(super) cookie_col1_w: f32,        // cookies: NAME column
    pub(super) cookie_col3_w: f32,        // cookies: PATH column
    pub(super) cookie_col4_w: f32,        // cookies: FLAGS column
    /// Active column drag: (drag_id, start_x, start_width)
    /// drag_id: 0=resp_header_col1, 1=cookie_col1, 2=cookie_col3, 3=cookie_col4
    pub(super) resp_col_drag: Option<(u8, f32, f32)>,
    /// Right-click context menu position (window coords) for body-level copy menu
    pub(super) context_menu_pos: Option<gpui::Point<gpui::Pixels>>,
    /// Window-space origin of this panel, captured each frame for JSON context-menu positioning
    pub(super) bounds_origin: Point<Pixels>,
    /// Active JSON tree right-click context menu
    pub(super) json_context_menu: Option<JsonCtxMenu>,
    /// Layout bounds of the response headers table, captured each frame for hit-testing
    pub(super) hdr_table_bounds: Option<Bounds<Pixels>>,
    /// Active text selection within the header value column
    pub(super) hdr_sel: Option<HdrSel>,
    /// Bounds of the JSON tree container, captured each frame for mouse hit-testing
    pub(super) json_tree_bounds: Option<Bounds<Pixels>>,
    /// Active drag-select across JSON tree value cells
    pub(super) json_sel: Option<SelectionRange>,
    /// Whether a JSON tree selection drag is in progress
    pub(super) json_selecting: bool,
}

impl ResponsePanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let body_viewer = cx.new(|cx| {
            InputState::new(window, cx).code_editor("")
        });
        let jsonpath_input = cx.new(|cx| {
            InputState::new(window, cx)
        });
        let extraction_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true)
        });
        Self {
            active_tab: 0,
            response: None,
            loading: false,
            error: None,
            copy_feedback: None,
            body_viewer,
            body_pending: None,
            body_view_mode: BodyViewMode::Pretty,
            raw_body: String::new(),
            formatted_body: String::new(),
            formatted_lang: String::new(),
            json_value: None,
            json_tree_collapsed: std::collections::HashSet::new(),
            json_rows: Vec::new(),
            json_scroll_handle: UniformListScrollHandle::new(),
            json_scroll_handle_div: ScrollHandle::new(),
            content_scroll_handle: ScrollHandle::new(),
            expanded_strings: std::collections::HashSet::new(),
            test_results: Vec::new(),
            jsonpath_input,
            extraction_result: None,
            extraction_editor,
            resp_header_col1_w: 180.0,
            cookie_col1_w: 150.0,
            cookie_col3_w: 100.0,
            cookie_col4_w: 80.0,
            resp_col_drag: None,
            context_menu_pos: None,
            bounds_origin: Point::default(),
            json_context_menu: None,
            hdr_table_bounds: None,
            hdr_sel: None,
            json_tree_bounds: None,
            json_sel: None,
            json_selecting: false,
        }
    }

    /// Set test results from script execution
    pub fn set_test_results(&mut self, results: Vec<TestResult>, cx: &mut Context<Self>) {
        self.test_results = results;
        cx.notify();
    }

    pub(super) fn hdr_row_at(&self, ey: Pixels) -> Option<usize> {
        let bounds = self.hdr_table_bounds?;
        let rel_y = f32::from(ey) - f32::from(bounds.origin.y) - HDR_LABEL_ROW_H;
        if rel_y < 0.0 { return None; }
        let row = (rel_y / HDR_ROW_H) as usize;
        let n = self.response.as_ref()?.headers.len();
        (row < n).then_some(row)
    }

    pub(super) fn hdr_val_byte_at(&self, ex: Pixels, row: usize) -> usize {
        let bounds = self.hdr_table_bounds.unwrap_or_default();
        let val_col_x = f32::from(bounds.origin.x) + self.resp_header_col1_w + HDR_SPACER_W + HDR_PADDING;
        let char_x = (f32::from(ex) - val_col_x).max(0.0);
        let char_idx = (char_x / HDR_CHAR_W) as usize;
        let val = self.response.as_ref()
            .and_then(|r| r.headers.get(row))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        val.char_indices()
            .nth(char_idx)
            .map(|(byte_pos, _)| byte_pos)
            .unwrap_or(val.len())
    }

    pub(super) fn show_copy_feedback(&mut self, feedback: CopyFeedback, cx: &mut Context<Self>) {
        self.copy_feedback = Some(feedback);
        cx.notify();

        // Clear feedback after 1.5 seconds
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(1500)).await;
            this.update(cx, |this, cx| {
                this.copy_feedback = None;
                cx.notify();
            }).ok();
        }).detach();
    }

    pub fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        self.error = None;
        cx.notify();
    }

    pub fn set_response(&mut self, response: ResponseData, cx: &mut Context<Self>) {
        debug!("Response: {} {} ({} bytes, {:?})", response.status, response.status_text, response.body.len(), response.time);
        let content_type = response.headers.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.to_lowercase());

        let (body_text, language) = if let Some(ct) = &content_type {
            if ct.contains("application/json") || ct.contains("+json") {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response.body) {
                    let formatted = serde_json::to_string_pretty(&json).unwrap_or_else(|_| response.body.clone());
                    (formatted, "json".to_string())
                } else {
                    warn!("JSON parse failed for '{}' response ({} bytes)", ct, response.body.len());
                    (response.body.clone(), "json".to_string())
                }
            } else if ct.contains("text/html") {
                (response.body.clone(), "html".to_string())
            } else if ct.contains("application/xml") || ct.contains("text/xml") || ct.contains("+xml") {
                (types::pretty_xml(&response.body), "xml".to_string())
            } else {
                self.detect_language_from_content(&response.body)
            }
        } else {
            self.detect_language_from_content(&response.body)
        };

        // Auto-switch to Preview when the response is HTML
        self.body_view_mode = if language == "html" {
            BodyViewMode::Preview
        } else {
            BodyViewMode::Pretty
        };

        self.raw_body = response.body.clone();
        self.formatted_body = body_text.clone();
        self.formatted_lang = language.clone();

        self.body_pending = Some((body_text, language));

        self.json_value = serde_json::from_str::<serde_json::Value>(&response.body).ok();
        self.json_tree_collapsed.clear();
        self.expanded_strings.clear();
        self.json_sel = None;
        self.json_selecting = false;
        self.rebuild_json_rows();

        self.response = Some(response);
        self.loading = false;
        self.error = None;
        cx.notify();
    }

    fn detect_language_from_content(&self, body: &str) -> (String, String) {
        let trimmed = body.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                let formatted = serde_json::to_string_pretty(&json).unwrap_or_else(|_| body.to_string());
                (formatted, "json".to_string())
            } else {
                (body.to_string(), String::new())
            }
        } else if trimmed.starts_with('<') {
            if trimmed.contains("<!DOCTYPE html") || trimmed.contains("<html") {
                (body.to_string(), "html".to_string())
            } else {
                (types::pretty_xml(body), "xml".to_string())
            }
        } else {
            (body.to_string(), String::new())
        }
    }

    /// Returns (status, status_text, time_ms, size_bytes) for the status bar, if any response received.
    pub fn last_response_summary(&self) -> Option<(u16, &str, u64, usize)> {
        self.response.as_ref().map(|r| (r.status, r.status_text.as_str(), r.time.as_millis() as u64, r.size))
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Returns the current error message, if the last request failed.
    pub fn last_error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn set_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.loading = false;
        self.error = Some(error);
        cx.notify();
    }

    pub(super) fn set_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        cx.notify();
    }
}


#[cfg(test)]
mod tests_gpui;

#[cfg(test)]
mod tests {
    use super::*;

    // ResponseData tests
    #[test]
    fn test_response_data_default() {
        let data = ResponseData::default();
        assert_eq!(data.status, 0);
        assert!(data.status_text.is_empty());
        assert!(data.headers.is_empty());
        assert!(data.body.is_empty());
        assert_eq!(data.size, 0);
    }

    #[test]
    fn test_response_is_success_200() {
        let data = ResponseData {
            status: 200,
            ..Default::default()
        };
        assert!(data.is_success());
        assert!(!data.is_error());
    }

    #[test]
    fn test_response_is_success_201() {
        let data = ResponseData {
            status: 201,
            ..Default::default()
        };
        assert!(data.is_success());
        assert!(!data.is_error());
    }

    #[test]
    fn test_response_is_success_299() {
        let data = ResponseData {
            status: 299,
            ..Default::default()
        };
        assert!(data.is_success());
        assert!(!data.is_error());
    }

    #[test]
    fn test_response_redirect_not_success() {
        let data = ResponseData {
            status: 301,
            ..Default::default()
        };
        assert!(!data.is_success());
        assert!(!data.is_error());
    }

    #[test]
    fn test_response_is_error_400() {
        let data = ResponseData {
            status: 400,
            ..Default::default()
        };
        assert!(!data.is_success());
        assert!(data.is_error());
    }

    #[test]
    fn test_response_is_error_404() {
        let data = ResponseData {
            status: 404,
            ..Default::default()
        };
        assert!(!data.is_success());
        assert!(data.is_error());
    }

    #[test]
    fn test_response_is_error_500() {
        let data = ResponseData {
            status: 500,
            ..Default::default()
        };
        assert!(!data.is_success());
        assert!(data.is_error());
    }

    #[test]
    fn test_response_is_error_503() {
        let data = ResponseData {
            status: 503,
            ..Default::default()
        };
        assert!(!data.is_success());
        assert!(data.is_error());
    }

    // truncate_error tests
    #[test]
    fn test_truncate_error_multibyte() {
        // 41 kanji × 3 bytes each = 123 bytes, 41 chars — exceeds 40-char threshold
        // Old byte-slicing at index 37 would land mid-character and panic
        let kanji = "日".repeat(41);
        let result = types::truncate_error(&kanji); // must not panic
        assert!(result.ends_with("..."));
        // Verify result is valid UTF-8 (no split multi-byte sequences)
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
    }

    #[test]
    fn test_truncate_error_short_stays_intact() {
        let short = "connection refused";
        assert_eq!(types::truncate_error(short), "connection refused");
    }

    #[test]
    fn test_truncate_error_exactly_40_chars() {
        let s = "a".repeat(40);
        assert_eq!(types::truncate_error(&s), s); // not truncated
    }

    #[test]
    fn test_truncate_error_41_chars() {
        let s = "a".repeat(41);
        let result = types::truncate_error(&s);
        assert!(result.ends_with("..."));
        assert!(result.len() < 41 + 3); // shorter than original
    }

    // format_size tests
    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1), "1 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(10240), "10.0 KB");
        assert_eq!(format_size(1024 * 1024 - 1), "1024.0 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 2), "2.0 MB");
        assert_eq!(format_size(1024 * 1024 + 512 * 1024), "1.5 MB");
    }
}
