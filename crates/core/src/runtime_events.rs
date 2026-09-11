use crate::{AppInfo, ProcessId, SignalBaseline, WindowInfo, search_text};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Mutex, OnceLock, PoisonError},
};

/// Runtime invalidation is deliberately small: enough history to bridge short
/// producer/consumer gaps without turning desktop state into an event log.
pub const RUNTIME_EVENT_CAPACITY: usize = 128;
const MAX_SCOPE_TEXT_BYTES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProcessScope {
    app: String,
    pid: ProcessId,
    process_instance: String,
}

impl RuntimeProcessScope {
    pub fn new(
        app: impl Into<String>,
        pid: ProcessId,
        process_instance: impl Into<String>,
    ) -> Option<Self> {
        let app = app.into();
        let process_instance = process_instance.into();
        if !bounded_text(&app) || !bounded_text(&process_instance) || pid.get() == 0 {
            return None;
        }
        Some(Self {
            app,
            pid,
            process_instance,
        })
    }

    pub fn app(&self) -> &str {
        &self.app
    }

    pub fn pid(&self) -> ProcessId {
        self.pid
    }

    pub fn process_instance(&self) -> &str {
        &self.process_instance
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWindowScope {
    process: RuntimeProcessScope,
    title: String,
}

impl RuntimeWindowScope {
    pub fn new(process: RuntimeProcessScope, title: impl Into<String>) -> Option<Self> {
        let title = title.into();
        if !bounded_text(&title) {
            return None;
        }
        Some(Self { process, title })
    }

    pub fn process(&self) -> &RuntimeProcessScope {
        &self.process
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeEvent {
    ProcessStarted {
        current: RuntimeProcessScope,
    },
    ProcessReplaced {
        previous: RuntimeProcessScope,
        current: RuntimeProcessScope,
    },
    ProcessExited {
        previous: RuntimeProcessScope,
    },
    WindowCreated {
        current: RuntimeWindowScope,
    },
    WindowDestroyed {
        previous: RuntimeWindowScope,
    },
    WindowGenerationChanged {
        previous: RuntimeWindowScope,
        current: RuntimeWindowScope,
    },
    AccessibilityTreeInvalidated {
        process: RuntimeProcessScope,
    },
    /// Conservative fallback for a provider/session reset or for a native
    /// notification that cannot be represented inside the bounded semantic
    /// event model.
    ProviderReset,
}

impl RuntimeEvent {
    fn sort_key(&self) -> (u8, String) {
        match self {
            Self::ProcessExited { previous } => (0, process_key(previous)),
            Self::ProcessReplaced { previous, .. } => (1, process_key(previous)),
            Self::WindowDestroyed { previous } => (2, window_key(previous)),
            Self::WindowGenerationChanged { previous, .. } => (3, window_key(previous)),
            Self::AccessibilityTreeInvalidated { process } => (4, process_key(process)),
            Self::WindowCreated { current } => (5, window_key(current)),
            Self::ProcessStarted { current } => (6, process_key(current)),
            Self::ProviderReset => (7, String::new()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeEventCursor(u64);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeEventMetrics {
    pub capacity: usize,
    pub depth: usize,
    pub max_depth: usize,
    pub published: u64,
    pub dropped: u64,
}

#[derive(Clone, Debug)]
pub struct RuntimeEventBatch {
    pub events: Vec<RuntimeEvent>,
    pub cursor: RuntimeEventCursor,
    pub overflowed: bool,
    pub metrics: RuntimeEventMetrics,
}

#[derive(Clone, Debug)]
struct Envelope {
    sequence: u64,
    event: RuntimeEvent,
}

#[derive(Debug)]
struct RuntimeEventBus {
    capacity: usize,
    next_sequence: u64,
    retained: VecDeque<Envelope>,
    published: u64,
    dropped: u64,
    max_depth: usize,
}

impl RuntimeEventBus {
    fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            next_sequence: 1,
            retained: VecDeque::new(),
            published: 0,
            dropped: 0,
            max_depth: 0,
        }
    }

    fn cursor(&self) -> RuntimeEventCursor {
        RuntimeEventCursor(self.next_sequence.saturating_sub(1))
    }

    fn publish(&mut self, event: RuntimeEvent) {
        if self.retained.len() == self.capacity {
            self.retained.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.published = self.published.saturating_add(1);
        self.retained.push_back(Envelope { sequence, event });
        self.max_depth = self.max_depth.max(self.retained.len());
    }

    fn read_since(&self, cursor: RuntimeEventCursor) -> RuntimeEventBatch {
        let tail = self.cursor();
        let oldest = self.retained.front().map(|event| event.sequence);
        let overflowed =
            cursor.0 > tail.0 || oldest.is_some_and(|oldest| cursor.0.saturating_add(1) < oldest);
        let events = self
            .retained
            .iter()
            .filter(|event| event.sequence > cursor.0)
            .map(|event| event.event.clone())
            .collect();
        RuntimeEventBatch {
            events,
            cursor: tail,
            overflowed,
            metrics: self.metrics(),
        }
    }

    fn metrics(&self) -> RuntimeEventMetrics {
        RuntimeEventMetrics {
            capacity: self.capacity,
            depth: self.retained.len(),
            max_depth: self.max_depth,
            published: self.published,
            dropped: self.dropped,
        }
    }
}

static RUNTIME_EVENTS: OnceLock<Mutex<RuntimeEventBus>> = OnceLock::new();

fn global_bus() -> &'static Mutex<RuntimeEventBus> {
    RUNTIME_EVENTS.get_or_init(|| Mutex::new(RuntimeEventBus::new(RUNTIME_EVENT_CAPACITY)))
}

fn lock_bus() -> std::sync::MutexGuard<'static, RuntimeEventBus> {
    global_bus().lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn runtime_event_cursor() -> RuntimeEventCursor {
    lock_bus().cursor()
}

pub fn runtime_events_since(cursor: RuntimeEventCursor) -> RuntimeEventBatch {
    lock_bus().read_since(cursor)
}

pub fn runtime_event_metrics() -> RuntimeEventMetrics {
    lock_bus().metrics()
}

pub fn publish_runtime_event(event: RuntimeEvent) {
    lock_bus().publish(event);
}

/// Converts two already-normalized provider snapshots into bounded semantic
/// invalidation events and publishes them. Native window ids participate only
/// in this transient comparison; they are never copied into an event.
pub fn publish_runtime_invalidation_diff(
    previous: &SignalBaseline,
    current: &SignalBaseline,
) -> usize {
    let events = runtime_invalidation_diff(previous, current);
    let count = events.len();
    if count == 0 {
        return 0;
    }
    let mut bus = lock_bus();
    for event in events {
        bus.publish(event);
    }
    count
}

pub fn runtime_invalidation_diff(
    previous: &SignalBaseline,
    current: &SignalBaseline,
) -> Vec<RuntimeEvent> {
    let mut events = Vec::new();
    if previous.completeness.apps && current.completeness.apps {
        diff_apps(previous, current, &mut events);
    }
    if previous.completeness.windows && current.completeness.windows {
        diff_windows(previous, current, &mut events);
    }
    events.sort_by_key(RuntimeEvent::sort_key);
    events.dedup();
    events
}

fn diff_apps(previous: &SignalBaseline, current: &SignalBaseline, events: &mut Vec<RuntimeEvent>) {
    let previous_groups = group_apps(&previous.apps);
    let current_groups = group_apps(&current.apps);
    let mut keys = previous_groups
        .keys()
        .chain(current_groups.keys())
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();

    for key in keys {
        let old = previous_groups.get(&key).cloned().unwrap_or_default();
        let new = current_groups.get(&key).cloned().unwrap_or_default();
        if old.len() == 1 && new.len() == 1 {
            let old_app = &previous.apps[old[0]];
            let new_app = &current.apps[new[0]];
            if app_runtime_identity(old_app) != app_runtime_identity(new_app) {
                if let (Some(previous), Some(current)) =
                    (process_scope(old_app), process_scope(new_app))
                {
                    events.push(RuntimeEvent::ProcessReplaced { previous, current });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
            continue;
        }

        for index in old {
            let old_app = &previous.apps[index];
            if !new.iter().any(|candidate| {
                app_runtime_identity(old_app) == app_runtime_identity(&current.apps[*candidate])
            }) {
                if let Some(previous) = process_scope(old_app) {
                    events.push(RuntimeEvent::ProcessExited { previous });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
        }
        for index in new {
            let new_app = &current.apps[index];
            if !previous_groups
                .get(&key)
                .into_iter()
                .flatten()
                .any(|candidate| {
                    app_runtime_identity(new_app)
                        == app_runtime_identity(&previous.apps[*candidate])
                })
            {
                if let Some(current) = process_scope(new_app) {
                    events.push(RuntimeEvent::ProcessStarted { current });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
        }
    }
}

fn diff_windows(
    previous: &SignalBaseline,
    current: &SignalBaseline,
    events: &mut Vec<RuntimeEvent>,
) {
    let previous_groups = group_windows(&previous.windows);
    let current_groups = group_windows(&current.windows);
    let mut keys = previous_groups
        .keys()
        .chain(current_groups.keys())
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();

    for key in keys {
        let old = previous_groups.get(&key).cloned().unwrap_or_default();
        let new = current_groups.get(&key).cloned().unwrap_or_default();
        if old.len() == 1 && new.len() == 1 {
            let old_window = &previous.windows[old[0]];
            let new_window = &current.windows[new[0]];
            if window_runtime_identity(old_window) != window_runtime_identity(new_window) {
                if let (Some(previous), Some(current)) =
                    (window_scope(old_window), window_scope(new_window))
                {
                    events.push(RuntimeEvent::WindowGenerationChanged { previous, current });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
            continue;
        }

        for index in old {
            let old_window = &previous.windows[index];
            if !new.iter().any(|candidate| {
                window_runtime_identity(old_window)
                    == window_runtime_identity(&current.windows[*candidate])
            }) {
                if let Some(previous) = window_scope(old_window) {
                    events.push(RuntimeEvent::WindowDestroyed { previous });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
        }
        for index in new {
            let new_window = &current.windows[index];
            if !previous_groups
                .get(&key)
                .into_iter()
                .flatten()
                .any(|candidate| {
                    window_runtime_identity(new_window)
                        == window_runtime_identity(&previous.windows[*candidate])
                })
            {
                if let Some(current) = window_scope(new_window) {
                    events.push(RuntimeEvent::WindowCreated { current });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
        }
    }
}

fn group_apps(apps: &[AppInfo]) -> BTreeMap<String, Vec<usize>> {
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (index, app) in apps.iter().enumerate() {
        groups.entry(app_semantic_key(app)).or_default().push(index);
    }
    groups
}

fn group_windows(windows: &[WindowInfo]) -> BTreeMap<String, Vec<usize>> {
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (index, window) in windows.iter().enumerate() {
        groups
            .entry(format!(
                "{}\u{0}{}",
                search_text::normalize(&window.app),
                search_text::normalize(&window.title)
            ))
            .or_default()
            .push(index);
    }
    groups
}

fn app_semantic_key(app: &AppInfo) -> String {
    app.bundle_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| format!("bundle:{}", search_text::normalize(value)))
        .unwrap_or_else(|| format!("name:{}", search_text::normalize(&app.name)))
}

fn app_runtime_identity(app: &AppInfo) -> Option<(u32, &str)> {
    app.process_instance
        .as_deref()
        .map(|instance| (app.pid.get(), instance))
}

fn window_runtime_identity(window: &WindowInfo) -> Option<(u32, &str, &str)> {
    window
        .process_instance
        .as_deref()
        .map(|instance| (window.pid.get(), instance, window.id.as_str()))
}

fn process_scope(app: &AppInfo) -> Option<RuntimeProcessScope> {
    RuntimeProcessScope::new(
        app.name.clone(),
        app.pid,
        app.process_instance.as_deref()?.to_string(),
    )
}

fn window_scope(window: &WindowInfo) -> Option<RuntimeWindowScope> {
    let process = RuntimeProcessScope::new(
        window.app.clone(),
        window.pid,
        window.process_instance.as_deref()?.to_string(),
    )?;
    RuntimeWindowScope::new(process, window.title.clone())
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_SCOPE_TEXT_BYTES
}

fn process_key(scope: &RuntimeProcessScope) -> String {
    format!(
        "{}|{}|{}",
        search_text::normalize(scope.app()),
        scope.pid().get(),
        scope.process_instance()
    )
}

fn window_key(scope: &RuntimeWindowScope) -> String {
    format!(
        "{}|{}",
        process_key(scope.process()),
        search_text::normalize(scope.title())
    )
}

#[cfg(test)]
#[path = "runtime_events_tests.rs"]
mod tests;
