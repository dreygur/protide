//! Test-only helpers shared across `protide-ui` panel tests.

use gpui::TestAppContext;

/// Install the globals every rendered panel reads from.
///
/// `gpui_component::Theme` and `crate::theme` are process-globals on the
/// `TestAppContext`, so each `#[gpui::test]` must seed them before building a
/// window or `render()` panics on a missing global.
pub fn init_theme(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(gpui_component::Theme::default());
        crate::theme::init(cx);
    });
}
