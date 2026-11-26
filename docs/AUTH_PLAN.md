# Vibe Play authentication: implementation plan (Supabase)

Status: **plan only, nothing implemented.** Written after inspecting the repository (see section 1). Items marked **DECISION** need an
answer from you; items marked **NEEDS YOU** need a dashboard action or a value only you can provide.

## 1. What exists today (inspected)

| Area | Fact | Consequence |
|---|---|---|
| Desktop apps | Rust workspace: `waitplay-core`, `-bridge`, `-hooks`, `-app`. The tray app (`waitplay-app`) uses `tao` + `wry` (WebView2 on Windows, WKWebView on macOS). Native tray, widget and settings windows. | Auth logic belongs in Rust, owned by the tray app process only. |
| **Desktop UI is not React** | Widget and Settings are single-file **plain HTML/JS pages** (`assets/mac_widget.html`, `settings.html`, `win_widget.html`, `win_settings.html`) talking to Rust over `window.ipc`. React exists only in the marketing website (`src/`) and the legacy dashboard (`apps/web`). | Adding React to the desktop app would add a bundler, a build step and dependencies. **DECISION 1** (recommended: keep the existing plain HTML/JS style for the login and account screens; no new frontend framework). |
| Accounts / backend | None. `grep` finds no Supabase, OAuth, keychain or credential code. The only "token" is the local bridge token (loopback, unrelated to a user identity). | Greenfield; nothing to migrate. Bridge, hooks, agent detection stay untouched. |
| Local storage | `settings.json` (validated field by field), per-user folders, bridge token in an owner-only file. | Settings stay as they are. **No auth token may ever be written to `settings.json`.** |
| Networking | `waitplay-core` uses `ureq` with **no TLS** (loopback only). The bridge has `tokio-rustls` + `webpki-roots` for Twilio. | HTTPS to Supabase needs a TLS client: enable rustls for the auth crate only (ring + webpki-roots are already in the dependency tree). |
| URL schemes | None registered (`Info.plist.in` has no `CFBundleURLTypes`; no Windows protocol handler). The app is unsigned for now. | Prefer an RFC 8252 **loopback redirect** over a custom scheme (section 3). |
| Single instance | Tray app is single-instance (lock file); a second launch asks the first to show Settings. | Exactly one process refreshes tokens. Companion windows and hooks never touch auth. |
| Privacy copy | Website, FAQ and `docs/PRIVACY.md` say "no account, no server of its own". | They become false the moment login ships. Must be rewritten in the same change (section 9). |

## 2. Architecture

```
 tray app (Rust, single instance)                         Supabase (Free plan)
 +-----------------------------------------------+        +--------------------------+
 | waitplay-auth (new crate, no UI)              |  HTTPS | Auth (GoTrue)            |
 |  - gotrue client (REST, ureq + rustls)        |<------>| Postgres + RLS           |
 |  - PKCE + loopback callback listener          |        | Edge Function:           |
 |  - session manager (refresh, expiry, offline) |        |   delete-account         |
 |  - SecretStore trait                          |        +--------------------------+
 |      Windows: Credential Manager (DPAPI)      |
 |      macOS:   Keychain (Security.framework)   |
 +----------------------+------------------------+
                        | window.ipc (typed JSON messages) + events
        login window / Settings > Account  (plain HTML/JS, same visual language)
```

Rules: Rust owns every secret, every network call and all state transitions; the pages only render state pushed by Rust and send
intents (`signInPassword`, `signInGoogle`, `sendCode`, `verifyCode`, `signOut`, ...). Nothing in the pages can read a token.
Local features never import the auth crate: it is optional at runtime and at compile time.

## 3. Sign-in flows (all through Supabase Auth REST; no embedded WebView for Google)

**Google (PKCE, system browser).**
1. Rust generates `code_verifier` (32 random bytes, base64url) and `code_challenge = base64url(SHA-256(verifier))`, a one-time `nonce`
   (16 random bytes) and starts a listener on `127.0.0.1:<random port>` (loopback only, one request, 5 minute timeout).
2. Opens the **default browser** at `<SUPABASE_URL>/auth/v1/authorize?provider=google&redirect_to=http://127.0.0.1:<port>/auth/callback/<nonce>&code_challenge=...&code_challenge_method=s256`.
3. Google, then Supabase, then the browser lands on our loopback URL with `?code=`. The listener checks: method GET, `Host` is
   `127.0.0.1:<port>`, path nonce equals ours (constant-time compare), nonce unused (single use, then the listener closes, which is
   the replay protection), `code` present, no `error`. Anything else gets a plain 400 page and **does not** consume the flow.
