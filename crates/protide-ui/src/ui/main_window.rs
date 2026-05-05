//! Main application window

use std::path::PathBuf;

use gpui::{
    App, Context, Entity, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyBinding,
    MouseButton, ParentElement, Render, SharedString, Styled, WeakEntity, Window, div, prelude::*,
    px,
};

gpui::actions!(
    main_window,
    [
        SendRequest,
        SaveRequest,
        ToggleSidebar,
        ToggleMockServer,
        ShowHelp,
        ShowAbout,
        DismissOverlay,
        Quit
    ]
);

pub fn register_keybindings(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-enter", SendRequest, None),
        KeyBinding::new("ctrl-s", SaveRequest, None),
        KeyBinding::new("ctrl-b", ToggleSidebar, None),
        KeyBinding::new("ctrl-shift-m", ToggleMockServer, None),
        KeyBinding::new("f1", ShowHelp, None),
        KeyBinding::new("ctrl-shift-a", ShowAbout, None),
        KeyBinding::new("escape", DismissOverlay, None),
        KeyBinding::new("ctrl-q", Quit, None),
    ]);
}

use super::panels::{ExplorerPanel, MockServerPanel, RequestPanel, ResponsePanel};
use crate::theme;
use crate::ui::components::icons::{
    ICON_CLOSE, ICON_COPY, ICON_FOLDER, ICON_MAXIMIZE, ICON_MD, ICON_MENU, ICON_MINIMIZE,
    ICON_REFRESH, ICON_SETTINGS, ICON_SM, ICON_WINDOW_CLOSE, icon,
};
use crate::ui::components::modal::{ModalKind, ModalState, render_modal_shell};

/// Pending action for confirm modals
#[derive(Clone, Debug, Default)]
enum ModalPending {
    #[default]
    None,
    ExplorerDelete(PathBuf),
}

/// Main window containing the application layout
pub struct MainWindow {
    explorer: Entity<ExplorerPanel>,
    request_panel: Entity<RequestPanel>,
    response_panel: Entity<ResponsePanel>,
    mock_server_panel: Entity<MockServerPanel>,
    show_mock_server: bool,
    sidebar_collapsed: bool,
    sidebar_width: f32,
    request_height: f32,
    mock_server_width: f32,
    codegen_panel_width: f32,
    drag_sidebar: Option<(f32, f32)>,
    drag_response: Option<(f32, f32)>,
    drag_mock_server: Option<(f32, f32)>,
    drag_codegen: Option<(f32, f32)>,
    modal: Option<ModalState>,
    modal_pending: ModalPending,
    show_help: bool,
    show_about: bool,
    focus: FocusHandle,
    /// Which title-bar menu is open (0=Protide, 1=Request, 2=View, 3=Help)
    open_menu: Option<u8>,
}

impl MainWindow {
    pub fn build(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let main_window_weak: WeakEntity<MainWindow> = cx.entity().downgrade();
        let explorer = cx.new(|cx| ExplorerPanel::new(cx, main_window_weak.clone()));
        let response_panel = cx.new(|cx| ResponsePanel::new(cx));
        let response_panel_clone = response_panel.clone();
        let request_panel = cx.new(|cx| RequestPanel::new(cx, response_panel_clone));
        let mock_server_panel = cx.new(|cx| MockServerPanel::new(cx, main_window_weak));

        // Connect explorer to request panel for history loading
        let request_panel_clone = request_panel.clone();
        explorer.update(cx, |explorer, cx| {
            explorer.set_request_panel(request_panel_clone, cx);
        });

        // Connect request panel to explorer for environment variable substitution
        let explorer_clone = explorer.clone();
        request_panel.update(cx, |panel, cx| {
            panel.set_explorer_panel(explorer_clone, cx);
        });

        Self {
            explorer,
            request_panel,
            response_panel,
            mock_server_panel,
            show_mock_server: false,
            sidebar_collapsed: false,
            sidebar_width: crate::prefs::get_f32("main.sidebar_width", 250.0),
            request_height: crate::prefs::get_f32("main.request_height", 320.0),
            mock_server_width: crate::prefs::get_f32("main.mock_server_width", 320.0),
            codegen_panel_width: crate::prefs::get_f32("main.codegen_panel_width", 400.0),
            drag_sidebar: None,
            drag_response: None,
            drag_mock_server: None,
            drag_codegen: None,
            modal: None,
            modal_pending: ModalPending::None,
            show_help: false,
            show_about: false,
            focus: cx.focus_handle(),
            open_menu: None,
        }
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }

