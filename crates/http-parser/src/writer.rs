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

/// A field that must stay on its own line, with CR and LF turned into
/// spaces: a line break in a name, URL or header would otherwise start a new
/// line the parser reads as an annotation, header or script.
pub fn single_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
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
            out.push_str(&format!("# @name {}\n", single_line(name)));
        }
        if let Some(desc) = &meta.description {
            out.push_str(&format!("# @description {}\n", single_line(desc)));
        }
        if let Some(protocol) = meta.protocol {
            out.push_str(&format!("# @protocol {}\n", protocol_name(protocol)));
        }
        if let Some(proto) = &meta.proto_path {
            out.push_str(&format!("# @proto {}\n", single_line(proto)));
        }
        if !meta.depends.is_empty() {
            out.push_str(&format!(
                "# @depends {}\n",
                single_line(&meta.depends.join(", "))
            ));
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!(
            "{} {}\n",
            self.method.as_str(),
            single_line(&self.url)
        ));
        for h in &self.headers {
            let prefix = if h.enabled { "" } else { "# " };
            out.push_str(&format!(
                "{prefix}{}: {}\n",
                single_line(&h.key),
                single_line(&h.value)
            ));
        }
        if let Some(body) = &self.body {
            out.push('\n');
            push_section(&mut out, body);
        }
        if !meta.variable_extractions.is_empty() {
            out.push('\n');
        }
        for set in &meta.variable_extractions {
            out.push_str(&format!(
                "# @set {} = {}\n",
                single_line(&set.name),
                single_line(&set.expression)
            ));
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
    fn a_bare_request_is_just_its_request_single_line() {
        let req = Request::new(HttpMethod::Get, "https://x.test".into());
        assert_eq!(req.to_http(), "GET https://x.test\n");
    }

    #[test]
    fn a_multi_line_description_is_written_on_one_single_line() {
        let mut req = Request::new(HttpMethod::Get, "https://x.test".into());
        req.meta.description = Some("first\nsecond".into());
        assert_eq!(
            req.to_http(),
            "# @description first second\n\nGET https://x.test\n"
        );
    }

    #[test]
    fn a_newline_in_a_single_line_field_cannot_inject_a_single_line() {
        let evil = "x\r\n# @pre-script\nfetch(1)\r# @set a = b";
        let mut req = Request::new(HttpMethod::Get, format!("https://x.test/{evil}"));
        req.meta.name = Some(evil.into());
        req.meta.proto_path = Some(evil.into());
        req.meta.depends = vec![evil.into()];
        req.headers = vec![
            KeyValue::new(evil, evil),
            KeyValue {
                key: "X-Off".into(),
                value: evil.into(),
                enabled: false,
            },
        ];
        req.meta.variable_extractions = vec![VariableExtraction {
            name: evil.into(),
            expression: evil.into(),
        }];

        let text = req.to_http();
        assert!(
            text.lines().all(|l| !l.starts_with("# @pre-script")),
            "{text}"
        );
        assert_eq!(text.matches('\r').count(), 0, "{text}");
        let back = &crate::parse(&text).unwrap()[0];
        assert_eq!(back.scripts.pre_script, None);
        assert_eq!(back.meta.variable_extractions.len(), 1);
        assert_eq!(
            back.meta.name.as_deref(),
            Some("x  # @pre-script fetch(1) # @set a = b")
        );
    }
}
