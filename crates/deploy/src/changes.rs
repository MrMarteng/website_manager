//! Reads rclone's JSON log (`--use-json-log --verbose --stats …`): file changes for real runs
//! and dry runs, transfer statistics for the progress display, and readable log text.

use serde::{Deserialize, Serialize};

/// Kind of change to a file on the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// New or changed file.
    Upload,
    /// File removed from the server.
    Delete,
}

/// Transfer statistics rclone reports every second (and once at the end).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TransferStats {
    /// Bytes transferred so far.
    pub bytes: u64,
    /// Bytes to transfer as known so far (grows while rclone compares).
    pub total_bytes: u64,
    /// Files transferred so far.
    pub transfers: u64,
    /// Files to transfer as known so far.
    pub total_transfers: u64,
    /// Files compared so far.
    pub checks: u64,
    /// Files to compare as known so far.
    pub total_checks: u64,
    /// Estimated seconds left; `None` while rclone cannot tell.
    pub eta: Option<f64>,
    /// Bytes per second.
    pub speed: f64,
    /// Errors so far.
    pub errors: u64,
}

/// Meaning of one line of rclone output.
#[derive(Debug, Clone, PartialEq)]
pub enum LogEvent {
    /// A file was (or in a dry run would be) uploaded or deleted; path relative to the mapping.
    Change {
        /// Upload or deletion.
        kind: ChangeKind,
        /// Path relative to the mapping's target.
        path: String,
        /// The message formatted for the log.
        text: String,
    },
    /// Periodic statistics.
    Stats {
        /// The numbers.
        stats: TransferStats,
        /// rclone's multi-line summary.
        text: String,
    },
    /// Any other message, already formatted for the log.
    Text(String),
}

#[derive(Deserialize)]
struct JsonLine {
    #[serde(default)]
    level: String,
    #[serde(default)]
    msg: String,
    object: Option<String>,
    stats: Option<TransferStats>,
}

/// Interprets one line of rclone output. Lines that are not JSON (for example a crash message)
/// are passed through as text.
#[must_use]
pub fn parse_json_line(line: &str) -> LogEvent {
    let Ok(entry) = serde_json::from_str::<JsonLine>(line) else {
        return LogEvent::Text(line.to_owned());
    };
    if let Some(stats) = entry.stats {
        return LogEvent::Stats {
            stats,
            text: entry.msg.trim().to_owned(),
        };
    }
    let level = entry.level.to_uppercase();
    let Some(object) = entry.object else {
        return LogEvent::Text(format!("{level}: {}", entry.msg));
    };
    let text = format!("{level}: {object}: {}", entry.msg);
    match change_kind(&entry.msg) {
        Some(kind) => LogEvent::Change {
            kind,
            path: object,
            text,
        },
        None => LogEvent::Text(text),
    }
}

fn change_kind(message: &str) -> Option<ChangeKind> {
    let uploads = [
        "Skipped copy as --dry-run",
        "Copied (",
        "Updated modification time",
    ];
    if uploads.iter().any(|prefix| message.starts_with(prefix)) {
        Some(ChangeKind::Upload)
    } else if message.starts_with("Skipped delete as --dry-run") || message == "Deleted" {
        Some(ChangeKind::Delete)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(line: &str) -> Option<(ChangeKind, String)> {
        match parse_json_line(line) {
            LogEvent::Change { kind, path, .. } => Some((kind, path)),
            _ => None,
        }
    }

    #[test]
    fn dry_run_and_real_changes() {
        let cases = [
            (
                r#"{"level":"notice","msg":"Skipped copy as --dry-run is set (size 2)","skipped":"copy","size":2,"object":"sub/b.css","objectType":"*local.Object"}"#,
                Some((ChangeKind::Upload, "sub/b.css")),
            ),
            (
                r#"{"level":"notice","msg":"Skipped delete as --dry-run is set (size 4)","object":"alt.html"}"#,
                Some((ChangeKind::Delete, "alt.html")),
            ),
            (
                r#"{"level":"info","msg":"Copied (new)","size":2,"object":"a.html"}"#,
                Some((ChangeKind::Upload, "a.html")),
            ),
            (
                r#"{"level":"info","msg":"Deleted","object":"alt.html"}"#,
                Some((ChangeKind::Delete, "alt.html")),
            ),
            (
                r#"{"level":"notice","msg":"Skipped set directory modification time as --dry-run is set","object":"sub","objectType":"string"}"#,
                None,
            ),
        ];
        for (line, expected) in cases {
            assert_eq!(
                change(line),
                expected.map(|(kind, path)| (kind, path.to_owned())),
                "{line}"
            );
        }
    }

    #[test]
    fn statistics_and_text() {
        let line = r#"{"level":"notice","msg":"\nTransferred: 1 KiB / 2 KiB, 50%\n\n","stats":{"bytes":1024,"totalBytes":2048,"transfers":1,"totalTransfers":2,"eta":3,"speed":512.5,"errors":0,"elapsedTime":1.2}}"#;
        let LogEvent::Stats { stats, text } = parse_json_line(line) else {
            panic!("statistics expected");
        };
        assert_eq!((stats.bytes, stats.total_bytes), (1024, 2048));
        assert_eq!((stats.transfers, stats.total_transfers), (1, 2));
        assert_eq!(stats.eta, Some(3.0));
        assert_eq!(text, "Transferred: 1 KiB / 2 KiB, 50%");

        let no_eta = r#"{"level":"notice","msg":"","stats":{"bytes":0,"eta":null}}"#;
        assert!(matches!(
            parse_json_line(no_eta),
            LogEvent::Stats {
                stats: TransferStats { eta: None, .. },
                ..
            }
        ));
        assert_eq!(
            parse_json_line(r#"{"level":"error","msg":"connection refused"}"#),
            LogEvent::Text("ERROR: connection refused".into())
        );
        assert_eq!(
            parse_json_line("panic: boom"),
            LogEvent::Text("panic: boom".into())
        );
    }
}
