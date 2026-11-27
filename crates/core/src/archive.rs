//! Writing, verifying and extracting `.tsweb` packages.
//!
//! A package is a ZIP archive with `manifest.json`, `manifest.sig` and the payload files.
//! The signature covers the manifest bytes; the manifest lists the SHA-256 of every payload
//! file. Verification therefore covers the whole content.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use ed25519_dalek::{SigningKey, VerifyingKey};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::hashing::{sha256_file, sha256_reader};
use crate::manifest::Manifest;
use crate::{CoreError, MANIFEST_FILE, SIGNATURE_FILE, signing};

/// Upper limit for the manifest and signature size, to reject oversized entries early.
const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;

/// One payload file: where it is stored in the package and where it comes from.
#[derive(Debug, Clone)]
pub struct PackageEntry {
    /// Path inside the package, with forward slashes.
    pub package_path: String,
    /// Source file on disk.
    pub source: PathBuf,
}

/// A package whose signature and checksums were verified.
#[derive(Debug, Clone)]
pub struct VerifiedPackage {
    pub manifest: Manifest,
    pub publisher: VerifyingKey,
}

/// Writes a signed package to `output`.
///
/// `manifest.files` is replaced by the checksums of `entries`.
///
/// # Errors
/// Fails for unsafe or duplicate package paths and for I/O errors.
pub fn write_package(
    output: &Path,
    mut manifest: Manifest,
    entries: &[PackageEntry],
    key: &SigningKey,
) -> Result<Manifest, CoreError> {
    let mut files = BTreeMap::new();
    for entry in entries {
        if safe_relative_path(&entry.package_path).is_none() || is_reserved(&entry.package_path) {
            return Err(CoreError::UnsafeEntry(entry.package_path.clone()));
        }
        let checksum = sha256_file(&entry.source)?;
        if files.insert(entry.package_path.clone(), checksum).is_some() {
            return Err(CoreError::UnsafeEntry(entry.package_path.clone()));
        }
    }
    manifest.files = files;
    let manifest_bytes = manifest.to_canonical_bytes()?;
    let signature = signing::sign(key, &manifest_bytes);

    let file = File::create(output).map_err(|e| CoreError::io(output, e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file(MANIFEST_FILE, options)?;
    io::Write::write_all(&mut zip, &manifest_bytes).map_err(|e| CoreError::io(output, e))?;
    zip.start_file(SIGNATURE_FILE, options)?;
    io::Write::write_all(&mut zip, signature.as_bytes()).map_err(|e| CoreError::io(output, e))?;

    for entry in entries {
        zip.start_file(entry.package_path.as_str(), options)?;
        let mut source = File::open(&entry.source).map_err(|e| CoreError::io(&entry.source, e))?;
        io::copy(&mut source, &mut zip).map_err(|e| CoreError::io(&entry.source, e))?;
    }
    zip.finish()?;
    Ok(manifest)
}

/// Verifies signature, checksums and entry paths of a package without extracting it.
///
/// # Errors
/// Fails if anything about the package is not as signed.
pub fn verify_package(path: &Path) -> Result<VerifiedPackage, CoreError> {
    let mut archive = open_archive(path)?;
    let verified = read_signed_manifest(&mut archive)?;
    check_entries(&mut archive, &verified.manifest, None)?;
    Ok(verified)
}

/// Verifies a package and extracts it into `target`, which must not exist yet.
///
/// Extraction happens into a temporary sibling folder that is renamed only after every
/// check has passed, so a broken package leaves nothing behind.
///
/// # Errors
/// Fails if `target` exists, the package is not as signed, or writing fails.
pub fn extract_package(path: &Path, target: &Path) -> Result<VerifiedPackage, CoreError> {
    if target.exists() {
        return Err(CoreError::io(
            target,
            io::Error::new(io::ErrorKind::AlreadyExists, "target folder already exists"),
        ));
    }
    let staging = target.with_extension("partial");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| CoreError::io(&staging, e))?;
    }
    fs::create_dir_all(&staging).map_err(|e| CoreError::io(&staging, e))?;

    let result = (|| {
        let mut archive = open_archive(path)?;
        let verified = read_signed_manifest(&mut archive)?;
        check_entries(&mut archive, &verified.manifest, Some(&staging))?;
        Ok(verified)
    })();

    match result {
        Ok(verified) => {
            fs::rename(&staging, target).map_err(|e| CoreError::io(target, e))?;
            Ok(verified)
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            Err(error)
        }
    }
}

