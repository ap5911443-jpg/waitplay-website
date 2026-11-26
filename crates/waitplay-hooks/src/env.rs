//! What a watcher needs from the outside world, as a trait so the state machines can be tested with a fake clock, and the
//! poll loop that drives them.

use crate::common::{self, PopRequest};
use serde_json::{Map, Value};
use std::io::Write;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use waitplay_core::{protocol::now_ms, signals, state};

pub trait Env {
    fn now_ms(&self) -> i64;
    /// Queues one event for the bridge (`agent`/`sessionId`/`timestamp` are filled in by the environment).
    fn post(&mut self, event: &str, model: Option<Value>, data: Option<Map<String, Value>>);
    fn pop_app(&mut self, req: &PopRequest);
    fn is_enabled(&self) -> bool;
    fn is_prompt_dismissed(&self, session: &str) -> bool;
    fn clear_dismissal(&mut self, session: &str);
    fn write_close_signal(&mut self, session: &str);
    fn clear_close_signal(&mut self, session: &str);
    fn log(&mut self, msg: &str);
}

/// A long-running source of events: polled on a fixed interval, with optional timers (debounces, heartbeats).
pub trait Watcher {
    fn poll(&mut self, env: &mut dyn Env);
    /// Earliest pending timer, epoch milliseconds.
    fn deadline(&self) -> Option<i64>;
    /// Runs every timer that is due at `env.now_ms()`.
    fn fire(&mut self, env: &mut dyn Env);
    /// No turn is open, so polling can slow down until one starts.
    fn is_idle(&self) -> bool {
        false
    }
    /// While a turn is open, [`run_loop`] posts a `heartbeat` event every [`HEARTBEAT_EVERY`] so the bridge's silence watchdog does
    /// not call a long answer or a long tool "finished". Watchers that send their own (OMP) leave this off.
    fn wants_heartbeat(&self) -> bool {
        false
    }
}

/// How often an open turn says "still going", and for how long at most (a turn whose end marker never comes must not look busy
/// forever: after this the bridge's watchdog closes it).
pub const HEARTBEAT_EVERY: Duration = Duration::from_secs(5);
pub const HEARTBEAT_MAX: Duration = Duration::from_secs(30 * 60);

/// Production environment. Events go through one worker thread so a slow bridge never stalls polling and events keep their
/// order, like the fire-and-forget requests of the Node watchers (but strictly ordered).
pub struct RealEnv {
    agent: &'static str,
    session: &'static str,
    tx: mpsc::Sender<Value>,
}

impl RealEnv {
    pub fn new(agent: &'static str, session: &'static str) -> RealEnv {
        let (tx, rx) = mpsc::channel::<Value>();
        // Runs for the life of the process; the watchers never return
        std::thread::spawn(move || {
            for payload in rx {
                common::post_event_quiet(&payload);
            }
        });
        RealEnv { agent, session, tx }
    }
}

impl Env for RealEnv {
    fn now_ms(&self) -> i64 {
        now_ms()
    }

    fn post(&mut self, event: &str, model: Option<Value>, data: Option<Map<String, Value>>) {
        let payload = common::event_payload(self.agent, self.session, event, model, None, data);
        let _ = self.tx.send(payload);
    }

    fn pop_app(&mut self, req: &PopRequest) {
        common::spawn_app(req);
    }

    fn is_enabled(&self) -> bool {
        state::is_enabled()
    }

    fn is_prompt_dismissed(&self, session: &str) -> bool {
        signals::is_prompt_dismissed(session)
    }

    fn clear_dismissal(&mut self, session: &str) {
        signals::clear_dismissal(session);
    }

    fn write_close_signal(&mut self, session: &str) {
        signals::write_close_signal(session);
    }

    fn clear_close_signal(&mut self, session: &str) {
        let _ = std::fs::remove_file(signals::close_file_path(session));
    }

    fn log(&mut self, msg: &str) {
        // stdout may be closed or a dead pipe when the watcher runs detached; never panic over a log line
        let _ = writeln!(std::io::stdout(), "{}", msg);
    }
}

