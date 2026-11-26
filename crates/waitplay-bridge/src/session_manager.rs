//! Tracks one state machine per session id (port of `SessionManager`).

use crate::state_machine::{AgentStateMachine, StateMachineListener, StateMachineOptions, DEFAULT_START_DELAY_MS};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use waitplay_core::{now_ms, AgentEvent, AgentEventPayload, AgentSessionInfo, AgentStatus, AgentType};

pub type SessionCallback = Arc<dyn Fn(&AgentSessionInfo) + Send + Sync>;

/// A session that has sat in WORKING / GAME_ACTIVE with no event for this long is considered finished.
const WATCHDOG_MS: i64 = 20_000;

#[derive(Default)]
pub struct SessionManagerOptions {
    /// Default 4000.
    pub default_start_delay_ms: Option<i64>,
    /// Invoked once per transition to COMPLETED. Must not block (spawn work instead).
    pub on_task_completed: Option<SessionCallback>,
}

struct Shared {
    sessions: Mutex<Vec<(String, Arc<AgentStateMachine>)>>,
    default_start_delay_ms: i64,
    change_callbacks: Mutex<Vec<(usize, SessionCallback)>>,
    next_callback_id: AtomicUsize,
    on_task_completed: Option<SessionCallback>,
}

pub struct SessionManager {
    shared: Arc<Shared>,
}

/// Listener attached to each machine; holds the manager weakly so dropping the manager never leaks machines.
struct ManagerListener {
    shared: Weak<Shared>,
}

impl StateMachineListener for ManagerListener {
    fn on_status_change(&self, new_status: AgentStatus, _prev: AgentStatus, session: &AgentSessionInfo) {
        let Some(shared) = self.shared.upgrade() else { return };
        shared.notify(session);
        if new_status == AgentStatus::Completed {
            if let Some(cb) = &shared.on_task_completed {
                cb(session);
            }
        }
    }
}

impl Shared {
    fn sessions(&self) -> MutexGuard<'_, Vec<(String, Arc<AgentStateMachine>)>> {
        self.sessions.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn notify(&self, session: &AgentSessionInfo) {
        let callbacks: Vec<SessionCallback> = self
            .change_callbacks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(_, c)| c.clone())
            .collect();
        for cb in callbacks {
            cb(session);
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(SessionManagerOptions::default())
    }
}

impl SessionManager {
    pub fn new(options: SessionManagerOptions) -> Self {
        SessionManager {
            shared: Arc::new(Shared {
                sessions: Mutex::new(Vec::new()),
                default_start_delay_ms: options.default_start_delay_ms.unwrap_or(DEFAULT_START_DELAY_MS),
                change_callbacks: Mutex::new(Vec::new()),
                next_callback_id: AtomicUsize::new(0),
                on_task_completed: options.on_task_completed,
            }),
        }
    }

    /// Registers a change callback; the returned id removes it again via [`SessionManager::remove_session_change`].
    pub fn on_session_change(&self, callback: SessionCallback) -> usize {
        let id = self.shared.next_callback_id.fetch_add(1, Ordering::Relaxed);
        self.shared.change_callbacks.lock().unwrap_or_else(|e| e.into_inner()).push((id, callback));
        id
    }

    pub fn remove_session_change(&self, id: usize) {
        self.shared.change_callbacks.lock().unwrap_or_else(|e| e.into_inner()).retain(|(i, _)| *i != id);
    }

    pub fn get_or_create_session(&self, session_id: &str, agent: Option<AgentType>) -> Arc<AgentStateMachine> {
        let mut sessions = self.shared.sessions();
        if let Some((_, machine)) = sessions.iter().find(|(id, _)| id == session_id) {
            if let Some(a) = agent {
                machine.set_agent(a);
            }
            return machine.clone();
        }
        let machine = AgentStateMachine::new(
            session_id,
            StateMachineOptions {
                agent: Some(agent.unwrap_or(AgentType::ClaudeCode)),
                start_delay_ms: Some(self.shared.default_start_delay_ms),
                auto_launch: Some(true),
            },
        );
        machine.subscribe(Arc::new(ManagerListener { shared: Arc::downgrade(&self.shared) }));
        // Bounded memory: past MAX_SESSIONS the least recently active session goes (idle ones before working ones)
        let evicted = if sessions.len() >= crate::limits::MAX_SESSIONS {
            let idx = sessions
                .iter()
                .enumerate()
                .min_by_key(|(_, (_, m))| {
                    let s = m.get_session();
                    (matches!(s.status, AgentStatus::Working | AgentStatus::GameActive), s.last_event_at)
                })
                .map(|(i, _)| i);
            idx.map(|i| sessions.remove(i).1)
        } else {
            None
        };
        sessions.push((session_id.to_string(), machine.clone()));
        drop(sessions);
        if let Some(old) = evicted {
            old.destroy();
        }
        machine
    }