4. Rust exchanges the code: `POST /auth/v1/token?grant_type=pkce` with `{auth_code, code_verifier}`. The verifier never leaves memory.
5. The browser tab shows a tiny self-contained "You can return to Vibe Play" page (no external resources, `Referrer-Policy: no-referrer`).

Why loopback and not `waitplay://`: identical on Windows and macOS, needs no registry/plist change, works for unsigned builds, and
the redirect target is not claimable by another app (a custom scheme can be registered by anything). To verify at implementation:
the Supabase allow-list pattern `http://127.0.0.1:*/auth/callback/*` (Supabase documents `*` as "any sequence of non-separator
characters", and `.` and `/` are the separators, so a port should match). If the dashboard rejects it, the fallback is a fixed port.

**Email + password.** `POST /auth/v1/signup`, `POST /auth/v1/token?grant_type=password`. Email verification is required
(Supabase "Confirm email" on).

**Email OTP, verification and password reset: 6-digit codes typed into the app, not links.** Links open in a browser that has no
way back to the app; codes do. Supabase email templates are switched to `{{ .Token }}`. Flows: `POST /auth/v1/otp` then `POST
/auth/v1/verify` (`type=email`); signup confirmation (`type=signup`); recovery (`POST /auth/v1/recover`, verify `type=recovery`, then
`PUT /auth/v1/user` with the new password). Codes expire (Supabase default 1 hour) and are rate limited server-side; the UI adds a
resend cooldown.

**Apple.** Implemented behind a disabled button and a config flag; it cannot be tested until the Apple Developer membership is
active (needs Services ID, Team ID, Key ID and a `.p8` key; Supabase needs a client secret JWT that must be regenerated at most
every 6 months). Not required for direct (non-App-Store) distribution.

**Sign out.** `POST /auth/v1/logout?scope=local`, then wipe the Keychain/Credential Manager item and the cached profile, even if
the request fails (offline sign-out must still sign out locally). "Sign out of other devices" uses `scope=others`.

## 4. Secure session storage

- **Stored in the OS secret store only:** the refresh token (and the current access token + expiry, so a restart does not need an
  immediate network call). One item, service `com.waitplay.app`, account = Supabase user id.
- **Windows:** Credential Manager (`CredWriteW`/`CredReadW`/`CredDeleteW`, `CRED_PERSIST_LOCAL_MACHINE`, which Windows protects with
  DPAPI for the user). Uses the `windows` crate that is already a dependency (one extra feature flag, no new crate).
- **macOS:** Keychain generic password via Security.framework. Plan: the small `security-framework` crate (macOS-only, one new
  dependency) rather than hand-written `unsafe` FFI. **DECISION 2** (alternative: ~120 lines of FFI, no new crate, more `unsafe`).
- **Not secret, may be cached in an app file** (`account.json`, no tokens): user id, email, display name, plan, status, `cached_at`.
  This is what Settings shows offline.
- Never in `localStorage`, `settings.json`, logs, `window.ipc` payloads or crash output. A hygiene test greps for token-shaped
  strings in logs after a scripted run, like the existing bridge token test.
- **Refresh:** a timer in the tray app refreshes the access token ~60 s before expiry. Supabase rotates refresh tokens, so only the
  tray process refreshes (single instance) and writes the new pair to the store **before** discarding the old one. Network failure
  is retried with backoff and never signs the user out; only a definitive `invalid_grant`/401 from the refresh endpoint does
  ("session expired, please sign in again"). Local features are unaffected either way.

## 5. Offline behaviour (hard requirement)

- App start never waits for the network and never requires a session. Agent detection, bridge, widget, entertainment window and
  `settings.json` load exactly as today.
- Signed in + offline: state `signedInOffline` using the cached profile; a small "offline" badge in Account only.
- Signed out: the app works fully; the Account page offers "Sign in". There is no login wall in front of any current feature.
- Cloud features (sync, device list, plan refresh) are the only things that need a session; they degrade to "last synced ...".
- Future Pro entitlements are cached with a grace period (proposed 7 days) so a short outage cannot lock a paying user out.
- Auth code is compiled behind its own crate and a runtime "auth configured" check: a build with no Supabase URL/key simply hides
  the Account page (this keeps development and CI builds working).

## 6. Database (Supabase Postgres, migrations in `supabase/migrations/`)

All tables in `public`, RLS **enabled and forced**; `anon` gets no table privileges; policies use `(select auth.uid())`.

| Table | Purpose | Client access |
|---|---|---|
| `profiles` (`id` = `auth.users.id` on delete cascade, `display_name`, `avatar_url`, `status` active/suspended, timestamps) | profile + account status | select own; update own `display_name`/`avatar_url` only (column grants) |
| `entitlements` (`user_id`, `plan` free/pro, `status`, `current_period_end`, `provider`, `provider_customer_id`, `device_limit`) | plan, device limit, future billing | **select own only**; written only by the service role (future billing webhook) |
| `devices` (`id`, `user_id`, `device_id` random UUID generated locally, `name`, `platform`, `app_version`, `first_seen`, `last_seen`, `revoked_at`) | connected devices | select own; rows created/updated only through `register_device()` |
| `user_preferences` (`user_id` pk, `data jsonb`, `version`, `updated_at`) | future cloud sync | select/insert/update/delete own; size-limited by a check constraint |

Functions: `handle_new_user()` trigger on `auth.users` (SECURITY DEFINER, fixed `search_path`) creates the profile and a `free` entitlement;
`register_device(device_id, name, platform, app_version)` (SECURITY INVOKER with RLS, counts non-revoked devices against
`entitlements.device_limit`, returns `ok` or `limit_reached`); `revoke_device(id)`.
Proposed free limit: 2 devices (**DECISION 3**, changeable later by editing one value).

**Account deletion** cannot use the anon key. Edge Function `delete-account` (Deno, deployed to your project): verifies the caller's
JWT, then calls `auth.admin.deleteUser()` with the service-role key that Supabase injects into the function's environment; foreign
keys cascade every row. The app requires a fresh sign-in (password or OTP) plus a typed confirmation first, then wipes local state.
The service-role key never exists in this repository, the app, CI logs or your messages to me.

**RLS proof:** pgTAP tests (`supabase/tests/`) create two users and assert that user B cannot select, insert, update or delete user
A's rows in every table, that `anon` sees nothing, that clients cannot edit `entitlements`, and that the device limit holds. Run with
`supabase test db` (Docker) locally and in CI.

## 7. UI (follows the existing Vibe Play look: glass panel, system font stack, same switch/segmented/button components)

- **Sign-in window** (one small window, opened from tray menu "Sign in..." and from Settings > Account): Continue with Google, Continue
  with Apple (disabled until configured), Continue with email; email screen toggles Sign in / Create account; password field with
  show/hide; "Email me a code instead"; code screen (6 boxes, resend with cooldown); "Forgot password?" (code, then new password);
  inline errors, spinner states, everything keyboard reachable, `prefers-color-scheme` aware.
- **Settings > Account page** (added to both Settings pages as an additive nav item): name + email, avatar initial, account status,
  plan badge (Free; Upgrade disabled until billing exists), connected devices with Revoke, "Sign out", "Sign out of other devices",
  "Delete account" (confirm sheet). Signed-out state shows a single Sign in button.
- Tray menu gains one line: "Sign in..." or the signed-in email with "Account...".
- Your screenshot did not reach me (no attachment is visible in this conversation). Please send it again; I will match it before any
  UI work starts.

## 8. Work breakdown (each phase ends with green tests; nothing merges red)

| Phase | Content | Tests |
|---|---|---|
| 0 | Your decisions + Supabase dashboard setup (section 10) | n/a |
| 1 | SQL migrations, RLS, functions, `delete-account` function | pgTAP RLS suite, Deno test for the function |
| 2 | `waitplay-auth` crate: config, PKCE, loopback listener, GoTrue client, session manager, `SecretStore` trait with in-memory fake | unit tests (RFC 7636 test vector, state/nonce/replay, malformed callbacks, expiry/refresh/invalid_grant/network-down against a local mock GoTrue server) |
| 3 | Windows Credential Manager and macOS Keychain stores | round-trip tests on the Windows machine here and on the macOS CI runners (Intel and Apple Silicon) |
| 4 | App integration: IPC messages, login window, Settings > Account, tray entries, device registration, offline states | webview smoke test (CDP on Windows as already done; macOS CI smoke), local-features-offline test |
| 5 | Docs: `docs/AUTH_SETUP.md`, privacy policy + website copy + `SECURITY.md` updates | existing hygiene tests extended (no service-role key anywhere, no token in logs, privacy claims) |
| 6 | Real-provider test matrix (section 9) | manual, by you, on real machines |

New dependencies (all justified): rustls TLS for `ureq` in the auth crate (already in the tree via the bridge), `sha2` (PKCE),
`security-framework` (macOS only, or FFI). No new JavaScript dependency. No change to the bridge, hooks or the floating widget.

## 9. Test matrix and the "production ready" rule

| Case | Windows | macOS Intel | macOS Apple Silicon | Who/how |
|---|---|---|---|---|
| Email/password sign up, verify code, sign in, sign out | automated against a **test** Supabase project | CI runner | CI runner (when a runner is available) | automated with a dedicated test user |
| Email OTP, password reset | automated (needs a mailbox the test can read) or manual | manual | manual | see "NEEDS YOU" |
| Token refresh, rotation, expired and revoked sessions | automated (mock GoTrue + test project) | CI | CI | automated |
| Invalid / replayed / wrong-nonce callbacks | automated (loopback listener unit + integration tests) | CI | CI | automated |
| Offline start, offline sign-out, network loss during refresh | automated | CI | CI | automated |
| Multiple launches (second launch must not start a second auth flow or refresh) | automated | CI | CI | automated |
| Secret storage really in Credential Manager / Keychain, not on disk | automated | CI | CI | automated |
| **Google login with a real Google account** | **you, interactively** | **you, on a real Mac** | **you, on a real Mac** | I cannot enter your Google credentials or 2FA; CI cannot do an interactive Google login |
| Apple login | blocked until the membership is active | | | |

I will not describe authentication as production-ready until the Google rows have been run by a human on Windows and on both Mac
types, and the unsigned-app behaviour (Gatekeeper/SmartScreen) has been checked on those machines.

Documentation corrections that ship with the feature: `docs/PRIVACY.md` (a hosted account now exists: what is stored in Supabase,
what is not, how to delete it), website privacy section/FAQ ("no account" claims), `docs/RELEASE_SIGNING.md` unaffected.

## 10. What I need from you (NEEDS YOU)

Send me **only** these (all are public values):
1. `SUPABASE_URL` (looks like `https://<ref>.supabase.co`) and the project ref.
2. The **anon / publishable key**. It is designed to ship inside apps; RLS is what protects the data.

**Never send me** the `service_role` / secret key, your database password, the Google client secret, an Apple `.p8` key, or any
access token. Those go only into the Supabase dashboard (or Edge Function secrets) by you.

Dashboard work (I will provide exact click paths and the SQL in `docs/AUTH_SETUP.md`):
1. Create **two** Supabase projects on the Free plan: `waitplay-dev` (tests, throw-away data) and `waitplay-prod`. Free-plan facts to
   confirm on your dashboard: projects pause after a week of inactivity; the built-in email sender is limited to a handful of
   messages per hour, which is fine for development but **not** for real users' OTP codes.
2. **Authentication > Providers > Email**: enable; "Confirm email" ON; set email OTP length 6.
3. **Authentication > Email Templates**: I will give you templates that show `{{ .Token }}` (the 6-digit code).
4. **Authentication > URL Configuration**: Site URL = your website; add Redirect URL `http://127.0.0.1:*/auth/callback/*`.
5. **Authentication > Providers > Google** (needs a Google Cloud project): OAuth consent screen (app name, support email, privacy
   policy URL; scopes `openid email profile` only), then a **Web application** OAuth client with the authorized redirect URI shown in
   the Supabase Google provider page (`https://<ref>.supabase.co/auth/v1/callback`); paste the client ID and secret **into Supabase**.
   Note: publishing the consent screen to all users and showing your brand needs a domain you own; a github.com URL as homepage is
   usually rejected. Until then the app is in "Testing" mode (up to 100 test users you add by email).
6. **Custom SMTP** before launch (Resend, Brevo, Postmark, ...): a sending domain and DNS records; needed for reliable OTP delivery.
7. Install the Supabase CLI and run the migrations and function deploy yourself with commands I will give you, or tell me you
   prefer to paste the SQL into the SQL editor. I do not need your CLI login.
8. A mailbox I can use for automated tests of email codes (for example a plus-address on a mailbox you control), plus a second
   test user for the RLS tests. Or confirm that you will run those two flows by hand.
9. Later: Apple Developer membership (Apple login and signing), and the billing provider choice (Stripe, Lemon Squeezy or Paddle).

## 11. Decisions I need

1. **Frontend:** keep plain HTML/JS for the desktop login and account screens (recommended), or introduce a TypeScript/React build for the desktop windows (adds a bundler, build step, CI changes and dependencies, and means rebuilding the existing widget/settings style in React).
2. **macOS Keychain:** `security-framework` crate (recommended) or raw FFI.
3. **Free plan device limit:** 2 (proposed) or another number.
4. **Email flows:** 6-digit codes inside the app (recommended; works without deep links) versus magic links (needs the app to catch a browser callback; worse when the link is opened on a phone).
5. **Login optional everywhere** until a paid feature exists (recommended, and required by the offline rule).
