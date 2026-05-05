//! Response viewer panel

use std::time::Duration;

use gpui::{
    deferred, div, prelude::*, px, Context, Entity, IntoElement, MouseButton, ParentElement,
    Render, SharedString, Styled, Window,
};

use protide_core::chaining;
use protide_core::scripting::results::TestResult;
use crate::theme;
use crate::ui::components::code_editor::{CodeEditor, Language};
use crate::ui::components::icons::{
    icon, ICON_SM, ICON_MD, ICON_CLOSE, ICON_CHECK, ICON_CIRCLE_CHECK,
    ICON_ARROW_DOWN, ICON_COPY, ICON_GLOBE, ICON_CHEVRON_DOWN, ICON_CHEVRON_RIGHT,
};
use crate::ui::components::TextInput;

/// Response data from an HTTP request
#[derive(Clone, Default)]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub time: Duration,
    pub size: usize,
}

impl ResponseData {
    #[allow(dead_code)]
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    #[allow(dead_code)]
    pub fn is_error(&self) -> bool {
        self.status >= 400
    }
}

/// Parsed cookie from Set-Cookie header
#[derive(Clone, Debug)]
pub struct ParsedCookie {
    pub name: String,
    pub value: String,
    pub path: Option<String>,
    pub domain: Option<String>,
    pub expires: Option<String>,
    pub secure: bool,
    pub http_only: bool,
}

impl ParsedCookie {
    /// Parse a Set-Cookie header value
    pub fn parse(header_value: &str) -> Option<Self> {
        let mut parts = header_value.split(';');
        let name_value = parts.next()?.trim();
        let mut split = name_value.splitn(2, '=');
        let name = split.next()?.trim().to_string();
        let value = split.next().unwrap_or("").trim().to_string();

        if name.is_empty() {
            return None;
        }

        let mut cookie = ParsedCookie {
            name,
            value,
            path: None,
            domain: None,
            expires: None,
            secure: false,
            http_only: false,
        };

        for part in parts {
            let part = part.trim();
            let lower = part.to_lowercase();
            if lower == "secure" {
                cookie.secure = true;
            } else if lower == "httponly" {
                cookie.http_only = true;
            } else if let Some(val) = part.strip_prefix("Path=").or_else(|| part.strip_prefix("path=")) {
                cookie.path = Some(val.to_string());
            } else if let Some(val) = part.strip_prefix("Domain=").or_else(|| part.strip_prefix("domain=")) {
                cookie.domain = Some(val.to_string());
            } else if let Some(val) = part.strip_prefix("Expires=").or_else(|| part.strip_prefix("expires=")) {
                cookie.expires = Some(val.to_string());
            }
        }
        Some(cookie)
    }
}

/// Copy feedback type
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum CopyFeedback {
    Body,
    Headers,
    Cookies,
}

/// Response viewer panel
pub struct ResponsePanel {
    /// Active tab index for body/headers/cookies/tests/extract
    active_tab: usize,
    /// Response data (None if no request sent yet)
    response: Option<ResponseData>,
    /// Loading state
    loading: bool,
    /// Error message
    error: Option<String>,
    /// Copy feedback state (shows "Copied!" briefly)
    copy_feedback: Option<CopyFeedback>,
    /// CodeEditor for viewing response body
    body_viewer: Entity<CodeEditor>,
    /// Parsed JSON value for tree rendering (Some when body is valid JSON)
    json_value: Option<serde_json::Value>,
    /// Set of collapsed JSON paths (using "/" as separator, root = "")
    json_tree_collapsed: std::collections::HashSet<String>,
    /// Test results from script execution
    test_results: Vec<TestResult>,
    /// JSONPath expression input for extraction
    jsonpath_input: Entity<TextInput>,
    /// Result of JSONPath extraction
    extraction_result: Option<Result<String, String>>,
    /// Read-only editor for displaying extracted value with syntax highlighting
    extraction_editor: Entity<CodeEditor>,
    /// Column widths for resizable tables
    resp_header_col1_w: f32,   // response headers: NAME column
    cookie_col1_w: f32,        // cookies: NAME column
    cookie_col3_w: f32,        // cookies: PATH column
    cookie_col4_w: f32,        // cookies: FLAGS column
    /// Active column drag: (drag_id, start_x, start_width)
    /// drag_id: 0=resp_header_col1, 1=cookie_col1, 2=cookie_col3, 3=cookie_col4
    resp_col_drag: Option<(u8, f32, f32)>,
}

