# Security policy

The Website Manager handles signed packages, hoster credentials and uploads to live websites, so
security reports are welcome and taken seriously.

## Reporting a vulnerability

Please **do not open a public issue**. Report privately via
[GitHub private vulnerability reporting](https://github.com/MrMarteng/website_manager/security/advisories/new)
("Security" tab → "Report a vulnerability").

Include what you found, how to reproduce it, the affected version and platform, and the impact you
expect. You will get an answer within 7 days. Fixes are released as soon as possible and credited
in the changelog unless you prefer to stay anonymous.

## Scope

In scope, for example:

* bypassing package verification (signature, checksums, publisher binding, path checks)
* code execution when opening, previewing or rebuilding a package beyond what is documented
* leaking credentials (package encryption, keychain, logs, process list)
* uploads that delete or overwrite protected content on the server
* weaknesses in the PHP preview sandbox or the web view (CSP, capabilities)

The security concept is described in [docs/architecture.md](docs/architecture.md#6-security-concept).

## Supported versions

Until 1.0.0 only the latest release receives fixes.
