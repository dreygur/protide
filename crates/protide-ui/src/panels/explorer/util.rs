/// Sanitize a string to be used as a filename
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Convert a Request to .http file content
pub fn request_to_http_content(request: &http_parser::Request) -> Result<String, String> {
    let mut content = String::new();

    if let Some(name) = &request.meta.name {
        content.push_str(&format!("# @name {}\n", name));
    }

    if let Some(desc) = &request.meta.description {
        content.push_str(&format!(
            "# @description {}\n",
            desc.lines().next().unwrap_or("")
        ));
    }

    if !content.is_empty() {
        content.push('\n');
    }

    content.push_str(&format!("{} {}\n", request.method.as_str(), request.url));

    for header in &request.headers {
        if header.enabled {
            content.push_str(&format!("{}: {}\n", header.key, header.value));
        }
    }

    if let Some(body) = &request.body {
        content.push('\n');
        content.push_str(body);
        if !body.ends_with('\n') {
            content.push('\n');
        }
    }

    Ok(content)
}