impl ResponsePanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let body_viewer = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_read_only(true)
                .with_line_numbers(true)
        });
        let jsonpath_input = cx.new(|cx| {
            TextInput::new(cx, "$.data.id")
        });
        let extraction_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_read_only(true)
                .with_line_numbers(false)
        });
        Self {
            active_tab: 0,
            response: None,
            loading: false,
            error: None,
            copy_feedback: None,
            body_viewer,
            json_value: None,
            json_tree_collapsed: std::collections::HashSet::new(),
            test_results: Vec::new(),
            jsonpath_input,
            extraction_result: None,
            extraction_editor,
            resp_header_col1_w: 180.0,
            cookie_col1_w: 150.0,
            cookie_col3_w: 100.0,
            cookie_col4_w: 80.0,
            resp_col_drag: None,
        }
    }

    /// Set test results from script execution
    pub fn set_test_results(&mut self, results: Vec<TestResult>, cx: &mut Context<Self>) {
        self.test_results = results;
        cx.notify();
    }

    fn show_copy_feedback(&mut self, feedback: CopyFeedback, cx: &mut Context<Self>) {
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
        // Detect language from Content-Type header
        let content_type = response.headers.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.to_lowercase());

        let (body_text, language) = if let Some(ct) = &content_type {
            if ct.contains("application/json") || ct.contains("+json") {
                // Try to pretty-print JSON
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response.body) {
                    let formatted = serde_json::to_string_pretty(&json).unwrap_or_else(|_| response.body.clone());
                    (formatted, Language::Json)
                } else {
                    (response.body.clone(), Language::Json)
                }
            } else if ct.contains("text/html") {
                (response.body.clone(), Language::Html)
            } else if ct.contains("application/xml") || ct.contains("text/xml") || ct.contains("+xml") {
                (response.body.clone(), Language::Xml)
            } else {
                // Detect from content
                self.detect_language_from_content(&response.body)
            }
        } else {
            // No Content-Type - detect from content
            self.detect_language_from_content(&response.body)
        };

        // Update body viewer with content and language
        self.body_viewer.update(cx, |editor, cx| {
            editor.set_content(&body_text, cx);
            editor.set_language(language, cx);
        });

        // Store parsed JSON for tree rendering
        self.json_value = serde_json::from_str::<serde_json::Value>(&response.body).ok();
        self.json_tree_collapsed.clear();

        self.response = Some(response);
        self.loading = false;
        self.error = None;
        cx.notify();
    }

    fn detect_language_from_content(&self, body: &str) -> (String, Language) {
        let trimmed = body.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                let formatted = serde_json::to_string_pretty(&json).unwrap_or_else(|_| body.to_string());
                (formatted, Language::Json)
            } else {
                (body.to_string(), Language::Plain)
            }
        } else if trimmed.starts_with('<') {
            if trimmed.contains("<!DOCTYPE html") || trimmed.contains("<html") {
                (body.to_string(), Language::Html)
            } else {
                (body.to_string(), Language::Xml)
            }
        } else {
            (body.to_string(), Language::Plain)
        }
    }

    fn toggle_json_collapse(&mut self, path: String, cx: &mut Context<Self>) {
        if self.json_tree_collapsed.contains(&path) {
            self.json_tree_collapsed.remove(&path);
        } else {
            self.json_tree_collapsed.insert(path);
        }
        cx.notify();
    }

    fn render_json_node(
        &self,
        value: &serde_json::Value,
        path: String,
        depth: usize,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let indent = (depth * 16) as f32;
        let is_collapsed = self.json_tree_collapsed.contains(&path);

        match value {
            serde_json::Value::Null => div()
                .pl(px(indent))
                .text_color(theme.colors.text_muted)
                .child("null")
                .into_any_element(),

            serde_json::Value::Bool(b) => div()
                .pl(px(indent))
                .text_color(theme.colors.method_delete)
                .child(if *b { "true" } else { "false" })
                .into_any_element(),

            serde_json::Value::Number(n) => div()
                .pl(px(indent))
                .text_color(theme.colors.method_put)
                .child(n.to_string())
                .into_any_element(),

            serde_json::Value::String(s) => div()
                .pl(px(indent))
                .text_color(theme.colors.status_success)
                .child(format!("\"{}\"", s.replace('\"', "\\\"")))
                .into_any_element(),

            serde_json::Value::Array(arr) => {
                let count = arr.len();
                if count == 0 {
                    return div()
                        .pl(px(indent))
                        .text_color(theme.colors.text_muted)
                        .child("[]")
                        .into_any_element();
                }
                let path_clone = path.clone();
                if is_collapsed {
                    return div()
                        .id(SharedString::from(format!("json-arr-{}", path)))
                        .pl(px(indent))
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_json_collapse(path_clone.clone(), cx);
                        }))
                        .child(icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted))
                        .child(div().text_color(theme.colors.text_secondary)
                            .child(format!("[ {} items ]", count)))
                        .into_any_element();
                }
                let mut container = div().flex().flex_col();
                let path_clone2 = path.clone();
                container = container.child(
                    div()
                        .id(SharedString::from(format!("json-arr-{}", path)))
                        .pl(px(indent))
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_json_collapse(path_clone2.clone(), cx);
                        }))
                        .child(icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted))
                        .child(div().text_color(theme.colors.text_muted).child("["))
                );
                for (i, item) in arr.iter().enumerate() {
                    container = container.child(
                        self.render_json_node(item, format!("{}/{}", path, i), depth + 1, cx)
                    );
                }
                container = container.child(
                    div()
                        .pl(px(indent))
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .text_color(theme.colors.text_muted)
                        .child(format!("]  ({} items)", count))
                );
                container.into_any_element()
            }

            serde_json::Value::Object(obj) => {
                let count = obj.len();
                if count == 0 {
                    return div()
                        .pl(px(indent))
                        .text_color(theme.colors.text_muted)
                        .child("{}")
                        .into_any_element();
                }
                let path_clone = path.clone();
                if is_collapsed {
                    return div()
                        .id(SharedString::from(format!("json-obj-{}", path)))
                        .pl(px(indent))
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_json_collapse(path_clone.clone(), cx);
                        }))
                        .child(icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted))
                        .child(div().text_color(theme.colors.text_secondary)
                            .child(format!("{{ {} keys }}", count)))
                        .into_any_element();
                }
                let mut container = div().flex().flex_col();
                let path_clone2 = path.clone();
                container = container.child(
                    div()
                        .id(SharedString::from(format!("json-obj-{}", path)))
                        .pl(px(indent))
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_json_collapse(path_clone2.clone(), cx);
                        }))
                        .child(icon(ICON_CHEVRON_DOWN, ICON_SM, theme.colors.text_muted))
                        .child(div().text_color(theme.colors.text_muted).child("{"))
                );
                for (key, val) in obj {
                    let child_path = format!("{}/{}", path, key);
                    if val.is_object() || val.is_array() {
                        container = container.child(
                            div()
                                .pl(px(indent + 16.0))
                                .flex()
                                .gap(px(4.0))
                                .child(div().text_color(theme.colors.accent)
                                    .child(format!("\"{}\":", key)))
                        );
                        container = container.child(
                            self.render_json_node(val, child_path, depth + 2, cx)
                        );
                    } else {
                        container = container.child(
                            div()
                                .pl(px(indent + 16.0))
                                .flex()
                                .gap(px(4.0))
                                .child(div().text_color(theme.colors.accent)
                                    .child(format!("\"{}\":", key)))
                                .child(self.render_json_node(val, child_path, depth + 2, cx))
                        );
                    }
                }
                container = container.child(
                    div()
                        .pl(px(indent))
                        .text_color(theme.colors.text_muted)
                        .child("}")
                );
                container.into_any_element()
            }
        }
    }

    /// Returns (status, status_text, time_ms, size_bytes) for the status bar, if any response received.
    pub fn last_response_summary(&self) -> Option<(u16, &str, u64, usize)> {
        self.response.as_ref().map(|r| (r.status, r.status_text.as_str(), r.time.as_millis() as u64, r.size))
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn set_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.loading = false;
        self.error = Some(error);
        cx.notify();
    }

    fn set_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        cx.notify();
    }
}

