# Paketformat `.tsweb` (Version 1)

Ein Paket ist ein ZIP-Archiv:

| Pfad | Pflicht | Inhalt |
|---|---|---|
| `manifest.json` | ja | Beschreibung und Prüfsummen (unten) |
| `manifest.sig` | ja | Zeile 1: öffentlicher Ed25519-Schlüssel (Base64), Zeile 2: Signatur der exakten Bytes von `manifest.json` (Base64) |
| `dist/…` | ja | fertig gebaute Website; wird angezeigt und hochgeladen |
| `project/…` | nein | Quellen zum Neubauen und für den Dev-Modus |
| `deploy.conf` | nein | Upload-Regeln ([deploy-conf.md](deploy-conf.md)), ohne Pfade des Absender-Rechners |
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
  "changelog": ["Öffnungszeiten ergänzt"],
  "logo": "project/public/logo.svg",
  "has_credentials": false,
  "files": { "dist/index.html": "<sha256>" }
}
```

`name` besteht aus `a-z`, `A-Z`, `0-9`, `-`, `_`. `created` ist UTC im Format RFC 3339 und dient
dem Vergleich mit der Versionsdatei auf dem Server.

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
* Aus der `deploy.conf` werden `key_file` und `known_hosts_file` entfernt.
