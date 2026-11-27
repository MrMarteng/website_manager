# Mitwirken

## Arbeitsweise

* Änderungen auf einem Branch, Commit-Nachrichten nach [Conventional Commits](https://www.conventionalcommits.org/).
* Neues Verhalten braucht einen Test. Dokumentation und Changelog ändern sich im selben Commit
  (Abschnitt unten).
* Vor dem Push: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`. Dieselben Schritte laufen in der CI (`.github/workflows/build.yml`);
  Installer entstehen dort bei Tags `v*` und auf Anforderung.

## Dokumentation, Changelog, Version

**Sprache:** `CHANGELOG.md`, Code-Kommentare (`//`, `///`, `//!`, Kommentare in JS, CSS, PHP, YAML)
und Commit-Nachrichten sind englisch. Texte der Oberfläche stehen in `apps/desktop/ui/i18n.js`
(Deutsch und Englisch); die Doku unter `docs/` ist deutsch.

Gehören zur Änderung und kommen in denselben Commit wie der Code.

* **`docs/architektur.md`** beschreibt nur den Ist-Zustand. Ändert sich ein Baustein, Ablauf,
  Sicherheitsmerkmal, Ablageort oder eine Entscheidung, wird der Abschnitt angepasst. Geplantes
  gehört nicht hinein.
* **`docs/formate/*.md`** sind die Spezifikation; Code und Spezifikation ändern sich gemeinsam.
* **`CHANGELOG.md`** ([Keep a Changelog](https://keepachangelog.com/en/1.1.0/), englisch): jede für Nutzer
  oder Agentur sichtbare Änderung unter `[Unreleased]` in der passenden Rubrik (`Added`,
  `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`). Interne Umbauten ohne Wirkung nach
  außen brauchen keinen Eintrag.
* **`docs/spike/*.md`** sind datierte Berichte und werden nicht nachgeführt.

**Versionierung** nach [Semantic Versioning 2.0.0](https://semver.org/lang/de/):

* Eine Version für `Cargo.toml` (Workspace), `apps/desktop/src-tauri/tauri.conf.json` und
  `apps/desktop/package.json`; der Test `version_sync` prüft die Übereinstimmung.
* `0.y.z` bis zur ersten öffentlichen Version: `y` bei neuen Funktionen oder inkompatiblen
  Änderungen, `z` bei Fehlerbehebungen.
* Ab `1.0.0`: **MAJOR** bei inkompatiblen Änderungen an Paketformat, `deploy.conf`,
  `website-manager.config.json` oder CLI-Optionen, **MINOR** bei neuen abwärtskompatiblen
  Funktionen, **PATCH** bei Fehlerbehebungen.
* `format_version` im Manifest ist unabhängig davon und steigt nur bei inkompatiblen Formatänderungen.
* **Release:** `[Unreleased]` in `## [X.Y.Z] - JJJJ-MM-TT` umbenennen, neuen leeren
  `[Unreleased]`-Abschnitt anlegen, Version an allen drei Stellen anheben, Vergleichslinks am Ende
  des Changelogs ergänzen, nach dem Merge Tag `vX.Y.Z` setzen.

## Rust-Guideline

* **Formatierung:** `rustfmt` mit Standardeinstellungen, keine eigene `rustfmt.toml`.
* **Lints:** `clippy` mit `pedantic` (Workspace-Lints in `Cargo.toml`). In der CI ist jede Warnung
  ein Fehler (`-D warnings`). Ausnahmen nur lokal mit `#[allow(...)]` und kurzer Begründung.
* **Fehler:** Bibliotheken definieren eigene Fehlertypen mit `thiserror`; Fehler werden mit `?`
  weitergereicht, nicht verschluckt. Meldungen nennen Datei, Zeile oder Wert, um den es geht.
* **Kein `unwrap()`/`expect()` außerhalb von Tests** (`clippy::unwrap_used` und
  `clippy::expect_used` stehen auf `deny`; `clippy.toml` erlaubt sie in Tests).
* **Sicherheit:** Pfade aus Paketen nie ungeprüft verwenden (`wm_core::archive::safe_relative_path`);
  Passwörter nie als Programmargument übergeben; externe Programme ohne Shell starten.
* **Benennung:** beschreibend und suchbar, keine Abkürzungen außer etablierten (`url`, `id`).
* **Dokumentation:** öffentliche Funktionen mit `///`, inklusive `# Errors`, wenn sie scheitern können.

## Oberfläche

* Die Web-Ansicht ruft nur die freigegebenen Rust-Befehle auf (`apps/desktop/src-tauri/src/commands.rs`);
  kein Datei-, Prozess- oder Netzwerkzugriff aus JavaScript.
* Alle Texte stehen in `apps/desktop/ui/i18n.js`; jede Sprache hat dieselben Schlüssel.
