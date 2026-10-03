# Desktop Packaging and Release Builds

This document describes how PacketSmith is built into installable artifacts,
how the automated release workflow publishes them, and how code signing is
configured. Packaging commands run on the target operating system: macOS builds
use `codesign`, `lipo`, and `hdiutil`; Windows builds use the NSIS compiler.

## Artifacts

| Platform | Artifact | Description |
| --- | --- | --- |
| macOS | `PacketSmith-<version>-macos-universal.dmg` | Universal (Apple Silicon + Intel) DMG containing `PacketSmith.app` and an `/Applications` shortcut |
| Windows | `PacketSmith-<version>-windows-x64-setup.exe` | Per-user NSIS installer with Start menu shortcut and uninstaller |
| Windows | `PacketSmith-<version>-windows-x64.zip` | Portable folder for users who prefer not to install |

A combined `SHA256SUMS.txt` with SHA-256 checksums is published alongside the
artifacts; each build job also uploads per-artifact `.sha256` files as workflow
artifacts.

## Local Packaging

The `package` command is provided by the workspace automation crate:

```bash
cargo xtask package --help
```

macOS:

```bash
# Universal DMG (recommended); builds both targets and merges with lipo.
cargo xtask package --format all --arch universal

# Apple Silicon or Intel only.
cargo xtask package --format all --arch arm64
cargo xtask package --format all --arch x64

# Sign with a Developer ID identity and then package the signed bundle.
cargo xtask package --format app --arch universal --sign "Developer ID Application: Example (TEAMID)"
cargo xtask package --format dmg --arch universal --skip-build

# Build without signing (arm64 binaries will not launch until signed).
cargo xtask package --format app --arch universal --sign none
```

Windows (in a Developer PowerShell with the MSVC toolchain and NSIS installed):

```powershell
cargo xtask package --format all --arch x64
```

Artifacts are written to `target/dist/`. Staged, unsigned build output lives in
`target/dist/macos/` and `target/dist/windows/`. `--dry-run` prints the exact
commands without executing them, and `--debug` packages a debug build. On a
non-target host, pass an explicit `--format` (for example
`--format dmg --dry-run`) to preview another platform's pipeline.

The package command always builds the `packetsmith-app` binary with the
`gpui-ui` feature. Windows release builds additionally force the static C
runtime (`+crt-static`) so no Visual C++ redistributable is required.

## Application Icons

Placeholder monogram icons are committed under `packaging/icons/` as PNG,
ICO, and ICNS files. Regenerate them with the dependency-free generator:

```bash
python3 packaging/icons/generate.py
```

macOS bundles pick up `packaging/icons/AppIcon.icns`; Windows executables embed
`packaging/icons/app-icon.ico` through a build script, and the NSIS installer
uses the same ICO. Replace the generated assets with final brand artwork when it
is available and rerun the generator only if the style changes.

## Release Workflow

`.github/workflows/release.yml` builds installers on native runners:

- **macOS** (`macos-14`, Apple Silicon): builds both Rust targets, merges them
  with `lipo`, ad-hoc signs (or Developer ID signs when configured), creates the
  DMG, optionally notarizes and staples it, and uploads checksums.
- **Windows** (`windows-latest`): builds the static-CRT executable, stages the
  portable folder, signs the executable when configured, builds the NSIS
  installer and zip, signs the installer, and uploads checksums.
- **release**: runs only for `v*` tags, collects both jobs' artifacts, writes
  `SHA256SUMS.txt`, and publishes a GitHub release with generated release notes.
  Tags containing a hyphen (for example `v0.1.0-beta.1`) are marked prereleases.

Trigger a build with a tag push (publishes a release) or manually from the
Actions tab (builds and uploads workflow artifacts only):

```bash
git tag v0.1.0
git push origin v0.1.0
```

### Signing Secrets

Signing is optional and off by default. Unsigned builds are published as
"developer release" artifacts: macOS users must clear the quarantine attribute
and Windows users will see SmartScreen warnings. Configure these repository
secrets to sign:

| Secret | Purpose |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64-encoded Developer ID Application `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` |
| `APPLE_ID` | Apple ID used for notarization |
| `APPLE_APP_PASSWORD` | App-specific password for notarization |
| `APPLE_TEAM_ID` | Apple Developer team identifier |
| `WINDOWS_CERTIFICATE` | Base64-encoded code-signing `.pfx` |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for the `.pfx` |

When the Apple certificate is present, the workflow imports it into a temporary
keychain and signs with hardened runtime, the
`com.apple.security.network.client` entitlement, and a secure timestamp.
Notarization runs only when the certificate, Apple ID, app password, and team
ID are all configured. Windows signing uses `packaging/windows/sign.ps1`, which
signs with SHA-256 and a DigiCert timestamp and then verifies the signature.

## Installed Application Data

Installed builds store application state outside the installation directory:

| Platform | Location |
| --- | --- |
| macOS | `~/Library/Application Support/PacketSmith/` |
| Windows | `%APPDATA%\PacketSmith\` |
| Linux | `$XDG_DATA_HOME/packetsmith/` or `~/.local/share/packetsmith/` |

Set `PACKETSMITH_DATA_DIR` to override the location for development and tests.

## Current Limitations

- macOS builds are ad-hoc signed when no Developer ID is configured; notarized
  releases require the signing secrets above.
- Windows CI ships x64 only. `cargo xtask package --arch arm64` builds an ARM64
  installer manually, but no ARM64 runner is configured yet.
- Installer backgrounds, protocol handlers, file associations, and per-machine
  installs are not implemented. Auto-update is a separate workstream.
- Linux distribution packaging (tarball, AppImage, deb, rpm) is not part of this
  workflow yet.
