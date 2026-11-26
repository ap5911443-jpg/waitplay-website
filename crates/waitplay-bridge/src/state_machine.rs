//! Per-session agent state machine (port of `AgentStateMachine` in packages/shared).
//!
//! `task_started` moves a session to WORKING and arms a launch timer; if the task is still running when it fires the
//! session becomes GAME_ACTIVE. Completing, failing, ending or waiting for input before that cancels the launch.
//! Listeners are always invoked after the internal lock is released, so they may call back into the machine.

use crate::js;
use serde_json::{Map, Value};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Duration;
use tokio::task::JoinHandle;
use waitplay_core::{now_ms, AgentEvent, AgentSessionInfo, AgentStatus, AgentType};

pub const DEFAULT_START_DELAY_MS: i64 = 4000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StopReason {
    Completed,
    Failed,
    Ended,
}

/// Observer hooks; every method defaults to a no-op so listeners implement only what they need.
pub trait StateMachineListener: Send + Sync {
    fn on_status_change(&self, _new_status: AgentStatus, _prev_status: AgentStatus, _session: &AgentSessionInfo) {}
    fn on_game_launch_requested(&self, _session: &AgentSessionInfo) {}
    fn on_game_stop_requested(&self, _session: &AgentSessionInfo, _reason: StopReason) {}
    /// The only pause reason is "input_required".
    fn on_game_pause_requested(&self, _session: &AgentSessionInfo) {}
    fn on_game_resume_requested(&self, _session: &AgentSessionInfo) {}
}

#[derive(Clone, Copy, Default, Debug)]
pub struct StateMachineOptions {
    pub agent: Option<AgentType>,
    /// Default 4000.
    pub start_delay_ms: Option<i64>,
    /// Default true.
    pub auto_launch: Option<bool>,
}

enum Note {
    Status(AgentStatus, AgentStatus, AgentSessionInfo),
    Launch(AgentSessionInfo),
    Stop(AgentSessionInfo, StopReason),
    Pause(AgentSessionInfo),
    Resume(AgentSessionInfo),
}

struct LaunchTimer {
    generation: u64,
    handle: JoinHandle<()>,
}

struct Inner {
    session: AgentSessionInfo,
    start_delay_ms: i64,
    auto_launch: bool,
    listeners: Vec<(usize, Arc<dyn StateMachineListener>)>,
    next_listener_id: usize,
    launch_timer: Option<LaunchTimer>,
    timer_generation: u64,
}

pub struct AgentStateMachine {
    me: Weak<AgentStateMachine>,
    inner: Mutex<Inner>,
}

impl Inner {
    fn set_status(&mut self, new_status: AgentStatus, out: &mut Vec<Note>) {
        let prev = self.session.status;
        if prev == new_status {
            return;
        }
        self.session.status = new_status;
        self.session.last_event_at = now_ms();
        out.push(Note::Status(new_status, prev, self.session.clone()));
    }

    fn clear_launch_timer(&mut self) {
        if let Some(t) = self.launch_timer.take() {
            t.handle.abort();
        }
    }

    fn trigger_game_active(&mut self, out: &mut Vec<Note>) {
        self.set_status(AgentStatus::GameActive, out);
        out.push(Note::Launch(self.session.clone()));
    }
}

