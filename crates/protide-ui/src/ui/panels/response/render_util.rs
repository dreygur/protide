use super::*;

impl ResponsePanel {
    pub(super) fn run_extraction(&mut self, cx: &mut Context<Self>) {
        let Some(response) = &self.response else {
            return;
        };

        let jsonpath = self.jsonpath_input.read(cx).get_text().to_string();
        if jsonpath.is_empty() {
            self.extraction_result = Some(Err("Enter a JSONPath expression".to_string()));
            cx.notify();
            return;
        }

        let result = chaining::extract_jsonpath(&response.body, &jsonpath);
        if let Err(ref e) = result {
            warn!("JSONPath '{}' extraction failed: {}", jsonpath, e);
        }
        if let Ok(ref value) = result {
            let (content, lang) = if value.trim().starts_with('{') || value.trim().starts_with('[') {
                let pretty = serde_json::from_str::<serde_json::Value>(value)
                    .ok()
                    .and_then(|v| serde_json::to_string_pretty(&v).ok())
                    .unwrap_or_else(|| value.clone());
                (pretty, Language::Json)
            } else {
                (value.clone(), Language::Json)
            };
            self.extraction_editor.update(cx, |editor, cx| {
                editor.set_content(&content, cx);
                editor.set_language(lang, cx);
            });
        }
        self.extraction_result = Some(result);
        cx.notify();
    }

    pub(super) fn render_json_context_menu(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let Some(ref menu) = self.json_context_menu else {
            return div().into_any_element();
        };
        let theme = theme::current(cx);
        let items = menu.items.clone();
        let x = menu.x;
        let y = menu.y;

        div()
            .id("json-cm-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.json_context_menu = None;
                cx.notify();
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(|this, _, _, cx| {
                this.json_context_menu = None;
                cx.notify();
            }))
            .child({
                let hover_bg = theme.colors.bg_tertiary;
                let mut menu_box = div()
                    .id("json-cm-box")
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .w(px(175.0))
                    .bg(theme.colors.bg_elevated)
                    .border_1()
                    .border_color(theme.colors.border);
                for (i, (label, value)) in items.into_iter().enumerate() {
                    let label_s = SharedString::from(label);
                    menu_box = menu_box.child(
                        div()
                            .id(i)
                            .w_full()
                            .px(px(12.0))
                            .py(px(8.0))
                            .text_size(px(12.0))
                            .text_color(theme.colors.text_primary)
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover_bg))
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.clone()));
                            })
                            .child(label_s),
                    );
                }
                menu_box
            })
            .into_any_element()
    }

    pub(super) fn render_col_drag_handle(&self, drag_id: u8, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let col_w = match drag_id {
            0 => self.resp_header_col1_w,
            1 => self.cookie_col1_w,
            2 => self.cookie_col3_w,
            3 => self.cookie_col4_w,
            _ => 0.0,
        };
        div()
            .id(SharedString::from(format!("col-drag-handle-{}", drag_id)))
            .w(px(4.0))
            .self_stretch()
            .cursor_col_resize()
            .bg(theme.colors.border.opacity(0.3))
            .hover(|s| s.bg(theme.colors.accent.opacity(0.5)))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                this.resp_col_drag = Some((drag_id, f32::from(event.position.x), col_w));
                cx.notify();
            }))
    }
}
