//! Theme configuration - follows system preference
//!
//! This module provides a comprehensive design token system including:
//! - Colors (semantic and contextual)
//! - Typography (sizes, weights, line heights)
//! - Spacing (8pt grid system)
//! - Component dimensions
//! - Border radius scale
//! - Opacity levels
//! - Focus indicators

#![allow(dead_code)]

use gpui::{App, Pixels, rgb, Hsla};

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
    pub method_head: Hsla,
    pub method_options: Hsla,

    // Protocol colors
    pub protocol_ws: Hsla,
    pub protocol_grpc: Hsla,
    pub protocol_graphql: Hsla,

    // Status codes
    pub status_success: Hsla,
    pub status_redirect: Hsla,
    pub status_client_error: Hsla,
    pub status_server_error: Hsla,

    // Semantic colors
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    pub info: Hsla,

    // Interactive states
    pub hover_overlay: Hsla,
    pub active_overlay: Hsla,
    pub selected_bg: Hsla,

    // Focus indicator
    pub focus_ring: Hsla,
    pub focus_ring_error: Hsla,

    // Modal backdrop
    pub overlay: Hsla,
}

impl Colors {
    pub fn dark() -> Self {
        Self {
            // Backgrounds — Zed-style ultra-dark IDE palette
            bg_primary: rgb(0x0d0d0f).into(),    // app bg
            bg_secondary: rgb(0x111113).into(),  // sidebar bg
            bg_tertiary: rgb(0x131315).into(),   // panel bg (request/response)
            bg_elevated: rgb(0x1b1b1e).into(),   // input/elevated bg

            // Text
            text_primary: rgb(0xe4e4ed).into(),
            text_secondary: rgb(0x7f7f92).into(),
            text_muted: rgb(0x3e3e4a).into(),

            // Borders
            border: rgb(0x252529).into(),
            border_focused: rgb(0x4ade80).into(),

            // Accent — green like Zed
            accent: rgb(0x4ade80).into(),
            accent_hover: rgb(0x6ee7a0).into(),

            // HTTP Methods — design spec colors
            method_get: rgb(0x4ade80).into(),    // green
            method_post: rgb(0x60a5fa).into(),   // blue
            method_put: rgb(0xfbbf24).into(),    // yellow
            method_patch: rgb(0xfb923c).into(),  // orange
            method_delete: rgb(0xf87171).into(), // red
            method_head: rgb(0xa78bfa).into(),   // purple
            method_options: rgb(0x94a3b8).into(), // slate

            // Protocol colors
            protocol_ws: rgb(0x34d399).into(),      // emerald
            protocol_grpc: rgb(0x818cf8).into(),    // indigo
            protocol_graphql: rgb(0xf472b6).into(), // pink

            // Status codes
            status_success: rgb(0x4ade80).into(),
            status_redirect: rgb(0xfbbf24).into(),
            status_client_error: rgb(0xfb923c).into(),
            status_server_error: rgb(0xf87171).into(),

            // Semantic colors
            success: rgb(0x4ade80).into(),
            warning: rgb(0xfbbf24).into(),
            error: rgb(0xf87171).into(),
            info: rgb(0x60a5fa).into(),

            // Interactive states (white overlays for dark theme)
            hover_overlay: Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.035 },
            active_overlay: Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.055 },
            selected_bg: Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.055 },

            // Focus indicator
            focus_ring: rgb(0x4ade80).into(),
            focus_ring_error: rgb(0xf87171).into(),

            // Modal backdrop
            overlay: Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.5 },
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
            method_head: rgb(0x7c3aed).into(),
            method_options: rgb(0x475569).into(),

            // Protocol colors
            protocol_ws: rgb(0x059669).into(),
            protocol_grpc: rgb(0x4338ca).into(),
            protocol_graphql: rgb(0xdb2777).into(),

            // Status codes
            status_success: rgb(0x2e7d32).into(),
            status_redirect: rgb(0xef6c00).into(),
            status_client_error: rgb(0xc62828).into(),
            status_server_error: rgb(0xc62828).into(),

            // Semantic colors
            success: rgb(0x2e7d32).into(),
            warning: rgb(0xef6c00).into(),
            error: rgb(0xc62828).into(),
            info: rgb(0x007acc).into(),

            // Interactive states (black overlays for light theme)
            hover_overlay: Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.04 },
            active_overlay: Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.08 },
            selected_bg: Hsla { h: 203.0 / 360.0, s: 1.0, l: 0.4, a: 0.15 },

            // Focus indicator
            focus_ring: rgb(0x007acc).into(),
            focus_ring_error: rgb(0xc62828).into(),

            // Modal backdrop
            overlay: Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.4 },
        }
    }
}

