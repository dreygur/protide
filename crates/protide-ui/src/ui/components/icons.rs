//! Icon helper — wraps GPUI svg() with a fixed size and color.
//! Icons come from gpui-component-assets (Lucide icon set).

use gpui::{svg, px, IntoElement, Hsla, Styled};

pub const ICON_SM: f32 = 11.0;
pub const ICON_MD: f32 = 13.0;
pub const ICON_LG: f32 = 15.0;

/// Render a Lucide SVG icon at the given size and color.
pub fn icon(path: &'static str, size: f32, color: Hsla) -> impl IntoElement {
    svg()
        .path(path)
        .size(px(size))
        .text_color(color)
}

// ── Path constants ─────────────────────────────────────────────────────────
pub const ICON_CLOSE:          &str = "icons/close.svg";
pub const ICON_CHECK:          &str = "icons/check.svg";
pub const ICON_CIRCLE_CHECK:   &str = "icons/circle-check.svg";
pub const ICON_CIRCLE_X:       &str = "icons/circle-x.svg";
pub const ICON_CHEVRON_DOWN:   &str = "icons/chevron-down.svg";
pub const ICON_CHEVRON_UP:     &str = "icons/chevron-up.svg";
pub const ICON_CHEVRON_LEFT:   &str = "icons/chevron-left.svg";
pub const ICON_CHEVRON_RIGHT:  &str = "icons/chevron-right.svg";
pub const ICON_ARROW_DOWN:     &str = "icons/arrow-down.svg";
pub const ICON_ARROW_UP:       &str = "icons/arrow-up.svg";
pub const ICON_ARROW_LEFT:     &str = "icons/arrow-left.svg";
pub const ICON_ARROW_RIGHT:    &str = "icons/arrow-right.svg";
pub const ICON_MENU:           &str = "icons/menu.svg";
pub const ICON_SETTINGS:       &str = "icons/settings.svg";
pub const ICON_COPY:           &str = "icons/copy.svg";
pub const ICON_FILE:           &str = "icons/file.svg";
pub const ICON_FOLDER:         &str = "icons/folder.svg";
pub const ICON_FOLDER_OPEN:    &str = "icons/folder-open.svg";
pub const ICON_DELETE:         &str = "icons/delete.svg";
pub const ICON_USER:           &str = "icons/user.svg";
pub const ICON_INFO:           &str = "icons/info.svg";
pub const ICON_PLAY:           &str = "icons/play.svg";
pub const ICON_PAUSE:          &str = "icons/pause.svg";
pub const ICON_REFRESH:        &str = "icons/redo-2.svg";
pub const ICON_LOADER:         &str = "icons/loader-circle.svg";
pub const ICON_SEARCH:         &str = "icons/search.svg";
pub const ICON_PLUS:           &str = "icons/plus.svg";
pub const ICON_MINUS:          &str = "icons/minus.svg";
pub const ICON_MAXIMIZE:       &str = "icons/window-maximize.svg";
pub const ICON_MINIMIZE:       &str = "icons/window-minimize.svg";
pub const ICON_WINDOW_CLOSE:   &str = "icons/window-close.svg";
pub const ICON_SAVE:           &str = "icons/arrow-down.svg";
pub const ICON_EXTERNAL:       &str = "icons/external-link.svg";
pub const ICON_GLOBE:          &str = "icons/globe.svg";
pub const ICON_EDIT:           &str = "icons/replace.svg";
pub const ICON_KEY:            &str = "icons/eye.svg";
pub const ICON_FORM:           &str = "icons/layout-dashboard.svg";
pub const ICON_BEARER:         &str = "icons/circle-user.svg";
pub const ICON_TIMER:          &str = "icons/loader-circle.svg";
pub const ICON_STAR:           &str = "icons/star.svg";
pub const ICON_ELLIPSIS:       &str = "icons/ellipsis.svg";
