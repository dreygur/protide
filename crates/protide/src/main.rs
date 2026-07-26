use std::borrow::Cow;
use std::sync::Arc;
use anyhow::Result;
use gpui::{AssetSource, Menu, MenuItem, SharedString, WindowOptions, size, px, AppContext as _};
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

/// gpui-component's bundled assets (icons, fonts) plus Protide's own logo, so the
/// UI can reference the logo by path without reaching into this crate's `assets/`.
struct ProtideAssets;

impl AssetSource for ProtideAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path == protide_ui::LOGO_ASSET_PATH {
            return Ok(Some(Cow::Borrowed(APP_ICON_PNG)));
        }
        Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Assets.list(path)
    }
}

/// macOS takes the Dock / app-switcher icon from the `.app` bundle's
/// `CFBundleIconFile` (see `packaging/macos/`), and `WindowOptions::icon` is
/// X11-only - so an unbundled binary (`cargo run`) shows the generic executable
/// icon unless we hand the image to NSApplication ourselves.
#[cfg(target_os = "macos")]
fn set_dock_icon() {
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(APP_ICON_PNG);
    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
        unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
    }
}

fn main() -> Result<()> {
    // Default to info level; override with RUST_LOG=debug cargo run
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    gpui_platform::application()
        .with_assets(ProtideAssets)
        .run(|cx| {
            #[cfg(target_os = "macos")]
            set_dock_icon();

            gpui_component::init(cx);
            gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
            {
                let t = gpui_component::Theme::global_mut(cx);
                t.radius = gpui::px(2.0);
                t.radius_lg = gpui::px(4.0);
                t.window_border = gpui::transparent_black();
                t.colors.ring = gpui::rgb(0x4ade80).into();
                t.colors.foreground = gpui::rgb(0xe4e4ed).into();
                t.colors.muted_foreground = gpui::rgb(0x7f7f92).into();
                t.mono_font_family = "JetBrains Mono".into();
            }

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
                    // macOS draws its own titlebar on top of ours unless this is
                    // transparent - opaque here means two stacked title bars.
                    appears_transparent: true,
                    // Vertically centered for the 40px toolbar: (40 - 12) / 2.
                    traffic_light_position: Some(gpui::point(px(9.0), px(14.0))),
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
