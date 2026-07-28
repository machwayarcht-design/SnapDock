#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod layout;
mod presets;
mod settings;
mod windowsutil;

use serde::Serialize;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::Emitter;
use tauri::Manager;
use tauri::webview::Color;
use tauri::{LogicalSize, PhysicalPosition};
#[cfg(windows)]
use windows_sys::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE,
};
use tauri_plugin_global_shortcut::{Builder, Code, Modifiers, Shortcut, ShortcutState};

// Window content sizes (logical px). One window hosts every view; we resize it
// per view so the frosted panel, the shortcuts sheet, the About sheet and the
// slide-in toast each get a fitting frame.
const PANEL_W: f64 = 420.0;
const PANEL_H: f64 = 340.0;
const SHORTCUTS_W: f64 = 460.0;
const SHORTCUTS_H: f64 = 600.0;
const ABOUT_W: f64 = 380.0;
const ABOUT_H: f64 = 440.0;
const TOAST_W: f64 = 380.0;
const TOAST_H: f64 = 170.0;

#[tauri::command]
fn arrange(mode: u8, window: tauri::WebviewWindow) {
    #[cfg(windows)]
    let exclude = window.hwnd().map(|h| h.0 as isize).unwrap_or(0isize);
    #[cfg(not(windows))]
    let exclude: isize = 0;

    layout::arrange(mode, exclude);
}

#[tauri::command]
fn hide_panel(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
fn confirm_save(slot: u8) {
    presets::confirm_save(slot);
}

#[tauri::command]
fn cancel_save() {
    presets::cancel_save();
}

/// Read the persisted settings (language + launchAtStartup) for the frontend,
/// which uses `language` to pick the locale file to load.
#[tauri::command]
fn get_settings() -> settings::Settings {
    settings::get()
}

/// Open an external URL with the user's default browser (used by the About
/// window's GitHub / website links). Native ShellExecute — no console flash.
#[tauri::command]
fn open_url(url: String) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        let op: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
        let file: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        ShellExecuteW(
            0,
            op.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        );
    }
    #[cfg(not(windows))]
    let _ = url;
}

fn size_and_center(win: &tauri::WebviewWindow, w: f64, h: f64) {
    let _ = win.set_size(LogicalSize::new(w, h));
    let _ = win.center();
}

/// Resize, horizontally center, then pin the window near the top of the screen
/// for the slide-in toast (restore / save notifications descend from the top).
fn size_and_top(win: &tauri::WebviewWindow, w: f64, h: f64) {
    let _ = win.set_size(LogicalSize::new(w, h));
    let _ = win.center();
    if let Ok(pos) = win.outer_position() {
        let _ = win.set_position(PhysicalPosition::new(pos.x, 24));
    }
}

/// Reveal the SnapDock panel (Ctrl+~). Centered, focused, entrance animation.
fn show_panel(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        size_and_center(&win, PANEL_W, PANEL_H);
        let _ = win.show();
        let _ = win.set_focus();
        let _ = app.emit("show-panel", ());
    }
}

/// Show a centered sheet (shortcuts / about) and tell the frontend which one.
fn show_sheet(app: &tauri::AppHandle, event: &str, w: f64, h: f64) {
    if let Some(win) = app.get_webview_window("main") {
        size_and_center(&win, w, h);
        let _ = win.show();
        let _ = win.set_focus();
        let _ = app.emit(event, ());
    }
}

#[derive(Clone, Serialize)]
struct NotifyPayload {
    kind: String,
    slot: u32,
    modal: bool,
}

/// A transient, non-blocking toast that slides in from the top of the screen.
/// Used for restore results and simple save results — never steals focus.
fn notify_toast(app: &tauri::AppHandle, kind: &str, slot: u8) {
    if let Some(win) = app.get_webview_window("main") {
        size_and_top(&win, TOAST_W, TOAST_H);
        let _ = win.show();
        let _ = app.emit(
            "notify",
            NotifyPayload {
                kind: kind.into(),
                slot: slot as u32,
                modal: false,
            },
        );
    }
}

/// A centered modal that needs a decision (replace-existing-layout confirm).
fn notify_modal(app: &tauri::AppHandle, kind: &str, slot: u8) {
    if let Some(win) = app.get_webview_window("main") {
        size_and_center(&win, PANEL_W, PANEL_H);
        let _ = win.show();
        let _ = win.set_focus();
        let _ = app.emit(
            "notify",
            NotifyPayload {
                kind: kind.into(),
                slot: slot as u32,
                modal: true,
            },
        );
    }
}

