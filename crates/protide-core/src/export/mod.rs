//! Export functionality for API collections

mod markdown;

pub use markdown::export_collection_markdown;

use std::path::Path;

/// Supported export formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Html,
}

/// Export a collection directory to the given format.
/// Returns the document as a String.
pub fn export_collection(root: &Path, format: ExportFormat) -> Result<String, String> {
    match format {
        ExportFormat::Markdown => export_collection_markdown(root),
        ExportFormat::Html => {
            let md = export_collection_markdown(root)?;
            Ok(markdown_to_html(&md))
        }
    }
}

/// Minimal markdown → HTML wrapper (wraps in a styled HTML page).
fn markdown_to_html(md: &str) -> String {
    // Simple conversion: wrap each line, handle headers/code fences at basic level.
    // For a full implementation a markdown crate could be added; this is sufficient
    // for a readable HTML export without adding dependencies.
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>API Documentation</title>
<style>
  body { font-family: system-ui, sans-serif; max-width: 900px; margin: 40px auto; padding: 0 20px; line-height: 1.6; color: #333; }
  h1 { border-bottom: 2px solid #333; padding-bottom: 8px; }
  h2 { border-bottom: 1px solid #ccc; padding-bottom: 4px; margin-top: 2em; }
  h3 { margin-top: 1.5em; }
  code { background: #f5f5f5; padding: 2px 6px; border-radius: 3px; font-size: 0.9em; }
  pre { background: #f5f5f5; padding: 16px; overflow-x: auto; border-left: 4px solid #666; }
  pre code { background: none; padding: 0; }
  .method { display: inline-block; padding: 2px 8px; color: #fff; border-radius: 3px; font-weight: bold; font-size: 0.85em; margin-right: 8px; }
  .GET    { background: #61affe; }
  .POST   { background: #49cc90; }
  .PUT    { background: #fca130; }
  .DELETE { background: #f93e3e; }
  .PATCH  { background: #50e3c2; color: #333; }
  hr { border: none; border-top: 1px solid #eee; margin: 2em 0; }
</style>
</head>
<body>
"#,
    );

    let mut in_code_block = false;
    for line in md.lines() {
        if line.starts_with("```") {
            if in_code_block {
                html.push_str("</code></pre>\n");
                in_code_block = false;
            } else {
                let lang = line.trim_start_matches('`').trim();
                html.push_str(&format!("<pre><code class=\"language-{}\">", escape_html(lang)));
                in_code_block = true;
            }
            continue;
        }
        if in_code_block {
            html.push_str(&escape_html(line));
            html.push('\n');
            continue;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", escape_html(rest)));
        } else if let Some(rest) = line.strip_prefix("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", escape_html(rest)));
        } else if let Some(rest) = line.strip_prefix("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", escape_html(rest)));
        } else if line.starts_with("---") {
            html.push_str("<hr>\n");
        } else if line.starts_with("- ") || line.starts_with("* ") {
            html.push_str(&format!("<li>{}</li>\n", inline_format(&line[2..])));
        } else if line.is_empty() {
            html.push_str("<br>\n");
        } else {
            html.push_str(&format!("<p>{}</p>\n", inline_format(line)));
        }
    }

    html.push_str("</body>\n</html>\n");
    html
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn inline_format(s: &str) -> String {
    let escaped = escape_html(s);
    // Handle inline `code`
    let mut out = String::new();
    let mut chars = escaped.chars().peekable();
    let mut in_code = false;
    while let Some(c) = chars.next() {
        if c == '`' {
            if in_code {
                out.push_str("</code>");
                in_code = false;
            } else {
                out.push_str("<code>");
                in_code = true;
            }
        } else if c == '*' && chars.peek() == Some(&'*') {
            // Skip bold for simplicity
            chars.next();
            out.push_str("<strong>");
        } else {
            out.push(c);
        }
    }
    out
}
