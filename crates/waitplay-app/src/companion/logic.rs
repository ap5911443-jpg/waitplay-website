//! When the companion window is shown, hidden or parked. Pure state machine: no windows, no I/O, no clocks of its own, so every
//! rule here is unit tested. A port of the session registry and `CheckAgentStatusCore` of the C# `MainWindow`.
//!
//! One companion process serves every terminal. Each prompt registers its session together with the terminal window (and tab)
//! it came from. The window is shown only while the focused terminal tab belongs to a session that is still working, hidden
//! while the user is elsewhere, and parked (hidden, frozen, RAM handed back) as soon as no session is left working.

use crate::platform::{WindowId, NO_WINDOW};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Grace period for peeking at another app and coming straight back without the window flickering. Short on purpose: showing and
/// hiding a warm window is cheap, so a peek costs one quick hide and show instead of half a second of the window lingering.
pub const AWAY_GRACE: Duration = Duration::from_millis(40);

#[derive(Clone, Debug, PartialEq)]
pub struct AgentSession {
    pub id: String,
    pub terminal: String,
    /// The terminal's top-level window.
    pub hwnd: WindowId,
    /// Latest poll: `Some(true)` own tab in front, `Some(false)` another tab, `None` unknown (no tab strip yet).
    pub own_tab: Option<bool>,
    /// The turn ended, or the user stepped aside for this prompt.
    pub finished: bool,
    /// Launch time; close signals older than this belong to earlier turns.
    pub started_at: i64,
    /// X pressed for this prompt: stays hidden until the next launch.
    pub dismissed: bool,
    /// The agent asked the user a question and waits for the answer: the window is out of the way until it is answered.
    pub waiting: bool,
}

/// A request to show the companion, from a hook launch or from the widget button.
#[derive(Clone, Debug, PartialEq)]
pub struct LaunchRequest {
    pub session_id: String,
    pub terminal: String,
    pub ts: i64,
    pub hwnd: WindowId,
}

/// What a bridge session says about a session we know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Working { agent: String, model: String, tool: String },
    Waiting { agent: String, prompt: String },
}

/// Effects the window layer performs, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Hide the window (muted, media paused), then deep-park it once it has settled.
    HideWhileAway,
    /// Already hidden mid-turn, now parked for good: deepen the park sooner.
    DeepenPark,
    /// Show the window; `activate` also takes the keyboard focus. Resumes media and wakes the page.
    Show { activate: bool },
    /// Already visible: just bring it to the front (a widget press while the window is up).
    Raise,
    /// A minimized window has to come back for a launch.
    Restore,
    FocusTerminal(WindowId),
    /// Scrolling or typing in this terminal now steps aside (NO_WINDOW disarms).
    SetInputTarget(WindowId),
    /// The session whose tab is in front changed: point the header at it and keep clear of its tab row.
    ShowSession { id: String, terminal: String, agent: String, hwnd: WindowId },
    SetPollRate,
}

pub trait Env {
    fn now(&self) -> Instant;
    fn foreground(&self) -> WindowId;
    fn is_waitplay_window(&mut self, id: WindowId) -> bool;
    /// The window is minimized (iconic).
    fn is_minimized(&self, id: WindowId) -> bool;
}

pub struct Core {
    pub sessions: HashMap<String, AgentSession>,
    pub owner_id: Option<String>,
    /// Opened by the user (widget button): stays while the window they were in, or this one, is in front.
    pub manual: bool,
    /// Hidden while the user is in another tab/app.
    pub hidden_away: bool,
    /// The "nothing left to wait for" park was already applied for this idle spell.
    pub parked_noted: bool,
    /// Set one second after start so the window can appear before visibility rules apply.
    pub auto_close_armed: bool,
    /// Widget-launched windows (no session) follow the window that was in front when they opened.
    pub desktop_parent: WindowId,
    pub away_since: Option<Instant>,
    /// Terminal of the session that finished last: the keyboard goes back there if we hold it.
    pub last_finished_hwnd: WindowId,
    pub my_hwnd: WindowId,
    /// The ON/OFF switch, refreshed by the poller thread (never read from disk on the UI thread).
    pub enabled: bool,
}

fn key(id: &str) -> String {
    id.to_lowercase()
}

