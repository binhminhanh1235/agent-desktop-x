use std::time::Instant;

use agent_desktop_core::{
    AdapterError, AppError, Deadline, DeliverySemantics, ErrorCode, PermissionReport,
    PlatformAdapter, SignalBaseline, SignalFilter, context::CommandContext,
};
use serde_json::{Value, json};

use crate::{cli::Commands, cli_args::batch::BatchArgs};

use super::{
    assertion,
    bounded_json::serialized_size,
    execution_support::{
        BuildBodyArgs, apply_verification, build_body, result_disposition, step_deadline,
        verification_phase,
    },
    plan_trace::PlanTraceEntry,
    preparation::{PreparedCommand, prepare},
    result_entry::{bounded_entry, not_started_entry},
};

pub(super) fn execute(
    args: BatchArgs,
    adapter: &dyn PlatformAdapter,
    permission_report: &PermissionReport,
    context: &CommandContext,
) -> Result<Value, AppError> {
    let deadline = Deadline::after(args.timeout_ms)
        .map_err(|error| error.with_disposition(DeliverySemantics::not_delivered()))?;
    let batch_context = context.clone().with_inherited_deadline(deadline);
    let plan = prepare(
        &args.commands_json,
        permission_report,
        &batch_context,
        args.semantic,
    )?;
    let compound = plan.compound;
    let mut commands = plan.commands;
    let total = commands.len();
    let mut results = Vec::with_capacity(total);
    let mut plan_trace = Vec::with_capacity(total);
    let mut results_bytes = 0;
    let mut completed = 0;
    let mut pending_baseline: Option<Result<SignalBaseline, AdapterError>> = None;
    let mut stopped = None;

    for index in 0..total {
        let started = Instant::now();
        if deadline.is_expired() {
            let error = batch_timeout(index, &commands[index].name, args.timeout_ms);
            push_small_entry(
                &mut results,
                &mut results_bytes,
                not_started_entry(index, &commands[index].name, "deadline", error),
            );
            if compound {
                plan_trace.push(PlanTraceEntry::not_started(
                    index,
                    &commands[index].name,
                    started.elapsed().as_millis(),
                    "deadline",
                ));
            }
            stopped = Some(json!({ "reason": "deadline", "index": index }));
            break;
        }

        let current_step_deadline = step_deadline(deadline, commands[index].timeout_ms)?;
        let current_baseline = pending_baseline.take();
        pending_baseline = match commands.get(index + 1).and_then(event_filter) {
            Some(filter) => match adapter.capture_signal_baseline(&filter, current_step_deadline) {
                Ok(baseline) => Some(Ok(baseline)),
                Err(error) => {
                    let wait_index = index + 1;
                    let wait_command = &commands[wait_index].name;
                    let error = baseline_error(
                        index,
                        &commands[index].name,
                        wait_index,
                        wait_command,
                        error,
                    );
                    push_small_entry(
                        &mut results,
                        &mut results_bytes,
                        not_started_entry(
                            index,
                            &commands[index].name,
                            "pre_action_baseline_failed",
                            error,
                        ),
                    );
                    if compound {
                        plan_trace.push(PlanTraceEntry::not_started(
                            index,
                            &commands[index].name,
                            started.elapsed().as_millis(),
                            "pre_action_baseline_failed",
                        ));
                    }
                    stopped = Some(json!({
                        "reason": "pre_action_baseline_failed",
                        "blocked_index": index,
                        "blocked_command": commands[index].name,
                        "wait_index": wait_index,
                        "wait_command": wait_command,
                    }));
                    break;
                }
            },
            None => None,
        };

        if let Some(ended_session) = session_ended_for(&commands[index]) {
            let error = batch_session_ended(index, &commands[index].name, &ended_session);
            push_small_entry(
                &mut results,
                &mut results_bytes,
                not_started_entry(index, &commands[index].name, "session_ended", error),
            );
            if compound {
                plan_trace.push(PlanTraceEntry::not_started(
                    index,
                    &commands[index].name,
                    started.elapsed().as_millis(),
                    "session_ended",
                ));
            }
            if args.stop_on_error {
                stopped = Some(json!({ "reason": "stop_on_error", "index": index }));
                break;
            }
            continue;
        }

        let item_context = commands[index]
            .context
            .clone()
            .with_inherited_deadline(current_step_deadline)
            .with_event_baseline(current_baseline);

        if condition_blocks(
            index,
            &mut commands[index],
            adapter,
            permission_report,
            &item_context,
            &mut results,
            &mut results_bytes,
            compound.then_some(&mut plan_trace),
            started,
        )? {
            if args.stop_on_error {
                stopped = Some(json!({
                    "reason": "stop_on_error",
                    "index": index,
                    "phase": "condition",
                }));
                break;
            }
            continue;
        }

        let command = std::mem::replace(&mut commands[index].command, Commands::Version);
        let result = crate::dispatch::dispatch(command, adapter, permission_report, &item_context);
        completed += 1;
        let result = apply_verification(
            index,
            &mut commands[index],
            result,
            adapter,
            permission_report,
            &item_context,
        );
        let failed = result.is_err();
        let disposition = result_disposition(&result, commands[index].mutating);
        let phase = verification_phase(&result);
        let (entry, oversized) = bounded_entry(index, &commands[index].name, result, results_bytes);
        push_small_entry(&mut results, &mut results_bytes, entry);
        if compound {
            plan_trace.push(PlanTraceEntry::completed(
                index,
                &commands[index].name,
                !failed,
                started.elapsed().as_millis(),
                phase,
                disposition,
            ));
        }

        if oversized {
            stopped = Some(json!({ "reason": "output_limit", "index": index }));
            break;
        }
        if failed && args.stop_on_error {
            stopped = Some(json!({
                "reason": "stop_on_error",
                "index": index,
                "phase": phase.unwrap_or("command"),
            }));
            break;
        }
        if deadline.is_expired() && index + 1 < total {
            stopped = Some(json!({ "reason": "deadline", "after_index": index }));
            break;
        }
    }

    build_body(BuildBodyArgs {
        args: &args,
        total,
        completed,
        results,
        plan_trace,
        compound,
        stopped,
        deadline,
    })
}

