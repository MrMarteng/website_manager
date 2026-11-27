use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

use crate::CoreError;
use crate::signing::{decode_public_key, encode_public_key, fingerprint};

/// Remembers which publisher key belongs to which project (trust on first use).
///
/// The first package of a project binds its publisher key; later packages of the same
/// project with another key are rejected.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TrustStore {
    #[serde(skip)]
    path: PathBuf,
    projects: BTreeMap<String, String>,
}

/// Result of checking a package's publisher against the trust store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustDecision {
    /// The project is already bound to this key.
    Known,
    /// The project is new; the key will be bound when the package is accepted.
    FirstUse,
}

impl TrustStore {
    /// Loads the trust store from `path`, or returns an empty one if the file does not exist.
    ///
    /// # Errors
    /// Fails if the file exists but cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, CoreError> {
        let mut store = if path.exists() {
            let text = fs::read_to_string(path).map_err(|e| CoreError::io(path, e))?;
            serde_json::from_str(&text).map_err(|e| CoreError::json("trust store", e))?
        } else {
            Self::default()
        };
        store.path = path.to_path_buf();
        Ok(store)
    }

    /// Checks `key` for `project` without changing the store.
    ///
    /// # Errors
    /// Returns [`CoreError::PublisherMismatch`] if the project is bound to another key.
    pub fn check(&self, project: &str, key: &VerifyingKey) -> Result<TrustDecision, CoreError> {
        match self.projects.get(project) {
            None => Ok(TrustDecision::FirstUse),
            Some(stored) => {
                let expected = decode_public_key(stored)?;
                if &expected == key {
                    Ok(TrustDecision::Known)
                } else {
                    Err(CoreError::PublisherMismatch {
                        project: project.to_owned(),
                        expected: fingerprint(&expected),
                        found: fingerprint(key),
                    })
                }
            }
        }
    }

    /// Binds `project` to `key` and saves the store.
    ///
    /// # Errors
    /// Fails if the project is bound to another key or the store cannot be written.
    pub fn bind(&mut self, project: &str, key: &VerifyingKey) -> Result<(), CoreError> {
        self.check(project, key)?;
        self.projects
            .insert(project.to_owned(), encode_public_key(key));
        self.save()
    }

    fn save(&self) -> Result<(), CoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        let text =
            serde_json::to_string_pretty(self).map_err(|e| CoreError::json("trust store", e))?;
        fs::write(&self.path, text).map_err(|e| CoreError::io(&self.path, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::generate_signing_key;

    #[test]
    fn first_use_then_known_then_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trust.json");
        let publisher = generate_signing_key().unwrap().verifying_key();
        let attacker = generate_signing_key().unwrap().verifying_key();

        let mut store = TrustStore::load(&path).unwrap();
        assert_eq!(
            store.check("cafe", &publisher).unwrap(),
            TrustDecision::FirstUse
        );
        store.bind("cafe", &publisher).unwrap();

        let reloaded = TrustStore::load(&path).unwrap();
        assert_eq!(
            reloaded.check("cafe", &publisher).unwrap(),
            TrustDecision::Known
        );
        assert!(matches!(
            reloaded.check("cafe", &attacker),
            Err(CoreError::PublisherMismatch { .. })
        ));
    }
}
