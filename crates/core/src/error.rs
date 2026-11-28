//! The error type shared by all functions of `wm-core`.

use std::path::PathBuf;

/// Errors of the package, signature and runtime functions.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// Reading or writing a file or folder failed.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// File or folder concerned.
        path: PathBuf,
        /// Underlying operating system error.
        #[source]
        source: std::io::Error,
    },
    /// A JSON document (manifest, trust store, project record …) could not be read.
    #[error("invalid JSON in {what}: {source}")]
    Json {
        /// Description of the parsed content, usually the file name.
        what: String,
        /// Underlying parser error.
        #[source]
        source: serde_json::Error,
    },
    /// The package is not a readable ZIP archive.
    #[error("archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// A signing or public key has the wrong format or length.
    #[error("invalid key: {0}")]
    InvalidKey(String),
    /// The signature does not match the manifest.
    #[error("signature check failed: the package was changed or not signed by this key")]
    BadSignature,
    /// An archive entry has an absolute path, `..`, or is a link; refused against zip slip.
    #[error("package entry {0:?} leaves the target folder or is not a plain file")]
    UnsafeEntry(String),
    /// The archive contains a file the signed manifest does not list.
    #[error("package entry {0:?} is not listed in the manifest")]
    UnlistedEntry(String),
    /// The manifest lists a file the archive does not contain.
    #[error("file {0:?} listed in the manifest is missing from the package")]
    MissingEntry(String),
    /// A file's SHA-256 differs from the manifest (or a download from its published checksum).
    #[error("checksum mismatch for {0:?}")]
    ChecksumMismatch(String),
    /// The package was made by a newer, incompatible version of the tool.
    #[error("unsupported package format version {0}")]
    UnsupportedFormat(u32),
    /// The package comes from another publisher than the one the project is bound to.
    #[error(
        "package {project:?} is signed by {found}, but this project is bound to publisher {expected}"
    )]
    PublisherMismatch {
        /// Project name from the manifest.
        project: String,
        /// Fingerprint of the publisher recorded on first use.
        expected: String,
        /// Fingerprint of the key that signed this package.
        found: String,
    },
    /// Secret detection found likely secrets in files to be packed (paths and reasons).
    #[error("possible secrets found, packing aborted: {0:?}")]
    SecretsFound(Vec<String>),
    /// A glob pattern of the package configuration is invalid.
    #[error("invalid pattern {pattern:?}: {message}")]
    Pattern {
        /// The pattern as written.
        pattern: String,
        /// Parser message.
        message: String,
    },
    /// A download (Node.js, release index) failed.
    #[error("download of {url} failed: {message}")]
    Download {
        /// Address that was requested.
        url: String,
        /// Network or HTTP error.
        message: String,
    },
    /// No Node.js build exists for this operating system or processor.
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),
    /// The credentials file or its content is invalid.
    #[error("credentials: {0}")]
    Credentials(String),
    /// Decrypting the credentials failed: wrong password or modified file.
    #[error("wrong package password, or the credentials were changed")]
    WrongPassword,
    /// The operating system's random number generator failed.
    #[error("random number generator failed: {0}")]
    Random(String),
}

impl CoreError {
    /// Wraps an I/O error together with the affected path.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Wraps a JSON error together with a description of the parsed content.
    pub fn json(what: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Json {
            what: what.into(),
            source,
        }
    }
}
