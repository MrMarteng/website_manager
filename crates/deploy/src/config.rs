//! Parser for `deploy.conf`, compatible with the original `deploy.sh`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::DeployError;

/// Transfer protocol of the webspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Protocol {
    /// SSH file transfer (default).
    #[default]
    Sftp,
    /// FTP with explicit TLS; plain FTP is not supported.
    Ftps,
}

/// The `[server]` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    /// Transfer protocol.
    pub protocol: Protocol,
    /// Server name.
    pub host: String,
    /// Login name.
    pub user: String,
    /// Port; 22 for SFTP and 21 for FTPS unless set.
    pub port: u16,
    /// SSH key; relative paths are resolved against the folder of the `deploy.conf`.
    pub key_file: Option<PathBuf>,
    /// Recorded host keys of the server (SFTP), checked by rclone.
    pub known_hosts_file: Option<PathBuf>,
}

/// How a mapping is transferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingMode {
    /// Make the target identical to the source (deletes extra files, except protected ones).
    Sync,
    /// Copy new and changed files, never delete.
    Copy,
    /// Copy only if the target has no files yet (initial content such as CMS data).
    Seed,
}

/// One line `<local> -> <remote> | <options>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    /// Local folder relative to the base folder.
    pub source: String,
    /// Remote folder.
    pub target: String,
    /// `None` uses the mode chosen for the whole run (sync or copy).
    pub mode: Option<MappingMode>,
    /// Globs relative to the mapping that are neither uploaded nor deleted on the server.
    pub protect: Vec<String>,
    /// Line number in the file, for messages.
    pub line: usize,
}

/// A parsed `deploy.conf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployConfig {
    /// The `[server]` block.
    pub server: ServerConfig,
    /// Folder the mapping sources are relative to.
    pub base_dir: PathBuf,
    /// Mappings in file order.
    pub mappings: Vec<Mapping>,
}

