//! Website Manager desktop app (Tauri). The web view only calls the commands below;
//! files, processes, dialogs and the browser are handled here in Rust.

mod commands;
mod packages;
mod preview;
mod tools;
mod upload;

use std::sync::{Arc, Mutex};

use tauri::RunEvent;

/// Shared state of the running app.
#[derive(Default)]
pub struct AppState {
    pub preview: Mutex<Option<preview::Preview>>,
    pub log: Arc<Mutex<Vec<String>>>,
}

impl AppState {
    /// Appends a line to the expert log.
    pub fn log(&self, line: impl Into<String>) {
        push_log(&self.log, line);
    }
}

/// Appends a line to a shared log, keeping the most recent lines only.
pub fn push_log(log: &Mutex<Vec<String>>, line: impl Into<String>) {
    const MAX_LINES: usize = 5000;
    if let Ok(mut lines) = log.lock() {
        lines.push(line.into());
        let excess = lines.len().saturating_sub(MAX_LINES);
        lines.drain(..excess);
    }
}

/// Starts the app.
///
/// # Panics
/// Never by design; a failing start is reported on stderr and ends the process.
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::pick_package,
            commands::open_package,
            commands::start_preview,
            commands::stop_preview,
            commands::preview_status,
            commands::upload,
            commands::read_log,
            commands::export_log,
        ])
        .build(tauri::generate_context!());

    match app {
        Ok(app) => app.run(|handle, event| {
            if let RunEvent::Exit = event {
                commands::shutdown(handle);
            }
        }),
        Err(error) => eprintln!("Website Manager could not start: {error}"),
    }
}
