# Paketformat `.tsweb` (Version 1)

Ein Paket ist ein ZIP-Archiv:

| Pfad | Pflicht | Inhalt |
|---|---|---|
| `manifest.json` | ja | Beschreibung und Prüfsummen (unten) |
| `manifest.sig` | ja | Zeile 1: öffentlicher Ed25519-Schlüssel (Base64), Zeile 2: Signatur der exakten Bytes von `manifest.json` (Base64) |
| `dist/…` | ja | fertig gebaute Website; wird angezeigt und hochgeladen |
| `project/…` | nein | Quellen zum Neubauen und für den Dev-Modus; `project/vendor/*.tgz` enthält Pakete aus privaten npm-Registries |
| `deploy.conf` | nein | Upload-Regeln ([deploy-conf.md](deploy-conf.md)), ohne Pfade des Absender-Rechners |
| `known_hosts` | nein | SSH-Host-Schlüssel des Servers im OpenSSH-Format; die `deploy.conf` verweist mit `known_hosts_file = known_hosts` darauf |
| `credentials.enc` | nein | verschlüsselte Zugangsdaten (unten); nur bei der Erstauslieferung |
| `compliance/…` | nein | Compliance-Bericht |

## Prüfregeln beim Öffnen

1. Signatur über `manifest.json` muss gültig sein.
2. Jeder Eintrag außer Manifest und Signatur muss im Manifest stehen, und jeder Manifest-Eintrag
   muss vorhanden sein; die SHA-256 jedes Eintrags muss stimmen.
3. Pfade sind relativ, mit `/` getrennt, ohne `.`/`..`, Laufwerksbuchstaben, `\` oder
   Steuerzeichen; symbolische Links sind verboten.
4. **Absender-Bindung:** Das erste Paket eines Projekts (`name`) legt den Schlüssel fest. Pakete
   mit anderem Schlüssel für denselben Namen werden abgelehnt.
5. Entpackt wird in einen Zwischenordner, der erst nach bestandener Prüfung umbenannt wird.

## `manifest.json`

```json
{
  "format_version": 1,
  "name": "cafe-glueckseelig",
  "version": "2026.11.27",
  "created": "2026-11-27T08:00:00Z",
  "node_version": "22.12.0",
  "build_output": "dist",
  "changelog": ["Öffnungszeiten ergänzt"],
  "logo": "project/public/logo.svg",
  "has_credentials": false,
  "files": { "dist/index.html": "<sha256>" }
}
```

`name` besteht aus `a-z`, `A-Z`, `0-9`, `-`, `_`. `created` ist UTC im Format RFC 3339 und dient
dem Vergleich mit der Versionsdatei auf dem Server.

## `credentials.enc`

```json
{
  "kdf": "argon2id",
  "params": { "memory_kib": 65536, "iterations": 3, "parallelism": 1 },
  "salt": "<16 Byte, Base64>",
  "nonce": "<12 Byte, Base64>",
  "ciphertext": "<AES-256-GCM, Base64>"
}
```

Klartext ist JSON `{"password": "…", "privateKey": "…"}` (beide optional). Das Paketpasswort hat
mindestens 8 Zeichen. Der Export aus der App (`<name>.wmzugang`) hat dasselbe Format.

## `website-manager.config.json` (im Website-Projekt)

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
  "changelog": ["Öffnungszeiten ergänzt"],
  "compliance": "COMPLIANCE_SCAN.json",
  "logo": "public/logo.svg"
}
```

* `include` ist eine **Positivliste**: Nur passende Quelldateien kommen unter `project/` ins Paket.
  `node_modules`, `.git`, `.astro`, `.venv` werden nie aufgenommen.
* `pack` bricht ab, wenn eine Datei nach Geheimnis aussieht (`.env`, Schlüsseldateien, `.npmrc`,
  `auth.json`, private Schlüssel oder Tokens im Inhalt).
* Aus der `deploy.conf` werden `key_file` und `known_hosts_file` entfernt. Bei SFTP liest `pack`
  die Host-Schlüssel des Servers mit `ssh-keyscan` und legt sie als `known_hosts` bei
  (abschaltbar mit `--no-host-key`).
* Stammt ein Paket im `package-lock.json` nicht von `registry.npmjs.org`, holt `pack` es mit
  `npm pack` (mit der npm-Konfiguration der Agentur) nach `project/vendor/` und setzt `resolved` auf
  `file:vendor/<datei>.tgz`; `integrity` bleibt.
* Für tscms-Projekte gehört `backend/**` einschließlich `backend/vendor/` in `include`, sonst
  läuft das Backend nach dem Neubauen nicht.
* `build_output` (aus `dist` der Konfiguration) nennt den Ausgabeordner relativ zu `project/`;
  `node_version` sollte eine genaue Version sein (`22.12.0`).
* `--credentials <datei.json>` legt Zugangsdaten verschlüsselt bei; das Paketpasswort kommt aus
  `WM_PACKAGE_PASSWORD` oder wird abgefragt.
