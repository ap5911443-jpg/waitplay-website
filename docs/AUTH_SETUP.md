# Account sign-in: Supabase setup and operations

Project: `https://gvuqncuejklrsaaphsrq.supabase.co` (Free plan). The app talks to it with the **publishable key only**
(`sb_publishable_...`), which is meant to be public and ships inside the app; Row Level Security protects the data. The
`service_role` / `sb_secret_...` key must never be put in the app, in this repository, in CI variables used by builds, or in chat.

## 1. What is in the repository

| Path | What |
|---|---|
| `crates/waitplay-auth/` | Rust: PKCE, loopback callback, Supabase REST client, OS secret store (Windows Credential Manager, macOS Keychain), session manager, account service. 38 automated tests + 2 opt-in live checks. |
| `crates/waitplay-app/src/account_ui.rs`, `assets/account.html` | The sign-in / account window, shared by Windows and macOS (plain HTML/JS like the other Vibe Play windows). Opened from the tray menu (**Account...**) and **Settings > General > Account**. |
| `supabase/migrations/20260101000000_auth_foundation.sql` | Tables, Row Level Security, triggers, `register_device()` / `revoke_device()`. |
| `supabase/tests/rls.test.sql` | pgTAP: 34 assertions that user B cannot see or change user A's data, `anon` sees nothing, clients cannot edit their plan or status, the device limit holds. |
| `supabase/functions/delete-account/` | Edge Function (Deno) that deletes the caller's account using the service-role key that Supabase injects server-side. |

Build-time configuration (both values are public): `WAITPLAY_SUPABASE_URL` and `WAITPLAY_SUPABASE_ANON_KEY`. If unset, the values of
the Vibe Play project above are compiled in. A build whose key starts with `sb_secret_` or contains `service_role` is refused at runtime.

## 2. One-time dashboard setup (you)

1. **SQL.** Dashboard > SQL Editor > New query: paste `supabase/migrations/20260101000000_auth_foundation.sql` > Run. (Or `supabase link --project-ref gvuqncuejklrsaaphsrq && supabase db push`.) It is safe to run twice.
2. **Authentication > Providers > Email**: enabled. Turn **Confirm email ON**. Set **Email OTP length** to 6. Minimum password length 8.
3. **Authentication > Email Templates**: replace the body of these three templates so the user gets a code instead of a link:
   - *Confirm signup* and *Magic link*:
     `<h2>Your Vibe Play code</h2><p>Enter this code in Vibe Play:</p><p style="font-size:28px;letter-spacing:6px"><b>{{ .Token }}</b></p><p>It expires in one hour. If you did not ask for it, ignore this email.</p>`
   - *Reset password*: the same text with "to reset your password".
