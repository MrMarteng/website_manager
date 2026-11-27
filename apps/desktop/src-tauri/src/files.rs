//! Files of a version for the file list; opening happens in the system's own programs.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use wm_core::archive::safe_relative_path;

/// Most files listed.
const MAX_FILES: usize = 5000;
/// Top-level folders shown in the file list.
const LISTED_ROOTS: &[&str] = &["dist", "project", "compliance"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    /// Path relative to the version folder, with `/`.
    pub path: String,
    pub size: u64,
}

/// Lists the files below `dist/`, `project/` and `compliance/`, sorted.
#[must_use]
pub fn list(folder: &Path) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    for root in LISTED_ROOTS {
        collect(folder, &folder.join(root), &mut entries);
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.truncate(MAX_FILES);
    entries
}

fn collect(base: &Path, dir: &Path, entries: &mut Vec<FileEntry>) {
    let Ok(children) = fs::read_dir(dir) else {
        return;
    };
    for child in children.filter_map(Result::ok) {
        if entries.len() >= MAX_FILES {
            return;
        }
        let Ok(kind) = child.file_type() else {
            continue;
        };
        let path = child.path();
        if kind.is_dir() {
            collect(base, &path, entries);
        } else if kind.is_file()
            && let Ok(relative) = path.strip_prefix(base)
        {
            entries.push(FileEntry {
                path: relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/"),
                size: child.metadata().map_or(0, |meta| meta.len()),
            });
        }
    }
}

/// Resolves a path from the window to an existing file inside `folder`.
///
/// # Errors
/// Fails for paths that leave the folder or do not exist.
pub fn resolve(folder: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = safe_relative_path(relative).ok_or("invalid path")?;
    let path = folder.join(relative);
    if path.is_file() {
        Ok(path)
    } else {
        Err("file not found".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_only_package_folders_and_rejects_escapes() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("dist/css")).unwrap();
        fs::create_dir_all(dir.path().join(".runtime")).unwrap();
        fs::write(dir.path().join("dist/index.html"), "x").unwrap();
        fs::write(dir.path().join("dist/css/a.css"), "yy").unwrap();
        fs::write(dir.path().join(".runtime/router.php"), "z").unwrap();
        let paths: Vec<String> = list(dir.path()).into_iter().map(|e| e.path).collect();
        assert_eq!(paths, vec!["dist/css/a.css", "dist/index.html"]);
        assert!(resolve(dir.path(), "dist/index.html").is_ok());
        assert!(resolve(dir.path(), "../etc/passwd").is_err());
        assert!(resolve(dir.path(), "dist/missing.html").is_err());
    }
}
