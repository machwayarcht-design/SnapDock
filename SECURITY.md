# Security Policy

## Reporting a Vulnerability

We take security seriously. If you discover a vulnerability in SnapDock, please:

- **Preferred:** Open a [GitHub Security Advisory](../../security/advisories/new) for private disclosure.
- **Or:** Email the maintainers (see the GitHub link in the About window) with a clear description, steps to reproduce, and the impact.

We will acknowledge your report within a reasonable time and work with you on a
fix and coordinated disclosure. Credit will be given unless you prefer to remain
anonymous.

## Code Signing Status

SnapDock release binaries can be Authenticode-signed using the provided
`build-and-sign.ps1` pipeline (which invokes `osslsigncode`). **You must supply
your own `.pfx` certificate** — the repository does not include any signing keys.
Unsigned binaries from the Releases page are provided as-is; always verify the
source and build locally if you need a trusted, signed binary.

## Security Audit Welcome

SnapDock is fully open source. We welcome independent security audits and
responsible disclosures. Transparency is a core principle of this project.

## Behavioral Statement

SnapDock performs only the operations required for its legitimate function:

- Enumerates top-level windows (`EnumWindows`) to arrange them.
- Repositions windows via `SetWindowPos`.
- Registers global hotkeys for layout control.
- Writes a single optional autostart registry value under
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\SnapDock` (no admin rights).

It does **not** collect data, communicate over the network, install services, or
modify system files. The app is not malware.
