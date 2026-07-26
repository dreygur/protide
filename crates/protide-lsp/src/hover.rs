use crate::symbols::{utf16_offset_to_byte_offset, var_at_cursor};
use tower_lsp::lsp_types::*;

pub fn hover_at(content: &str, pos: Position) -> Option<Hover> {
    let line = content.lines().nth(pos.line as usize).unwrap_or("");

    // {{variable}} hover - show @set expression or note it's an env var
    if let Some(var_name) = var_at_cursor(line, pos.character as usize) {
        let value = find_set_expr(content, var_name)
            .map(|expr| format!("**`{{{{{var_name}}}}}`** → `{expr}`"))
            .unwrap_or_else(|| format!("**`{{{{{var_name}}}}}`** - environment variable"));
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: None,
        });
    }

    let word = word_at(line, pos.character as usize);
    let docs = match word.to_uppercase().as_str() {
        "GET" => "**GET** - Retrieve a resource. Safe and idempotent.",
        "POST" => "**POST** - Submit data to create or process a resource.",
        "PUT" => "**PUT** - Replace a resource entirely. Idempotent.",
        "PATCH" => "**PATCH** - Partially update a resource.",
        "DELETE" => "**DELETE** - Remove a resource. Idempotent.",
        "HEAD" => "**HEAD** - Same as GET but returns only headers.",
        "OPTIONS" => "**OPTIONS** - Describe communication options for a resource.",
        "@NAME" | "NAME" if line.trim_start().starts_with("# @") => {
            "**@name** - Assigns a name to this request for use in chaining with `@depends`."
        }
        "@DESCRIPTION" | "DESCRIPTION" if line.trim_start().starts_with("# @") => {
            "**@description** - Human-readable description of this request. Included in exported docs."
        }
        "@PROTOCOL" | "PROTOCOL" if line.trim_start().starts_with("# @") => {
            "**@protocol** - Override protocol detection.\nValues: `http`, `graphql`, `websocket`, `grpc`, `trpc`, `socketio`"
        }
        "@SET" | "SET" if line.trim_start().starts_with("# @") => {
            "**@set** - Extract a value from the response and store it as a variable.\nSyntax: `# @set varName = $.path.to.value`"
        }
        "@DEPENDS" | "DEPENDS" if line.trim_start().starts_with("# @") => {
            "**@depends** - Declare that this request depends on another named request."
        }
        "@PROTO" | "PROTO" if line.trim_start().starts_with("# @") => {
            "**@proto** - Path to the .proto file for gRPC requests."
        }
        _ => return None,
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: docs.to_string(),
        }),
        range: None,
    })
}

fn find_set_expr<'a>(content: &'a str, var_name: &str) -> Option<&'a str> {
    content.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("# @set")?;
        let rest = rest.trim_start();
        let eq = rest.find('=')?;
        if rest[..eq].trim() == var_name {
            Some(rest[eq + 1..].trim())
        } else {
            None
        }
    })
}

