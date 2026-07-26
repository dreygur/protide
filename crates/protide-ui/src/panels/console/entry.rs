/// Severity tier for a console log entry.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    #[default]
    Info,
    Debug,
    Error,
}

/// Where a console entry originated.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub enum ConsoleEntrySource {
    #[default]
    Request,
    Script,
    System,
    Team,
}

#[derive(Clone, Debug)]
pub struct ConsoleEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub level: LogLevel,
    pub source: ConsoleEntrySource,
    pub protocol: String,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub response_body: String,
    /// Actionable troubleshooting steps shown in the context menu for DNS / IO errors.
    pub troubleshoot_hint: Option<String>,
}

impl ConsoleEntry {
    /// Build a team-event entry (peer joined/left, sync status)
    pub fn team(message: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Local::now(),
            level: LogLevel::Info,
            source: ConsoleEntrySource::Team,
            protocol: String::new(),
            method: String::new(),
            url: message.into(),
            status: 0,
            duration_ms: 0,
            error: None,
            response_body: String::new(),
            troubleshoot_hint: None,
        }
    }

    /// Build an entry for a failed `@set` chaining extraction, so a
    /// JSONPath that doesn't resolve is visible instead of silently leaving
    /// the target variable stale.
    pub fn extraction_error(message: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Local::now(),
            level: LogLevel::Error,
            source: ConsoleEntrySource::Request,
            protocol: String::new(),
            method: String::new(),
            url: message.into(),
            status: 0,
            duration_ms: 0,
            error: None,
            response_body: String::new(),
            troubleshoot_hint: None,
        }
    }

    /// Build a system diagnostic entry (P2P internals: mDNS, PAKE, DHT, listen addr)
    pub fn system(message: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Local::now(),
            level: LogLevel::Debug,
            source: ConsoleEntrySource::System,
            protocol: "SYS".to_string(),
            method: String::new(),
            url: message.into(),
            status: 0,
            duration_ms: 0,
            error: None,
            response_body: String::new(),
            troubleshoot_hint: None,
        }
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none() && (200..300).contains(&self.status)
    }

    pub fn is_error(&self) -> bool {
        self.level == LogLevel::Error || self.error.is_some() || self.status >= 400
    }

    /// Build a cURL command from this entry (best-effort, headers not stored).
    pub fn as_curl(&self) -> String {
        format!("curl -X {} \"{}\"", self.method, self.url)
    }

    /// Full error detail string suitable for clipboard copy.
    pub fn error_details(&self) -> String {
        let mut out = format!(
            "[{}] {} {}",
            self.timestamp.format("%H:%M:%S"),
            self.method,
            self.url
        );
        if self.status > 0 {
            out.push_str(&format!("\nStatus: {}", self.status));
        }
        if let Some(e) = &self.error {
            out.push_str(&format!("\nError: {}", e));
        }
        if let Some(hint) = &self.troubleshoot_hint {
            out.push_str(&format!("\n\nTroubleshooting:\n{}", hint));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_error_entry_is_flagged_as_an_error() {
        let entry = ConsoleEntry::extraction_error("@set token: $.items[10].id did not match");
        assert_eq!(entry.level, LogLevel::Error);
        assert!(
            entry.is_error(),
            "extraction_error entries must surface as errors, not blend in as info"
        );
        assert!(entry.url.contains("did not match"));
    }
}

#[cfg(test)]
mod classification_tests {
    use super::*;

    fn request_entry(status: u16, error: Option<&str>) -> ConsoleEntry {
        ConsoleEntry {
            timestamp: chrono::Local::now(),
            level: LogLevel::Info,
            source: ConsoleEntrySource::Request,
            protocol: "HTTP".to_string(),
            method: "GET".to_string(),
            url: "https://x.test/a".to_string(),
            status,
            duration_ms: 12,
            error: error.map(str::to_string),
            response_body: String::new(),
            troubleshoot_hint: None,
        }
    }

    #[test]
    fn only_a_clean_2xx_counts_as_success() {
        for status in [200, 201, 204, 299] {
            assert!(request_entry(status, None).is_success(), "status {status}");
        }
        for status in [0, 100, 199, 300, 400, 500] {
            assert!(!request_entry(status, None).is_success(), "status {status}");
        }
        assert!(
            !request_entry(200, Some("body decode failed")).is_success(),
            "a 2xx that still carried a transport error is not a success"
        );
    }

    #[test]
    fn errors_are_recognised_from_the_status_the_level_or_the_error_field() {
        assert!(request_entry(404, None).is_error());
        assert!(request_entry(500, None).is_error());
        assert!(request_entry(0, Some("dns failure")).is_error());
        let mut e = request_entry(200, None);
        e.level = LogLevel::Error;
        assert!(e.is_error(), "an explicitly Error-level entry is an error");
        assert!(!request_entry(200, None).is_error());
        assert!(
            !request_entry(302, None).is_error(),
            "a redirect is not an error"
        );
    }

    #[test]
    fn success_and_error_are_never_both_true() {
        for status in [0, 200, 204, 301, 399, 400, 404, 500, 599, u16::MAX] {
            for error in [None, Some("boom")] {
                let e = request_entry(status, error);
                assert!(
                    !(e.is_success() && e.is_error()),
                    "status {status} error {error:?} is both a success and an error"
                );
            }
        }
    }

    #[test]
    fn the_helper_constructors_pick_the_right_level_and_source() {
        let team = ConsoleEntry::team("peer joined");
        assert_eq!(team.source, ConsoleEntrySource::Team);
        assert_eq!(team.level, LogLevel::Info);
        assert!(!team.is_error());

        let system = ConsoleEntry::system("mDNS listening");
        assert_eq!(system.source, ConsoleEntrySource::System);
        assert_eq!(system.level, LogLevel::Debug);
        assert_eq!(system.protocol, "SYS");
        assert!(!system.is_error(), "a diagnostic is not an error");

        let extraction = ConsoleEntry::extraction_error("$.a did not match");
        assert_eq!(extraction.source, ConsoleEntrySource::Request);
        assert_eq!(extraction.level, LogLevel::Error);
    }

    #[test]
    fn error_details_include_every_field_that_is_present() {
        let mut e = request_entry(503, Some("connection reset"));
        e.troubleshoot_hint = Some("Check the host is reachable".to_string());
        let details = e.error_details();
        assert!(details.contains("GET https://x.test/a"), "{details}");
        assert!(details.contains("Status: 503"), "{details}");
        assert!(details.contains("Error: connection reset"), "{details}");
        assert!(details.contains("Troubleshooting:"), "{details}");
        assert!(details.contains("Check the host is reachable"), "{details}");
    }

    #[test]
    fn error_details_omit_the_fields_that_are_absent() {
        // status 0 means "never got a response" - printing "Status: 0" would be
        // a lie, and an absent error/hint must not leave empty labels behind.
        let details = request_entry(0, None).error_details();
        assert!(!details.contains("Status:"), "{details}");
        assert!(!details.contains("Error:"), "{details}");
        assert!(!details.contains("Troubleshooting:"), "{details}");
    }

    #[test]
    fn the_curl_snippet_carries_the_method_and_url() {
        let curl = request_entry(200, None).as_curl();
        assert_eq!(curl, "curl -X GET \"https://x.test/a\"");
    }

    #[test]
    fn a_multibyte_url_survives_both_copy_helpers() {
        let mut e = request_entry(500, Some("エラー"));
        e.url = "https://x.test/日本語?q=🎉".to_string();
        assert!(e.as_curl().contains("https://x.test/日本語?q=🎉"));
        assert!(e.error_details().contains("エラー"));
    }
}
