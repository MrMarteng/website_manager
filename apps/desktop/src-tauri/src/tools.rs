//! Locating the bundled helper programs (PHP, rclone).
//!
//! Release builds ship them as Tauri sidecars next to the executable. Until the sidecar
//! binaries are bundled (phase 0 open point), the environment variables `WM_PHP` and
//! `WM_RCLONE` or the programs on `PATH` are used.

use std::path::PathBuf;

/// A helper program and where it was found.
pub struct Tool {
    pub program: PathBuf,
    pub source: &'static str,
}

fn locate(variable: &str, sidecar: &str) -> Tool {
    if let Some(path) = std::env::var_os(variable) {
        return Tool {
            program: PathBuf::from(path),
            source: "environment",
        };
    }
    let file_name = if cfg!(windows) {
        format!("{sidecar}.exe")
    } else {
        sidecar.to_owned()
    };
    if let Some(next_to_exe) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(&file_name)))
        .filter(|path| path.is_file())
    {
        return Tool {
            program: next_to_exe,
            source: "bundled",
        };
    }
    Tool {
        program: PathBuf::from(sidecar),
        source: "PATH",
    }
}

pub fn php() -> Tool {
    locate("WM_PHP", "php")
}

pub fn rclone() -> Tool {
    locate("WM_RCLONE", "rclone")
}
