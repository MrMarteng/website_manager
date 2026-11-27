# Mitwirken

## Arbeitsweise

* Änderungen auf einem Branch, Commit-Nachrichten nach [Conventional Commits](https://www.conventionalcommits.org/).
* Neues Verhalten braucht einen Test. Formate (`docs/formate/`) und Code ändern sich im selben Commit.
* Vor dem Push: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`.

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
