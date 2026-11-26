//! What the island says and when sounds play: turns the session list into grouped notifications ("toasts") and sound cues.
//! Pure (time and observations are passed in), so the grouping rules are unit tested.
//!
//! Rules: nothing fires for what was already true when the app started; several sessions that start or finish within one
//! [`GROUP_MS`] window become one notification; a finishing session says whether others are still working (and that the shared
//! entertainment window therefore stays); only the last one to finish says the window is closing.

use std::collections::{HashMap, VecDeque};

/// Events closer together than this are one group.
const GROUP_MS: i64 = 1_200;
/// At most this many toasts wait their turn; older ones are dropped, the newest matter.
const QUEUE_MAX: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Working,
    Waiting,
    Done,
    Failed,
}

impl Act {
    pub fn active(self) -> bool {
        matches!(self, Act::Working | Act::Waiting)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sess {
    pub id: String,
    pub name: String,
    pub act: Act,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub id: u64,
    /// `started`, `done`, `alldone`, `error`, `ready`: the page picks the accent from it.
    pub kind: &'static str,
    pub title: String,
    pub body: String,
}

/// Which sounds an observation asks for. The caller decides, from the user's settings, which of them to play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cues {
    pub started: bool,
    pub completed: bool,
    pub all_done: bool,
    pub error: bool,
    pub opened: bool,
}

impl Cues {
    pub fn any(self) -> bool {
        self.started || self.completed || self.all_done || self.error || self.opened
    }
}

/// The words that depend on settings and the entertainment window.
pub struct Context<'a> {
    /// The platform the shared window shows, or will show ("Instagram").
    pub platform: &'a str,
    /// Automation is on, so a starting agent opens the window.
    pub automation: bool,
    /// Is the entertainment window open right now.
    pub entertainment_open: bool,
}

#[derive(Default)]
pub struct Tracker {
    primed: bool,
    prev: HashMap<String, Act>,
    ent_was_open: bool,
    /// The most sessions that were active at once since nothing was active.
    peak: usize,
    next_id: u64,
    started: Vec<String>,
    done: Vec<String>,
    failed: Vec<String>,
    group_since: Option<i64>,
    opened_pending: bool,
    queue: VecDeque<Toast>,
    current: Option<(Toast, i64)>,
}

impl Tracker {
    pub fn new() -> Tracker {
        Tracker::default()
    }

    /// One observation. Returns the sound cues that fired with it. Toasts are collected for [`Tracker::toast`].
    pub fn observe(&mut self, now: i64, sessions: &[Sess], ctx: &Context, toast_secs: u32, notifications: bool) -> Cues {
        let mut cues = Cues::default();
        let active_now = sessions.iter().filter(|s| s.act.active()).count();
        if !self.primed {
            // what was already going on when the app started is not news
            self.primed = true;
            self.prev = sessions.iter().map(|s| (s.id.clone(), s.act)).collect();
            self.ent_was_open = ctx.entertainment_open;
            self.peak = active_now;
            return cues;
        }

        for s in sessions {
            let before = self.prev.get(&s.id).copied();
            let was_active = before.is_some_and(Act::active);
            match (was_active, s.act) {
                (false, a) if a.active() && before != Some(a) => {
                    if before.is_none() || matches!(before, Some(Act::Done | Act::Failed)) {
                        self.started.push(s.name.clone());
                    }
                }
                (true, Act::Done) => self.done.push(s.name.clone()),
                (true, Act::Failed) => self.failed.push(s.name.clone()),
                _ => {}
            }
        }
        // an active session that vanished from the list counts as over, but without a "finished" claim
        self.prev = sessions.iter().map(|s| (s.id.clone(), s.act)).collect();

        if ctx.entertainment_open && !self.ent_was_open {
            self.opened_pending = true;
        }
        self.ent_was_open = ctx.entertainment_open;
        self.peak = self.peak.max(active_now);

        let has_events = !self.started.is_empty() || !self.done.is_empty() || !self.failed.is_empty() || self.opened_pending;
        if has_events && self.group_since.is_none() {
            self.group_since = Some(now);
        }
        if let Some(since) = self.group_since {
            if now - since >= GROUP_MS {
                cues = self.flush(active_now, sessions, ctx, notifications);
                self.group_since = None;
            }
        }
        if active_now == 0 && self.group_since.is_none() {
            self.peak = 0;
        }
        self.advance(now, toast_secs);
        cues
    }

