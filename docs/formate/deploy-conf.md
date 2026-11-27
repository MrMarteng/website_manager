# `deploy.conf`

Kompatibel mit dem bisherigen `deploy.sh`; Erweiterungen sind mit *(neu)* markiert und werden von
`deploy.sh` ignoriert.

```text
[server]
protocol = sftp            # (neu) sftp | ftps, Standard sftp
host = example.com
user = web123
port = 22                  # Standard 22 (sftp) bzw. 21 (ftps)
key_file = keys/id_ed25519 # optional; relativ zur deploy.conf, ~/ erlaubt
known_hosts_file = …       # optional
base_dir = ./              # Bezug der lokalen Pfade, Standard: Ordner der deploy.conf

# <lokal> -> <Server> | <Optionen>
dist/backend/data -> /site/backend/data | mode=seed
dist -> /site | protect=backend/data/*.json,backend/uploads/*/**
```

## Regeln

* Der Server-Block endet mit der ersten Zeile, die ` -> ` enthält; spätere `key = value`-Zeilen
  werden ignoriert. Unbekannte Schlüssel werden ignoriert.
* Optionen nach ` | `, mehrere mit `;` getrennt:
  * `mode=seed` – nur kopieren, wenn das Ziel (außer `.htaccess`) noch leer ist.
  * `mode=copy` – nur hinzufügen/aktualisieren, nie löschen.
  * ohne `mode` – Modus des Laufs (Standard `sync`: Ziel wird identisch, überzählige Dateien werden gelöscht).
  * `protect=<glob>,<glob>` – weder hochladen noch am Ziel löschen.
* Immer ausgeschlossen: `node_modules/**`, `.git/**`, `.DS_Store`, `.venv/**`, `.astro/**`, `Thumbs.db`.
* *(neu)* Die Zuordnung mit dem kürzesten Ziel ist die Website-Wurzel. Dort schreibt der Upload
  zuletzt `.website-manager.json` (Name, Version, `created` des Pakets); die Datei ist vor `sync`
  geschützt.
* *(neu)* Passwörter stehen nie in der Datei; sie werden rclone über `RCLONE_SFTP_PASS` bzw.
  `RCLONE_FTP_PASS` (verschleiert) übergeben.