struct ConditionContext<'a> {
    adapter: &'a dyn PlatformAdapter,
    permission_report: &'a PermissionReport,
    context: &'a CommandContext,
}

fn condition_blocks(
    index: usize,
    command: &mut PreparedCommand,
    runtime: ConditionContext<'_>,
    results: &mut Vec<Value>,
    results_bytes: &mut usize,
    plan_trace: Option<&mut Vec<PlanTraceEntry>>,
    started: Instant,
) -> Result<bool, AppError> {
    let Some(condition) = command.condition.take() else {
        return Ok(false);
    };
    let assertion_name = condition.name.clone();
    let condition_result = assertion::run(
        condition,
        runtime.adapter,
        runtime.permission_report,
        runtime.context,
    );
    let error = match condition_result {
        Ok(result) if result.matched => return Ok(false),
        Ok(result) => assertion::condition_error(index, &command.name, &result),
        Err(error) => assertion::phase_runtime_error(
            index,
            &command.name,
            "condition",
            &assertion_name,
            error,
            DeliverySemantics::not_delivered(),
        ),
    };
    push_small_entry(
        results,
        results_bytes,
        not_started_entry(index, &command.name, "condition", error),
    );
    if let Some(plan_trace) = plan_trace {
        plan_trace.push(PlanTraceEntry::not_started(
            index,
            &command.name,
            started.elapsed().as_millis(),
            "condition",
        ));
    }
    Ok(true)
}

fn event_filter(command: &PreparedCommand) -> Option<SignalFilter> {
    match &command.command {
        Commands::Wait(args) if args.event.event.is_some() => Some(SignalFilter {
            app: args.app.clone(),
            process: None,
        }),
        _ => None,
    }
}

fn baseline_error(
    blocked_index: usize,
    blocked_command: &str,
    wait_index: usize,
    wait_command: &str,
    mut source: AdapterError,
) -> AppError {
    let cause_details = source.details.take();
    source.message = format!(
        "Batch entry {blocked_index} ('{blocked_command}') was not started because the baseline for following wait entry {wait_index} ('{wait_command}') failed: {}",
        source.message
    );
    let mut details = json!({
        "kind": "pre_action_baseline_failed",
        "blocked_index": blocked_index,
        "blocked_command": blocked_command,
        "wait_index": wait_index,
        "wait_command": wait_command,
    });
    if let Some(cause_details) = cause_details {
        details["cause_details"] = cause_details;
    }
    source.details = Some(details);
    source.disposition = DeliverySemantics::not_delivered();
    source.into()
}

fn session_ended_for(command: &PreparedCommand) -> Option<String> {
    let session_id = command.context.session_id()?;
    match agent_desktop_core::session::read_manifest(session_id) {
        Ok(Some(manifest)) if manifest.ended_at.is_some() => Some(session_id.to_owned()),
        _ => None,
    }
}

fn batch_session_ended(index: usize, command: &str, session_id: &str) -> AppError {
    AdapterError::new(
        ErrorCode::InvalidArgs,
        format!(
            "Batch entry {index} ('{command}') was not started because session '{session_id}' has ended"
        ),
    )
    .with_suggestion("Re-run the entry outside the ended session or start a new session")
    .with_details(json!({
        "kind": "batch_session_ended",
        "batch_index": index,
        "batch_command": command,
        "session_id": session_id,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
    .into()
}

fn batch_timeout(index: usize, command: &str, timeout_ms: u64) -> AppError {
    AdapterError::timeout("Batch deadline elapsed before the entry started")
        .with_details(json!({
            "kind": "batch_deadline",
            "batch_index": index,
            "batch_command": command,
            "timeout_ms": timeout_ms,
        }))
        .with_disposition(DeliverySemantics::not_delivered())
        .into()
}

fn push_small_entry(results: &mut Vec<Value>, used: &mut usize, entry: Value) {
    *used = used.saturating_add(serialized_size(&entry).saturating_add(1));
    results.push(entry);
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
