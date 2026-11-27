use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{CoreError, FORMAT_VERSION};

/// Describes a package and lists the checksum of every payload file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Version of the package format.
    pub format_version: u32,
    /// Project name; also the key for publisher pinning.
    pub name: String,
    /// Version of this delivery, e.g. `2026.11.27` or `1.2.0`.
    pub version: String,
    /// Creation time as RFC 3339 string.
    pub created: String,
    /// Node.js version needed to build or run the development server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_version: Option<String>,
    /// Human-readable list of changes in this delivery.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changelog: Vec<String>,
    /// Path of a logo inside the package shown instead of the default logo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    /// Whether the package contains encrypted hoster credentials.
    #[serde(default)]
    pub has_credentials: bool,
    /// SHA-256 checksum per payload path (forward slashes, relative to the package root).
    pub files: BTreeMap<String, String>,
}

impl Manifest {
    /// Serialises the manifest in a stable form; these exact bytes are signed.
    ///
    /// # Errors
    /// Fails only if serialisation fails.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec_pretty(self).map_err(|e| CoreError::json("manifest", e))
    }

    /// Parses a manifest and checks the format version.
    ///
    /// # Errors
    /// Fails for invalid JSON or an unsupported format version.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CoreError> {
        let manifest: Self =
            serde_json::from_slice(bytes).map_err(|e| CoreError::json("manifest", e))?;
        if manifest.format_version != FORMAT_VERSION {
            return Err(CoreError::UnsupportedFormat(manifest.format_version));
        }
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Manifest {
        Manifest {
            format_version: FORMAT_VERSION,
            name: "beispiel".into(),
            version: "1.0.0".into(),
            created: "2026-11-27T08:00:00Z".into(),
            node_version: Some("22.12.0".into()),
            changelog: vec!["Erste Auslieferung".into()],
            logo: None,
            has_credentials: false,
            files: BTreeMap::from([("dist/index.html".into(), "ab".into())]),
        }
    }

    #[test]
    fn round_trip() {
        let bytes = sample().to_canonical_bytes().unwrap();
        assert_eq!(Manifest::from_bytes(&bytes).unwrap(), sample());
    }

    #[test]
    fn rejects_unknown_format_version() {
        let mut manifest = sample();
        manifest.format_version = 99;
        let bytes = manifest.to_canonical_bytes().unwrap();
        assert!(matches!(
            Manifest::from_bytes(&bytes),
            Err(CoreError::UnsupportedFormat(99))
        ));
    }
}
