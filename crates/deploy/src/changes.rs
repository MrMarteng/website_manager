//! Reads file changes from rclone's `--verbose` output (real runs and dry runs).

/// Kind of change to a file on the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Upload,
    Delete,
}

/// Recognises a file change in one rclone log line; the path is relative to the mapping.
#[must_use]
pub fn parse_line(line: &str) -> Option<(ChangeKind, String)> {
    let rest = ["NOTICE: ", "INFO  : "]
        .iter()
        .find_map(|marker| line.split_once(marker).map(|(_, rest)| rest))?;
    let patterns = [
        (": Skipped copy as --dry-run", ChangeKind::Upload),
        (": Skipped delete as --dry-run", ChangeKind::Delete),
        (": Copied (", ChangeKind::Upload),
        (": Updated modification time", ChangeKind::Upload),
    ];
    for (pattern, kind) in patterns {
        if let Some((path, _)) = rest.split_once(pattern) {
            return Some((kind, path.to_owned()));
        }
    }
    rest.strip_suffix(": Deleted")
        .map(|path| (ChangeKind::Delete, path.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_and_real_lines() {
        let cases = [
            (
                "2026/11/27 08:09:14 NOTICE: sub/b.css: Skipped copy as --dry-run is set (size 2)",
                Some((ChangeKind::Upload, "sub/b.css")),
            ),
            (
                "2026/11/27 08:09:14 NOTICE: alt.html: Skipped delete as --dry-run is set (size 4)",
                Some((ChangeKind::Delete, "alt.html")),
            ),
            (
                "2026/11/27 08:09:22 INFO  : a.html: Copied (new)",
                Some((ChangeKind::Upload, "a.html")),
            ),
            (
                "2026/11/27 08:09:22 INFO  : alt.html: Deleted",
                Some((ChangeKind::Delete, "alt.html")),
            ),
            (
                "2026/11/27 08:09:14 NOTICE: sub: Skipped set directory modification time as --dry-run is set",
                None,
            ),
            ("Transferred:   4 B / 4 B, 100%", None),
        ];
        for (line, expected) in cases {
            let parsed = parse_line(line);
            assert_eq!(
                parsed.as_ref().map(|(kind, path)| (*kind, path.as_str())),
                expected,
                "{line}"
            );
        }
    }
}