    fn toggle_mock_server(&mut self, cx: &mut Context<Self>) {
        self.show_mock_server = !self.show_mock_server;
        cx.notify();
    }

    pub fn show_modal(&mut self, state: ModalState, cx: &mut Context<Self>) {
        self.modal = Some(state);
        self.modal_pending = ModalPending::None;
        cx.notify();
    }

    pub fn show_confirm_delete(
        &mut self,
        state: ModalState,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.modal = Some(state);
        self.modal_pending = ModalPending::ExplorerDelete(path);
        cx.notify();
    }

    fn dismiss_modal(&mut self, cx: &mut Context<Self>) {
        self.modal = None;
        self.modal_pending = ModalPending::None;
        cx.notify();
    }

    fn dismiss_overlay(&mut self, cx: &mut Context<Self>) {
        if self.modal.is_some() {
            self.modal = None;
            self.modal_pending = ModalPending::None;
        } else if self.show_help {
            self.show_help = false;
        } else if self.show_about {
            self.show_about = false;
        }
        cx.notify();
    }

    fn confirm_modal_action(&mut self, cx: &mut Context<Self>) {
        let pending = std::mem::replace(&mut self.modal_pending, ModalPending::None);
        self.modal = None;
        match pending {
            ModalPending::ExplorerDelete(path) => {
                self.explorer
                    .update(cx, |panel, cx| panel.execute_delete(path, cx));
            }
            ModalPending::None => {}
        }
        cx.notify();
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let show_codegen = self.request_panel.read(cx).codegen_content.is_some();
        let is_dragging = self.drag_sidebar.is_some()
            || self.drag_response.is_some()
            || self.drag_mock_server.is_some()
            || self.drag_codegen.is_some();
        let is_col_drag = self.drag_sidebar.is_some()
            || self.drag_mock_server.is_some()
            || self.drag_codegen.is_some();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_primary)
            .text_color(theme.colors.text_primary)
            .track_focus(&self.focus)
            .key_context("MainWindow")
            .on_action(cx.listener(|this, _: &SendRequest, _, cx| {
                this.request_panel.update(cx, |p, cx| p.send_request(cx));
            }))
            .on_action(cx.listener(|this, _: &SaveRequest, _, cx| {
                this.request_panel.update(cx, |p, cx| p.save_request(cx));
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.toggle_sidebar(cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleMockServer, _, cx| {
                this.toggle_mock_server(cx);
            }))
            .on_action(cx.listener(|this, _: &ShowHelp, _, cx| {
                this.show_help = true;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ShowAbout, _, cx| {
                this.show_about = true;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &DismissOverlay, _, cx| {
                this.dismiss_overlay(cx);
            }))
            .on_action(|_: &Quit, _, cx: &mut App| {
                cx.quit();
            })
            .child(self.render_title_bar(cx))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    // Collapsed sidebar strip
                    .when(self.sidebar_collapsed, |el| {
                        el.child(
                            div()
                                .id("sidebar-collapsed-strip")
                                .w(px(32.0))
                                .h_full()
                                .flex_shrink_0()
                                .bg(theme.colors.bg_secondary)
                                .border_r_1()
                                .border_color(theme.colors.border)
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(2.0))
                                .pt(px(8.0))
                                // Hamburger: expand sidebar
                                .child(
                                    div()
                                        .id("collapse-toggle")
                                        .w(px(28.0))
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.toggle_sidebar(cx);
                                        }))
                                        .child(icon(ICON_MENU, ICON_MD, theme.colors.text_muted)),
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(1.0))
                                        .bg(theme.colors.border)
                                        .mx_auto()
                                        .mt(px(2.0))
                                        .mb(px(2.0)),
                                )
                                // Collections icon
                                .child({
                                    let explorer = self.explorer.clone();
                                    div()
                                        .id("collapsed-collections")
                                        .w(px(28.0))
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.toggle_sidebar(cx);
                                            explorer.update(cx, |p, cx| {
                                                p.expand_section_collections(cx)
                                            });
                                        }))
                                        .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_muted))
                                })
                                // History icon
                                .child({
                                    let explorer = self.explorer.clone();
                                    div()
                                        .id("collapsed-history")
                                        .w(px(28.0))
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.toggle_sidebar(cx);
                                            explorer
                                                .update(cx, |p, cx| p.expand_section_history(cx));
                                        }))
                                        .child(icon(ICON_REFRESH, ICON_MD, theme.colors.text_muted))
                                })
                                // Environments icon
                                .child({
                                    let explorer = self.explorer.clone();
                                    div()
                                        .id("collapsed-env")
                                        .w(px(28.0))
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.toggle_sidebar(cx);
                                            explorer.update(cx, |p, cx| p.expand_section_env(cx));
                                        }))
                                        .child(icon(
                                            ICON_SETTINGS,
                                            ICON_MD,
                                            theme.colors.text_muted,
                                        ))
                                }),
                        )
                    })
                    // Full sidebar
                    .when(!self.sidebar_collapsed, |el| {
                        el.child(
                            div()
                                .w(px(self.sidebar_width))
                                .h_full()
                                .flex_shrink_0()
                                .bg(theme.colors.bg_secondary)
                                .overflow_hidden()
                                .child(self.explorer.clone()),
                        )
                        // Sidebar resize handle
                        .child(
                            div()
                                .id("sidebar-resize-handle")
                                .w(px(4.0))
                                .h_full()
                                .flex_shrink_0()
                                .border_l_1()
                                .border_color(theme.colors.border)
                                .cursor_col_resize()
                                .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(
                                        |this, event: &gpui::MouseDownEvent, _window, _cx| {
                                            this.drag_sidebar = Some((
                                                f32::from(event.position.x),
                                                this.sidebar_width,
                                            ));
                                        },
                                    ),
                                ),
                        )
                    })
                    // Main content area
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            // Request panel
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .min_h(px(150.0))
                                    .h(px(self.request_height))
                                    .w_full()
                                    .overflow_hidden()
                                    .child(self.request_panel.clone()),
                            )
                            // Response resize handle
                            .child(
                                div()
                                    .id("response-resize-handle")
                                    .w_full()
                                    .h(px(4.0))
                                    .flex_shrink_0()
                                    .border_t_1()
                                    .border_color(theme.colors.border)
                                    .cursor_row_resize()
                                    .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            |this, event: &gpui::MouseDownEvent, _window, _cx| {
                                                this.drag_response = Some((
                                                    f32::from(event.position.y),
                                                    this.request_height,
                                                ));
                                            },
                                        ),
                                    ),
                            )
                            // Response panel
                            .child(
                                div()
                                    .flex_1()
                                    .min_h(px(150.0))
                                    .w_full()
                                    .overflow_hidden()
                                    .child(self.response_panel.clone()),
                            ),
                    )
                    // Mock Server panel (optional right sidebar)
                    .when(self.show_mock_server, |el| {
                        el
                            // Mock server resize handle (left edge)
                            .child(
                                div()
                                    .id("mock-server-resize-handle")
                                    .w(px(4.0))
                                    .h_full()
                                    .flex_shrink_0()
                                    .border_r_1()
                                    .border_color(theme.colors.border)
                                    .cursor_col_resize()
                                    .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            |this, event: &gpui::MouseDownEvent, _window, _cx| {
                                                this.drag_mock_server = Some((
                                                    f32::from(event.position.x),
                                                    this.mock_server_width,
                                                ));
                                            },
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .w(px(self.mock_server_width))
                                    .h_full()
                                    .flex_shrink_0()
                                    .overflow_hidden()
                                    .child(self.mock_server_panel.clone()),
                            )
                    })
                    // Codegen panel (optional right sidebar)
                    .when(show_codegen, |el| {
                        el.child(
                            div()
                                .id("codegen-resize-handle")
                                .w(px(4.0))
                                .h_full()
                                .flex_shrink_0()
                                .border_l_1()
                                .border_color(theme.colors.border)
                                .cursor_col_resize()
                                .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(
                                        |this, event: &gpui::MouseDownEvent, _window, _cx| {
                                            this.drag_codegen = Some((
                                                f32::from(event.position.x),
                                                this.codegen_panel_width,
                                            ));
                                        },
                                    ),
                                ),
                        )
                        .child(self.render_codegen_panel(cx))
                    })
                    // Drag overlay — captures mouse during resize, must be last child
                    .when(is_dragging, |el| {
                        el.child(
                            div()
                                .id("resize-drag-overlay")
                                .absolute()
                                .top_0()
                                .left_0()
                                .w_full()
                                .h_full()
                                .when(is_col_drag, |el| el.cursor_col_resize())
                                .when(!is_col_drag, |el| el.cursor_row_resize())
                                .on_mouse_move(cx.listener(
                                    |this, event: &gpui::MouseMoveEvent, _window, cx| {
                                        let mouse_x = f32::from(event.position.x);
                                        let mouse_y = f32::from(event.position.y);
                                        if let Some((start_x, start_w)) = this.drag_sidebar {
                                            this.sidebar_width =
                                                (start_w + mouse_x - start_x).max(150.0).min(600.0);
                                            cx.notify();
                                        }
                                        if let Some((start_y, start_h)) = this.drag_response {
                                            this.request_height =
                                                (start_h + mouse_y - start_y).max(150.0).min(800.0);
                                            cx.notify();
                                        }
                                        if let Some((start_x, start_w)) = this.drag_mock_server {
                                            // dragging left edge: moving left increases width
                                            this.mock_server_width = (start_w
                                                - (mouse_x - start_x))
                                                .max(200.0)
                                                .min(700.0);
                                            cx.notify();
                                        }
                                        if let Some((start_x, start_w)) = this.drag_codegen {
                                            // dragging left edge: moving left increases width
                                            this.codegen_panel_width = (start_w
                                                - (mouse_x - start_x))
                                                .max(250.0)
                                                .min(800.0);
                                            cx.notify();
                                        }
                                    },
                                ))
                                .on_mouse_up(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _window, cx| {
                                        if this.drag_sidebar.take().is_some() {
                                            crate::prefs::set_f32(
                                                "main.sidebar_width",
                                                this.sidebar_width,
                                            );
                                        }
                                        if this.drag_response.take().is_some() {
                                            crate::prefs::set_f32(
                                                "main.request_height",
                                                this.request_height,
                                            );
                                        }
                                        if this.drag_mock_server.take().is_some() {
                                            crate::prefs::set_f32(
                                                "main.mock_server_width",
                                                this.mock_server_width,
                                            );
                                        }
                                        if this.drag_codegen.take().is_some() {
                                            crate::prefs::set_f32(
                                                "main.codegen_panel_width",
                                                this.codegen_panel_width,
                                            );
                                        }
                                        cx.notify();
                                    }),
                                ),
                        )
                    }),
            )
            .child(self.render_status_bar(cx))
            // Full-window modal overlay (always on top)
            .when_some(self.modal.clone(), |el, modal| {
                let theme = theme::current(cx);
                let is_confirm = modal.kind == ModalKind::Confirm;
                let buttons = if is_confirm {
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(8.0))
                        .mt(px(4.0))
                        .child(
                            div()
                                .id("modal-cancel")
                                .px(px(20.0))
                                .py(px(8.0))
                                .bg(theme.colors.bg_tertiary)
                                .border_1()
                                .border_color(theme.colors.border)
                                .text_color(theme.colors.text_secondary)
                                .text_size(px(12.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.bg_elevated))
                                .on_click(cx.listener(|this, _, _, cx| this.dismiss_modal(cx)))
                                .child("Cancel"),
                        )
                        .child(
                            div()
                                .id("modal-confirm")
                                .px(px(20.0))
                                .py(px(8.0))
                                .bg(theme.colors.error)
                                .text_color(theme.colors.bg_primary)
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .cursor_pointer()
                                .hover(|s| s.opacity(0.85))
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.confirm_modal_action(cx)),
                                )
                                .child("Delete"),
                        )
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .justify_end()
                        .mt(px(4.0))
                        .child(
                            div()
                                .id("modal-ok")
                                .px(px(24.0))
                                .py(px(8.0))
                                .bg(theme.colors.accent)
                                .text_color(theme.colors.bg_primary)
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.accent_hover))
                                .on_click(cx.listener(|this, _, _, cx| this.dismiss_modal(cx)))
                                .child("OK"),
                        )
                        .into_any_element()
                };
                el.child(render_modal_shell(&modal, &theme, buttons))
            })
            .when(self.open_menu.is_some(), |el| el
                .child(
                    div()
                        .absolute().top_0().left_0().w_full().h_full()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.open_menu = None;
                            cx.notify();
                        }))
                )
                .child(gpui::deferred(self.render_menu_dropdown(cx)).with_priority(10))
            )
            .when(self.show_help, |el| el.child(self.render_help_overlay(cx)))
            .when(self.show_about, |el| {
                el.child(self.render_about_overlay(cx))
            })
    }
}

