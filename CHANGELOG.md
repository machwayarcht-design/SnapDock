# Changelog

All notable changes to SnapDock are documented here. The V1.x line is the
MVP-era iteration of the project.

## [1.7.1] — First Public MVP
- **Added** real ICO application icon (extracted from `icon.ico`), shown in the About window.
- **Added** Authenticode code-signing pipeline via `build-and-sign.ps1` (uses `osslsigncode`; supply your own `.pfx` certificate).
- **Improved** extreme build slimming for an even smaller payload.

## [1.7.0]
- Removed redundant debug logging and dead code.
- Version bumped to 1.7.0.
- Aggressive size optimization in the release build (`opt-level="z"`, `lto`, `strip`, `panic="abort"`).

## [1.6.1c] / [1.6.1b]
- Eliminated the white border around the transparent window: removed the shadow flag and switched to a transparent WebView2 background with the DWM border color set to `none`.

## [1.6.1]
- UI polish: the `Ctrl + ~` panel no longer uses a transparent surface, and the `Ctrl + 1` toast no longer leaves a residual panel underneath.

## [1.6.0] — Complete MVP
- Rebuilt the tray menu (layout submenu applies a layout on click).
- Added bilingual UI (English / 简体中文).
- Added the shortcut guide (Apple-style read-only sheet).
- Added launch-on-startup via `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\SnapDock` (no admin required).
- Added the About window (sheet with real icon, version, designer, GitHub link).
- Changed `Ctrl + 1–4` restore to show a top slide-in toast instead of a panel.

## [1.0.0] – [1.5.0] — Core Engine
- Core window-arrangement engine: `layout::arrange`, `swap_direction`, `rotate`.
- Windows API wrappers: `EnumWindows`, `SetWindowPos`.
- Global hotkeys and system tray integration.
- Settings persistence.
