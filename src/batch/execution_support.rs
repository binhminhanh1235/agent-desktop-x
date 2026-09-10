use agent_desktop_core::{
    AdapterError, AppError, Deadline, DeliverySemantics, ErrorCode, PermissionReport,
    PlatformAdapter, context::CommandContext,
};
use serde_json::{Value, json};

use crate::cli_args::batch::BatchArgs;

use super::{
    assertion,
    bounded_json::serialized_size,
    plan_trace::PlanTraceEntry,
    preparation::{MAX_BATCH_ENTRIES, MAX_BATCH_JSON_BYTES, PreparedCommand},
    result_entry::MAX_BATCH_OUTPUT_BYTES,
};

pub(super) fn apply_verification(
    index: usize,
    command: &mut PreparedCommand,
    result: Result<Value, AppError>,
    adapter: &dyn PlatformAdapter,
    permission_report: &PermissionReport,
    context: &CommandContext,
) -> Result<Value, AppError> {
    let value = result?;
    let Some(verification) = command.verification.take() else {
        return Ok(value);
    };
    let disposition = assertion::successful_disposition(&value, command.mutating);
    let assertion_name = verification.name.clone();
    match assertion::run(verification, adapter, permission_report, context) {
        Ok(result) if result.matched => Ok(assertion::attach_verification(value, &result)),
        Ok(result) => Err(assertion::verification_error(
            index,
            &command.name,
            &result,
            disposition,
        )),
        Err(error) => Err(assertion::phase_runtime_error(
            index,
            &command.name,
            "verification",
            &assertion_name,
            error,
            disposition,
        )),
    }
}

pub(super) fn result_disposition(
    result: &Result<Value, AppError>,
    mutating: bool,
) -> Option<DeliverySemantics> {
    match result {
        Ok(value) => Some(assertion::successful_disposition(value, mutating)),
        Err(AppError::Adapter(error)) => Some(error.disposition),
        Err(_) => None,
    }
}

pub(super) fn verification_phase(result: &Result<Value, AppError>) -> Option<&'static str> {
    match result {
        Err(AppError::Adapter(error))
            if error
                .details
                .as_ref()
                .and_then(|details| details.get("kind"))
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.starts_with("compound_verification_")) =>
        {
            Some("verification")
        }
        _ => None,
    }
}

pub(super) fn step_deadline(
    parent: Deadline,
    timeout_ms: Option<u64>,
) -> Result<Deadline, AppError> {
    match timeout_ms {
        Some(timeout_ms) => Ok(Deadline::detached_after(timeout_ms)?.capped(parent.remaining())),
        None => Ok(parent),
    }
}

pub(super) struct BuildBodyArgs<'a> {
    pub(super) args: &'a BatchArgs,
    pub(super) total: usize,
    pub(super) completed: usize,
    pub(super) results: Vec<Value>,
    pub(super) plan_trace: Vec<PlanTraceEntry>,
    pub(super) compound: bool,
    pub(super) stopped: Option<Value>,
    pub(super) deadline: Deadline,
}

pub(super) fn build_body(input: BuildBodyArgs<'_>) -> Result<Value, AppError> {
    let mut body = json!({
        "results": input.results,
        "semantics": {
            "atomic": false,
            "order": "sequential",
            "batch_retries": false,
            "command_retry_contracts": "preserved",
            "successful_action_disposition": "data.disposition",
            "error_disposition": "error.disposition",
        },
        "total_entries": input.total,
        "completed_entries": input.completed,
        "not_started_entries": input.total.saturating_sub(input.completed),
        "timeout_ms": input.args.timeout_ms,
        "elapsed_ms": input.deadline.elapsed().as_millis(),
        "limits": {
            "max_entries": MAX_BATCH_ENTRIES,
            "max_input_bytes": MAX_BATCH_JSON_BYTES,
            "max_output_bytes": MAX_BATCH_OUTPUT_BYTES,
        }
    });
    if input.compound {
        body["compound"] = json!({
            "version": 1,
            "semantic_only": input.args.semantic,
            "stop_on_error": input.args.stop_on_error,
            "per_step_deadlines": true,
            "conditions": true,
            "verification": true,
            "mutation_replay": false,
        });
        body["plan_trace"] = serde_json::to_value(input.plan_trace)?;
    }
    if let Some(stopped) = input.stopped {
        body["stopped"] = stopped;
    }
    if serialized_size(&body) > MAX_BATCH_OUTPUT_BYTES {
        let disposition = if input.completed == 0 {
            DeliverySemantics::not_delivered()
        } else {
            DeliverySemantics::uncertain()
        };
        return Err(AdapterError::new(
            ErrorCode::Internal,
            "Batch response exceeded its output contract after final serialization",
        )
        .with_details(json!({
            "kind": "batch_output_limit",
            "completed_entries": input.completed,
            "max_output_bytes": MAX_BATCH_OUTPUT_BYTES,
        }))
        .with_disposition(disposition)
        .into());
    }
    Ok(body)
}