impl DeployConfig {
    /// Reads and parses a `deploy.conf`.
    ///
    /// # Errors
    /// Fails if the file cannot be read or is invalid.
    pub fn load(path: &Path) -> Result<Self, DeployError> {
        let text = fs::read_to_string(path).map_err(|source| DeployError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let config_dir = path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        Self::parse(&text, &config_dir)
    }

    /// Parses `deploy.conf` content; `config_dir` anchors relative paths.
    ///
    /// # Errors
    /// Fails for syntax errors, unknown options or missing `host`/`user`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use teleschirm_deploy::{DeployConfig, MappingMode, Protocol};
    ///
    /// let text = "[server]
    /// host = example.com
    /// user = web123
    ///
    /// dist/backend/data -> /site/backend/data | mode=seed
    /// dist -> /site | protect=backend/data/*.json";
    /// let config = DeployConfig::parse(text, Path::new("/projects/cafe"))?;
    ///
    /// assert_eq!(config.server.protocol, Protocol::Sftp);
    /// assert_eq!(config.server.port, 22);
    /// assert_eq!(config.mappings[0].mode, Some(MappingMode::Seed));
    /// assert_eq!(config.mappings[1].protect, ["backend/data/*.json"]);
    /// # Ok::<(), teleschirm_deploy::DeployError>(())
    /// ```
    pub fn parse(text: &str, config_dir: &Path) -> Result<Self, DeployError> {
        let mut server = RawServer::default();
        let mut mappings = Vec::new();
        let mut in_server_block = true;

        for (index, raw_line) in text.lines().enumerate() {
            let line_number = index + 1;
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line == "[server]" {
                continue;
            }
            if raw_line.contains(" -> ") {
                in_server_block = false;
                if let Some(mapping) = parse_mapping(raw_line, line_number)? {
                    mappings.push(mapping);
                }
                continue;
            }
            if in_server_block && let Some((key, value)) = line.split_once('=') {
                server.set(key.trim(), value.trim());
            }
        }

        let host = server
            .host
            .ok_or_else(|| DeployError::Incomplete("host is missing".into()))?;
        let user = server
            .user
            .ok_or_else(|| DeployError::Incomplete("user is missing".into()))?;
        let protocol = match server.protocol.as_deref() {
            None | Some("sftp") => Protocol::Sftp,
            Some("ftps") => Protocol::Ftps,
            Some(other) => {
                return Err(DeployError::Incomplete(format!(
                    "unknown protocol {other:?} (allowed: sftp, ftps)"
                )));
            }
        };
        let default_port = if protocol == Protocol::Sftp { 22 } else { 21 };
        let port = match server.port.as_deref() {
            None | Some("") => default_port,
            Some(text) => text
                .parse()
                .map_err(|_| DeployError::Incomplete(format!("invalid port {text:?}")))?,
        };
        let base_dir = server
            .base_dir
            .map_or_else(|| config_dir.to_path_buf(), |dir| resolve(config_dir, &dir));

        Ok(Self {
            server: ServerConfig {
                protocol,
                host,
                user,
                port,
                key_file: server.key_file.map(|file| resolve(config_dir, &file)),
                known_hosts_file: server
                    .known_hosts_file
                    .map(|file| resolve(config_dir, &file)),
            },
            base_dir,
            mappings,
        })
    }
}

impl DeployConfig {
    /// Replaces the website root folder on the server (the shortest target) in all mappings,
    /// e.g. `/site` → `/httpdocs` turns `/site/backend/data` into `/httpdocs/backend/data`.
    #[must_use]
    pub fn with_remote_root(mut self, new_root: &str) -> Self {
        let Some(old_root) = self
            .mappings
            .iter()
            .map(|mapping| mapping.target.clone())
            .min_by_key(String::len)
        else {
            return self;
        };
        let old_root = old_root.trim_end_matches('/').to_owned();
        let new_root = format!("/{}", new_root.trim_matches('/'));
        let new_root = if new_root == "/" {
            String::new()
        } else {
            new_root
        };
        for mapping in &mut self.mappings {
            if let Some(rest) = mapping.target.strip_prefix(&old_root)
                && (rest.is_empty() || rest.starts_with('/'))
            {
                let joined = format!("{new_root}{rest}");
                mapping.target = if joined.is_empty() {
                    "/".into()
                } else {
                    joined
                };
            }
        }
        self
    }
}

#[derive(Default)]
struct RawServer {
    protocol: Option<String>,
    host: Option<String>,
    user: Option<String>,
    port: Option<String>,
    key_file: Option<String>,
    known_hosts_file: Option<String>,
    base_dir: Option<String>,
}

impl RawServer {
    fn set(&mut self, key: &str, value: &str) {
        let value = Some(value.to_owned());
        match key {
            "protocol" => self.protocol = value,
            "host" => self.host = value,
            "user" => self.user = value,
            "port" => self.port = value,
            "key_file" => self.key_file = value,
            "known_hosts_file" => self.known_hosts_file = value,
            "base_dir" => self.base_dir = value,
            // Unknown keys are ignored, as in deploy.sh.
            _ => {}
        }
    }
}

fn parse_mapping(raw_line: &str, line: usize) -> Result<Option<Mapping>, DeployError> {
    let Some((source, rest)) = raw_line.split_once(" ->") else {
        return Ok(None);
    };
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    let (target, options) = match rest.split_once(" | ") {
        Some((target, options)) => (target, options),
        None => (rest, ""),
    };
    let source = source.trim();
    let target = target.trim();
    // Empty sides and `key=value` sources are skipped, as in deploy.sh.
    if source.is_empty() || target.is_empty() || source.contains('=') {
        return Ok(None);
    }

    let mut mode = None;
    let mut protect = Vec::new();
    for option in options.split(';') {
        let (key, value) = option.split_once('=').unwrap_or((option, ""));
        match key.trim() {
            "" => {}
            "mode" => {
                mode = Some(match value.trim() {
                    "seed" => MappingMode::Seed,
                    "copy" => MappingMode::Copy,
                    other => {
                        return Err(DeployError::Syntax {
                            line,
                            message: format!("unknown mode {other:?} (allowed: seed, copy)"),
                        });
                    }
                });
            }
            "protect" => protect.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|pattern| !pattern.is_empty())
                    .map(str::to_owned),
            ),
            other => {
                return Err(DeployError::Syntax {
                    line,
                    message: format!("unknown option {other:?} (allowed: protect, mode)"),
                });
            }
        }
    }

    Ok(Some(Mapping {
        source: source.to_owned(),
        target: target.to_owned(),
        mode,
        protect,
        line,
    }))
}

