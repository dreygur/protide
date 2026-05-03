mod app;
mod chaining;
mod codegen;
mod export;
mod import;
mod mock_server;
mod models;
mod protocols;
mod scripting;
mod theme;
mod ui;
mod workspace;

use anyhow::Result;
use gpui::{WindowOptions, size, px, AppContext as _};
use gpui_component::Root;
use gpui_component_assets::Assets;
use ui::MainWindow;
use ui::panels::RequestHistory;

fn main() -> Result<()> {
    gpui_platform::application()
        .with_assets(Assets)
        .run(|cx| {
            gpui_component::init(cx);

            // Initialize theme based on system preference
            theme::init(cx);

            // Initialize request history
            cx.set_global(RequestHistory::new());

            let window_options = WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds {
                    origin: gpui::Point::default(),
                    size: size(px(1400.0), px(900.0)),
                })),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("API Dash".into()),
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