impl AgentStateMachine {
    /// Must be called inside a Tokio runtime when `task_started` events will be handled (the launch timer needs one).
    pub fn new(session_id: &str, options: StateMachineOptions) -> Arc<Self> {
        let agent = options.agent.unwrap_or(AgentType::ClaudeCode);
        let now = now_ms();
        Arc::new_cyclic(|me| AgentStateMachine {
            me: me.clone(),
            inner: Mutex::new(Inner {
                session: AgentSessionInfo {
                    session_id: session_id.to_string(),
                    agent,
                    status: AgentStatus::Idle,
                    started_at: now,
                    last_event_at: now,
                    current_task_started_at: None,
                    last_task_duration_ms: None,
                    total_tasks_completed: 0,
                    waiting_for_input_prompt: None,
                    last_error: None,
                    model: None,
                    tool: None,
                    intent: None,
                    agent_display_name: None,
                    terminal: None,
                },
                start_delay_ms: options.start_delay_ms.unwrap_or(DEFAULT_START_DELAY_MS),
                auto_launch: options.auto_launch.unwrap_or(true),
                listeners: Vec::new(),
                next_listener_id: 0,
                launch_timer: None,
                timer_generation: 0,
            }),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_agent(&self, agent: AgentType) {
        self.lock().session.agent = agent;
    }

    pub fn get_session(&self) -> AgentSessionInfo {
        self.lock().session.clone()
    }

    pub fn get_status(&self) -> AgentStatus {
        self.lock().session.status
    }

    pub fn update_options(&self, options: StateMachineOptions) {
        let mut g = self.lock();
        if let Some(d) = options.start_delay_ms {
            g.start_delay_ms = d;
        }
        if let Some(a) = options.auto_launch {
            g.auto_launch = a;
        }
    }

    /// Returns an id for [`AgentStateMachine::unsubscribe`].
    pub fn subscribe(&self, listener: Arc<dyn StateMachineListener>) -> usize {
        let mut g = self.lock();
        let id = g.next_listener_id;
        g.next_listener_id += 1;
        g.listeners.push((id, listener));
        id
    }

    pub fn unsubscribe(&self, id: usize) {
        self.lock().listeners.retain(|(i, _)| *i != id);
    }

    fn dispatch(&self, notes: Vec<Note>) {
        if notes.is_empty() {
            return;
        }
        let listeners: Vec<Arc<dyn StateMachineListener>> = self.lock().listeners.iter().map(|(_, l)| l.clone()).collect();
        for note in &notes {
            for l in &listeners {
                match note {
                    Note::Status(new, prev, s) => l.on_status_change(*new, *prev, s),
                    Note::Launch(s) => l.on_game_launch_requested(s),
                    Note::Stop(s, r) => l.on_game_stop_requested(s, *r),
                    Note::Pause(s) => l.on_game_pause_requested(s),
                    Note::Resume(s) => l.on_game_resume_requested(s),
                }
            }
        }
    }

    /// `handleEvent` with the TypeScript defaults: `timestamp` falls back to now.
    pub fn handle_event(
        &self,
        event: AgentEvent,
        timestamp: Option<i64>,
        data: Option<&Map<String, Value>>,
        agent: Option<AgentType>,
    ) -> AgentStatus {
        let timestamp = timestamp.unwrap_or_else(now_ms);
        let mut notes = Vec::new();
        let status = {
            let mut g = self.lock();
            self.apply(&mut g, event, timestamp, data, agent, &mut notes);
            g.session.status
        };
        self.dispatch(notes);
        status
    }

    fn apply(
        &self,
        g: &mut Inner,
        event: AgentEvent,
        timestamp: i64,
        data: Option<&Map<String, Value>>,
        agent: Option<AgentType>,
        out: &mut Vec<Note>,
    ) {
        if event == AgentEvent::Heartbeat {
            // only a session that is working needs it; it must never wake a finished or waiting one
            if matches!(g.session.status, AgentStatus::Working | AgentStatus::GameActive) {
                g.session.last_event_at = timestamp;
            }
            return;
        }
        g.session.last_event_at = timestamp;
        if let Some(a) = agent {
            g.session.agent = a;
        }
        if let Some(d) = data {
            if js::truthy(d.get("tool")) {
                g.session.tool = d.get("tool").map(js::to_js_string);
            }
            if js::truthy(d.get("model")) {
                g.session.model = d.get("model").map(js::to_js_string);
            }
            if js::truthy(d.get("intent")) {
                g.session.intent = d.get("intent").map(js::to_js_string);
            }
            if js::truthy(d.get("terminal")) {
                g.session.terminal = d.get("terminal").map(js::to_js_string);
            }
        }

        match event {
            AgentEvent::SessionStarted => {
                g.clear_launch_timer();
                g.session.started_at = timestamp;
                g.set_status(AgentStatus::Idle, out);
            }

            AgentEvent::TaskStarted => {
                // A new task cancels any pending launch.
                g.clear_launch_timer();
                g.session.current_task_started_at = Some(timestamp);
                g.session.waiting_for_input_prompt = None;
                g.session.last_error = None;

                g.set_status(AgentStatus::Working, out);

                if g.auto_launch {
                    if g.start_delay_ms <= 0 {
                        g.trigger_game_active(out);
                    } else {
                        self.arm_launch_timer(g);
                    }
                }
            }

            AgentEvent::TaskRunning => match g.session.status {
                AgentStatus::Idle | AgentStatus::Completed => {
                    // Arrives directly, or resumes after an intermediate completion.
                    self.apply(g, AgentEvent::TaskStarted, timestamp, data, None, out);
                }
                AgentStatus::WaitingForInput => {
                    if g.launch_timer.is_none() && g.auto_launch {
                        g.trigger_game_active(out);
                    } else {
                        g.set_status(AgentStatus::Working, out);
                    }
                    out.push(Note::Resume(g.session.clone()));
                }
                _ => {}
            },

            AgentEvent::WaitingForInput => {
                g.clear_launch_timer();
                let was_game_active = g.session.status == AgentStatus::GameActive;
                g.session.waiting_for_input_prompt = match data.and_then(|d| d.get("prompt")) {
                    Some(Value::String(s)) => Some(s.clone()),
                    _ => None,
                };
                g.set_status(AgentStatus::WaitingForInput, out);
                if was_game_active {
                    out.push(Note::Pause(g.session.clone()));
                }
            }

            AgentEvent::TaskCompleted => {
                let had_active_game = g.session.status == AgentStatus::GameActive;
                g.clear_launch_timer();
                if let Some(start) = g.session.current_task_started_at.filter(|s| *s != 0) {
                    g.session.last_task_duration_ms = Some(timestamp - start);
                }
                g.session.total_tasks_completed += 1;
                g.session.waiting_for_input_prompt = None;

                g.set_status(AgentStatus::Completed, out);

                if had_active_game {
                    out.push(Note::Stop(g.session.clone(), StopReason::Completed));
                }
            }

            AgentEvent::TaskFailed => {
                let had_active_game = g.session.status == AgentStatus::GameActive;
                g.clear_launch_timer();
                if let Some(start) = g.session.current_task_started_at.filter(|s| *s != 0) {
                    g.session.last_task_duration_ms = Some(timestamp - start);
                }
                g.session.last_error = Some(match data.and_then(|d| d.get("error")) {
                    Some(Value::String(s)) => s.clone(),
                    _ => "Task encountered an error".to_string(),
                });

                g.set_status(AgentStatus::Failed, out);

                if had_active_game {
                    out.push(Note::Stop(g.session.clone(), StopReason::Failed));
                }
            }

            AgentEvent::Heartbeat => {}

            AgentEvent::SessionEnded => {
                let had_active_game = g.session.status == AgentStatus::GameActive;
                g.clear_launch_timer();
                g.set_status(AgentStatus::Idle, out);
                if had_active_game {
                    out.push(Note::Stop(g.session.clone(), StopReason::Ended));
                }
            }
        }
    }

    fn arm_launch_timer(&self, g: &mut Inner) {
        g.timer_generation += 1;
        let generation = g.timer_generation;
        // The deadline is fixed now, not when the task is first polled.
        let deadline = tokio::time::Instant::now() + Duration::from_millis(g.start_delay_ms as u64);
        let me = self.me.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep_until(deadline).await;
            if let Some(machine) = me.upgrade() {
                machine.on_launch_timer(generation);
            }
        });
        g.launch_timer = Some(LaunchTimer { generation, handle });
    }

    fn on_launch_timer(&self, generation: u64) {
        let mut notes = Vec::new();
        {
            let mut g = self.lock();
            match &g.launch_timer {
                Some(t) if t.generation == generation => {}
                _ => return,
            }
            g.launch_timer = None;
            // Only launch if STILL working.
            if g.session.status == AgentStatus::Working {
                g.trigger_game_active(&mut notes);
            }
        }
        self.dispatch(notes);
    }

    pub fn destroy(&self) {
        let mut g = self.lock();
        g.clear_launch_timer();
        g.listeners.clear();
    }
}

impl Drop for AgentStateMachine {
    fn drop(&mut self) {
        if let Ok(g) = self.inner.get_mut() {
            g.clear_launch_timer();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Lets the spawned timer task run after virtual time was advanced.
    async fn tick(ms: u64) {
        tokio::time::advance(Duration::from_millis(ms)).await;
        for _ in 0..4 {
            tokio::task::yield_now().await;
        }
    }

    #[derive(Default)]
    struct Flags {
        launched: AtomicBool,
        stopped: AtomicBool,
        stopped_failed: AtomicBool,
        paused: AtomicBool,
        resumed: AtomicBool,
    }

    impl StateMachineListener for Flags {
        fn on_game_launch_requested(&self, _: &AgentSessionInfo) {
            self.launched.store(true, Ordering::SeqCst);
        }
        fn on_game_stop_requested(&self, _: &AgentSessionInfo, reason: StopReason) {
            self.stopped.store(true, Ordering::SeqCst);
            if reason == StopReason::Failed {
                self.stopped_failed.store(true, Ordering::SeqCst);
            }
        }
        fn on_game_pause_requested(&self, _: &AgentSessionInfo) {
            self.paused.store(true, Ordering::SeqCst);
        }
        fn on_game_resume_requested(&self, _: &AgentSessionInfo) {
            self.resumed.store(true, Ordering::SeqCst);
        }
    }

    fn opts(delay: i64) -> StateMachineOptions {
        StateMachineOptions { start_delay_ms: Some(delay), auto_launch: Some(true), ..Default::default() }
    }

    fn map(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[tokio::test(start_paused = true)]
    async fn initializes_in_idle_state() {
        let sm = AgentStateMachine::new("test-session", StateMachineOptions::default());
        assert_eq!(sm.get_status(), AgentStatus::Idle);
        assert_eq!(sm.get_session().session_id, "test-session");
        sm.destroy();
    }

    #[tokio::test(start_paused = true)]
    async fn task_started_transitions_to_working_and_starts_launch_delay() {
        let sm = AgentStateMachine::new("delay-session", opts(4000));
        let flags = Arc::new(Flags::default());
        sm.subscribe(flags.clone());

        sm.handle_event(AgentEvent::TaskStarted, None, None, None);
        assert_eq!(sm.get_status(), AgentStatus::Working);
        assert!(!flags.launched.load(Ordering::SeqCst), "Game should not launch immediately");

        tick(4001).await;

        assert_eq!(sm.get_status(), AgentStatus::GameActive);
        assert!(flags.launched.load(Ordering::SeqCst), "Game should launch after delay");
        sm.destroy();
    }

    #[tokio::test(start_paused = true)]
    async fn default_launch_delay_is_four_seconds() {
        let sm = AgentStateMachine::new("default-delay", StateMachineOptions::default());
        sm.handle_event(AgentEvent::TaskStarted, None, None, None);
        tick(3999).await;
        assert_eq!(sm.get_status(), AgentStatus::Working);
        tick(2).await;
        assert_eq!(sm.get_status(), AgentStatus::GameActive);
        sm.destroy();
    }

    #[tokio::test(start_paused = true)]
    async fn completion_before_launch_delay_cancels_game_launch() {
        let sm = AgentStateMachine::new("quick-task-session", opts(4000));
        let flags = Arc::new(Flags::default());
        sm.subscribe(flags.clone());

        sm.handle_event(AgentEvent::TaskStarted, None, None, None);
        assert_eq!(sm.get_status(), AgentStatus::Working);

        tick(1500).await;
        sm.handle_event(AgentEvent::TaskCompleted, None, None, None);
        assert_eq!(sm.get_status(), AgentStatus::Completed);

        tick(5000).await;

        assert!(!flags.launched.load(Ordering::SeqCst), "Game should not launch if task completed early");
        assert!(!flags.stopped.load(Ordering::SeqCst), "No game was running, so stop should not be requested");
        assert_eq!(sm.get_status(), AgentStatus::Completed);
        sm.destroy();
    }

    #[tokio::test(start_paused = true)]
    async fn waiting_for_input_pauses_game_and_resumes_upon_task_running() {
        let sm = AgentStateMachine::new("input-session", opts(1000));
        let flags = Arc::new(Flags::default());
        sm.subscribe(flags.clone());

        sm.handle_event(AgentEvent::TaskStarted, None, None, None);
        tick(1001).await;
        assert_eq!(sm.get_status(), AgentStatus::GameActive);

        sm.handle_event(AgentEvent::WaitingForInput, None, Some(&map(json!({"prompt": "Confirm tool run?"}))), None);
        assert_eq!(sm.get_status(), AgentStatus::WaitingForInput);
        assert!(flags.paused.load(Ordering::SeqCst));
        assert_eq!(sm.get_session().waiting_for_input_prompt.as_deref(), Some("Confirm tool run?"));

        sm.handle_event(AgentEvent::TaskRunning, None, None, None);
        assert_eq!(sm.get_status(), AgentStatus::GameActive);
        assert!(flags.resumed.load(Ordering::SeqCst));

        sm.destroy();
    }

    #[tokio::test(start_paused = true)]
    async fn task_failed_stops_active_game_and_records_error() {
        let sm = AgentStateMachine::new("fail-session", opts(1000));
        let flags = Arc::new(Flags::default());
        sm.subscribe(flags.clone());

        sm.handle_event(AgentEvent::TaskStarted, None, None, None);
        tick(1001).await;
        assert_eq!(sm.get_status(), AgentStatus::GameActive);

        sm.handle_event(AgentEvent::TaskFailed, None, Some(&map(json!({"error": "Network timeout"}))), None);
        assert_eq!(sm.get_status(), AgentStatus::Failed);
        assert!(flags.stopped_failed.load(Ordering::SeqCst));
        assert_eq!(sm.get_session().last_error.as_deref(), Some("Network timeout"));

        sm.destroy();
    }

    #[tokio::test(start_paused = true)]
    async fn new_task_cancels_previous_completed_state_for_multi_turn_conversations() {
        let sm = AgentStateMachine::new("multi-turn-session", opts(2000));

        sm.handle_event(AgentEvent::TaskStarted, None, None, None);
        sm.handle_event(AgentEvent::TaskCompleted, None, None, None);
        assert_eq!(sm.get_status(), AgentStatus::Completed);
        assert_eq!(sm.get_session().total_tasks_completed, 1);

        sm.handle_event(AgentEvent::TaskStarted, None, None, None);
        assert_eq!(sm.get_status(), AgentStatus::Working);

        sm.handle_event(AgentEvent::TaskCompleted, None, None, None);
        assert_eq!(sm.get_status(), AgentStatus::Completed);
        assert_eq!(sm.get_session().total_tasks_completed, 2);

        sm.destroy();
    }

    #[tokio::test(start_paused = true)]
    async fn zero_delay_launches_immediately_and_task_running_restarts() {
        let sm = AgentStateMachine::new("zero", opts(0));
        sm.handle_event(AgentEvent::TaskStarted, None, None, None);
        assert_eq!(sm.get_status(), AgentStatus::GameActive);
        sm.handle_event(AgentEvent::SessionEnded, None, None, None);
        assert_eq!(sm.get_status(), AgentStatus::Idle);
        // task_running from IDLE behaves like task_started
        sm.handle_event(AgentEvent::TaskRunning, None, None, None);
        assert_eq!(sm.get_status(), AgentStatus::GameActive);
        sm.destroy();
    }

    #[tokio::test(start_paused = true)]
    async fn event_data_updates_session_metadata_and_duration() {
        let sm = AgentStateMachine::new("meta", opts(0));
        sm.handle_event(
            AgentEvent::TaskStarted,
            Some(1000),
            Some(&map(json!({"tool": "bash", "model": "m1", "intent": 7, "terminal": ""}))),
            Some(AgentType::Omp),
        );
        sm.handle_event(AgentEvent::TaskCompleted, Some(4500), None, None);
        let s = sm.get_session();
        assert_eq!(s.tool.as_deref(), Some("bash"));
        assert_eq!(s.model.as_deref(), Some("m1"));
        assert_eq!(s.intent.as_deref(), Some("7"));
        assert_eq!(s.terminal, None, "falsy values are ignored");
        assert_eq!(s.agent, AgentType::Omp);
        assert_eq!(s.last_task_duration_ms, Some(3500));
        sm.destroy();
    }
}
