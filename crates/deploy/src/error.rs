use std::path::PathBuf;

/// Errors while reading a `deploy.conf`, planning or running an upload.
#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("deploy.conf line {line}: {message}")]
    Syntax { line: usize, message: String },
    #[error("deploy.conf is incomplete: {0}")]
    Incomplete(String),
    #[error("local folder of mapping {source_dir:?} is missing: {path}")]
    MissingSource { source_dir: String, path: PathBuf },
    #[error("rclone could not be started ({program}): {source}")]
    RcloneStart {
        program: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("rclone failed: {0}")]
    RcloneFailed(String),
    #[error("{failed} of {total} mapping(s) failed")]
    MappingsFailed { failed: usize, total: usize },
    #[error("version file on the server is unreadable: {0}")]
    VersionFile(String),
}
