// SnapDock V1.6 — local settings persistence + Windows startup registration.
// Settings live at %APPDATA%\SnapDock\settings.json and hold only what the
// V1.6 task book asks for: the UI language and the launch-at-startup flag.
// Startup is registered via the per-user Run key (no admin rights required).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(rename = "launchAtStartup", default)]
    pub launch_at_startup: bool,
}

fn default_language() -> String {
    "en".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: default_language(),
            launch_at_startup: false,
        }
    }
}

static SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);

/// %APPDATA%\SnapDock (created on first save).
fn settings_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("SnapDock"))
}

fn settings_path() -> Option<PathBuf> {
    settings_dir().map(|d| d.join("settings.json"))
}

/// Read settings.json at startup (falls back to defaults). Caches the result.
pub fn load() -> Settings {
    let s = settings_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|txt| serde_json::from_str::<Settings>(&txt).ok())
        .unwrap_or_default();
    *SETTINGS.lock().unwrap() = Some(s.clone());
    s
}

/// Current in-memory settings (defaults if not loaded yet).
pub fn get() -> Settings {
    SETTINGS
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_default()
}

fn save(s: &Settings) {
    if let Some(dir) = settings_dir() {
        let _ = fs::create_dir_all(&dir);
    }
    if let Some(path) = settings_path() {
        if let Ok(json) = serde_json::to_string_pretty(s) {
            let _ = fs::write(path, json);
        }
    }
    *SETTINGS.lock().unwrap() = Some(s.clone());
}

/// Persist a language choice ("en" / "zh-CN"). Takes effect on next launch;
/// callers typically restart the app right after.
pub fn set_language(lang: &str) {
    let mut s = get();
    s.language = lang.to_string();
    save(&s);
}

/// Toggle launch-at-startup: writes settings.json AND updates the Run key.
pub fn set_launch_at_startup(enabled: bool) {
    let mut s = get();
    s.launch_at_startup = enabled;
    save(&s);
    #[cfg(windows)]
    apply_startup_registry(enabled);
}

/// Save whatever is currently in memory (used on Quit).
pub fn flush() {
    let s = get();
    save(&s);
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Add/remove `SnapDock` under HKCU\...\Run so Windows starts it at logon.
/// Uses the current-user hive only — no administrator privileges required.
#[cfg(windows)]
fn apply_startup_registry(enabled: bool) {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, REG_SZ,
    };

    unsafe {
        let subkey = to_wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
        let mut hkey: HKEY = 0;
        // RegCreateKeyW creates-or-opens the key (and any intermediate
        // keys) using default access — enough for our per-user Run entry.
        let rc = RegCreateKeyW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            &mut hkey,
        );
        if rc != ERROR_SUCCESS {
            return;
        }
        let name = to_wide("SnapDock");
        if enabled {
            if let Ok(exe) = std::env::current_exe() {
                // Quote the path so spaces in the install folder are handled.
                let val = format!("\"{}\"", exe.display());
                let wide = to_wide(&val);
                let bytes = (wide.len() * std::mem::size_of::<u16>()) as u32;
                RegSetValueExW(
                    hkey,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    wide.as_ptr() as *const u8,
                    bytes,
                );
            }
        } else {
            // Deleting a missing value is fine; we ignore the result.
            RegDeleteValueW(hkey, name.as_ptr());
        }
        RegCloseKey(hkey);
    }
}
