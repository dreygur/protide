use tower_lsp::lsp_types::*;

pub fn complete(content: &str, pos: Position) -> Vec<CompletionItem> {
    let line = content
        .lines()
        .nth(pos.line as usize)
        .unwrap_or("")
        .to_string();
    let before = &line[..line.len().min(pos.character as usize)];
    let trimmed = before.trim_start();

    if trimmed.starts_with("# @protocol") && trimmed.len() > "# @protocol".len() {
        return protocol_value_completions();
    }
    if trimmed.starts_with("# @depends") && trimmed.len() > "# @depends".len() {
        return depends_completions(content);
    }
    if trimmed.starts_with("# @") {
        return annotation_completions();
    }
    if let Some(open) = before.rfind("{{") {
        if !before[open..].contains("}}") {
            return variable_completions(content);
        }
    }
    if is_request_line(before) {
        return method_completions();
    }
    if before.contains(':') {
        return header_value_completions(before);
    }
    if !before.contains(' ') && !before.starts_with('#') {
        return header_name_completions();
    }
    vec![]
}

fn is_request_line(before: &str) -> bool {
    let upper = before.trim_start().to_uppercase();
    ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
        .iter()
        .any(|m| upper.starts_with(m) || m.starts_with(upper.trim()))
}

fn annotation_completions() -> Vec<CompletionItem> {
    [
        ("@name", "Name this request for chaining"),
        ("@description", "Human-readable description"),
        (
            "@protocol",
            "Override protocol (http|graphql|websocket|grpc|trpc|socketio)",
        ),
        ("@set", "Extract response value to variable: @set var = $.path"),
        ("@depends", "Declare dependency on named request"),
        ("@proto", "Path to .proto file for gRPC"),
    ]
    .iter()
    .map(|(label, detail)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(detail.to_string()),
        ..Default::default()
    })
    .collect()
}

fn protocol_value_completions() -> Vec<CompletionItem> {
    ["http", "graphql", "websocket", "grpc", "trpc", "socketio"]
        .iter()
        .map(|v| CompletionItem {
            label: v.to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            ..Default::default()
        })
        .collect()
}

fn depends_completions(content: &str) -> Vec<CompletionItem> {
    let Ok(requests) = http_parser::parse(content) else {
        return vec![];
    };
    requests
        .iter()
        .filter_map(|r| r.meta.name.as_deref())
        .map(|name| CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::REFERENCE),
            ..Default::default()
        })
        .collect()
}

fn variable_completions(content: &str) -> Vec<CompletionItem> {
    let mut vars: Vec<String> = Vec::new();

    // Collect @set declarations
    for line in content.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("# @set") {
            let rest = rest.trim();
            let name = rest.split(|c: char| c.is_whitespace() || c == '=').next().unwrap_or("");
            if !name.is_empty() && !vars.contains(&name.to_string()) {
                vars.push(name.to_string());
            }
        }
    }

    // Collect {{varName}} usages throughout the file
    let mut search = content;
    while let Some(open) = search.find("{{") {
        search = &search[open + 2..];
        if let Some(close) = search.find("}}") {
            let name = search[..close].trim().to_string();
            if !name.is_empty() && !vars.contains(&name) {
                vars.push(name);
            }
            search = &search[close + 2..];
        } else {
            break;
        }
    }

    vars.into_iter()
        .map(|name| CompletionItem {
            label: format!("{{{{{}}}}}", name),
            insert_text: Some(name.clone()),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some(format!("Variable: {name}")),
            ..Default::default()
        })
        .collect()
}

fn method_completions() -> Vec<CompletionItem> {
    ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
        .iter()
        .map(|m| CompletionItem {
            label: m.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        })
        .collect()
}

fn header_name_completions() -> Vec<CompletionItem> {
    [
        ("Content-Type", "application/json"),
        ("Authorization", "Bearer <token>"),
        ("Accept", "application/json"),
        ("X-Request-ID", ""),
        ("X-API-Key", ""),
        ("Cache-Control", "no-cache"),
    ]
    .iter()
    .map(|(name, value)| CompletionItem {
        label: format!("{name}: {value}"),
        kind: Some(CompletionItemKind::PROPERTY),
        insert_text: Some(format!("{name}: ")),
        ..Default::default()
    })
    .collect()
}

fn header_value_completions(line: &str) -> Vec<CompletionItem> {
    if line.to_lowercase().contains("content-type:") {
        return [
            "application/json",
            "application/x-www-form-urlencoded",
            "multipart/form-data",
            "text/plain",
        ]
        .iter()
        .map(|v| CompletionItem {
            label: v.to_string(),
            kind: Some(CompletionItemKind::VALUE),
            ..Default::default()
        })
        .collect();
    }
    vec![]
}