fn open_archive(path: &Path) -> Result<ZipArchive<File>, CoreError> {
    let file = File::open(path).map_err(|e| CoreError::io(path, e))?;
    Ok(ZipArchive::new(file)?)
}

fn read_small_entry(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>, CoreError> {
    let entry = archive
        .by_name(name)
        .map_err(|_| CoreError::MissingEntry(name.to_owned()))?;
    let mut bytes = Vec::new();
    entry
        .take(MAX_METADATA_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|e| CoreError::io(name, e))?;
    Ok(bytes)
}

fn read_signed_manifest(archive: &mut ZipArchive<File>) -> Result<VerifiedPackage, CoreError> {
    let manifest_bytes = read_small_entry(archive, MANIFEST_FILE)?;
    let signature_bytes = read_small_entry(archive, SIGNATURE_FILE)?;
    let signature = String::from_utf8(signature_bytes).map_err(|_| CoreError::BadSignature)?;
    let publisher = signing::verify(&signature, &manifest_bytes)?;
    let manifest = Manifest::from_bytes(&manifest_bytes)?;
    Ok(VerifiedPackage {
        manifest,
        publisher,
    })
}

/// Checks every entry; with `target` set, also writes the payload files there.
fn check_entries(
    archive: &mut ZipArchive<File>,
    manifest: &Manifest,
    target: Option<&Path>,
) -> Result<(), CoreError> {
    let mut seen = BTreeSet::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        if entry.is_dir() {
            if safe_relative_path(name.trim_end_matches('/')).is_none() {
                return Err(CoreError::UnsafeEntry(name));
            }
            continue;
        }
        if is_reserved(&name) {
            continue;
        }
        let is_symlink = entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170_000 == 0o120_000);
        let Some(relative) = safe_relative_path(&name).filter(|_| !is_symlink) else {
            return Err(CoreError::UnsafeEntry(name));
        };
        let Some(expected) = manifest.files.get(&name) else {
            return Err(CoreError::UnlistedEntry(name));
        };
        if !seen.insert(name.clone()) {
            return Err(CoreError::UnsafeEntry(name));
        }

        let actual = if let Some(root) = target {
            let destination = root.join(&relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
            }
            let mut file =
                File::create(&destination).map_err(|e| CoreError::io(&destination, e))?;
            let mut tee = TeeReader::new(&mut entry, &mut file);
            sha256_reader(&mut tee).map_err(|e| CoreError::io(&destination, e))?
        } else {
            sha256_reader(&mut entry).map_err(|e| CoreError::io(&name, e))?
        };
        if &actual != expected {
            return Err(CoreError::ChecksumMismatch(name));
        }
    }
    if let Some(missing) = manifest.files.keys().find(|name| !seen.contains(*name)) {
        return Err(CoreError::MissingEntry(missing.clone()));
    }
    Ok(())
}

fn is_reserved(name: &str) -> bool {
    name == MANIFEST_FILE || name == SIGNATURE_FILE
}

/// Returns the path if `name` is a plain relative path that stays inside the target folder.
///
/// Rejects absolute paths, drive letters, backslashes, `.`/`..` components and control
/// characters, so the same rule holds on Windows, macOS and Linux.
#[must_use]
pub fn safe_relative_path(name: &str) -> Option<PathBuf> {
    if name.is_empty()
        || name.contains('\\')
        || name.contains(':')
        || name.starts_with('/')
        || name.chars().any(char::is_control)
    {
        return None;
    }
    let mut path = PathBuf::new();
    for part in name.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return None;
        }
        path.push(part);
    }
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
        .then_some(path)
}

/// Copies everything read into a writer while hashing.
struct TeeReader<'a, R: Read, W: io::Write> {
    reader: &'a mut R,
    writer: &'a mut W,
}

impl<'a, R: Read, W: io::Write> TeeReader<'a, R, W> {
    fn new(reader: &'a mut R, writer: &'a mut W) -> Self {
        Self { reader, writer }
    }
}

