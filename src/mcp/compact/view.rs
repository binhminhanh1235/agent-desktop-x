use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Mutex, MutexGuard, OnceLock, PoisonError},
    time::{Duration, Instant},
};

use agent_desktop_core::{
    AppError, Deadline, PlatformAdapter, WindowFilter,
    runtime_events::{
        RuntimeEvent, RuntimeEventCursor, RuntimeEventMetrics, runtime_event_cursor,
        runtime_event_metrics, runtime_events_since,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};

#[path = "view_state.rs"]
mod view_state;
use view_state::{canonical_windows, encoded_len};

const VIEW_CAPACITY: usize = 64;
const VIEW_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct ViewRequest {
    #[serde(default)]
    pub(super) previous_view_id: Option<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ViewId(String);

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObservationScope {
    command: String,
    app: Option<String>,
}

#[derive(Clone, Debug)]
struct StoredView {
    id: ViewId,
    generation: u64,
    expires_at: Instant,
    scope: ObservationScope,
    state: BTreeMap<String, Value>,
    parent: Option<ViewId>,
    invalidated: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct ViewInvalidationMetrics {
    events_applied: u64,
    views_invalidated: u64,
    unrelated_views_retained: u64,
    overflow_resets: u64,
}

#[derive(Debug)]
struct ViewStore {
    capacity: usize,
    ttl: Duration,
    next_id: u64,
    next_generation: u64,
    entries: BTreeMap<ViewId, StoredView>,
    insertion_order: VecDeque<ViewId>,
    event_cursor: RuntimeEventCursor,
    event_metrics: RuntimeEventMetrics,
    invalidation: ViewInvalidationMetrics,
}

pub(super) struct ViewObservation {
    pub(super) metadata: Value,
    pub(super) delta: Option<Value>,
}

impl ViewStore {
    fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            capacity: capacity.max(1),
            ttl,
            next_id: 1,
            next_generation: 1,
            entries: BTreeMap::new(),
            insertion_order: VecDeque::new(),
            event_cursor: runtime_event_cursor(),
            event_metrics: runtime_event_metrics(),
            invalidation: ViewInvalidationMetrics::default(),
        }
    }

    fn apply_pending_events(&mut self) {
        let batch = runtime_events_since(self.event_cursor);
        self.event_cursor = batch.cursor;
        self.event_metrics = batch.metrics;
        if batch.overflowed {
            self.invalidate_all_for_overflow();
        }
        for event in batch.events {
            self.apply_event(&event);
        }
    }

    fn invalidate_all_for_overflow(&mut self) {
        let mut invalidated = 0_u64;
        for view in self.entries.values_mut() {
            if !view.invalidated {
                view.invalidated = true;
                invalidated = invalidated.saturating_add(1);
            }
        }
        self.invalidation.views_invalidated = self
            .invalidation
            .views_invalidated
            .saturating_add(invalidated);
        self.invalidation.overflow_resets = self.invalidation.overflow_resets.saturating_add(1);
    }

    fn apply_event(&mut self, event: &RuntimeEvent) {
        let mut invalidated = 0_u64;
        let mut retained = 0_u64;
        for view in self.entries.values_mut() {
            if event_invalidates_scope(event, &view.scope) {
                if !view.invalidated {
                    view.invalidated = true;
                    invalidated = invalidated.saturating_add(1);
                }
            } else {
                retained = retained.saturating_add(1);
            }
        }
        self.invalidation.events_applied = self.invalidation.events_applied.saturating_add(1);
        self.invalidation.views_invalidated = self
            .invalidation
            .views_invalidated
            .saturating_add(invalidated);
        self.invalidation.unrelated_views_retained = self
            .invalidation
            .unrelated_views_retained
            .saturating_add(retained);
    }

    fn live(&mut self, raw_id: &str, now: Instant) -> Result<StoredView, AppError> {
        let id = parse_view_id(raw_id)?;
        let Some(view) = self.entries.get(&id).cloned() else {
            return Err(view_error(
                "VIEW_UNKNOWN",
                format!("view '{}' is unknown in this process", raw_id),
            ));
        };
        if now >= view.expires_at {
            self.remove(&id);
            return Err(view_error(
                "VIEW_EXPIRED",
                format!("view '{}' has expired", raw_id),
            ));
        }
        if view.invalidated {
            return Err(view_error(
                "VIEW_STALE",
                format!(
                    "view '{}' was invalidated by a runtime lifecycle event",
                    raw_id
                ),
            ));
        }
        Ok(view)
    }

    fn previous(
        &mut self,
        raw_id: &str,
        scope: &ObservationScope,
        now: Instant,
    ) -> Result<StoredView, AppError> {
        let view = self.live(raw_id, now)?;
        if &view.scope != scope {
            return Err(view_error(
                "VIEW_SCOPE_MISMATCH",
                "previous view belongs to an incompatible observation scope",
            ));
        }
        Ok(view)
    }

    fn insert(
        &mut self,
        scope: ObservationScope,
        state: BTreeMap<String, Value>,
        parent: Option<ViewId>,
        now: Instant,
    ) -> StoredView {
        while self.entries.len() >= self.capacity {
            let Some(oldest) = self.insertion_order.pop_front() else {
                break;
            };
            self.entries.remove(&oldest);
        }
        let id = ViewId(format!("v1-{:016x}", self.next_id));
        self.next_id = self.next_id.saturating_add(1);
        let generation = self.next_generation;
        self.next_generation = self.next_generation.saturating_add(1);
        let view = StoredView {
            id: id.clone(),
            generation,
            expires_at: now + self.ttl,
            scope,
            state,
            parent,
            invalidated: false,
        };
        self.insertion_order.push_back(id.clone());
        self.entries.insert(id, view.clone());
        view
    }

    fn remove(&mut self, id: &ViewId) {
        self.entries.remove(id);
        self.insertion_order.retain(|candidate| candidate != id);
    }
}

pub(super) fn record_observation(
    command: &str,
    args: &Value,
    result: &Value,
    request: &ViewRequest,
) -> Result<ViewObservation, AppError> {
    if command != "list-windows" {
        return Err(view_error(
            "VIEW_UNSUPPORTED",
            "P1A view handles currently support only the list-windows observation surface",
        ));
    }
    let scope = scope_for(command, args)?;
    let current = canonical_windows(result)?;
    let full_result_bytes = encoded_len(result)?;
    let now = Instant::now();
    let mut store = global_store();

    let previous = request
        .previous_view_id
        .as_deref()
        .map(|id| store.previous(id, &scope, now))
        .transpose()?;
    let delta = previous
        .as_ref()
        .map(|prior| super::view_delta::diff(&prior.state, &current))
        .transpose()?;
    let parent = previous.as_ref().map(|prior| prior.id.clone());
    let view = store.insert(scope.clone(), current.clone(), parent, now);
    let delta_entries = delta
        .as_ref()
        .map(super::view_delta::entry_count)
        .unwrap_or(0);
    let delta_bytes = delta.as_ref().map(encoded_len).transpose()?.unwrap_or(0);

    let metadata = json!({
        "view_id": view.id.0,
        "generation": view.generation,
        "created_sequence": view.generation,
        "ttl_ms": store.ttl.as_millis() as u64,
        "scope": {
            "command": scope.command,
            "app": scope.app,
        },
        "parent_view_id": view.parent.as_ref().map(|id| id.0.as_str()),
        "metrics": {
            "full_entries": current.len(),
            "delta_entries": delta_entries,
            "full_result_bytes": full_result_bytes,
            "delta_payload_bytes": delta_bytes,
            "harness_calls_this_observation": 1,
            "event_queue_depth": store.event_metrics.depth,
            "event_queue_max_depth": store.event_metrics.max_depth,
            "event_queue_dropped": store.event_metrics.dropped,
            "invalidation_events_applied": store.invalidation.events_applied,
            "views_invalidated": store.invalidation.views_invalidated,
            "unrelated_views_retained": store.invalidation.unrelated_views_retained,
            "event_overflow_resets": store.invalidation.overflow_resets,
        }
    });
    Ok(ViewObservation { metadata, delta })
}

pub(super) fn validate_expected_view(
    raw_id: &str,
    adapter: &dyn PlatformAdapter,
    deadline: Deadline,
) -> Result<Value, AppError> {
    let view = {
        let mut store = global_store();
        store.live(raw_id, Instant::now())?
    };
    if view.scope.command != "list-windows" {
        return Err(view_error(
            "VIEW_UNSUPPORTED",
            "expected view does not belong to a freshness-checkable P1A surface",
        ));
    }
    let windows = adapter.list_windows(
        &WindowFilter {
            focused_only: false,
            app: view.scope.app.clone(),
        },
        deadline,
    )?;
    let current = canonical_windows(&serde_json::to_value(windows)?)?;
    if current != view.state {
        return Err(view_error(
            "VIEW_STALE",
            "desktop state no longer matches the expected observation view",
        ));
    }
    Ok(json!({
        "state": "passed",
        "expected_view_id": view.id.0,
        "generation": view.generation,
        "scope": {
            "command": view.scope.command,
            "app": view.scope.app,
        }
    }))
}

fn global_store() -> MutexGuard<'static, ViewStore> {
    static STORE: OnceLock<Mutex<ViewStore>> = OnceLock::new();
    let mut store = STORE
        .get_or_init(|| Mutex::new(ViewStore::new(VIEW_CAPACITY, VIEW_TTL)))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    store.apply_pending_events();
    store
}

