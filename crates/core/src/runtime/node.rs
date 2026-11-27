//! Downloads official Node.js builds from nodejs.org, verifies their SHA-256 and caches them.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use super::Platform;
use crate::CoreError;
use crate::archive::safe_relative_path;
use crate::hashing::sha256_file;

const DIST_URL: &str = "https://nodejs.org/dist";

/// An installed Node.js version.
#[derive(Debug, Clone)]
pub struct NodeRuntime {
    /// Folder that contains the `node` executable (put on `PATH` for child processes).
    pub bin_dir: PathBuf,
    /// The `node` executable.
    pub node: PathBuf,
    /// The npm entry script, started as `node <npm_cli> …` to avoid shell wrappers.
    pub npm_cli: PathBuf,
}

/// Name of the release folder, e.g. `node-v22.12.0-darwin-arm64`.
#[must_use]
pub fn release_name(version: &str, platform: Platform) -> String {
    format!("node-v{version}-{}-{}", platform.os, platform.arch)
}

/// Archive file name of a release.
#[must_use]
pub fn archive_name(version: &str, platform: Platform) -> String {
    let extension = if platform.is_windows() {
        "zip"
    } else {
        "tar.gz"
    };
    format!("{}.{extension}", release_name(version, platform))
}

/// Finds the checksum of `file_name` in the content of a `SHASUMS256.txt`.
#[must_use]
pub fn checksum_from_shasums(shasums: &str, file_name: &str) -> Option<String> {
    shasums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let checksum = parts.next()?;
        (parts.next()? == file_name).then(|| checksum.to_lowercase())
    })
}

/// Paths of an extracted release in `release_dir`.
#[must_use]
pub fn runtime_paths(release_dir: &Path, platform: Platform) -> NodeRuntime {
    if platform.is_windows() {
        NodeRuntime {
            bin_dir: release_dir.to_path_buf(),
            node: release_dir.join("node.exe"),
            npm_cli: release_dir.join("node_modules/npm/bin/npm-cli.js"),
        }
    } else {
        NodeRuntime {
            bin_dir: release_dir.join("bin"),
            node: release_dir.join("bin/node"),
            npm_cli: release_dir.join("lib/node_modules/npm/bin/npm-cli.js"),
        }
    }
}

/// Returns the cached runtime for `version`, downloading and verifying it first if needed.
///
/// # Errors
/// Fails if the download fails, the checksum does not match or extraction fails.
pub fn ensure(version: &str, cache_dir: &Path) -> Result<NodeRuntime, CoreError> {
    if !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err(CoreError::Download {
            url: version.to_owned(),
            message: "Node.js version must look like 22.12.0".into(),
        });
    }
    let platform = Platform::current()?;
    let release = release_name(version, platform);
    let release_dir = cache_dir.join(&release);
    let runtime = runtime_paths(&release_dir, platform);
    if runtime.node.is_file() {
        return Ok(runtime);
    }

    fs::create_dir_all(cache_dir).map_err(|e| CoreError::io(cache_dir, e))?;
    let archive = archive_name(version, platform);
    let shasums_url = format!("{DIST_URL}/v{version}/SHASUMS256.txt");
    let shasums = download_text(&shasums_url)?;
    let expected =
        checksum_from_shasums(&shasums, &archive).ok_or_else(|| CoreError::Download {
            url: shasums_url.clone(),
            message: format!("no checksum for {archive}"),
        })?;

    let archive_url = format!("{DIST_URL}/v{version}/{archive}");
    let archive_path = cache_dir.join(format!("{archive}.download"));
    download_file(&archive_url, &archive_path)?;
    let actual = sha256_file(&archive_path)?;
    if actual != expected {
        let _ = fs::remove_file(&archive_path);
        return Err(CoreError::ChecksumMismatch(archive));
    }

    let staging = cache_dir.join(format!("{release}.partial"));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| CoreError::io(&staging, e))?;
    }
    fs::create_dir_all(&staging).map_err(|e| CoreError::io(&staging, e))?;
    let extracted = if platform.is_windows() {
        extract_zip(&archive_path, &staging)
    } else {
        extract_tar_gz(&archive_path, &staging)
    };
    let _ = fs::remove_file(&archive_path);
    extracted?;
    // Archives contain one top-level folder named like the release.
    fs::rename(staging.join(&release), &release_dir).map_err(|e| CoreError::io(&release_dir, e))?;
    let _ = fs::remove_dir_all(&staging);
    Ok(runtime)
}

