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

use gpui::{App, Hsla, Pixels, rgb};

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

    // Collaboration / sync
    pub team_accent: Hsla,
    pub sync_active: Hsla,

    // Modal backdrop
    pub overlay: Hsla,
}

impl Colors {
    pub fn dark() -> Self {
        Self {
            // Backgrounds - Zed-style ultra-dark IDE palette
            bg_primary: rgb(0x0d0d0f).into(),   // app bg
            bg_secondary: rgb(0x111113).into(), // sidebar bg
            bg_tertiary: rgb(0x131315).into(),  // panel bg (request/response)
            bg_elevated: rgb(0x1b1b1e).into(),  // input/elevated bg

            // Text
            text_primary: rgb(0xe4e4ed).into(),
            text_secondary: rgb(0x7f7f92).into(),
            text_muted: rgb(0x3e3e4a).into(),

            // Borders
            border: rgb(0x252529).into(),
            border_focused: rgb(0x4ade80).into(),

            // Accent - green like Zed
            accent: rgb(0x4ade80).into(),
            accent_hover: rgb(0x6ee7a0).into(),

            // HTTP Methods - design spec colors
            method_get: rgb(0x4ade80).into(),     // green
            method_post: rgb(0x60a5fa).into(),    // blue
            method_put: rgb(0xfbbf24).into(),     // yellow
            method_patch: rgb(0xfb923c).into(),   // orange
            method_delete: rgb(0xf87171).into(),  // red
            method_head: rgb(0xa78bfa).into(),    // purple
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
            hover_overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 0.035,
            },
            active_overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 0.055,
            },
            selected_bg: Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 0.055,
            },

            // Focus indicator
            focus_ring: rgb(0x4ade80).into(),
            focus_ring_error: rgb(0xf87171).into(),

            // Collaboration / sync
            team_accent: rgb(0x4ade80).into(), // green, matches accent
            sync_active: rgb(0x22c55e).into(), // brighter green for active state

            // Modal backdrop
            overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5,
            },
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
            hover_overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.04,
            },
            active_overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.08,
            },
            selected_bg: Hsla {
                h: 203.0 / 360.0,
                s: 1.0,
                l: 0.4,
                a: 0.15,
            },

            // Focus indicator
            focus_ring: rgb(0x007acc).into(),
            focus_ring_error: rgb(0xc62828).into(),

            // Collaboration / sync
            team_accent: rgb(0x007acc).into(), // blue, matches accent
            sync_active: rgb(0x008000).into(), // darker green for active

            // Modal backdrop
            overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.4,
            },
        }
    }
}

/// User-selectable theme mode. `System` tracks the OS light/dark setting;
/// `Light`/`Dark` pin the theme regardless of OS setting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

const THEME_MODE_PREF_KEY: &str = "theme.mode";

impl ThemeMode {
    fn as_str(self) -> &'static str {
        match self {
            ThemeMode::System => "system",
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "light" => ThemeMode::Light,
            "dark" => ThemeMode::Dark,
            _ => ThemeMode::System,
        }
    }

    fn load() -> Self {
        crate::prefs::get_string(THEME_MODE_PREF_KEY)
            .map(|s| Self::from_str(&s))
            .unwrap_or(ThemeMode::System)
    }

    fn save(self) {
        crate::prefs::set_string(THEME_MODE_PREF_KEY, self.as_str());
    }

    /// Cycle System -> Light -> Dark -> System.
    fn next(self) -> Self {
        match self {
            ThemeMode::System => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::System,
        }
    }
}

