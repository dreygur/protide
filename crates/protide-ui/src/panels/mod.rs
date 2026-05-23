//! Panel components for the dockable layout
//!
//! This module contains the main UI panels:
//! - `ExplorerPanel` - File tree and environment management
//! - `RequestPanel` - HTTP request builder
//! - `ResponsePanel` - Response viewer with syntax highlighting
//! - `RequestHistory` - Request history tracking
//! - `MockServerPanel` - Mock server configuration

#[path = "console/mod.rs"]
mod console;
mod docs;
mod explorer;
mod history;
mod mock_server;
pub mod presence;
mod presence_render;
mod request;
mod request_types;
mod request_utils;
mod response;
pub mod runner;

pub use console::{ConsoleEntry, ConsoleEntrySource, ConsolePanel, LogLevel};
pub use docs::DocsPanel;
pub use explorer::ExplorerPanel;
pub use history::RequestHistory;
pub use mock_server::MockServerPanel;
pub use request::RequestPanel;
pub use response::ResponsePanel;
pub use response::format_size;
pub use runner::RunnerPanel;
