# Test unter Windows

Für Windows 10 und 11 (x64). Zwei Wege zum Programm: den fertigen Installer aus GitHub Actions
laden (empfohlen) oder selbst bauen.

## 1. Programm besorgen

### a) Installer aus GitHub Actions

1. Auf GitHub: **Actions → Build → Run workflow** (oder ein Tag `v*` pushen).
2. Nach dem Lauf im Abschnitt **Artifacts** `website-manager-Windows-<Nummer>` laden und entpacken.
   Enthalten sind der Installer (`*.msi` oder `*-setup.exe`) und das Kommandozeilenprogramm
   `website-manager.exe`.
3. Installer starten. Die Installer sind noch nicht signiert; SmartScreen meldet „Der Computer wurde
   durch Windows geschützt“ → **Weitere Informationen → Trotzdem ausführen**.

### b) Selbst bauen

Einmalig installieren:

* [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/de/visual-cpp-build-tools/),
  Arbeitsauslastung „Desktopentwicklung mit C++“
* Rust über [rustup](https://rustup.rs) (`rustup-init.exe`, Standardeinstellungen)
* [Node.js LTS](https://nodejs.org) (nur für die Tauri-CLI) und [Git](https://git-scm.com)
* WebView2 ist in Windows 10/11 enthalten.

Dann in PowerShell:

```powershell
git clone <Repository-URL> website-manager
cd website-manager\apps\desktop
npm ci
npm run dev      # Programm im Entwicklungsmodus starten
npm run build    # Installer bauen → ..\..\target\release\bundle\msi und \nsis
```

Mit `npm run dev` ist die Dateiendung `.tsweb` nicht registriert; Doppelklick auf ein Paket nur mit
installiertem Build testen.

## 2. PHP und rclone bereitstellen

Die App liefert beide noch nicht mit.

1. **PHP 8.2 oder neuer:** ZIP „VS17 x64 Non Thread Safe“ von
   [windows.php.net](https://windows.php.net/download/) nach `C:\tools\php` entpacken,
   `php.ini-production` nach `php.ini` kopieren und darin `extension_dir = "ext"`,
   `extension=mbstring`, `extension=fileinfo` und `extension=gd` aktivieren (Semikolon am
   Zeilenanfang entfernen; `dom` und `json` sind eingebaut).
2. **rclone:** ZIP „Windows amd64“ von [rclone.org](https://rclone.org/downloads/) nach
   `C:\tools\rclone` entpacken.
3. Benutzer-Umgebungsvariablen setzen (Einstellungen → System → Info → Erweiterte
   Systemeinstellungen → Umgebungsvariablen) und das Programm danach neu starten:

   | Variable | Wert |
   |---|---|
   | `WM_PHP` | `C:\tools\php\php.exe` |
   | `WM_RCLONE` | `C:\tools\rclone\rclone.exe` |

Prüfen in einer neuen PowerShell: `& $env:WM_PHP -v` und `& $env:WM_RCLONE version`.

## 3. Testpaket

Pakete werden auf dem Rechner der Agentur gebaut (`website-manager pack`, siehe README) und per USB-Stick,
Netzlaufwerk oder Mail auf den Windows-Rechner gebracht. Für den Test eignen sich eine
Erstauslieferung mit Zugangsdaten und ein Update desselben Projekts.

## 4. Prüfliste

| # | Prüfung | Erwartet |
|---|---|---|
| 1 | Programm starten | Fenster erscheint, Sprache folgt Windows (Deutsch/Englisch) |
| 2 | Paket per Knopf öffnen | Übersicht mit Version, Absender, Änderungen |
| 3 | Paket per Ziehen ins Fenster und per Doppelklick (installierter Build) | wird geöffnet; bei laufendem Programm im selben Fenster |
| 4 | Fremdes Paket (anderer Schlüssel, gleicher Name) | wird abgelehnt |
| 5 | Zugang übernehmen (Paketpasswort) | in **Anmeldeinformationsverwaltung → Windows-Anmeldeinformationen** steht `com.teleschirm.website-manager` |
| 6 | Im Browser ansehen | Website im Standardbrowser, **keine** Firewall-Abfrage; Redaktionssystem-Anmeldung lädt |
| 7 | Vorschau beenden, Programm schließen | im Task-Manager kein `php.exe` mehr |
| 8 | Änderungen prüfen / hochladen in einen Testordner (z. B. `C:\temp\server`) | Liste neuer und gelöschter Dateien, Dateien im Ordner, `.website-manager.json` vorhanden |
| 9 | Update öffnen | Einstellungen und Zugang bleiben, alte Version unter „Versionen“ |
| 10 | Version wiederherstellen und hochladen | Rückfrage wegen neuerer Version auf dem Ziel |
| 11 | Neu bauen | lädt Node.js beim ersten Mal, meldet „Neu gebaut“; Vorschau zeigt die neue Fassung |
| 12 | Entwicklungsmodus starten und beenden | Seite öffnet sich im Browser; danach kein `node.exe`/`php.exe` im Task-Manager |
| 13 | Benutzername oder Ordner mit Leerzeichen/Umlauten | alles wie oben |
| 14 | Protokoll als Datei speichern | Textdatei mit dem Ablauf |

## 5. Wo die Daten liegen

| Was | Ort |
|---|---|
| Projekte, Versionen, `trust.json` | `%APPDATA%\com.teleschirm.website-manager\` |
| Node.js-Cache | `%LOCALAPPDATA%\com.teleschirm.website-manager\node\` |
| Zugang | Windows-Anmeldeinformationsverwaltung |

Zum Zurücksetzen das Programm schließen, beide Ordner löschen und den Eintrag in der
Anmeldeinformationsverwaltung entfernen.

## 6. Fehler melden

Im Programm **„Für Fachleute: Protokoll“ → „Als Datei speichern …“** und die Datei mit der Nummer
aus der Prüfliste an das Issue hängen. Bei Abstürzen zusätzlich `npm run dev` aus PowerShell
starten und die Ausgabe des Terminals mitschicken.