impl Render for ResponsePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_col_dragging = self.resp_col_drag.is_some();

        div()
            .id("response-panel-root")
            .size_full()
            .flex()
            .flex_col()
            .relative()
            .bg(theme.colors.bg_primary)
            .child(self.render_header(cx))
            .child(self.render_tabs(cx))
            .child(
                div()
                    .id("response-content")
                    .flex_1()
                    .w_full()
                    .p(px(12.0))
                    .overflow_scroll()
                    .child(self.render_content(cx)),
            )
            // Column resize overlay
            .when(is_col_dragging, |el| el.child(
                deferred(
                    div()
                        .id("resp-col-resize-overlay")
                        .absolute().top_0().left_0().w_full().h_full()
                        .cursor_col_resize()
                        .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                            if let Some((drag_id, start_x, start_w)) = this.resp_col_drag {
                                let delta = f32::from(event.position.x) - start_x;
                                let new_w = (start_w + delta).max(60.0);
                                match drag_id {
                                    0 => this.resp_header_col1_w = new_w.min(600.0),
                                    1 => this.cookie_col1_w = new_w.min(400.0),
                                    2 => this.cookie_col3_w = new_w.min(300.0),
                                    3 => this.cookie_col4_w = new_w.min(200.0),
                                    _ => {}
                                }
                                cx.notify();
                            }
                        }))
                        .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.resp_col_drag = None;
                            cx.notify();
                        }))
                ).with_priority(2)
            ))
    }
}