/// Removes cached Node.js releases whose version is not in `keep`; returns the removed folders.
///
/// # Errors
/// Fails if the cache folder cannot be read or a release cannot be removed.
pub fn prune(cache_dir: &Path, keep: &[String]) -> Result<Vec<PathBuf>, CoreError> {
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return Ok(Vec::new());
    };
    let mut removed = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(version) = name
            .strip_prefix("node-v")
            .and_then(|rest| rest.split('-').next())
        else {
            continue;
        };
        if keep.iter().any(|kept| kept == version) || !entry.path().is_dir() {
            continue;
        }
        fs::remove_dir_all(entry.path()).map_err(|e| CoreError::io(entry.path(), e))?;
        removed.push(entry.path());
    }
    Ok(removed)
}

fn download_text(url: &str) -> Result<String, CoreError> {
    ureq::get(url)
        .call()
        .map_err(|e| CoreError::Download {
            url: url.to_owned(),
            message: e.to_string(),
        })?
        .into_string()
        .map_err(|e| CoreError::Download {
            url: url.to_owned(),
            message: e.to_string(),
        })
}

fn download_file(url: &str, target: &Path) -> Result<(), CoreError> {
    let response = ureq::get(url).call().map_err(|e| CoreError::Download {
        url: url.to_owned(),
        message: e.to_string(),
    })?;
    let mut file = File::create(target).map_err(|e| CoreError::io(target, e))?;
    io::copy(&mut response.into_reader(), &mut file).map_err(|e| CoreError::io(target, e))?;
    Ok(())
}

fn extract_tar_gz(archive: &Path, target: &Path) -> Result<(), CoreError> {
    let file = File::open(archive).map_err(|e| CoreError::io(archive, e))?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    tar.set_preserve_permissions(true);
    for entry in tar.entries().map_err(|e| CoreError::io(archive, e))? {
        let mut entry = entry.map_err(|e| CoreError::io(archive, e))?;
        // `unpack_in` refuses paths that leave `target`.
        entry
            .unpack_in(target)
            .map_err(|e| CoreError::io(target, e))?;
    }
    Ok(())
}

fn extract_zip(archive: &Path, target: &Path) -> Result<(), CoreError> {
    let file = File::open(archive).map_err(|e| CoreError::io(archive, e))?;
    let mut zip = zip::ZipArchive::new(file)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().trim_end_matches('/').to_owned();
        let relative =
            safe_relative_path(&name).ok_or_else(|| CoreError::UnsafeEntry(name.clone()))?;
        let destination = target.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&destination).map_err(|e| CoreError::io(&destination, e))?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        let mut output = File::create(&destination).map_err(|e| CoreError::io(&destination, e))?;
        io::copy(&mut entry, &mut output).map_err(|e| CoreError::io(&destination, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAC: Platform = Platform {
        os: "darwin",
        arch: "arm64",
    };
    const WINDOWS: Platform = Platform {
        os: "win",
        arch: "x64",
    };

    #[test]
    fn archive_names() {
        assert_eq!(
            archive_name("22.12.0", MAC),
            "node-v22.12.0-darwin-arm64.tar.gz"
        );
        assert_eq!(
            archive_name("22.12.0", WINDOWS),
            "node-v22.12.0-win-x64.zip"
        );
    }

    #[test]
    fn finds_checksum() {
        let shasums =
            "aaaa  node-v22.12.0-darwin-x64.tar.gz\nBBBB  node-v22.12.0-darwin-arm64.tar.gz\n";
        assert_eq!(
            checksum_from_shasums(shasums, "node-v22.12.0-darwin-arm64.tar.gz").as_deref(),
            Some("bbbb")
        );
        assert!(checksum_from_shasums(shasums, "node-v22.12.0-win-x64.zip").is_none());
    }

    #[test]
    fn windows_paths_have_no_bin_folder() {
        let runtime = runtime_paths(Path::new("C"), WINDOWS);
        assert_eq!(runtime.node, Path::new("C").join("node.exe"));
    }

    #[test]
    fn prune_keeps_listed_versions() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "node-v22.12.0-darwin-arm64",
            "node-v20.1.0-darwin-arm64",
            "other",
        ] {
            fs::create_dir(dir.path().join(name)).unwrap();
        }
        let removed = prune(dir.path(), &["22.12.0".to_owned()]).unwrap();
        assert_eq!(removed.len(), 1);
        assert!(dir.path().join("node-v22.12.0-darwin-arm64").exists());
        assert!(dir.path().join("other").exists());
    }

    #[test]
    fn rejects_odd_versions() {
        let dir = tempfile::tempdir().unwrap();
        assert!(ensure("../../x", dir.path()).is_err());
    }
}
