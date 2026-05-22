use std::sync::Arc;
use anyhow::Result;
use gpui::{Menu, MenuItem, WindowOptions, size, px, AppContext as _};
use gpui_component::Root;
use gpui_component_assets::Assets;
use protide_ui::{
    MainWindow, register_keybindings,
    SendRequest, SaveRequest, ToggleSidebar, ToggleMockServer,
    ShowHelp, ShowAbout, Quit,
};
use protide_ui::panels::RequestHistory;
use protide_core::sync::{SyncEngine, SyncConfig};

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
            gpui_component::Theme::global_mut(cx).window_border = gpui::transparent_black();

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
                window_decorations: Some(gpui::WindowDecorations::Client),
                app_id: Some("protide".into()),
                icon: load_app_icon(),
                ..Default::default()
            };

            let node_name = std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "developer".into());
            let pairing_code = protide_core::sync::pake::generate_pairing_code();
            let mut engine = SyncEngine::new(SyncConfig {
                node_name,
                p2p_enabled: true,
                live_probe_enabled: true,
                pairing_code: Some(pairing_code),
                node_id_path: dirs::config_dir().map(|d| d.join("protide").join("node_id")),
                ..Default::default()
            });
            let _ = engine.init();
            let sync_engine = Some(engine);

            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| MainWindow::build(window, cx, sync_engine));
                cx.new(|cx| Root::new(view, window, cx).window_shadow_size(gpui::px(0.0)))
            })
            .expect("Failed to open main window");
        });

    Ok(())
}
