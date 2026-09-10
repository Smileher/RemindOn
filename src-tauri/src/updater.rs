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
#[cfg(any(target_os = "windows", target_os = "macos"))]
use tauri::{Emitter, Manager};

#[cfg(any(target_os = "windows", target_os = "macos"))]
const UPDATE_MANIFEST_URL: &str =
    "https://github.com/Smileher/RemindOn/releases/latest/download/latest.json";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const DOWNLOAD_PROGRESS_EVENT: &str = "portable-download-progress";

#[cfg(target_os = "windows")]
const PORTABLE_DOWNLOAD_TARGET: &str = "windows-x86_64-portable";
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

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[tauri::command]
pub async fn download_portable_update(
    app: tauri::AppHandle,
    expected_version: String,
) -> Result<String, String> {
    if get_update_mode(app.clone())? != UpdateMode::Portable {
        return Err("当前副本不是便携运行模式".to_string());
    }
    validate_release_version(&expected_version)?;

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|error| error.to_string())?;
    let manifest_bytes = client
        .get(UPDATE_MANIFEST_URL)
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
        return Err("更新版本已经变化，请重新检查更新".to_string());
    }
    let asset = manifest
        .downloads
        .get(PORTABLE_DOWNLOAD_TARGET)
        .ok_or_else(|| "当前平台没有可下载的便携更新".to_string())?;
    validate_download_asset(&expected_version, asset)?;

    let download_dir = app
        .path()
        .download_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&download_dir).map_err(|error| error.to_string())?;
    let expected_hash = asset.sha256.to_ascii_lowercase();
    let preferred_path = download_dir.join(&asset.file_name);
    if preferred_path.is_file()
        && checksum_file(&preferred_path).is_ok_and(|hash| hash == expected_hash)
    {
        emit_download_progress(&app, 1, Some(1));
        return Ok(preferred_path.to_string_lossy().into_owned());
    }

    let final_path = available_download_path(&download_dir, &asset.file_name);
    let part_path = partial_download_path(&final_path);
    if part_path.exists() {
        fs::remove_file(&part_path).map_err(|error| error.to_string())?;
    }
    let result = download_to_file(&client, &app, asset, &part_path).await;
    if let Err(error) = result {
        let _ = fs::remove_file(&part_path);
        return Err(error);
    }
    let downloaded_hash = checksum_file(&part_path)?;
    if downloaded_hash != expected_hash {
        let _ = fs::remove_file(&part_path);
        return Err("下载文件校验失败，请重新下载".to_string());
    }
    if let Err(error) = fs::rename(&part_path, &final_path) {
        let _ = fs::remove_file(&part_path);
        return Err(error.to_string());
    }
    Ok(final_path.to_string_lossy().into_owned())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[tauri::command]
pub async fn download_portable_update(
    _app: tauri::AppHandle,
    _expected_version: String,
) -> Result<String, String> {
    Err("当前平台不支持便携更新".to_string())
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
        Err("更新版本格式无效".to_string())
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn validate_download_asset(version: &str, asset: &PortableDownloadAsset) -> Result<(), String> {
    if asset.file_name != expected_download_file_name(version) {
        return Err("更新文件名无效".to_string());
    }
    let expected_prefix =
        format!("https://github.com/Smileher/RemindOn/releases/download/v{version}/");
    if !asset.url.starts_with(&expected_prefix) {
        return Err("更新下载地址无效".to_string());
    }
    if asset.sha256.len() != 64 || !asset.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("更新文件校验值无效".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn expected_download_file_name(version: &str) -> String {
    format!("RemindOn_{version}_x64_portable.exe")
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
    Ok(())
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn emit_download_progress(app: &tauri::AppHandle, downloaded: u64, total: Option<u64>) {
    let percentage = total.map(|total| ((downloaded.saturating_mul(100) / total).min(100)) as u8);
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
}
