use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const WS_EX_TOOLWINDOW: i32 = 0x0000_0080;
const WS_EX_NOACTIVATE: i32 = 0x0800_0000;

/// Window classes that are never part of a user's working layout.
const DENY_CLASSES: &[&str] = &[
    "Progman",                // desktop
    "WorkerW",                // desktop worker
    "Shell_TrayWnd",          // taskbar
    "Shell_SecondaryTrayWnd", // secondary taskbar
    "Windows.UI.Core.CoreWindow",
    "ApplicationFrameWindow", // UWP frame wrapper
    "IME",                    // IME default window
    "MSCTFIME UI",            // CTF IME UI
    "CiceroUIWndFrame",       // CTF framework
    "SysListView32",          // desktop list view
];

pub struct Ctx {
    pub list: Vec<HWND>,
    pub target_mon: HMONITOR,
}

extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let ctx = &mut *(lparam as *mut Ctx);
        if is_eligible(hwnd, ctx.target_mon) {
            ctx.list.push(hwnd);
        }
    }
    TRUE
}

/// Enumerate *every* top-level window (including minimized / backgrounded),
/// used by preset restore to locate workspace apps that were covered or
/// minimized when the user temporarily switched to other programs.
extern "system" fn enum_cb_all(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let list = &mut *(lparam as *mut Vec<HWND>);
        list.push(hwnd);
    }
    TRUE
}

fn get_window_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
        if n <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

fn get_class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    unsafe {
        GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(0);
    String::from_utf16_lossy(&buf[..end])
}

/// True for IME / CTF helper windows that share the host process's exe name.
/// They can look like ordinary top-level windows (visible, titled) but must
/// never be tiled, matched, or minimized by a layout restore.
fn is_ime_or_helper_raw(title: &str, class: &str) -> bool {
    if title == "Default IME" || title == "MSCTFIME UI" {
        return true;
    }
    let class = class.to_lowercase();
    class == "ime"
        || class.starts_with("ime ")
        || class == "msctfime ui"
        || class == "cicerouiwndframe"
        || class.starts_with("ctf")
}

fn is_ime_or_helper(hwnd: HWND) -> bool {
    is_ime_or_helper_raw(&get_window_text(hwnd), &get_class_name(hwnd))
}

fn has_visible_area(hwnd: HWND) -> bool {
    let r = get_rect(hwnd);
    r.right > r.left && r.bottom > r.top
}

/// A "real" user-facing window: not a system shell, not an IME helper,
/// has a non-zero rect, and can be activated. Used by both save-time and
/// restore-time filtering so the two paths agree on what counts as a window.
fn is_user_window(hwnd: HWND) -> bool {
    unsafe {
        if !is_window(hwnd) {
            return false;
        }
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if (ex & WS_EX_TOOLWINDOW) != 0 {
            return false;
        }
        if (ex & WS_EX_NOACTIVATE) != 0 {
            return false;
        }
        if is_ime_or_helper(hwnd) {
            return false;
        }
        let class = get_class_name(hwnd);
        if DENY_CLASSES.contains(&class.as_str()) {
            return false;
        }
        if !has_visible_area(hwnd) {
            return false;
        }
        true
    }
}

fn is_eligible(hwnd: HWND, target_mon: HMONITOR) -> bool {
    unsafe {
        if IsIconic(hwnd) != 0 {
            return false;
        }
        if IsWindowVisible(hwnd) == 0 {
            return false;
        }
        if GetWindowTextLengthW(hwnd) <= 0 {
            return false;
        }
        if !is_user_window(hwnd) {
            return false;
        }
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if mon != target_mon {
            return false;
        }
        true
    }
}

/// Monitor of the foreground window, falling back to the primary monitor.
pub fn target_monitor() -> HMONITOR {
    unsafe {
        let fg = GetForegroundWindow();
        if fg != 0 {
            let m = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
            if m != 0 {
                return m;
            }
        }
        MonitorFromWindow(0, MONITOR_DEFAULTTOPRIMARY)
    }
}

/// Work area (excludes the taskbar) of the given monitor.
pub fn work_area(mon: HMONITOR) -> RECT {
    unsafe {
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(mon, &mut info) != 0 {
            info.rcWork
        } else {
            RECT {
                left: 0,
                top: 0,
                right: GetSystemMetrics(SM_CXSCREEN),
                bottom: GetSystemMetrics(SM_CYSCREEN),
            }
        }
    }
}

/// Enumerate visible, non-minimized, working windows on the target monitor.
/// `exclude` is the SnapDock panel HWND so it is never tiled.
pub fn list_eligible_windows(exclude: HWND) -> Vec<HWND> {
    let target_mon = target_monitor();
    let mut ctx = Ctx {
        list: Vec::new(),
        target_mon,
    };
    unsafe {
        EnumWindows(Some(enum_cb), &mut ctx as *mut Ctx as LPARAM);
    }
    ctx.list.into_iter().filter(|&h| h != exclude).collect()
}

