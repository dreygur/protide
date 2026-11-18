use gpui::Pixels;

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
