//! Native file dialogs that run without holding the `App` borrow.

use gpui::{Context, Window};
use std::path::PathBuf;

/// Which native dialog to present.
pub(crate) enum Pick {
    /// Choose an existing file.
    File,
    /// Choose a directory.
    Folder,
    /// Choose a destination path to write to.
    Save,
}

impl Pick {
    async fn show(self, dialog: rfd::AsyncFileDialog) -> Option<PathBuf> {
        let handle = match self {
            Pick::File => dialog.pick_file().await,
            Pick::Folder => dialog.pick_folder().await,
            Pick::Save => dialog.save_file().await,
        };
        handle.map(|h| h.path().to_path_buf())
    }
}

/// Present a native dialog, then run `then` with the chosen path.
///
/// Always prefer this over `rfd`'s blocking `FileDialog`. The blocking dialogs
/// spin a nested platform run loop while the calling handler still holds the
/// `App` `RefCell`, so every gpui callback delivered while the dialog is open
/// fails to borrow and is silently dropped - on macOS that surfaces as
/// "RefCell already borrowed" plus missed redraws and appearance changes.
/// Awaiting the async dialog inside `cx.spawn` releases the borrow first.
pub(crate) fn prompt<T: 'static>(
    cx: &mut Context<T>,
    pick: Pick,
    build: impl FnOnce(rfd::AsyncFileDialog) -> rfd::AsyncFileDialog + 'static,
    then: impl FnOnce(&mut T, PathBuf, &mut Context<T>) + 'static,
) {
    cx.spawn(async move |this, cx| {
        if let Some(path) = pick.show(build(rfd::AsyncFileDialog::new())).await {
            let _ = this.update(cx, |this, cx| then(this, path, cx));
        }
    })
    .detach();
}

/// [`prompt`], for callers whose continuation also needs the [`Window`].
pub(crate) fn prompt_in<T: 'static>(
    window: &Window,
    cx: &mut Context<T>,
    pick: Pick,
    build: impl FnOnce(rfd::AsyncFileDialog) -> rfd::AsyncFileDialog + 'static,
    then: impl FnOnce(&mut T, PathBuf, &mut Window, &mut Context<T>) + 'static,
) {
    cx.spawn_in(window, async move |this, cx| {
        if let Some(path) = pick.show(build(rfd::AsyncFileDialog::new())).await {
            let _ = this.update_in(cx, |this, window, cx| then(this, path, window, cx));
        }
    })
    .detach();
}
