use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::CoreError;
use crate::hashing::sha256_bytes;

/// Creates a new random Ed25519 signing key.
///
/// # Errors
/// Fails if the operating system cannot provide random numbers.
pub fn generate_signing_key() -> Result<SigningKey, CoreError> {
    let mut seed = [0_u8; 32];
    getrandom::fill(&mut seed).map_err(|e| CoreError::Random(e.to_string()))?;
    Ok(SigningKey::from_bytes(&seed))
}

/// Encodes a public key as Base64 (the form stored in trust stores and shown to users).
#[must_use]
pub fn encode_public_key(key: &VerifyingKey) -> String {
    STANDARD.encode(key.as_bytes())
}

/// Decodes a Base64 public key.
///
/// # Errors
/// Fails for invalid Base64 or an invalid key.
pub fn decode_public_key(text: &str) -> Result<VerifyingKey, CoreError> {
    let bytes = STANDARD
        .decode(text.trim())
        .map_err(|e| CoreError::InvalidKey(e.to_string()))?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| CoreError::InvalidKey("public key must be 32 bytes".into()))?;
    VerifyingKey::from_bytes(&array).map_err(|e| CoreError::InvalidKey(e.to_string()))
}

/// Short, readable fingerprint of a public key (first 16 hex digits of its SHA-256).
#[must_use]
pub fn fingerprint(key: &VerifyingKey) -> String {
    let digest = sha256_bytes(key.as_bytes());
    digest
        .get(..16)
        .unwrap_or(&digest)
        .as_bytes()
        .chunks(4)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect::<Vec<_>>()
        .join(":")
}

/// Writes a signing key as Base64 to `path` (permissions 0600 on Unix).
///
/// # Errors
/// Fails if the file cannot be written.
pub fn save_signing_key(key: &SigningKey, path: &Path) -> Result<(), CoreError> {
    fs::write(path, STANDARD.encode(key.to_bytes())).map_err(|e| CoreError::io(path, e))?;
    restrict_permissions(path)
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), CoreError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| CoreError::io(path, e))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), CoreError> {
    Ok(())
}

/// Reads a Base64 signing key from `path`.
///
/// # Errors
/// Fails if the file cannot be read or does not contain a valid key.
pub fn load_signing_key(path: &Path) -> Result<SigningKey, CoreError> {
    let text = fs::read_to_string(path).map_err(|e| CoreError::io(path, e))?;
    let bytes = STANDARD
        .decode(text.trim())
        .map_err(|e| CoreError::InvalidKey(e.to_string()))?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| CoreError::InvalidKey("signing key must be 32 bytes".into()))?;
    Ok(SigningKey::from_bytes(&array))
}

/// Signature file content: public key and signature, both Base64, one per line.
#[must_use]
pub fn sign(key: &SigningKey, message: &[u8]) -> String {
    let signature = key.sign(message);
    format!(
        "{}\n{}\n",
        encode_public_key(&key.verifying_key()),
        STANDARD.encode(signature.to_bytes())
    )
}

/// Verifies signature file content against `message` and returns the signer's public key.
///
/// # Errors
/// Fails if the content is malformed or the signature does not match.
pub fn verify(signature_file: &str, message: &[u8]) -> Result<VerifyingKey, CoreError> {
    let mut lines = signature_file.lines();
    let key = decode_public_key(lines.next().ok_or(CoreError::BadSignature)?)?;
    let signature_bytes = STANDARD
        .decode(lines.next().ok_or(CoreError::BadSignature)?.trim())
        .map_err(|_| CoreError::BadSignature)?;
    let signature = Signature::from_slice(&signature_bytes).map_err(|_| CoreError::BadSignature)?;
    key.verify(message, &signature)
        .map_err(|_| CoreError::BadSignature)?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify() {
        let key = generate_signing_key().unwrap();
        let signature = sign(&key, b"manifest");
        let signer = verify(&signature, b"manifest").unwrap();
        assert_eq!(signer, key.verifying_key());
    }

    #[test]
    fn changed_message_is_rejected() {
        let key = generate_signing_key().unwrap();
        let signature = sign(&key, b"manifest");
        assert!(matches!(
            verify(&signature, b"manifest!"),
            Err(CoreError::BadSignature)
        ));
    }

    #[test]
    fn key_file_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("signing.key");
        let key = generate_signing_key().unwrap();
        save_signing_key(&key, &path).unwrap();
        assert_eq!(load_signing_key(&path).unwrap().to_bytes(), key.to_bytes());
    }

    #[test]
    fn fingerprint_is_grouped() {
        let key = generate_signing_key().unwrap();
        let print = fingerprint(&key.verifying_key());
        assert_eq!(print.len(), 19);
        assert_eq!(print.matches(':').count(), 3);
    }
}
