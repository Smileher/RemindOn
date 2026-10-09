#[cfg(any(target_os = "windows", target_os = "macos"))]
use reqwest::Url;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use serde::Deserialize;
use serde::Serialize;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use sha2::{Digest, Sha256};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::collections::HashMap;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::fs::{self, File, OpenOptions};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::io::{Read, Write};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::path::{Path, PathBuf};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use tauri_plugin_updater::UpdaterExt;

use crate::AppState;

#[cfg(any(target_os = "windows", target_os = "macos"))]
const UPDATE_MANIFEST_URL: &str =
    "https://github.com/Smileher/RemindOn/releases/latest/download/latest.json";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const GITEE_UPDATE_MANIFEST_URL: &str =
    "https://gitee.com/smileher/RemindOn/releases/download/latest/latest.json";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const DOWNLOAD_PROGRESS_EVENT: &str = "portable-download-progress";
const UPDATE_STATUS_EVENT: &str = "update-status";
const UPDATE_PROGRESS_EVENT: &str = "update-progress";
const UPDATE_INTERVAL_SECONDS: u64 = 24 * 60 * 60;
#[cfg(any(target_os = "windows", target_os = "macos"))]
const MAX_AUTO_UPDATE_ATTEMPTS: u32 = 3;
#[cfg(any(target_os = "windows", target_os = "macos"))]
const AUTO_UPDATE_PAUSED: &str = "automatic-update-paused";

