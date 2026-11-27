# Phase 0 – Machbarkeit (Spike)

Stand: 27.11.2026 · getestet auf macOS 26 (Apple Silicon) mit einer echten Kunden-Website
(Astro 5.18, tscms-Backend, Kontaktformular). Windows und Linux sind **noch nicht getestet**.

## Ergebnis

**Tauri mit Rust ist bestätigt.** Der Kernablauf – Paket erstellen, prüfen, öffnen, ansehen,
hochladen – läuft ohne Node.js. Bauen mit heruntergeladenem Node funktioniert ebenfalls. Offen
sind die Tests unter Windows und Linux, das Mitliefern von PHP und rclone sowie die unten
genannten Blocker für Phase 1b.

| Prüfpunkt | Ergebnis |
|---|---|
| `pack` (870 Dateien inkl. Quellen) | 1,2 s, Paket 5,3 MB |
| Signatur, Prüfsummen, Absender-Bindung | manipulierte Datei, fremder Absender, Pfad-Ausbruch und geändertes Manifest werden abgelehnt |
| PHP-Vorschau abgeschottet | Seiten und tscms-Anmeldung laufen; `shell_exec`/`proc_open` gesperrt, Dateien außerhalb des Pakets nicht lesbar, nur `127.0.0.1` |
| `.htaccess`-Sperren in der Vorschau | durch eigenen Router nachgebildet (siehe Befund 1) |
| Upload (Testordner statt Server) | `seed` überspringt befüllte Ziele, `protect` erhält Live-Daten, veraltete Dateien werden entfernt, Versionsdatei bleibt erhalten |
| Node.js 22.12.0 laden + prüfen | 10 s, 201 MB entpackt im Cache, zweiter Aufruf sofort |
| `npm ci --ignore-scripts` + `astro build` | funktioniert (1 s bzw. 3 s mit warmem npm-Cache) |
| App-Größe (macOS, release) | `.app` 11,5 MB, DMG 3,6 MB – ohne PHP und rclone |
| Arbeitsspeicher (release) | Hauptprozess ca. 100 MB, dazu die WebKit-Prozesse des Systems |

Tests: 36 Unit- und Integrationstests (`cargo test --workspace`), darunter ein Test mit echtem
rclone gegen einen lokalen Ordner. `clippy` (pedantic) ohne Befund.

## Befunde

1. **PHP-Testserver ignoriert `.htaccess`.** Ohne Gegenmaßnahme waren in der Vorschau
   `backend/users/*.json` (Passwort-Hashes) und `backend/config.php` abrufbar. Behoben: Router
   `apps/desktop/src-tauri/src/router.php` wertet `Require all denied|granted` (auch in
   `<FilesMatch>`/`<IfModule>`) aus und sperrt versteckte Dateien. Er ist eine Annäherung für die
   lokale Vorschau, kein Ersatz für die Server-Regeln.
2. **Blocker 1b: private Paketquelle.** `package-lock.json` lädt `@teleschirm/astro-cms` von
   `git.nas.local`; beim Kunden scheitert `npm ci`. Lösung in Phase 1: `pack` legt private Pakete
   als Tarball unter `project/vendor/` ab und schreibt das Lockfile auf `file:`-Verweise um.
3. **Blocker 1b unter Windows: Umgebungsvariablen in npm-Skripten.**
   `"build": "ASTRO_TELEMETRY_DISABLED=1 astro build"` läuft unter `cmd.exe` nicht. Lösung: Das
   Tool setzt `ASTRO_TELEMETRY_DISABLED` selbst und ruft `astro build` direkt auf; die Vorlagen
   sollten das Präfix entfernen.
4. **Host-Schlüssel des Servers.** Mit Passwort und ohne `known_hosts_file` prüft rclone den
   SSH-Host-Schlüssel nicht (Angriff per Man-in-the-Middle möglich). Lösung in Phase 1: `pack`
   nimmt den Host-Schlüssel des Servers ins Paket auf, das Tool schreibt daraus eine eigene
   `known_hosts`-Datei.
5. **Composer-Abhängigkeiten gehören ins Paket.** Ohne `backend/vendor` baut Astro, warnt aber,
   dass das Backend nicht läuft. In `dist/` ist `vendor/` enthalten; für `project/` muss
   `include` es aufnehmen (E2).
6. **Kontaktformular in der Vorschau:** `challenge.php` antwortet mit 500, weil `config.php`
   (SMTP-Zugang) bewusst nicht im Paket ist. Die Oberfläche sollte das als erwartbar erklären.
7. **PHP-Anforderungen von tscms:** PHP ≥ 8.2 mit `dom`, `fileinfo`, `json`, `mbstring`,
   `session`; für Bilder voraussichtlich `gd`. Das mitgelieferte PHP (static-php-cli) braucht
   genau diese Erweiterungen.
8. **Node-Cache wächst:** 201 MB je Version. Phase 1 braucht eine Aufräumregel (z. B. nur
   Versionen geöffneter Projekte behalten).
9. **Signatur der Node-Prüfsummen:** `SHASUMS256.txt` wird über HTTPS geladen, aber nicht per GPG
   geprüft. Für Phase 1 die Signatur (`SHASUMS256.txt.sig`) mit den Release-Schlüsseln von Node.js prüfen.

## Nicht geprüft

* **Windows und Linux** (WebView2, WebKitGTK, Pfade, Firewall, SmartScreen) – nächster Schritt
  auf dem Windows-Rechner.
* **Echter Server-Upload** per SFTP mit Passwort – es fehlt noch ein Test-Webspace.
* **Abkoppeln von Astro ≥ 7.4 im Dev-Modus** – die Beispiel-Website nutzt Astro 5.
* **Mitgelieferte Programme:** PHP und rclone kommen noch aus `PATH` bzw. `WM_PHP`/`WM_RCLONE`.
* **Optik der Oberfläche:** Die App startet fehlerfrei; Darstellung und Bedienung sind noch nicht
  von Hand durchgesehen.
* **Verschlüsselte Zugangsdaten (E1)** – nicht Teil des Spikes.
* Node-Berechtigungsmodell (`--permission`) – nicht versucht.

## Nächste Schritte

1. Windows-Test: `cargo build`, App starten, Paket öffnen, Vorschau, Upload in Testordner.
2. Test-Webspace festlegen, echter Upload mit Passwort und Host-Schlüssel.
3. PHP (static-php-cli) und rclone als Sidecars bauen und mitliefern.
4. Blocker 2–4 beheben, dann Phase 1a.