impl Core {
    pub fn new(desktop_parent: WindowId) -> Core {
        Core {
            sessions: HashMap::new(),
            owner_id: None,
            manual: false,
            hidden_away: false,
            parked_noted: false,
            auto_close_armed: false,
            desktop_parent,
            away_since: None,
            last_finished_hwnd: NO_WINDOW,
            my_hwnd: NO_WINDOW,
            enabled: true,
        }
    }

    /// A session whose prompt is still running and was not dismissed.
    pub fn any_wanted(&self) -> bool {
        self.sessions.values().any(|s| !s.finished && !s.dismissed)
    }

    /// Hidden with nothing to come back for: polls slowly and skips the bridge.
    pub fn is_parked(&self) -> bool {
        self.hidden_away && !self.manual && !self.any_wanted()
    }

    fn away_for(&mut self, env: &dyn Env, grace: Duration) -> bool {
        let now = env.now();
        let since = *self.away_since.get_or_insert(now);
        now.saturating_duration_since(since) >= grace
    }

    /// The unfinished session living in the focused terminal window's selected tab. Unknown tab state (non-tabbed terminal, or
    /// capture still running) counts as a match; a known own-tab beats an unknown one; the newest prompt wins a tie.
    pub fn owner_of(&self, fg: WindowId) -> Option<&AgentSession> {
        let mut best: Option<&AgentSession> = None;
        for s in self.sessions.values() {
            if s.finished || s.dismissed || s.waiting || s.hwnd != fg || s.own_tab == Some(false) {
                continue;
            }
            best = match best {
                None => Some(s),
                Some(b) => {
                    let s_wins = (s.own_tab == Some(true) && b.own_tab != Some(true)) || (s.own_tab == b.own_tab && s.started_at > b.started_at);
                    if s_wins {
                        Some(s)
                    } else {
                        Some(b)
                    }
                }
            };
        }
        best
    }

    // ── the periodic rule set (every 100ms, 500ms while parked) ────────────────────────────────────────────────────────

    pub fn tick(&mut self, env: &mut dyn Env, agent_name_of: &dyn Fn(&str) -> String) -> Vec<Action> {
        let mut out = Vec::new();
        if !self.enabled {
            // The user controls the window while Vibe Play is paused
            return out;
        }
        if !self.auto_close_armed {
            return out;
        }

        // Opened by the user (widget button): no owning tab. It stays while the window they were in, or this one, is in front;
        // leaving parks it. A parked one waits for the next press, or for a prompt when sessions exist.
        if self.manual || self.sessions.is_empty() {
            if self.hidden_away {
                return out;
            }
            let dfg = env.foreground();
            if dfg == NO_WINDOW || dfg == self.my_hwnd || env.is_waitplay_window(dfg) || dfg == self.desktop_parent {
                self.away_since = None;
                return out;
            }
            if self.manual && self.owner_of(dfg).is_some() {
                // went to a terminal that is being waited on: session rules take over
                self.manual = false;
                self.away_since = None;
            } else {
                if self.away_for(env, AWAY_GRACE) {
                    self.park(&mut out);
                }
                return out;
            }
        }

        // Every terminal's agent has finished (or was dismissed): nothing left to wait for. Park once; the process, the page and
        // the playback position stay for the next prompt.
        if !self.any_wanted() {
            if !self.parked_noted {
                self.parked_noted = true;
                self.park_finished(env, &mut out);
            }
            return out;
        }

        let fg = env.foreground();
        // The terminal that owns the window was minimized: that is "away" no matter which window Windows hands the focus to next
        // (often this very window or the widget, which would otherwise read as "the user is here"). It comes back by itself when
        // the terminal is restored and in front again, as long as the agent is still working.
        if let Some(term) = self.owner_id.as_ref().and_then(|id| self.sessions.get(&key(id))).map(|s| s.hwnd).filter(|h| *h != NO_WINDOW) {
            if env.is_minimized(term) {
                out.push(Action::SetInputTarget(NO_WINDOW));
                if !self.hidden_away {
                    self.hide_while_away(&mut out);
                }
                return out;
            }
        }
        if fg == NO_WINDOW {
            // locked screen etc.
            self.away_since = None;
            return out;
        }
        if fg == self.my_hwnd || env.is_waitplay_window(fg) {
            self.away_since = None;
            return out;
        }

        if let Some(owner) = self.owner_of(fg).cloned() {
            self.away_since = None;
            self.show_session(&owner, agent_name_of, &mut out);
            if self.hidden_away {
                self.show_after_away(false, &mut out);
            }
            out.push(Action::SetInputTarget(owner.hwnd)); // scrolling/typing in this terminal now dismisses
            return out;
        }
        self.owner_id = None;
        out.push(Action::SetInputTarget(NO_WINDOW));
        // grace so peeking at another tab and straight back doesn't flicker the window
        if self.away_for(env, AWAY_GRACE) && !self.hidden_away {
            self.hide_while_away(&mut out);
        }
        out
    }

