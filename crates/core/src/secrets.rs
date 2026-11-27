use std::fs;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::CoreError;

/// File names that must never end up in a package.
const FORBIDDEN_NAMES: &[&str] = &[
    "**/.env",
    "**/.env.*",
    "**/*.pem",
    "**/*.key",
    "**/*.p12",
    "**/*.pfx",
    "**/id_rsa*",
    "**/id_ed25519*",
    "**/id_ecdsa*",
    "**/.npmrc",
    "**/auth.json",
    "**/.htpasswd",
];

/// Files that are explicitly allowed although they match a forbidden name.
const ALLOWED_NAMES: &[&str] = &["**/.env.example"];

/// Text markers that indicate private keys or tokens inside a file.
const FORBIDDEN_CONTENT: &[&str] = &[
    "-----BEGIN RSA PRIVATE KEY-----",
    "-----BEGIN OPENSSH PRIVATE KEY-----",
    "-----BEGIN PRIVATE KEY-----",
    "-----BEGIN EC PRIVATE KEY-----",
    "ghp_",
    "glpat-",
    "xoxb-",
];

/// Files larger than this are not scanned for content markers.
const MAX_SCANNED_BYTES: u64 = 2 * 1024 * 1024;

/// Detects files that look like secrets, by name and by content.
pub struct SecretScanner {
    forbidden: GlobSet,
    allowed: GlobSet,
}

impl SecretScanner {
    /// Creates a scanner with the built-in rules.
    ///
    /// # Errors
    /// Fails only if a built-in pattern is invalid.
    pub fn new() -> Result<Self, CoreError> {
        Ok(Self {
            forbidden: build_set(FORBIDDEN_NAMES)?,
            allowed: build_set(ALLOWED_NAMES)?,
        })
    }

    /// Returns a reason if the file at `path` (stored as `package_path`) looks like a secret.
    #[must_use]
    pub fn check(&self, package_path: &str, path: &Path) -> Option<String> {
        if self.allowed.is_match(package_path) {
            return None;
        }
        if self.forbidden.is_match(package_path) {
            return Some(format!("{package_path}: forbidden file name"));
        }
        let size = fs::metadata(path).map_or(0, |m| m.len());
        if size > MAX_SCANNED_BYTES {
            return None;
        }
        let content = fs::read(path).ok()?;
        let text = String::from_utf8_lossy(&content);
        FORBIDDEN_CONTENT
            .iter()
            .find(|marker| text.contains(*marker))
            .map(|marker| format!("{package_path}: contains {marker:?}"))
    }
}

fn build_set(patterns: &[&str]) -> Result<GlobSet, CoreError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| CoreError::Pattern {
            pattern: (*pattern).to_owned(),
            message: e.to_string(),
        })?;
        builder.add(glob);
    }
    builder.build().map_err(|e| CoreError::Pattern {
        pattern: patterns.join(", "),
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_names_and_content() {
        let dir = tempfile::tempdir().unwrap();
        let env = dir.path().join(".env");
        let example = dir.path().join(".env.example");
        let key = dir.path().join("notes.txt");
        let plain = dir.path().join("index.html");
        fs::write(&env, "X=1").unwrap();
        fs::write(&example, "X=").unwrap();
        fs::write(&key, "-----BEGIN OPENSSH PRIVATE KEY-----").unwrap();
        fs::write(&plain, "<h1>Hallo</h1>").unwrap();

        let scanner = SecretScanner::new().unwrap();
        assert!(scanner.check("project/.env", &env).is_some());
        assert!(scanner.check("project/.env.example", &example).is_none());
        assert!(scanner.check("project/notes.txt", &key).is_some());
        assert!(scanner.check("dist/index.html", &plain).is_none());
    }
}
