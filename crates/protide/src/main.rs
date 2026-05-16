use std::sync::Arc;
use anyhow::Result;
use gpui::{Menu, MenuItem, WindowOptions, size, px, AppContext as _};
use gpui_component::Root;
use gpui_component_assets::Assets;
use protide_ui::ui::{
    MainWindow, register_keybindings,
    SendRequest, SaveRequest, ToggleSidebar, ToggleMockServer,
    ShowHelp, ShowAbout, Quit,
};
use protide_ui::ui::panels::RequestHistory;

const APP_ICON_PNG: &[u8] = include_bytes!("../assets/protide-logo.png");

fn load_app_icon() -> Option<Arc<image::RgbaImage>> {
    let img = image::load_from_memory(APP_ICON_PNG).ok()?;
    Some(Arc::new(img.to_rgba8()))
}

fn main() -> Result<()> {
    // Default to info level; override with RUST_LOG=debug cargo run
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    gpui_platform::application()
        .with_assets(Assets)
        .run(|cx| {
            gpui_component::init(cx);

            cx.text_system()
                .add_fonts(vec![
                    std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice()),
                    std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice()),
                    std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Italic.ttf").as_slice()),
                    std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-BoldItalic.ttf").as_slice()),
                ])
                .expect("Failed to load JetBrains Mono fonts");

            protide_ui::theme::init(cx);
            register_keybindings(cx);

            cx.set_menus([
                Menu::new("Protide").items([
                    MenuItem::action("About Protide", ShowAbout),
                    MenuItem::separator(),
                    MenuItem::action("Quit Protide", Quit),
                ]),
                Menu::new("Request").items([
                    MenuItem::action("Send Request", SendRequest),
                    MenuItem::action("Save Request", SaveRequest),
                ]),
                Menu::new("View").items([
                    MenuItem::action("Toggle Sidebar", ToggleSidebar),
                    MenuItem::action("Toggle Mock Server", ToggleMockServer),
                ]),
                Menu::new("Help").items([
                    MenuItem::action("Keyboard Shortcuts", ShowHelp),
                ]),
            ]);

            cx.set_global(RequestHistory::new());

            let window_options = WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds {
                    origin: gpui::Point::default(),
                    size: size(px(1400.0), px(900.0)),
                })),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Protide".into()),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                app_id: Some("protide".into()),
                icon: load_app_icon(),
                ..Default::default()
            };

            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| MainWindow::build(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open main window");
        });

    Ok(())
}
