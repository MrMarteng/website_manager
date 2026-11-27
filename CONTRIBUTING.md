# Contributing

## Workflow

* Changes go on a branch; commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).
* New behaviour needs a test. Documentation and changelog change in the same commit (see below).
* Before pushing: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`. The same steps run in CI (`.github/workflows/build.yml`); installers
  are built there on tags `v*` and on demand.

## Documentation, changelog, version

**Language:** documentation (`README.md`, `CONTRIBUTING.md`, `docs/`), `CHANGELOG.md`, code
comments (`//`, `///`, `//!`, comments in JS, CSS, PHP, YAML) and commit messages are in English.
User interface texts live in `apps/desktop/ui/i18n.js` (German and English).

These belong to the change and go into the same commit as the code:

* **`docs/architecture.md`** describes the current state only. When a building block, flow,
  security property, storage location or decision changes, the section is updated. Plans do not
  belong there.
* **`docs/formats/*.md`** are the specification; code and specification change together.
* **`CHANGELOG.md`** ([Keep a Changelog](https://keepachangelog.com/en/1.1.0/)): every change
  visible to users or agencies goes under `[Unreleased]` in the matching section (`Added`,
  `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`). Internal refactorings without external
  effect need no entry.
* **`docs/spike/*.md`** are dated reports and are not kept up to date.

**Versioning** follows [Semantic Versioning 2.0.0](https://semver.org/):

* One version for `Cargo.toml` (workspace), `apps/desktop/src-tauri/tauri.conf.json` and
  `apps/desktop/package.json`; the test `version_sync` checks that they match.
* `0.y.z` until the first public release: `y` for new features or incompatible changes, `z` for
  fixes.
* From `1.0.0`: **MAJOR** for incompatible changes to the package format, `deploy.conf`,
  `website-manager.config.json` or CLI options, **MINOR** for new backwards-compatible features,
  **PATCH** for fixes.
* `format_version` in the manifest is independent and only increases for incompatible format
  changes.
* **Release:** rename `[Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`, add a new empty `[Unreleased]`
  section, raise the version in all three places, add the comparison links at the end of the
  changelog, and tag `vX.Y.Z` after the merge.

## Rust guideline

* **Formatting:** `rustfmt` with default settings, no own `rustfmt.toml`.
* **Lints:** `clippy` with `pedantic` (workspace lints in `Cargo.toml`). In CI every warning is an
  error (`-D warnings`). Exceptions only locally with `#[allow(...)]` and a short reason.
* **Errors:** libraries define their own error types with `thiserror`; errors are passed on with
  `?`, not swallowed. Messages name the file, line or value concerned.
* **No `unwrap()`/`expect()` outside tests** (`clippy::unwrap_used` and `clippy::expect_used` are
  set to `deny`; `clippy.toml` allows them in tests).
* **Security:** never use paths from packages unchecked (`wm_core::archive::safe_relative_path`);
  never pass passwords as program arguments; start external programs without a shell.
* **Naming:** descriptive and searchable, no abbreviations except established ones (`url`, `id`).
* **Documentation:** public functions with `///`, including `# Errors` if they can fail.

## User interface

* The web view only calls the permitted Rust commands (`apps/desktop/src-tauri/src/commands.rs`);
  no file, process or network access from JavaScript.
* All texts live in `apps/desktop/ui/i18n.js`; every language has the same keys.
