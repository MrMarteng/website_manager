//! Hoster credentials in the operating system's keychain (macOS Keychain, Windows Credential
//! Manager, Secret Service on Linux), one entry per project.

use wm_core::credentials::Credentials;

const SERVICE: &str = "com.teleschirm.website-manager";

fn entry(project: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, project).map_err(|e| format!("code:keychain-unavailable {e}"))
}

/// Stored credentials of a project, if any.
///
/// # Errors
/// Fails if the keychain is not available.
pub fn load(project: &str) -> Result<Option<Credentials>, String> {
    match entry(project)?.get_password() {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| format!("stored credentials are unreadable: {e}")),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("code:keychain-unavailable {error}")),
    }
}

/// Stores credentials for a project, replacing earlier ones.
///
/// # Errors
/// Fails if the keychain is not available.
pub fn store(project: &str, credentials: &Credentials) -> Result<(), String> {
    let text = serde_json::to_string(credentials).map_err(|e| e.to_string())?;
    entry(project)?
        .set_password(&text)
        .map_err(|e| format!("code:keychain-unavailable {e}"))
}

/// Removes stored credentials; no error if there are none.
///
/// # Errors
/// Fails if the keychain is not available.
pub fn delete(project: &str) -> Result<(), String> {
    match entry(project)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("code:keychain-unavailable {error}")),
    }
}

/// Whether credentials are stored (keychain errors count as "no").
#[must_use]
pub fn has(project: &str) -> bool {
    matches!(load(project), Ok(Some(_)))
}
