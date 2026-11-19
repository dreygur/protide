//! Asks before a script the user has not trusted runs.

use protide_core::scripting::trust::TrustStore;

/// The app wide script trust store. The app installs the one in the config
/// dir at startup; read through `cx.default_global()`, so tests that never
/// install it get an empty in memory store and never touch the real file.
#[derive(Default)]
pub struct ScriptTrust(pub TrustStore);

impl gpui::Global for ScriptTrust {}
