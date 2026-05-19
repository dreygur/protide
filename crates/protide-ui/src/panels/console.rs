//! Unified request/response console — tiered logging with Info, Debug, and Error levels.

use std::collections::VecDeque;

use gpui::{
    div, prelude::*, px, ClipboardItem, Context, FocusHandle, IntoElement, KeyDownEvent,
    MouseButton, ParentElement, Render, ScrollHandle, SharedString, Styled, Window,
};

use crate::theme;
use crate::components::icons::{icon, ICON_SM, ICON_CLOSE, ICON_COPY, ICON_ELLIPSIS};

const MAX_ENTRIES: usize = 500;

// ── Log level ─────────────────────────────────────────────────────────────────

/// Severity tier for a console log entry.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    #[default]
    Info,
    Debug,
    Error,
}

// ── Entry source ──────────────────────────────────────────────────────────────

/// Where a console entry originated.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub enum ConsoleEntrySource {
    #[default]
    Request,
    Script,
    System,
    Team,
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ConsoleEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub level: LogLevel,
    pub source: ConsoleEntrySource,
    pub protocol: String,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub response_body: String,
    /// Actionable troubleshooting steps shown in the context menu for DNS / IO errors.
    pub troubleshoot_hint: Option<String>,
}

impl ConsoleEntry {
    /// Build a team-event entry (peer joined/left, sync status)
    pub fn team(message: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Local::now(),
            level: LogLevel::Info,
            source: ConsoleEntrySource::Team,
            protocol: String::new(),
            method: String::new(),
            url: message.into(),
            status: 0,
            duration_ms: 0,
            error: None,
            response_body: String::new(),
            troubleshoot_hint: None,
        }
    }

    /// Build a system diagnostic entry (P2P internals: mDNS, PAKE, DHT, listen addr)
    pub fn system(message: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Local::now(),
            level: LogLevel::Debug,
            source: ConsoleEntrySource::System,
            protocol: "SYS".to_string(),
            method: String::new(),
            url: message.into(),
            status: 0,
            duration_ms: 0,
            error: None,
            response_body: String::new(),
            troubleshoot_hint: None,
        }
    }
}

impl ConsoleEntry {
    pub fn is_success(&self) -> bool {
        self.error.is_none() && (200..300).contains(&self.status)
    }

    pub fn is_error(&self) -> bool {
        self.level == LogLevel::Error || self.error.is_some() || self.status >= 400
    }

    /// Build a cURL command from this entry (best-effort, headers not stored).
    pub fn as_curl(&self) -> String {
        format!("curl -X {} \"{}\"", self.method, self.url)
    }

    /// Full error detail string suitable for clipboard copy.
    pub fn error_details(&self) -> String {
        let mut out = format!("[{}] {} {}", self.timestamp.format("%H:%M:%S"), self.method, self.url);
        if self.status > 0 {
            out.push_str(&format!("\nStatus: {}", self.status));
        }
        if let Some(e) = &self.error {
            out.push_str(&format!("\nError: {}", e));
        }
        if let Some(hint) = &self.troubleshoot_hint {
            out.push_str(&format!("\n\nTroubleshooting:\n{}", hint));
        }
        out
    }
}

// ── Panel ─────────────────────────────────────────────────────────────────────

pub struct ConsolePanel {
    entries: VecDeque<ConsoleEntry>,
    scroll: ScrollHandle,
    focus: FocusHandle,
    /// Context-menu state: (entry index, cursor position)
    context_menu: Option<(usize, gpui::Point<gpui::Pixels>)>,
    /// Whether to show team/sync events
    show_team: bool,
    /// Whether to show internal P2P diagnostic events
    show_system: bool,
    /// URL/message field of selected entry (double-click selects, Ctrl+C copies)
    url_sel_entry: Option<usize>,
}

