use serde_json::{Value, json};

use super::{
    DEFAULT_TIMEOUT_MS, MAX_ASSERTION_JSON_POINTER_CHARS, MAX_STEPS, MAX_WORKFLOW_NAME_BYTES,
    OBSERVE_COMMANDS,
};

pub(super) fn observe_schema() -> Value {
    json!({
        "type": "object",
        "required": ["command"],
        "properties": {
            "command": {
                "type": "string",
                "enum": OBSERVE_COMMANDS,
                "description": "Targeted read-only command. Full snapshot and screenshot are intentionally excluded."
            },
            "args": {
                "type": "object",
                "description": "Arguments for the selected read-only command.",
                "additionalProperties": true
            }
        },
        "additionalProperties": false
    })
}

pub(super) fn execute_schema(include_workflow: bool) -> Value {
    let assertion = json!({
        "type": "object",
        "required": ["command", "json_pointer", "equals"],
        "properties": {
            "command": { "type": "string" },
            "args": { "type": "object", "additionalProperties": true },
            "json_pointer": {
                "type": "string",
                "maxLength": MAX_ASSERTION_JSON_POINTER_CHARS
            },
            "equals": {}
        },
        "additionalProperties": false
    });
    let step = json!({
        "type": "object",
        "required": ["command"],
        "properties": {
            "command": { "type": "string" },
            "args": { "type": "object", "additionalProperties": true },
            "session": { "type": "string" },
            "timeout_ms": { "type": "integer", "minimum": 1 },
            "condition": assertion.clone(),
            "verify": assertion
        },
        "additionalProperties": false
    });
    let mut properties = json!({
        "steps": {
            "type": "array",
            "minItems": 1,
            "maxItems": MAX_STEPS,
            "items": step
        },
        "stop_on_error": {
            "type": "boolean",
            "default": true
        },
        "timeout_ms": {
            "type": "integer",
            "minimum": 1,
            "default": DEFAULT_TIMEOUT_MS
        }
    });
    let mut required = vec![json!("steps")];
    if include_workflow {
        properties["workflow"] = json!({
            "type": "string",
            "minLength": 1,
            "maxLength": MAX_WORKFLOW_NAME_BYTES,
            "description": "Stable caller-defined workflow identifier; P0C does not persist workflow definitions."
        });
        required.insert(0, json!("workflow"));
    }
    json!({
        "type": "object",
        "required": required,
        "properties": properties,
        "additionalProperties": false
    })
}