    pub fn process_event(&self, payload: &AgentEventPayload) -> (AgentSessionInfo, AgentStatus) {
        let machine = self.get_or_create_session(&payload.session_id, Some(payload.agent));
        let mut event_data = payload.data.clone().unwrap_or_default();
        if let Some(model) = payload.model.as_deref().filter(|m| !m.is_empty()) {
            if !crate::js::truthy(event_data.get("model")) {
                event_data.insert("model".into(), model.into());
            }
        }
        if let Some(terminal) = payload.terminal.as_deref().filter(|t| !t.is_empty()) {
            if !crate::js::truthy(event_data.get("terminal")) {
                event_data.insert("terminal".into(), terminal.into());
            }
        }
        let status = machine.handle_event(payload.event, Some(payload.timestamp), Some(&event_data), Some(payload.agent));
        (machine.get_session(), status)
    }

    pub fn get_session(&self, session_id: &str) -> Option<AgentSessionInfo> {
        self.shared.sessions().iter().find(|(id, _)| id == session_id).map(|(_, m)| m.get_session())
    }

    fn machines(&self) -> Vec<Arc<AgentStateMachine>> {
        self.shared.sessions().iter().map(|(_, m)| m.clone()).collect()
    }

    /// Applies the 20s watchdog (a silent WORKING/GAME_ACTIVE session is completed) and lists every session.
    pub fn get_all_sessions(&self) -> Vec<AgentSessionInfo> {
        let now = now_ms();
        let machines = self.machines();
        for machine in &machines {
            let s = machine.get_session();
            if matches!(s.status, AgentStatus::Working | AgentStatus::GameActive) && now - s.last_event_at > WATCHDOG_MS {
                machine.handle_event(AgentEvent::TaskCompleted, Some(now), None, None);
            }
        }
        machines.iter().map(|m| m.get_session()).collect()
    }

    pub fn reset_session(&self, session_id: &str) -> bool {
        let removed = {
            let mut sessions = self.shared.sessions();
            sessions.iter().position(|(id, _)| id == session_id).map(|i| sessions.remove(i))
        };
        match removed {
            Some((_, machine)) => {
                machine.destroy();
                true
            }
            None => false,
        }
    }

    pub fn clear_all(&self) {
        let drained: Vec<_> = std::mem::take(&mut *self.shared.sessions());
        for (_, machine) in drained {
            machine.destroy();
        }
    }

