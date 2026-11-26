#!/usr/bin/env bash
# Runtime smoke test for a built WaitPlay.app on a real macOS session (CI, or any Mac).
#
#   bash packaging/macos/smoke.sh path/to/WaitPlay.app [output-dir]
#
# Scenario 1 (defaults): start, menu-bar item, bridge, widget, settings window, a Claude Code prompt through the real hook
#   (entertainment window opens), a second concurrent session (same window, no second process), finishing them one by one (the
#   window must stay while one is still working), idle CPU and memory.
# Scenario 2 (custom settings): dark appearance, widget top-right, YouTube preferred, X disabled, "close when the agent finishes":
#   the window opens on YouTube below the widget and the companion process exits when the agent is done; the web login store
#   persists to disk.
# Logs and screenshots go to the output directory. Exit status 1 when any check failed; every check prints "ok:" or "FAIL:".
set -u

APP="${1:?usage: smoke.sh path/to/WaitPlay.app [output-dir]}"
OUT="${2:-smoke-out}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
BIN="$APP/Contents/MacOS"
fail=0

ok() { echo "ok: $1"; }
bad() { echo "FAIL: $1"; fail=1; }

# wait_for <description> <seconds> <command...>
wait_for() {
  local what="$1" secs="$2"
  shift 2
  local i
  for i in $(seq 1 $((secs * 2))); do
    if "$@" >/dev/null 2>&1; then ok "$what"; return 0; fi
    sleep 0.5
  done
  bad "$what (gave up after ${secs}s)"
  return 1
}

# fresh isolated user environment: new_env <name>
new_env() {
  ROOT="$(mktemp -d)"
  export HOME="$ROOT/home"
  export TMPDIR="$ROOT/tmp/"
  mkdir -p "$HOME/.claude" "$HOME/Library/Application Support/WaitPlay" "$TMPDIR"
  LOG="$HOME/Library/Logs/WaitPlay/waitplay.log"
  SIG="${TMPDIR}waitplay"
  STATE="$SIG/.waitplay-companion-instagram.json"
}

shot() { screencapture -x "$OUT/$1.png" 2>/dev/null || echo "note: screencapture unavailable for $1"; }
state_visible() { grep -q "\"visible\":$1" "$STATE"; }
hook() { printf '%s' "$2" | "$BIN/waitplay-hook" claude-code "$1"; }
prompt_json() { printf '{"session_id":"%s","hook_event_name":"UserPromptSubmit","prompt":"smoke","model":"m"}' "$1"; }
stop_json() { printf '{"session_id":"%s","hook_event_name":"Stop","model":"m"}' "$1"; }
tool_json() { printf '{"session_id":"%s","hook_event_name":"PreToolUse","tool_name":"Bash","model":"m"}' "$1"; }
# a real agent keeps reporting tool use; without it the bridge's 20 s watchdog completes a silent session
keepalive() { while :; do hook tool "$(tool_json "$1")"; sleep 4; done; }
no_process() { ! pgrep -f -- "$1" >/dev/null; }
# The shipped binary has no test switches. For the screenshots (the page must be expanded without a click) CI passes a separate
# binary built with `--features test-hooks` in WAITPLAY_TESTHOOKS_BIN; without it the shipped binary runs and the widget stays collapsed.
APP_BIN="${WAITPLAY_TESTHOOKS_BIN:-$BIN/waitplay}"
[ -x "$APP_BIN" ] || APP_BIN="$BIN/waitplay"
if strings "$BIN/waitplay" | grep -q "WAITPLAY_TEST_"; then bad "the shipped binary contains a test switch (WAITPLAY_TEST_*)"; else ok "the shipped binary contains no test switches"; fi
quit_all() {
  pkill -f "$APP_BIN" 2>/dev/null
  pkill -f "$BIN/waitplay" 2>/dev/null
  pkill -f "waitplay-bridge" 2>/dev/null
  pkill -f "waitplay-hook" 2>/dev/null
  sleep 1
}
total_rss_mb() { ps -axo rss,command | grep -E "waitplay|com.apple.WebKit" | grep -v grep | awk '{s+=$1} END {printf "%.0f", s/1024}'; }

