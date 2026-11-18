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

mod colors;
mod tokens;

pub use colors::Colors;
pub use tokens::{BorderRadius, ComponentSizes, Opacity, Spacing, Typography, sizes};

use gpui::{App, Hsla};

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
