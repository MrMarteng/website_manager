# Contributing

## Workflow

* Discuss larger changes in an issue first. Security problems: see [SECURITY.md](SECURITY.md), not
  an issue. Everyone follows the [code of conduct](CODE_OF_CONDUCT.md).
* Changes go on a branch (in a fork if you are not a maintainer) and reach `main` only through a
  pull request. `main` requires the check "Format, Clippy, Tests"; only maintainers can merge, and
  the code owners (`.github/CODEOWNERS`) are asked for a review. Once the project has several
  maintainers, an approving code owner review becomes mandatory. Pull requests are squash-merged,
  so the **pull request title** must
  follow [Conventional Commits](https://www.conventionalcommits.org/) (it becomes the commit
  message). Workflows of first-time and outside contributors run after a maintainer approves them.
* Release tags `v*` can only be created by maintainers.
* Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).
* New behaviour needs a test. Documentation and changelog change in the same commit (see below).
* Installers bundle PHP and rclone. Locally: `cd apps/desktop && npm run sidecars` (downloads the
  pinned versions for this computer and checks their SHA-256), then `npm run build:release`. To
  update a program, edit `apps/desktop/sidecars.json` and run
  `python3 .github/scripts/sidecars.py lock`.
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
* **Release:**
  1. In a pull request: rename `[Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`, add a new empty
     `[Unreleased]` section, raise the version in all three places, add the comparison links at
     the end of the changelog.
  2. After the merge, a maintainer tags `vX.Y.Z` on `main` and pushes the tag.
  3. The workflow checks tag, versions and changelog, builds all installers and creates a **draft
     pre-release** with the installers, the CLI programs, `SHA256SUMS` and the changelog section
     as text (`.github/scripts/release.py`).
  4. Review the draft on GitHub and publish it.

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
