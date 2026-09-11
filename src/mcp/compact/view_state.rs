use super::view_error;
use agent_desktop_core::AppError;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const MAX_VIEW_ENTRIES: usize = 256;
const MAX_VIEW_STATE_BYTES: usize = 128 * 1024;

pub(super) fn canonical_windows(result: &Value) -> Result<BTreeMap<String, Value>, AppError> {
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

pub(super) fn encoded_len(value: &Value) -> Result<usize, AppError> {
    Ok(serde_json::to_vec(value)?.len())
}
