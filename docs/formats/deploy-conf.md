# `deploy.conf`

Compatible with the existing `deploy.sh`; extensions are marked *(new)* and are ignored by
`deploy.sh`.

```text
[server]
protocol = sftp            # (new) sftp | ftps, default sftp
host = example.com
user = web123
port = 22                  # default 22 (sftp) or 21 (ftps)
key_file = keys/id_ed25519 # optional; relative to deploy.conf, ~/ allowed
known_hosts_file = …       # optional
base_dir = ./              # base of the local paths, default: folder of deploy.conf

# <local> -> <server> | <options>
dist/backend/data -> /site/backend/data | mode=seed
dist -> /site | protect=backend/data/*.json,backend/uploads/*/**
```

## Rules

* The server block ends at the first line containing ` -> `; later `key = value` lines are
  ignored. Unknown keys are ignored.
* Options after ` | `, several separated by `;`:
  * `mode=seed` – copy only if the target (apart from `.htaccess`) is still empty.
  * `mode=copy` – only add/update, never delete.
  * without `mode` – the run's mode (default `sync`: the target becomes identical, extra files are
    deleted).
  * `protect=<glob>,<glob>` – neither uploaded nor deleted on the target.
* Always excluded: `node_modules/**`, `.git/**`, `.DS_Store`, `.venv/**`, `.astro/**`, `Thumbs.db`.
* *(new)* The mapping with the shortest target is the website root. The upload writes
  `.website-manager.json` (name, version, `created` of the package) there last; the file is
  protected from `sync`.
* *(new)* Passwords are never in the file; they are passed to rclone via `RCLONE_SFTP_PASS` or
  `RCLONE_FTP_PASS` (obscured).
