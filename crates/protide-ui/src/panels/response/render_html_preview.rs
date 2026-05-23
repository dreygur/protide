use super::*;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum HtmlStyle {
    Title,
    H1,
    H2,
    H3,
    Body,
    Code,
    Dim,
}

#[derive(Debug, Clone)]
pub(super) struct HtmlLine {
    pub text: String,
    pub style: HtmlStyle,
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&#x27;", "'")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…")
}

/// Extract the `<title>` text from an HTML document.
fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title>")? + 7;
    let end = lower[start..].find("</title>")? + start;
    let raw = html[start..end].trim();
    if raw.is_empty() { None } else { Some(decode_entities(raw)) }
}

/// Convert HTML into a flat list of styled lines for the preview canvas.
/// Strips `<script>`, `<style>`, `<head>` blocks and extracts meaningful text.
pub(super) fn extract_blocks(html: &str) -> Vec<HtmlLine> {
    let mut lines: Vec<HtmlLine> = Vec::new();

    if let Some(title) = extract_title(html) {
        lines.push(HtmlLine { text: title, style: HtmlStyle::Title });
    }

    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut pos = 0usize;
    let mut skip_depth = 0usize;
    let mut current = String::new();
    let mut current_style = HtmlStyle::Body;

    let flush = |lines: &mut Vec<HtmlLine>, current: &mut String, style: &HtmlStyle| {
        let t = current.split_whitespace().collect::<Vec<_>>().join(" ");
        if !t.is_empty() {
            lines.push(HtmlLine { text: decode_entities(&t), style: style.clone() });
        }
        current.clear();
    };

    while pos < len {
        if bytes[pos] != b'<' {
            if skip_depth == 0 {
                let start = pos;
                while pos < len && bytes[pos] != b'<' { pos += 1; }
                let chunk = &html[start..pos];
                let normalized = chunk.replace(['\n', '\r', '\t'], " ");
                current.push_str(&normalized);
            } else {
                pos += 1;
            }
            continue;
        }

        // Read to end of tag
        let tag_start = pos;
        pos += 1;
        while pos < len {
            match bytes[pos] {
                b'"' => { pos += 1; while pos < len && bytes[pos] != b'"' { pos += 1; } if pos < len { pos += 1; } }
                b'\'' => { pos += 1; while pos < len && bytes[pos] != b'\'' { pos += 1; } if pos < len { pos += 1; } }
                b'>' => { pos += 1; break; }
                _ => { pos += 1; }
            }
        }
        let tag_raw = &html[tag_start..pos];
        let inner = tag_raw.trim_start_matches('<').trim_end_matches('>').trim();
        let is_closing = inner.starts_with('/');
        let name_part = if is_closing { &inner[1..] } else { inner };
        let name = name_part.split_whitespace().next().unwrap_or("").to_lowercase();

        match name.as_str() {
            "script" | "style" | "head" => {
                if !is_closing {
                    flush(&mut lines, &mut current, &current_style);
                    skip_depth += 1;
                } else {
                    skip_depth = skip_depth.saturating_sub(1);
                }
            }
            _ if skip_depth > 0 => {}
            "title" if is_closing => {
                // title was captured above; skip inline text
                current.clear();
            }
            "h1" if !is_closing => {
                flush(&mut lines, &mut current, &current_style);
                current_style = HtmlStyle::H1;
            }
            "h2" if !is_closing => {
                flush(&mut lines, &mut current, &current_style);
                current_style = HtmlStyle::H2;
            }
            "h3" | "h4" | "h5" | "h6" if !is_closing => {
                flush(&mut lines, &mut current, &current_style);
                current_style = HtmlStyle::H3;
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" if is_closing => {
                flush(&mut lines, &mut current, &current_style);
                current_style = HtmlStyle::Body;
            }
            "p" | "li" | "dt" | "dd" | "blockquote" if !is_closing => {
                flush(&mut lines, &mut current, &current_style);
            }
            "p" | "blockquote" if is_closing => {
                flush(&mut lines, &mut current, &current_style);
            }
            "div" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside"
            if !is_closing => {
                flush(&mut lines, &mut current, &current_style);
            }
            "div" | "section" | "article" | "main" | "header" | "footer" if is_closing => {
                flush(&mut lines, &mut current, &current_style);
            }
            "br" | "hr" => {
                flush(&mut lines, &mut current, &current_style);
                if name == "hr" {
                    lines.push(HtmlLine { text: "─".repeat(40), style: HtmlStyle::Dim });
                }
            }
            "pre" | "code" if !is_closing => {
                flush(&mut lines, &mut current, &current_style);
                current_style = HtmlStyle::Code;
            }
            "pre" | "code" if is_closing => {
                flush(&mut lines, &mut current, &current_style);
                current_style = HtmlStyle::Body;
            }
            _ => {}
        }
    }
    flush(&mut lines, &mut current, &current_style);

    // Deduplicate consecutive blank-looking lines
    let mut result = Vec::with_capacity(lines.len());
    let mut prev_empty = false;
    for line in lines {
        let empty = line.text.trim().is_empty();
        if empty && prev_empty { continue; }
        prev_empty = empty;
        if !line.text.trim().is_empty() {
            result.push(line);
        }
    }
    result
}

impl ResponsePanel {
    /// Render the Preview tab: strip `<script>`/`<style>` and lay out HTML blocks
    /// as native GPUI elements with per-style typography.
    pub(super) fn render_html_preview(&self, body: &str, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let blocks = extract_blocks(body);

        if blocks.is_empty() {
            return div()
                .flex_1()
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_muted)
                        .child("No readable content"),
                )
                .into_any_element();
        }

        let rendered: Vec<gpui::AnyElement> = blocks.into_iter().map(|line| {
            match line.style {
                HtmlStyle::Title => div()
                    .w_full()
                    .pb(px(8.0))
                    .mb(px(4.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .text_size(px(15.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme.colors.text_primary)
                    .child(line.text)
                    .into_any_element(),

                HtmlStyle::H1 => div()
                    .w_full()
                    .pt(px(12.0))
                    .pb(px(4.0))
                    .text_size(px(17.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme.colors.text_primary)
                    .child(line.text)
                    .into_any_element(),

                HtmlStyle::H2 => div()
                    .w_full()
                    .pt(px(10.0))
                    .pb(px(2.0))
                    .text_size(px(14.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.colors.text_primary)
                    .child(line.text)
                    .into_any_element(),

                HtmlStyle::H3 => div()
                    .w_full()
                    .pt(px(8.0))
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.colors.text_secondary)
                    .child(line.text)
                    .into_any_element(),

                HtmlStyle::Code => div()
                    .w_full()
                    .my(px(4.0))
                    .p(px(6.0))
                    .bg(theme.colors.bg_tertiary)
                    .border_l_4()
                    .border_color(theme.colors.accent.opacity(0.4))
                    .text_size(px(11.5))
                    .text_color(theme.colors.text_secondary)
                    .child(line.text)
                    .into_any_element(),

                HtmlStyle::Dim => div()
                    .w_full()
                    .py(px(4.0))
                    .text_size(px(11.0))
                    .text_color(theme.colors.text_muted.opacity(0.5))
                    .child(line.text)
                    .into_any_element(),

                HtmlStyle::Body => div()
                    .w_full()
                    .py(px(2.0))
                    .text_size(px(12.5))
                    .text_color(theme.colors.text_secondary)
                    .child(line.text)
                    .into_any_element(),
            }
        }).collect();

        div()
            .flex_1()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .p(px(4.0))
            .bg(theme.colors.bg_primary)
            .children(rendered)
            .into_any_element()
    }
}
