use std::collections::{HashMap, HashSet};
use http_parser::ParseError;
use tower_lsp::lsp_types::*;

pub fn compute_diagnostics(content: &str) -> Vec<Diagnostic> {
    match http_parser::parse(content) {
        Ok(requests) => {
            let mut diags = check_references(content, &requests);
            diags.extend(detect_cycles(&requests));
            diags
        }
        Err(e) => vec![parse_error_diagnostic(&e)],
    }
}

fn check_references(content: &str, requests: &[http_parser::Request]) -> Vec<Diagnostic> {
    let names: HashSet<&str> = requests
        .iter()
        .filter_map(|r| r.meta.name.as_deref())
        .collect();

    let mut diagnostics = Vec::new();
    for req in requests {
        for dep in &req.meta.depends {
            if !names.contains(dep.as_str()) {
                if let Some(line_num) = find_depends_line(content, dep) {
                    diagnostics.push(Diagnostic {
                        range: line_range(line_num),
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: format!("Unknown request name: '{dep}' (not found in this file)"),
                        source: Some("protide-lsp".to_string()),
                        ..Default::default()
                    });
                }
            }
        }
    }
    diagnostics
}

fn detect_cycles(requests: &[http_parser::Request]) -> Vec<Diagnostic> {
    let name_to_line: HashMap<&str, u32> = requests
        .iter()
        .filter_map(|r| r.meta.name.as_deref().map(|n| (n, r.line as u32)))
        .collect();

    let adj: HashMap<&str, Vec<&str>> = requests
        .iter()
        .filter_map(|r| {
            r.meta.name.as_deref().map(|n| {
                let deps = r.meta.depends.iter()
                    .filter_map(|d| if name_to_line.contains_key(d.as_str()) { Some(d.as_str()) } else { None })
                    .collect();
                (n, deps)
            })
        })
        .collect();

    let mut color: HashMap<&str, u8> = HashMap::new();
    let mut diagnostics = Vec::new();

    for &name in adj.keys() {
        if color.get(name).copied().unwrap_or(0) == 0 {
            dfs(name, &adj, &mut color, &mut vec![], &name_to_line, &mut diagnostics);
        }
    }
    diagnostics
}

fn dfs<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    color: &mut HashMap<&'a str, u8>,
    path: &mut Vec<&'a str>,
    name_to_line: &HashMap<&'a str, u32>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    color.insert(node, 1);
    path.push(node);

    if let Some(neighbors) = adj.get(node) {
        for &nb in neighbors {
            match color.get(nb).copied().unwrap_or(0) {
                1 => {
                    let start = path.iter().position(|&n| n == nb).unwrap_or(0);
                    let cycle_str = format!("{} → {}", path[start..].join(" → "), nb);
                    if let Some(&line) = name_to_line.get(nb) {
                        diagnostics.push(Diagnostic {
                            range: line_range(line),
                            severity: Some(DiagnosticSeverity::WARNING),
                            message: format!("Circular @depends: {cycle_str}"),
                            source: Some("protide-lsp".to_string()),
                            ..Default::default()
                        });
                    }
                }
                0 => dfs(nb, adj, color, path, name_to_line, diagnostics),
                _ => {}
            }
        }
    }

    path.pop();
    color.insert(node, 2);
}

fn find_depends_line(content: &str, dep: &str) -> Option<u32> {
    content.lines().enumerate().find_map(|(i, line)| {
        let rest = line.trim_start().strip_prefix("# @depends")?;
        if rest.trim() == dep { Some(i as u32) } else { None }
    })
}

fn parse_error_diagnostic(e: &ParseError) -> Diagnostic {
    let line = match e {
        ParseError::UnexpectedToken { line, .. } => line.saturating_sub(1) as u32,
        ParseError::InvalidMethod { line, .. } => line.saturating_sub(1) as u32,
        ParseError::MissingUrl { line, .. } => line.saturating_sub(1) as u32,
        ParseError::InvalidUrl { line, .. } => line.saturating_sub(1) as u32,
        ParseError::InvalidProtocol { line, .. } => line.saturating_sub(1) as u32,
    };
    Diagnostic {
        range: line_range(line),
        severity: Some(DiagnosticSeverity::ERROR),
        message: e.to_string(),
        source: Some("protide-lsp".to_string()),
        ..Default::default()
    }
}

pub fn line_range(line: u32) -> Range {
    Range {
        start: Position { line, character: 0 },
        end: Position { line, character: u32::MAX },
    }
}
