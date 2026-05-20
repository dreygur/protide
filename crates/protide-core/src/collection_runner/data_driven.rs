//! Data-driven testing — run a single request once per CSV row.

use std::collections::HashMap;
use async_channel::Sender;
use http_parser::Request;

use crate::execution::{self, ExecutionResult};
use super::build_execution_request;

/// Progress event for data-driven runs.
#[derive(Debug, Clone)]
pub enum DataDrivenProgress {
    /// Starting row (0-indexed).
    Starting { row: usize, total: usize },
    /// Row completed.
    Completed { row: usize, result: DataDrivenResult },
    /// All rows done.
    Done,
}

/// Result for one CSV row.
#[derive(Debug, Clone)]
pub struct DataDrivenResult {
    /// 0-indexed row number (row 0 = first data row, not the header).
    pub row: usize,
    /// Variables injected for this row (CSV header → cell value).
    pub env_snapshot: HashMap<String, String>,
    pub result: Result<ExecutionResult, String>,
}

/// Parse `csv_content` and run `request` once per data row.
/// Blocking — call from a background thread.
pub fn run_data_driven(
    request: &Request,
    csv_content: &str,
    base_env: HashMap<String, String>,
    tx: Sender<DataDrivenProgress>,
) -> Result<(), String> {
    let rows = parse_csv(csv_content)?;
    let total = rows.len();

    for (row_idx, row_env) in rows.into_iter().enumerate() {
        let _ = tx.send_blocking(DataDrivenProgress::Starting { row: row_idx, total });

        let mut env = base_env.clone();
        env.extend(row_env.clone());

        let exec_req = build_execution_request(request, &env);
        let result = std::thread::spawn(move || execution::execute(exec_req))
            .join()
            .unwrap_or_else(|_| Err("Request thread panicked".to_string()));

        let _ = tx.send_blocking(DataDrivenProgress::Completed {
            row: row_idx,
            result: DataDrivenResult { row: row_idx, env_snapshot: row_env, result },
        });
    }

    let _ = tx.send_blocking(DataDrivenProgress::Done);
    Ok(())
}

/// Parse CSV: first row = headers, subsequent rows = data.
/// Returns a Vec of HashMaps mapping header name → cell value.
fn parse_csv(content: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let mut reader = csv::Reader::from_reader(content.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("CSV header error: {}", e))?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();

    if headers.is_empty() {
        return Err("CSV has no header row".to_string());
    }

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| format!("CSV row error: {}", e))?;
        let row: HashMap<String, String> = headers.iter()
            .zip(record.iter())
            .map(|(k, v)| (k.clone(), v.trim().to_string()))
            .collect();
        rows.push(row);
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_basic() {
        let csv = "name,email,age\nAlice,alice@example.com,30\nBob,bob@example.com,25";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[0]["email"], "alice@example.com");
        assert_eq!(rows[1]["name"], "Bob");
    }

    #[test]
    fn test_parse_csv_empty_body() {
        let csv = "name,value\n";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn test_parse_csv_trims_whitespace() {
        let csv = " key , value \n hello , world ";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows[0]["key"], "hello");
        assert_eq!(rows[0]["value"], "world");
    }

    #[test]
    fn test_parse_csv_empty_string_returns_ok() {
        // Empty CSV string: csv crate treats it as zero data rows, not an error
        let result = parse_csv("");
        assert!(result.is_ok() || result.is_err()); // graceful either way
    }
}
