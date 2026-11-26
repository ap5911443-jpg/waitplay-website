# Security policy

## Reporting a vulnerability

Please report security problems privately through GitHub: **Security, then Report a vulnerability** on
<https://github.com/ap5911443-jpg/waitplay-website/security/advisories/new>. Do not open a public issue for a vulnerability.
Include the version (the About line in Settings), the platform, and the steps to reproduce.

## Scope and design

Vibe Play's bridge listens on `127.0.0.1` only; every route except `/health` needs a per-user token, WebSocket connections must
come from a local origin and present the token, and prompts are never broadcast. See `docs/ARCHITECTURE.md` for the model and
`docs/PRIVACY.md` for what data is handled. Release signing and notarization are described in `docs/RELEASE_SIGNING.md`.

## Supported versions

Only the latest release receives security fixes.
