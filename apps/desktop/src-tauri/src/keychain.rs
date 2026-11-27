//! Hoster credentials in the operating system's keychain (macOS Keychain, Windows Credential
//! Manager, Secret Service on Linux), one entry per project.
//!
//! Every keychain read can make the system ask for the user's password. The keychain is
//! therefore read at most once per project and app session (memory cache), and whether
//! credentials exist is taken from `project.json` (`credentialsStored`), never from the keychain.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use wm_core::credentials::Credentials;

const SERVICE: &str = "com.teleschirm.website-manager";

/// Credentials read or stored during this app session.
static CACHE: LazyLock<Mutex<HashMap<String, Credentials>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn entry(project: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, project).map_err(|e| format!("code:keychain-unavailable {e}"))
}

fn cached(project: &str) -> Option<Credentials> {
    CACHE.lock().ok()?.get(project).cloned()
}

fn remember(project: &str, credentials: &Credentials) {
    if let Ok(mut cache) = CACHE.lock() {
        cache.insert(project.to_owned(), credentials.clone());
    }
}

/// Stored credentials of a project, if any. Reads the keychain only on the first call per
/// project and session.
///
/// # Errors
/// Fails if the keychain is not available.
pub fn load(project: &str) -> Result<Option<Credentials>, String> {
    if let Some(credentials) = cached(project) {
        return Ok(Some(credentials));
    }
    match entry(project)?.get_password() {
        Ok(text) => {
            let credentials: Credentials = serde_json::from_str(&text)
                .map_err(|e| format!("stored credentials are unreadable: {e}"))?;
            remember(project, &credentials);
            Ok(Some(credentials))
        }
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
        .map_err(|e| format!("code:keychain-unavailable {e}"))?;
    remember(project, credentials);
    Ok(())
}

/// Removes stored credentials; no error if there are none.
///
/// # Errors
/// Fails if the keychain is not available.
pub fn delete(project: &str) -> Result<(), String> {
    if let Ok(mut cache) = CACHE.lock() {
        cache.remove(project);
    }
    match entry(project)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("code:keychain-unavailable {error}")),
    }
}
