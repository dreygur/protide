use crate::semantic_tokens::try_request_line;
use tower_lsp::lsp_types::*;

pub fn format_document(content: &str) -> Vec<TextEdit> {
    let mut edits = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut in_headers = false;
    let mut in_body = false;
    let mut body_lines: Vec<usize> = Vec::new();
    let mut is_json = false;

    for (i, &line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("###") {
            flush_body(&lines, &body_lines, is_json, &mut edits);
            body_lines.clear();
            in_headers = false;
            in_body = false;
            is_json = false;
            continue;
        }

        if in_body {
            body_lines.push(i);
            continue;
        }

        // A blank line ends the header block and opens the body region - but
        // only once a request line has actually been seen. Blank lines above
        // the request (after `# @name` annotations, say) used to flip this
        // flag, swallowing the whole request into a phantom "body" so nothing
        // in the block was ever formatted.
        if trimmed.is_empty() {
            in_body = in_headers;
            continue;
        }

        if try_request_line(trimmed).is_some() {
            in_headers = true;
            continue;
        }

        // Normalize header name casing
        if let Some((colon, name)) = header_name_at(trimmed) {
            let normalized = normalize_header(name);
            if normalized != name {
                let indent = line.len() - trimmed.len();
                let rest = &trimmed[colon..];
                edits.push(line_edit(
                    i as u32,
                    line.len() as u32,
                    format!("{}{normalized}{rest}", &line[..indent]),
                ));
            }
            if name.to_lowercase() == "content-type"
                && trimmed[colon + 1..]
                    .to_lowercase()
                    .contains("application/json")
            {
                is_json = true;
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
    let raw: String = body_lines
        .iter()
        .map(|&i| lines[i])
        .collect::<Vec<_>>()
        .join("\n");
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
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
        let last = body_lines.last().copied().unwrap_or(0) as u32;
        let extra: String = pretty_lines[body_lines.len()..].join("\n");
        edits.push(TextEdit {
            range: Range {
                start: Position {
                    line: last + 1,
                    character: 0,
                },
                end: Position {
                    line: last + 1,
                    character: 0,
                },
            },
            new_text: format!("{extra}\n"),
        });
    }
}

/// Split `Name: value` into `(colon offset, name)`, but only for lines that
/// really are headers.
///
/// A colon alone is not enough: request lines (`GET https://…`) and bare URLs
/// contain one too, and title-casing everything before it rewrote `GET` to
/// `Get` and `https://…` to `Https://…`, corrupting the request on every
/// "Format Document".
fn header_name_at(trimmed: &str) -> Option<(usize, &str)> {
    let colon = trimmed.find(':')?;
    let name = &trimmed[..colon];
    let is_token = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    (is_token && !trimmed[colon..].starts_with("://")).then_some((colon, name))
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
            end: Position {
                line,
                character: end_char,
            },
        },
        new_text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Apply `format_document`'s edits to `content` the way an LSP client
    /// would, so tests assert on the resulting document rather than on edit
    /// bookkeeping.
    fn formatted(content: &str) -> String {
        let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
        let mut appended: Vec<(usize, String)> = Vec::new();
        for edit in format_document(content) {
            let i = edit.range.start.line as usize;
            if edit.range.start == edit.range.end {
                appended.push((i, edit.new_text.trim_end_matches('\n').to_string()));
            } else if let Some(line) = lines.get_mut(i) {
                *line = edit.new_text;
            }
        }
        for (i, extra) in appended.into_iter().rev() {
            lines.insert(i, extra);
        }
        lines.join("\n")
    }

    #[test]
    fn header_names_are_title_cased() {
        let out = formatted("GET https://example.com\ncontent-type: application/json\n");
        assert!(
            out.contains("Content-Type: application/json"),
            "got {out:?}"
        );
    }

    #[test]
    fn already_correct_headers_produce_no_edits() {
        let content = "GET https://example.com\nContent-Type: application/json\nAccept: */*\n";
        assert!(format_document(content).is_empty());
    }

    #[test]
    fn header_indentation_and_value_are_preserved() {
        let out = formatted("GET https://example.com\n  x-api-KEY: Bearer ABC\n");
        assert!(out.contains("  X-Api-Key: Bearer ABC"), "got {out:?}");
    }

    // ── REGRESSION: the formatter must not rewrite request lines ─────────────
    // Any line containing a colon used to be treated as a header, so the
    // request line's own colon (in "https://") made the formatter title-case
    // the method: "GET https://x" became "Get https://x", corrupting every
    // request in the file on a single "Format Document".
    #[test]
    fn the_request_line_is_never_rewritten() {
        for line in [
            "GET https://example.com/users",
            "POST https://example.com/users",
            "  DELETE https://example.com/users/1",
        ] {
            let content = format!("{line}\n");
            assert!(
                format_document(&content).is_empty(),
                "formatting must leave the request line {line:?} alone",
            );
        }
    }

    #[test]
    fn a_bare_url_line_is_never_rewritten() {
        assert!(format_document("https://example.com/users\n").is_empty());
    }

    #[test]
    fn a_json_body_is_pretty_printed() {
        let content = "\
GET https://example.com
Content-Type: application/json

{\"a\":1,\"b\":2}
";
        let out = formatted(content);
        assert!(out.contains("\"a\": 1"), "got {out:?}");
        assert!(out.contains("\"b\": 2"), "got {out:?}");
    }

    // ── REGRESSION: annotations above the request must not hide the body ─────
    // A blank line used to open the "body" region unconditionally, so the
    // blank line between `# @name` and the request swallowed the whole block
    // and nothing in it was ever formatted.
    #[test]
    fn a_block_introduced_by_annotations_is_still_formatted() {
        let content = "\
# @name Create

POST https://example.com/users
content-type: application/json

{\"a\":1}
";
        let out = formatted(content);
        assert!(
            out.contains("Content-Type: application/json"),
            "headers after a `# @name` block must still be normalised, got {out:?}",
        );
        assert!(out.contains("\"a\": 1"), "got {out:?}");
    }

    #[test]
    fn an_invalid_json_body_is_left_untouched() {
        let content = "\
GET https://example.com
Content-Type: application/json

{not json at all
";
        let out = formatted(content);
        assert!(out.contains("{not json at all"), "got {out:?}");
    }

    #[test]
    fn a_non_json_body_is_left_untouched() {
        let content = "\
POST https://example.com
Content-Type: text/plain

{\"a\":1}
";
        assert!(format_document(content).is_empty());
    }

    #[test]
    fn each_hash_separated_block_is_formatted_independently() {
        let content = "\
### One
GET https://example.com/a
accept: */*

### Two
GET https://example.com/b
cache-control: no-cache
";
        let out = formatted(content);
        assert!(out.contains("Accept: */*"), "got {out:?}");
        assert!(out.contains("Cache-Control: no-cache"), "got {out:?}");
    }

    #[test]
    fn empty_and_whitespace_only_documents_produce_no_edits() {
        assert!(format_document("").is_empty());
        assert!(format_document("\n\n\n").is_empty());
    }

    #[test]
    fn a_multibyte_document_does_not_panic() {
        let content = "\
### 日本語のリクエスト
GET https://例え.com/ユーザー
content-type: application/json

{\"名前\":\"日本語\"}
";
        let _ = format_document(content);
    }
}
