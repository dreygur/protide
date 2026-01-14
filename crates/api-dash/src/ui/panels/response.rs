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
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

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
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .border_b_1()
            .border_color(theme.colors.border)
            // Title
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Response"),
            )
            // Status info
            .child(self.render_status_info(cx))
    }

    fn render_status_info(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        if self.loading {
            div()
                .text_size(px(12.0))
                .text_color(theme.colors.text_muted)
                .child("Loading...")
        } else if let Some(error) = &self.error {
            div()
                .text_size(px(12.0))
                .text_color(gpui::rgb(0xef4444))
                .child(format!("Error: {}", error))
        } else if let Some(response) = &self.response {
            let status_color = if response.is_success() {
                gpui::rgb(0x22c55e) // green
            } else if response.is_error() {
                gpui::rgb(0xef4444) // red
            } else {
                gpui::rgb(0xf59e0b) // amber
            };

            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .text_size(px(12.0))
                // Status badge
                .child(
                    div()
                        .px(px(8.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .bg(status_color)
                        .text_color(gpui::white())
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(format!("{} {}", response.status, response.status_text)),
                )
                // Time
                .child(
                    div()
                        .text_color(theme.colors.text_muted)
                        .child(format!("{}ms", response.time.as_millis())),
                )
                // Size
                .child(
                    div()
                        .text_color(theme.colors.text_muted)
                        .child(format_size(response.size)),
                )
        } else {
            div()
                .text_size(px(12.0))
                .text_color(theme.colors.text_muted)
                .child("Send a request to see the response")
        }
    }

    fn render_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let tabs = ["Body", "Headers"];
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
                    .id(SharedString::from(format!("response-tab-{}", i)))
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

    fn render_content(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        if self.loading {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.colors.text_muted)
                        .child("Loading..."),
                )
                .into_any_element();
        }

        if let Some(error) = &self.error {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(gpui::rgb(0xef4444))
                        .child(error.clone()),
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
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.colors.text_muted)
                        .child("Response will appear here after sending a request"),
                )
                .into_any_element()
        }
    }

    fn render_body_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        // Try to format JSON
        let body = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response.body) {
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| response.body.clone())
        } else {
            response.body.clone()
        };

        div()
            .w_full()
            .p(px(8.0))
            .rounded(px(4.0))
            .bg(theme.colors.bg_tertiary)
            .child(
                div()
                    .text_size(px(12.0))
                    .font_family("monospace")
                    .text_color(theme.colors.text_primary)
                    .child(body),
            )
            .into_any_element()
    }

    fn render_headers_tab(&self, response: &ResponseData, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .children(response.headers.iter().map(|(key, value)| {
                div()
                    .w_full()
                    .flex()
                    .gap(px(8.0))
                    .py(px(4.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .min_w(px(150.0))
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_secondary)
                            .child(key.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(12.0))
                            .text_color(theme.colors.text_primary)
                            .child(value.clone()),
                    )
            }))
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