impl ConsolePanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_ENTRIES),
            scroll: ScrollHandle::new(),
            focus: cx.focus_handle(),
            context_menu: None,
            show_team: true,
            show_system: true,
            url_sel_entry: None,
        }
    }

    /// Append a new entry, evicting the oldest when the buffer is full.
    pub fn log(&mut self, entry: ConsoleEntry, cx: &mut Context<Self>) {
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.entries.clear();
        self.context_menu = None;
        self.url_sel_entry = None;
        cx.notify();
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn toggle_team(&mut self, cx: &mut Context<Self>) {
        self.show_team = !self.show_team;
        cx.notify();
    }

    pub fn toggle_system(&mut self, cx: &mut Context<Self>) {
        self.show_system = !self.show_system;
        cx.notify();
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

impl Render for ConsolePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let entry_count = self.entries.len();

        let entries: Vec<ConsoleEntry> = self.entries.iter().cloned().collect();
        let context_menu = self.context_menu;
        let url_sel = self.url_sel_entry;

        div()
            .id("console-panel")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_primary)
            .border_t_1()
            .border_color(theme.colors.border)
            .track_focus(&self.focus)
            // Ctrl+C: copy the selected entry's URL/message
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.control && event.keystroke.key.as_str() == "c" {
                    if let Some(idx) = this.url_sel_entry
                        && let Some(entry) = this.entries.get(idx) {
                            cx.write_to_clipboard(ClipboardItem::new_string(entry.url.clone()));
                        }
                    cx.stop_propagation();
                }
            }))
            // Single-click outside rows clears URL selection and dismisses context menu
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                let changed = this.url_sel_entry.is_some() || this.context_menu.is_some();
                this.url_sel_entry = None;
                this.context_menu = None;
                if changed { cx.notify(); }
            }))
            // Header bar
            .child(
                div()
                    .w_full()
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .gap(px(8.0))
                    .bg(theme.colors.bg_secondary)
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_secondary)
                            .child("CONSOLE")
                    )
                    .child(
                        div()
                            .px(px(5.0))
                            .py(px(1.0))
                            .bg(theme.colors.accent.opacity(0.15))
                            .text_size(px(10.0))
                            .text_color(theme.colors.accent)
                            .child(SharedString::from(format!("{}", entry_count)))
                    )
                    .child(div().flex_1())
                    // Level legend chips
                    .child(level_chip("INFO",  theme.colors.success, &theme))
                    .child(level_chip("DEBUG", theme.colors.info,    &theme))
                    .child(level_chip("ERROR", theme.colors.error,   &theme))
                    // Team events toggle
                    .child(
                        div()
                            .id("console-toggle-team")
                            .px(px(5.0))
                            .py(px(1.0))
                            .flex_shrink_0()
                            .cursor_pointer()
                            .bg(if self.show_team {
                                theme.colors.team_accent.opacity(0.15)
                            } else {
                                gpui::Hsla::default()
                            })
                            .text_size(px(9.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(if self.show_team {
                                theme.colors.team_accent
                            } else {
                                theme.colors.text_muted.opacity(0.5)
                            })
                            .hover(|s| s.bg(theme.colors.team_accent.opacity(0.1)))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_team(cx)))
                            .child("TEAM")
                    )
                    // System/P2P diagnostic toggle
                    .child(
                        div()
                            .id("console-toggle-system")
                            .px(px(5.0))
                            .py(px(1.0))
                            .flex_shrink_0()
                            .cursor_pointer()
                            .bg(if self.show_system {
                                theme.colors.info.opacity(0.15)
                            } else {
                                gpui::Hsla::default()
                            })
                            .text_size(px(9.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(if self.show_system {
                                theme.colors.info
                            } else {
                                theme.colors.text_muted.opacity(0.5)
                            })
                            .hover(|s| s.bg(theme.colors.info.opacity(0.1)))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_system(cx)))
                            .child("SYS")
                    )
                    .child(div().w(px(6.0)))
                    // Clear button
                    .child(
                        div()
                            .id("console-clear")
                            .px(px(8.0))
                            .h(px(22.0))
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_muted)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_click(cx.listener(|this, _, _, cx| this.clear(cx)))
                            .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted))
                            .child("Clear")
                    )
            )
            // Entry list
            .child(
                div()
                    .id("console-entries")
                    .w_full()
                    .flex_1()
                    .overflow_scroll()
                    .track_scroll(&self.scroll)
                    .when(entries.is_empty(), |el| {
                        el.child(
                            div()
                                .w_full()
                                .h_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.text_muted)
                                        .child("No requests yet.")
                                )
                        )
                    })
                    .children(
                        entries.iter().enumerate()
                        .filter(|(_, entry)| {
                            (self.show_team   || entry.source != ConsoleEntrySource::Team) &&
                            (self.show_system || entry.source != ConsoleEntrySource::System)
                        })
                        .map(|(i, entry)| {
                            let status = entry.status;
                            let is_ws  = entry.protocol == "WebSocket" || entry.protocol == "Socket.IO";
                            let is_team = entry.source == ConsoleEntrySource::Team;

                            // Level-first color: Error=red, Debug=blue, Info=green/status-based
                            let row_color = if is_team {
                                theme.colors.team_accent
                            } else {
                                match entry.level {
                                    LogLevel::Error => theme.colors.error,
                                    LogLevel::Debug => theme.colors.info,
                                    LogLevel::Info  => {
                                        if entry.error.is_some() || status >= 500 {
                                            theme.colors.error
                                        } else if status >= 400 {
                                            theme.colors.method_delete
                                        } else if status >= 300 {
                                            theme.colors.method_patch
                                        } else if status == 0 {
                                            theme.colors.text_muted
                                        } else {
                                            theme.colors.status_success
                                        }
                                    }
                                }
                            };

                            // Left accent bar color for the row
                            let level_bar = if is_team {
                                theme.colors.team_accent
                            } else {
                                match entry.level {
                                    LogLevel::Error => theme.colors.error,
                                    LogLevel::Debug => theme.colors.info,
                                    LogLevel::Info  => theme.colors.success.opacity(0.0),
                                }
                            };

                            let method_color = theme.method_color(&entry.method);
                            let has_hint = entry.troubleshoot_hint.is_some();
                            let has_error = entry.error.is_some();

                            let base_row = div()
                                .id(SharedString::from(format!("console-entry-{}", i)))
                                .w_full()
                                .h(px(24.0))
                                .flex()
                                .items_center()
                                .gap(px(10.0))
                                .border_b_1()
                                .border_color(theme.colors.border.opacity(0.4))
                                .hover(|s| s.bg(theme.colors.hover_overlay))
                                .on_mouse_down(
                                    MouseButton::Right,
                                    cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                        this.context_menu = Some((i, event.position));
                                        cx.stop_propagation();
                                        cx.notify();
                                    }),
                                )
                                // Left severity bar (3px)
                                .child(
                                    div()
                                        .w(px(3.0))
                                        .h(px(24.0))
                                        .flex_shrink_0()
                                        .bg(level_bar)
                                )
                                // Timestamp
                                .child(
                                    div()
                                        .w(px(56.0))
                                        .flex_shrink_0()
                                        .text_size(px(10.0))
                                        .text_color(theme.colors.text_muted)
                                        .child(SharedString::from(
                                            entry.timestamp.format("%H:%M:%S").to_string()
                                        ))
                                );

                            if is_team {
                                // Team entry: simpler layout with team badge + message
                                base_row
                                    .child(
                                        div()
                                            .px(px(4.0))
                                            .py(px(1.0))
                                            .flex_shrink_0()
                                            .bg(theme.colors.team_accent.opacity(0.12))
                                            .text_size(px(9.0))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.colors.team_accent)
                                            .child("TEAM")
                                    )
                                    // Message (url field repurposed for team messages)
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("console-url-{}", i)))
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .overflow_hidden()
                                            .text_size(px(11.5))
                                            .text_color(theme.colors.text_primary)
                                            .whitespace_nowrap()
                                            .cursor_text()
                                            .when(url_sel == Some(i), |el| el.bg(theme.colors.accent.opacity(0.15)))
                                            .child(SharedString::from(entry.url.clone()))
                                            .on_mouse_down(MouseButton::Left,
                                                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                                    cx.stop_propagation();
                                                    if event.click_count >= 2 {
                                                        this.url_sel_entry = Some(i);
                                                        if let Some(e) = this.entries.get(i) {
                                                            cx.write_to_clipboard(ClipboardItem::new_string(e.url.clone()));
                                                        }
                                                    } else {
                                                        this.url_sel_entry = if this.url_sel_entry == Some(i) { None } else { Some(i) };
                                                    }
                                                    cx.notify();
                                                })
                                            )
                                    )
                                    .child(div().w(px(108.0)).flex_shrink_0())
                            } else {
                                // Request/script/system entry: full layout
                                base_row
                                    // Level badge (DEBUG only)
                                    .when(entry.level == LogLevel::Debug, |el| {
                                        el.child(
                                            div()
                                                .px(px(4.0))
                                                .py(px(1.0))
                                                .flex_shrink_0()
                                                .bg(theme.colors.info.opacity(0.12))
                                                .text_size(px(9.0))
                                                .font_weight(gpui::FontWeight::BOLD)
                                                .text_color(theme.colors.info)
                                                .child("DBG")
                                        )
                                    })
                                    // Protocol badge
                                    .child(
                                        div()
                                            .px(px(4.0))
                                            .py(px(1.0))
                                            .min_w(px(36.0))
                                            .flex_shrink_0()
                                            .flex()
                                            .justify_center()
                                            .bg(theme.colors.accent.opacity(0.1))
                                            .text_size(px(9.0))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.colors.accent)
                                            .child(SharedString::from(entry.protocol.clone()))
                                    )
                                    // Method badge (skip for WS/SIO)
                                    .when(!is_ws, |el| {
                                        el.child(
                                            div()
                                                .w(px(46.0))
                                                .flex_shrink_0()
                                                .text_size(px(10.0))
                                                .font_weight(gpui::FontWeight::BOLD)
                                                .text_color(method_color)
                                                .child(SharedString::from(entry.method.clone()))
                                        )
                                    })
                                    // URL (truncated) — double-click to select & copy
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("console-url-{}", i)))
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .overflow_hidden()
                                            .text_size(px(11.5))
                                            .text_color(theme.colors.text_primary)
                                            .whitespace_nowrap()
                                            .cursor_text()
                                            .when(url_sel == Some(i), |el| el.bg(theme.colors.accent.opacity(0.15)))
                                            .child(SharedString::from(entry.url.clone()))
                                            .on_mouse_down(MouseButton::Left,
                                                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                                    cx.stop_propagation();
                                                    if event.click_count >= 2 {
                                                        this.url_sel_entry = Some(i);
                                                        if let Some(e) = this.entries.get(i) {
                                                            cx.write_to_clipboard(ClipboardItem::new_string(e.url.clone()));
                                                        }
                                                    } else {
                                                        this.url_sel_entry = if this.url_sel_entry == Some(i) { None } else { Some(i) };
                                                    }
                                                    cx.notify();
                                                })
                                            )
                                    )
                                    // Troubleshoot indicator
                                    .when(has_hint, |el| {
                                        el.child(
                                            div()
                                                .w(px(14.0))
                                                .h(px(14.0))
                                                .flex_shrink_0()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .bg(theme.colors.warning.opacity(0.2))
                                                .text_size(px(9.0))
                                                .font_weight(gpui::FontWeight::BOLD)
                                                .text_color(theme.colors.warning)
                                                .child("?")
                                        )
                                    })
                                    // Status / error
                                    .child(
                                        div()
                                            .w(px(54.0))
                                            .flex_shrink_0()
                                            .text_size(px(11.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(row_color)
                                            .child(SharedString::from(if has_error {
                                                "ERR".to_string()
                                            } else if status == 0 {
                                                "—".to_string()
                                            } else {
                                                status.to_string()
                                            }))
                                    )
                                    // Duration
                                    .child(
                                        div()
                                            .w(px(54.0))
                                            .flex_shrink_0()
                                            .pr(px(8.0))
                                            .text_size(px(10.5))
                                            .text_color(theme.colors.text_muted)
                                            .child(SharedString::from(if entry.duration_ms > 0 {
                                                format!("{}ms", entry.duration_ms)
                                            } else {
                                                String::new()
                                            }))
                                    )
                            }
                        })
                    )
            )
            // Right-click context menu (deferred so it renders above list)
            .when_some(context_menu, |el, (idx, pos)| {
                if let Some(entry) = self.entries.get(idx) {
                    let entry = entry.clone();
                    let x = f32::from(pos.x);
                    let y = f32::from(pos.y);
                    let has_hint     = entry.troubleshoot_hint.is_some();
                    let has_error    = entry.error.is_some();
                    let hint_text    = entry.troubleshoot_hint.clone().unwrap_or_default();
                    let error_detail = entry.error_details();
                    el.child(
                        gpui::deferred(
                            div()
                                .absolute()
                                .left(px(x))
                                .top(px(y))
                                .w(px(210.0))
                                .bg(theme.colors.bg_secondary)
                                .border_1()
                                .border_color(theme.colors.border)
                                .shadow_lg()
                                .overflow_hidden()
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                                    cx.stop_propagation();
                                }))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        // Copy as cURL
                                        .child({
                                            let curl = entry.as_curl();
                                            let bg = theme.colors.bg_tertiary;
                                            let tm = theme.colors.text_muted;
                                            let tp = theme.colors.text_primary;
                                            div()
                                                .id("console-ctx-curl")
                                                .w_full().h(px(28.0))
                                                .flex().items_center().px(px(12.0)).gap(px(8.0))
                                                .cursor_pointer()
                                                .hover(move |s| s.bg(bg))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    cx.write_to_clipboard(ClipboardItem::new_string(curl.clone()));
                                                    this.context_menu = None;
                                                    cx.notify();
                                                }))
                                                .child(icon(ICON_COPY, ICON_SM, tm))
                                                .child(div().text_size(px(12.0)).text_color(tp).child("Copy as cURL"))
                                        })
                                        // Copy response body
                                        .child({
                                            let body = entry.response_body.clone();
                                            let bg = theme.colors.bg_tertiary;
                                            let tm = theme.colors.text_muted;
                                            let tp = theme.colors.text_primary;
                                            div()
                                                .id("console-ctx-body")
                                                .w_full().h(px(28.0))
                                                .flex().items_center().px(px(12.0)).gap(px(8.0))
                                                .cursor_pointer()
                                                .hover(move |s| s.bg(bg))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    cx.write_to_clipboard(ClipboardItem::new_string(body.clone()));
                                                    this.context_menu = None;
                                                    cx.notify();
                                                }))
                                                .child(icon(ICON_ELLIPSIS, ICON_SM, tm))
                                                .child(div().text_size(px(12.0)).text_color(tp).child("Copy Response"))
                                        })
                                        // Copy error details (only when there is an error)
                                        .when(has_error, |el| {
                                            let detail = error_detail.clone();
                                            let bg = theme.colors.bg_tertiary;
                                            let tm = theme.colors.text_muted;
                                            let tp = theme.colors.text_primary;
                                            el.child(
                                                div()
                                                    .id("console-ctx-err-detail")
                                                    .w_full().h(px(28.0))
                                                    .flex().items_center().px(px(12.0)).gap(px(8.0))
                                                    .cursor_pointer()
                                                    .hover(move |s| s.bg(bg))
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        cx.write_to_clipboard(ClipboardItem::new_string(detail.clone()));
                                                        this.context_menu = None;
                                                        cx.notify();
                                                    }))
                                                    .child(icon(ICON_COPY, ICON_SM, tm))
                                                    .child(div().text_size(px(12.0)).text_color(tp).child("Copy Error Details"))
                                            )
                                        })
                                        // Separator before Troubleshoot
                                        .when(has_hint, |el| el.child(
                                            div()
                                                .h(px(1.0))
                                                .mx(px(8.0))
                                                .my(px(2.0))
                                                .bg(theme.colors.border)
                                        ))
                                        // Troubleshoot (only for DNS / IO errors)
                                        .when(has_hint, |el| {
                                            el.child(
                                                div()
                                                    .id("console-ctx-troubleshoot")
                                                    .w_full()
                                                    .px(px(12.0))
                                                    .py(px(6.0))
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(2.0))
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        cx.write_to_clipboard(ClipboardItem::new_string(hint_text.clone()));
                                                        this.context_menu = None;
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(6.0))
                                                            .child(
                                                                div()
                                                                    .size(px(12.0))
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .bg(theme.colors.warning.opacity(0.2))
                                                                    .text_size(px(8.0))
                                                                    .font_weight(gpui::FontWeight::BOLD)
                                                                    .text_color(theme.colors.warning)
                                                                    .child("?")
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_size(px(12.0))
                                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                                    .text_color(theme.colors.warning)
                                                                    .child("Troubleshoot")
                                                            )
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(10.0))
                                                            .text_color(theme.colors.text_muted)
                                                            .child("Click to copy steps to clipboard")
                                                    )
                                            )
                                        })
                                )
                        ).with_priority(10)
                    )
                } else {
                    el
                }
            })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn level_chip(label: &'static str, color: gpui::Hsla, _theme: &crate::theme::Theme) -> gpui::AnyElement {
    div()
        .px(px(5.0))
        .py(px(1.0))
        .bg(color.opacity(0.12))
        .text_size(px(9.0))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(color.opacity(0.7))
        .child(label)
        .into_any_element()
}

