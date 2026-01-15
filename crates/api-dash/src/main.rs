//! API Dash - Free and open-source API testing application
//!
//! A native desktop application built with GPUI supporting:
//! - HTTP/REST
//! - GraphQL
//! - gRPC
//! - WebSocket
//! - Socket.IO
//! - tRPC

mod app;
mod chaining;
mod codegen;
mod import;
mod mock_server;
mod models;
mod protocols;
mod scripting;
mod theme;
mod ui;
mod workspace;

use anyhow::Result;
use gpui::{Application, WindowOptions, size, px, AppContext as _};
use ui::MainWindow;
use ui::panels::RequestHistory;

fn main() -> Result<()> {
    Application::new().run(|cx| {
        // Initialize theme based on system preference
        theme::init(cx);

        // Initialize request history
        cx.set_global(RequestHistory::new());

        // Open main window with native decorations
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

        cx.open_window(window_options, |window, app| {
            app.new(|cx| MainWindow::build(window, cx))
        })
        .expect("Failed to open main window");
    });

    Ok(())
}