/// Current theme colors (will follow system preference)
#[derive(Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub colors: Colors,
    pub is_dark: bool,

    // Design tokens
    pub spacing: Spacing,
    pub typography: Typography,
    pub sizes: ComponentSizes,
    pub radius: BorderRadius,
    pub opacity: Opacity,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            colors: Colors::dark(),
            is_dark: true,
            spacing: Spacing::new(),
            typography: Typography::new(),
            sizes: ComponentSizes::new(),
            radius: BorderRadius::new(),
            opacity: Opacity::new(),
        }
    }

    pub fn light() -> Self {
        Self {
            colors: Colors::light(),
            is_dark: false,
            spacing: Spacing::new(),
            typography: Typography::new(),
            sizes: ComponentSizes::new(),
            radius: BorderRadius::new(),
            opacity: Opacity::new(),
        }
    }

    /// Get the color for an HTTP method or protocol label
    pub fn method_color(&self, method: &str) -> Hsla {
        match method.to_uppercase().as_str() {
            "GET" => self.colors.method_get,
            "POST" => self.colors.method_post,
            "PUT" => self.colors.method_put,
            "PATCH" => self.colors.method_patch,
            "DELETE" => self.colors.method_delete,
            "HEAD" => self.colors.method_head,
            "OPTIONS" => self.colors.method_options,
            "WS" | "WEBSOCKET" | "SIO" => self.colors.protocol_ws,
            "GRPC" | "TRPC" => self.colors.protocol_grpc,
            "GQL" | "GRAPHQL" => self.colors.protocol_graphql,
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

/// Spacing scale based on 8-point grid system
/// Use these constants for consistent spacing throughout the UI
#[derive(Clone, Copy)]
pub struct Spacing {
    pub xs: Pixels,      // 4px - tight spacing
    pub sm: Pixels,      // 8px - small spacing
    pub md: Pixels,      // 12px - medium spacing
    pub base: Pixels,    // 16px - standard spacing
    pub lg: Pixels,      // 24px - large spacing
    pub xl: Pixels,      // 32px - extra large spacing
    pub xxl: Pixels,     // 48px - extra extra large spacing
}

impl Spacing {
    pub fn new() -> Self {
        use gpui::px;
        Self {
            xs: px(4.0),
            sm: px(8.0),
            md: px(12.0),
            base: px(16.0),
            lg: px(24.0),
            xl: px(32.0),
            xxl: px(48.0),
        }
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self::new()
    }
}

/// Typography scale for consistent text sizing
#[derive(Clone, Copy)]
pub struct Typography {
    pub xs: Pixels,      // 10px - tiny text
    pub sm: Pixels,      // 12px - small text
    pub base: Pixels,    // 13px - body text
    pub md: Pixels,      // 14px - medium text
    pub lg: Pixels,      // 15px - large text
    pub xl: Pixels,      // 16px - extra large text
}

impl Typography {
    pub fn new() -> Self {
        use gpui::px;
        Self {
            xs: px(10.0),
            sm: px(12.0),
            base: px(13.0),
            md: px(14.0),
            lg: px(15.0),
            xl: px(16.0),
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self::new()
    }
}

/// Raw dimension constants — use these in components that lack `cx`.
/// All interactive components of the same tier should share these values.
pub mod sizes {
    pub const INPUT_XS: f32 = 24.0;     // inline / extra-small input
    pub const INPUT_SM: f32 = 28.0;     // compact input, small button, compact row
    pub const INPUT_MD: f32 = 32.0;     // standard input, medium button, standard row
    pub const INPUT_LG: f32 = 36.0;     // large input, large button
    pub const PANEL_HEADER: f32 = 32.0; // section / collapsible headers
    pub const TOOLBAR: f32 = 40.0;      // toolbars, tab bars, nav bars
    pub const URL_BAR: f32 = 64.0;      // primary URL bar
}

/// Pixel-typed component dimensions for use via `theme.sizes.*`.
#[derive(Clone, Copy)]
pub struct ComponentSizes {
    pub input_xs: Pixels,       // 24px - inline / extra-small input
    pub input_sm: Pixels,       // 28px - compact input / small button / compact row
    pub input_md: Pixels,       // 32px - standard input / medium button / standard row
    pub input_lg: Pixels,       // 36px - large input / large button
    pub panel_header: Pixels,   // 32px - section / collapsible headers
    pub toolbar: Pixels,        // 40px - toolbars, tab bars, nav bars
    pub url_bar: Pixels,        // 64px - primary URL bar
}

impl ComponentSizes {
    pub fn new() -> Self {
        use gpui::px;
        use crate::theme::sizes;
        Self {
            input_xs: px(sizes::INPUT_XS),
            input_sm: px(sizes::INPUT_SM),
            input_md: px(sizes::INPUT_MD),
            input_lg: px(sizes::INPUT_LG),
            panel_header: px(sizes::PANEL_HEADER),
            toolbar: px(sizes::TOOLBAR),
            url_bar: px(sizes::URL_BAR),
        }
    }
}

impl Default for ComponentSizes {
    fn default() -> Self {
        Self::new()
    }
}

/// Border radius scale for consistent rounded corners
#[derive(Clone, Copy)]
pub struct BorderRadius {
    pub sm: Pixels,      // 4px - subtle rounding
    pub md: Pixels,      // 6px - standard rounding
    pub lg: Pixels,      // 8px - pronounced rounding
    pub xl: Pixels,      // 12px - large rounding
}

impl BorderRadius {
    pub fn new() -> Self {
        use gpui::px;
        Self {
            sm: px(4.0),
            md: px(6.0),
            lg: px(8.0),
            xl: px(12.0),
        }
    }
}

impl Default for BorderRadius {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard opacity levels for consistency
#[derive(Clone, Copy)]
pub struct Opacity {
    pub disabled: f32,          // 0.4 - disabled elements
    pub muted: f32,             // 0.6 - muted text
    pub hover: f32,             // 0.08 - hover overlay
    pub pressed: f32,           // 0.12 - pressed/active overlay
    pub selected: f32,          // 0.2 - selected background
}

impl Opacity {
    pub fn new() -> Self {
        Self {
            disabled: 0.4,
            muted: 0.6,
            hover: 0.08,
            pressed: 0.12,
            selected: 0.2,
        }
    }
}

impl Default for Opacity {
    fn default() -> Self {
        Self::new()
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
