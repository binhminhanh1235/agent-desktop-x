use agent_desktop_core::{
    AppError, DeliverySemantics, PermissionReport, PlatformAdapter, context::CommandContext,
};
use serde_json::{Value, json};

use super::preparation::PreparedAssertion;

pub(super) struct AssertionResult {
    pub(super) command: String,
    pub(super) json_pointer: String,
    pub(super) matched: bool,
}

pub(super) fn run(
    mut assertion: PreparedAssertion,
    adapter: &dyn PlatformAdapter,
    permission_report: &PermissionReport,
    context: &CommandContext,
) -> Result<AssertionResult, AppError> {
    let command_name = assertion.name.clone();
    let command = std::mem::replace(&mut assertion.command, crate::cli::Commands::Version);
    let value = crate::dispatch::dispatch(command, adapter, permission_report, context)?;
    let actual = if assertion.json_pointer.is_empty() {
        Some(&value)
    } else {
        value.pointer(&assertion.json_pointer)
    };
    Ok(AssertionResult {
        command: command_name,
        json_pointer: assertion.json_pointer,
        matched: actual == Some(&assertion.equals),
    })
}

pub(super) fn condition_error(
    index: usize,
    command: &str,
    result: &AssertionResult,
) -> AppError {
    agent_desktop_core::AdapterError::new(
        agent_desktop_core::ErrorCode::ActionFailed,
        format!("Compound condition for batch entry {index} ('{command}') was not satisfied"),
    )
    .with_details(json!({
        "kind": "compound_condition_failed",
        "batch_index": index,
        "batch_command": command,
        "assertion_command": result.command,
        "json_pointer": result.json_pointer,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
    .into()
}

pub(super) fn verification_error(
    index: usize,
    command: &str,
    result: &AssertionResult,
    disposition: DeliverySemantics,
) -> AppError {
    agent_desktop_core::AdapterError::new(
        agent_desktop_core::ErrorCode::ActionFailed,
        format!("Compound verification for batch entry {index} ('{command}') failed"),
    )
    .with_details(json!({
        "kind": "compound_verification_failed",
        "batch_index": index,
        "batch_command": command,
        "assertion_command": result.command,
        "json_pointer": result.json_pointer,
        "action_completed": true,
    }))
    .with_disposition(disposition)
    .into()
}

pub(super) fn phase_runtime_error(
    index: usize,
    command: &str,
    phase: &'static str,
    assertion_command: &str,
    source: AppError,
    disposition: DeliverySemantics,
) -> AppError {
    let mut error = match source {
        AppError::Adapter(error) => error,
        other => agent_desktop_core::AdapterError::internal(other.to_string()),
    };
    let cause = error.details.take();
    let mut details = json!({
        "kind": format!("compound_{phase}_error"),
        "batch_index": index,
        "batch_command": command,
        "assertion_command": assertion_command,
    });
    if let Some(cause) = cause {
        details["cause_details"] = cause;
    }
    error.message = format!(
        "Compound {phase} for batch entry {index} ('{command}') failed: {}",
        error.message
    );
    error.details = Some(details);
    error.disposition = disposition;
    error.into()
}

pub(super) fn successful_disposition(value: &Value, mutating: bool) -> DeliverySemantics {
    value
        .get("disposition")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(|| {
            if mutating {
                DeliverySemantics::delivered_unverified()
            } else {
                DeliverySemantics::not_delivered()
            }
        })
}

pub(super) fn attach_verification(mut value: Value, result: &AssertionResult) -> Value {
    let marker = json!({
        "verified": true,
        "command": result.command,
        "json_pointer": result.json_pointer,
    });
    if let Some(object) = value.as_object_mut() {
        object.insert("compound_verification".into(), marker);
        value
    } else {
        json!({
            "result": value,
            "compound_verification": marker,
        })
    }
}
