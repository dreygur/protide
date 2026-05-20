/// ActionRow: a reusable row with hover-revealed ghost action buttons.
///
/// Hover visibility is implemented with GPUI's `.group()` / `.group_hover()`
/// mechanism - action icons use no `on_scroll_wheel` or `on_mouse_move`
/// handlers, so they never intercept scroll events.  Only `on_click` on the
/// action buttons calls `cx.stop_propagation()` to block the row click.
use gpui::{
    AnyElement, App, ClickEvent, ElementId, Hsla, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Pixels, SharedString, Window, div, prelude::*, px,
};

use crate::theme;

pub struct ActionRow {
    id: ElementId,
    group: SharedString,
    height: Pixels,
    selected: bool,
    accent: Hsla,
    bg_tertiary: Hsla,
    /// Ghost action elements revealed at full opacity when the row is hovered.
    actions: Vec<AnyElement>,
    /// Main content elements.
    content: Vec<AnyElement>,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    on_right_click: Option<Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>>,
}

impl ActionRow {
    /// Create a new ActionRow.
    ///
    /// `group` must be unique per row - use the item index or a stable key.
    pub fn new(
        id: impl Into<ElementId>,
        group: impl Into<SharedString>,
        theme: &theme::Theme,
    ) -> Self {
        Self {
            id: id.into(),
            group: group.into(),
            height: px(28.0),
            selected: false,
            accent: theme.colors.accent,
            bg_tertiary: theme.colors.bg_tertiary,
            actions: vec![],
            content: vec![],
            on_click: None,
            on_right_click: None,
        }
    }

    pub fn height(mut self, h: Pixels) -> Self {
        self.height = h;
        self
    }

    pub fn selected(mut self, v: bool) -> Self {
        self.selected = v;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn on_right_click(
        mut self,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_right_click = Some(Box::new(handler));
        self
    }

    /// Append a ghost action element (shown on hover via group_hover).
    pub fn action(mut self, el: impl IntoElement) -> Self {
        self.actions.push(el.into_any_element());
        self
    }

    /// Append a main content element.
    pub fn child(mut self, el: impl IntoElement) -> Self {
        self.content.push(el.into_any_element());
        self
    }
}

impl IntoElement for ActionRow {
    type Element = AnyElement;

    fn into_element(self) -> AnyElement {
        let group = self.group.clone();
        let has_actions = !self.actions.is_empty();
        let accent = self.accent;
        let bg_tertiary = self.bg_tertiary;

        div()
            .id(self.id)
            .group(group.clone())
            // relative so absolute-positioned guide lines resolve against this row
            .relative()
            .w_full()
            .h(self.height)
            .flex()
            .items_center()
            .cursor_pointer()
            .when(self.selected, |el| el.bg(accent.opacity(0.1)))
            .when(!self.selected, |el| el.hover(|s| s.bg(bg_tertiary)))
            .when_some(self.on_click, |el, h| el.on_click(h))
            .when_some(self.on_right_click, |el, h| {
                el.on_mouse_down(MouseButton::Right, h)
            })
            .children(self.content)
            // Action container: always in layout (preserves right-edge spacing),
            // invisible normally, visible when the row group is hovered.
            // No scroll/wheel handlers - zero interference with parent scrollers.
            .when(has_actions, |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(1.0))
                        .pr(px(2.0))
                        .invisible()
                        .group_hover(group, |s| s.visible())
                        .children(self.actions),
                )
            })
            .into_any_element()
    }
}