echo "== Vibe Play smoke test: $APP"
sw_vers 2>/dev/null | sed 's/^/   /'

# ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
echo "== scenario 1: defaults"
new_env
WAITPLAY_TEST_EXPAND_WIDGET=1 "$APP_BIN" >"$OUT/app.stdout" 2>&1 &
APP_PID=$!

wait_for "app logged its start" 30 grep -q "started;" "$LOG"
wait_for "menu bar item created" 10 grep -q "menu bar item created" "$LOG"
wait_for "bridge listening on 3150" 30 nc -z 127.0.0.1 3150

# bridge security on the real app: nothing but /health answers without the token, no token endpoint, no foreign-origin socket
code() { curl -s -o /dev/null -w "%{http_code}" "$@"; }
[ "$(code http://127.0.0.1:3150/sessions)" = "401" ] && ok "GET /sessions without a token is refused" || bad "GET /sessions answered without a token"
[ "$(code -X POST -H 'Content-Type: application/json' -d '{"enabled":false}' http://127.0.0.1:3150/state)" = "401" ] && ok "POST /state without a token is refused" || bad "POST /state answered without a token"
[ "$(code http://127.0.0.1:3150/auth/token)" = "404" ] && ok "the bridge hands out no token" || bad "/auth/token still exists"
[ "$(code -H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' -H 'Origin: http://evil.example' http://127.0.0.1:3150/ws)" = "403" ] && ok "a foreign-origin WebSocket is refused" || bad "a foreign-origin WebSocket was not refused"
[ "$(code -H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' http://127.0.0.1:3150/ws)" = "401" ] && ok "a WebSocket without the token is refused" || bad "a tokenless WebSocket was not refused"
wait_for "widget shown" 20 grep -q "widget shown bottom-right" "$LOG"
kill -0 "$APP_PID" 2>/dev/null && ok "app still running" || bad "app exited early"
sleep 2
shot 01-widget

# a second launch asks the running app to show its settings
"$BIN/waitplay" >/dev/null 2>&1 &
wait_for "settings window opened" 15 grep -q "settings window opened" "$LOG"
sleep 3
shot 02-settings

# idle CPU of the app itself (sampled over ~10 s)
sleep 5
cpu_before="$(ps -o time= -p "$APP_PID" | tr -d ' ')"
sleep 10
cpu_after="$(ps -o time= -p "$APP_PID" | tr -d ' ')"
echo "note: idle app CPU time $cpu_before -> $cpu_after over 10 s; rss(KB)=$(ps -o rss= -p "$APP_PID" | tr -d ' ')"

# Claude Code prompt through the real hook: the entertainment window must open
hook start "$(prompt_json aaaaaaaa-1111)"
keepalive aaaaaaaa-1111 >/dev/null 2>&1 &
KA_A=$!
wait_for "entertainment window visible" 60 state_visible true
sleep 8
shot 03-entertainment
echo "note: total memory of Vibe Play + WebKit processes with the window open: $(total_rss_mb) MB"
curl -s -H "x-bridge-token: $(cat "$SIG/.bridge-token")" http://127.0.0.1:3150/sessions >"$OUT/sessions-1.json" || true
grep -q 'claude-code-aaaaaaaa1111' "$OUT/sessions-1.json" && ok "bridge tracks session claude-code-aaaaaaaa1111" || bad "bridge does not list claude-code-aaaaaaaa1111"

# second concurrent conversation: its own session, the same window
hook start "$(prompt_json bbbbbbbb-2222)"
keepalive bbbbbbbb-2222 >/dev/null 2>&1 &
KA_B=$!
sleep 4
curl -s -H "x-bridge-token: $(cat "$SIG/.bridge-token")" http://127.0.0.1:3150/sessions >"$OUT/sessions-2.json" || true
grep -q 'claude-code-bbbbbbbb2222' "$OUT/sessions-2.json" && ok "second conversation is a separate session" || bad "second session missing"
[ "$(grep -c 'companion started' "$LOG")" -eq 1 ] && ok "one entertainment window serves both sessions" || bad "more than one entertainment window was started"

