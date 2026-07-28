# Contributing to SnapDock

Thanks for your interest in improving SnapDock! This document explains how to
build, sign, and submit changes.

## Building

Requirements:

- **Rust** (stable toolchain)
- **MinGW-w64** toolchain with the GNU target `x86_64-pc-windows-gnu`
- **Windows** with **WebView2** (preinstalled on most Windows 10/11)

Add the target and build:

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu --release
```

The binary is produced at:

```
src-tauri/target/x86_64-pc-windows-gnu/release/snapdock.exe
```

## Signing (optional)

To Authenticode-sign the binary, use the provided pipeline:

```powershell
pwsh ./build-and-sign.ps1 -CertificatePath cert.pfx
```

You must supply your own `.pfx` certificate. The repository does not include
any signing keys.

## Submitting Changes

1. **Fork** the repository and create a feature branch.
2. Make your changes with clear, focused commits.
3. Open a **Pull Request** describing the change and its motivation.

We use a **Conventional Commits** style, for example:

- `feat: add custom shortcut editor`
- `fix: remove white border on panel`
- `docs: clarify build instructions`

## Code Style

- **Rust:** run `cargo fmt` before committing.
- **Frontend:** plain, framework-free **vanilla JavaScript** (HTML / CSS / JS).
  There is **no** React / Vue / TypeScript build step — keep it native WebView.
- Keep the binary small; avoid heavy dependencies.

## Language

Both **Chinese (中文)** and **English** are welcome in issues, discussions, and
pull requests.
