# Vibe Play privacy policy

*Last updated: 2026-11-24. Applies to the Vibe Play desktop app (macOS and Windows), the agent hooks and the website in this repository.*

**Short version:** Vibe Play has no analytics, no telemetry and works fully **without an account**. Everything it learns about your
coding agents stays on your computer. It connects to the internet only to show you the entertainment site you opened, if you set it
up yourself to send an SMS through Twilio, and, **only if you choose to create an account**, to sign in with Supabase (section
*Network connections*).

## What Vibe Play handles, and where it goes

| Data | Why | Where it goes |
|---|---|---|
| Agent status events: idle / working / waiting / finished, timestamps, session id, agent name, model name, tool name, terminal name | To show the status dot and to open or close the entertainment window | Sent by the hooks to the **bridge on your own computer** (`127.0.0.1:3150`, token protected, loopback only). Kept in the bridge's memory. Not written to disk by Vibe Play, not sent anywhere |
| The first 100 characters of your prompt and, when an agent asks you something, the question | Only to label the session in your own widget and settings | Same as above. These texts are **not** included in the bridge's WebSocket broadcasts and are not sent to any server |
| Your source files, diffs, file contents, API keys | Not needed | **Not read** by Vibe Play |
| Settings (theme, widget corner, platform choice, automation switch) | To remember your choices | A local file (see *Your data on disk*) |
| Sign-ins to Instagram, X or YouTube | So you stay signed in | Stored on disk by the system web view (WebKit on macOS, WebView2 on Windows), exactly like a browser profile. Vibe Play never sees your password |
| Optional SMS (Twilio) | Only if **you** configure it: "Vibe Play: *agent* finished work for session *id*" | Sent to `api.twilio.com` with your Twilio credentials, which you supply. The message contains the agent name, a session id and the duration, **not** your prompt |

## Network connections

1. **The entertainment site you open** (instagram.com, x.com, youtube.com and their login providers). Their own privacy policies
   apply to what you do there; Vibe Play only hosts the page in a window.
2. **api.twilio.com**, only if you configured SMS.
3. **The website** (the marketing site) loads no third-party resources: its fonts are served from the site itself, there are no
   analytics, cookies or trackers.
4. **Supabase (optional, only while signed in or signing in).** Used for sign-in, your profile, your plan and your device list.
   Sent: your email address, the sign-in method, a display name you choose, a random id and name of this computer, the app version
   and platform. **Never sent: your prompts, code, agent activity, files, or the entertainment sites you visit.** Supabase stores
   the account in a database where every user can read only their own rows (Row Level Security). **Delete account** in the Account
   window removes the account and all of its rows; **Sign out** removes the local sign-in.

Nothing else. In particular Vibe Play does not check for updates, report crashes or measure usage. **Only if you create an account
or sign in** (optional, see below), Vibe Play also talks to its Supabase project at `gvuqncuejklrsaaphsrq.supabase.co` over HTTPS,
and, for Google sign-in, your browser talks to Google and Supabase.

## Your data on disk

| What | macOS | Windows |
|---|---|---|
| Settings | `~/Library/Application Support/WaitPlay/settings.json` | `%LOCALAPPDATA%\WaitPlay\settings.json` |
| Web sign-ins and site data | managed by WebKit: `~/Library/WebKit/com.waitplay.app` and `~/Library/HTTPStorages/com.waitplay.app.binarycookies` | `%LOCALAPPDATA%\WaitPlay\webview2` |
| Log (no tokens, no prompts) | `~/Library/Logs/WaitPlay/waitplay.log` | `%LOCALAPPDATA%\WaitPlay\logs` (see `docs/WINDOWS.md`) |
| Bridge token and hand-off files (owner-only) | `$TMPDIR/waitplay/` | `%TEMP%\waitplay\` |
| Launch at login | `~/Library/LaunchAgents/com.waitplay.app.plist` | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `Vibe Play` |
| Hooks Vibe Play added to your agents | `~/.claude/settings.json` (Claude Code), plus the other agents' own settings when you enabled them; a backup `settings.json.waitplay-backup` is left next to it | same, in `%USERPROFILE%` |
| Account sign-in (optional): refresh token | Keychain item `com.waitplay.app/session` | Credential Manager entry `com.waitplay.app/session` (DPAPI protected); never a file |
| Account cache (optional): email, name, plan, device list, last sync time; a random device id | `account.json`, `device.json` next to `settings.json` (no tokens) | the same, in `%LOCALAPPDATA%\WaitPlay` |

## Uninstalling and removing your data

**macOS**
1. In Vibe Play, open *Settings*, and under *Integrations* choose *Remove* for every agent (this takes only Vibe Play's hooks out of
   their configuration files and keeps everything else). Turn off *Launch at login*.
2. Quit Vibe Play from the menu bar item, then drag `WaitPlay.app` from *Applications* to the Trash.
3. Delete the data you no longer want:
   ```sh
   rm -rf ~/Library/Application\ Support/WaitPlay ~/Library/Logs/WaitPlay \
          ~/Library/WebKit/com.waitplay.app ~/Library/HTTPStorages/com.waitplay.app.binarycookies \
          ~/Library/LaunchAgents/com.waitplay.app.plist "${TMPDIR}waitplay"
   security delete-generic-password -s com.waitplay.app -a session
   ```

**Windows**
1. In Vibe Play's settings, remove the agent integrations and turn off *Launch at login* (which deletes the `Run` registry value).
2. Quit Vibe Play from the tray icon and delete the folder you unpacked it to.
3. Delete the data you no longer want (PowerShell):
   ```powershell
   Remove-Item -Recurse -Force "$env:LOCALAPPDATA\WaitPlay", "$env:TEMP\waitplay"
   cmdkey /delete:com.waitplay.app/session
   Remove-ItemProperty -Path HKCU:\Software\Microsoft\Windows\CurrentVersion\Run -Name Vibe Play -ErrorAction SilentlyContinue
   ```

To remove only your web sign-ins, delete the *web sign-ins* folder from the table; the next window starts signed out.

**Vibe Play account (if you created one):** open *Account* (tray menu or Settings) and choose **Delete account** to remove it and all
of its data from the server; **Sign out** only removes this computer's sign-in. If the app is already gone, remove the saved
sign-in with `security delete-generic-password -s com.waitplay.app -a session` (macOS) or `cmdkey /delete:com.waitplay.app/session`
(Windows); to delete the server-side account without the app, ask us through the issue tracker below from the account's email.

## Children

Vibe Play is a developer tool and is not directed at children.

## Changes and contact

Changes to this policy are made in this file and are visible in the repository history. Questions or concerns: open an issue at
<https://github.com/ap5911443-jpg/waitplay-website/issues>. Security problems: see the repository's security policy.