impl ResponsePanel {
    fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.0))
            .border_b_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_secondary)
            // Title with icon
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(icon(ICON_ARROW_DOWN, ICON_SM, theme.colors.text_muted))
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Response")
                    )
            )
            // Status info
            .child(self.render_status_info(cx))
    }

    fn render_status_info(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        if self.loading {
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .size(px(14.0))
                        
                        .border_2()
                        .border_color(theme.colors.accent.opacity(0.5))
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child("Sending...")
                )
        } else if let Some(error) = &self.error {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(10.0))
                .py(px(4.0))
                .bg(theme.colors.status_client_error.opacity(0.1))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .child(icon(ICON_CLOSE, ICON_SM, theme.colors.status_client_error))
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.colors.status_client_error)
                        .child(truncate_error(error))
                )
        } else if let Some(response) = &self.response {
            let status_color = theme.status_color(response.status);
            let time_color = if response.status == 0 {
                theme.colors.text_muted
            } else {
                theme.status_color(response.status)
            };
            let description = status_description(response.status);

            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                // Status badge with description
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        // Status code badge
                        .child(
                            div()
                                .px(px(10.0))
                                .py(px(4.0))
                                .bg(status_color.opacity(0.12))
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(status_color)
                                        .child(format!("{} {}", response.status, response.status_text))
                                )
                        )
                        // Description text (if available)
                        .when_some(description, |el, desc| {
                            el.child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(format!("· {}", desc))
                            )
                        })
                )
                // Time
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .text_size(px(11.0))
                        .text_color(time_color)
                        .child(format!("{}ms", response.time.as_millis()))
                )
                // Size
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .text_size(px(11.0))
                        .text_color(theme.colors.text_muted)
                        .child(format_size(response.size))
                )
        } else {
            div()
        }
    }

    fn render_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let tabs = ["Body", "Headers", "Cookies", "Tests", "Extract"];
        let active_tab = self.active_tab;
        let test_count = self.test_results.len();
        let passed_count = self.test_results.iter().filter(|t| t.passed).count();

        div()
            .id("response-tabs")
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .px(px(16.0))
            .gap(px(0.0))
            .overflow_scroll()
            .border_b_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_primary)
            .children(tabs.iter().enumerate().map(|(i, tab)| {
                let is_active = i == active_tab;
                let is_tests_tab = i == 3;
                let show_badge = is_tests_tab && test_count > 0;
                let all_passed = passed_count == test_count;

                div()
                    .flex()
                    .items_center()
                    .h_full()
                    .child(
                        div()
                            .id(SharedString::from(format!("response-tab-{}", i)))
                            .px(px(12.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_secondary.opacity(0.3)))
                            .child(
                                div()
                                    .text_size(px(13.0))
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
                                    .child(*tab)
                            )
                            .when(show_badge, |el| {
                                el.child(
                                    div()
                                        .px(px(5.0))
                                        .py(px(1.0))
                                        .bg(if is_active {
                                            if all_passed {
                                                theme.colors.status_success.opacity(0.15)
                                            } else {
                                                theme.colors.status_client_error.opacity(0.15)
                                            }
                                        } else {
                                            theme.colors.bg_tertiary
                                        })
                                        .text_size(px(10.0))
                                        .text_color(if is_active {
                                            if all_passed {
                                                theme.colors.status_success
                                            } else {
                                                theme.colors.status_client_error
                                            }
                                        } else {
                                            theme.colors.text_muted
                                        })
                                        .child(format!("{}/{}", passed_count, test_count))
                                )
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_tab(i, cx);
                            }))
                    )
                    .when(i < tabs.len() - 1, |el| {
                        el.child(
                            div()
                                .h(px(16.0))
                                .w(px(1.0))
                                .bg(theme.colors.border)
                        )
                    })
            }))
    }

    fn render_content(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        if self.loading {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(12.0))
                .child(
                    div()
                        .size(px(32.0))
                        
                        .border_3()
                        .border_color(theme.colors.accent.opacity(0.4))
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.colors.text_muted)
                        .child("Sending request...")
                )
                .into_any_element();
        }

        if let Some(error) = &self.error {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(12.0))
                .child(
                    div()
                        .size(px(48.0))
                        
                        .bg(theme.colors.status_client_error.opacity(0.1))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_size(px(24.0))
                                .text_color(theme.colors.status_client_error)
                                .child("!")
                        )
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.colors.text_primary)
                        .child("Request Failed")
                )
                .child(
                    div()
                        .max_w(px(400.0))
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child(error.clone())
                )
                .into_any_element();
        }

        // Tests tab can be shown even without a response (but will be empty)
        if self.active_tab == 3 {
            return self.render_tests_tab(cx);
        }

        if let Some(response) = &self.response {
            match self.active_tab {
                0 => self.render_body_tab(response, cx),
                1 => self.render_headers_tab(response, cx),
                2 => self.render_cookies_tab(response, cx),
                4 => self.render_extract_tab(response, cx),
                _ => div().into_any_element(),
            }
        } else {
            // Empty state - engaging and helpful
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(16.0))
                // Illustration container
                .child(
                    div()
                        .size(px(80.0))
                        .bg(theme.colors.bg_tertiary)
                        .border_1()
                        .border_color(theme.colors.border.opacity(0.5))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(4.0))
                                // Animated-style arrows
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(icon(ICON_ARROW_DOWN, ICON_SM, theme.colors.text_muted.opacity(0.4)))
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(icon(ICON_ARROW_DOWN, ICON_MD, theme.colors.text_muted.opacity(0.6)))
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(icon(ICON_ARROW_DOWN, ICON_MD, theme.colors.text_muted))
                                )
                        )
                )
                // Title
                .child(
                    div()
                        .text_size(px(15.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text_primary)
                        .child("Ready to receive")
                )
                // Description
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child("Send a request to see the response here")
                )
                // Keyboard shortcut hint
                .child(
                    div()
                        .mt(px(8.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted.opacity(0.7))
                                .child("Press")
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(3.0))
                                        .bg(theme.colors.bg_elevated)
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .text_size(px(10.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme.colors.text_secondary)
                                        .child("⌘")
                                )
                                .child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(3.0))
                                        .bg(theme.colors.bg_elevated)
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .text_size(px(10.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme.colors.text_secondary)
                                        .child("↵")
                                )
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted.opacity(0.7))
                                .child("to send")
                        )
                )
                .into_any_element()
        }
    }

    fn render_body_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        // Get Content-Type header for format label
        let content_type = response.headers.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.to_lowercase());

        // Determine format label and color
        let (format_label, format_color) = if let Some(ct) = &content_type {
            if ct.contains("application/json") || ct.contains("+json") {
                ("JSON", theme.colors.status_success)
            } else if ct.contains("text/html") {
                ("HTML", theme.colors.method_patch)
            } else if ct.contains("application/xml") || ct.contains("text/xml") || ct.contains("+xml") {
                ("XML", theme.colors.method_put)
            } else if ct.contains("text/css") {
                ("CSS", theme.colors.method_delete)
            } else if ct.contains("javascript") || ct.contains("text/js") {
                ("JS", theme.colors.accent)
            } else {
                ("Text", theme.colors.text_muted)
            }
        } else {
            // Detect from content
            let trimmed = response.body.trim();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                ("JSON", theme.colors.status_success)
            } else if trimmed.starts_with('<') {
                ("XML", theme.colors.method_put)
            } else {
                ("Text", theme.colors.text_muted)
            }
        };

        if response.body.trim().is_empty() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(icon(ICON_CIRCLE_CHECK, ICON_MD, theme.colors.text_muted.opacity(0.5)))
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("Response body is empty")
                        )
                )
                .into_any_element();
        }

        let line_count = self.body_viewer.read(cx).content().lines().count();

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Toolbar row — relative so Copy button can anchor to right_0
            .child({
                let is_copied = self.copy_feedback == Some(CopyFeedback::Body);
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .relative()
                    // Left: Format badge + line count
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .bg(format_color.opacity(0.12))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(format_color)
                                    .child(format_label)
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(format!("{} lines", line_count))
                            )
                    )
                    // Right: Copy button — absolute right_0 (ml_auto unreliable in overflow_scroll)
                    .child(
                        div()
                            .id("copy-body-btn")
                            .absolute()
                            .right_0()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(10.0))
                            .py(px(5.0))
                            .text_size(px(11.0))
                            .when(is_copied, |el| el.text_color(theme.colors.status_success).border_color(theme.colors.status_success))
                            .when(!is_copied, |el| el.text_color(theme.colors.text_secondary).border_color(theme.colors.border))
                            .cursor_pointer()
                            .border_1()
                            .bg(theme.colors.bg_primary)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let content = this.body_viewer.read(cx).content().to_string();
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(content));
                                this.show_copy_feedback(CopyFeedback::Body, cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .when(is_copied, |el| el.child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success)))
                                    .when(!is_copied, |el| el.child(icon(ICON_COPY, ICON_MD, theme.colors.text_secondary)))
                            )
                            .child(if is_copied { "Copied!" } else { "Copy" })
                    )
            })
            // Body content: JSON tree if parseable, else CodeEditor
            .child(
                if let Some(json_val) = &self.json_value {
                    div()
                        .flex_1()
                        .w_full()
                        .id("json-tree-scroll")
                        .overflow_scroll()
                        .p(px(12.0))
                        .text_size(px(12.0))
                        .child(self.render_json_node(json_val, String::new(), 0, cx))
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .w_full()
                        .overflow_hidden()
                        .child(self.body_viewer.clone())
                        .into_any_element()
                }
            )
            .into_any_element()
    }

    fn render_headers_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let header_count = response.headers.len();

        if header_count == 0 {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(icon(ICON_COPY, ICON_MD, theme.colors.text_muted.opacity(0.5)))
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("No headers in response")
                        )
                )
                .into_any_element();
        }

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Header toolbar
            .child({
                let header_is_copied = self.copy_feedback == Some(CopyFeedback::Headers);
                let headers_text: String = response.headers
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n");
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .relative()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .bg(theme.colors.accent.opacity(0.12))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.accent)
                                    .child(format!("{}", header_count))
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .child("response headers")
                            )
                    )
                    // Copy headers button — absolute right_0 (ml_auto unreliable in overflow_scroll)
                    .child(
                        div()
                            .id("copy-headers-btn")
                            .absolute()
                            .right_0()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(10.0))
                            .py(px(5.0))
                            .text_size(px(11.0))
                            .when(header_is_copied, |el| el.text_color(theme.colors.status_success).border_color(theme.colors.status_success))
                            .when(!header_is_copied, |el| el.text_color(theme.colors.text_secondary).border_color(theme.colors.border))
                            .cursor_pointer()
                            .border_1()
                            .bg(theme.colors.bg_primary)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(headers_text.clone()));
                                this.show_copy_feedback(CopyFeedback::Headers, cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .when(header_is_copied, |el| el.child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success)))
                                    .when(!header_is_copied, |el| el.child(icon(ICON_COPY, ICON_MD, theme.colors.text_secondary)))
                            )
                            .child(if header_is_copied { "Copied!" } else { "Copy" })
                    )
            })
            // Headers table
            .child(
                div()
                    .w_full()
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    // Table header
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .bg(theme.colors.bg_secondary)
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .child(
                                div()
                                    .w(px(self.resp_header_col1_w))
                                    .min_w(px(60.0))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("NAME")
                            )
                            .child(self.render_col_drag_handle(0, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("VALUE")
                            )
                    )
                    // Table rows
                    .children(response.headers.iter().enumerate().map(|(i, (key, value))| {
                        let is_last = i == header_count - 1;
                        let col1_w = self.resp_header_col1_w;
                        div()
                            .w_full()
                            .flex()
                            .when(i % 2 == 0, |el| el.bg(theme.colors.bg_tertiary.opacity(0.3)))
                            .when(!is_last, |el| el.border_b_1().border_color(theme.colors.border.opacity(0.5)))
                            .child(
                                div()
                                    .w(px(col1_w))
                                    .min_w(px(60.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.accent)
                                    .overflow_hidden()
                                    .child(key.clone())
                            )
                            .child(div().w(px(4.0)))
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .text_size(px(12.0))
                                    .font_family("JetBrains Mono")
                                    .text_color(theme.colors.text_primary)
                                    .overflow_hidden()
                                    .child(value.clone())
                            )
                    }))
            )
            .into_any_element()
    }

    fn render_cookies_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        // Parse cookies from Set-Cookie headers
        let cookies: Vec<ParsedCookie> = response
            .headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
            .filter_map(|(_, v)| ParsedCookie::parse(v))
            .collect();

        let cookie_count = cookies.len();

        if cookie_count == 0 {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(icon(ICON_GLOBE, ICON_MD, theme.colors.text_muted.opacity(0.5)))
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("No cookies in response")
                        )
                )
                .into_any_element();
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            // Toolbar
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pb(px(12.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .bg(theme.colors.accent.opacity(0.12))
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.accent)
                                    .child(format!("{} cookies", cookie_count))
                            )
                    )
            )
            // Cookie table
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_tertiary)
                    .overflow_hidden()
                    // Header row
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .bg(theme.colors.bg_secondary)
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .child(
                                div()
                                    .w(px(self.cookie_col1_w))
                                    .min_w(px(60.0))
                                    .px(px(12.0))
                                    .py(px(10.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("NAME")
                            )
                            .child(self.render_col_drag_handle(1, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.0))
                                    .py(px(10.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("VALUE")
                            )
                            .child(self.render_col_drag_handle(2, cx))
                            .child(
                                div()
                                    .w(px(self.cookie_col3_w))
                                    .min_w(px(60.0))
                                    .px(px(12.0))
                                    .py(px(10.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("PATH")
                            )
                            .child(self.render_col_drag_handle(3, cx))
                            .child(
                                div()
                                    .w(px(self.cookie_col4_w))
                                    .min_w(px(60.0))
                                    .px(px(12.0))
                                    .py(px(10.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("FLAGS")
                            )
                    )
                    // Cookie rows
                    .children(cookies.into_iter().enumerate().map(|(i, cookie)| {
                        let has_border = i > 0;
                        let mut flags = Vec::new();
                        if cookie.secure { flags.push("Secure"); }
                        if cookie.http_only { flags.push("HttpOnly"); }
                        let flags_str = flags.join(", ");
                        let col1 = self.cookie_col1_w;
                        let col3 = self.cookie_col3_w;
                        let col4 = self.cookie_col4_w;

                        div()
                            .w_full()
                            .flex()
                            .when(has_border, |el| el.border_t_1().border_color(theme.colors.border.opacity(0.5)))
                            .hover(|s| s.bg(theme.colors.bg_secondary.opacity(0.3)))
                            .child(
                                div()
                                    .w(px(col1)).min_w(px(60.0))
                                    .px(px(12.0)).py(px(8.0))
                                    .text_size(px(12.0)).font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.accent).overflow_hidden()
                                    .child(cookie.name)
                            )
                            .child(div().w(px(4.0)))
                            .child(
                                div()
                                    .flex_1().px(px(12.0)).py(px(8.0))
                                    .text_size(px(12.0)).font_family("JetBrains Mono")
                                    .text_color(theme.colors.text_primary).overflow_hidden()
                                    .child(cookie.value)
                            )
                            .child(div().w(px(4.0)))
                            .child(
                                div()
                                    .w(px(col3)).min_w(px(60.0))
                                    .px(px(12.0)).py(px(8.0))
                                    .text_size(px(11.0)).text_color(theme.colors.text_muted).overflow_hidden()
                                    .child(cookie.path.unwrap_or_else(|| "/".to_string()))
                            )
                            .child(div().w(px(4.0)))
                            .child(
                                div()
                                    .w(px(col4)).min_w(px(60.0))
                                    .px(px(12.0)).py(px(8.0))
                                    .text_size(px(10.0))
                                    .text_color(if flags_str.is_empty() { theme.colors.text_muted.opacity(0.5) } else { theme.colors.status_success })
                                    .child(if flags_str.is_empty() { "-".to_string() } else { flags_str })
                            )
                    }))
            )
            .into_any_element()
    }

    fn render_tests_tab(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let test_count = self.test_results.len();
        let passed_count = self.test_results.iter().filter(|t| t.passed).count();
        let failed_count = test_count - passed_count;

        if test_count == 0 {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(icon(ICON_CHECK, ICON_MD, theme.colors.text_muted.opacity(0.5)))
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("No tests have been run yet")
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted.opacity(0.7))
                                .child("Add tests in the Scripts tab and send a request")
                        )
                )
                .into_any_element();
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            // Summary bar
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .pb(px(12.0))
                    .child(
                        div()
                            .px(px(10.0))
                            .py(px(6.0))
                            .bg(theme.colors.status_success.opacity(0.12))
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success))
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.status_success)
                                    .child(format!("{} passed", passed_count))
                            )
                    )
                    .when(failed_count > 0, |el| {
                        el.child(
                            div()
                                .px(px(10.0))
                                .py(px(6.0))
                                .bg(theme.colors.status_client_error.opacity(0.12))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(icon(ICON_CLOSE, ICON_SM, theme.colors.status_client_error))
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme.colors.status_client_error)
                                        .child(format!("{} failed", failed_count))
                                )
                        )
                    })
            )
            // Test results list
            .child(
                div()
                    .id("tests-list")
                    .flex_1()
                    .w_full()
                    .overflow_scroll()
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_tertiary)
                    .children(self.test_results.iter().enumerate().map(|(i, result)| {
                        let is_last = i == test_count - 1;
                        div()
                            .w_full()
                            .px(px(12.0))
                            .py(px(10.0))
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .when(!is_last, |el| {
                                el.border_b_1()
                                  .border_color(theme.colors.border.opacity(0.5))
                            })
                            // Status icon
                            .child(
                                div()
                                    .size(px(20.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(result.passed, |el| {
                                        el.bg(theme.colors.status_success.opacity(0.15))
                                          .child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success))
                                    })
                                    .when(!result.passed, |el| {
                                        el.bg(theme.colors.status_client_error.opacity(0.15))
                                          .child(icon(ICON_CLOSE, ICON_SM, theme.colors.status_client_error))
                                    })
                            )
                            // Test name
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.text_primary)
                                            .child(result.name.clone())
                                    )
                                    .when(!result.passed && !result.expected.is_empty(), |el| {
                                        el.child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(8.0))
                                                .child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .text_color(theme.colors.text_muted)
                                                        .child(format!("Expected: {}", result.expected))
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .text_color(theme.colors.text_muted)
                                                        .child(format!("Actual: {}", result.actual))
                                                )
                                        )
                                    })
                            )
                    }))
            )
            .into_any_element()
    }

    fn render_extract_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        // Check if body is JSON
        let is_json = response.body.trim().starts_with('{') || response.body.trim().starts_with('[');

        if !is_json {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(24.0))
                                .text_color(theme.colors.text_muted.opacity(0.5))
                                .child("$")
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("JSONPath extraction requires JSON response")
                        )
                )
                .into_any_element();
        }

        let jsonpath_value = self.jsonpath_input.read(cx).get_text().to_string();

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            // Input row
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    // JSONPath label
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.colors.text_secondary)
                            .child("JSONPath:")
                    )
                    // Input field
                    .child(
                        div()
                            .flex_1()
                            .h(px(32.0))
                            .child(self.jsonpath_input.clone())
                    )
                    // Extract button
                    .child(
                        div()
                            .id("extract-btn")
                            .h(px(32.0))
                            .px(px(12.0))
                            .flex()
                            .items_center()
                            .bg(theme.colors.accent)
                            .text_size(px(12.0))
                            .text_color(theme.colors.bg_primary)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.accent.opacity(0.85)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.run_extraction(cx);
                            }))
                            .child("Extract")
                    )
            )
            // Quick patterns
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Quick:")
                    )
                    .children(
                        [("$.data", "$.data"), ("$[0]", "$[0]"), ("$.id", "$.id"), ("$.token", "$.token")]
                            .into_iter()
                            .map(|(label, pattern)| {
                                let pattern = pattern.to_string();
                                div()
                                    .id(SharedString::from(format!("pattern-{}", label)))
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .bg(theme.colors.bg_tertiary)
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .text_size(px(10.0))
                                    .font_family("JetBrains Mono")
                                    .text_color(theme.colors.text_secondary)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.bg_elevated))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.jsonpath_input.update(cx, |input, cx| {
                                            input.set_text(&pattern, cx);
                                        });
                                        this.run_extraction(cx);
                                    }))
                                    .child(label)
                            })
                    )
            )
            // Result display
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_tertiary)
                    .overflow_hidden()
                    .child(
                        match &self.extraction_result {
                            Some(Ok(value)) => {
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    // Success header
                                    .child(
                                        div()
                                            .w_full()
                                            .px(px(12.0))
                                            .py(px(8.0))
                                            .border_b_1()
                                            .border_color(theme.colors.border)
                                            .bg(theme.colors.status_success.opacity(0.1))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.0))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .child(icon(ICON_CHECK, ICON_SM, theme.colors.status_success))
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.0))
                                                            .text_color(theme.colors.status_success)
                                                            .child(format!("Extracted: {}", jsonpath_value))
                                                    )
                                            )
                                            .child(
                                                div()
                                                    .id("copy-extract-btn")
                                                    .px(px(8.0))
                                                    .py(px(4.0))
                                                    .border_1()
                                                    .border_color(theme.colors.border)
                                                    .text_size(px(10.0))
                                                    .text_color(theme.colors.text_secondary)
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(theme.colors.bg_elevated))
                                                    .on_click({
                                                        let value = value.clone();
                                                        cx.listener(move |_this, _, _, cx| {
                                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.clone()));
                                                        })
                                                    })
                                                    .child("Copy")
                                            )
                                    )
                                    // Value display with syntax highlighting
                                    .child(
                                        div()
                                            .id("extract-value")
                                            .flex_1()
                                            .overflow_hidden()
                                            .child(self.extraction_editor.clone())
                                    )
                                    .into_any_element()
                            }
                            Some(Err(error)) => {
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    // Error header
                                    .child(
                                        div()
                                            .w_full()
                                            .px(px(12.0))
                                            .py(px(8.0))
                                            .border_b_1()
                                            .border_color(theme.colors.border)
                                            .bg(theme.colors.status_client_error.opacity(0.1))
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .child(icon(ICON_CLOSE, ICON_SM, theme.colors.status_client_error))
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(theme.colors.status_client_error)
                                                    .child("Extraction failed")
                                            )
                                    )
                                    // Error message
                                    .child(
                                        div()
                                            .flex_1()
                                            .p(px(12.0))
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(theme.colors.text_muted)
                                                    .child(error.clone())
                                            )
                                    )
                                    .into_any_element()
                            }
                            None => {
                                div()
                                    .size_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .gap(px(8.0))
                                            .child(
                                                div()
                                                    .text_size(px(14.0))
                                                    .text_color(theme.colors.text_muted.opacity(0.5))
                                                    .child("$")
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(theme.colors.text_muted)
                                                    .child("Enter a JSONPath expression and click Extract")
                                            )
                                    )
                                    .into_any_element()
                            }
                        }
                    )
            )
            .into_any_element()
    }

    fn run_extraction(&mut self, cx: &mut Context<Self>) {
        let Some(response) = &self.response else {
            return;
        };

        let jsonpath = self.jsonpath_input.read(cx).get_text().to_string();
        if jsonpath.is_empty() {
            self.extraction_result = Some(Err("Enter a JSONPath expression".to_string()));
            cx.notify();
            return;
        }

        let result = chaining::extract_jsonpath(&response.body, &jsonpath);
        if let Ok(ref value) = result {
            let (content, lang) = if value.trim().starts_with('{') || value.trim().starts_with('[') {
                let pretty = serde_json::from_str::<serde_json::Value>(value)
                    .ok()
                    .and_then(|v| serde_json::to_string_pretty(&v).ok())
                    .unwrap_or_else(|| value.clone());
                (pretty, Language::Json)
            } else {
                (value.clone(), Language::Json)
            };
            self.extraction_editor.update(cx, |editor, cx| {
                editor.set_content(&content, cx);
                editor.set_language(lang, cx);
            });
        }
        self.extraction_result = Some(result);
        cx.notify();
    }

    fn render_col_drag_handle(&self, drag_id: u8, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let col_w = match drag_id {
            0 => self.resp_header_col1_w,
            1 => self.cookie_col1_w,
            2 => self.cookie_col3_w,
            3 => self.cookie_col4_w,
            _ => 0.0,
        };
        div()
            .id(SharedString::from(format!("col-drag-handle-{}", drag_id)))
            .w(px(4.0))
            .self_stretch()
            .cursor_col_resize()
            .bg(theme.colors.border.opacity(0.3))
            .hover(|s| s.bg(theme.colors.accent.opacity(0.5)))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                this.resp_col_drag = Some((drag_id, f32::from(event.position.x), col_w));
                cx.notify();
            }))
    }
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn truncate_error(error: &str) -> String {
    if error.len() > 40 {
        format!("{}...", &error[..37])
    } else {
        error.to_string()
    }
}

