//! Managed runtimes (Node.js) that the Website Manager downloads and caches per version, and
//! npm lockfile helpers.

pub mod node;
pub mod npm;

/// Operating system and CPU architecture in the naming of the Node.js downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    /// `darwin`, `linux` or `win`.
    pub os: &'static str,
    /// `arm64` or `x64`.
    pub arch: &'static str,
}

impl Platform {
    /// Platform this program runs on.
    ///
    /// # Errors
    /// Fails on operating systems or architectures without official Node.js builds.
    pub fn current() -> Result<Self, crate::CoreError> {
        let os = match std::env::consts::OS {
            "macos" => "darwin",
            "linux" => "linux",
            "windows" => "win",
            other => return Err(crate::CoreError::UnsupportedPlatform(other.to_owned())),
        };
        let arch = match std::env::consts::ARCH {
            "aarch64" => "arm64",
            "x86_64" => "x64",
            other => return Err(crate::CoreError::UnsupportedPlatform(other.to_owned())),
        };
        Ok(Self { os, arch })
    }

    /// Whether this is Windows (zip archives, `node.exe`).
    #[must_use]
    pub fn is_windows(self) -> bool {
        self.os == "win"
    }
}
