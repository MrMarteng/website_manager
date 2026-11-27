//! Credentials files written by earlier releases must stay readable after dependency updates.
#![allow(clippy::unwrap_used, clippy::expect_used)] // tests may unwrap

use wm_core::credentials::decrypt;

#[test]
fn file_from_release_0_3_decrypts() {
    // Written by `website-manager pack --credentials` 0.3.x (aes-gcm 0.10, Argon2id 64 MiB/3).
    let content = include_str!("fixtures/credentials-0.3.enc");
    let credentials = decrypt(content, "testpaket123").unwrap();
    assert_eq!(credentials.password.as_deref(), Some("nur-ein-test"));
    assert!(credentials.private_key.is_none());
    assert!(decrypt(content, "falsches-passwort").is_err());
}
