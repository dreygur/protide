use gpui::Context;
use super::*;

impl ExplorerPanel {
    pub(super) fn toggle_history(&mut self, cx: &mut Context<Self>) {
        self.history_expanded = !self.history_expanded;
        cx.notify();
    }

    pub(super) fn load_history_item(&mut self, entry_id: u64, cx: &mut Context<Self>) {
        let entry_data: Option<(String, String, Vec<(String, String)>, Option<String>)> =
            cx.read_global::<RequestHistory, _>(|history, _| {
                history.get(entry_id).map(|entry| {
                    (
                        entry.method.clone(),
                        entry.url.clone(),
                        entry.headers.clone(),
                        entry.body.clone(),
                    )
                })
            });

        if let Some((method, url, headers, body)) = entry_data
            && let Some(request_panel) = &self.request_panel {
                request_panel.update(cx, |panel, cx| {
                    panel.load_from_history(method, url, headers, body, cx);
                });
            }
    }

    pub(super) fn get_history_entries(&self, cx: &Context<Self>) -> Vec<HistoryEntry> {
        cx.read_global::<RequestHistory, _>(|history, _| history.entries().to_vec())
    }
}
