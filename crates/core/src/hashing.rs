//! SHA-256 checksums of files, streams and byte slices, as lowercase hex.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::CoreError;

/// Returns the SHA-256 checksum of a file as lowercase hex.
///
/// # Errors
/// Fails if the file cannot be read.
pub fn sha256_file(path: &Path) -> Result<String, CoreError> {
    let mut file = File::open(path).map_err(|e| CoreError::io(path, e))?;
    sha256_reader(&mut file).map_err(|e| CoreError::io(path, e))
}

/// Returns the SHA-256 checksum of everything read from `reader` as lowercase hex.
///
/// # Errors
/// Fails if reading fails.
pub fn sha256_reader(reader: &mut impl Read) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Returns the SHA-256 checksum of a byte slice as lowercase hex.
#[must_use]
pub fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_digest_of_empty_input() {
        assert_eq!(
            sha256_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn reader_and_bytes_agree() {
        let data = b"website manager";
        assert_eq!(sha256_reader(&mut &data[..]).unwrap(), sha256_bytes(data));
    }
}
