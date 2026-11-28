//! Website Manager desktop app (Tauri). The web view only calls the commands in
//! `commands.rs`; files, processes, dialogs, the keychain and the browser are handled in Rust.

mod commands;
mod deployment;
mod error;
mod files;
mod keychain;
mod overview;
mod preview;
mod site;
mod tools;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, DragDropEvent, Emitter, Manager, RunEvent, WindowEvent};

/// Event sent to the window when a package should be opened (double-click, drag and drop).
pub const OPEN_EVENT: &str = "package-open-requested";
/// Event with one line of upload output.
pub const UPLOAD_EVENT: &str = "upload-progress";
/// Event with one line of build or development server output.
pub const BUILD_EVENT: &str = "build-progress";
/// Event with the running preview and development server ([`commands::ServerStatus`]).
pub const STATUS_EVENT: &str = "server-status";

/// Shared state of the running app.
#[derive(Default)]
pub struct AppState {
    pub preview: Mutex<Option<preview::Preview>>,
    pub dev: Mutex<Option<site::DevServer>>,
    pub log: Arc<Mutex<Vec<String>>>,
    /// Packages handed over before the window asked for them.
    pub pending_open: Mutex<Vec<PathBuf>>,
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

fn is_package(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("tsweb"))
}

/// Queues packages to open and tells the window.
fn request_open(app: &AppHandle, paths: impl IntoIterator<Item = PathBuf>) {
    let packages: Vec<PathBuf> = paths.into_iter().filter(|path| is_package(path)).collect();
    if packages.is_empty() {
        return;
    }
    if let Some(state) = app.try_state::<AppState>()
        && let Ok(mut pending) = state.pending_open.lock()
    {
        pending.extend(packages);
    }
    let _ = app.emit(OPEN_EVENT, ());
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
    }
}

/// Starts the app.
pub fn run() {
    let app = tauri::Builder::default()
        // A second start (double-click on a package) hands its arguments to this instance.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            request_open(app, argv.into_iter().skip(1).map(PathBuf::from));
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            request_open(app.handle(), std::env::args_os().skip(1).map(PathBuf::from));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event {
                request_open(window.app_handle(), paths.clone());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::pick_package,
            commands::open_package,
            commands::take_pending_open,
            commands::project_overview,
            commands::import_package_credentials,
            commands::forget_credentials,
            commands::export_credentials,
            commands::import_credentials_file,
            commands::save_settings,
            commands::start_preview,
            commands::stop_preview,
            commands::preview_status,
            commands::server_status,
            commands::upload,
            commands::build_site,
            commands::start_dev,
            commands::stop_dev,
            commands::dev_status,
            commands::restore_version,
            commands::remove_version,
            commands::list_files,
            commands::open_file,
            commands::reveal_project,
            commands::open_compliance,
            commands::open_website,
            commands::read_log,
            commands::export_log,
        ])
        .build(tauri::generate_context!());

    match app {
        Ok(app) => app.run(|handle, event| match event {
            RunEvent::Exit => commands::shutdown(handle),
            #[cfg(target_os = "macos")]
            RunEvent::Opened { urls } => {
                request_open(
                    handle,
                    urls.into_iter().filter_map(|url| url.to_file_path().ok()),
                );
            }
            _ => {}
        }),
        Err(error) => eprintln!("Website Manager could not start: {error}"),
    }
}