impl<R: Read, W: io::Write> Read for TeeReader<'_, R, W> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.reader.read(buffer)?;
        self.writer.write_all(&buffer[..read])?;
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FORMAT_VERSION;
    use crate::signing::generate_signing_key;
    use std::io::Write;

    fn manifest() -> Manifest {
        Manifest {
            format_version: FORMAT_VERSION,
            name: "beispiel".into(),
            version: "1.0.0".into(),
            created: "2026-11-27T08:00:00Z".into(),
            node_version: None,
            changelog: vec![],
            logo: None,
            has_credentials: false,
            files: BTreeMap::new(),
        }
    }

    fn sample_package(dir: &Path) -> PathBuf {
        let source = dir.join("index.html");
        fs::write(&source, "<h1>Hallo</h1>").unwrap();
        let output = dir.join("beispiel.tsweb");
        let key = generate_signing_key().unwrap();
        write_package(
            &output,
            manifest(),
            &[PackageEntry {
                package_path: "dist/index.html".into(),
                source,
            }],
            &key,
        )
        .unwrap();
        output
    }

    #[test]
    fn write_verify_extract() {
        let dir = tempfile::tempdir().unwrap();
        let package = sample_package(dir.path());
        let verified = verify_package(&package).unwrap();
        assert_eq!(verified.manifest.files.len(), 1);

        let target = dir.path().join("out");
        extract_package(&package, &target).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("dist/index.html")).unwrap(),
            "<h1>Hallo</h1>"
        );
    }

    #[test]
    fn rejects_existing_target() {
        let dir = tempfile::tempdir().unwrap();
        let package = sample_package(dir.path());
        let target = dir.path().join("out");
        fs::create_dir(&target).unwrap();
        assert!(extract_package(&package, &target).is_err());
    }

    /// Rewrites a package with one extra or changed entry, keeping the original signature.
    fn tamper(package: &Path, name: &str, content: &[u8]) -> PathBuf {
        let tampered = package.with_file_name("tampered.tsweb");
        let mut archive = ZipArchive::new(File::open(package).unwrap()).unwrap();
        let mut writer = ZipWriter::new(File::create(&tampered).unwrap());
        let options = SimpleFileOptions::default();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            if entry.name() == name {
                continue;
            }
            writer.start_file(entry.name().to_owned(), options).unwrap();
            io::copy(&mut entry, &mut writer).unwrap();
        }
        writer.start_file(name, options).unwrap();
        writer.write_all(content).unwrap();
        writer.finish().unwrap();
        tampered
    }

    #[test]
    fn rejects_changed_file() {
        let dir = tempfile::tempdir().unwrap();
        let package = sample_package(dir.path());
        let tampered = tamper(&package, "dist/index.html", b"<script>evil()</script>");
        assert!(matches!(
            verify_package(&tampered),
            Err(CoreError::ChecksumMismatch(_))
        ));
    }

    #[test]
    fn rejects_unlisted_file() {
        let dir = tempfile::tempdir().unwrap();
        let package = sample_package(dir.path());
        let tampered = tamper(&package, "project/postinstall.js", b"evil()");
        assert!(matches!(
            verify_package(&tampered),
            Err(CoreError::UnlistedEntry(_))
        ));
    }

    #[test]
    fn rejects_path_traversal_and_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        let package = sample_package(dir.path());
        let tampered = tamper(&package, "../../evil.sh", b"evil");
        let target = dir.path().join("out");
        assert!(matches!(
            extract_package(&tampered, &target),
            Err(CoreError::UnsafeEntry(_))
        ));
        assert!(!target.exists());
        assert!(!dir.path().join("out.partial").exists());
    }

    #[test]
    fn rejects_changed_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let package = sample_package(dir.path());
        let mut forged = manifest();
        forged.name = "anders".into();
        let tampered = tamper(
            &package,
            MANIFEST_FILE,
            &forged.to_canonical_bytes().unwrap(),
        );
        assert!(matches!(
            verify_package(&tampered),
            Err(CoreError::BadSignature)
        ));
    }

    #[test]
    fn safe_paths() {
        assert!(safe_relative_path("dist/index.html").is_some());
        for bad in [
            "",
            "/etc/passwd",
            "../x",
            "a/../../x",
            "a/./b",
            "C:/x",
            "a\\b",
            "a//b",
            "a/\u{0}b",
        ] {
            assert!(
                safe_relative_path(bad).is_none(),
                "{bad:?} must be rejected"
            );
        }
    }
}
