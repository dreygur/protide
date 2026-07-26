use crate::symbols::utf16_offset_to_byte_offset;
use tower_lsp::lsp_types::*;

pub fn complete(content: &str, pos: Position) -> Vec<CompletionItem> {
    let line = content
        .lines()
        .nth(pos.line as usize)
        .unwrap_or("")
        .to_string();
    // `pos.character` is a UTF-16 code-unit offset, not a byte offset - using
    // it directly to slice the UTF-8 `line` panics as soon as a multi-byte
    // character precedes the cursor.
    let before = &line[..utf16_offset_to_byte_offset(&line, pos.character as usize)];
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
    if let Some(open) = before.rfind("{{")
        && !before[open..].contains("}}")
    {
        return variable_completions(content);
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
        (
            "@set",
            "Extract response value to variable: @set var = $.path",
        ),
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
            let name = rest
                .split(|c: char| c.is_whitespace() || c == '=')
                .next()
                .unwrap_or("");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(items: &[CompletionItem]) -> Vec<&str> {
        items.iter().map(|i| i.label.as_str()).collect()
    }

    fn pos(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    #[test]
    fn annotation_prefix_offers_annotation_keywords() {
        let items = complete("# @\n", pos(0, 3));
        assert!(labels(&items).contains(&"@name"));
        assert!(labels(&items).contains(&"@depends"));
        assert!(
            items
                .iter()
                .all(|i| i.kind == Some(CompletionItemKind::KEYWORD))
        );
    }

    #[test]
    fn protocol_annotation_offers_protocol_values_not_annotations() {
        let items = complete("# @protocol \n", pos(0, 12));
        let labels = labels(&items);
        assert!(labels.contains(&"graphql"));
        assert!(labels.contains(&"socketio"));
        assert!(!labels.contains(&"@name"));
    }

    #[test]
    fn depends_annotation_offers_names_declared_in_the_document() {
        let content = "\
# @name Login
POST https://example.com/login

###
# @depends \nGET https://example.com/me
";
        let items = complete(content, pos(4, 11));
        assert_eq!(labels(&items), vec!["Login"]);
    }

    #[test]
    fn depends_annotation_on_unparseable_content_yields_nothing_rather_than_panicking() {
        assert!(complete("# @depends x\nnot a request\n", pos(0, 12)).is_empty());
    }

    #[test]
    fn open_braces_offer_variables_from_set_and_from_other_usages() {
        let content = "\
# @set token = $.token
GET https://example.com/a?x={{other}}
GET https://example.com/b?y={{
";
        let items = complete(content, pos(2, 30));
        let labels = labels(&items);
        assert!(labels.contains(&"{{token}}"), "got {labels:?}");
        assert!(labels.contains(&"{{other}}"), "got {labels:?}");
        // The inserted text must be the bare name - the `{{` is already typed.
        let token = items.iter().find(|i| i.label == "{{token}}").unwrap();
        assert_eq!(token.insert_text.as_deref(), Some("token"));
    }

    #[test]
    fn closed_braces_do_not_trigger_variable_completion() {
        let items = complete("GET https://x.com?a={{id}}\n", pos(0, 26));
        assert!(!labels(&items).iter().any(|l| l.starts_with("{{")));
    }

    #[test]
    fn a_partial_method_offers_http_methods() {
        let items = complete("GE\n", pos(0, 2));
        assert!(labels(&items).contains(&"GET"));
        assert!(labels(&items).contains(&"OPTIONS"));
    }

    #[test]
    fn a_content_type_header_offers_media_types() {
        let items = complete("Content-Type: \n", pos(0, 14));
        assert!(labels(&items).contains(&"application/json"));
    }

    #[test]
    fn an_unknown_header_offers_no_values() {
        assert!(complete("X-Custom: \n", pos(0, 10)).is_empty());
    }

    #[test]
    fn a_bare_word_offers_common_header_names() {
        let items = complete("Cont\n", pos(0, 4));
        assert!(items.iter().any(|i| i.label.starts_with("Content-Type")));
        // Accepting the item must not re-type the value example.
        let ct = items
            .iter()
            .find(|i| i.label.starts_with("Content-Type"))
            .unwrap();
        assert_eq!(ct.insert_text.as_deref(), Some("Content-Type: "));
    }

    #[test]
    fn a_position_past_the_end_of_the_line_does_not_panic() {
        // Every branch must tolerate an out-of-range character offset.
        for line in [
            "# @",
            "# @protocol http",
            "GET https://example.com",
            "Content-Type: application/json",
            "",
        ] {
            let content = format!("{line}\n");
            let _ = complete(&content, pos(0, 9_999));
        }
    }

    #[test]
    fn a_position_past_the_end_of_the_document_does_not_panic() {
        let _ = complete("GET https://example.com\n", pos(500, 3));
    }

    #[test]
    fn a_multibyte_line_does_not_panic_at_any_offset() {
        // `pos.character` is a UTF-16 offset; slicing the UTF-8 line with it
        // directly would land mid-character and panic.
        let line = "GET https://例え.com/日本語?x={{id}}";
        let content = format!("{line}\n");
        let units = line.chars().map(char::len_utf16).sum::<usize>();
        for i in 0..=units + 3 {
            let _ = complete(&content, pos(0, i as u32));
        }
    }
}
