use gpui::Context;
use super::*;
use super::super::request_utils::{url_decode, url_encode};

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Get the base URL without query string
    pub(super) fn get_base_url(&self) -> &str {
        self.url.split('?').next().unwrap_or(&self.url)
    }

    /// Parse query params from URL and update params list
    pub(super) fn sync_params_from_url(&mut self, cx: &mut Context<Self>) {
        if self.syncing_params {
            return;
        }
        self.syncing_params = true;

        if let Some(query_start) = self.url.find('?') {
            let query_string = &self.url[query_start + 1..];
            let mut new_params: Vec<KeyValuePair> = Vec::new();

            for pair in query_string.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let mut parts = pair.splitn(2, '=');
                let key = url_decode(parts.next().unwrap_or(""));
                let value = url_decode(parts.next().unwrap_or(""));
                new_params.push(KeyValuePair { key, value, enabled: true });
            }

            if new_params.is_empty() {
                new_params.push(KeyValuePair::default());
            }

            self.params = new_params;
        } else {
            self.params = vec![KeyValuePair::default()];
        }

        while self.params.len() < 3 {
            self.params.push(KeyValuePair::default());
        }

        self.syncing_params = false;
        cx.notify();
    }

    /// Build URL from base URL and params
    pub(super) fn sync_url_from_params(&mut self, cx: &mut Context<Self>) {
        if self.syncing_params {
            return;
        }
        self.syncing_params = true;

        let base_url = self.get_base_url().to_string();

        let query_parts: Vec<String> = self
            .params
            .iter()
            .filter(|p| p.enabled && !p.key.is_empty())
            .map(|p| {
                if p.value.is_empty() {
                    url_encode(&p.key)
                } else {
                    format!("{}={}", url_encode(&p.key), url_encode(&p.value))
                }
            })
            .collect();

        let old_len = self.url.len();
        if query_parts.is_empty() {
            self.url = base_url;
        } else {
            self.url = format!("{}?{}", base_url, query_parts.join("&"));
        }

        let new_len = self.url.len();
        if self.url_selection.start > new_len {
            self.url_selection.start = new_len;
        }
        if self.url_selection.end > new_len {
            self.url_selection.end = new_len;
        }

        self.syncing_params = false;
        if old_len != new_len {
            cx.notify();
        }
    }
}
