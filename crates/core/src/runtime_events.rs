use crate::{ProcessId, SignalBaseline};
use std::{
    collections::VecDeque,
    sync::{Mutex, OnceLock, PoisonError},
};

#[path = "runtime_event_diff.rs"]
mod runtime_event_diff;

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
    runtime_event_diff::runtime_invalidation_diff(previous, current)
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_SCOPE_TEXT_BYTES
}

#[cfg(test)]
#[path = "runtime_events_tests.rs"]
mod tests;
