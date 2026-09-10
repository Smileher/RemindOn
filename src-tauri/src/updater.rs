use serde::Serialize;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::path::Path;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateMode {
    Installed,
    Portable,
    Development,
    Unsupported,
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

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn only_the_registered_installation_can_update() {
        let root = std::env::temp_dir().join(format!("remindon-updater-{}", std::process::id()));
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
}
