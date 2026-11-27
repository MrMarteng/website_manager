# Changelog

All notable changes. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.3.2] - 2026-11-27

### Changed

- Dependencies updated: `zip` 9, `ureq` 3, `aes-gcm` 0.11, `ed25519-dalek` 3, `sha2` 0.11.
  Packages and credentials files from earlier versions stay readable (regression test).

### Fixed

- Development mode on Windows opened a wrong address (`http://localhost:4321/` followed by a
  colour code) and showed "404". Colour codes are now switched off (`NO_COLOR`, `FORCE_COLOR=0`)
  and removed from tool output, so the log is plain text as well.

## [0.3.1] - 2026-11-27

First release on GitHub; fixes from the first test on Windows.

### Added

- Releases: a tag `v*` creates a draft pre-release with installers for Windows, macOS and
  Linux, the CLI programs and `SHA256SUMS`; the release text comes from this changelog.
- GitHub workflow: checks (formatting, Clippy, tests) on every push; installers for Windows, macOS
  and Linux on tags and on demand.
- Guide for testing on Windows (`docs/windows-test.md`).
- Security policy (private vulnerability reporting), code of conduct, issue and pull request
  templates, code owners and weekly Dependabot updates.

### Changed

- `node_version` may be a major version (`22`) or `lts`; it resolves to the newest matching
  Node.js release (offline: the newest cached one). Without `node_version` the newest LTS is used
  instead of 22.12.0.
- Documentation and changelog are written in English; `docs/architektur.md` is now
  `docs/architecture.md`, `docs/formate/` is now `docs/formats/` (`paket.md` → `package.md`).

### Fixed

- Rebuilding a package whose lockfile still points at a private npm registry (created before
  0.3.0) now stops with a clear message instead of a network error from npm.

## [0.3.0] - 2026-11-27

Rebuild and development mode on the customer's computer (phase 1b), tested on macOS only.

### Added

- "Rebuild" and development mode in the app: the Node.js version of the package is downloaded and
  verified, dependencies are installed with `npm ci --ignore-scripts`, npm scripts run without a
  shell (also on Windows); the new build replaces the version's website, the previous build is
  kept. Development mode opens in the browser and stops together with all child processes.
- `pack` ships packages from private npm registries as tarballs (`project/vendor/`).
- Manifest: optional field `build_output`.
- Node.js versions no longer needed are removed from the cache.

### Fixed

- The keychain is read only when uploading, at most once per project and session; whether
  credentials are stored is kept in `project.json`. Before, macOS asked for the password on every
  refresh of the view (for example after "Rebuild").
- Stopping development mode no longer prints "No such process".

## [0.2.0] - 2026-11-27

Core workflow (phase 1a), tested on macOS only.

### Added

- Projects with versions: an update replaces the current version, the last five are kept, older
  ones go to the trash; an earlier version can be restored and uploaded again. Files of the
  previous version changed by the customer are reported on update.
- Credentials: `pack --credentials` ships them encrypted (Argon2id, AES-256-GCM, package
  password); the app stores them in the keychain, exports and imports them with a transfer
  password and can remove them. Login with password or SSH key.
- Server identity: `pack` records the server's SSH host keys in the package; without a matching
  key the app uploads only after confirmation.
- Upload: live progress, list of new and deleted files, "Upload now" checks first and asks with
  the number of changes; open the website in the browser after the upload.
- Settings per project (connection, server, user, port, target folder, website address), own
  mappings and protection rules in expert mode; settings survive updates.
- Open packages by double-click, second app start and drag and drop; start page with the known
  websites.
- File list with filter, opening in the matching program, show folder; open the audit report.
- `teleschirm-deploy`: list of uploaded and deleted files per run, change of the target folder
  (`with_remote_root`).

## [0.1.0] - 2026-11-27

Feasibility state (phase 0), tested on macOS only.

### Added

- Package format `.tsweb` (format version 1): Ed25519-signed manifest with SHA-256 per file,
  rejection of unlisted files, path traversal protection, extraction through a staging folder.
- Publisher binding per project: the first package sets the key, others are rejected.
- Secret detection when packing (file names and content markers).
- Download and verification of official Node.js builds with cache.
- `teleschirm-deploy`: parser for `deploy.conf` (compatible with `deploy.sh`, new:
  `protocol = ftps`), upload via rclone with `seed`/`protect`, password via environment variable,
  version file `.website-manager.json` on the server.
- CLI `website-manager`: `keygen`, `pack`, `verify`, `deploy`, `node`.
- Desktop app (Tauri 2, German/English): open packages, PHP preview on `127.0.0.1` with
  `open_basedir`, disabled process functions and a router for `.htaccess` restrictions, upload
  with dry run and warning about a newer version on the server, log with export.
- Documentation: architecture, formats, changelog, spike report, Rust guideline with rules for
  maintaining documentation and Semantic Versioning; test for consistent version numbers.

[Unreleased]: https://github.com/MrMarteng/website_manager/compare/v0.3.2...HEAD
[0.3.2]: https://github.com/MrMarteng/website_manager/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/MrMarteng/website_manager/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/MrMarteng/website_manager/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MrMarteng/website_manager/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MrMarteng/website_manager/releases/tag/v0.1.0
