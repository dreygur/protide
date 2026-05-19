use tower_lsp::lsp_types::SemanticToken;

pub const TOK_KEYWORD: u32 = 0;
pub const TOK_STRING: u32 = 1;
pub const TOK_PROPERTY: u32 = 2;
pub const TOK_PARAMETER: u32 = 3;
pub const TOK_COMMENT: u32 = 4;
pub const TOK_NUMBER: u32 = 5;

pub fn tokenize(content: &str) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for (line_num, line) in content.lines().enumerate() {
        let ln = line_num as u32;
        let trimmed = line.trim_start();

        let tok = if trimmed.starts_with("###") {
            Some((0u32, line.len() as u32, TOK_KEYWORD))
        } else if trimmed.starts_with("# @") {
            Some((0, line.len() as u32, TOK_COMMENT))
        } else if trimmed.starts_with('#') {
            Some((0, line.len() as u32, TOK_COMMENT))
        } else if let Some(rest) = try_request_line(trimmed) {
            let method_end = line.find(' ').unwrap_or(line.len());
            let indent = (line.len() - trimmed.len()) as u32;
            tokens.push(make_token(
                ln,
                prev_line,
                indent,
                prev_start,
                method_end as u32,
                TOK_KEYWORD,
            ));
            prev_line = ln;
            prev_start = indent;
            let url_start = indent + method_end as u32 + 1;
            let url_len = rest.trim().len() as u32;
            Some((url_start, url_len, TOK_STRING))
        } else if trimmed.contains(':') && !trimmed.starts_with('{') {
            let colon = line.find(':').unwrap_or(0);
            let indent = (line.len() - trimmed.len()) as u32;
            tokens.push(make_token(
                ln,
                prev_line,
                indent,
                prev_start,
                colon as u32,
                TOK_PROPERTY,
            ));
            prev_line = ln;
            prev_start = indent;
            None
        } else {
            None
        };

        if let Some((start, len, tok_type)) = tok {
            tokens.push(make_token(ln, prev_line, start, prev_start, len, tok_type));
            prev_line = ln;
            prev_start = start;
        }

        // Highlight {{variables}} and numeric literals within the line
        highlight_inline(line, ln, &mut prev_line, &mut prev_start, &mut tokens);
    }

    tokens
}

fn highlight_inline(
    line: &str,
    ln: u32,
    prev_line: &mut u32,
    prev_start: &mut u32,
    tokens: &mut Vec<SemanticToken>,
) {
    let mut search = line;
    let mut offset = 0usize;
    while let Some(open) = search.find("{{") {
        if let Some(close) = search[open + 2..].find("}}") {
            let var_start = (offset + open) as u32;
            let var_len = (close + 4) as u32;
            tokens.push(make_token(ln, *prev_line, var_start, *prev_start, var_len, TOK_PARAMETER));
            *prev_line = ln;
            *prev_start = var_start;
            let skip = open + close + 4;
            offset += skip;
            search = &search[skip.min(search.len())..];
        } else {
            break;
        }
    }

    // Highlight standalone numeric values (e.g. status codes in comments, port numbers)
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c.is_ascii_digit() {
            let prev_char = if i > 0 { line.as_bytes().get(i - 1).copied() } else { None };
            if prev_char.map_or(true, |p| !p.is_ascii_alphanumeric() && p != b'_') {
                let end = line[i..]
                    .find(|ch: char| !ch.is_ascii_digit())
                    .map(|n| i + n)
                    .unwrap_or(line.len());
                let next = line.as_bytes().get(end).copied();
                if next.map_or(true, |n| !n.is_ascii_alphanumeric() && n != b'_') {
                    let start = i as u32;
                    let len = (end - i) as u32;
                    tokens.push(make_token(ln, *prev_line, start, *prev_start, len, TOK_NUMBER));
                    *prev_line = ln;
                    *prev_start = start;
                }
            }
        }
    }
}

pub fn try_request_line(s: &str) -> Option<&str> {
    let methods = [
        "GET ", "POST ", "PUT ", "PATCH ", "DELETE ", "HEAD ", "OPTIONS ",
        "WEBSOCKET ", "GRPC ",
    ];
    for m in &methods {
        if s.starts_with(m) {
            return Some(&s[m.len()..]);
        }
    }
    None
}

pub fn make_token(
    line: u32,
    prev_line: u32,
    start: u32,
    prev_start: u32,
    len: u32,
    tok_type: u32,
) -> SemanticToken {
    let delta_line = line - prev_line;
    let delta_start = if delta_line == 0 {
        start - prev_start
    } else {
        start
    };
    SemanticToken {
        delta_line,
        delta_start,
        length: len,
        token_type: tok_type,
        token_modifiers_bitset: 0,
    }
}