    // ── transitions ────────────────────────────────────────────────────────────────────────────────────────────────────

    fn show_session(&mut self, s: &AgentSession, agent_name_of: &dyn Fn(&str) -> String, out: &mut Vec<Action>) {
        if self.owner_id.as_deref() == Some(s.id.as_str()) {
            return;
        }
        self.owner_id = Some(s.id.clone());
        out.push(Action::ShowSession { id: s.id.clone(), terminal: s.terminal.clone(), agent: agent_name_of(&s.id), hwnd: s.hwnd });
    }

    pub fn hide_while_away(&mut self, out: &mut Vec<Action>) {
        self.hidden_away = true;
        out.push(Action::SetInputTarget(NO_WINDOW));
        out.push(Action::HideWhileAway);
        out.push(Action::SetPollRate);
    }

    /// Nothing wants the window any more (task over, user stepped aside, window left): hidden until a prompt or the widget wakes it.
    pub fn park(&mut self, out: &mut Vec<Action>) {
        self.manual = false;
        self.parked_noted = true;
        self.owner_id = None;
        if self.hidden_away {
            out.push(Action::DeepenPark);
            out.push(Action::SetPollRate);
        } else {
            self.hide_while_away(out);
        }
    }

    /// The last session ended. If the user was in this window, the keyboard goes back to that session's terminal.
    fn park_finished(&mut self, env: &mut dyn Env, out: &mut Vec<Action>) {
        let fg = env.foreground();
        if (fg == self.my_hwnd || env.is_waitplay_window(fg)) && self.last_finished_hwnd != NO_WINDOW {
            out.push(Action::FocusTerminal(self.last_finished_hwnd)); // still the foreground window here, so handing it over is allowed
        }
        self.park(out);
    }

    pub fn show_after_away(&mut self, activate: bool, out: &mut Vec<Action>) {
        self.hidden_away = false;
        self.parked_noted = false;
        self.away_since = None;
        out.push(Action::Show { activate });
        out.push(Action::SetPollRate);
    }

    /// The window became visible by a route that did not go through `show_after_away` (e.g. the first show).
    pub fn became_visible(&mut self) {
        self.hidden_away = false;
        self.parked_noted = false;
    }

    /// A session ended. `by_user`: the user stepped aside for this prompt (the agent is still running).
    pub fn finish_session(&mut self, id: &str, by_user: bool) -> bool {
        let Some(s) = self.sessions.get_mut(&key(id)) else { return false };
        if s.finished {
            return false;
        }
        s.finished = true;
        if !by_user && s.hwnd != NO_WINDOW {
            self.last_finished_hwnd = s.hwnd;
        }
        true
    }

    /// The user scrolled or typed in the terminal that owns the window: they want the agent's output, so step aside for this
    /// prompt (other terminals' sessions are unaffected). Returns the finished id for the caller to forget.
    pub fn on_terminal_interaction(&mut self) -> (Vec<Action>, Option<String>) {
        let mut out = Vec::new();
        let Some(owner) = self.owner_id.clone() else { return (out, None) };
        if !self.sessions.contains_key(&key(&owner)) {
            return (out, None);
        }
        self.finish_session(&owner, true);
        self.owner_id = None;
        if !self.any_wanted() {
            self.park(&mut out);
        } else {
            self.hide_while_away(&mut out);
        }
        (out, Some(owner))
    }

