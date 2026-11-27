//! Ships npm packages from private registries inside the package.
//!
//! Customers cannot reach the agency's registry. `pack` therefore stores such packages as
//! tarballs under `project/vendor/` and points the lockfile at them (`file:vendor/…`); the
//! integrity hashes stay, so npm still verifies them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::pack::PackError;

pub use wm_core::runtime::npm::private_packages;

/// Points a lockfile entry at a vendored tarball.
pub fn point_to_tarball(lock: &mut Value, key: &str, file_name: &str) {
    if let Some(entry) = lock
        .get_mut("packages")
        .and_then(|packages| packages.get_mut(key))
        .and_then(Value::as_object_mut)
    {
        entry.insert(
            "resolved".into(),
            Value::String(format!("file:vendor/{file_name}")),
        );
    }
}

/// Result of vendoring: rewritten lockfile and the tarballs, both in the staging folder.
pub struct Vendored {
    pub lockfile: PathBuf,
    pub tarballs: Vec<PathBuf>,
    pub names: Vec<String>,
}

/// Fetches private packages with `npm pack` (using the agency's npm configuration) and
/// rewrites the lockfile. `None` if the project has no private packages.
///
/// # Errors
/// Fails if the lockfile is invalid or `npm pack` fails.
pub fn vendor(project: &Path, staging: &Path) -> Result<Option<Vendored>, PackError> {
    let lock_path = project.join("package-lock.json");
    let text = fs::read_to_string(&lock_path).map_err(|e| PackError::File {
        path: lock_path.clone(),
        message: e.to_string(),
    })?;
    let mut lock: Value = serde_json::from_str(&text)
        .map_err(|e| PackError::Config(format!("package-lock.json: {e}")))?;
    let private = private_packages(&lock);
    if private.is_empty() {
        return Ok(None);
    }

    let vendor_dir = staging.join("vendor");
    fs::create_dir_all(&vendor_dir).map_err(|e| PackError::File {
        path: vendor_dir.clone(),
        message: e.to_string(),
    })?;
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let mut tarballs = Vec::new();
    let mut names = Vec::new();
    for package in &private {
        let spec = format!("{}@{}", package.name, package.version);
        let output = Command::new(npm)
            .args(["pack", &spec, "--silent", "--pack-destination"])
            .arg(&vendor_dir)
            .current_dir(project)
            .output()
            .map_err(|e| PackError::Build(format!("npm pack {spec}: {e}")))?;
        let file_name = String::from_utf8_lossy(&output.stdout)
            .lines()
            .last()
            .unwrap_or_default()
            .trim()
            .to_owned();
        let tarball = vendor_dir.join(&file_name);
        if !output.status.success() || file_name.is_empty() || !tarball.is_file() {
            return Err(PackError::Build(format!(
                "npm pack {spec} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        point_to_tarball(&mut lock, &package.key, &file_name);
        tarballs.push(tarball);
        names.push(spec);
    }

    let lockfile = staging.join("package-lock.json");
    let content = serde_json::to_string_pretty(&lock)
        .map_err(|e| PackError::Config(format!("package-lock.json: {e}")))?;
    fs::write(&lockfile, content + "\n").map_err(|e| PackError::File {
        path: lockfile.clone(),
        message: e.to_string(),
    })?;
    Ok(Some(Vendored {
        lockfile,
        tarballs,
        names,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock() -> Value {
        serde_json::json!({
            "lockfileVersion": 3,
            "packages": {
                "": { "name": "site" },
                "node_modules/astro": {
                    "version": "5.18.2",
                    "resolved": "https://registry.npmjs.org/astro/-/astro-5.18.2.tgz",
                    "integrity": "sha512-a"
                },
                "node_modules/@teleschirm/astro-cms": {
                    "version": "0.3.1",
                    "resolved": "https://git.nas.local/api/packages/teleschirm/npm/@teleschirm/astro-cms/-/0.3.1/astro-cms-0.3.1.tgz",
                    "integrity": "sha512-b"
                },
                "node_modules/a/node_modules/@x/y": {
                    "version": "1.0.0",
                    "resolved": "https://npm.example.com/@x/y/-/y-1.0.0.tgz"
                },
                "node_modules/local": { "version": "1.0.0", "resolved": "file:../local" }
            }
        })
    }

    #[test]
    fn finds_private_packages_only() {
        let names: Vec<String> = private_packages(&lock())
            .into_iter()
            .map(|package| format!("{}@{}", package.name, package.version))
            .collect();
        assert_eq!(names, vec!["@teleschirm/astro-cms@0.3.1", "@x/y@1.0.0"]);
    }

    #[test]
    fn rewrite_keeps_integrity() {
        let mut lock = lock();
        point_to_tarball(
            &mut lock,
            "node_modules/@teleschirm/astro-cms",
            "teleschirm-astro-cms-0.3.1.tgz",
        );
        let entry = &lock["packages"]["node_modules/@teleschirm/astro-cms"];
        assert_eq!(
            entry["resolved"],
            "file:vendor/teleschirm-astro-cms-0.3.1.tgz"
        );
        assert_eq!(entry["integrity"], "sha512-b");
        assert!(
            private_packages(&lock)
                .iter()
                .all(|p| p.name != "@teleschirm/astro-cms")
        );
    }
}
