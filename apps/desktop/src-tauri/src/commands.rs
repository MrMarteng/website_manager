//! Commands the window may call. Tauri hands over arguments by value.
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::packages::{self, PackageSummary};
use crate::upload::{self, UploadResult, UploadTarget};
use crate::{AppState, preview};

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

fn projects_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = data_dir(app)?.join("projects");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Shows the file dialog and opens the chosen package.
#[tauri::command]
pub async fn pick_package(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<PackageSummary>, String> {
    let Some(choice) = app
        .dialog()
        .file()
        .add_filter("Website-Paket", &["tsweb"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = choice.into_path().map_err(|e| e.to_string())?;
    open_path(&app, &state, &path).map(Some)
}

/// Opens a package by path (file association, drag and drop).
#[tauri::command]
pub async fn open_package(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<PackageSummary, String> {
    open_path(&app, &state, &PathBuf::from(path))
}

fn open_path(
    app: &AppHandle,
    state: &AppState,
    path: &std::path::Path,
) -> Result<PackageSummary, String> {
    state.log(format!("open package {}", path.display()));
    let result = packages::open(
        path,
        &projects_dir(app)?,
        &data_dir(app)?.join("trust.json"),
    );
    match &result {
        Ok(summary) => state.log(format!(
            "package ok: {} {} from publisher {}{}",
            summary.name,
            summary.version,
            summary.publisher,
            if summary.first_use {
                " (new, now bound to this project)"
            } else {
                ""
            }
        )),
        Err(error) => state.log(format!("package rejected: {error}")),
    }
    result
}

/// Starts the preview for an opened package and opens it in the default browser.
#[tauri::command]
pub async fn start_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    folder: String,
) -> Result<String, String> {
    let folder = packages::checked_folder(&folder, &projects_dir(&app)?)?;
    stop_current(&state);
    let running = preview::start(&folder, &state.log)?;
    let url = running.url.clone();
    state.log(format!("preview running at {url}"));
    if let Ok(mut slot) = state.preview.lock() {
        *slot = Some(running);
    }
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(url)
}

/// Stops the preview.
#[tauri::command]
pub fn stop_preview(state: State<'_, AppState>) {
    stop_current(&state);
}

/// URL of the running preview, if any.
#[tauri::command]
pub fn preview_status(state: State<'_, AppState>) -> Option<String> {
    let mut slot = state.preview.lock().ok()?;
    let running = slot.as_mut()?;
    if running.is_running() {
        Some(running.url.clone())
    } else {
        if let Some(stopped) = slot.take() {
            stopped.stop();
        }
        None
    }
}

/// Uploads an opened package (or simulates it with `dry_run`).
#[tauri::command]
pub async fn upload(
    app: AppHandle,
    state: State<'_, AppState>,
    folder: String,
    target: UploadTarget,
    dry_run: bool,
    allow_older: bool,
) -> Result<UploadResult, String> {
    let folder = packages::checked_folder(&folder, &projects_dir(&app)?)?;
    state.log(format!("upload {} (dry run: {dry_run})", folder.display()));
    let result = tauri::async_runtime::spawn_blocking(move || {
        upload::run(&folder, target, dry_run, allow_older)
    })
    .await
    .map_err(|e| e.to_string())?;
    match &result {
        Ok(done) => {
            for line in &done.log {
                state.log(line.clone());
            }
        }
        Err(error) => state.log(format!("upload failed: {error}")),
    }
    result
}

/// The expert log.
#[tauri::command]
pub fn read_log(state: State<'_, AppState>) -> Vec<String> {
    state
        .log
        .lock()
        .map(|lines| lines.clone())
        .unwrap_or_default()
}

/// Saves the log to a file chosen by the user (for support requests).
#[tauri::command]
pub async fn export_log(app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    let Some(choice) = app
        .dialog()
        .file()
        .set_file_name("website-manager-log.txt")
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let path = choice.into_path().map_err(|e| e.to_string())?;
    let text = state
        .log
        .lock()
        .map(|lines| lines.join("\n"))
        .unwrap_or_default();
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(true)
}

fn stop_current(state: &AppState) {
    if let Ok(mut slot) = state.preview.lock()
        && let Some(running) = slot.take()
    {
        state.log(format!("preview stopped ({})", running.folder));
        running.stop();
    }
}

/// Stops background processes when the app quits.
pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        stop_current(&state);
    }
}