impl MainWindow {
    fn render_title_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let show_mock = self.show_mock_server;

        div()
            .id("titlebar")
            .h(theme.sizes.toolbar)
            .w_full()
            .flex()
            .items_center()
            .bg(theme.colors.bg_primary)
            .border_b_1()
            .border_color(theme.colors.border)
            // Logo + title (draggable)
            .child(
                div()
                    .id("titlebar-drag")
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .px(px(8.0))
                    .h_full()
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _cx: &mut App| {
                        window.start_window_move();
                    })
                    // Logo badge
                    .child(
                        div()
                            .size(px(18.0))
                            .bg(theme.colors.accent)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.colors.bg_primary)
                                    .child("P"),
                            ),
                    )
                    // Title
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Protide"),
                    ),
            )
            // Menu bar buttons
            .child({
                let open = self.open_menu;
                let menus: &[(u8, &str)] = &[(0, "Protide"), (1, "Request"), (2, "View"), (3, "Help")];
                div()
                    .flex().items_center().h_full()
                    .children(menus.iter().map(|&(id, label)| {
                        let is_open = open == Some(id);
                        div()
                            .id(("menu-btn", id as usize))
                            .h_full().px(px(10.0))
                            .flex().items_center()
                            .cursor_pointer()
                            .text_size(px(12.0))
                            .when(is_open, |el| el
                                .bg(theme.colors.bg_tertiary)
                                .text_color(theme.colors.text_primary)
                            )
                            .when(!is_open, |el| el
                                .text_color(theme.colors.text_secondary)
                                .hover(|s| s.bg(theme.colors.bg_elevated).text_color(theme.colors.text_primary))
                            )
                            .child(label)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_menu = if this.open_menu == Some(id) { None } else { Some(id) };
                                cx.notify();
                            }))
                    }))
            })
            // Drag region (fills remaining space)
            .child(div().flex_1().h_full().on_mouse_down(
                gpui::MouseButton::Left,
                |_, window, _cx: &mut App| {
                    window.start_window_move();
                },
            ))
            // Mock server toggle
            .child(
                div()
                    .id("btn-mock-server")
                    .h(px(22.0))
                    .px(px(8.0))
                    .mr(px(6.0))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .bg(if show_mock {
                        theme.colors.accent.opacity(0.15)
                    } else {
                        theme.colors.bg_elevated
                    })
                    .border_1()
                    .border_color(if show_mock {
                        theme.colors.accent.opacity(0.4)
                    } else {
                        theme.colors.border
                    })
                    .hover(|s| s.border_color(theme.colors.accent.opacity(0.5)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_mock_server(cx);
                    }))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if show_mock {
                                theme.colors.accent
                            } else {
                                theme.colors.text_secondary
                            })
                            .child("Mock Server"),
                    ),
            )
            // Window controls
            .child(
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    .border_l_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .id("btn-minimize")
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(theme.colors.bg_elevated))
                            .on_click(|_, window, _cx: &mut App| {
                                window.minimize_window();
                            })
                            .child(icon(ICON_MINIMIZE, 12.0, theme.colors.text_secondary)),
                    )
                    .child(
                        div()
                            .id("btn-maximize")
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(theme.colors.bg_elevated))
                            .on_click(|_, window, _cx: &mut App| {
                                window.toggle_fullscreen();
                            })
                            .child(icon(ICON_MAXIMIZE, 12.0, theme.colors.text_secondary)),
                    )
                    .child(
                        div()
                            .id("btn-close")
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(theme.colors.error).text_color(theme.colors.bg_primary))
                            .on_click(|_, _window, cx: &mut App| {
                                cx.quit();
                            })
                            .child(icon(ICON_WINDOW_CLOSE, 12.0, theme.colors.text_secondary)),
                    ),
            )
    }

    fn render_status_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        // Read protocol from request panel
        let protocol = self.request_panel.read(cx).mode_label();
        let protocol_color = theme.method_color(protocol);

        // Read last response summary
        let response_info = self.response_panel.read(cx).last_response_summary();
        let is_loading = self.response_panel.read(cx).is_loading();

        let sep = || {
            div()
                .w(px(1.0))
                .h(px(10.0))
                .bg(theme.colors.border)
                .mx(px(6.0))
        };

        div()
            .id("status-bar")
            .h(px(22.0))
            .w_full()
            .flex()
            .items_center()
            .flex_shrink_0()
            .px(px(10.0))
            .gap(px(0.0))
            .bg(theme.colors.bg_primary)
            .border_t_1()
            .border_color(theme.colors.border)
            // Active env dot + label
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(div().size(px(6.0)).bg(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Local Dev"),
                    ),
            )
            .child(sep())
            // Protocol badge
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(protocol_color)
                    .child(protocol),
            )
            .child(sep())
            // Response info or ready state
            .child(if is_loading {
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Sending…"),
                    )
                    .into_any_element()
            } else if let Some((status, _, time_ms, size_bytes)) = response_info {
                let status_color = theme.status_color(status);
                let size_str = if size_bytes >= 1024 * 1024 {
                    format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0))
                } else if size_bytes >= 1024 {
                    format!("{:.1} KB", size_bytes as f64 / 1024.0)
                } else {
                    format!("{} B", size_bytes)
                };
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(status_color)
                            .child(format!("{}", status)),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("·"),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child(format!("{}ms", time_ms)),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("·"),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child(size_str),
                    )
                    .into_any_element()
            } else {
                div()
                    .text_size(px(10.0))
                    .text_color(theme.colors.text_muted)
                    .child("Ready")
                    .into_any_element()
            })
    }

    fn render_codegen_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let panel = self.request_panel.read(cx);
        let editor = panel.codegen_editor.clone();
        let current_lang = panel.codegen_language;
        let width = self.codegen_panel_width;

        use protide_core::codegen::Language as CodegenLanguage;
        let languages: &[(CodegenLanguage, &str)] = &[
            (CodegenLanguage::Curl, "cURL"),
            (CodegenLanguage::Python, "Python"),
            (CodegenLanguage::JavaScript, "JS"),
            (CodegenLanguage::Go, "Go"),
            (CodegenLanguage::Rust, "Rust"),
        ];

        div()
            .id("codegen-panel")
            .w(px(width))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_secondary)
            .border_l_1()
            .border_color(theme.colors.border)
            // Header
            .child(
                div()
                    .h(theme.sizes.toolbar)
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    // Language tabs
                    .child(div().flex().items_center().gap(px(2.0)).flex_1().children(
                        languages.iter().map(|&(lang, label)| {
                            let is_active = lang == current_lang;
                            div()
                                .id(SharedString::from(format!("codegen-tab-{}", label)))
                                .px(px(8.0))
                                .py(px(3.0))
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .cursor_pointer()
                                .when(is_active, |el| {
                                    el.bg(theme.colors.accent.opacity(0.15))
                                        .text_color(theme.colors.accent)
                                        .border_1()
                                        .border_color(theme.colors.accent.opacity(0.3))
                                })
                                .when(!is_active, |el| {
                                    el.text_color(theme.colors.text_secondary).hover(|s| {
                                        s.bg(theme.colors.bg_tertiary)
                                            .text_color(theme.colors.text_primary)
                                    })
                                })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.request_panel
                                        .update(cx, |panel, cx| panel.generate_code(lang, cx));
                                }))
                                .child(label)
                        }),
                    ))
                    // Copy button
                    .child(
                        div()
                            .id("codegen-copy")
                            .h(px(28.0))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_secondary)
                            .cursor_pointer()
                            .bg(theme.colors.bg_elevated)
                            .border_1()
                            .border_color(theme.colors.border)
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_panel
                                    .update(cx, |panel, cx| panel.copy_generated_code(cx));
                            }))
                            .child(icon(ICON_COPY, ICON_SM, theme.colors.text_secondary))
                            .child("Copy"),
                    )
                    // Close button
                    .child(
                        div()
                            .id("codegen-close")
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_muted)
                            .hover(|s| {
                                s.bg(theme.colors.bg_elevated)
                                    .text_color(theme.colors.text_primary)
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_panel
                                    .update(cx, |panel, cx| panel.close_codegen_panel(cx));
                            }))
                            .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted)),
                    ),
            )
            // Code editor
            .child(div().flex_1().overflow_hidden().child(editor))
    }

    fn render_menu_dropdown(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let toolbar_h = 40.0f32;

        // (label, shortcut_hint, action_fn)
        type ActionFn = Box<dyn Fn(&mut gpui::Window, &mut gpui::App)>;
        let items: Vec<(&str, &str, ActionFn)> = match self.open_menu {
            Some(0) => vec![
                ("About Protide",     "",          Box::new(|w, cx| w.dispatch_action(Box::new(ShowAbout), cx))),
                ("---",               "",          Box::new(|_, _| {})),
                ("Quit",              "Ctrl+Q",    Box::new(|w, cx| w.dispatch_action(Box::new(Quit), cx))),
            ],
            Some(1) => vec![
                ("Send Request",      "Ctrl+Enter", Box::new(|w, cx| w.dispatch_action(Box::new(SendRequest), cx))),
                ("Save Request",      "Ctrl+S",     Box::new(|w, cx| w.dispatch_action(Box::new(SaveRequest), cx))),
            ],
            Some(2) => vec![
                ("Toggle Sidebar",    "Ctrl+B",     Box::new(|w, cx| w.dispatch_action(Box::new(ToggleSidebar), cx))),
                ("Toggle Mock Server","Ctrl+Shift+M",Box::new(|w, cx| w.dispatch_action(Box::new(ToggleMockServer), cx))),
            ],
            Some(3) => vec![
                ("Keyboard Shortcuts","F1",          Box::new(|w, cx| w.dispatch_action(Box::new(ShowHelp), cx))),
            ],
            _ => vec![],
        };

        // Horizontal offset per menu id (approximate, based on title bar layout)
        let left_px = match self.open_menu {
            Some(0) => 88.0,
            Some(1) => 148.0,
            Some(2) => 220.0,
            Some(3) => 272.0,
            _       => 88.0,
        };

        div()
            .id("menu-dropdown")
            .absolute()
            .top(px(toolbar_h))
            .left(px(left_px))
            .min_w(px(200.0))
            .py(px(4.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(items.into_iter().enumerate().map(|(i, (label, hint, action))| {
                if label == "---" {
                    return div()
                        .id(("menu-sep", i))
                        .my(px(3.0))
                        .mx(px(6.0))
                        .h(px(1.0))
                        .bg(theme.colors.border)
                        .into_any_element();
                }
                div()
                    .id(("menu-item", i))
                    .px(px(12.0)).py(px(7.0))
                    .flex().items_center().justify_between()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_menu = None;
                        cx.notify();
                        action(window, cx);
                    }))
                    .child(
                        div().text_size(px(12.0)).text_color(theme.colors.text_primary).child(label)
                    )
                    .when(!hint.is_empty(), |el| el.child(
                        div().text_size(px(10.0)).text_color(theme.colors.text_muted).ml(px(24.0)).child(hint)
                    ))
                    .into_any_element()
            }))
    }

    fn render_help_overlay(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        let shortcuts: &[(&str, &str, &str)] = &[
            ("Request", "Ctrl+Enter", "Send request"),
            ("Request", "Ctrl+S", "Save request"),
            ("View", "Ctrl+B", "Toggle sidebar"),
            ("View", "Ctrl+Shift+M", "Toggle mock server"),
            ("Help", "F1", "Show keyboard shortcuts"),
            ("Help", "Ctrl+Shift+A", "About Protide"),
            ("General", "Escape", "Close dialog / overlay"),
        ];

        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.bg_primary.opacity(0.7))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.show_help = false;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w(px(480.0))
                    .bg(theme.colors.bg_elevated)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {}) // stop propagation
                    .child(
                        // Header
                        div()
                            .px(px(20.0))
                            .py(px(14.0))
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("Keyboard Shortcuts"),
                            )
                            .child(
                                div()
                                    .id("help-close")
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(theme.colors.text_primary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_help = false;
                                        cx.notify();
                                    }))
                                    .child("✕"),
                            ),
                    )
                    .child(
                        // Shortcut rows
                        div()
                            .px(px(20.0))
                            .py(px(12.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(shortcuts.iter().map(|(group, key, desc)| {
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .py(px(5.0))
                                    .border_b_1()
                                    .border_color(theme.colors.border.opacity(0.4))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(10.0))
                                            .child(
                                                div()
                                                    .w(px(60.0))
                                                    .text_size(px(10.0))
                                                    .text_color(theme.colors.text_muted)
                                                    .child(*group),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(theme.colors.text_secondary)
                                                    .child(*desc),
                                            ),
                                    )
                                    .child(div().flex().items_center().gap(px(3.0)).children(
                                        key.split('+').map(|k| {
                                            div()
                                                .px(px(7.0))
                                                .py(px(3.0))
                                                .bg(theme.colors.bg_tertiary)
                                                .border_1()
                                                .border_color(theme.colors.border)
                                                .text_size(px(10.0))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.colors.text_primary)
                                                .child(k.trim())
                                        }),
                                    ))
                            })),
                    )
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(10.0))
                            .border_t_1()
                            .border_color(theme.colors.border)
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Press Escape or click outside to close"),
                    ),
            )
    }

    fn render_about_overlay(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.bg_primary.opacity(0.7))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.show_about = false;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w(px(360.0))
                    .bg(theme.colors.bg_elevated)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .child(
                        // Header with close
                        div()
                            .px(px(20.0))
                            .py(px(14.0))
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("About"),
                            )
                            .child(
                                div()
                                    .id("about-close")
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(theme.colors.text_primary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_about = false;
                                        cx.notify();
                                    }))
                                    .child("✕"),
                            ),
                    )
                    .child(
                        div()
                            .px(px(28.0))
                            .py(px(24.0))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(14.0))
                            // Logo badge
                            .child(
                                div()
                                    .size(px(56.0))
                                    .bg(theme.colors.accent)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(28.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.colors.bg_primary)
                                            .child("P"),
                                    ),
                            )
                            // Name + version
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .text_size(px(20.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.colors.text_primary)
                                            .child("Protide"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child(format!(
                                                "Version {}",
                                                env!("CARGO_PKG_VERSION")
                                            )),
                                    ),
                            )
                            // Description
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .text_center()
                                    .child("Free and open-source API testing tool"),
                            )
                            // Divider
                            .child(div().w_full().h(px(1.0)).bg(theme.colors.border))
                            // Developer
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(3.0))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("Developed by"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_primary)
                                            .child("Rakibul Yeasin"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(10.0))
                            .border_t_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .id("about-ok")
                                    .px(px(28.0))
                                    .py(px(7.0))
                                    .bg(theme.colors.accent)
                                    .text_color(theme.colors.bg_primary)
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.accent_hover))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_about = false;
                                        cx.notify();
                                    }))
                                    .child("Close"),
                            ),
                    ),
            )
    }
}