    /// X / Esc / Alt+F4: don't re-pop for any prompt that is still running. A window the user opened themselves (widget button)
    /// says nothing about the agent, so it dismisses nothing. Returns the ids to record as dismissed.
    pub fn dismiss_all(&mut self) -> Vec<String> {
        if self.manual || self.sessions.is_empty() {
            return Vec::new();
        }
        let mut ids = Vec::new();
        for s in self.sessions.values_mut() {
            if s.finished || s.dismissed {
                continue;
            }
            s.dismissed = true;
            ids.push(s.id.clone());
        }
        ids
    }

    // ── results from the poller thread ─────────────────────────────────────────────────────────────────────────────────

    pub fn apply_launch(&mut self, l: &LaunchRequest, minimized: bool, is_waitplay_window: &mut dyn FnMut(WindowId) -> bool, visible: bool) -> Vec<Action> {
        let mut out = Vec::new();
        if minimized {
            out.push(Action::Restore);
        }
        if l.session_id.is_empty() {
            // Widget button while we're running: show here, and stay while the user is in the window they pressed it from
            self.desktop_parent = if l.hwnd != NO_WINDOW && !is_waitplay_window(l.hwnd) { l.hwnd } else { NO_WINDOW };
            self.manual = true;
            self.away_since = None;
            if self.hidden_away {
                self.show_after_away(true, &mut out);
            } else if visible {
                out.push(Action::Raise);
            } else {
                out.push(Action::Show { activate: true });
            }
            return out;
        }
        let k = key(&l.session_id);
        let s = self.sessions.entry(k).or_insert_with(|| AgentSession {
            id: l.session_id.clone(),
            terminal: String::new(),
            hwnd: NO_WINDOW,
            own_tab: None,
            finished: false,
            started_at: 0,
            dismissed: false,
            waiting: false,
        });
        if !l.terminal.is_empty() {
            s.terminal = l.terminal.clone();
        }
        s.hwnd = l.hwnd;
        s.started_at = l.ts;
        s.finished = false;
        s.dismissed = false;
        s.waiting = false;
        s.own_tab = None;
        let id = s.id.clone();
        self.parked_noted = false;
        if self.owner_id.as_deref() == Some(id.as_str()) {
            self.owner_id = None; // refresh the header for the new prompt
        }
        out
    }

    /// `ts` is the time of the close signal; `i64::MAX` means the terminal window or tab is gone.
    pub fn apply_close(&mut self, id: &str, ts: i64) -> bool {
        let started = match self.sessions.get(&key(id)) {
            Some(s) => s.started_at,
            None => return false,
        };
        if ts >= started {
            return self.finish_session(id, false);
        }
        false
    }

    pub fn apply_own_tab(&mut self, id: &str, own: Option<bool>) {
        if let Some(s) = self.sessions.get_mut(&key(id)) {
            s.own_tab = own;
        }
    }

    /// Bridge detail for the header, and a second completion source. Returns ids that were finished and the status to show.
    pub fn apply_bridge(&mut self, entries: &[BridgeSession], agent_name_of: &dyn Fn(&str) -> String) -> (Vec<String>, Option<Status>) {
        let mut finished = Vec::new();
        let mut status = None;
        for b in entries {
            let Some(s) = self.sessions.get(&key(&b.session_id)) else { continue };
            let (sid, started) = (s.id.clone(), s.started_at);
            // A COMPLETED left over from the previous turn predates this launch
            if b.status == "COMPLETED" && b.last_event_at >= started {
                if self.finish_session(&sid, false) {
                    finished.push(sid);
                }
                continue;
            }
            if let Some(s) = self.sessions.get_mut(&key(&sid)) {
                // The agent is asking the user something: the window steps aside until the status leaves WAITING_FOR_INPUT
                // (a status older than this launch is left over from the previous turn)
                s.waiting = b.status == "WAITING_FOR_INPUT" && b.last_event_at >= started;
            }
            if self.owner_id.as_deref() != Some(sid.as_str()) {
                continue;
            }
            let agent = agent_name_of(&sid);
            if b.status == "WORKING" || b.status == "GAME_ACTIVE" {
                status = Some(Status::Working { agent, model: b.model.clone(), tool: b.tool.clone() });
            } else if b.status == "WAITING_FOR_INPUT" {
                status = Some(Status::Waiting { agent, prompt: b.waiting_prompt.clone() });
            }
        }
        (finished, status)
    }
}

