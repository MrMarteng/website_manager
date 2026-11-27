# Phase 0 – feasibility (spike)

As of 2026-11-27 · tested on macOS 26 (Apple Silicon) with a real customer website (Astro 5.18,
tscms backend, contact form). Windows and Linux were **not tested yet**.

This is a dated report and is not kept up to date.

## Result

**Tauri with Rust is confirmed.** The core workflow – create, verify, open, preview and upload a
package – runs without Node.js. Building with a downloaded Node.js works as well. Open are the
tests on Windows and Linux, shipping PHP and rclone, and the blockers for phase 1b listed below.

| Check | Result |
|---|---|
| `pack` (870 files including sources) | 1.2 s, package 5.3 MB |
| signature, checksums, publisher binding | modified file, other publisher, path traversal and changed manifest are rejected |
| sandboxed PHP preview | pages and tscms login work; `shell_exec`/`proc_open` disabled, files outside the package not readable, `127.0.0.1` only |
| `.htaccess` restrictions in the preview | emulated by an own router (see finding 1) |
| upload (test folder instead of server) | `seed` skips filled targets, `protect` keeps live data, stale files are removed, the version file is kept |
| download and verify Node.js 22.12.0 | 10 s, 201 MB unpacked in the cache, second call immediate |
| `npm ci --ignore-scripts` + `astro build` | works (1 s and 3 s with a warm npm cache) |
| app size (macOS, release) | `.app` 11.5 MB, DMG 3.6 MB – without PHP and rclone |
| memory (release) | main process about 100 MB, plus the system's WebKit processes |

Tests: 36 unit and integration tests (`cargo test --workspace`), including a test with the real
rclone against a local folder. `clippy` (pedantic) without findings.

## Findings

1. **The PHP test server ignores `.htaccess`.** Without countermeasure, `backend/users/*.json`
   (password hashes) and `backend/config.php` were reachable in the preview. Fixed: the router
   `apps/desktop/src-tauri/src/router.php` evaluates `Require all denied|granted` (also inside
   `<FilesMatch>`/`<IfModule>`) and blocks hidden files. It is an approximation for the local
   preview, not a replacement for the server's rules.
2. **Blocker 1b: private package source.** `package-lock.json` loads `@teleschirm/astro-cms` from
   `git.nas.local`; `npm ci` fails at the customer. Solution in phase 1: `pack` stores private
   packages as tarballs under `project/vendor/` and rewrites the lockfile to `file:` references.
3. **Blocker 1b on Windows: environment variables in npm scripts.**
   `"build": "ASTRO_TELEMETRY_DISABLED=1 astro build"` does not run in `cmd.exe`. Solution: the
   tool sets `ASTRO_TELEMETRY_DISABLED` itself and calls `astro build` directly; the templates
   should drop the prefix.
4. **The server's host key.** With a password and without `known_hosts_file`, rclone does not
   verify the SSH host key (man-in-the-middle attack possible). Solution in phase 1: `pack`
   records the server's host key in the package, and the tool writes its own `known_hosts` file
   from it.
5. **Composer dependencies belong in the package.** Without `backend/vendor`, Astro builds but
   warns that the backend does not work. `dist/` contains `vendor/`; for `project/`, `include`
   has to add it (E2).
6. **Contact form in the preview:** `challenge.php` answers with 500 because `config.php` (SMTP
   access) is deliberately not in the package. The UI should explain this as expected.
7. **PHP requirements of tscms:** PHP ≥ 8.2 with `dom`, `fileinfo`, `json`, `mbstring`,
   `session`; probably `gd` for images. The shipped PHP (static-php-cli) needs exactly these
   extensions.
8. **The Node.js cache grows:** 201 MB per version. Phase 1 needs a cleanup rule (for example,
   keep only versions of opened projects).
9. **Signature of the Node.js checksums:** `SHASUMS256.txt` is loaded over HTTPS but not verified
   with GPG. For phase 1, verify the signature (`SHASUMS256.txt.sig`) with the Node.js release
   keys.

## Not tested

* **Windows and Linux** (WebView2, WebKitGTK, paths, firewall, SmartScreen) – next step on the
  Windows computer.
* **Real server upload** via SFTP with password – a test webspace is still missing.
* **Detaching of Astro ≥ 7.4 in development mode** – the sample website uses Astro 5.
* **Shipped programs:** PHP and rclone still come from `PATH` or `WM_PHP`/`WM_RCLONE`.
* **Look of the UI:** the app starts without errors; appearance and handling have not been
  reviewed by hand yet.
* **Encrypted credentials (E1)** – not part of the spike.
* Node.js permission model (`--permission`) – not tried.

## Next steps

1. Windows test: `cargo build`, start the app, open a package, preview, upload into a test folder.
2. Choose a test webspace, real upload with password and host key.
3. Build PHP (static-php-cli) and rclone as sidecars and ship them.
4. Fix blockers 2–4, then phase 1a.