/// Current theme colors (will follow system preference)
#[derive(Clone)]
pub struct Theme {
    pub colors: Colors,
    pub is_dark: bool,
    pub mode: ThemeMode,

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
            mode: ThemeMode::System,
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
            mode: ThemeMode::System,
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
    pub xs: Pixels,   // 4px - tight spacing
    pub sm: Pixels,   // 8px - small spacing
    pub md: Pixels,   // 12px - medium spacing
    pub base: Pixels, // 16px - standard spacing
    pub lg: Pixels,   // 24px - large spacing
    pub xl: Pixels,   // 32px - extra large spacing
    pub xxl: Pixels,  // 48px - extra extra large spacing
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
    pub xs: Pixels,   // 10px - tiny text
    pub sm: Pixels,   // 12px - small text
    pub base: Pixels, // 13px - body text
    pub md: Pixels,   // 14px - medium text
    pub lg: Pixels,   // 15px - large text
    pub xl: Pixels,   // 16px - extra large text
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

/// Raw dimension constants - use these in components that lack `cx`.
/// All interactive components of the same tier should share these values.
pub mod sizes {
    pub const INPUT_XS: f32 = 24.0; // inline / extra-small input
    pub const INPUT_SM: f32 = 28.0; // compact input, small button, compact row
    pub const INPUT_MD: f32 = 32.0; // standard input, medium button, standard row
    pub const INPUT_LG: f32 = 36.0; // large input, large button
    pub const PANEL_HEADER: f32 = 32.0; // section / collapsible headers
    pub const TOOLBAR: f32 = 40.0; // toolbars, tab bars, nav bars
    pub const URL_BAR: f32 = 64.0; // primary URL bar
    /// Gap between the expand/collapse chevron and the item icon in tree rows.
    pub const CHEVRON_ICON_GAP: f32 = 4.0;
    /// Gap between an icon and its adjacent label/badge in tree rows.
    /// Adjust this single value to switch between Compact and Spacious density.
    pub const ICON_TEXT_GAP: f32 = 8.0;
}

/// Pixel-typed component dimensions for use via `theme.sizes.*`.
#[derive(Clone, Copy)]
pub struct ComponentSizes {
    pub input_xs: Pixels,     // 24px - inline / extra-small input
    pub input_sm: Pixels,     // 28px - compact input / small button / compact row
    pub input_md: Pixels,     // 32px - standard input / medium button / standard row
    pub input_lg: Pixels,     // 36px - large input / large button
    pub panel_header: Pixels, // 32px - section / collapsible headers
    pub toolbar: Pixels,      // 40px - toolbars, tab bars, nav bars
    pub url_bar: Pixels,      // 64px - primary URL bar
}

impl ComponentSizes {
    pub fn new() -> Self {
        use crate::theme::sizes;
        use gpui::px;
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
    pub sm: Pixels, // 4px - subtle rounding
    pub md: Pixels, // 6px - standard rounding
    pub lg: Pixels, // 8px - pronounced rounding
    pub xl: Pixels, // 12px - large rounding
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
    pub disabled: f32, // 0.4 - disabled elements
    pub muted: f32,    // 0.6 - muted text
    pub hover: f32,    // 0.08 - hover overlay
    pub pressed: f32,  // 0.12 - pressed/active overlay
    pub selected: f32, // 0.2 - selected background
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

fn theme_for_appearance(appearance: gpui::WindowAppearance, mode: ThemeMode) -> Theme {
    let mut theme = match mode {
        ThemeMode::Light => Theme::light(),
        ThemeMode::Dark => Theme::dark(),
        ThemeMode::System => match appearance {
            gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight => Theme::light(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => Theme::dark(),
        },
    };
    theme.mode = mode;
    theme
}

/// Initialize theme based on the persisted user preference (defaulting to
/// following the system light/dark setting).
///
/// `App::window_appearance()` reads the OS-level light/dark setting directly from
/// the platform (no `Window` required), matching how Zed's `SystemAppearance::init`
/// bootstraps its theme before any window exists.
///
/// On Linux this value comes from an async xdg-desktop-portal D-Bus query that
/// hasn't resolved yet at this point in startup, so it reports the platform's
/// hardcoded `Light` default rather than the real setting. Callers must also
/// call [`sync_with_window`] once a window exists (and keep it live via
/// `observe_window_appearance`) to pick up the real value and future changes.
pub fn init(cx: &mut App) {
    let theme = theme_for_appearance(cx.window_appearance(), ThemeMode::load());
    cx.set_global(theme);
}

/// Re-sync the global theme from a window's current appearance.
///
/// Call this from `observe_window_appearance` so the theme tracks the OS
/// setting after `init()` (whose reading can be stale, see above) and whenever
/// the user changes their system theme at runtime. A no-op when the user has
/// pinned an explicit Light/Dark mode.
pub fn sync_with_window(window: &gpui::Window, cx: &mut App) {
    let mode = current(cx).mode;
    if mode != ThemeMode::System {
        return;
    }
    cx.set_global(theme_for_appearance(window.appearance(), mode));
}

/// Explicitly select a theme mode (System/Light/Dark), persist the choice,
/// and apply it immediately.
pub fn set_mode(mode: ThemeMode, window: &gpui::Window, cx: &mut App) {
    mode.save();
    cx.set_global(theme_for_appearance(window.appearance(), mode));
}

/// Cycle the theme mode (System -> Light -> Dark -> System) and apply it.
///
/// Cycles the *mode* itself, not the resolved colors - so the button's icon
/// (keyed off `theme.mode`, not `theme.is_dark`) changes on every click even
/// when System currently resolves to the same colors as Dark.
pub fn toggle(window: &gpui::Window, cx: &mut App) {
    set_mode(current(cx).mode.next(), window, cx);
}

/// Get the current theme
pub fn current<C: gpui::AppContext>(cx: &C) -> Theme {
    cx.read_global::<Theme, _>(|theme, _| theme.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::WindowAppearance;

    const ALL_MODES: [ThemeMode; 3] = [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark];

    #[test]
    fn mode_survives_a_persistence_round_trip() {
        for mode in ALL_MODES {
            assert_eq!(ThemeMode::from_str(mode.as_str()), mode);
        }
    }

    #[test]
    fn an_unrecognised_persisted_mode_falls_back_to_system() {
        // A prefs file hand-edited, corrupted, or written by a build that knew
        // about a mode this one does not must not pin a wrong theme.
        for s in ["", "System", "DARK", "sepia", " light", "null"] {
            assert_eq!(
                ThemeMode::from_str(s),
                ThemeMode::System,
                "unexpected mode for {s:?}"
            );
        }
    }

    #[test]
    fn cycling_the_mode_visits_every_mode_and_returns_to_the_start() {
        let mut seen = vec![ThemeMode::System];
        let mut mode = ThemeMode::System;
        for _ in 0..3 {
            mode = mode.next();
            seen.push(mode);
        }
        assert_eq!(mode, ThemeMode::System, "three steps must close the cycle");
        assert_eq!(
            seen,
            vec![
                ThemeMode::System,
                ThemeMode::Light,
                ThemeMode::Dark,
                ThemeMode::System
            ]
        );
    }

    #[test]
    fn a_pinned_mode_ignores_the_system_appearance() {
        for appearance in [
            WindowAppearance::Light,
            WindowAppearance::VibrantLight,
            WindowAppearance::Dark,
            WindowAppearance::VibrantDark,
        ] {
            assert!(!theme_for_appearance(appearance, ThemeMode::Light).is_dark);
            assert!(theme_for_appearance(appearance, ThemeMode::Dark).is_dark);
        }
    }

    #[test]
    fn system_mode_follows_the_window_appearance() {
        for (appearance, want_dark) in [
            (WindowAppearance::Light, false),
            (WindowAppearance::VibrantLight, false),
            (WindowAppearance::Dark, true),
            (WindowAppearance::VibrantDark, true),
        ] {
            assert_eq!(
                theme_for_appearance(appearance, ThemeMode::System).is_dark,
                want_dark,
                "wrong resolution for {appearance:?}"
            );
        }
    }

    #[test]
    fn the_resolved_theme_remembers_which_mode_produced_it() {
        // The titlebar icon is keyed off `mode`, not `is_dark`, so a resolved
        // theme that forgot its mode would freeze the toggle button's icon.
        for mode in ALL_MODES {
            assert_eq!(
                theme_for_appearance(WindowAppearance::Dark, mode).mode,
                mode
            );
        }
    }

    #[test]
    fn every_http_method_has_its_own_colour() {
        let theme = Theme::dark();
        let methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
        for (i, a) in methods.iter().enumerate() {
            for b in &methods[i + 1..] {
                assert_ne!(
                    theme.method_color(a),
                    theme.method_color(b),
                    "{a} and {b} must be distinguishable"
                );
            }
        }
    }

    #[test]
    fn method_colours_are_case_insensitive() {
        let theme = Theme::dark();
        for m in ["get", "Get", "gEt"] {
            assert_eq!(theme.method_color(m), theme.colors.method_get, "{m}");
        }
        assert_eq!(
            theme.method_color("websocket"),
            theme.colors.protocol_ws,
            "lowercase protocol labels must resolve too"
        );
    }

    #[test]
    fn protocol_labels_map_to_their_protocol_colour() {
        let theme = Theme::dark();
        for label in ["WS", "WEBSOCKET", "SIO"] {
            assert_eq!(
                theme.method_color(label),
                theme.colors.protocol_ws,
                "{label}"
            );
        }
        for label in ["GRPC", "TRPC"] {
            assert_eq!(
                theme.method_color(label),
                theme.colors.protocol_grpc,
                "{label}"
            );
        }
        for label in ["GQL", "GRAPHQL"] {
            assert_eq!(
                theme.method_color(label),
                theme.colors.protocol_graphql,
                "{label}"
            );
        }
    }

    #[test]
    fn an_unknown_method_gets_the_neutral_colour_instead_of_panicking() {
        let theme = Theme::dark();
        // Empty, non-ASCII and oversized labels all reach method_color from
        // imported collections and .http files, which are user-authored.
        for label in ["", "PURGE", "日本語", "🎉", "a".repeat(4096).as_str()] {
            assert_eq!(theme.method_color(label), theme.colors.text_secondary);
        }
    }

    #[test]
    fn status_colours_cover_every_class_boundary() {
        let theme = Theme::dark();
        for (status, want) in [
            (200, theme.colors.status_success),
            (299, theme.colors.status_success),
            (300, theme.colors.status_redirect),
            (399, theme.colors.status_redirect),
            (400, theme.colors.status_client_error),
            (499, theme.colors.status_client_error),
            (500, theme.colors.status_server_error),
            (599, theme.colors.status_server_error),
        ] {
            assert_eq!(theme.status_color(status), want, "status {status}");
        }
    }

    #[test]
    fn statuses_outside_the_known_classes_get_the_neutral_colour() {
        let theme = Theme::dark();
        // 0 is Protide's "no response yet / transport error" sentinel; 1xx and
        // 6xx+ are legal-but-unclassified codes a server can still send.
        for status in [0, 1, 100, 199, 600, 999, u16::MAX] {
            assert_eq!(
                theme.status_color(status),
                theme.colors.text_secondary,
                "status {status}"
            );
        }
    }

    #[test]
    fn the_light_and_dark_palettes_are_not_swapped() {
        let (light, dark) = (Theme::light(), Theme::dark());
        assert!(!light.is_dark);
        assert!(dark.is_dark);
        assert!(
            light.colors.bg_primary.l > dark.colors.bg_primary.l,
            "the light theme's background must be lighter than the dark theme's"
        );
        assert!(
            light.colors.text_primary.l < dark.colors.text_primary.l,
            "the light theme's text must be darker than the dark theme's"
        );
    }

    #[test]
    fn both_palettes_separate_foreground_from_background() {
        // Not a contrast-ratio check (appearance is out of scope) - just a guard
        // against a palette edit that leaves text invisible on its own bg.
        for theme in [Theme::light(), Theme::dark()] {
            assert!(
                (theme.colors.text_primary.l - theme.colors.bg_primary.l).abs() > 0.3,
                "text_primary must be clearly separated from bg_primary"
            );
        }
    }
}
