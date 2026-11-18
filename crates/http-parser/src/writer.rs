//! Serializes a [`Request`] back to .http text the parser reads back.

use crate::ast::{Protocol, Request};

fn protocol_name(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::Http => "http",
        Protocol::GraphQL => "graphql",
        Protocol::WebSocket => "websocket",
        Protocol::Grpc => "grpc",
        Protocol::SocketIO => "socketio",
        Protocol::Trpc => "trpc",
    }
}

/// Bodies and scripts are parsed as lines joined by `\n`, so a trailing
/// newline in `text` stands for a blank line and must be kept as one.
fn push_section(out: &mut String, text: &str) {
    out.push_str(text);
    out.push('\n');
}

impl Request {
    /// The request as .http text, without a `###` line: annotations, request
    /// line, headers (disabled ones commented out), body, `@set` lines and
    /// scripts.
    pub fn to_http(&self) -> String {
        let meta = &self.meta;
        let mut out = String::new();
        if let Some(name) = &meta.name {
            out.push_str(&format!("# @name {name}\n"));
        }
        // A description can only span one annotation line; the rest would
        // read back as stray content.
        if let Some(desc) = &meta.description {
            let first = desc.lines().next().unwrap_or("");
            out.push_str(&format!("# @description {first}\n"));
        }
        if let Some(protocol) = meta.protocol {
            out.push_str(&format!("# @protocol {}\n", protocol_name(protocol)));
        }
        if let Some(proto) = &meta.proto_path {
            out.push_str(&format!("# @proto {proto}\n"));
        }
        if !meta.depends.is_empty() {
            out.push_str(&format!("# @depends {}\n", meta.depends.join(", ")));
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("{} {}\n", self.method.as_str(), self.url));
        for h in &self.headers {
            let prefix = if h.enabled { "" } else { "# " };
            out.push_str(&format!("{prefix}{}: {}\n", h.key, h.value));
        }
        if let Some(body) = &self.body {
            out.push('\n');
            push_section(&mut out, body);
        }
        if !meta.variable_extractions.is_empty() {
            out.push('\n');
        }
        for set in &meta.variable_extractions {
            out.push_str(&format!("# @set {} = {}\n", set.name, set.expression));
        }
        let scripts = [
            ("pre-script", &self.scripts.pre_script),
            ("post-script", &self.scripts.post_script),
            ("tests", &self.scripts.tests),
        ];
        // A blank line between two scripts would become part of the first.
        let mut gap = "\n";
        for (marker, script) in scripts {
            if let Some(script) = script {
                out.push_str(&format!("{gap}# @{marker}\n"));
                push_section(&mut out, script);
                gap = "";
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{HttpMethod, KeyValue, Request, VariableExtraction};
    use pretty_assertions::assert_eq;

    fn full() -> Request {
        let mut r = Request::new(HttpMethod::Post, "{{base}}/users".into());
        r.meta.name = Some("create-user".into());
        r.meta.description = Some("Creates one".into());
        r.meta.protocol = Some(crate::Protocol::Http);
        r.meta.depends = vec!["login".into(), "me".into()];
        r.meta.variable_extractions = vec![VariableExtraction {
            name: "id".into(),
            expression: "$.id".into(),
        }];
        r.headers = vec![
            KeyValue::new("Content-Type", "application/json"),
            KeyValue {
                key: "X-Off".into(),
                value: "1".into(),
                enabled: false,
            },
        ];
        r.body = Some("{\"name\": \"a\"}".into());
        r.scripts.pre_script = Some("log(1)".into());
        r.scripts.tests = Some("expect(response.status).toBe(201);".into());
        r
    }

    #[test]
    fn every_field_round_trips_through_the_parser() {
        let req = full();
        let back = crate::parse(&req.to_http()).unwrap();
        assert_eq!(back.len(), 1);
        let back = &back[0];
        assert_eq!(back.meta.name, req.meta.name);
        assert_eq!(back.meta.description, req.meta.description);
        assert_eq!(back.meta.protocol, req.meta.protocol);
        assert_eq!(back.meta.depends, req.meta.depends);
        assert_eq!(back.meta.variable_extractions.len(), 1);
        assert_eq!(back.meta.variable_extractions[0].expression, "$.id");
        assert_eq!(back.method, req.method);
        assert_eq!(back.url, req.url);
        assert_eq!(back.headers, req.headers);
        assert_eq!(back.body, req.body);
        assert_eq!(back.scripts.pre_script, req.scripts.pre_script);
        assert_eq!(back.scripts.post_script, None);
        assert_eq!(back.scripts.tests, req.scripts.tests);
    }

    #[test]
    fn a_bare_request_is_just_its_request_line() {
        let req = Request::new(HttpMethod::Get, "https://x.test".into());
        assert_eq!(req.to_http(), "GET https://x.test\n");
    }

    #[test]
    fn only_the_first_description_line_is_written() {
        let mut req = Request::new(HttpMethod::Get, "https://x.test".into());
        req.meta.description = Some("first\nsecond".into());
        assert_eq!(
            req.to_http(),
            "# @description first\n\nGET https://x.test\n"
        );
    }
}
