use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Mutex, MutexGuard, OnceLock, PoisonError},
    time::{Duration, Instant},
};

use agent_desktop_core::AppError;
use serde::Deserialize;
use serde_json::{Value, json};

const VIEW_CAPACITY: usize = 64;
const VIEW_TTL: Duration = Duration::from_secs(30);
const MAX_VIEW_ENTRIES: usize = 256;
const MAX_VIEW_STATE_BYTES: usize = 128 * 1024;
const MAX_DELTA_ENTRIES: usize = 256;
const MAX_DELTA_BYTES: usize = 64 * 1024;

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
    created_at: Instant,
    expires_at: Instant,
    scope: ObservationScope,
    state: BTreeMap<String, Value>,
    parent: Option<ViewId>,
}

#[derive(Debug)]
struct ViewStore {
    capacity: usize,
    ttl: Duration,
    next_id: u64,
    next_generation: u64,
    entries: BTreeMap<ViewId, StoredView>,
    insertion_order: VecDeque<ViewId>,
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
        }
    }

    fn previous(
        &mut self,
        raw_id: &str,
        scope: &ObservationScope,
        now: Instant,
    ) -> Result<StoredView, AppError> {
        let id = parse_view_id(raw_id)?;
        let Some(view) = self.entries.get(&id).cloned() else {
            return Err(view_error(
                "VIEW_UNKNOWN",
                format!("previous view '{}' is unknown in this process", raw_id),
            ));
        };
        if now >= view.expires_at {
            self.remove(&id);
            return Err(view_error(
                "VIEW_EXPIRED",
                format!("previous view '{}' has expired", raw_id),
            ));
        }
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
            created_at: now,
            expires_at: now + self.ttl,
            scope,
            state,
            parent,
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
        .map(|prior| diff(&prior.state, &current))
        .transpose()?;
    let parent = previous.as_ref().map(|prior| prior.id.clone());
    let view = store.insert(scope.clone(), current.clone(), parent, now);
    let delta_entries = delta.as_ref().map(delta_entry_count).unwrap_or(0);
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
        }
    });
    let _lifetime = (view.created_at, view.expires_at);
    Ok(ViewObservation { metadata, delta })
}

fn global_store() -> MutexGuard<'static, ViewStore> {
    static STORE: OnceLock<Mutex<ViewStore>> = OnceLock::new();
    STORE
        .get_or_init(|| Mutex::new(ViewStore::new(VIEW_CAPACITY, VIEW_TTL)))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
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

fn canonical_windows(result: &Value) -> Result<BTreeMap<String, Value>, AppError> {
    let windows = result.as_array().ok_or_else(|| {
        view_error(
            "VIEW_STATE_INVALID",
            "list-windows result was not an array and cannot be canonicalized",
        )
    })?;
    if windows.len() > MAX_VIEW_ENTRIES {
        return Err(view_error(
            "VIEW_STATE_LIMIT",
            format!("view contains more than {MAX_VIEW_ENTRIES} windows"),
        ));
    }

    let mut state = BTreeMap::new();
    for window in windows {
        let object = window.as_object().ok_or_else(|| {
            view_error("VIEW_STATE_INVALID", "window entry was not a JSON object")
        })?;
        let app = required_string(object.get("app_name"), "app_name")?;
        let title = required_string(object.get("title"), "title")?;
        let process_instance = required_string(object.get("process_instance"), "process_instance")?;
        let pid = object
            .get("pid")
            .and_then(Value::as_u64)
            .ok_or_else(|| view_error("VIEW_IDENTITY_UNSAFE", "window pid is unavailable"))?;
        let key = semantic_key(app, pid, process_instance, title);
        let safe_state = json!({
            "app_name": app,
            "pid": pid,
            "process_instance": process_instance,
            "title": title,
            "bounds": object.get("bounds").cloned().unwrap_or(Value::Null),
            "is_focused": object.get("is_focused").cloned().unwrap_or(Value::Null),
            "accessible": object.get("accessible").cloned().unwrap_or(Value::Null),
            "minimized": object.get("minimized").cloned().unwrap_or(Value::Null),
            "visible": object.get("visible").cloned().unwrap_or(Value::Null),
        });
        if state.insert(key, safe_state).is_some() {
            return Err(view_error(
                "VIEW_IDENTITY_AMBIGUOUS",
                "two windows share the same safe semantic identity; refusing to invent identity from runtime handles or array order",
            ));
        }
    }
    if encoded_len(&json!(state))? > MAX_VIEW_STATE_BYTES {
        return Err(view_error(
            "VIEW_STATE_LIMIT",
            format!("canonical view exceeds {MAX_VIEW_STATE_BYTES} bytes"),
        ));
    }
    Ok(state)
}

fn required_string<'a>(value: Option<&'a Value>, field: &str) -> Result<&'a str, AppError> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            view_error(
                "VIEW_IDENTITY_UNSAFE",
                format!(
                    "window {field} is unavailable; safe semantic identity cannot be established"
                ),
            )
        })
}

fn semantic_key(app: &str, pid: u64, process_instance: &str, title: &str) -> String {
    let material = format!(
        "{}:{}|{}|{}:{}|{}:{}",
        app.len(),
        app.to_lowercase(),
        pid,
        process_instance.len(),
        process_instance,
        title.len(),
        title
    );
    format!("wsem-{:016x}", fnv1a64(material.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn diff(
    previous: &BTreeMap<String, Value>,
    current: &BTreeMap<String, Value>,
) -> Result<Value, AppError> {
    let added = current
        .iter()
        .filter(|(key, _)| !previous.contains_key(*key))
        .map(|(key, state)| json!({ "key": key, "state": state }))
        .collect::<Vec<_>>();
    let removed = previous
        .keys()
        .filter(|key| !current.contains_key(*key))
        .map(|key| json!({ "key": key }))
        .collect::<Vec<_>>();
    let changed = current
        .iter()
        .filter_map(|(key, state)| {
            previous
                .get(key)
                .filter(|previous_state| *previous_state != state)
                .map(|_| json!({ "key": key, "state": state }))
        })
        .collect::<Vec<_>>();
    let count = added.len() + removed.len() + changed.len();
    if count > MAX_DELTA_ENTRIES {
        return Err(view_error(
            "VIEW_DELTA_LIMIT",
            format!("state delta exceeds {MAX_DELTA_ENTRIES} entries"),
        ));
    }
    let delta = json!({
        "added": added,
        "removed": removed,
        "changed": changed,
    });
    if encoded_len(&delta)? > MAX_DELTA_BYTES {
        return Err(view_error(
            "VIEW_DELTA_LIMIT",
            format!("state delta exceeds {MAX_DELTA_BYTES} bytes"),
        ));
    }
    Ok(delta)
}

fn delta_entry_count(delta: &Value) -> usize {
    ["added", "removed", "changed"]
        .iter()
        .map(|name| delta[*name].as_array().map(Vec::len).unwrap_or(0))
        .sum()
}

fn encoded_len(value: &Value) -> Result<usize, AppError> {
    Ok(serde_json::to_vec(value)?.len())
}

fn parse_view_id(raw: &str) -> Result<ViewId, AppError> {
    let valid = raw.len() == 19
        && raw.starts_with("v1-")
        && raw[3..].bytes().all(|byte| byte.is_ascii_hexdigit());
    if !valid {
        return Err(view_error(
            "VIEW_ID_INVALID",
            "previous_view_id is not a bounded P1A view identifier",
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
