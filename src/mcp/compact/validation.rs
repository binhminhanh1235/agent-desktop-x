use agent_desktop_core::AppError;
use serde_json::Value;

use super::{MAX_ASSERTION_JSON_POINTER_CHARS, MAX_STEPS};

pub(super) fn execution_bounds(
    operation: &str,
    steps: &[Value],
    timeout_ms: u64,
) -> Result<(), AppError> {
    if steps.is_empty() {
        return Err(AppError::invalid_input_with_suggestion(
            format!("desktop.{operation} requires at least one step"),
            "Provide one or more semantic command steps.",
        ));
    }
    if steps.len() > MAX_STEPS {
        return Err(AppError::invalid_input_with_suggestion(
            format!(
                "desktop.{operation} accepts at most {MAX_STEPS} steps; received {}",
                steps.len()
            ),
            "Split the work into smaller bounded compound calls.",
        ));
    }
    require_positive_timeout(operation, "timeout_ms", timeout_ms)?;

    for (index, step) in steps.iter().enumerate() {
        if step.get("timeout_ms").and_then(Value::as_u64) == Some(0) {
            return Err(AppError::invalid_input_with_suggestion(
                format!("desktop.{operation} step {index} timeout_ms must be at least 1"),
                "Use a positive per-step timeout in milliseconds.",
            ));
        }
        for assertion_name in ["condition", "verify"] {
            let Some(pointer) = step
                .get(assertion_name)
                .and_then(|assertion| assertion.get("json_pointer"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            if pointer.chars().count() > MAX_ASSERTION_JSON_POINTER_CHARS {
                return Err(AppError::invalid_input_with_suggestion(
                    format!(
                        "desktop.{operation} step {index} {assertion_name} json_pointer exceeds the {MAX_ASSERTION_JSON_POINTER_CHARS}-character limit"
                    ),
                    "Use a shorter JSON Pointer targeting only the state needed for the assertion.",
                ));
            }
        }
    }

    Ok(())
}

fn require_positive_timeout(operation: &str, field: &str, timeout_ms: u64) -> Result<(), AppError> {
    if timeout_ms == 0 {
        return Err(AppError::invalid_input_with_suggestion(
            format!("desktop.{operation} {field} must be at least 1"),
            "Use a positive timeout in milliseconds.",
        ));
    }
    Ok(())
}
