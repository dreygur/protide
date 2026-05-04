use anyhow::Result;
use gpui::{WindowOptions, size, px, AppContext as _};
use gpui_component::Root;
use gpui_component_assets::Assets;
use protide_ui::ui::MainWindow;
use protide_ui::ui::panels::RequestHistory;

fn main() -> Result<()> {
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