/// Word surrounding `utf16_cursor`, an LSP `Position.character` (a UTF-16
/// code-unit offset). Treating that offset as a `char` index silently picked
/// the wrong word whenever an astral-plane character (most emoji, 2 UTF-16
/// units but 1 `char`) preceded the cursor on the same line.
pub fn word_at(line: &str, utf16_cursor: usize) -> &str {
    fn is_word(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '@'
    }
    let cursor = utf16_offset_to_byte_offset(line, utf16_cursor);
    let start = line[..cursor]
        .rfind(|c: char| !is_word(c))
        .map(|i| i + line[i..].chars().next().map_or(1, char::len_utf8))
        .unwrap_or(0);
    let end = line[cursor..]
        .find(|c: char| !is_word(c))
        .map(|i| cursor + i)
        .unwrap_or(line.len());
    &line[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    fn text(h: &Hover) -> &str {
        match &h.contents {
            HoverContents::Markup(m) => &m.value,
            _ => panic!("expected markup hover contents"),
        }
    }

    #[test]
    fn hovering_a_method_documents_it() {
        let h = hover_at("GET https://example.com\n", pos(0, 1)).unwrap();
        assert!(text(&h).contains("**GET**"));
    }

    #[test]
    fn hovering_a_lowercase_method_still_documents_it() {
        let h = hover_at("delete https://example.com\n", pos(0, 2)).unwrap();
        assert!(text(&h).contains("**DELETE**"));
    }

    #[test]
    fn hovering_a_variable_with_a_set_declaration_shows_the_expression() {
        let content = "# @set token = $.data.token\nGET https://x.com?t={{token}}\n";
        let h = hover_at(content, pos(1, 24)).unwrap();
        assert!(text(&h).contains("$.data.token"), "got {}", text(&h));
    }

    #[test]
    fn hovering_an_undeclared_variable_calls_it_an_environment_variable() {
        let h = hover_at("GET https://x.com?t={{apiKey}}\n", pos(1 - 1, 24)).unwrap();
        assert!(
            text(&h).contains("environment variable"),
            "got {}",
            text(&h)
        );
    }

    #[test]
    fn hovering_an_annotation_documents_it() {
        let h = hover_at("# @depends Login\nGET https://x.com\n", pos(0, 5)).unwrap();
        assert!(text(&h).contains("**@depends**"));
    }

    #[test]
    fn the_annotation_docs_do_not_leak_onto_ordinary_lines() {
        // "name" only means "@name" inside a `# @` annotation line.
        assert!(hover_at("name\n", pos(0, 2)).is_none());
    }

    #[test]
    fn hovering_a_word_with_no_documentation_is_none() {
        assert!(hover_at("Content-Type: application/json\n", pos(0, 2)).is_none());
    }

    #[test]
    fn a_position_past_the_end_of_the_document_is_none_not_a_panic() {
        assert!(hover_at("GET https://example.com\n", pos(999, 999)).is_none());
    }

    #[test]
    fn a_position_past_the_end_of_the_line_does_not_panic() {
        let _ = hover_at("GET https://example.com\n", pos(0, 9_999));
    }

    // ── word_at ──────────────────────────────────────────────────────────────

    #[test]
    fn word_at_returns_the_token_under_the_cursor() {
        assert_eq!(word_at("GET https://x.com", 0), "GET");
        assert_eq!(word_at("GET https://x.com", 2), "GET");
        assert_eq!(word_at("# @name Login", 4), "@name");
    }

    #[test]
    fn word_at_on_the_trailing_edge_of_a_word_still_returns_that_word() {
        // Cursor sitting immediately after "GET" - editors treat that as
        // hovering the word, so `GET |https://…` must still document GET.
        assert_eq!(word_at("GET https://x.com", 3), "GET");
    }

    #[test]
    fn word_at_surrounded_by_separators_is_empty() {
        assert_eq!(word_at("GET  https://x.com", 4), "");
    }

    #[test]
    fn word_at_past_the_end_of_the_line_does_not_panic() {
        assert_eq!(word_at("GET", 9_999), "GET");
        assert_eq!(word_at("", 9_999), "");
    }

    #[test]
    fn word_at_uses_utf16_offsets_not_char_counts() {
        // '😀' is one `char` but *two* UTF-16 code units. An LSP client puts
        // the cursor on "GET" at character 3, and treating that as a char
        // index would have selected the wrong token.
        let line = "😀 GET";
        assert_eq!(word_at(line, 3), "GET");
        assert_eq!(word_at(line, 5), "GET");
    }

    #[test]
    fn word_at_never_panics_on_a_multibyte_line() {
        let line = "# @name 日本語Ünïcödé😀";
        let units = line.chars().map(char::len_utf16).sum::<usize>();
        for i in 0..=units + 3 {
            let _ = word_at(line, i);
        }
    }
}