fn handle_save(app: &tauri::AppHandle, slot: u8) {
    match presets::request_save(slot) {
        presets::SaveResult::Saved(s) => notify_toast(app, "save-saved", s),
        presets::SaveResult::Confirm(s) => notify_modal(app, "save-confirm", s),
        presets::SaveResult::NoLayout => notify_toast(app, "save-nolayout", slot),
    }
}

fn handle_restore(app: &tauri::AppHandle, slot: u8) {
    match presets::restore(slot) {
        presets::RestoreResult::Restored(s) => notify_toast(app, "load-restored", s),
        presets::RestoreResult::Empty(s) => notify_toast(app, "load-empty", s),
        presets::RestoreResult::NoMatch(s) => notify_toast(app, "load-nomatch", s),
    }
}

/// Load the locale JSON that also drives the frontend, so the native tray menu
/// and the webview always speak the same language. Single source of truth.
fn locale_json(lang: &str) -> serde_json::Value {
    let raw = if lang == "zh-CN" {
        include_str!("../../frontend/locales/zh-CN.json")
    } else {
        include_str!("../../frontend/locales/en.json")
    };
    serde_json::from_str(raw).unwrap_or(serde_json::Value::Null)
}

fn main() {
    let panel_sc = Shortcut::new(Some(Modifiers::CONTROL), Code::Backquote);
    let rot_sc = Shortcut::new(Some(Modifiers::CONTROL), Code::Tab);
    let left_sc = Shortcut::new(Some(Modifiers::CONTROL), Code::ArrowLeft);
    let right_sc = Shortcut::new(Some(Modifiers::CONTROL), Code::ArrowRight);
    let up_sc = Shortcut::new(Some(Modifiers::CONTROL), Code::ArrowUp);
    let down_sc = Shortcut::new(Some(Modifiers::CONTROL), Code::ArrowDown);
    let save1 = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit1);
    let save2 = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit2);
    let save3 = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit3);
    let save4 = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit4);
    let load1 = Shortcut::new(Some(Modifiers::CONTROL), Code::Digit1);
    let load2 = Shortcut::new(Some(Modifiers::CONTROL), Code::Digit2);
    let load3 = Shortcut::new(Some(Modifiers::CONTROL), Code::Digit3);
    let load4 = Shortcut::new(Some(Modifiers::CONTROL), Code::Digit4);

    let global_shortcut = Builder::new()
        .with_shortcuts(vec![
            panel_sc.clone(),
            rot_sc.clone(),
            left_sc.clone(),
            right_sc.clone(),
            up_sc.clone(),
            down_sc.clone(),
            save1.clone(),
            save2.clone(),
            save3.clone(),
            save4.clone(),
            load1.clone(),
            load2.clone(),
            load3.clone(),
            load4.clone(),
        ])
        .expect("failed to register global shortcut")
        .with_handler(move |app, s, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            if s == &panel_sc {
                if let Some(win) = app.get_webview_window("main") {
                    if win.is_visible().unwrap_or(false) {
                        let _ = app.emit("hide-panel", ());
                    } else {
                        show_panel(app);
                    }
                }
            } else if s == &rot_sc {
                layout::rotate();
            } else if s == &left_sc {
                layout::swap_direction(0);
            } else if s == &right_sc {
                layout::swap_direction(1);
            } else if s == &up_sc {
                layout::swap_direction(2);
            } else if s == &down_sc {
                layout::swap_direction(3);
            } else if s == &save1 {
                handle_save(app, 0);
            } else if s == &save2 {
                handle_save(app, 1);
            } else if s == &save3 {
                handle_save(app, 2);
            } else if s == &save4 {
                handle_save(app, 3);
            } else if s == &load1 {
                handle_restore(app, 0);
            } else if s == &load2 {
                handle_restore(app, 1);
            } else if s == &load3 {
                handle_restore(app, 2);
            } else if s == &load4 {
                handle_restore(app, 3);
            }
        })
        .build();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_panel(app);
        }))
        .plugin(global_shortcut)
        .invoke_handler(tauri::generate_handler![
            arrange,
            hide_panel,
            confirm_save,
            cancel_save,
            get_settings,
            open_url
        ])
        .setup(|app| {
            presets::load_on_startup();
            let cfg = settings::load();
            let lang = cfg.language.clone();

            // Kill the white frame around the transparent window:
            // 1) WebView2 defaults to a white render surface -> force fully
            //    transparent background so no white edge shows.
            // 2) Windows DWM draws a default border on borderless windows ->
            //    set the border color to "none" (Win11 22H2+, ignored on Win10).
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_background_color(Some(Color(0, 0, 0, 0)));
                #[cfg(windows)]
                if let Ok(hwnd) = win.hwnd() {
                    unsafe {
                        let none: u32 = DWMWA_COLOR_NONE;
                        let _ = DwmSetWindowAttribute(
                            hwnd.0 as isize,
                            DWMWA_BORDER_COLOR as u32,
                            &none as *const _ as *const _,
                            std::mem::size_of::<u32>() as u32,
                        );
                    }
                }
            }

            // Localized tray labels (same JSON the webview loads).
            let t = locale_json(&lang);
            let tr = &t["tray"];
            let label = |k: &str, d: &str| -> String {
                tr.get(k)
                    .and_then(|v| v.as_str())
                    .unwrap_or(d)
                    .to_string()
            };

            let title_i = MenuItem::with_id(app, "title", &label("title", "SnapDock"), false, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;

            // Layouts ▸ (each item applies its layout immediately)
            let l_single = MenuItem::with_id(app, "layout_single", &label("single", "Single Window"), true, None::<&str>)?;
            let l_horizontal = MenuItem::with_id(app, "layout_horizontal", &label("horizontal", "Horizontal Split"), true, None::<&str>)?;
            let l_vertical = MenuItem::with_id(app, "layout_vertical", &label("vertical", "Vertical Split"), true, None::<&str>)?;
            let l_grid = MenuItem::with_id(app, "layout_grid", &label("grid", "2 × 2 Grid"), true, None::<&str>)?;
            let layouts = Submenu::with_id_and_items(
                app,
                "layouts_menu",
                &label("layouts", "Layouts"),
                true,
                &[&l_single, &l_horizontal, &l_vertical, &l_grid],
            )?;

            let shortcuts_i = MenuItem::with_id(app, "shortcuts", &label("shortcuts", "Keyboard Shortcuts"), true, None::<&str>)?;

            // Language ▸ (checkmark on the active language; switch => restart)
            let en_i = CheckMenuItem::with_id(app, "lang_en", "English", true, lang == "en", None::<&str>)?;
            let zh_i = CheckMenuItem::with_id(app, "lang_zh", "简体中文", true, lang == "zh-CN", None::<&str>)?;
            let language = Submenu::with_id_and_items(
                app,
                "language_menu",
                &label("language", "Language"),
                true,
                &[&en_i, &zh_i],
            )?;

            let startup_i = CheckMenuItem::with_id(app, "startup", &label("startup", "Launch at Startup"), true, cfg.launch_at_startup, None::<&str>)?;

            let sep2 = PredefinedMenuItem::separator(app)?;
            let about_i = MenuItem::with_id(app, "about", &label("about", "About"), true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", &label("quit", "Quit"), true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &title_i, &sep1, &layouts, &shortcuts_i, &language, &startup_i, &sep2, &about_i,
                    &quit_i,
                ],
            )?;

            let startup_cb = startup_i.clone();
            let tray_icon = Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;
            let _tray = TrayIconBuilder::with_id("snapdock-tray")
                .icon(tray_icon)
                .tooltip("SnapDock")
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "layout_single" => layout::arrange(1, 0isize),
                    "layout_horizontal" => layout::arrange(2, 0isize),
                    "layout_vertical" => layout::arrange(3, 0isize),
                    "layout_grid" => layout::arrange(4, 0isize),
                    "shortcuts" => show_sheet(app, "show-shortcuts", SHORTCUTS_W, SHORTCUTS_H),
                    "about" => show_sheet(app, "show-about", ABOUT_W, ABOUT_H),
                    "lang_en" => {
                        settings::set_language("en");
                        app.restart();
                    }
                    "lang_zh" => {
                        settings::set_language("zh-CN");
                        app.restart();
                    }
                    "startup" => {
                        let next = !settings::get().launch_at_startup;
                        settings::set_launch_at_startup(next);
                        let _ = startup_cb.set_checked(next);
                    }
                    "quit" => {
                        settings::flush();
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running SnapDock");
}