    fn flush(&mut self, active_now: usize, sessions: &[Sess], ctx: &Context, notifications: bool) -> Cues {
        let (started, done, failed) = (std::mem::take(&mut self.started), std::mem::take(&mut self.done), std::mem::take(&mut self.failed));
        let opened = std::mem::take(&mut self.opened_pending);
        let mut cues = Cues { started: !started.is_empty(), error: !failed.is_empty(), opened, ..Cues::default() };
        let mut toasts: Vec<(&'static str, String, String)> = Vec::new();
        let others: Vec<&str> = sessions.iter().filter(|s| s.act.active()).map(|s| s.name.as_str()).collect();

        if !started.is_empty() {
            if started.len() == 1 {
                let body = if ctx.automation { format!("{} is opening.", ctx.platform) } else { String::new() };
                toasts.push(("started", format!("{} started working.", started[0]), body));
            } else {
                toasts.push(("started", format!("{} AI coding sessions are now active.", active_now.max(started.len())), list(&started)));
            }
        }
        if !failed.is_empty() {
            let (title, body) = if failed.len() == 1 {
                (format!("{} encountered an error.", failed[0]), "Check your coding session.".to_string())
            } else {
                (format!("{} sessions encountered errors.", failed.len()), list(&failed))
            };
            toasts.push(("error", title, body));
        }
        if !done.is_empty() {
            if active_now == 0 {
                cues.all_done = true;
                let closing = if ctx.entertainment_open { "Your entertainment window is closing." } else { "You can get back to your code." };
                if done.len() == 1 && self.peak <= 1 {
                    toasts.push(("done", format!("{} finished its task.", done[0]), "You can get back to your code.".to_string()));
                } else {
                    toasts.push(("alldone", "All coding sessions completed.".to_string(), closing.to_string()));
                }
            } else {
                cues.completed = true;
                let who = if done.len() == 1 { format!("{} completed.", done[0]) } else { format!("{} sessions completed.", done.len()) };
                let still = if others.len() == 1 { format!("{} is still working.", others[0]) } else { format!("{} sessions are still working.", others.len()) };
                let stays = if ctx.entertainment_open { format!(" {} stays open.", ctx.platform) } else { String::new() };
                toasts.push(("done", who, format!("{}{}", still, stays)));
            }
            // a finish is also "a completion" for the sound rules when it was the last one
            cues.completed |= cues.all_done;
        }
        if opened && started.is_empty() && active_now == 0 {
            toasts.push(("ready", format!("{} is ready.", ctx.platform), String::new()));
        }
        if notifications {
            for (kind, title, body) in toasts {
                self.next_id += 1;
                self.queue.push_back(Toast { id: self.next_id, kind, title, body });
                while self.queue.len() > QUEUE_MAX {
                    self.queue.pop_front();
                }
            }
        }
        cues
    }

    fn advance(&mut self, now: i64, toast_secs: u32) {
        if let Some((_, until)) = &self.current {
            if now >= *until {
                self.current = None;
            }
        }
        if self.current.is_none() {
            if let Some(t) = self.queue.pop_front() {
                self.current = Some((t, now + toast_secs as i64 * 1000));
            }
        }
    }

    /// The notification to show right now, if any. Call [`Tracker::observe`] (or [`Tracker::tick`]) first.
    pub fn toast(&self) -> Option<&Toast> {
        self.current.as_ref().map(|(t, _)| t)
    }

    /// Expires and promotes toasts without a new observation.
    pub fn tick(&mut self, now: i64, toast_secs: u32) {
        self.advance(now, toast_secs);
    }

    /// The user closed the notification.
    pub fn dismiss_toast(&mut self) {
        self.current = None;
    }
}

/// The line under the agent status: what Vibe Play is doing with the shared entertainment window, in words. `open` is the label of
/// the platform whose window is open, `just_finished` that a session ended in the last few seconds.
pub struct EntLine {
    pub state: &'static str,
    pub text: String,
    pub hint: String,
}

pub fn entertainment_line(automation: bool, open: Option<&str>, platform: &str, active: usize, opening: bool, just_finished: bool) -> EntLine {
    let line = |state: &'static str, text: String, hint: &str| EntLine { state, text, hint: hint.to_string() };
    if !automation {
        return line("paused", "Automation paused".into(), "Agents are still being watched.");
    }
    match (open, active) {
        (Some(l), 0) if just_finished => line("closing", format!("{} closing", l), "Closing automatically."),
        (Some(l), 0) => line("open", format!("{} open", l), ""),
        (Some(l), 1) => line("active", format!("{} active", l), "Closes when the session finishes."),
        (Some(l), _) => line("active", format!("{} active", l), "Stays open until every session finishes."),
        (None, 0) => line("waiting", "Waiting for an agent".into(), ""),
        (None, _) if opening => line("opening", format!("Opening {}\u{2026}", platform), ""),
        (None, _) => line("hidden", "Entertainment hidden".into(), "Open it from here any time."),
    }
}

fn list(names: &[String]) -> String {
    let mut shown: Vec<&str> = names.iter().take(3).map(String::as_str).collect();
    if names.len() > 3 {
        shown.push("…");
    }
    shown.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(id: &str, name: &str, act: Act) -> Sess {
        Sess { id: id.into(), name: name.into(), act }
    }
    fn ctx(open: bool) -> Context<'static> {
        Context { platform: "Instagram", automation: true, entertainment_open: open }
    }
    /// Observe at `t`, then once more `GROUP_MS` later so the group flushes.
    fn settle(tr: &mut Tracker, t: i64, list: &[Sess], open: bool) -> Cues {
        let a = tr.observe(t, list, &ctx(open), 5, true);
        let b = tr.observe(t + GROUP_MS, list, &ctx(open), 5, true);
        Cues { started: a.started | b.started, completed: a.completed | b.completed, all_done: a.all_done | b.all_done, error: a.error | b.error, opened: a.opened | b.opened }
    }
    fn drain(tr: &mut Tracker) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut t = 100_000;
        for _ in 0..6 {
            if let Some(x) = tr.toast() {
                out.push((x.title.clone(), x.body.clone()));
            }
            t += 6_000;
            tr.tick(t, 5);
            // the tick that expires a toast promotes the next one in the same call, so look again before moving on
        }
        out.dedup();
        out
    }

    #[test]
    fn what_was_already_running_at_start_is_silent() {
        let mut tr = Tracker::new();
        let cues = settle(&mut tr, 0, &[s("a", "Claude Code", Act::Working)], true);
        assert!(!cues.any());
        assert!(tr.toast().is_none());
    }

    #[test]
    fn one_agent_starting_says_so_and_cues_once() {
        let mut tr = Tracker::new();
        tr.observe(0, &[], &ctx(false), 5, true);
        let cues = settle(&mut tr, 1_000, &[s("a", "Claude Code", Act::Working)], false);
        assert!(cues.started && !cues.completed);
        let t = tr.toast().unwrap();
        assert_eq!((t.kind, t.title.as_str(), t.body.as_str()), ("started", "Claude Code started working.", "Instagram is opening."));
    }

    #[test]
    fn three_agents_starting_together_are_one_notification() {
        let mut tr = Tracker::new();
        tr.observe(0, &[], &ctx(false), 5, true);
        let list = [s("a", "Claude Code", Act::Working), s("b", "Cursor", Act::Working), s("c", "Codex", Act::Working)];
        settle(&mut tr, 1_000, &list, false);
        let t = tr.toast().unwrap();
        assert_eq!(t.title, "3 AI coding sessions are now active.");
        assert_eq!(t.body, "Claude Code, Cursor, Codex");
        assert!(drain_len(&mut tr) <= 1, "no second toast for the same group");
    }
    fn drain_len(tr: &mut Tracker) -> usize {
        drain(tr).len()
    }

    #[test]
    fn first_of_two_finishing_says_the_other_is_still_working_and_the_window_stays() {
        let mut tr = Tracker::new();
        tr.observe(0, &[], &ctx(false), 5, true);
        settle(&mut tr, 1_000, &[s("a", "Claude Code", Act::Working), s("b", "Cursor", Act::Working)], true);
        tr.dismiss_toast();
        let cues = settle(&mut tr, 20_000, &[s("a", "Claude Code", Act::Done), s("b", "Cursor", Act::Working)], true);
        assert!(cues.completed && !cues.all_done);
        let t = tr.toast().unwrap();
        assert_eq!((t.title.as_str(), t.body.as_str()), ("Claude Code completed.", "Cursor is still working. Instagram stays open."));
    }

    #[test]
    fn last_one_finishing_after_several_says_all_done_and_closing() {
        let mut tr = Tracker::new();
        tr.observe(0, &[], &ctx(false), 5, true);
        settle(&mut tr, 1_000, &[s("a", "Claude Code", Act::Working), s("b", "Cursor", Act::Working)], true);
        settle(&mut tr, 20_000, &[s("a", "Claude Code", Act::Done), s("b", "Cursor", Act::Working)], true);
        tr.dismiss_toast();
        let cues = settle(&mut tr, 40_000, &[s("a", "Claude Code", Act::Done), s("b", "Cursor", Act::Done)], true);
        assert!(cues.all_done);
        let seen = drain(&mut tr);
        assert!(seen.iter().any(|(t, b)| t == "All coding sessions completed." && b == "Your entertainment window is closing."), "{seen:?}");
    }

    #[test]
    fn a_single_agent_finishing_alone_is_a_plain_finished_message() {
        let mut tr = Tracker::new();
        tr.observe(0, &[], &ctx(false), 5, true);
        settle(&mut tr, 1_000, &[s("a", "Codex", Act::Working)], true);
        tr.dismiss_toast();
        let cues = settle(&mut tr, 9_000, &[s("a", "Codex", Act::Done)], true);
        assert!(cues.all_done);
        let t = tr.toast().unwrap();
        assert_eq!((t.title.as_str(), t.body.as_str()), ("Codex finished its task.", "You can get back to your code."));
    }

    #[test]
    fn three_finishing_at_once_are_one_group_and_one_sound() {
        let mut tr = Tracker::new();
        tr.observe(0, &[], &ctx(false), 5, true);
        let w = [s("a", "A", Act::Working), s("b", "B", Act::Working), s("c", "C", Act::Working)];
        settle(&mut tr, 1_000, &w, true);
        tr.dismiss_toast();
        let d = [s("a", "A", Act::Done), s("b", "B", Act::Done), s("c", "C", Act::Done)];
        let mut cues = tr.observe(30_000, &d, &ctx(true), 5, true);
        cues.all_done |= tr.observe(30_500, &d, &ctx(true), 5, true).all_done;
        let more = tr.observe(31_300, &d, &ctx(true), 5, true);
        assert!(cues.all_done || more.all_done);
        assert_eq!(tr.toast().unwrap().title, "All coding sessions completed.");
    }

    #[test]
    fn errors_are_reported_and_cued() {
        let mut tr = Tracker::new();
        tr.observe(0, &[], &ctx(false), 5, true);
        settle(&mut tr, 1_000, &[s("a", "Claude Code", Act::Working)], true);
        tr.dismiss_toast();
        let cues = settle(&mut tr, 5_000, &[s("a", "Claude Code", Act::Failed)], true);
        assert!(cues.error);
        let t = tr.toast().unwrap();
        assert_eq!((t.kind, t.title.as_str(), t.body.as_str()), ("error", "Claude Code encountered an error.", "Check your coding session."));
    }

    #[test]
    fn a_stale_done_session_seen_again_makes_no_noise() {
        let mut tr = Tracker::new();
        tr.observe(0, &[s("a", "Codex", Act::Done)], &ctx(false), 5, true);
        let cues = settle(&mut tr, 5_000, &[s("a", "Codex", Act::Done)], false);
        assert!(!cues.any() && tr.toast().is_none());
    }

    #[test]
    fn notifications_off_still_cues_sounds_but_shows_nothing() {
        let mut tr = Tracker::new();
        tr.observe(0, &[], &ctx(false), 5, false);
        tr.observe(1_000, &[s("a", "Codex", Act::Working)], &ctx(false), 5, false);
        let cues = tr.observe(2_300, &[s("a", "Codex", Act::Working)], &ctx(false), 5, false);
        assert!(cues.started);
        assert!(tr.toast().is_none());
    }

    #[test]
    fn manual_open_with_no_agent_says_ready() {
        let mut tr = Tracker::new();
        tr.observe(0, &[], &ctx(false), 5, true);
        let cues = settle(&mut tr, 1_000, &[], true);
        assert!(cues.opened);
        assert_eq!(tr.toast().unwrap().title, "Instagram is ready.");
    }

    #[test]
    fn toasts_expire_and_queue_in_order() {
        let mut tr = Tracker::new();
        tr.observe(0, &[], &ctx(false), 3, true);
        settle(&mut tr, 1_000, &[s("a", "A", Act::Working)], false);
        assert_eq!(tr.toast().unwrap().title, "A started working.");
        settle(&mut tr, 3_000, &[s("a", "A", Act::Done)], false);
        assert_eq!(tr.toast().unwrap().title, "A started working.", "the second waits its turn");
        tr.tick(7_300, 3);
        assert_eq!(tr.toast().map(|t| t.title.as_str()), Some("A finished its task."));
        tr.tick(10_400, 3);
        assert!(tr.toast().is_none());
    }

    #[test]
    fn the_entertainment_line_follows_the_window_not_the_agent() {
        let l = |auto, open: Option<&str>, active, opening, fin| entertainment_line(auto, open, "YouTube", active, opening, fin);
        assert_eq!(l(false, Some("YouTube"), 1, false, false).state, "paused");
        assert_eq!(l(true, None, 1, true, false).text, "Opening YouTube\u{2026}");
        assert_eq!(l(true, None, 1, false, false).state, "hidden", "a window the user hid is not 'opening' again");
        assert_eq!(l(true, Some("YouTube"), 2, false, false).hint, "Stays open until every session finishes.");
        assert_eq!(l(true, Some("YouTube"), 0, false, true).state, "closing");
        assert_eq!(l(true, Some("YouTube"), 0, false, false).state, "open");
        assert_eq!(l(true, None, 0, false, false).state, "waiting");
    }
}