/// While no turn is open a watcher only needs to notice that one starts: a slower poll keeps an idle Mac idle.
pub const IDLE_POLL_MS: u64 = 300;

/// Drives a watcher forever: `poll` every `poll_ms` (every `IDLE_POLL_MS` while it reports idle), timers as soon as they are due.
pub fn run_loop(poll_ms: u64, watcher: &mut dyn Watcher, env: &mut dyn Env) -> ! {
    let busy_interval = Duration::from_millis(poll_ms);
    let idle_interval = Duration::from_millis(poll_ms.max(IDLE_POLL_MS));
    let mut next_poll = Instant::now();
    let mut busy_since: Option<Instant> = None;
    let mut next_beat = Instant::now() + HEARTBEAT_EVERY;
    loop {
        let now = Instant::now();
        if watcher.wants_heartbeat() && !watcher.is_idle() {
            let since = *busy_since.get_or_insert(now);
            if now >= next_beat && now.duration_since(since) < HEARTBEAT_MAX {
                env.post("heartbeat", None, None);
                next_beat = now + HEARTBEAT_EVERY;
            }
        } else {
            busy_since = None;
            next_beat = now + HEARTBEAT_EVERY;
        }
        if now >= next_poll {
            watcher.poll(env);
            let interval = if watcher.is_idle() { idle_interval } else { busy_interval };
            next_poll += interval;
            if next_poll < now {
                next_poll = now + interval;
            }
        }
        let mut wait = next_poll.saturating_duration_since(Instant::now());
        if let Some(d) = watcher.deadline() {
            let left = d - env.now_ms();
            if left <= 0 {
                watcher.fire(env);
                continue;
            }
            wait = wait.min(Duration::from_millis(left as u64));
        }
        std::thread::sleep(wait.max(Duration::from_millis(1)));
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashSet;

    /// Records everything a watcher does. `calls` holds `post:<event>`, `pop:<session>`, `close:<session>` ... in order.
    #[derive(Default)]
    pub struct MockEnv {
        pub now: i64,
        pub enabled: bool,
        pub dismissed: HashSet<String>,
        pub calls: Vec<String>,
        pub posts: Vec<(String, Option<Value>, Option<Map<String, Value>>)>,
        pub pops: Vec<PopRequest>,
        pub logs: Vec<String>,
    }

    impl MockEnv {
        pub fn new() -> MockEnv {
            MockEnv { now: 1_000_000, enabled: true, ..Default::default() }
        }

        pub fn events(&self) -> Vec<&str> {
            self.posts.iter().map(|p| p.0.as_str()).collect()
        }

        pub fn advance(&mut self, ms: i64) {
            self.now += ms;
        }

        pub fn data(&self, i: usize) -> Value {
            self.posts[i].2.clone().map(Value::Object).unwrap_or(Value::Null)
        }
    }

    impl Env for MockEnv {
        fn now_ms(&self) -> i64 {
            self.now
        }
        fn post(&mut self, event: &str, model: Option<Value>, data: Option<Map<String, Value>>) {
            self.calls.push(format!("post:{}", event));
            self.posts.push((event.to_string(), model, data));
        }
        fn pop_app(&mut self, req: &PopRequest) {
            self.calls.push(format!("pop:{}", req.session));
            self.pops.push(req.clone());
        }
        fn is_enabled(&self) -> bool {
            self.enabled
        }
        fn is_prompt_dismissed(&self, session: &str) -> bool {
            self.dismissed.contains(session)
        }
        fn clear_dismissal(&mut self, session: &str) {
            self.calls.push(format!("clear_dismissal:{}", session));
            self.dismissed.remove(session);
        }
        fn write_close_signal(&mut self, session: &str) {
            self.calls.push(format!("close:{}", session));
        }
        fn clear_close_signal(&mut self, session: &str) {
            self.calls.push(format!("unclose:{}", session));
        }
        fn log(&mut self, msg: &str) {
            self.logs.push(msg.to_string());
        }
    }
}