pub fn get_rect(hwnd: HWND) -> RECT {
    let mut r = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        GetWindowRect(hwnd, &mut r);
    }
    r
}

pub fn set_rect(hwnd: HWND, r: RECT) {
    unsafe {
        SetWindowPos(
            hwnd,
            0isize,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

/// A maximized (Zoomed) window ignores `SetWindowPos` — Chrome and many apps
/// must be restored to a normal state before they can be resized by a layout.
pub fn is_maximized(hwnd: HWND) -> bool {
    unsafe { IsZoomed(hwnd) != 0 }
}

pub fn restore(hwnd: HWND) {
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
    }
}

/// The foreground (focused) window — used as the swap source for Ctrl+方向.
pub fn foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

/// Whether a window handle still refers to a live window.
pub fn is_window(h: HWND) -> bool {
    unsafe { IsWindow(h) != 0 }
}

/// Executable basename for a window (e.g. "chrome.exe"). Used to re-match
/// saved layouts against live windows after a restart, when HWNDs are invalid.
pub fn exe_name(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }
        let h = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
        if h == 0 {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut size: u32 = buf.len() as u32;
        let path = if QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut size) != 0 {
            Some(String::from_utf16_lossy(&buf[..size as usize]))
        } else {
            None
        };
        CloseHandle(h);
        path.map(|p| p.rsplit('\\').next().unwrap_or(&p).to_string())
    }
}

/// Window title text (used as a secondary matching key for saved layouts).
pub fn window_title(hwnd: HWND) -> String {
    get_window_text(hwnd)
}

/// Enumerate real user windows (minimized or not), excluding our own panel,
/// IME helpers, system shells and toolwindows, returning (HWND, exe, title).
/// This is what lets preset restore find workspace apps that are currently
/// minimized or sitting behind other programs.
pub fn list_all_meta() -> Vec<(HWND, String, String)> {
    let self_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default();
    let mut all: Vec<HWND> = Vec::new();
    unsafe {
        EnumWindows(Some(enum_cb_all), &mut all as *mut Vec<HWND> as LPARAM);
    }
    all.into_iter()
        .filter(|&h| is_user_window(h))
        .filter_map(|h| {
            let exe = exe_name(h)?;
            if exe == self_exe {
                return None;
            }
            let title = window_title(h);
            if title.is_empty() {
                return None;
            }
            Some((h, exe, title))
        })
        .collect()
}

/// Whether a window is currently visible (not hidden). Minimized windows still
/// count as visible for our purposes; we use is_iconic to detect minimization.
pub fn is_window_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd) != 0 }
}

/// Minimize a window (used to tuck away temporary programs on restore).
pub fn minimize_window(hwnd: HWND) {
    unsafe {
        ShowWindow(hwnd, SW_MINIMIZE);
    }
}

/// Whether a window is currently minimized (iconic).
pub fn is_iconic(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd) != 0 }
}

/// Raise a window to the top of the Z-order WITHOUT stealing keyboard focus.
/// `SetForegroundWindow` is unreliable (and focus-stealing) when invoked from a
/// global-shortcut handler; raising via SetWindowPos(HWND_TOP) keeps the
/// restored workspace above the temporary programs while leaving the user's
/// focus undisturbed — this is what stops "restore opens everything" behaviour.
pub fn raise_window(hwnd: HWND) {
    unsafe {
        SetWindowPos(
            hwnd,
            0isize, // HWND_TOP
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        );
    }
}

/// Monitor a window lives on (nearest) — used to scope the "minimize temp
/// programs" step to the restored workspace's own monitor.
pub fn monitor_of(hwnd: HWND) -> HMONITOR {
    unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }
}

/// Whether `hwnd` lives on `mon` — scopes "minimize temp windows" to the same
/// monitor as the restored workspace, so other monitors are left untouched.
pub fn is_on_monitor(hwnd: HWND, mon: HMONITOR) -> bool {
    unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) == mon }
}

#[cfg(test)]
mod tests {
    use super::is_ime_or_helper_raw;

    #[test]
    fn default_ime_is_filtered() {
        assert!(is_ime_or_helper_raw("Default IME", "IME"));
    }

    #[test]
    fn msctfime_ui_is_filtered() {
        assert!(is_ime_or_helper_raw("MSCTFIME UI", "MSCTFIME UI"));
    }

    #[test]
    fn ctf_class_is_filtered() {
        assert!(is_ime_or_helper_raw("", "CTFDummyClass"));
    }

    #[test]
    fn ordinary_windows_passthrough() {
        assert!(!is_ime_or_helper_raw("微信", "WeChatMainWndForPC"));
        assert!(!is_ime_or_helper_raw("v1.5 - 文件资源管理器", "CabinetWClass"));
        assert!(!is_ime_or_helper_raw("Clash for Windows", "Chrome_WidgetWin_1"));
    }
}
