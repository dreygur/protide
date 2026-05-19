use tower_lsp::lsp_types::*;

pub fn format_document(content: &str) -> Vec<TextEdit> {
    let mut edits = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut in_body = false;
    let mut body_lines: Vec<usize> = Vec::new();
    let mut is_json = false;

    for (i, &line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("###") {
            flush_body(&lines, &body_lines, is_json, &mut edits);
            body_lines.clear();
            in_body = false;
            is_json = false;
            continue;
        }

        if in_body {
            body_lines.push(i);
            continue;
        }

        // Empty line after headers = body region starts
        if trimmed.is_empty() {
            in_body = true;
            continue;
        }

        // Normalize header name casing
        if let Some(colon) = trimmed.find(':') {
            let name = &trimmed[..colon];
            let normalized = normalize_header(name);
            if normalized != name {
                let indent = line.len() - trimmed.len();
                let rest = &trimmed[colon..];
                edits.push(line_edit(i as u32, line.len() as u32, format!("{}{normalized}{rest}", &line[..indent])));
            }
            if name.to_lowercase() == "content-type" {
                if trimmed[colon + 1..].to_lowercase().contains("application/json") {
                    is_json = true;
                }
            }
        }
    }

    // Flush last block
    flush_body(&lines, &body_lines, is_json, &mut edits);
    edits
}

fn flush_body(lines: &[&str], body_lines: &[usize], is_json: bool, edits: &mut Vec<TextEdit>) {
    if !is_json || body_lines.is_empty() {
        return;
    }
    let raw: String = body_lines.iter().map(|&i| lines[i]).collect::<Vec<_>>().join("\n");
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) else { return };
    let pretty = serde_json::to_string_pretty(&val).unwrap_or(raw);
    let pretty_lines: Vec<&str> = pretty.lines().collect();

    for (offset, &body_ln) in body_lines.iter().enumerate() {
        let new_text = pretty_lines.get(offset).copied().unwrap_or("").to_string();
        let old_text = lines[body_ln];
        if new_text != old_text {
            edits.push(line_edit(body_ln as u32, old_text.len() as u32, new_text));
        }
    }
    // If pretty has more lines than original, append them after last body line
    if pretty_lines.len() > body_lines.len() {
        let last = *body_lines.last().unwrap() as u32;
        let extra: String = pretty_lines[body_lines.len()..].join("\n");
        edits.push(TextEdit {
            range: Range {
                start: Position { line: last + 1, character: 0 },
                end: Position { line: last + 1, character: 0 },
            },
            new_text: format!("{extra}\n"),
        });
    }
}

fn normalize_header(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

fn line_edit(line: u32, end_char: u32, new_text: String) -> TextEdit {
    TextEdit {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: end_char },
        },
        new_text,
    }
}
