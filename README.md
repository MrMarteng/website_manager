# Website Manager

Desktop-Programm, mit dem Website-Inhaber ein von ihrer Webagentur geliefertes Website-Paket
öffnen, lokal ansehen und auf ihren eigenen Webspace hochladen – ohne technische Kenntnisse.

*A desktop app that lets website owners open a website package delivered by their web agency,
preview it locally and upload it to their own webspace – no technical knowledge needed.*

> Stand: Phase 0 (Machbarkeit). Ergebnisse: [docs/spike/phase-0.md](docs/spike/phase-0.md).
> Architektur: [docs/architektur.md](docs/architektur.md).

## So funktioniert es

1. Die Agentur erzeugt mit `website-manager pack` ein signiertes Paket (`.tsweb`).
2. Der Kunde öffnet es im Website Manager. Signatur und Prüfsummen werden geprüft; das erste Paket
   bindet den Absender an die Website, spätere Pakete anderer Absender werden abgelehnt.
3. **Ansehen:** Die fertige Website läuft lokal (PHP auf `127.0.0.1`, abgeschottet) im Browser.
4. **Hochladen:** nach den Regeln der mitgelieferten `deploy.conf` per SFTP/FTPS (rclone);
   Inhalte aus dem Redaktionssystem auf dem Server bleiben unangetastet.

## Aufbau

| Ordner | Inhalt |
|---|---|
| `crates/core` | Paketformat, Signatur (Ed25519), Prüfsummen, Absender-Bindung, Geheimnis-Prüfung, Node-Laufzeit |
| `crates/deploy` | `teleschirm-deploy`: `deploy.conf` lesen, Upload-Plan, Ausführung mit rclone |
| `crates/cli` | Programm `website-manager`: `keygen`, `pack`, `verify`, `deploy`, `node` |
| `apps/desktop` | Tauri-App (Rust + Oberfläche in HTML/JS, Deutsch/Englisch) |
| `docs/formate` | Spezifikation von `.tsweb`, Manifest, `website-manager.config.json`, `deploy.conf` |

## Entwickeln

Voraussetzungen: Rust (stable), Node.js (nur für die Tauri-CLI), rclone und PHP ≥ 8.2 im `PATH`
(bis PHP und rclone mit der App ausgeliefert werden; alternativ `WM_PHP` und `WM_RCLONE`).

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd apps/desktop && npm install && npm run dev
```

## Paket erstellen (Agentur)

```bash
website-manager keygen --out ~/.config/website-manager/keys     # einmalig, geheimen Schlüssel sichern
website-manager pack --project <website> --key ~/.config/website-manager/keys/signing.key
website-manager verify <paket>.tsweb
```

Das Projekt braucht eine `website-manager.config.json`, siehe [docs/formate/paket.md](docs/formate/paket.md).

## Lizenz

Apache License 2.0, siehe [LICENSE](LICENSE). Name und Logo sind nicht lizenziert, siehe
[TRADEMARKS.md](TRADEMARKS.md). Beiträge: [CONTRIBUTING.md](CONTRIBUTING.md).
