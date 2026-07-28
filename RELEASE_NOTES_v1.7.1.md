# SnapDock v1.7.1

The first public MVP of SnapDock — a lightweight Windows window-layout manager
that lets you instantly switch between custom window layouts using global hotkeys.

## Added
- Real ICO application icon (extracted from `icon.ico`), shown in the About window.
- Authenticode code-signing pipeline (`build-and-sign.ps1` using `osslsigncode`).

## Improved
- Extreme size optimization — total payload ~3.7 MB (exe ~3.53 MB + WebView2Loader.dll 160 KB).
- Portable package `portable.zip` is only ~1.55 MB — roughly 1/8 the size of the Snipaste installer, with zero bundled runtime.
- Slimmed release build (`opt-level="z"`, `lto`, `strip`, `panic="abort"`).

## Fixed
- White border on transparent windows (WebView2 transparent background + DWM border color set to `none`).
- `Ctrl + 1` toast no longer leaves a residual panel underneath.

## Assets
- **SnapDock_v1.7.1_portable.zip** — extract-and-run, no installation.
- **SnapDock_v1.7.1.exe** + **WebView2Loader.dll** — standalone runnable files.
- **Source code (zip / tar.gz)** — full source.

> Note: The Release binaries are intended to be signed with your own Authenticode
> certificate. Use `build-and-sign.ps1 -CertificatePath cert.pfx` before publishing.
