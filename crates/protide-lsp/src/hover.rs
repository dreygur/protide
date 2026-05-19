use tower_lsp::lsp_types::*;
use crate::symbols::var_at_cursor;

pub fn hover_at(content: &str, pos: Position) -> Option<Hover> {
    let line = content.lines().nth(pos.line as usize).unwrap_or("");

    // {{variable}} hover — show @set expression or note it's an env var
    if let Some(var_name) = var_at_cursor(line, pos.character as usize) {
        let value = find_set_expr(content, var_name)
            .map(|expr| format!("**`{{{{{var_name}}}}}`** → `{expr}`"))
            .unwrap_or_else(|| format!("**`{{{{{var_name}}}}}`** — environment variable"));
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
        "GET" => "**GET** — Retrieve a resource. Safe and idempotent.",
        "POST" => "**POST** — Submit data to create or process a resource.",
        "PUT" => "**PUT** — Replace a resource entirely. Idempotent.",
        "PATCH" => "**PATCH** — Partially update a resource.",
        "DELETE" => "**DELETE** — Remove a resource. Idempotent.",
        "HEAD" => "**HEAD** — Same as GET but returns only headers.",
        "OPTIONS" => "**OPTIONS** — Describe communication options for a resource.",
        "@NAME" | "NAME" if line.trim_start().starts_with("# @") => {
            "**@name** — Assigns a name to this request for use in chaining with `@depends`."
        }
        "@DESCRIPTION" | "DESCRIPTION" if line.trim_start().starts_with("# @") => {
            "**@description** — Human-readable description of this request. Included in exported docs."
        }
        "@PROTOCOL" | "PROTOCOL" if line.trim_start().starts_with("# @") => {
            "**@protocol** — Override protocol detection.\nValues: `http`, `graphql`, `websocket`, `grpc`, `trpc`, `socketio`"
        }
        "@SET" | "SET" if line.trim_start().starts_with("# @") => {
            "**@set** — Extract a value from the response and store it as a variable.\nSyntax: `# @set varName = $.path.to.value`"
        }
        "@DEPENDS" | "DEPENDS" if line.trim_start().starts_with("# @") => {
            "**@depends** — Declare that this request depends on another named request."
        }
        "@PROTO" | "PROTO" if line.trim_start().starts_with("# @") => {
            "**@proto** — Path to the .proto file for gRPC requests."
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
        if rest[..eq].trim() == var_name { Some(rest[eq + 1..].trim()) } else { None }
    })
}

pub fn word_at(line: &str, char_idx: usize) -> &str {
    let chars: Vec<char> = line.chars().collect();
    let start = chars[..char_idx.min(chars.len())]
        .iter()
        .rposition(|c| !c.is_alphanumeric() && *c != '_' && *c != '@')
        .map(|i| i + 1)
        .unwrap_or(0);
    let end = chars[char_idx.min(chars.len())..]
        .iter()
        .position(|c| !c.is_alphanumeric() && *c != '_' && *c != '@')
        .map(|i| char_idx + i)
        .unwrap_or(chars.len());
    let byte_start = chars[..start].iter().map(|c| c.len_utf8()).sum::<usize>();
    let byte_end = chars[..end].iter().map(|c| c.len_utf8()).sum::<usize>();
    &line[byte_start..byte_end]
}
