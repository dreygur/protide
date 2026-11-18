use super::*;

impl ResponsePanel {
    pub fn set_response(&mut self, response: ResponseData, cx: &mut Context<Self>) {
        debug!(
            "Response: {} {} ({} bytes, {:?})",
            response.status,
            response.status_text,
            response.body.len(),
            response.time
        );
        let content_type = response
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.to_lowercase());

        let (body_text, language) = if let Some(ct) = &content_type {
            if ct.contains("application/json") || ct.contains("+json") {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response.body) {
                    let formatted = serde_json::to_string_pretty(&json)
                        .unwrap_or_else(|_| response.body.clone());
                    (formatted, "json".to_string())
                } else {
                    warn!(
                        "JSON parse failed for '{}' response ({} bytes)",
                        ct,
                        response.body.len()
                    );
                    (response.body.clone(), "json".to_string())
                }
            } else if ct.contains("text/html") {
                (response.body.clone(), "html".to_string())
            } else if ct.contains("application/xml")
                || ct.contains("text/xml")
                || ct.contains("+xml")
            {
                (types::pretty_xml(&response.body), "xml".to_string())
            } else {
                self.detect_language_from_content(&response.body)
            }
        } else {
            self.detect_language_from_content(&response.body)
        };

        // Auto-switch to Preview when the response is HTML
        self.body_view_mode = if language == "html" {
            BodyViewMode::Preview
        } else {
            BodyViewMode::Pretty
        };

        self.raw_body = response.body.clone();
        self.formatted_body = body_text.clone();
        self.formatted_lang = language.clone();

        self.body_pending = Some((body_text, language));

        self.json_value = serde_json::from_str::<serde_json::Value>(&response.body).ok();
        self.json_tree_collapsed.clear();
        self.expanded_strings.clear();
        self.json_sel = None;
        self.json_selecting = false;
        // Header selections are byte offsets into the *old* response's header
        // values; carrying them over would highlight (and copy) arbitrary spans
        // of the new response's headers.
        self.hdr_sel = None;
        self.rebuild_json_rows();

        self.response = Some(response);
        self.loading = false;
        self.error = None;
        cx.notify();
    }

    fn detect_language_from_content(&self, body: &str) -> (String, String) {
        let trimmed = body.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                let formatted =
                    serde_json::to_string_pretty(&json).unwrap_or_else(|_| body.to_string());
                (formatted, "json".to_string())
            } else {
                (body.to_string(), String::new())
            }
        } else if trimmed.starts_with('<') {
            if trimmed.contains("<!DOCTYPE html") || trimmed.contains("<html") {
                (body.to_string(), "html".to_string())
            } else {
                (types::pretty_xml(body), "xml".to_string())
            }
        } else {
            (body.to_string(), String::new())
        }
    }
}