    pub fn get_active_count(&self) -> usize {
        self.shared.sessions().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(agent: AgentType, session: &str, event: AgentEvent) -> AgentEventPayload {
        AgentEventPayload { agent, session_id: session.into(), event, timestamp: now_ms(), model: None, terminal: None, data: None }
    }

    #[tokio::test(start_paused = true)]
    async fn creates_and_tracks_multiple_independent_sessions() {
        let sm = SessionManager::default();
        let a = sm.process_event(&ev(AgentType::ClaudeCode, "session-alpha", AgentEvent::TaskStarted));
        let b = sm.process_event(&ev(AgentType::ClaudeCode, "session-beta", AgentEvent::SessionStarted));

        assert_eq!(a.1, AgentStatus::Working);
        assert_eq!(b.1, AgentStatus::Idle);
        assert_eq!(sm.get_active_count(), 2);
        assert_eq!(sm.get_session("session-alpha").unwrap().status, AgentStatus::Working);
        assert_eq!(sm.get_session("session-beta").unwrap().status, AgentStatus::Idle);
        sm.clear_all();
    }

    #[tokio::test(start_paused = true)]
    async fn completion_of_session_a_does_not_affect_or_close_session_b() {
        let sm = SessionManager::default();
        sm.process_event(&ev(AgentType::ClaudeCode, "session-A", AgentEvent::TaskStarted));
        sm.process_event(&ev(AgentType::ClaudeCode, "session-B", AgentEvent::TaskStarted));

        assert_eq!(sm.get_session("session-A").unwrap().status, AgentStatus::Working);
        assert_eq!(sm.get_session("session-B").unwrap().status, AgentStatus::Working);

        sm.process_event(&ev(AgentType::ClaudeCode, "session-A", AgentEvent::TaskCompleted));

        assert_eq!(sm.get_session("session-A").unwrap().status, AgentStatus::Completed);
        assert_eq!(sm.get_session("session-B").unwrap().status, AgentStatus::Working);
        sm.clear_all();
    }

    #[tokio::test(start_paused = true)]
    async fn invokes_completion_callback_once_when_a_task_completes() {
        let completed: Arc<Mutex<Vec<String>>> = Arc::default();
        let sink = completed.clone();
        let sm = SessionManager::new(SessionManagerOptions {
            on_task_completed: Some(Arc::new(move |s| sink.lock().unwrap().push(s.session_id.clone()))),
            ..Default::default()
        });

        sm.process_event(&ev(AgentType::Codex, "notify-session", AgentEvent::TaskStarted));
        sm.process_event(&ev(AgentType::Codex, "notify-session", AgentEvent::TaskCompleted));
        sm.process_event(&ev(AgentType::Codex, "notify-session", AgentEvent::TaskCompleted));

        assert_eq!(*completed.lock().unwrap(), vec!["notify-session".to_string()]);
        sm.clear_all();
    }

    #[tokio::test(start_paused = true)]
    async fn resetting_one_session_does_not_delete_or_interfere_with_others() {
        let sm = SessionManager::default();
        sm.process_event(&ev(AgentType::ClaudeCode, "session-1", AgentEvent::SessionStarted));
        sm.process_event(&ev(AgentType::ClaudeCode, "session-2", AgentEvent::SessionStarted));
        assert_eq!(sm.get_active_count(), 2);

        assert!(sm.reset_session("session-1"));
        assert_eq!(sm.get_active_count(), 1);
        assert!(sm.get_session("session-1").is_none());
        assert!(sm.get_session("session-2").is_some());
        assert!(!sm.reset_session("session-1"));
        sm.clear_all();
    }

    #[tokio::test(start_paused = true)]
    async fn change_callbacks_fire_on_background_launch_and_can_be_removed() {
        let seen: Arc<Mutex<Vec<AgentStatus>>> = Arc::default();
        let sink = seen.clone();
        let sm = SessionManager::default();
        let id = sm.on_session_change(Arc::new(move |s| sink.lock().unwrap().push(s.status)));

        sm.process_event(&ev(AgentType::Omp, "bg", AgentEvent::TaskStarted));
        tokio::time::advance(std::time::Duration::from_millis(4001)).await;
        for _ in 0..4 {
            tokio::task::yield_now().await;
        }
        assert_eq!(*seen.lock().unwrap(), vec![AgentStatus::Working, AgentStatus::GameActive]);

        sm.remove_session_change(id);
        sm.process_event(&ev(AgentType::Omp, "bg", AgentEvent::TaskCompleted));
        assert_eq!(seen.lock().unwrap().len(), 2);
        sm.clear_all();
    }

    #[tokio::test(start_paused = true)]
    async fn watchdog_completes_silent_working_sessions() {
        let sm = SessionManager::default();
        sm.process_event(&ev(AgentType::Omp, "stale", AgentEvent::TaskStarted));
        // A status change stamps lastEventAt with the wall clock; a later event in the same status carries its own
        // (old) timestamp, which is what a silent session looks like to the watchdog.
        let machine = sm.get_or_create_session("stale", None);
        machine.handle_event(AgentEvent::TaskRunning, Some(now_ms() - 25_000), None, None);
        let sessions = sm.get_all_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].status, AgentStatus::Completed);
        assert_eq!(sessions[0].total_tasks_completed, 1);
        sm.clear_all();
    }

    #[tokio::test(start_paused = true)]
    async fn session_count_is_capped_and_idle_sessions_are_evicted_before_working_ones() {
        use crate::limits::MAX_SESSIONS;
        let sm = SessionManager::default();
        // one long-running working session, then idle ones past the cap
        sm.process_event(&ev(AgentType::ClaudeCode, "worker", AgentEvent::TaskStarted));
        for i in 0..(MAX_SESSIONS + 20) {
            sm.process_event(&ev(AgentType::ClaudeCode, &format!("idle-{i}"), AgentEvent::SessionStarted));
        }
        assert_eq!(sm.get_active_count(), MAX_SESSIONS);
        assert!(sm.get_session("worker").is_some(), "the working session is kept");
        assert!(sm.get_session(&format!("idle-{}", MAX_SESSIONS + 19)).is_some(), "the newest session is kept");
        assert!(sm.get_session("idle-0").is_none(), "the oldest idle session was evicted");
        sm.clear_all();
    }

    #[tokio::test(start_paused = true)]
    async fn heartbeats_keep_a_working_session_alive_but_never_wake_a_finished_or_waiting_one() {
        let sm = SessionManager::default();
        sm.process_event(&ev(AgentType::ClaudeCode, "long", AgentEvent::TaskStarted));
        let m = sm.get_or_create_session("long", None);
        // silent for 25 s by the clock, but a heartbeat arrived just now
        m.handle_event(AgentEvent::TaskRunning, Some(now_ms() - 25_000), None, None);
        m.handle_event(AgentEvent::Heartbeat, Some(now_ms()), None, None);
        assert_eq!(sm.get_all_sessions()[0].status, AgentStatus::Working, "a heartbeat is life");

        m.handle_event(AgentEvent::WaitingForInput, None, None, None);
        m.handle_event(AgentEvent::Heartbeat, None, None, None);
        assert_eq!(m.get_status(), AgentStatus::WaitingForInput, "a heartbeat does not resume a session that waits for the user");

        m.handle_event(AgentEvent::TaskCompleted, None, None, None);
        m.handle_event(AgentEvent::Heartbeat, None, None, None);
        assert_eq!(m.get_status(), AgentStatus::Completed, "nor reopen a finished one");
        assert_eq!(m.get_session().total_tasks_completed, 1);
        sm.clear_all();
    }
}
