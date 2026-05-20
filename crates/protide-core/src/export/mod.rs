//! Export functionality for API collections

mod markdown;
mod openapi;

pub use markdown::export_collection_markdown;
pub use openapi::export_openapi;

use std::path::Path;

/// Supported export formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Html,
    OpenApi,
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
        ExportFormat::OpenApi => export_openapi(root),
    }
}

/// Minimal markdown → HTML wrapper (wraps in a styled HTML page).
fn markdown_to_html(md: &str) -> String {
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
  table { border-collapse: collapse; width: 100%; margin: 1em 0; }
  th, td { border: 1px solid #ddd; padding: 8px 12px; text-align: left; }
  th { background: #f0f0f0; font-weight: 600; }
  tr:nth-child(even) { background: #fafafa; }
  hr { border: none; border-top: 1px solid #eee; margin: 2em 0; }
</style>
</head>
<body>
"#,
    );

    let lines: Vec<&str> = md.lines().collect();
    let mut i = 0;
    let mut in_code_block = false;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("```") {
            if in_code_block {
                html.push_str("</code></pre>\n");
                in_code_block = false;
            } else {
                let lang = line.trim_start_matches('`').trim();
                html.push_str(&format!("<pre><code class=\"language-{}\">", escape_html(lang)));
                in_code_block = true;
            }
            i += 1;
            continue;
        }
        if in_code_block {
            html.push_str(&escape_html(line));
            html.push('\n');
            i += 1;
            continue;
        }

        // Table: collect consecutive | lines, skip separator row (|---|)
        if line.starts_with('|') {
            let mut rows: Vec<&str> = Vec::new();
            while i < lines.len() && lines[i].starts_with('|') {
                rows.push(lines[i]);
                i += 1;
            }
            html.push_str("<table>\n");
            let mut first_data = true;
            for (ri, row) in rows.iter().enumerate() {
                // Separator row: all cells are dashes/colons
                let cells: Vec<&str> = row.trim_matches('|').split('|').collect();
                if cells.iter().all(|c| c.trim().chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')) {
                    continue;
                }
                let tag = if ri == 0 { "th" } else { "td" };
                if ri == 0 { html.push_str("<thead>\n"); }
                else if first_data { html.push_str("<tbody>\n"); first_data = false; }
                html.push_str("<tr>");
                for cell in &cells {
                    html.push_str(&format!("<{}>{}</{}>", tag, inline_format(cell.trim()), tag));
                }
                html.push_str("</tr>\n");
                if ri == 0 { html.push_str("</thead>\n"); }
            }
            if !first_data { html.push_str("</tbody>\n"); }
            html.push_str("</table>\n");
            continue;
        }

        if let Some(rest) = line.strip_prefix("#### ") {
            html.push_str(&format!("<h4>{}</h4>\n", inline_format(rest)));
        } else if let Some(rest) = line.strip_prefix("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", inline_format(rest)));
        } else if let Some(rest) = line.strip_prefix("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", inline_format(rest)));
        } else if let Some(rest) = line.strip_prefix("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", inline_format(rest)));
        } else if line.starts_with("---") {
            html.push_str("<hr>\n");
        } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            html.push_str(&format!("<li>{}</li>\n", inline_format(rest)));
        } else if line.is_empty() {
            // skip — blank lines are structural separators, not visible breaks
        } else {
            html.push_str(&format!("<p>{}</p>\n", inline_format(line)));
        }
        i += 1;
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
    let mut out = String::new();
    let mut chars = escaped.chars().peekable();
    let mut in_code = false;
    let mut in_bold = false;
    while let Some(c) = chars.next() {
        if c == '`' {
            if in_code { out.push_str("</code>"); } else { out.push_str("<code>"); }
            in_code = !in_code;
        } else if c == '*' && chars.peek() == Some(&'*') {
            chars.next();
            if in_bold { out.push_str("</strong>"); } else { out.push_str("<strong>"); }
            in_bold = !in_bold;
        } else {
            out.push(c);
        }
    }
    // close any unclosed tags
    if in_code { out.push_str("</code>"); }
    if in_bold { out.push_str("</strong>"); }
    out
}