4. **Authentication > URL Configuration**: *Site URL* = your website; add to *Redirect URLs*: `http://127.0.0.1:*/auth/callback/*` (Supabase's `*` matches any characters except `.` and `/`, which covers the random port; if the dashboard rejects the pattern, tell me and I will switch to a fixed port).
5. **Authentication > Providers > Google**: enable it; enter the Client ID and Client Secret **in the dashboard** (see section 3).
6. **Edge Function**: `supabase functions deploy delete-account --project-ref gvuqncuejklrsaaphsrq` (the function's JWT check is on). Without it, "Delete account" shows an error and nothing is deleted.
7. **Before real users:** Authentication > SMTP Settings: configure a custom SMTP sender. The built-in sender is limited to a handful of emails per hour, which is too low for sign-in codes.
8. **Free-plan facts to keep in mind** (check the current numbers on your dashboard): inactive projects are paused after about a week; limits on monthly active users and database size apply.

## 3. Google

1. console.cloud.google.com > create a project > *APIs & Services > OAuth consent screen*: External, app name **Vibe Play**, your support email, privacy policy link (`https://github.com/ap5911443-jpg/waitplay-website/blob/main/docs/PRIVACY.md`), scopes **openid, email, profile** only. Leave it in *Testing* while you test (up to 100 test users you add by email); publishing and showing your brand to everyone needs a verified domain you own.
2. *Credentials > Create credentials > OAuth client ID* > type **Web application** > *Authorized redirect URI*: `https://gvuqncuejklrsaaphsrq.supabase.co/auth/v1/callback`.
3. Copy the Client ID and Client Secret into Supabase (Providers > Google). **Do not send the secret to anyone.**

Apple: implemented behind a disabled button; it needs the paid Apple Developer membership (Services ID, Team ID, Key ID and a `.p8` key).

## 4. How it behaves

- **Local features never depend on login.** Agent detection, the bridge, widget and entertainment window start and run exactly as before, signed in or not, online or not. Auth runs on its own threads and its failures are shown only in the Account window.
- **Google**: the system browser opens the Supabase authorize URL (PKCE `S256`, `redirect_to` = a one-shot `http://127.0.0.1:<random port>/auth/callback/<nonce>` listener). A callback with a wrong nonce or host is refused and does not end the flow; the first valid one ends it and closes the port (replay finds nothing). No embedded web view is used.
- **Email**: password sign-in; sign-up, sign-in codes and password reset all use 6-digit codes typed into the app.
- **Session**: the refresh token is stored in Credential Manager / Keychain only (`com.waitplay.app/session`); the access token lives in memory; a small non-secret `account.json` (name, email, plan, devices) is cached for offline display. Refresh happens about a minute before expiry; offline or a 5xx never signs you out; only a refused refresh token does.
- **Devices**: each installation has a random id (`device.json`); the free plan allows 2 devices (change `entitlements.device_limit`). "Remove" marks a device revoked: that device signs itself out at its next background check (cooperative). **Sign out of other devices** invalidates every other session server-side immediately.
- **Delete account**: calls the Edge Function `delete-account` (deployed on the project, verified live); the user is identified only from the verified access token; every row cascades; local session and cache are wiped only after the server confirmed. Any failure shows "not deleted" and changes nothing.

## 4b. Testing email flows with Supabase's built-in mail service (no custom domain yet)

Until Vibe Play has its own domain and an SMTP provider (planned: Resend), the project uses Supabase's built-in sender. Its documented
restrictions: it **only delivers to addresses that are members of your Supabase organization** (anything else fails with *Email
address not authorized*), it is limited to about **2 emails per hour**, and the limit can change. Vibe Play shows plain explanations for
both (see `crates/waitplay-auth/src/error.rs`). Do not put a made-up sender address or an SMTP provider in the dashboard.

| Flow | How to test it today |
|---|---|
| Google sign-in | no email involved; works for any Google account while the OAuth consent screen is in *Testing* (add the test users there) |
| Password sign-in, sign-out, expired session, devices, delete | create a user in the dashboard: *Authentication > Users > Add user > Create new user*, tick **Auto Confirm User**. No email is sent. |
| Sign-up confirmation code, sign-in code, password reset | each costs one email, 2 per hour in total, and the address must be a team member (Organization settings > Team > Invite a second address you own). Use an address that does **not** already have a Google account in this project, or sign-up will collide with the existing identity. |
| "Email address not authorized" / "too many emails" messages | appear automatically when you try another address or a third email within the hour; they are covered by tests |

After the domain and Resend SMTP are configured, none of this changes in the app: the same 6-digit codes are sent to anyone.

## 5. Running the tests

```sh
cargo test -p waitplay-auth                                   # 38 tests, mock Supabase, real OS secret store
cargo test -p waitplay-auth --test live -- --ignored          # read-only checks against the real project (no email sent)
supabase test db                                              # pgTAP RLS suite (needs Docker)
deno test supabase/functions/delete-account                   # Edge Function logic
```

## 6. Verified and not verified (honest status)

See `docs/AUTH_STATUS.md`.
