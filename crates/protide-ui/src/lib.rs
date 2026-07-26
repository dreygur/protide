pub mod theme;
pub mod panels;
pub mod last_paths;
pub mod prefs;
pub mod session;
mod main_window;
mod components;

/// Path the binary's `AssetSource` serves the Protide logo from. The UI can't
/// `include_bytes!` it - the PNG lives in the `protide` crate, which depends on
/// this one - so the logo travels as an asset path instead.
pub const LOGO_ASSET_PATH: &str = "protide-logo.png";

pub use main_window::{
    MainWindow, register_keybindings,
    SendRequest, SaveRequest, ToggleSidebar, ToggleMockServer,
    ShowHelp, ShowAbout, DismissOverlay, Quit,
};
