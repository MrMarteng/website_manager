//! Locating the helper programs PHP and rclone.
//!
//! Order: the environment variables `WM_PHP` / `WM_RCLONE` (override for developers), then the
//! programs shipped with the app (Tauri sidecars `wm-php` / `wm-rclone` next to the
//! executable), then `php` / `rclone` on the `PATH`. The sidecars carry a `wm-` prefix so the
//! Linux packages do not collide with `/usr/bin/php` or `/usr/bin/rclone`.

use std::fs;
use std::path::{Path, PathBuf};

/// Where a helper program was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Environment,
    Bundled,
    Path,
}

impl Source {
    /// Name used in the log.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::Bundled => "bundled",
            Self::Path => "PATH",
        }
    }
}

/// A helper program and where it was found.
pub struct Tool {
    pub program: PathBuf,
    pub source: Source,
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

fn locate(variable: &str, sidecar: &str, command: &str) -> Tool {
    if let Some(path) = std::env::var_os(variable) {
        return Tool {
            program: PathBuf::from(path),
            source: Source::Environment,
        };
    }
    if let Some(next_to_exe) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(executable_name(sidecar))))
        .filter(|path| path.is_file())
    {
        return Tool {
            program: next_to_exe,
            source: Source::Bundled,
        };
    }
    Tool {
        program: PathBuf::from(command),
        source: Source::Path,
    }
}

pub fn php() -> Tool {
    locate("WM_PHP", "wm-php", "php")
}

pub fn rclone() -> Tool {
    locate("WM_RCLONE", "wm-rclone", "rclone")
}

/// A folder to put first on `PATH` so that child processes find `php` (the CMS development
/// server starts `php -S` itself). For the bundled `wm-php` a copy named `php` is kept in
/// `cache_dir`; `None` if PHP comes from the `PATH` anyway.
///
/// # Errors
/// Fails if the copy cannot be created.
pub fn php_path_dir(cache_dir: &Path) -> Result<Option<PathBuf>, String> {
    let php = php();
    match php.source {
        Source::Path => Ok(None),
        Source::Environment => Ok(php.program.parent().map(Path::to_path_buf)),
        Source::Bundled => {
            let dir = cache_dir.join("bin");
            let target = dir.join(executable_name("php"));
            let source_len = fs::metadata(&php.program).map_err(|e| e.to_string())?.len();
            let up_to_date = fs::metadata(&target).is_ok_and(|meta| meta.len() == source_len);
            if !up_to_date {
                fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let _ = fs::remove_file(&target);
                // A hard link costs no space; across volumes fall back to a copy.
                if fs::hard_link(&php.program, &target).is_err() {
                    fs::copy(&php.program, &target).map_err(|e| e.to_string())?;
                }
            }
            Ok(Some(dir))
        }
    }
}
