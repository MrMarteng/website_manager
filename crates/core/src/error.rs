use std::path::PathBuf;

/// Errors of the package, signature and runtime functions.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON in {what}: {source}")]
    Json {
        what: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("invalid key: {0}")]
    InvalidKey(String),
    #[error("signature check failed: the package was changed or not signed by this key")]
    BadSignature,
    #[error("package entry {0:?} leaves the target folder or is not a plain file")]
    UnsafeEntry(String),
    #[error("package entry {0:?} is not listed in the manifest")]
    UnlistedEntry(String),
    #[error("file {0:?} listed in the manifest is missing from the package")]
    MissingEntry(String),
    #[error("checksum mismatch for {0:?}")]
    ChecksumMismatch(String),
    #[error("unsupported package format version {0}")]
    UnsupportedFormat(u32),
    #[error(
        "package {project:?} is signed by {found}, but this project is bound to publisher {expected}"
    )]
    PublisherMismatch {
        project: String,
        expected: String,
        found: String,
    },
    #[error("possible secrets found, packing aborted: {0:?}")]
    SecretsFound(Vec<String>),
    #[error("invalid pattern {pattern:?}: {message}")]
    Pattern { pattern: String, message: String },
    #[error("download of {url} failed: {message}")]
    Download { url: String, message: String },
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),
    #[error("credentials: {0}")]
    Credentials(String),
    #[error("wrong package password, or the credentials were changed")]
    WrongPassword,
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
