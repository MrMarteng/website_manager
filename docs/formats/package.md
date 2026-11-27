# Package format `.tsweb` (version 1)

A package is a ZIP archive:

| Path | Required | Content |
|---|---|---|
| `manifest.json` | yes | description and checksums (below) |
| `manifest.sig` | yes | line 1: Ed25519 public key (Base64), line 2: signature of the exact bytes of `manifest.json` (Base64) |
| `dist/…` | yes | the built website; shown in the preview and uploaded |
| `project/…` | no | sources for rebuilding and development mode; `project/vendor/*.tgz` holds packages from private npm registries |
| `deploy.conf` | no | upload rules ([deploy-conf.md](deploy-conf.md)), without paths of the publisher's computer |
| `known_hosts` | no | the server's SSH host keys in OpenSSH format; `deploy.conf` refers to it with `known_hosts_file = known_hosts` |
| `credentials.enc` | no | encrypted credentials (below); first delivery only |
| `compliance/…` | no | compliance report |

## Verification when opening

1. The signature over `manifest.json` must be valid.
2. Every entry except manifest and signature must be listed in the manifest, and every manifest
   entry must be present; the SHA-256 of every entry must match.
3. Paths are relative, separated by `/`, without `.`/`..`, drive letters, `\` or control
   characters; symbolic links are not allowed.
4. **Publisher binding:** the first package of a project (`name`) sets the key. Packages with
   another key for the same name are rejected.
5. Extraction goes into a staging folder that is renamed only after verification has passed.

## `manifest.json`

```json
{
  "format_version": 1,
  "name": "cafe-glueckseelig",
  "version": "2026.11.27",
  "created": "2026-11-27T08:00:00Z",
  "node_version": "22.12.0",
  "build_output": "dist",
  "changelog": ["Opening hours added"],
  "logo": "project/public/logo.svg",
  "has_credentials": false,
  "files": { "dist/index.html": "<sha256>" }
}
```

`name` consists of `a-z`, `A-Z`, `0-9`, `-`, `_`. `created` is UTC in RFC 3339 format and is
compared with the version file on the server.

## `credentials.enc`

```json
{
  "kdf": "argon2id",
  "params": { "memory_kib": 65536, "iterations": 3, "parallelism": 1 },
  "salt": "<16 bytes, Base64>",
  "nonce": "<12 bytes, Base64>",
  "ciphertext": "<AES-256-GCM, Base64>"
}
```

The plaintext is JSON `{"password": "…", "privateKey": "…"}` (both optional). The package
password has at least 8 characters. The export from the app (`<name>.wmzugang`) uses the same
format.

## `website-manager.config.json` (in the website project)

```json
{
  "name": "cafe-glueckseelig",
  "version": "2026.11.27",
  "build": ["npm", "run", "build"],
  "dist": "dist",
  "deploy_conf": "deploy.conf",
  "node_version": "22.12.0",
  "include": ["src/**", "public/**", "backend/**", "package.json", "package-lock.json", "astro.config.mjs"],
  "exclude": [],
  "changelog": ["Opening hours added"],
  "compliance": "COMPLIANCE_SCAN.json",
  "logo": "public/logo.svg"
}
```

* `include` is a **positive list**: only matching source files go into the package under
  `project/`. `node_modules`, `.git`, `.astro` and `.venv` are never included.
* `pack` aborts if a file looks like a secret (`.env`, key files, `.npmrc`, `auth.json`, private
  keys or tokens in the content).
* `key_file` and `known_hosts_file` are removed from `deploy.conf`. For SFTP, `pack` reads the
  server's host keys with `ssh-keyscan` and adds them as `known_hosts` (can be switched off with
  `--no-host-key`).
* If a package in `package-lock.json` does not come from `registry.npmjs.org`, `pack` fetches it
  with `npm pack` (using the agency's npm configuration) into `project/vendor/` and sets
  `resolved` to `file:vendor/<file>.tgz`; `integrity` is kept.
* For tscms projects, `include` must contain `backend/**` including `backend/vendor/`; otherwise
  the backend does not work after a rebuild.
* `build_output` (from `dist` in the configuration) names the output folder relative to
  `project/`; `node_version` should be an exact version (`22.12.0`).
* `--credentials <file.json>` adds encrypted credentials; the package password comes from
  `WM_PACKAGE_PASSWORD` or is asked for.
