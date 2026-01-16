//! Button component with consistent styling and variants
//!
//! Provides standardized button styles following the design system:
//! - Primary: Main actions (accent color)
//! - Secondary: Secondary actions (subtle background)
//! - Ghost: Minimal styling, transparent background
//! - Danger: Destructive actions (error color)

use gpui::prelude::*;
use gpui::*;
use crate::theme;

/// Button variant types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// Button size options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonSize {
    Small,   // 28px height
    Medium,  // 32px height
    Large,   // 36px height
}

/// Button styling helper functions
pub struct ButtonStyles;

impl ButtonStyles {
    /// Get button height based on size
    pub fn height(size: ButtonSize) -> Pixels {
        match size {
            ButtonSize::Small => px(28.0),
            ButtonSize::Medium => px(32.0),
            ButtonSize::Large => px(36.0),
        }
    }

    /// Get horizontal padding based on size
    pub fn padding_x(size: ButtonSize) -> Pixels {
        match size {
            ButtonSize::Small => px(12.0),
            ButtonSize::Medium => px(16.0),
            ButtonSize::Large => px(20.0),
        }
    }

    /// Get font size based on button size
    pub fn font_size(size: ButtonSize, theme: &theme::Theme) -> Pixels {
        match size {
            ButtonSize::Small => theme.typography.sm,
            ButtonSize::Medium => theme.typography.base,
            ButtonSize::Large => theme.typography.md,
        }
    }

    /// Get background color for button variant
    pub fn background_color(variant: ButtonVariant, theme: &theme::Theme) -> Hsla {
        match variant {
            ButtonVariant::Primary => theme.colors.accent,
            ButtonVariant::Secondary => theme.colors.bg_tertiary,
            ButtonVariant::Ghost => Hsla::transparent_black(),
            ButtonVariant::Danger => theme.colors.error,
        }
    }

    /// Get hover background color
    pub fn hover_color(variant: ButtonVariant, theme: &theme::Theme) -> Hsla {
        match variant {
            ButtonVariant::Primary => theme.colors.accent_hover,
            ButtonVariant::Secondary => theme.colors.bg_elevated,
            ButtonVariant::Ghost => theme.colors.hover_overlay,
            ButtonVariant::Danger => {
                // Darken error color slightly for hover
                let mut color = theme.colors.error;
                color.a = 0.9;
                color
            }
        }
    }

    /// Get text color for button variant
    pub fn text_color(variant: ButtonVariant, theme: &theme::Theme) -> Hsla {
        match variant {
            ButtonVariant::Primary => rgb(0xffffff).into(), // Always white for primary
            ButtonVariant::Secondary => theme.colors.text_primary,
            ButtonVariant::Ghost => theme.colors.text_primary,
            ButtonVariant::Danger => rgb(0xffffff).into(), // Always white for danger
        }
    }

    /// Get border color (for secondary and ghost variants)
    pub fn border_color(variant: ButtonVariant, theme: &theme::Theme) -> Option<Hsla> {
        match variant {
            ButtonVariant::Primary | ButtonVariant::Danger => None,
            ButtonVariant::Secondary => Some(theme.colors.border),
            ButtonVariant::Ghost => Some(Hsla::transparent_black()),
        }
    }

    /// Get border color on hover
    pub fn border_hover_color(variant: ButtonVariant, theme: &theme::Theme) -> Option<Hsla> {
        match variant {
            ButtonVariant::Primary | ButtonVariant::Danger => None,
            ButtonVariant::Secondary => Some(theme.colors.border_focused),
            ButtonVariant::Ghost => Some(theme.colors.border),
        }
    }

    /// Get disabled background color
    pub fn disabled_background(theme: &theme::Theme) -> Hsla {
        let mut color = theme.colors.bg_tertiary;
        color.a = 0.5;
        color
    }

    /// Get disabled text color
    pub fn disabled_text_color(theme: &theme::Theme) -> Hsla {
        let mut color = theme.colors.text_muted;
        color.a = theme.opacity.disabled;
        color
    }
}

/// Create a styled button element
///
/// # Example
/// ```
/// use crate::ui::components::button::*;
///
/// button(ButtonVariant::Primary, ButtonSize::Medium)
///     .child("Click me")
///     .on_click(|_, cx| {
///         // Handle click
///     })
/// ```
pub fn button(variant: ButtonVariant, size: ButtonSize) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .h(ButtonStyles::height(size))
        .px(ButtonStyles::padding_x(size))
        .rounded(px(6.0))
        .cursor_pointer()
        // Styling will be applied via map_some or when_some based on context
}

/// Button builder with fluent API
pub struct Button {
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
}

impl Button {
    pub fn new(variant: ButtonVariant, size: ButtonSize) -> Self {
        Self {
            variant,
            size,
            disabled: false,
        }
    }

    pub fn primary(size: ButtonSize) -> Self {
        Self::new(ButtonVariant::Primary, size)
    }

    pub fn secondary(size: ButtonSize) -> Self {
        Self::new(ButtonVariant::Secondary, size)
    }

    pub fn ghost(size: ButtonSize) -> Self {
        Self::new(ButtonVariant::Ghost, size)
    }

    pub fn danger(size: ButtonSize) -> Self {
        Self::new(ButtonVariant::Danger, size)
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Render the button with the given child content
    pub fn render(self, cx: &mut impl AppContext, content: impl Into<AnyElement>) -> impl IntoElement {
        let theme = theme::current(cx);
        let variant = self.variant;
        let size = self.size;
        let disabled = self.disabled;

        let bg_color = ButtonStyles::background_color(variant, &theme);
        let text_color = ButtonStyles::text_color(variant, &theme);
        let hover_color = ButtonStyles::hover_color(variant, &theme);
        let border_color = ButtonStyles::border_color(variant, &theme);
        let border_hover_color = ButtonStyles::border_hover_color(variant, &theme);

        div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .h(ButtonStyles::height(size))
            .px(ButtonStyles::padding_x(size))
            .rounded(theme.radius.md)
            .text_size(ButtonStyles::font_size(size, &theme))
            .when(!disabled, |el| {
                let mut el = el
                    .bg(bg_color)
                    .text_color(text_color)
                    .cursor_pointer()
                    .hover(|style| style.bg(hover_color));

                if let Some(border) = border_color {
                    el = el.border_1().border_color(border);
                    if let Some(hover_border) = border_hover_color {
                        el = el.hover(|style| style.border_color(hover_border));
                    }
                }
                el
            })
            .when(disabled, |el| {
                el.bg(ButtonStyles::disabled_background(&theme))
                    .text_color(ButtonStyles::disabled_text_color(&theme))
                    .cursor_not_allowed()
            })
            .child(content.into())
    }
}
