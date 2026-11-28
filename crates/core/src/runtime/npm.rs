//! npm lockfile helpers shared by `pack` and the desktop app.

use serde_json::Value;

/// Registries every customer can reach.
const PUBLIC_REGISTRIES: &[&str] = &["https://registry.npmjs.org/"];

/// A lockfile entry that comes from a private registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivatePackage {
    /// Key in `packages`, e.g. `node_modules/@scope/name`.
    pub key: String,
    /// Package name, e.g. `@scope/name`.
    pub name: String,
    /// Locked version.
    pub version: String,
}

/// Packages of a lockfile (version 2 or 3) whose `resolved` URL is not a public registry.
#[must_use]
pub fn private_packages(lock: &Value) -> Vec<PrivatePackage> {
    let Some(packages) = lock.get("packages").and_then(Value::as_object) else {
        return Vec::new();
    };
    packages
        .iter()
        .filter_map(|(key, entry)| {
            let resolved = entry.get("resolved")?.as_str()?;
            let remote = resolved.starts_with("https://") || resolved.starts_with("http://");
            if !remote
                || PUBLIC_REGISTRIES
                    .iter()
                    .any(|public| resolved.starts_with(public))
            {
                return None;
            }
            let name = key.rsplit_once("node_modules/").map(|(_, name)| name)?;
            Some(PrivatePackage {
                key: key.clone(),
                name: name.to_owned(),
                version: entry.get("version")?.as_str()?.to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_private_packages_only() {
        let lock = serde_json::json!({
            "packages": {
                "": { "name": "site" },
                "node_modules/astro": { "version": "5.18.2", "resolved": "https://registry.npmjs.org/astro/-/astro-5.18.2.tgz" },
                "node_modules/@x/cms": { "version": "0.3.1", "resolved": "https://git.example.com/npm/@x/cms/-/cms-0.3.1.tgz" },
                "node_modules/local": { "version": "1.0.0", "resolved": "file:vendor/local-1.0.0.tgz" }
            }
        });
        let names: Vec<String> = private_packages(&lock)
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, vec!["@x/cms"]);
    }
}
