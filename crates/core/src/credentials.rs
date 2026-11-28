//! Hoster credentials, encrypted with a password for transport (`credentials.enc`).
//!
//! Key derivation Argon2id, encryption AES-256-GCM. The parameters are stored next to the
//! ciphertext, so they can be raised later without breaking existing files.

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{self, Aead, KeyInit};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use crate::CoreError;

/// File name inside a package.
pub const CREDENTIALS_FILE: &str = "credentials.enc";

/// Login data for the webspace. `Debug` never shows the values.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    /// Password for SFTP or FTPS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Private SSH key (OpenSSH format) as text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Credentials")
            .field("password", &self.password.as_ref().map(|_| "***"))
            .field("private_key", &self.private_key.as_ref().map(|_| "***"))
            .finish()
    }
}

impl Credentials {
    /// Whether neither a password nor a key is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.password.is_none() && self.private_key.is_none()
    }
}

/// Cost parameters of the key derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    /// Memory in KiB.
    pub memory_kib: u32,
    /// Number of passes over the memory.
    pub iterations: u32,
    /// Number of lanes (threads).
    pub parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            memory_kib: 64 * 1024,
            iterations: 3,
            parallelism: 1,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct EncryptedFile {
    kdf: String,
    params: KdfParams,
    salt: String,
    nonce: String,
    ciphertext: String,
}

const KDF_NAME: &str = "argon2id";

/// Encrypts credentials with `password`; returns the file content.
///
/// # Errors
/// Fails if the password is too short (fewer than 8 characters) or random numbers are
/// unavailable.
///
/// # Examples
///
/// ```
/// use wm_core::CoreError;
/// use wm_core::credentials::{Credentials, decrypt, encrypt};
///
/// let access = Credentials { password: Some("webspace-secret".into()), private_key: None };
/// let file = encrypt(&access, "package password")?;
/// assert!(!file.contains("webspace-secret"));
///
/// assert_eq!(decrypt(&file, "package password")?, access);
/// assert!(matches!(decrypt(&file, "wrong password"), Err(CoreError::WrongPassword)));
/// # Ok::<(), CoreError>(())
/// ```
pub fn encrypt(credentials: &Credentials, password: &str) -> Result<String, CoreError> {
    encrypt_with(credentials, password, KdfParams::default())
}

/// Like [`encrypt`] with explicit cost parameters.
///
/// # Errors
/// Fails if the password is too short or random numbers are unavailable.
pub fn encrypt_with(
    credentials: &Credentials,
    password: &str,
    params: KdfParams,
) -> Result<String, CoreError> {
    if password.chars().count() < 8 {
        return Err(CoreError::Credentials(
            "package password must have at least 8 characters".into(),
        ));
    }
    let mut salt = [0_u8; 16];
    let mut nonce = [0_u8; 12];
    getrandom::fill(&mut salt).map_err(|e| CoreError::Random(e.to_string()))?;
    getrandom::fill(&mut nonce).map_err(|e| CoreError::Random(e.to_string()))?;
    let key = derive_key(password, &salt, params)?;
    let plaintext =
        serde_json::to_vec(credentials).map_err(|e| CoreError::json("credentials", e))?;
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| CoreError::Credentials(e.to_string()))?;
    let ciphertext = cipher
        .encrypt(&nonce_array(&nonce)?, plaintext.as_slice())
        .map_err(|e| CoreError::Credentials(e.to_string()))?;
    let file = EncryptedFile {
        kdf: KDF_NAME.into(),
        params,
        salt: STANDARD.encode(salt),
        nonce: STANDARD.encode(nonce),
        ciphertext: STANDARD.encode(ciphertext),
    };
    serde_json::to_string_pretty(&file).map_err(|e| CoreError::json("credentials", e))
}

/// Decrypts file content produced by [`encrypt`] (example there).
///
/// # Errors
/// [`CoreError::WrongPassword`] for a wrong password or changed file, other errors for
/// malformed content.
pub fn decrypt(content: &str, password: &str) -> Result<Credentials, CoreError> {
    let file: EncryptedFile =
        serde_json::from_str(content).map_err(|e| CoreError::json("credentials", e))?;
    if file.kdf != KDF_NAME {
        return Err(CoreError::Credentials(format!(
            "unknown key derivation {:?}",
            file.kdf
        )));
    }
    let decode = |text: &str| {
        STANDARD
            .decode(text)
            .map_err(|e| CoreError::Credentials(e.to_string()))
    };
    let salt = decode(&file.salt)?;
    let nonce = decode(&file.nonce)?;
    let ciphertext = decode(&file.ciphertext)?;
    if nonce.len() != 12 {
        return Err(CoreError::Credentials("invalid nonce".into()));
    }
    let key = derive_key(password, &salt, file.params)?;
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| CoreError::Credentials(e.to_string()))?;
    let plaintext = cipher
        .decrypt(&nonce_array(&nonce)?, ciphertext.as_slice())
        .map_err(|_| CoreError::WrongPassword)?;
    serde_json::from_slice(&plaintext).map_err(|e| CoreError::json("credentials", e))
}

fn nonce_array(bytes: &[u8]) -> Result<aead::Nonce<Aes256Gcm>, CoreError> {
    aead::Nonce::<Aes256Gcm>::try_from(bytes)
        .map_err(|_| CoreError::Credentials("invalid nonce".into()))
}

fn derive_key(password: &str, salt: &[u8], params: KdfParams) -> Result<[u8; 32], CoreError> {
    let params = Params::new(
        params.memory_kib,
        params.iterations,
        params.parallelism,
        Some(32),
    )
    .map_err(|e| CoreError::Credentials(e.to_string()))?;
    let mut key = [0_u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| CoreError::Credentials(e.to_string()))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: KdfParams = KdfParams {
        memory_kib: 1024,
        iterations: 1,
        parallelism: 1,
    };

    fn sample() -> Credentials {
        Credentials {
            password: Some("geheim-123".into()),
            private_key: None,
        }
    }

    #[test]
    fn round_trip() {
        let file = encrypt_with(&sample(), "paketpasswort", FAST).unwrap();
        assert!(!file.contains("geheim-123"));
        assert_eq!(decrypt(&file, "paketpasswort").unwrap(), sample());
    }

    #[test]
    fn wrong_password_and_tampering_are_rejected() {
        let file = encrypt_with(&sample(), "paketpasswort", FAST).unwrap();
        assert!(matches!(
            decrypt(&file, "falsch-falsch"),
            Err(CoreError::WrongPassword)
        ));

        let mut parsed: serde_json::Value = serde_json::from_str(&file).unwrap();
        let ciphertext = parsed["ciphertext"].as_str().unwrap();
        let mut bytes = STANDARD.decode(ciphertext).unwrap();
        bytes[0] ^= 1;
        parsed["ciphertext"] = STANDARD.encode(bytes).into();
        assert!(matches!(
            decrypt(&parsed.to_string(), "paketpasswort"),
            Err(CoreError::WrongPassword)
        ));
    }

    #[test]
    fn short_password_is_refused() {
        assert!(encrypt_with(&sample(), "kurz", FAST).is_err());
    }

    #[test]
    fn debug_hides_values() {
        assert!(!format!("{:?}", sample()).contains("geheim"));
    }
}