const STORE_BUILD: bool = option_env!("REMINDON_STORE_BUILD").is_some();

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const PORTABLE_DOWNLOAD_TARGET: &str = "windows-x86_64-portable";
#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
const PORTABLE_DOWNLOAD_TARGET: &str = "windows-aarch64-portable";
#[cfg(target_os = "macos")]
const PORTABLE_DOWNLOAD_TARGET: &str = "darwin-aarch64-portable";

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateMode {
    Installed,
    Portable,
    Development,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UpdatePhase {
    Idle,
    Checking,
    Available,
    Downloading,
    Installing,
    Error,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRuntimeState {
    pub phase: UpdatePhase,
    pub version: Option<String>,
    pub error: Option<String>,
}

impl Default for UpdateRuntimeState {
    fn default() -> Self {
        Self {
            phase: UpdatePhase::Idle,
            version: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percentage: Option<u8>,
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[derive(Deserialize)]
struct UpdateManifest {
    version: String,
    downloads: HashMap<String, PortableDownloadAsset>,
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortableDownloadAsset {
    url: String,
    file_name: String,
    sha256: String,
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortableDownloadProgress {
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    percentage: Option<u8>,
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[derive(Default, Deserialize, Serialize)]
struct UpdateAttempts {
    version: String,
    attempts: u32,
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn record_update_attempt(path: &Path, current_version: &str, target_version: &str, force: bool) -> Result<bool, String> {
    let mut record = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<UpdateAttempts>(&bytes).map_err(|error| format!("Invalid update attempt record: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => UpdateAttempts::default(),
        Err(error) => return Err(format!("Failed to read update attempt record: {error}")),
    };
    if record.version != target_version || !is_release_newer(&record.version, current_version)? {
        record = UpdateAttempts { version: target_version.to_string(), attempts: 0 };
    }
    if !force && record.attempts >= MAX_AUTO_UPDATE_ATTEMPTS {
        return Ok(false);
    }
    // 安装器可能直接退出旧进程，先落盘；新版启动后按实际版本重置计数。
    record.attempts = record.attempts.saturating_add(1);
    let bytes = serde_json::to_vec(&record).map_err(|error| error.to_string())?;
    crate::atomic_write(path, &bytes).map_err(|error| format!("Failed to save update attempt record: {error}"))?;
    Ok(true)
}

#[tauri::command]
pub fn get_update_mode(app: tauri::AppHandle) -> Result<UpdateMode, String> {
    if cfg!(debug_assertions) {
        return Ok(UpdateMode::Development);
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    if is_installed(&app, &executable) {
        Ok(UpdateMode::Installed)
    } else if cfg!(any(target_os = "windows", target_os = "macos")) {
        Ok(UpdateMode::Portable)
    } else {
        Ok(UpdateMode::Unsupported)
    }
}

#[tauri::command]
pub fn get_update_status(state: State<'_, AppState>) -> Result<UpdateRuntimeState, String> {
    Ok(state
        .0
        .update_state
        .lock().map_err(crate::lock_error)?
        .clone())
}

#[tauri::command]
pub fn get_update_progress(state: State<'_, AppState>) -> Result<Option<UpdateProgress>, String> {
    Ok(state
        .0
        .update_progress
        .lock().map_err(crate::lock_error)?
        .clone())
}

#[tauri::command]
pub async fn check_for_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    force: bool,
) -> Result<UpdateRuntimeState, String> {
    run_update_check(app, state.inner().clone(), force).await
}

pub fn start_background_update_checks(app: AppHandle, state: AppState) {
    std::thread::spawn(move || loop {
        if state.0.update_state.is_poisoned() || state.0.update_progress.is_poisoned() {
            eprintln!("Background update checks stopped because state is poisoned");
            break;
        }
        let _ = crate::report_native("Background update check", tauri::async_runtime::block_on(run_update_check(app.clone(), state.clone(), false)));
        std::thread::sleep(std::time::Duration::from_secs(UPDATE_INTERVAL_SECONDS));
    });
}

async fn run_update_check(app: AppHandle, state: AppState, force: bool) -> Result<UpdateRuntimeState, String> {
    if STORE_BUILD || cfg!(debug_assertions) {
        return Ok(set_update_state(
            &app,
            &state,
            UpdatePhase::Idle,
            None,
            None,
        )?);
    }

    {
        let mut current = state
            .0
            .update_state
            .lock().map_err(crate::lock_error)?;
        if matches!(
            current.phase,
            UpdatePhase::Checking | UpdatePhase::Downloading | UpdatePhase::Installing
        ) {
            return Ok(current.clone());
        }
        current.phase = UpdatePhase::Checking;
        current.version = None;
        current.error = None;
        emit_update_status(&app, &current);
    }
    *state
        .0
        .update_progress
        .lock().map_err(crate::lock_error)? = None;

    let result = run_platform_update(&app, &state, force).await;
    match result {
        Ok(state) => Ok(state),
        Err(error) => {
            set_update_state(&app, &state, UpdatePhase::Error, None, Some(error.clone()))?;
            Err(error)
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
async fn run_platform_update(
    app: &AppHandle,
    state: &AppState,
    force: bool,
) -> Result<UpdateRuntimeState, String> {
    match get_update_mode(app.clone())? {
        UpdateMode::Installed => {
            let update = app
                .updater()
                .map_err(|error| error.to_string())?
                .check()
                .await
                .map_err(|error| error.to_string())?;
            let Some(update) = update else {
                return Ok(set_update_state(app, state, UpdatePhase::Idle, None, None)?);
            };
            let attempts_path = crate::config_directory(app)?.join("update-attempts.json");
            if !record_update_attempt(&attempts_path, &app.package_info().version.to_string(), &update.version, force)? {
                return set_update_state(app, state, UpdatePhase::Error, Some(update.version), Some(AUTO_UPDATE_PAUSED.to_string()));
            }
            set_update_state(
                app,
                state,
                UpdatePhase::Available,
                Some(update.version.clone()),
                None,
            )?;
            set_update_state(
                app,
                state,
                UpdatePhase::Downloading,
                Some(update.version.clone()),
                None,
            )?;
            let mut downloaded = 0_u64;
            let progress_app = app.clone();
            let progress_state = state.clone();
            let install_app = app.clone();
            let install_state = state.clone();
            let install_version = update.version.clone();
            update
                .download_and_install(
                    move |chunk, total| {
                        downloaded = downloaded.saturating_add(chunk as u64);
                        let _ = crate::report_native("Set updater progress", set_update_progress(&progress_app, &progress_state, downloaded, total));
                    },
                    || {
                        let _ = crate::report_native("Set updater install state", set_update_state(
                            &install_app,
                            &install_state,
                            UpdatePhase::Installing,
                            Some(install_version),
                            None,
                        ));
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            #[cfg(target_os = "macos")]
            app.request_restart();
            get_update_status_from_state(state)
        }
        UpdateMode::Portable => {
            let Some((version, manifest_url)) = check_portable_update(app).await? else {
                return Ok(set_update_state(app, state, UpdatePhase::Idle, None, None)?);
            };
            set_update_state(
                app,
                state,
                UpdatePhase::Available,
                Some(version.clone()),
                None,
            )?;
            set_update_state(
                app,
                state,
                UpdatePhase::Downloading,
                Some(version.clone()),
                None,
            )?;
            download_portable_update_from_manifest(app, &version, &manifest_url).await?;
            Ok(set_update_state(
                app,
                state,
                UpdatePhase::Available,
                Some(version),
                None,
            )?)
        }
        UpdateMode::Development | UpdateMode::Unsupported => {
            Ok(set_update_state(app, state, UpdatePhase::Idle, None, None)?)
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
async fn run_platform_update(
    app: &AppHandle,
    state: &AppState,
    _force: bool,
) -> Result<UpdateRuntimeState, String> {
    Ok(set_update_state(app, state, UpdatePhase::Idle, None, None)?)
}

fn get_update_status_from_state(state: &AppState) -> Result<UpdateRuntimeState, String> {
    Ok(state
        .0
        .update_state
        .lock().map_err(crate::lock_error)?
        .clone())
}

fn set_update_state(
    app: &AppHandle,
    state: &AppState,
    phase: UpdatePhase,
    version: Option<String>,
    error: Option<String>,
) -> Result<UpdateRuntimeState, String> {
    let next = UpdateRuntimeState {
        phase,
        version,
        error,
    };
    *state
        .0
        .update_state
        .lock().map_err(crate::lock_error)? = next.clone();
    emit_update_status(app, &next);
    Ok(next)
}

fn emit_update_status(app: &AppHandle, state: &UpdateRuntimeState) {
    let _ = app.emit_to("main", UPDATE_STATUS_EVENT, state);
}

fn set_update_progress(
    app: &AppHandle,
    state: &AppState,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) -> Result<(), String> {
    let progress = UpdateProgress {
        downloaded_bytes,
        total_bytes,
        percentage: total_bytes
            .map(|total| ((downloaded_bytes.saturating_mul(100) / total).min(100)) as u8),
    };
    *state
        .0
        .update_progress
        .lock().map_err(crate::lock_error)? = Some(progress.clone());
    let _ = app.emit_to("main", UPDATE_PROGRESS_EVENT, &progress);
    Ok(())
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
async fn check_portable_update(app: &AppHandle) -> Result<Option<(String, &'static str)>, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| error.to_string())?;
    let current_version = app.package_info().version.to_string();
    let mut last_error = None;
    for manifest_url in [UPDATE_MANIFEST_URL, GITEE_UPDATE_MANIFEST_URL] {
        match fetch_portable_manifest(&client, manifest_url).await {
            Ok(manifest) => {
                validate_release_version(&manifest.version)?;
                let asset = manifest
                    .downloads
                    .get(PORTABLE_DOWNLOAD_TARGET)
                    .ok_or_else(|| {
                        "No portable update is available for the current platform".to_string()
                    })?;
                validate_download_asset(&manifest.version, asset)?;
                if is_release_newer(&manifest.version, &current_version)? {
                    return Ok(Some((manifest.version, manifest_url)));
                }
                return Ok(None);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "No update manifest is available".to_string()))
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
async fn fetch_portable_manifest(
    client: &reqwest::Client,
    manifest_url: &str,
) -> Result<UpdateManifest, String> {
    let bytes = client
        .get(manifest_url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?
        .bytes()
        .await
        .map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[tauri::command]
pub async fn download_portable_update(
    app: tauri::AppHandle,
    expected_version: String,
) -> Result<String, String> {
    download_portable_update_from_manifest(&app, &expected_version, UPDATE_MANIFEST_URL).await
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[tauri::command]
pub async fn download_portable_update_from_gitee(
    app: tauri::AppHandle,
    expected_version: String,
) -> Result<String, String> {
    download_portable_update_from_manifest(&app, &expected_version, GITEE_UPDATE_MANIFEST_URL).await
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
async fn download_portable_update_from_manifest(
    app: &tauri::AppHandle,
    expected_version: &str,
    manifest_url: &str,
) -> Result<String, String> {
    if get_update_mode(app.clone())? != UpdateMode::Portable {
        return Err("The current copy is not running in portable mode".to_string());
    }
    validate_release_version(expected_version)?;

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|error| error.to_string())?;
    let manifest_bytes = client
        .get(manifest_url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?
        .bytes()
        .await
        .map_err(|error| error.to_string())?;
    let manifest: UpdateManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| error.to_string())?;
    if manifest.version != expected_version {
        return Err("The available version changed; check for updates again".to_string());
    }
    let asset = manifest
        .downloads
        .get(PORTABLE_DOWNLOAD_TARGET)
        .ok_or_else(|| "No portable update is available for the current platform".to_string())?;
    validate_download_asset(expected_version, asset)?;

    let download_dir = app
        .path()
        .download_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&download_dir).map_err(|error| error.to_string())?;
    let expected_hash = asset.sha256.to_ascii_lowercase();
    if let Some(path) = verified_download_path(&download_dir, &asset.file_name, &expected_hash) {
        emit_download_progress(app, 1, Some(1));
        return Ok(path.to_string_lossy().into_owned());
    }
    let final_path = available_download_path(&download_dir, &asset.file_name);
    let part_path = partial_download_path(&final_path);
    if part_path.exists() {
        fs::remove_file(&part_path).map_err(|error| error.to_string())?;
    }
    if let Err(error) = download_to_file(&client, app, asset, &part_path).await {
        let _ = fs::remove_file(&part_path);
        return Err(error);
    }
    if checksum_file(&part_path)? != expected_hash {
        let _ = fs::remove_file(&part_path);
        return Err("The downloaded file failed verification; download it again".to_string());
    }
    fs::rename(&part_path, &final_path).map_err(|error| {
        let _ = fs::remove_file(&part_path);
        error.to_string()
    })?;
    Ok(final_path.to_string_lossy().into_owned())
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[tauri::command]
pub async fn install_update_from_gitee(
    app: tauri::AppHandle,
    expected_version: String,
) -> Result<(), String> {
    validate_release_version(&expected_version)?;
    let endpoint = Url::parse(GITEE_UPDATE_MANIFEST_URL).map_err(|error| error.to_string())?;
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|error| error.to_string())?
        .build()
        .map_err(|error| error.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No update is available from Gitee".to_string())?;
    if update.version != expected_version {
        return Err("The Gitee update version does not match".to_string());
    }
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| error.to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[tauri::command]
pub async fn download_portable_update_from_gitee(
    _app: tauri::AppHandle,
    _expected_version: String,
) -> Result<String, String> {
    Err("The current platform does not support portable updates".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[tauri::command]
pub async fn install_update_from_gitee(
    _app: tauri::AppHandle,
    _expected_version: String,
) -> Result<(), String> {
    Err("The current platform does not support in-app updates".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[tauri::command]
pub async fn download_portable_update(
    _app: tauri::AppHandle,
    _expected_version: String,
) -> Result<String, String> {
    Err("The current platform does not support portable updates".to_string())
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn validate_release_version(version: &str) -> Result<(), String> {
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        Ok(())
    } else {
        Err("Invalid update version format".to_string())
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn is_release_newer(candidate: &str, current: &str) -> Result<bool, String> {
    validate_release_version(current)?;
    let parse = |version: &str| {
        version
            .split('.')
            .map(|part| part.parse::<u64>().map_err(|error| error.to_string()))
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(parse(candidate)? > parse(current)?)
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn validate_download_asset(version: &str, asset: &PortableDownloadAsset) -> Result<(), String> {
    if asset.file_name != expected_download_file_name(version) {
        return Err("Invalid update file name".to_string());
    }
    let url = Url::parse(&asset.url).map_err(|_| "Invalid update download URL".to_string())?;
    let valid_host = matches!(url.host_str(), Some("github.com" | "gitee.com"));
    let expected_prefix = format!("/Smileher/RemindOn/releases/download/v{version}/");
    let expected_gitee_prefix = format!("/smileher/RemindOn/releases/download/v{version}/");
    if !valid_host
        || (!url.path().starts_with(&expected_prefix)
            && !url.path().starts_with(&expected_gitee_prefix))
    {
        return Err("Invalid update download URL".to_string());
    }
    if asset.sha256.len() != 64 || !asset.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Invalid update file checksum".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn expected_download_file_name(version: &str) -> String {
    if cfg!(target_arch = "aarch64") {
        format!("RemindOn_{version}_arm64_portable.zip")
    } else {
        format!("RemindOn_{version}_x64_portable.zip")
    }
}

#[cfg(target_os = "macos")]
fn expected_download_file_name(version: &str) -> String {
    format!("RemindOn_{version}_aarch64.dmg")
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn available_download_path(directory: &Path, file_name: &str) -> PathBuf {
    let preferred = directory.join(file_name);
    if !preferred.exists() {
        return preferred;
    }
    let file = Path::new(file_name);
    let stem = file
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("RemindOn");
    let extension = file.extension().and_then(|value| value.to_str());
    for index in 1.. {
        let candidate_name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = directory.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn verified_download_path(directory: &Path, file_name: &str, expected_hash: &str) -> Option<PathBuf> {
    let preferred = directory.join(file_name);
    if preferred.is_file() && checksum_file(&preferred).is_ok_and(|hash| hash == expected_hash) {
        return Some(preferred);
    }
    let file = Path::new(file_name);
    let stem = file.file_stem()?.to_str()?;
    let extension = file.extension()?.to_str()?;
    let prefix = format!("{stem} (");
    let suffix = format!(").{extension}");
    // 兼容已下载的编号副本；编号中间被删除也不能导致再次下载。
    for entry in fs::read_dir(directory).ok()? {
        let Ok(entry) = entry else { continue; };
        let name = entry.file_name();
        let Some(index) = name.to_str().and_then(|name| name.strip_prefix(&prefix))
            .and_then(|name| name.strip_suffix(&suffix)) else { continue; };
        if index.parse::<u64>().is_ok_and(|index| index > 0) {
            let path = entry.path();
            if path.is_file() && checksum_file(&path).is_ok_and(|hash| hash == expected_hash) {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn partial_download_path(final_path: &Path) -> PathBuf {
    let mut name = final_path.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    final_path.with_file_name(name)
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn checksum_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
async fn download_to_file(
    client: &reqwest::Client,
    app: &tauri::AppHandle,
    asset: &PortableDownloadAsset,
    part_path: &Path,
) -> Result<(), String> {
    let mut response = client
        .get(&asset.url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?;
    let total = response.content_length().filter(|length| *length > 0);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(part_path)
        .map_err(|error| error.to_string())?;
    let mut downloaded = 0_u64;
    emit_download_progress(app, downloaded, total);
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        file.write_all(&chunk).map_err(|error| error.to_string())?;
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        emit_download_progress(app, downloaded, total);
    }
    file.flush().map_err(|error| error.to_string())?;
    // Complete progress even when the server omits Content-Length.
    if downloaded > 0 {
        emit_download_progress(app, downloaded, Some(downloaded));
    }
    Ok(())
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn emit_download_progress(app: &tauri::AppHandle, downloaded: u64, total: Option<u64>) {
    let percentage = total.map(|total| ((downloaded.saturating_mul(100) / total).min(100)) as u8);
    if let Some(state) = app.try_state::<AppState>() {
        let _ = crate::report_native("Set download progress", set_update_progress(app, state.inner(), downloaded, total));
    }
    let _ = app.emit_to(
        "main",
        DOWNLOAD_PROGRESS_EVENT,
        PortableDownloadProgress {
            downloaded_bytes: downloaded,
            total_bytes: total,
            percentage,
        },
    );
}

#[cfg(target_os = "windows")]
fn is_installed(app: &tauri::AppHandle, executable: &Path) -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_64KEY};
    use winreg::RegKey;

    // NSIS currentUser installs record a quoted InstallLocation under the product name.
    let key = format!(
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{}",
        app.package_info().name
    );
    let location = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(key, KEY_READ | KEY_WOW64_64KEY)
        .and_then(|key| key.get_value::<String, _>("InstallLocation"));
    location.is_ok_and(|location| matches_windows_install(executable, &location))
}

#[cfg(target_os = "windows")]
fn matches_windows_install(executable: &Path, location: &str) -> bool {
    let install_dir = Path::new(location.trim_matches('"'));
    // A copied standalone executable must not start an installer for another copy.
    match (executable.parent(), install_dir.canonicalize()) {
        (Some(parent), Ok(installed)) if installed.join("uninstall.exe").is_file() => parent
            .canonicalize()
            .is_ok_and(|parent| parent == installed),
        _ => false,
    }
}

#[cfg(target_os = "macos")]
fn is_installed(app: &tauri::AppHandle, executable: &Path) -> bool {
    use tauri::Manager;

    let Some(bundle) = executable.ancestors().nth(3) else {
        return false;
    };
    if bundle
        .extension()
        .is_none_or(|extension| extension != "app")
    {
        return false;
    }
    // Copies still on the DMG or in App Translocation must be moved to Applications first.
    bundle.starts_with("/Applications")
        || app
            .path()
            .home_dir()
            .is_ok_and(|home| bundle.starts_with(home.join("Applications")))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn is_installed(_app: &tauri::AppHandle, _executable: &std::path::Path) -> bool {
    false
}

#[cfg(all(test, any(target_os = "windows", target_os = "macos")))]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "remindon-updater-{name}-{}-{suffix}",
            std::process::id()
        ))
    }

    fn valid_asset(version: &str) -> PortableDownloadAsset {
        let file_name = expected_download_file_name(version);
        PortableDownloadAsset {
            url: format!(
                "https://github.com/Smileher/RemindOn/releases/download/v{version}/{file_name}"
            ),
            file_name,
            sha256: "0".repeat(64),
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn only_the_registered_installation_can_update() {
        let root = test_root("install-mode");
        let installed = root.join("installed");
        let portable = root.join("portable");
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::create_dir_all(&portable).unwrap();
        let location = format!("\"{}\"", installed.display());
        assert!(!matches_windows_install(
            &installed.join("remindon.exe"),
            &location
        ));
        std::fs::write(installed.join("uninstall.exe"), b"test").unwrap();
        assert!(matches_windows_install(
            &installed.join("remindon.exe"),
            &location
        ));
        assert!(!matches_windows_install(
            &portable.join("remindon.exe"),
            &location
        ));
        assert!(!matches_windows_install(
            &installed.join("remindon.exe"),
            ""
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn portable_asset_must_match_version_repository_and_checksum_shape() {
        let mut asset = valid_asset("0.7.0");
        assert!(validate_release_version("0.7.0").is_ok());
        assert!(validate_download_asset("0.7.0", &asset).is_ok());
        assert!(validate_release_version("0.7").is_err());

        #[cfg(target_os = "windows")]
        {
            assert!(asset.file_name.ends_with("_portable.zip"));
            asset.file_name = asset.file_name.replace(".zip", ".exe");
            assert!(validate_download_asset("0.7.0", &asset).is_err());
            asset = valid_asset("0.7.0");
        }

        asset.url = asset.url.replace("Smileher/RemindOn", "other/project");
        assert!(validate_download_asset("0.7.0", &asset).is_err());
        asset = valid_asset("0.7.0");
        asset.sha256 = "not-a-checksum".to_string();
        assert!(validate_download_asset("0.7.0", &asset).is_err());
        asset = valid_asset("0.7.0");
        asset.file_name = "another-file.exe".to_string();
        assert!(validate_download_asset("0.7.0", &asset).is_err());
    }

    #[test]
    fn release_comparison_uses_numeric_components() {
        assert!(is_release_newer("1.10.0", "1.9.0").unwrap());
        assert!(!is_release_newer("1.2.0", "1.2.0").unwrap());
        assert!(!is_release_newer("1.1.9", "1.2.0").unwrap());
        assert!(is_release_newer("2.0.0", "1.99.99").unwrap());
    }

    #[test]
    fn existing_downloads_get_a_non_destructive_numbered_name() {
        let root = test_root("download-name");
        fs::create_dir_all(&root).unwrap();
        let file_name = expected_download_file_name("0.7.0");
        let preferred = root.join(&file_name);
        fs::write(&preferred, b"test").unwrap();
        let numbered = available_download_path(&root, &file_name);
        assert_ne!(numbered, preferred);
        assert!(!numbered.exists());
        assert_eq!(
            checksum_file(&preferred).unwrap(),
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
        assert!(partial_download_path(&numbered)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(".part"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verified_downloads_are_reused_without_overwriting_other_files() {
        let root = test_root("reuse-download");
        fs::create_dir_all(&root).unwrap();
        let file_name = expected_download_file_name("0.7.0");
        let preferred = root.join(&file_name);
        fs::write(&preferred, b"verified update").unwrap();
        let expected_hash = checksum_file(&preferred).unwrap();
        assert_eq!(verified_download_path(&root, &file_name, &expected_hash), Some(preferred.clone()));

        fs::write(&preferred, b"different file").unwrap();
        let first = available_download_path(&root, &file_name);
        fs::write(&first, b"partial file").unwrap();
        let second = available_download_path(&root, &file_name);
        fs::write(&second, b"verified update").unwrap();
        fs::remove_file(first).unwrap();
        assert_eq!(verified_download_path(&root, &file_name, &expected_hash), Some(second.clone()));
        assert_eq!(fs::read(&preferred).unwrap(), b"different file");

        fs::write(&second, b"corrupt update").unwrap();
        assert!(verified_download_path(&root, &file_name, &expected_hash).is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_update_attempts_pause_across_restarts_and_allow_manual_or_new_versions() {
        let root = test_root("update-attempts");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("update-attempts.json");
        for _ in 0..MAX_AUTO_UPDATE_ATTEMPTS {
            assert!(record_update_attempt(&path, "1.3.1", "1.3.2", false).unwrap());
        }
        let paused_record = fs::read(&path).unwrap();
        assert!(!record_update_attempt(&path, "1.3.1", "1.3.2", false).unwrap());
        assert_eq!(fs::read(&path).unwrap(), paused_record);
        assert!(record_update_attempt(&path, "1.3.1", "1.3.2", true).unwrap());
        assert!(!record_update_attempt(&path, "1.3.1", "1.3.2", false).unwrap());
        assert!(record_update_attempt(&path, "1.3.1", "1.3.3", false).unwrap());
        let record: UpdateAttempts = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(record.attempts, 1);
        assert_eq!(record.version, "1.3.3");
        assert!(record_update_attempt(&path, "1.3.3", "1.3.3", false).unwrap());
        let record: UpdateAttempts = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(record.attempts, 1);
        assert!(record_update_attempt(&path, "1.3.3", "1.3.4", false).unwrap());
        let record: UpdateAttempts = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(record.attempts, 1);
        fs::write(&path, b"invalid record").unwrap();
        assert!(record_update_attempt(&path, "1.3.1", "1.3.2", false).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"invalid record");
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(record_update_attempt(&path, "1.3.1", "1.3.2", true).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
