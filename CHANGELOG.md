# Changelog

Alle wesentlichen Änderungen. Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/),
Versionen nach [Semantic Versioning](https://semver.org/lang/de/).

## [Unreleased]

### Added

- Projekte mit Versionen: Ein Update ersetzt die aktuelle Version, die letzten fünf bleiben
  erhalten, ältere wandern in den Papierkorb; eine frühere Version lässt sich wiederherstellen und
  erneut hochladen. Vom Kunden geänderte Dateien der Vorversion werden beim Update gemeldet.
- Zugangsdaten: `pack --credentials` legt sie verschlüsselt bei (Argon2id, AES-256-GCM,
  Paketpasswort); die App übernimmt sie in den Schlüsselbund, exportiert und importiert sie mit
  eigenem Übertragungspasswort und kann sie entfernen. Anmeldung per Passwort oder SSH-Schlüssel.
- Server-Identität: `pack` nimmt die SSH-Host-Schlüssel des Servers ins Paket auf; ohne passenden
  Schlüssel lädt die App nur nach Bestätigung hoch.
- Upload: Fortschritt live, Liste der neuen und gelöschten Dateien, „Jetzt hochladen“ prüft erst
  und fragt dann mit der Zahl der Änderungen nach; Website nach dem Upload im Browser öffnen.
- Einstellungen je Projekt (Verbindung, Server, Benutzer, Port, Zielordner, Adresse der Website),
  im Expertenmodus eigene Zuordnungen und Schutzregeln; Einstellungen bleiben bei Updates erhalten.
- Pakete öffnen per Doppelklick, zweitem Programmstart und Ziehen ins Fenster; Startseite mit den
  bekannten Websites.
- Dateiliste mit Filter, Öffnen im passenden Programm, Ordner anzeigen; Prüfbericht öffnen.
- `teleschirm-deploy`: Liste der hochgeladenen und gelöschten Dateien je Lauf, Wechsel des
  Zielordners (`with_remote_root`).

## [0.1.0] - 2026-11-27

Machbarkeitsstand (Phase 0), getestet nur unter macOS.

### Added

- Paketformat `.tsweb` (Formatversion 1): Ed25519-signiertes Manifest mit SHA-256 je Datei,
  Ablehnung nicht gelisteter Dateien, Schutz vor Pfad-Ausbruch, Entpacken über Zwischenordner.
- Absender-Bindung je Projekt: Das erste Paket legt den Schlüssel fest, andere werden abgelehnt.
- Geheimnis-Prüfung beim Packen (Dateinamen und Inhaltsmerkmale).
- Download und Prüfung offizieller Node.js-Builds mit Cache.
- `teleschirm-deploy`: Parser für `deploy.conf` (kompatibel zu `deploy.sh`, neu: `protocol = ftps`),
  Upload über rclone mit `seed`/`protect`, Passwort per Umgebungsvariable, Versionsdatei
  `.website-manager.json` auf dem Server.
- CLI `website-manager`: `keygen`, `pack`, `verify`, `deploy`, `node`.
- Desktop-App (Tauri 2, Deutsch/Englisch): Paket öffnen, PHP-Vorschau auf `127.0.0.1` mit
  `open_basedir`, gesperrten Prozessfunktionen und Router für `.htaccess`-Sperren, Upload mit
  Prüflauf und Warnung bei neuerer Version auf dem Server, Protokoll mit Export.
- Dokumentation: Architektur, Formate, Changelog, Spike-Bericht, Rust-Guideline mit Regeln für
  Doku-Pflege und Semantic Versioning; Test für einheitliche Versionsnummern.

[Unreleased]: https://git.nas.local/teleschirm/website-manager/compare/v0.1.0...HEAD
[0.1.0]: https://git.nas.local/teleschirm/website-manager/src/tag/v0.1.0
