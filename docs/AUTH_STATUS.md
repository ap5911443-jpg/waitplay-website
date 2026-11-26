# Account sign-in: what is verified and what is not

Written from what was actually executed. Nothing here claims authentication is production ready.

## Verified (executed)

| Area | How |
|---|---|
| PKCE (RFC 7636 vector), base64url, constant-time compare, URL coding | unit tests (`cargo test -p waitplay-auth`) |
| Loopback callback: wrong nonce, wrong/rebinding Host, wrong method/path, empty or oversized code, request flood, cancel, timeout, **replay finds a closed port** | 5 unit tests + 3 end-to-end tests over real TCP |
| Password sign-in, sign-up with emailed code, sign-in/recovery codes, password reset, rate-limit mapping | 8 flow tests against a local mock of Supabase |
| Google flow end to end against the mock: authorize URL has `S256` challenge and no key, hostile request ignored, code exchange only succeeds when the verifier hashes to the challenge, cancelled login leaves the user signed out, bad code is rejected | flow tests |
| Refresh before expiry, **refresh-token rotation persisted before the old token is dropped**, 5xx/offline keeps the session, dead token signs out with a reason and wipes the store, garbage in the store is discarded | flow tests |
| Offline start: signed in from the cached profile with **zero network calls**; second launch sees the first launch's session | flow tests |
| Sign-out clears local state even with the server unreachable; account deletion wipes everything locally and a failed deletion keeps the user signed in; another account's cache is never shown | flow tests |
| Tokens never in `Debug` output, the view sent to the page, or `account.json` | flow test |
| Real **Windows Credential Manager** round trip (write, overwrite, read 400+ bytes, delete) | unit test on this PC |
| Real HTTPS (rustls) to the Vibe Play Supabase project: settings, wrong credentials -> "Wrong email or password.", unknown refresh token -> session expired; anonymous REST reads are refused | `cargo test -p waitplay-auth --test live -- --ignored` (found and fixed a real bug: a malformed refresh token returns `validation_failed`, now treated as expired) |
| Account window inside the real Windows app: Settings > Account... opens it; the email screen sends a wrong password to the **live** project and shows the server's answer; local validation works | driven through WebView2 (CDP) |
| Database: migration applies twice cleanly; **34 pgTAP assertions pass on Postgres 17** (user B cannot read/update/delete user A's rows, anon sees nothing, plan/status cannot be edited, device limit and revoke rules) | run in a local Docker Postgres with stand-ins for Supabase's `auth` schema (the suite found a real bug in `register_device`, fixed) |
| The same migration and the 34 pgTAP assertions on a **real local Supabase stack** (`supabase start`, `supabase test db`), and the Edge Function's Deno tests | GitHub Actions workflow `Supabase`, green on the commit that added them |
| macOS (Intel runner): the whole workspace test suite including the **real Keychain round trip**, the account window code compiling, and the runtime smoke test of the app | GitHub Actions `macOS` x86_64 job, green |
| No privileged Supabase key or JWT anywhere in the repository; config refuses `sb_secret_`/`service_role` keys; privacy claims match the optional account | repo hygiene tests |
| **Google OAuth on Windows against the real project** (Google provider enabled in your dashboard): PKCE authorize in the system browser, return through `http://127.0.0.1:<port>/auth/callback/<nonce>` (so the allow-list pattern works), code exchange, profile (name, email, "Signed in with Google", verified), free plan, **this device registered** in Supabase | run by you and me together on this PC |
| Session storage: **Windows Credential Manager** item `com.waitplay.app/session` (87 bytes, DPAPI, local-machine persistence) holding only `v`, `user_id` and the refresh token; the refresh token is in **no file** under the app's data folder and **not in the log**; `account.json` has no tokens | inspected after the live login |
| Relaunch: signed in at once from the stored token, background refresh **rotated** the refresh token | live |
| Sign out: credential and `account.json` removed. Expired session: an old, already-rotated token restored into Credential Manager is refused by Supabase, the app shows "Your session expired. Please sign in again.", and the dead token is deleted | live |
| In-app **Cancel** of a Google sign-in: the loopback listener port (opened for the flow) is closed again and the window shows "Sign-in cancelled." | live |
| The anonymous key can read **nothing** from `profiles`, `entitlements`, `devices`, `user_preferences` on your project (`42501 permission denied`): the migration is applied and row level security is active | live REST check |
| **`delete-account` deployed** (function `delete-account`, ACTIVE, `verify_jwt` on) and tested live: no token, publishable key as bearer, garbage bearer, wrong auth scheme, bad JSON and client-supplied `user_id` without a token all answered **401**; GET **405**; a valid user token with no/false/string `confirm` answered **400** and deleted nothing | live probes |
| **Account deletion end to end with a disposable user** (created for testing, email/password): signed in through the app (this also verified email+password sign-in live), the confirm sheet keeps *Delete my account* disabled until the exact email is typed, deletion returned the app to the sign-in screen, the Credential Manager item, `account.json` and `device.json` were removed, and the server then refuses that user (`invalid_credentials`) | live, through the real Windows app |
| Every refusal of the deletion function (400, 401, 403, 404, 429, 500, 502) shows a "not deleted" style message and leaves the session and local data intact | automated test |

## Not verified yet

| Item | Why |
|---|---|
| **Google login on macOS Intel and macOS Apple Silicon** | not run: needs a real Mac and you signing in. The macOS Keychain round trip and the app compile in CI; nothing has been used by a human on a Mac. |
| Email flows with real emails (sign-up confirmation code, sign-in code, password reset) | not run. Built-in Supabase mail only reaches the project team's addresses at about 2 emails/hour (see `docs/AUTH_SETUP.md`, "Testing email flows"); the templates must show `{{ .Token }}`. |
| Cross-user deletion proof on the live project | the function never reads a user id from the request (code review + unit test), so it can only delete the token's owner; the live test used one disposable user, so another real account being untouched was not demonstrated against a second live user. |
| **Offline behaviour on a real disconnected machine** | not run (needs the network adapter switched off, which needs administrator rights I do not have). Covered by tests: signed-in start makes no network call, a dead server keeps the session, local features never touch the auth crate. |
| Provider-side cancellation (pressing *Cancel* on Google's consent screen) | the in-app Cancel was tested live; the "access_denied" redirect is covered by unit tests only. |
| Apple sign-in | button disabled; needs the paid Apple Developer membership |
| Accessibility audit, localisation, a screen-reader pass | not done |
| Keychain prompts for **ad-hoc signed** macOS builds: the app's identity changes with each build, so macOS may ask to allow keychain access after an update | expected; disappears with Developer ID signing |

## Known limits

- **Device removal is cooperative**: a removed device signs itself out at its next background check. **Sign out of other devices** is the immediate, server-side kill.
- Account deletion asks the user to type their email (no fresh password/OTP re-authentication yet).
- Plan/billing is read-only: there is no payment provider yet; every account is Free.
