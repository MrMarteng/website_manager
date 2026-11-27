//! Commands the window may call. Tauri hands over arguments by value.
#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use wm_core::credentials::{self, CREDENTIALS_FILE};
use wm_core::projects::{ProjectSettings, ProjectStore};

use crate::deployment::{self, UploadRequest, UploadResult};
use crate::files::{self, FileEntry};
use crate::overview::{self, Overview};
use crate::{AppState, BUILD_EVENT, UPLOAD_EVENT, keychain, preview, site};

/// File extension of exported credentials.
const CREDENTIALS_EXPORT_EXTENSION: &str = "wmzugang";

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

fn store(app: &AppHandle) -> Result<ProjectStore, String> {
    let dir = data_dir(app)?.join("projects");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(ProjectStore::new(dir))
}

fn trust_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("trust.json"))
}

/// Folder of the current version of a project.
fn current_folder(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let store = store(app)?;
    let record = store.load(name).map_err(|e| e.to_string())?;
    Ok(store.version_dir(name, &record.current))
}

fn overview_of(app: &AppHandle, name: &str) -> Result<Overview, String> {
    overview::build(&store(app)?, &trust_file(app)?, name)
}

fn decrypt_error(error: wm_core::CoreError) -> String {
    match error {
        wm_core::CoreError::WrongPassword => "code:wrong-password".to_owned(),
        other => other.to_string(),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListItem {
    pub name: String,
    pub version: String,
}

/// Projects opened before, for the start screen.
#[tauri::command]
pub fn list_projects(app: AppHandle) -> Result<Vec<ProjectListItem>, String> {
    let store = store(&app)?;
    Ok(store
        .list()
        .into_iter()
        .filter_map(|name| {
            let record = store.load(&name).ok()?;
            let manifest = store.manifest(&name, &record.current).ok()?;
            Some(ProjectListItem {
                name,
                version: manifest.version,
            })
        })
        .collect())
}

/// Result of opening a package.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenResult {
    pub overview: Overview,
    pub first_use: bool,
    pub previous_version: Option<String>,
    /// Files of the previous version the customer had changed; the previous version is kept.
    pub locally_changed: Vec<String>,
}

fn open_path(app: &AppHandle, state: &AppState, path: &Path) -> Result<OpenResult, String> {
    state.log(format!("open package {}", path.display()));
    let store = store(app)?;
    let imported = match store.import(path, &trust_file(app)?) {
        Ok(imported) => imported,
        Err(error) => {
            state.log(format!("package rejected: {error}"));
            return Err(error.to_string());
        }
    };
    for outdated in &imported.outdated {
        if let Err(error) = trash::delete(outdated) {
            state.log(format!(
                "old version not moved to trash ({}): {error}",
                outdated.display()
            ));
        }
    }
    let name = imported.package.manifest.name.clone();
    state.log(format!(
        "package ok: {name} {} ({:?})",
        imported.package.manifest.version, imported.decision
    ));
    Ok(OpenResult {
        overview: overview_of(app, &name)?,
        first_use: imported.decision == wm_core::trust::TrustDecision::FirstUse,
        previous_version: imported.previous,
        locally_changed: imported.locally_changed,
    })
}

/// Shows the file dialog and opens the chosen package.
#[tauri::command]
pub async fn pick_package(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<OpenResult>, String> {
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

/// Opens the next package handed over by double-click or drag and drop.
#[tauri::command]
pub async fn take_pending_open(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<OpenResult>, String> {
    let next = state
        .pending_open
        .lock()
        .ok()
        .and_then(|mut pending| (!pending.is_empty()).then(|| pending.remove(0)));
    match next {
        Some(path) => open_path(&app, &state, &path).map(Some),
        None => Ok(None),
    }
}

/// Opens a package by path.
#[tauri::command]
pub async fn open_package(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<OpenResult, String> {
    open_path(&app, &state, &PathBuf::from(path))
}

#[tauri::command]
pub fn project_overview(app: AppHandle, name: String) -> Result<Overview, String> {
    overview_of(&app, &name)
}

/// Decrypts the credentials shipped in the current version and stores them in the keychain.
#[tauri::command]
pub async fn import_package_credentials(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    password: String,
) -> Result<Overview, String> {
    let file = current_folder(&app, &name)?.join(CREDENTIALS_FILE);
    let content = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let decrypted = credentials::decrypt(&content, &password).map_err(decrypt_error)?;
    keychain::store(&name, &decrypted)?;
    state.log(format!("credentials for {name} stored in the keychain"));
    overview_of(&app, &name)
}

#[tauri::command]
pub fn forget_credentials(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<Overview, String> {
    keychain::delete(&name)?;
    state.log(format!("credentials for {name} removed from the keychain"));
    overview_of(&app, &name)
}

/// Saves the stored credentials, encrypted with a new password, to a file for another computer.
#[tauri::command]
pub async fn export_credentials(
    app: AppHandle,
    name: String,
    password: String,
) -> Result<bool, String> {
    let stored = keychain::load(&name)?.ok_or("code:no-credentials")?;
    let content = credentials::encrypt(&stored, &password).map_err(|e| e.to_string())?;
    let Some(choice) = app
        .dialog()
        .file()
        .set_file_name(format!("{name}.{CREDENTIALS_EXPORT_EXTENSION}"))
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let path = choice.into_path().map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Reads an exported credentials file and stores its content in the keychain.
#[tauri::command]
pub async fn import_credentials_file(
    app: AppHandle,
    name: String,
    password: String,
) -> Result<Option<Overview>, String> {
    let Some(choice) = app
        .dialog()
        .file()
        .add_filter("Zugangsdaten", &[CREDENTIALS_EXPORT_EXTENSION])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = choice.into_path().map_err(|e| e.to_string())?;
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let decrypted = credentials::decrypt(&content, &password).map_err(decrypt_error)?;
    keychain::store(&name, &decrypted)?;
    overview_of(&app, &name).map(Some)
}

/// Saves the customer's settings after checking that they produce a valid configuration.
#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    settings: ProjectSettings,
) -> Result<Overview, String> {
    let store = store(&app)?;
    let mut record = store.load(&name).map_err(|e| e.to_string())?;
    let folder = store.version_dir(&name, &record.current);
    if folder.join("deploy.conf").is_file() {
        deployment::effective_config(&folder, &settings)?;
    }
    record.settings = settings;
    store.save(&record).map_err(|e| e.to_string())?;
    state.log(format!("settings of {name} saved"));
    overview_of(&app, &name)
}

/// Starts the preview of the current version and opens it in the default browser.
#[tauri::command]
pub async fn start_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<String, String> {
    let folder = current_folder(&app, &name)?;
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

/// Uploads the current version (or simulates it); progress lines arrive as events.
#[tauri::command]
pub async fn upload(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    request: UploadRequest,
) -> Result<UploadResult, String> {
    let store = store(&app)?;
    let record = store.load(&name).map_err(|e| e.to_string())?;
    let folder = store.version_dir(&name, &record.current);
    state.log(format!(
        "upload {name} {} (dry run: {})",
        record.current, request.dry_run
    ));
    let log = std::sync::Arc::clone(&state.log);
    let emitter = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        deployment::run(&folder, &name, &record.settings, request, &mut |line| {
            crate::push_log(&log, line);
            let _ = emitter.emit(UPLOAD_EVENT, line);
        })
    })
    .await
    .map_err(|e| e.to_string())?;
    if let Err(error) = &result {
        state.log(format!("upload failed: {error}"));
    }
    result
}

/// Makes an earlier version the current one; the window then uploads it.
#[tauri::command]
pub fn restore_version(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    id: String,
) -> Result<Overview, String> {
    store(&app)?
        .set_current(&name, &id)
        .map_err(|e| e.to_string())?;
    state.log(format!("{name}: version {id} is current again"));
    overview_of(&app, &name)
}

/// Moves an earlier version to the trash.
#[tauri::command]
pub fn remove_version(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    id: String,
) -> Result<Overview, String> {
    let folder = store(&app)?
        .remove_version(&name, &id)
        .map_err(|e| e.to_string())?;
    trash::delete(&folder).map_err(|e| e.to_string())?;
    state.log(format!("{name}: version {id} moved to the trash"));
    overview_of(&app, &name)
}

#[tauri::command]
pub fn list_files(app: AppHandle, name: String) -> Result<Vec<FileEntry>, String> {
    Ok(files::list(&current_folder(&app, &name)?))
}

/// Opens a file of the current version in the program the system uses for it.
#[tauri::command]
pub fn open_file(app: AppHandle, name: String, path: String) -> Result<(), String> {
    let file = files::resolve(&current_folder(&app, &name)?, &path)?;
    app.opener()
        .open_path(file.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Shows the folder of the current version in the file manager.
#[tauri::command]
pub fn reveal_project(app: AppHandle, name: String) -> Result<(), String> {
    let folder = current_folder(&app, &name)?;
    app.opener()
        .reveal_item_in_dir(folder.join("dist"))
        .map_err(|e| e.to_string())
}

/// Opens the first compliance report of the current version.
#[tauri::command]
pub fn open_compliance(app: AppHandle, name: String) -> Result<(), String> {
    let folder = current_folder(&app, &name)?;
    let report = files::list(&folder)
        .into_iter()
        .find(|entry| entry.path.starts_with("compliance/"))
        .ok_or("no compliance report")?;
    open_file(app, name, report.path)
}

/// Opens the public website (domain from the settings) in the browser.
#[tauri::command]
pub fn open_website(app: AppHandle, name: String) -> Result<(), String> {
    let record = store(&app)?.load(&name).map_err(|e| e.to_string())?;
    let domain = record.settings.domain.ok_or("no domain set")?;
    let url = if domain.starts_with("https://") || domain.starts_with("http://") {
        domain
    } else {
        format!("https://{domain}")
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
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

/// Managed Node.js for the current version (downloaded on first use).
fn node_for(
    app: &AppHandle,
    name: &str,
) -> Result<
    (
        PathBuf,
        wm_core::manifest::Manifest,
        wm_core::runtime::node::NodeRuntime,
    ),
    String,
> {
    let store = store(app)?;
    let record = store.load(name).map_err(|e| e.to_string())?;
    let manifest = store
        .manifest(name, &record.current)
        .map_err(|e| e.to_string())?;
    let version = manifest
        .node_version
        .clone()
        .unwrap_or_else(|| site::DEFAULT_NODE_VERSION.to_owned());
    let runtime =
        wm_core::runtime::node::ensure(&version, &node_cache(app)?).map_err(|e| e.to_string())?;
    Ok((store.version_dir(name, &record.current), manifest, runtime))
}

fn node_cache(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("node"))
}

/// Removes Node.js versions no project needs any more.
fn prune_node_cache(app: &AppHandle, state: &AppState) {
    let Ok(store) = store(app) else { return };
    let keep: Vec<String> = store
        .list()
        .iter()
        .filter_map(|name| {
            let record = store.load(name).ok()?;
            let manifest = store.manifest(name, &record.current).ok()?;
            Some(
                manifest
                    .node_version
                    .unwrap_or_else(|| site::DEFAULT_NODE_VERSION.to_owned()),
            )
        })
        .collect();
    if let Ok(cache) = node_cache(app) {
        match wm_core::runtime::node::prune(&cache, &keep) {
            Ok(removed) => {
                for folder in removed {
                    state.log(format!("unused Node.js removed: {}", folder.display()));
                }
            }
            Err(error) => state.log(format!("Node.js cache not cleaned: {error}")),
        }
    }
}

/// Rebuilds the current version from its sources; output lines arrive as events.
#[tauri::command]
pub async fn build_site(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<Overview, String> {
    state.log(format!("build {name}"));
    let log = std::sync::Arc::clone(&state.log);
    let emitter = app.clone();
    let worker_app = app.clone();
    let project = name.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut emit = |line: &str| {
            crate::push_log(&log, line);
            let _ = emitter.emit(BUILD_EVENT, line);
        };
        emit("preparing Node.js …");
        let (folder, manifest, runtime) = node_for(&worker_app, &project)?;
        let output = manifest.build_output.as_deref().unwrap_or("dist");
        site::build(&folder, output, &runtime, &mut emit)
    })
    .await
    .map_err(|e| e.to_string())?;
    if let Err(error) = &result {
        state.log(format!("build failed: {error}"));
    }
    result?;
    prune_node_cache(&app, &state);
    overview_of(&app, &name)
}

/// Starts the development server of the current version and opens it in the browser.
#[tauri::command]
pub async fn start_dev(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<String, String> {
    stop_dev_server(&state);
    let log = std::sync::Arc::clone(&state.log);
    let emitter = app.clone();
    let worker_app = app.clone();
    let project = name.clone();
    let server = tauri::async_runtime::spawn_blocking(move || {
        let emit = move |line: &str| {
            crate::push_log(&log, line);
            let _ = emitter.emit(BUILD_EVENT, line);
        };
        emit("preparing Node.js …");
        let (folder, _, runtime) = node_for(&worker_app, &project)?;
        site::start_dev(&folder, &project, &runtime, emit)
    })
    .await
    .map_err(|e| e.to_string())??;
    let url = server.url.clone();
    state.log(format!("development server running at {url}"));
    if let Ok(mut slot) = state.dev.lock() {
        *slot = Some(server);
    }
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(url)
}

#[tauri::command]
pub fn stop_dev(state: State<'_, AppState>) {
    stop_dev_server(&state);
}

/// URL of the running development server, if any.
#[tauri::command]
pub fn dev_status(state: State<'_, AppState>) -> Option<String> {
    let mut slot = state.dev.lock().ok()?;
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

fn stop_dev_server(state: &AppState) {
    if let Ok(mut slot) = state.dev.lock()
        && let Some(server) = slot.take()
    {
        state.log(format!("development server stopped ({})", server.project));
        server.stop();
    }
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
        stop_dev_server(&state);
    }
}
