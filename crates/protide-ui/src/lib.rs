mod components;
pub mod last_paths;
mod main_window;
pub mod panels;
pub mod prefs;
pub mod session;
#[cfg(test)]
mod test_support;
pub mod theme;

/// Path the binary's `AssetSource` serves the Protide logo from. The UI can't
/// `include_bytes!` it - the PNG lives in the `protide` crate, which depends on
/// this one - so the logo travels as an asset path instead.
pub const LOGO_ASSET_PATH: &str = "protide-logo.png";

pub use main_window::{
    DismissOverlay, MainWindow, Quit, SaveRequest, SendRequest, ShowAbout, ShowHelp,
    ToggleMockServer, ToggleSidebar, register_keybindings,
};
