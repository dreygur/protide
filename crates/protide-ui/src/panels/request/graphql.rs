use gpui::Context;
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Fetch the GraphQL schema via an introspection query to `self.url`.
    pub(super) fn fetch_graphql_schema(&mut self, cx: &mut Context<Self>) {
        let url = if let Some(ref exp) = self.explorer_panel {
            exp.read(cx).env_state().substitute(&self.url)
        } else {
            self.url.clone()
        };
        if url.is_empty() { return; }

        self.graphql_schema = GraphqlSchemaState::Loading;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx.background_executor()
                .spawn(async move { run_graphql_introspection(&url) })
                .await;
            let _ = cx.update(|cx| {
                let _ = this.update(cx, |panel, cx| {
                    panel.graphql_schema = result;
                    cx.notify();
                });
            });
        }).detach();
    }

    /// Import a GraphQL schema from a local .graphql or .json file.
    pub(super) fn import_graphql_schema_file(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let picked = rfd::AsyncFileDialog::new()
                .add_filter("GraphQL Schema", &["graphql", "gql", "json"])
                .pick_file()
                .await;
            if let Some(file) = picked {
                let path = file.path().to_path_buf();
                let result = cx.background_executor()
                    .spawn(async move { parse_schema_file(&path) })
                    .await;
                let _ = cx.update(|cx| {
                    let _ = this.update(cx, |panel, cx| {
                        panel.graphql_schema = result;
                        cx.notify();
                    });
                });
            }
        }).detach();
    }
}

// ── GraphQL schema helpers ────────────────────────────────────────────────────

/// Send the introspection query to `url` and return a `GraphqlSchemaState`.
pub(super) fn run_graphql_introspection(url: &str) -> GraphqlSchemaState {
    const INTROSPECTION: &str = r#"{"query":"{__schema{types{name kind description}}}"}"#;

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => return GraphqlSchemaState::Error(e.to_string()),
    };

    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(INTROSPECTION)
        .send();

    match resp {
        Err(e) => GraphqlSchemaState::Error(e.to_string()),
        Ok(r) => match r.json::<serde_json::Value>() {
            Err(e) => GraphqlSchemaState::Error(format!("Parse error: {e}")),
            Ok(json) => extract_schema_types(&json),
        },
    }
}

pub(super) fn extract_schema_types(json: &serde_json::Value) -> GraphqlSchemaState {
    let types = json.pointer("/data/__schema/types").and_then(|v| v.as_array());
    match types {
        None => GraphqlSchemaState::Error("Unexpected introspection response shape".into()),
        Some(arr) => {
            let types: Vec<GqlSchemaType> = arr
                .iter()
                .filter_map(|t| {
                    let name = t.get("name")?.as_str()?.to_string();
                    if name.starts_with("__") { return None; }
                    Some(GqlSchemaType {
                        name,
                        kind: t.get("kind").and_then(|k| k.as_str()).unwrap_or("").to_string(),
                    })
                })
                .collect();
            GraphqlSchemaState::Loaded(types)
        }
    }
}

pub(super) fn parse_schema_file(path: &std::path::Path) -> GraphqlSchemaState {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return GraphqlSchemaState::Error(e.to_string()),
    };

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
        return extract_schema_types(&json);
    }

    let types: Vec<GqlSchemaType> = content
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            for prefix in &["type ", "interface ", "enum ", "union ", "input ", "scalar "] {
                if t.starts_with(prefix) {
                    let rest = t[prefix.len()..].split_whitespace().next()?;
                    let name = rest.trim_end_matches('{').to_string();
                    if !name.starts_with("__") {
                        return Some(GqlSchemaType {
                            name,
                            kind: prefix.trim().to_uppercase(),
                        });
                    }
                }
            }
            None
        })
        .collect();

    if types.is_empty() {
        GraphqlSchemaState::Error("No type definitions found in file".into())
    } else {
        GraphqlSchemaState::Loaded(types)
    }
}

/// Return an actionable troubleshooting hint for DNS/network failures.
pub(super) fn dns_troubleshoot_hint(err: &str) -> Option<String> {
    let lower = err.to_lowercase();
    if lower.contains("resolve")
        || lower.contains("no such host")
        || lower.contains("dns")
        || lower.contains("name or service not known")
        || lower.contains("nodename nor servname")
        || lower.contains("unable to resolve")
        || lower.contains("failed to lookup")
        || lower.contains("connection refused")
        || lower.contains("network unreachable")
        || lower.contains("timed out")
    {
        Some(
            "1) Verify the hostname spelling in the URL.\n\
             2) Run: nslookup <hostname>  (or dig <hostname>)\n\
             3) Test basic connectivity: ping 8.8.8.8\n\
             4) For private/local hosts, check /etc/hosts or your VPN config.\n\
             5) If using a custom port, confirm the service is running: nc -zv <host> <port>"
                .to_string(),
        )
    } else {
        None
    }
}
