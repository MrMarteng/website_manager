//! Core of the Website Manager: the `.tsweb` package format, Ed25519 signatures with per-project
//! publisher pinning, SHA-256 checksums, secret detection and managed runtimes.

pub mod archive;
pub mod credentials;
pub mod error;
pub mod hashing;
pub mod manifest;
pub mod projects;
pub mod runtime;
pub mod secrets;
pub mod signing;
pub mod time;
pub mod trust;

pub use error::CoreError;

/// File name of the manifest inside a package.
pub const MANIFEST_FILE: &str = "manifest.json";
/// File name of the detached manifest signature inside a package.
pub const SIGNATURE_FILE: &str = "manifest.sig";
/// Current version of the package format.
pub const FORMAT_VERSION: u32 = 1;
