//! Theme configuration - follows system preference

#![allow(dead_code)]

use gpui::{App, rgb, Hsla};

/// Color palette for the application
#[derive(Clone)]
pub struct Colors {
    // Backgrounds
    pub bg_primary: Hsla,
    pub bg_secondary: Hsla,
    pub bg_tertiary: Hsla,
    pub bg_elevated: Hsla,

    // Text
    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,

    // Borders
    pub border: Hsla,
    pub border_focused: Hsla,

    // Accent
    pub accent: Hsla,
    pub accent_hover: Hsla,

    // HTTP Methods
    pub method_get: Hsla,
    pub method_post: Hsla,
    pub method_put: Hsla,
    pub method_patch: Hsla,
    pub method_delete: Hsla,

    // Status codes
    pub status_success: Hsla,
    pub status_redirect: Hsla,
    pub status_client_error: Hsla,
    pub status_server_error: Hsla,
}

impl Colors {
    pub fn dark() -> Self {
        Self {
            // Backgrounds
            bg_primary: rgb(0x1e1e1e).into(),
            bg_secondary: rgb(0x252526).into(),
            bg_tertiary: rgb(0x2d2d2d).into(),
            bg_elevated: rgb(0x333333).into(),

            // Text
            text_primary: rgb(0xcccccc).into(),
            text_secondary: rgb(0x9d9d9d).into(),
            text_muted: rgb(0x6d6d6d).into(),

            // Borders
            border: rgb(0x3c3c3c).into(),
            border_focused: rgb(0x007acc).into(),

            // Accent
            accent: rgb(0x007acc).into(),
            accent_hover: rgb(0x1c8cd4).into(),

            // HTTP Methods
            method_get: rgb(0x61affe).into(),
            method_post: rgb(0x49cc90).into(),
            method_put: rgb(0xfca130).into(),
            method_patch: rgb(0x50e3c2).into(),
            method_delete: rgb(0xf93e3e).into(),

            // Status codes
            status_success: rgb(0x49cc90).into(),
            status_redirect: rgb(0xfca130).into(),
            status_client_error: rgb(0xf93e3e).into(),
            status_server_error: rgb(0xf93e3e).into(),
        }
    }

    pub fn light() -> Self {
        Self {
            // Backgrounds
            bg_primary: rgb(0xffffff).into(),
            bg_secondary: rgb(0xf3f3f3).into(),
            bg_tertiary: rgb(0xe8e8e8).into(),
            bg_elevated: rgb(0xffffff).into(),

            // Text
            text_primary: rgb(0x1e1e1e).into(),
            text_secondary: rgb(0x616161).into(),
            text_muted: rgb(0x9e9e9e).into(),

            // Borders
            border: rgb(0xd4d4d4).into(),
            border_focused: rgb(0x007acc).into(),

            // Accent
            accent: rgb(0x007acc).into(),
            accent_hover: rgb(0x0066b8).into(),

            // HTTP Methods
            method_get: rgb(0x0066cc).into(),
            method_post: rgb(0x2e7d32).into(),
            method_put: rgb(0xef6c00).into(),
            method_patch: rgb(0x00897b).into(),
            method_delete: rgb(0xc62828).into(),

            // Status codes
            status_success: rgb(0x2e7d32).into(),
            status_redirect: rgb(0xef6c00).into(),
            status_client_error: rgb(0xc62828).into(),
            status_server_error: rgb(0xc62828).into(),
        }
    }
}

/// Current theme colors (will follow system preference)
#[derive(Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub colors: Colors,
    pub is_dark: bool,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            colors: Colors::dark(),
            is_dark: true,
        }
    }

    pub fn light() -> Self {
        Self {
            colors: Colors::light(),
            is_dark: false,
        }
    }

    /// Get the color for an HTTP method
    pub fn method_color(&self, method: &str) -> Hsla {
        match method.to_uppercase().as_str() {
            "GET" => self.colors.method_get,
            "POST" => self.colors.method_post,
            "PUT" => self.colors.method_put,
            "PATCH" => self.colors.method_patch,
            "DELETE" => self.colors.method_delete,
            _ => self.colors.text_secondary,
        }
    }

    /// Get the color for a status code
    pub fn status_color(&self, status: u16) -> Hsla {
        match status {
            200..=299 => self.colors.status_success,
            300..=399 => self.colors.status_redirect,
            400..=499 => self.colors.status_client_error,
            500..=599 => self.colors.status_server_error,
            _ => self.colors.text_secondary,
        }
    }
}

impl gpui::Global for Theme {}

/// Initialize theme based on system preference
pub fn init(cx: &mut App) {
    // Default to dark theme (TODO: detect system preference)
    cx.set_global(Theme::dark());
}

/// Get the current theme
pub fn current<C: gpui::AppContext>(cx: &C) -> Theme {
    cx.read_global::<Theme, _>(|theme, _| theme.clone())
}