/// Resolves `~/` and relative paths against `config_dir`.
fn resolve(config_dir: &Path, value: &str) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return home.join(rest);
    }
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        config_dir.join(path)
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# deploy.conf
[server]
host = example.com
user = web1
port = 2222
key_file = keys/id_ed25519
base_dir = ./

dist/backend/data -> /site/backend/data | mode=seed
dist -> /site | protect=backend/data/*.json, backend/uploads/*/**
";

    #[test]
    fn parses_server_and_mappings() {
        let config = DeployConfig::parse(SAMPLE, Path::new("/project")).unwrap();
        assert_eq!(config.server.host, "example.com");
        assert_eq!(config.server.port, 2222);
        assert_eq!(config.server.protocol, Protocol::Sftp);
        assert_eq!(
            config.server.key_file.as_deref(),
            Some(Path::new("/project/keys/id_ed25519"))
        );
        assert_eq!(config.base_dir, Path::new("/project/./"));
        assert_eq!(config.mappings.len(), 2);
        assert_eq!(config.mappings[0].mode, Some(MappingMode::Seed));
        assert_eq!(config.mappings[1].mode, None);
        assert_eq!(
            config.mappings[1].protect,
            vec!["backend/data/*.json", "backend/uploads/*/**"]
        );
        assert_eq!(config.mappings[1].line, 10);
    }

    #[test]
    fn combined_options() {
        let text = "host = h\nuser = u\ndist -> /x | mode=copy; protect=a/**\n";
        let config = DeployConfig::parse(text, Path::new("/p")).unwrap();
        assert_eq!(config.mappings[0].mode, Some(MappingMode::Copy));
        assert_eq!(config.mappings[0].protect, vec!["a/**"]);
    }

    #[test]
    fn defaults_and_ftps() {
        let text = "protocol = ftps\nhost = h\nuser = u\ndist -> /\n";
        let config = DeployConfig::parse(text, Path::new("/p")).unwrap();
        assert_eq!(config.server.protocol, Protocol::Ftps);
        assert_eq!(config.server.port, 21);
        assert!(config.server.key_file.is_none());
    }

    #[test]
    fn rejects_unknown_option_and_missing_host() {
        let unknown = "host = h\nuser = u\ndist -> /x | delete=all\n";
        assert!(matches!(
            DeployConfig::parse(unknown, Path::new("/p")),
            Err(DeployError::Syntax { line: 3, .. })
        ));
        assert!(matches!(
            DeployConfig::parse("user = u\n", Path::new("/p")),
            Err(DeployError::Incomplete(_))
        ));
    }

    #[test]
    fn remote_root_is_replaced_in_all_mappings() {
        let config = DeployConfig::parse(SAMPLE, Path::new("/p"))
            .unwrap()
            .with_remote_root("httpdocs/");
        let targets: Vec<&str> = config.mappings.iter().map(|m| m.target.as_str()).collect();
        assert_eq!(targets, vec!["/httpdocs/backend/data", "/httpdocs"]);
        let at_top = DeployConfig::parse(SAMPLE, Path::new("/p"))
            .unwrap()
            .with_remote_root("/");
        let targets: Vec<&str> = at_top.mappings.iter().map(|m| m.target.as_str()).collect();
        assert_eq!(targets, vec!["/backend/data", "/"]);
    }

    #[test]
    fn server_keys_after_mappings_are_ignored() {
        let text = "host = h\nuser = u\ndist -> /x\nhost = other\n";
        let config = DeployConfig::parse(text, Path::new("/p")).unwrap();
        assert_eq!(config.server.host, "h");
    }
}