/// The few bridge fields the companion cares about.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BridgeSession {
    pub session_id: String,
    pub status: String,
    pub last_event_at: i64,
    pub model: String,
    pub tool: String,
    pub waiting_prompt: String,
}

/// Parses the bridge's `GET /sessions` body.
pub fn parse_bridge_sessions(json: &str) -> Vec<BridgeSession> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    let Some(list) = v.get("sessions").and_then(|s| s.as_array()) else { return Vec::new() };
    let text = |o: &serde_json::Value, k: &str| o.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    list.iter()
        .filter(|o| o.is_object())
        .map(|o| BridgeSession {
            session_id: text(o, "sessionId"),
            status: text(o, "status"),
            last_event_at: o.get("lastEventAt").and_then(|x| x.as_i64()).unwrap_or(0),
            model: text(o, "model"),
            tool: text(o, "tool"),
            waiting_prompt: text(o, "waitingForInputPrompt"),
        })
        .collect()
}

/// Display name for a session id (the C# `GetAgentDisplayName`).
pub fn agent_name_of_session(raw: &str) -> String {
    if raw.is_empty() {
        return "Coding Agent".into();
    }
    let lower = raw.to_lowercase();
    if lower.contains("omp") {
        "OMP".into()
    } else {
        waitplay_core::protocol::agent_display_name(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEnv {
        t: Instant,
        fg: WindowId,
        wp_windows: Vec<WindowId>,
        minimized: Vec<WindowId>,
    }
    impl Env for TestEnv {
        fn now(&self) -> Instant {
            self.t
        }
        fn foreground(&self) -> WindowId {
            self.fg
        }
        fn is_waitplay_window(&mut self, id: WindowId) -> bool {
            self.wp_windows.contains(&id)
        }
        fn is_minimized(&self, id: WindowId) -> bool {
            self.minimized.contains(&id)
        }
    }

    const ME: WindowId = 100;
    const TERM_A: WindowId = 1;
    const TERM_B: WindowId = 2;
    const OTHER: WindowId = 9;

    fn name(id: &str) -> String {
        agent_name_of_session(id)
    }

    fn env(fg: WindowId, t: Instant) -> TestEnv {
        TestEnv { t, fg, wp_windows: vec![ME], minimized: vec![] }
    }

    fn core_with(sessions: &[(&str, WindowId)]) -> Core {
        let mut c = Core::new(NO_WINDOW);
        c.my_hwnd = ME;
        c.auto_close_armed = true;
        for (i, (id, hwnd)) in sessions.iter().enumerate() {
            let l = LaunchRequest { session_id: id.to_string(), terminal: "WT".into(), ts: 1000 + i as i64, hwnd: *hwnd };
            c.apply_launch(&l, false, &mut |_| false, false);
        }
        c
    }

    #[test]
    fn shows_the_session_whose_terminal_is_in_front() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        let t0 = Instant::now();
        let acts = c.tick(&mut env(TERM_A, t0), &name);
        assert!(acts.contains(&Action::ShowSession { id: "omp-session".into(), terminal: "WT".into(), agent: "OMP".into(), hwnd: TERM_A }));
        assert!(acts.contains(&Action::SetInputTarget(TERM_A)));
        assert_eq!(c.owner_id.as_deref(), Some("omp-session"));
        // same owner again: header is not re-pointed
        let acts = c.tick(&mut env(TERM_A, t0), &name);
        assert!(!acts.iter().any(|a| matches!(a, Action::ShowSession { .. })));
    }

    #[test]
    fn hides_after_the_grace_period_when_the_user_leaves() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        let t0 = Instant::now();
        c.tick(&mut env(TERM_A, t0), &name);
        // peek elsewhere: nothing within the grace period
        let acts = c.tick(&mut env(OTHER, t0 + Duration::from_millis(10)), &name);
        assert!(!acts.contains(&Action::HideWhileAway));
        assert!(acts.contains(&Action::SetInputTarget(NO_WINDOW)));
        let acts = c.tick(&mut env(OTHER, t0 + Duration::from_millis(100)), &name);
        assert!(acts.contains(&Action::HideWhileAway));
        assert!(c.hidden_away);
        // back to the terminal: shown again, without stealing focus
        let acts = c.tick(&mut env(TERM_A, t0 + Duration::from_millis(200)), &name);
        assert!(acts.contains(&Action::Show { activate: false }));
        assert!(!c.hidden_away);
    }

    #[test]
    fn quick_peek_and_back_never_hides() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        let t0 = Instant::now();
        c.tick(&mut env(TERM_A, t0), &name);
        c.tick(&mut env(OTHER, t0 + Duration::from_millis(10)), &name);
        c.tick(&mut env(TERM_A, t0 + Duration::from_millis(20)), &name);
        let acts = c.tick(&mut env(OTHER, t0 + Duration::from_millis(1000)), &name);
        assert!(!acts.contains(&Action::HideWhileAway), "the away timer restarted when the user came back");
    }

    #[test]
    fn two_terminals_each_get_their_own_session() {
        let mut c = core_with(&[("claude-code-session", TERM_A), ("omp-session", TERM_B)]);
        let t0 = Instant::now();
        let acts = c.tick(&mut env(TERM_B, t0), &name);
        assert!(acts.iter().any(|a| matches!(a, Action::ShowSession { id, .. } if id == "omp-session")));
        let acts = c.tick(&mut env(TERM_A, t0), &name);
        assert!(acts.iter().any(|a| matches!(a, Action::ShowSession { id, .. } if id == "claude-code-session")));
    }

    #[test]
    fn window_steps_aside_while_the_agent_asks_and_returns_after_the_answer() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        let t0 = Instant::now();
        c.tick(&mut env(TERM_A, t0), &name);
        let bridge = |status: &str| vec![BridgeSession { session_id: "omp-session".into(), status: status.into(), last_event_at: 5000, ..Default::default() }];

        c.apply_bridge(&bridge("WAITING_FOR_INPUT"), &name);
        let acts = c.tick(&mut env(TERM_A, t0 + Duration::from_millis(10)), &name);
        assert!(!acts.contains(&Action::HideWhileAway), "grace period first");
        let acts = c.tick(&mut env(TERM_A, t0 + Duration::from_millis(100)), &name);
        assert!(acts.contains(&Action::HideWhileAway));
        assert!(c.any_wanted(), "the prompt is still running");
        assert!(!c.is_parked(), "bridge polling continues so the answer is noticed");

        // answered: the agent is working again
        c.apply_bridge(&bridge("WORKING"), &name);
        let acts = c.tick(&mut env(TERM_A, t0 + Duration::from_millis(200)), &name);
        assert!(acts.contains(&Action::Show { activate: false }));
        assert!(!c.hidden_away);
    }

    #[test]
    fn waiting_status_from_a_previous_turn_is_ignored() {
        let mut c = core_with(&[("omp-session", TERM_A)]); // launched at ts 1000
        let stale = vec![BridgeSession { session_id: "omp-session".into(), status: "WAITING_FOR_INPUT".into(), last_event_at: 500, ..Default::default() }];
        c.apply_bridge(&stale, &name);
        assert!(c.owner_of(TERM_A).is_some());
    }

    #[test]
    fn another_tab_of_the_same_window_does_not_own_the_window() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        c.apply_own_tab("omp-session", Some(false));
        let t0 = Instant::now();
        let acts = c.tick(&mut env(TERM_A, t0), &name);
        assert!(!acts.iter().any(|a| matches!(a, Action::ShowSession { .. })));
        c.apply_own_tab("omp-session", Some(true));
        let acts = c.tick(&mut env(TERM_A, t0), &name);
        assert!(acts.iter().any(|a| matches!(a, Action::ShowSession { .. })));
    }

    #[test]
    fn known_own_tab_beats_unknown_and_newest_wins_ties() {
        let mut c = core_with(&[("a", TERM_A), ("b", TERM_A)]);
        // b is newer
        assert_eq!(c.owner_of(TERM_A).unwrap().id, "b");
        c.apply_own_tab("a", Some(true));
        assert_eq!(c.owner_of(TERM_A).unwrap().id, "a");
    }

    #[test]
    fn finishing_the_last_session_parks_once_and_returns_the_keyboard() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        let t0 = Instant::now();
        c.tick(&mut env(TERM_A, t0), &name);
        assert!(c.apply_close("omp-session", 2000));
        // the user is in the companion when the agent finishes
        let acts = c.tick(&mut env(ME, t0), &name);
        assert!(acts.contains(&Action::FocusTerminal(TERM_A)));
        assert!(acts.contains(&Action::HideWhileAway));
        assert!(c.parked_noted);
        assert!(c.is_parked());
        // and only once
        let acts = c.tick(&mut env(ME, t0), &name);
        assert!(acts.is_empty());
    }

    #[test]
    fn stale_close_signal_from_an_earlier_turn_is_ignored() {
        let mut c = core_with(&[("omp-session", TERM_A)]); // started_at = 1000
        assert!(!c.apply_close("omp-session", 500));
        assert!(c.any_wanted());
        assert!(c.apply_close("omp-session", i64::MAX)); // terminal gone
        assert!(!c.any_wanted());
    }

    #[test]
    fn new_prompt_revives_a_finished_session() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        c.apply_close("omp-session", 5000);
        assert!(!c.any_wanted());
        let l = LaunchRequest { session_id: "OMP-session".into(), terminal: "".into(), ts: 6000, hwnd: TERM_A };
        c.apply_launch(&l, false, &mut |_| false, false);
        assert!(c.any_wanted(), "session ids are case-insensitive");
        assert_eq!(c.sessions.len(), 1);
        assert_eq!(c.sessions.values().next().unwrap().terminal, "WT", "empty terminal keeps the earlier label");
    }

    #[test]
    fn user_scrolling_in_the_owner_terminal_steps_aside_for_this_prompt() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        let t0 = Instant::now();
        c.tick(&mut env(TERM_A, t0), &name);
        let (acts, id) = c.on_terminal_interaction();
        assert_eq!(id.as_deref(), Some("omp-session"));
        assert!(acts.contains(&Action::HideWhileAway));
        assert!(!c.any_wanted());
        // by_user: the keyboard is not sent back anywhere
        assert_eq!(c.last_finished_hwnd, NO_WINDOW);
    }

    #[test]
    fn interaction_with_another_session_still_running_only_hides() {
        let mut c = core_with(&[("a", TERM_A), ("b", TERM_B)]);
        let t0 = Instant::now();
        c.tick(&mut env(TERM_A, t0), &name);
        let (acts, _) = c.on_terminal_interaction();
        assert!(acts.contains(&Action::HideWhileAway));
        assert!(c.any_wanted());
        assert!(!c.parked_noted);
    }

    #[test]
    fn close_button_dismisses_running_prompts_but_not_a_manual_window() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        assert_eq!(c.dismiss_all(), vec!["omp-session".to_string()]);
        assert!(!c.any_wanted());
        let mut m = core_with(&[("omp-session", TERM_A)]);
        m.manual = true;
        assert!(m.dismiss_all().is_empty());
        assert!(core_with(&[]).dismiss_all().is_empty());
    }

    #[test]
    fn widget_press_opens_manual_window_that_follows_the_window_it_came_from() {
        let mut c = core_with(&[]);
        let l = LaunchRequest { session_id: "".into(), terminal: "".into(), ts: 1, hwnd: OTHER };
        let acts = c.apply_launch(&l, false, &mut |_| false, false);
        assert!(c.manual);
        assert_eq!(c.desktop_parent, OTHER);
        assert!(acts.contains(&Action::Show { activate: true }));
        let t0 = Instant::now();
        // staying in the window the user pressed it from, or in the companion: stays
        assert!(c.tick(&mut env(OTHER, t0), &name).is_empty());
        assert!(c.tick(&mut env(ME, t0), &name).is_empty());
        // leaving for something else parks it after the grace period
        c.tick(&mut env(77, t0), &name);
        let acts = c.tick(&mut env(77, t0 + Duration::from_millis(600)), &name);
        assert!(acts.contains(&Action::HideWhileAway));
        assert!(!c.manual);
        assert!(c.is_parked());
        // a second press brings it back
        let acts = c.apply_launch(&l, false, &mut |_| false, false);
        assert!(acts.contains(&Action::Show { activate: true }));
    }

    #[test]
    fn manual_window_hands_over_to_a_waited_on_terminal() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        c.manual = true;
        let t0 = Instant::now();
        let acts = c.tick(&mut env(TERM_A, t0), &name);
        assert!(!c.manual);
        assert!(acts.iter().any(|a| matches!(a, Action::ShowSession { .. })));
    }

    #[test]
    fn nothing_happens_while_disabled_or_before_arming() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        c.enabled = false;
        assert!(c.tick(&mut env(OTHER, Instant::now()), &name).is_empty());
        c.enabled = true;
        c.auto_close_armed = false;
        assert!(c.tick(&mut env(OTHER, Instant::now()), &name).is_empty());
    }

    #[test]
    fn widget_window_in_front_does_not_count_as_leaving() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        let t0 = Instant::now();
        c.tick(&mut env(TERM_A, t0), &name);
        let mut e = TestEnv { t: t0 + Duration::from_secs(5), fg: 55, wp_windows: vec![ME, 55], minimized: vec![] };
        let acts = c.tick(&mut e, &name);
        assert!(acts.is_empty());
        assert!(!c.hidden_away);
    }

    #[test]
    fn minimizing_the_terminal_hides_the_window_and_restoring_it_brings_it_back_while_the_agent_works() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        let t0 = Instant::now();
        c.tick(&mut env(TERM_A, t0), &name);
        assert!(!c.hidden_away);
        // minimizing hands the focus to a Vibe Play window (the entertainment window or the widget): that must not read as "the user is here"
        let minimized = |t: Instant| TestEnv { t, fg: ME, wp_windows: vec![ME], minimized: vec![TERM_A] };
        // a deliberate minimize hides at once: no grace period to wait out
        let acts = c.tick(&mut minimized(t0 + Duration::from_millis(1)), &name);
        assert!(acts.contains(&Action::HideWhileAway) && c.hidden_away);
        // still minimized: stays hidden, no flicker
        assert!(!c.tick(&mut minimized(t0 + Duration::from_secs(5)), &name).contains(&Action::Show { activate: false }));
        // restored and in front, the agent still working: back again
        let acts = c.tick(&mut env(TERM_A, t0 + Duration::from_secs(8)), &name);
        assert!(acts.contains(&Action::Show { activate: false }) && !c.hidden_away);
    }

    #[test]
    fn bridge_completion_and_status() {
        let mut c = core_with(&[("omp-session", TERM_A)]);
        c.tick(&mut env(TERM_A, Instant::now()), &name);
        let working = BridgeSession { session_id: "omp-session".into(), status: "WORKING".into(), last_event_at: 1500, model: "gpt".into(), tool: "bash".into(), ..Default::default() };
        let (fin, st) = c.apply_bridge(&[working], &name);
        assert!(fin.is_empty());
        assert_eq!(st, Some(Status::Working { agent: "OMP".into(), model: "gpt".into(), tool: "bash".into() }));
        let waiting = BridgeSession { session_id: "omp-session".into(), status: "WAITING_FOR_INPUT".into(), waiting_prompt: "ok?".into(), ..Default::default() };
        assert_eq!(c.apply_bridge(&[waiting], &name).1, Some(Status::Waiting { agent: "OMP".into(), prompt: "ok?".into() }));
        // a COMPLETED older than the launch belongs to the previous turn
        let old = BridgeSession { session_id: "omp-session".into(), status: "COMPLETED".into(), last_event_at: 999, ..Default::default() };
        assert!(c.apply_bridge(&[old], &name).0.is_empty());
        assert!(c.any_wanted());
        let done = BridgeSession { session_id: "omp-session".into(), status: "COMPLETED".into(), last_event_at: 1000, ..Default::default() };
        assert_eq!(c.apply_bridge(&[done], &name).0, vec!["omp-session".to_string()]);
        assert!(!c.any_wanted());
    }

    #[test]
    fn parses_the_bridge_body() {
        let body = r#"{"sessions":[{"sessionId":"s","status":"WORKING","lastEventAt":5,"model":"m","tool":"t","waitingForInputPrompt":"p","x":1},3]}"#;
        let v = parse_bridge_sessions(body);
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].session_id.as_str(), v[0].last_event_at, v[0].waiting_prompt.as_str()), ("s", 5, "p"));
        assert!(parse_bridge_sessions("not json").is_empty());
        assert!(parse_bridge_sessions("{}").is_empty());
    }
}
