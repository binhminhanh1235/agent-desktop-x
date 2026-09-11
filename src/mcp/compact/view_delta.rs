use std::collections::BTreeMap;

use agent_desktop_core::AppError;
use serde_json::{Value, json};

const MAX_DELTA_ENTRIES: usize = 256;
const MAX_DELTA_BYTES: usize = 64 * 1024;

pub(super) fn diff(
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
        return Err(delta_error(format!(
            "state delta exceeds {MAX_DELTA_ENTRIES} entries"
        )));
    }
    let delta = json!({
        "added": added,
        "removed": removed,
        "changed": changed,
    });
    if serde_json::to_vec(&delta)?.len() > MAX_DELTA_BYTES {
        return Err(delta_error(format!(
            "state delta exceeds {MAX_DELTA_BYTES} bytes"
        )));
    }
    Ok(delta)
}

pub(super) fn entry_count(delta: &Value) -> usize {
    ["added", "removed", "changed"]
        .iter()
        .map(|name| delta[*name].as_array().map(Vec::len).unwrap_or(0))
        .sum()
}

fn delta_error(message: impl AsRef<str>) -> AppError {
    AppError::invalid_input_with_suggestion(
        format!("VIEW_DELTA_LIMIT: {}", message.as_ref()),
        "Perform a fresh desktop.observe view request in the same compatible scope; views are short-lived observation evidence and never mutation authorization.",
    )
}
