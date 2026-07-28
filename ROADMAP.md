# Roadmap

## ✅ V1.7.1 (Current)

Delivered in the first public MVP:

- Bilingual UI (English / 简体中文)
- Shortcut guide sheet
- Persistent system tray
- About window with real icon, version, designer, and GitHub link
- Launch on startup (HKCU Run key, no admin)
- Real ICO application icon
- Authenticode signing pipeline (`build-and-sign.ps1`)

## 🐞 V1.8 (Planned)

- **Fix**: `Ctrl+1~4` restore must read saved workspace and never minimize unrelated windows
  (root cause: `current_state()` only knows layouts SnapDock itself arranged via
  `arrange()`; restore's "minimize others" fallback fires when no state is tracked)
- **Multi-monitor layout control**: per-monitor `arrange` / save / restore, monitor picker
  in panel + tray `Monitors ▸` submenu
- Optional "minimize others" toggle (default off) for clean-workspace users

## 🚀 V2.0 (Planned)

- Customizable keyboard shortcuts
- Cloud sync
- Automatic layouts
- Floating toolbar
- Workspace management
- Auto-launch applications
- Import / export layouts
