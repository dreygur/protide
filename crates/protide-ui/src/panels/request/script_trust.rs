//! Asks before a script the user has not trusted runs.

use super::*;
use protide_core::scripting::trust::TrustStore;

/// The app wide script trust store. The app installs the one in the config
/// dir at startup; read through `cx.default_global()`, so tests that never
/// install it get an empty in memory store and never touch the real file.
#[derive(Default)]
pub struct ScriptTrust(pub TrustStore);

impl gpui::Global for ScriptTrust {}

/// Work that runs the request's scripts, such as a send.
type Run<E> = fn(&mut RequestPanel<E>, &mut Context<RequestPanel<E>>);

/// How many scripts, in the order pre, post, tests, an action runs. A send
/// runs all three; a WebSocket connect runs only the pre script.
pub(super) const ALL_SCRIPTS: usize = 3;
pub(super) const PRE_SCRIPT: usize = 1;

/// A run held back until the user decides on its untrusted scripts.
pub(crate) struct PendingTrust<E: WebSocketExecutor> {
    scripts: Vec<String>,
    run: Run<E>,
    shown: bool,
}

fn trust_scripts(scripts: &[String], cx: &mut gpui::App) {
    let store = &mut cx.default_global::<ScriptTrust>().0;
    for script in scripts {
        if let Err(e) = store.trust(script) {
            log::warn!("Could not save script trust: {e}");
        }
    }
}

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Each script editor's text, with the text it held when loaded.
    fn scripts_with_origin(&self, cx: &gpui::App) -> [(String, &str); 3] {
        let text = |editor: &Entity<InputState>| editor.read(cx).value().to_string();
        [
            (text(&self.pre_script_editor), self.pre_script.as_str()),
            (text(&self.post_script_editor), self.post_script.as_str()),
            (text(&self.tests_editor), self.tests.as_str()),
        ]
    }

    /// Trust every script changed since it was loaded: the user wrote it.
    pub(super) fn trust_edited_scripts(&self, cx: &mut gpui::App) {
        let edited: Vec<String> = self
            .scripts_with_origin(cx)
            .into_iter()
            .filter(|(text, loaded)| text != loaded)
            .map(|(text, _)| text)
            .collect();
        trust_scripts(&edited, cx);
    }

    /// Do `run` now if the first `count` scripts, the ones it runs, are
    /// trusted, else hold it until the user has seen the untrusted ones and
    /// decided.
    pub(super) fn run_trusted(&mut self, count: usize, run: Run<E>, cx: &mut Context<Self>) {
        self.trust_edited_scripts(cx);
        let texts = self.scripts_with_origin(cx).map(|(text, _)| text);
        let scripts: Vec<String> = cx
            .default_global::<ScriptTrust>()
            .0
            .untrusted(texts[..count].iter().map(String::as_str))
            .into_iter()
            .map(str::to_string)
            .collect();
        if scripts.is_empty() {
            run(self, cx);
            return;
        }
        self.pending_trust = Some(PendingTrust {
            scripts,
            run,
            shown: false,
        });
        cx.notify();
    }

    /// The untrusted scripts of a held back run, once, for the dialog.
    pub(crate) fn trust_prompt(&mut self) -> Option<Vec<String>> {
        let pending = self.pending_trust.as_mut().filter(|p| !p.shown)?;
        pending.shown = true;
        Some(pending.scripts.clone())
    }

    /// Settle the held back run: trust its scripts and do it, or drop it.
    pub(crate) fn resolve_trust(&mut self, run: bool, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_trust.take() else {
            return;
        };
        if run {
            trust_scripts(&pending.scripts, cx);
            (pending.run)(self, cx);
            return;
        }
        if let Some(console) = &self.console_panel {
            let entry = ConsoleEntry::warning("Send cancelled: untrusted script not run");
            console.update(cx, |panel, cx| panel.log(entry, cx));
        }
    }
}