fn event_invalidates_scope(event: &RuntimeEvent, scope: &ObservationScope) -> bool {
    let event_app = match event {
        RuntimeEvent::ProcessStarted { current } => Some(current.app()),
        RuntimeEvent::ProcessReplaced { previous, .. }
        | RuntimeEvent::ProcessExited { previous } => Some(previous.app()),
        RuntimeEvent::WindowCreated { current } => Some(current.process().app()),
        RuntimeEvent::WindowDestroyed { previous }
        | RuntimeEvent::WindowGenerationChanged { previous, .. } => Some(previous.process().app()),
        RuntimeEvent::AccessibilityTreeInvalidated { process } => Some(process.app()),
        RuntimeEvent::ProviderReset => None,
    };
    match event_app {
        None => true,
        Some(event_app) => scope
            .app
            .as_deref()
            .is_none_or(|app| app.eq_ignore_ascii_case(event_app.trim())),
    }
}

fn scope_for(command: &str, args: &Value) -> Result<ObservationScope, AppError> {
    let app = match args.get("app") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => {
            let normalized = value.trim().to_lowercase();
            (!normalized.is_empty()).then_some(normalized)
        }
        Some(_) => {
            return Err(view_error(
                "VIEW_SCOPE_INVALID",
                "list-windows app scope must be a string when view evidence is requested",
            ));
        }
    };
    Ok(ObservationScope {
        command: command.to_string(),
        app,
    })
}

fn parse_view_id(raw: &str) -> Result<ViewId, AppError> {
    let valid = raw.len() == 19
        && raw.starts_with("v1-")
        && raw[3..].bytes().all(|byte| byte.is_ascii_hexdigit());
    if !valid {
        return Err(view_error(
            "VIEW_ID_INVALID",
            "view id is not a bounded P1A view identifier",
        ));
    }
    Ok(ViewId(raw.to_string()))
}

fn view_error(kind: &str, message: impl AsRef<str>) -> AppError {
    AppError::invalid_input_with_suggestion(
        format!("{kind}: {}", message.as_ref()),
        "Perform a fresh desktop.observe view request in the same compatible scope; views are short-lived observation evidence and never mutation authorization.",
    )
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
