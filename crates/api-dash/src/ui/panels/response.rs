//! Response viewer panel

use std::time::Duration;

use gpui::{
    div, prelude::*, px, Context, IntoElement, ParentElement, Render, SharedString, Styled,
    Window,
};

use crate::theme;

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

/// Response viewer panel
pub struct ResponsePanel {
    /// Active tab index for body/headers/cookies
    active_tab: usize,
    /// Response data (None if no request sent yet)
    response: Option<ResponseData>,
    /// Loading state
    loading: bool,
    /// Error message
    error: Option<String>,
}

impl ResponsePanel {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            active_tab: 0,
            response: None,
            loading: false,
            error: None,
        }
    }

    pub fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        self.error = None;
        cx.notify();
    }

    pub fn set_response(&mut self, response: ResponseData, cx: &mut Context<Self>) {
        self.response = Some(response);
        self.loading = false;
        self.error = None;
        cx.notify();
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

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_primary)
            // Header with status
            .child(self.render_header(cx))
            // Tabs
            .child(self.render_tabs(cx))
            // Content
            .child(
                div()
                    .id("response-content")
                    .flex_1()
                    .p(px(12.0))
                    .overflow_scroll()
                    .child(self.render_content(cx)),
            )
    }
}

impl ResponsePanel {
    fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .h(px(44.0))
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
                            .text_size(px(14.0))
                            .text_color(theme.colors.text_muted)
                            .child("↓")
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
                        .rounded_full()
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
                .rounded(px(6.0))
                .bg(theme.colors.status_client_error.opacity(0.1))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.colors.status_client_error)
                        .child("✕")
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.colors.status_client_error)
                        .child(truncate_error(error))
                )
        } else if let Some(response) = &self.response {
            let status_color = theme.status_color(response.status);
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
                                .rounded(px(6.0))
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
                // Time with icon
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .text_size(px(11.0))
                        .text_color(theme.colors.text_muted)
                        .child("⏱")
                        .child(format!("{}ms", response.time.as_millis()))
                )
                // Size with icon
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .text_size(px(11.0))
                        .text_color(theme.colors.text_muted)
                        .child("◉")
                        .child(format_size(response.size))
                )
        } else {
            div()
        }
    }

    fn render_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let tabs = [("Body", "{ }"), ("Headers", "≡")];
        let active_tab = self.active_tab;

        div()
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .px(px(12.0))
            .gap(px(2.0))
            .border_b_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.bg_primary)
            .children(tabs.iter().enumerate().map(|(i, (tab, icon))| {
                let is_active = i == active_tab;
                div()
                    .id(SharedString::from(format!("response-tab-{}", i)))
                    .px(px(14.0))
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded_t(px(6.0))
                    .cursor_pointer()
                    .when(is_active, |el| {
                        el.bg(theme.colors.bg_tertiary)
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.accent)
                                    .child(*icon)
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.text_primary)
                                    .child(*tab)
                            )
                    })
                    .when(!is_active, |el| {
                        el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.5)))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_muted)
                                    .child(*icon)
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child(*tab)
                            )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_tab(i, cx);
                    }))
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
                        .rounded_full()
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
                        .rounded_full()
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

        if let Some(response) = &self.response {
            match self.active_tab {
                0 => self.render_body_tab(response, cx),
                1 => self.render_headers_tab(response, cx),
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
                        .rounded(px(20.0))
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
                                        .text_size(px(14.0))
                                        .text_color(theme.colors.text_muted.opacity(0.4))
                                        .child("↓")
                                )
                                .child(
                                    div()
                                        .text_size(px(18.0))
                                        .text_color(theme.colors.text_muted.opacity(0.6))
                                        .child("↓")
                                )
                                .child(
                                    div()
                                        .text_size(px(22.0))
                                        .text_color(theme.colors.text_muted)
                                        .child("↓")
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
                                        .rounded(px(4.0))
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
                                        .rounded(px(4.0))
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

        // Detect content type and format
        let (body, format_label, format_color) = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response.body) {
            let formatted = serde_json::to_string_pretty(&json).unwrap_or_else(|_| response.body.clone());
            (formatted, "JSON", theme.colors.status_success)
        } else if response.body.trim().starts_with("<!DOCTYPE") || response.body.trim().starts_with("<html") {
            (response.body.clone(), "HTML", theme.colors.method_patch)
        } else if response.body.trim().starts_with("<?xml") || response.body.trim().starts_with("<") {
            (response.body.clone(), "XML", theme.colors.method_put)
        } else {
            (response.body.clone(), "Text", theme.colors.text_muted)
        };

        let is_empty = body.trim().is_empty();

        if is_empty {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child("Response body is empty")
                )
                .into_any_element();
        }

        let line_count = body.lines().count();

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Toolbar row
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    // Left: Format badge
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(4.0))
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
                    // Right: Copy button
                    .child(
                        div()
                            .id("copy-body-btn")
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(10.0))
                            .py(px(5.0))
                            .rounded(px(6.0))
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_secondary)
                            .cursor_pointer()
                            .border_1()
                            .border_color(theme.colors.border)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .child("⎘")
                            )
                            .child("Copy")
                    )
            )
            // Body content with line numbers style
            .child(
                div()
                    .w_full()
                    .rounded(px(8.0))
                    .bg(theme.colors.bg_tertiary)
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    // Code header bar
                    .child(
                        div()
                            .w_full()
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .border_b_1()
                            .border_color(theme.colors.border.opacity(0.5))
                            .bg(theme.colors.bg_secondary.opacity(0.5))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .children((0..3).map(|_| {
                                        div()
                                            .size(px(8.0))
                                            .rounded_full()
                                            .bg(theme.colors.text_muted.opacity(0.3))
                                    }))
                            )
                    )
                    // Code content
                    .child(
                        div()
                            .w_full()
                            .p(px(12.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_family("monospace")
                                    .text_color(theme.colors.text_primary)
                                    .child(body)
                            )
                    )
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
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child("No headers in response")
                )
                .into_any_element();
        }

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Header toolbar
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(4.0))
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
                    // Copy headers button
                    .child(
                        div()
                            .id("copy-headers-btn")
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(10.0))
                            .py(px(5.0))
                            .rounded(px(6.0))
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_secondary)
                            .cursor_pointer()
                            .border_1()
                            .border_color(theme.colors.border)
                            .hover(|s| s.bg(theme.colors.bg_tertiary).border_color(theme.colors.text_muted))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .child("⎘")
                            )
                            .child("Copy")
                    )
            )
            // Headers table
            .child(
                div()
                    .w_full()
                    .rounded(px(8.0))
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
                                    .w(px(180.0))
                                    .min_w(px(180.0))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("NAME")
                            )
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
                        div()
                            .w_full()
                            .flex()
                            .when(i % 2 == 0, |el| el.bg(theme.colors.bg_tertiary.opacity(0.3)))
                            .when(!is_last, |el| el.border_b_1().border_color(theme.colors.border.opacity(0.5)))
                            // Key column
                            .child(
                                div()
                                    .w(px(180.0))
                                    .min_w(px(180.0))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.colors.accent)
                                    .overflow_hidden()
                                    .child(key.clone())
                            )
                            // Value column
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .text_size(px(12.0))
                                    .font_family("monospace")
                                    .text_color(theme.colors.text_primary)
                                    .overflow_hidden()
                                    .child(value.clone())
                            )
                    }))
            )
            .into_any_element()
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