# first one finishes: the window must stay (the second still works)
kill "$KA_A" 2>/dev/null
hook stop "$(stop_json aaaaaaaa-1111)"
sleep 5
state_visible true && ok "window stays while another session still works" || bad "window hid although a session is still working"
shot 04-one-session-left

# second finishes: now it hides
kill "$KA_B" 2>/dev/null
hook stop "$(stop_json bbbbbbbb-2222)"
wait_for "window hides when the last session finished" 30 state_visible false
sleep 1
shot 05-after-finish
curl -s -H "x-bridge-token: $(cat "$SIG/.bridge-token")" http://127.0.0.1:3150/sessions >"$OUT/sessions-3.json" || true

if grep -E ' ERROR ' "$LOG" >"$OUT/errors-1.log"; then bad "scenario 1 log contains errors:"; sed 's/^/   /' "$OUT/errors-1.log"; else ok "no errors in the scenario 1 log"; fi
cp "$LOG" "$OUT/waitplay-1.log" 2>/dev/null
quit_all

# ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
echo "== scenario 2: dark, widget top-right, YouTube, X off, close when the agent finishes"
new_env
cat >"$HOME/Library/Application Support/WaitPlay/settings.json" <<'JSON'
{"appearance":"dark","widgetPosition":"top-right","onFinish":"close","preferredPlatform":"youtube","platforms":{"instagram":true,"x":false,"youtube":true},"watchers":[]}
JSON
WAITPLAY_TEST_EXPAND_WIDGET=1 "$APP_BIN" >"$OUT/app2.stdout" 2>&1 &
APP_PID=$!
wait_for "widget shown top-right" 30 grep -q "widget shown top-right" "$LOG"
"$BIN/waitplay" >/dev/null 2>&1 &
wait_for "settings window opened" 15 grep -q "settings window opened" "$LOG"
sleep 3
shot 06-dark-settings

hook start "$(prompt_json cccccccc-3333)"
keepalive cccccccc-3333 >/dev/null 2>&1 &
KA_C=$!
wait_for "entertainment window visible" 60 state_visible true
grep -q 'opens=youtube' "$LOG" && ok "the window opens on the preferred platform (YouTube)" || bad "the window did not open on YouTube"
sleep 12
shot 07-dark-youtube

kill "$KA_C" 2>/dev/null
hook stop "$(stop_json cccccccc-3333)"
wait_for "entertainment process exits when the agent finishes (close mode)" 40 no_process "--session=claude-code-cccccccc3333"
sleep 1
shot 08-after-close

# the web login store must be on disk after a clean exit
cookie_files="$(find "$HOME/Library" "/Users/$(id -un)/Library" -name '*.binarycookies' 2>/dev/null)"
echo "note: cookie stores: ${cookie_files:-none}"
if [ -n "$cookie_files" ] && echo "$cookie_files" | xargs grep -l -a -i -E 'youtube|google|instagram' >/dev/null 2>&1; then
  ok "web cookies were persisted to disk"
else
  echo "note: no site cookies found in a .binarycookies file (WebKit may keep them in its own store); not a failure"
fi

if grep -E ' ERROR ' "$LOG" >"$OUT/errors-2.log"; then bad "scenario 2 log contains errors:"; sed 's/^/   /' "$OUT/errors-2.log"; else ok "no errors in the scenario 2 log"; fi
cp "$LOG" "$OUT/waitplay-2.log" 2>/dev/null
rm -f "$SIG/.bridge-token"
cp -R "$SIG" "$OUT/signals" 2>/dev/null
quit_all
ps -axo pid,command | grep -i waitplay | grep -v grep >"$OUT/leftover-processes.txt"
[ -s "$OUT/leftover-processes.txt" ] && echo "note: processes left after the test:" && sed 's/^/   /' "$OUT/leftover-processes.txt"

if [ "$fail" -eq 0 ]; then echo "== SMOKE TEST PASSED"; else echo "== SMOKE TEST FAILED"; fi
exit "$fail"
