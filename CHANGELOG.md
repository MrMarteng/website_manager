# Changelog

Alle wesentlichen Änderungen. Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/),
Versionen nach [Semantic Versioning](https://semver.org/lang/de/).

## [Unreleased]

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
