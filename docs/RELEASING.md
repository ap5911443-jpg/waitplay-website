# Releasing Vibe Play

Release files (the version comes from `version` in the workspace `Cargo.toml`; every name below uses it):

| Platform | Files | Built by |
|---|---|---|
| macOS (Apple Silicon) | `WaitPlay-<version>-arm64.dmg` | `.github/workflows/macos.yml` |
| macOS (Intel) | `WaitPlay-<version>-x86_64.dmg` | `.github/workflows/macos.yml` |
| Windows | `WaitPlay-<version>-windows-x64-setup.exe` (installer), `WaitPlay-<version>-windows-x64.zip` (portable) | `.github/workflows/windows.yml` |
| Both | `SHA256SUMS-macos.txt`, `SHA256SUMS-windows.txt`, build-provenance attestations | the `release` jobs |

Without an Apple Developer ID (current state) the DMGs are ad-hoc signed and published as `*-UNSIGNED.dmg` pre-releases; the Windows
files are never code-signed yet. See `docs/RELEASE_SIGNING.md` for the Apple setup.

## Public release (CI, recommended)

```sh
# 1. bump the version once, in Cargo.toml ([workspace.package] version), commit, push, wait for the macOS and Windows workflows to pass
# 2. tag it: the `release` jobs build nothing, they publish what the green builds produced, with checksums and attestations
git tag v0.2.0 && git push website v0.2.0
```

## Build the files yourself

macOS (on a Mac, per architecture; unsigned/ad-hoc unless `APPLE_SIGNING_IDENTITY` is set, see `docs/RELEASE_SIGNING.md`):

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
for t in aarch64-apple-darwin x86_64-apple-darwin; do
  cargo build --release --workspace --target $t
  WAITPLAY_BUILD=$(git rev-parse --short HEAD) bash packaging/macos/build_app.sh $t
done
# -> dist/<target>/WaitPlay-<version>-<arch>.dmg
```

Windows (PowerShell, on Windows):

```powershell
$env:WAITPLAY_BUILD = (git rev-parse --short HEAD)
cargo build --release --workspace
powershell -ExecutionPolicy Bypass -File packaging\windows\build_installer.ps1
# -> dist\windows\WaitPlay-<version>-windows-x64-setup.exe, ...-x64.zip, SHA256SUMS-windows.txt
```

## What users see without signing

- **macOS**: Gatekeeper says the app "cannot be verified"; right-click the app, choose Open, then Open again (first launch only).
- **Windows**: SmartScreen shows "Windows protected your PC / unknown publisher"; choose *More info*, then *Run anyway*.

Verify a download: `shasum -a 256 <file>` / `Get-FileHash <file>` against the `SHA256SUMS-*.txt` of the same release, and for CI-built
files `gh attestation verify <file> --repo ap5911443-jpg/waitplay-website`.

## Updates

Vibe Play has **no automatic updater** today: it makes no network request to check for updates (the *Check for updates* button only
refreshes the page, and says so). Users update by installing the newer DMG or setup file over the old one. If an updater is added
it must be opt-in, use HTTPS only, and verify a signature before installing (Sparkle with EdDSA on macOS; a signed installer on
Windows), and `docs/PRIVACY.md` must be updated at the same time.