fn status_description(status: u16) -> Option<&'static str> {
    match status {
        // 1xx Informational
        100 => Some("Continue - Server received request headers"),
        101 => Some("Switching Protocols"),
        102 => Some("Processing - Server is processing the request"),
        103 => Some("Early Hints"),

        // 2xx Success
        200 => Some("Request succeeded"),
        201 => Some("Resource created successfully"),
        202 => Some("Request accepted for processing"),
        203 => Some("Non-authoritative information"),
        204 => Some("No content to return"),
        205 => Some("Reset content"),
        206 => Some("Partial content delivered"),
        207 => Some("Multi-status response"),
        208 => Some("Already reported"),
        226 => Some("IM Used"),

        // 3xx Redirection
        300 => Some("Multiple choices available"),
        301 => Some("Resource moved permanently"),
        302 => Some("Resource found at different URI"),
        303 => Some("See other resource"),
        304 => Some("Resource not modified"),
        305 => Some("Use proxy"),
        307 => Some("Temporary redirect"),
        308 => Some("Permanent redirect"),

        // 4xx Client Errors
        400 => Some("Bad request syntax or invalid"),
        401 => Some("Authentication required"),
        402 => Some("Payment required"),
        403 => Some("Access forbidden"),
        404 => Some("Resource not found"),
        405 => Some("Method not allowed"),
        406 => Some("Not acceptable format"),
        407 => Some("Proxy authentication required"),
        408 => Some("Request timeout"),
        409 => Some("Conflict with current state"),
        410 => Some("Resource no longer available"),
        411 => Some("Length required"),
        412 => Some("Precondition failed"),
        413 => Some("Payload too large"),
        414 => Some("URI too long"),
        415 => Some("Unsupported media type"),
        416 => Some("Range not satisfiable"),
        417 => Some("Expectation failed"),
        418 => Some("I'm a teapot"),
        421 => Some("Misdirected request"),
        422 => Some("Unprocessable entity"),
        423 => Some("Resource is locked"),
        424 => Some("Failed dependency"),
        425 => Some("Too early"),
        426 => Some("Upgrade required"),
        428 => Some("Precondition required"),
        429 => Some("Too many requests"),
        431 => Some("Request header fields too large"),
        451 => Some("Unavailable for legal reasons"),

        // 5xx Server Errors
        500 => Some("Internal server error"),
        501 => Some("Not implemented"),
        502 => Some("Bad gateway"),
        503 => Some("Service unavailable"),
        504 => Some("Gateway timeout"),
        505 => Some("HTTP version not supported"),
        506 => Some("Variant also negotiates"),
        507 => Some("Insufficient storage"),
        508 => Some("Loop detected"),
        510 => Some("Not extended"),
        511 => Some("Network authentication required"),

        _ => None,
    }
}

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
