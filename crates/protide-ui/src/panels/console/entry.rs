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
        let mut out = format!("[{}] {} {}", self.timestamp.format("%H:%M:%S"), self.method, self.url);
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
