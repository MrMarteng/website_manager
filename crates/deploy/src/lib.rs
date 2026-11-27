//! `teleschirm-deploy`: uploads a website according to a `deploy.conf`.
//!
//! The format is the one documented in `docs/formats/deploy-conf.md`: a `[server]` block
//! followed by mappings `<local> -> <remote> | <options>`. Options are `mode=seed|copy`
//! and `protect=<glob>,<glob>`. The actual transfer is done by rclone.

pub mod changes;
pub mod config;
pub mod error;
pub mod plan;
pub mod runner;

pub use changes::ChangeKind;
pub use config::{DeployConfig, Mapping, MappingMode, Protocol, ServerConfig};
pub use error::DeployError;
pub use plan::{Connection, Credential, Step, build_plan};
pub use runner::{RunSummary, Runner, VersionStamp};

/// File written to the site root after every successful upload.
pub const VERSION_FILE: &str = ".website-manager.json";
