use agent_desktop_core::{
    AdapterError, AppError, Deadline, DeliverySemantics, ErrorCode, PermissionReport,
    commands::batch::{BatchAssertion, BatchCommand},
    context::CommandContext,
};
use serde_json::{Value, json};

use crate::cli::Commands;

pub(super) const MAX_BATCH_JSON_BYTES: usize = 1024 * 1024;
pub(super) const MAX_BATCH_ENTRIES: usize = 64;
const MAX_ASSERTION_POINTER_BYTES: usize = 256;

pub(super) struct PreparedPlan {
    pub(super) commands: Vec<PreparedCommand>,
    pub(super) compound: bool,
}

pub(super) struct PreparedCommand {
    pub name: String,
    pub command: Commands,
    pub context: CommandContext,
    pub timeout_ms: Option<u64>,
    pub condition: Option<PreparedAssertion>,
    pub verification: Option<PreparedAssertion>,
    pub mutating: bool,
}

pub(super) struct PreparedAssertion {
    pub name: String,
    pub command: Commands,
    pub json_pointer: String,
    pub equals: Value,
}

impl PreparedCommand {
    pub(super) fn has_compound_metadata(&self) -> bool {
        self.timeout_ms.is_some() || self.condition.is_some() || self.verification.is_some()
    }
}

pub(super) fn prepare(
    input: &str,
    permission_report: &PermissionReport,
    context: &CommandContext,
    semantic_only: bool,
) -> Result<PreparedPlan, AppError> {
    if input.len() > MAX_BATCH_JSON_BYTES {
        return Err(limit_error(
            "Batch JSON exceeds the input limit",
            json!({ "actual_bytes": input.len(), "max_bytes": MAX_BATCH_JSON_BYTES }),
        ));
    }
    let items = agent_desktop_core::commands::batch::parse_commands(input)?;
    if items.len() > MAX_BATCH_ENTRIES {
        return Err(limit_error(
            "Batch contains too many entries",
            json!({ "actual_entries": items.len(), "max_entries": MAX_BATCH_ENTRIES }),
        ));
    }
    let parsed = items
        .into_iter()
        .enumerate()
        .map(|(index, item)| parse_one(index, item, permission_report, semantic_only))
        .collect::<Result<Vec<_>, _>>()?;
    let commands = parsed
        .into_iter()
        .map(|parsed| prepare_context(parsed, context))
        .collect::<Result<Vec<_>, _>>()?;
    let compound = semantic_only || commands.iter().any(PreparedCommand::has_compound_metadata);
    Ok(PreparedPlan { commands, compound })
}

struct ParsedCommand {
    index: usize,
    name: String,
    command: Commands,
    session: Option<String>,
    timeout_ms: Option<u64>,
    condition: Option<PreparedAssertion>,
    verification: Option<PreparedAssertion>,
    mutating: bool,
}

fn parse_one(
    index: usize,
    mut item: BatchCommand,
    permission_report: &PermissionReport,
    semantic_only: bool,
) -> Result<ParsedCommand, AppError> {
    let name = item.command.clone();
    let session = item.session.clone();
    if let Some(session) = session.as_deref() {
        agent_desktop_core::context::validate_session_id(session)
            .map_err(|error| located_error(index, &name, error))?;
    }
    if let Some(timeout_ms) = item.timeout_ms {
        Deadline::detached_after(timeout_ms)
            .map_err(AppError::Adapter)
            .map_err(|error| located_error(index, &name, error))?;
    }
    let condition = item
        .condition
        .take()
        .map(|assertion| prepare_assertion(index, &name, "condition", assertion, permission_report))
        .transpose()?;
    let verification = item
        .verify
        .take()
        .map(|assertion| {
            prepare_assertion(index, &name, "verification", assertion, permission_report)
        })
        .transpose()?;
    let command = super::parse_command(item).map_err(|error| located_error(index, &name, error))?;
    crate::command_policy::preflight(&command, permission_report)
        .map_err(|error| located_error(index, &name, error))?;
    if semantic_only && coordinate_only(&command) {
        return Err(located_error(
            index,
            &name,
            AppError::invalid_input_with_suggestion(
                "Semantic compound mode rejects coordinate-only mutating steps",
                "Use element refs or structured app/window targets, or omit --semantic for legacy batch behavior.",
            ),
        ));
    }
    Ok(ParsedCommand {
        index,
        name,
        mutating: command.is_mutating(),
        command,
        session,
        timeout_ms: item.timeout_ms,
        condition,
        verification,
    })
}

fn prepare_assertion(
    index: usize,
    parent: &str,
    phase: &'static str,
    assertion: BatchAssertion,
    permission_report: &PermissionReport,
) -> Result<PreparedAssertion, AppError> {
    validate_pointer(&assertion.json_pointer).map_err(|error| located_error(index, parent, error))?;
    let name = assertion.command.clone();
    let command = super::parse_command(BatchCommand {
        command: assertion.command,
        session: None,
        args: assertion.args,
        timeout_ms: None,
        condition: None,
        verify: None,
    })
    .map_err(|error| located_error(index, parent, error))?;
    crate::command_policy::preflight(&command, permission_report)
        .map_err(|error| located_error(index, parent, error))?;
    if command.is_mutating() {
        return Err(located_error(
            index,
            parent,
            AppError::invalid_input(format!(
                "Compound {phase} command '{name}' must be read-only"
            )),
        ));
    }
    Ok(PreparedAssertion {
        name,
        command,
        json_pointer: assertion.json_pointer,
        equals: assertion.equals,
    })
}

fn prepare_context(
    parsed: ParsedCommand,
    context: &CommandContext,
) -> Result<PreparedCommand, AppError> {
    let item_context = context
        .for_batch_item(parsed.session)
        .map_err(|error| located_error(parsed.index, &parsed.name, error))?;
    crate::command_policy::preflight_context(&parsed.command, &item_context)
        .map_err(|error| located_error(parsed.index, &parsed.name, error))?;
    for assertion in [&parsed.condition, &parsed.verification].into_iter().flatten() {
        crate::command_policy::preflight_context(&assertion.command, &item_context)
            .map_err(|error| located_error(parsed.index, &parsed.name, error))?;
    }
    Ok(PreparedCommand {
        name: parsed.name,
        command: parsed.command,
        context: item_context,
        timeout_ms: parsed.timeout_ms,
        condition: parsed.condition,
        verification: parsed.verification,
        mutating: parsed.mutating,
    })
}

fn validate_pointer(pointer: &str) -> Result<(), AppError> {
    if pointer.len() > MAX_ASSERTION_POINTER_BYTES
        || (!pointer.is_empty() && !pointer.starts_with('/'))
    {
        return Err(AppError::invalid_input_with_suggestion(
            "Compound assertion json_pointer must be empty or a JSON Pointer beginning with '/' and no longer than 256 bytes",
            "Use paths such as /result, /text, or /matches/0/name.",
        ));
    }
    Ok(())
}

fn coordinate_only(command: &Commands) -> bool {
    match command {
        Commands::MouseMove(_)
        | Commands::MouseClick(_)
        | Commands::MouseDown(_)
        | Commands::MouseUp(_)
        | Commands::MouseWheel(_) => true,
        Commands::Hover(args) => args.ref_id.is_none(),
        Commands::Drag(args) => args.target.from.is_none() || args.target.to.is_none(),
        _ => false,
    }
}

fn located_error(index: usize, command: &str, error: AppError) -> AppError {
    match error {
        AppError::Adapter(mut source) => {
            let cause_details = source.details.take();
            let mut details = json!({ "batch_index": index, "batch_command": command });
            if let Some(cause_details) = cause_details {
                details["cause_details"] = cause_details;
            }
            source.message = format!(
                "Batch entry {index} ('{command}') failed validation: {}",
                source.message
            );
            source.details = Some(details);
            source.disposition = DeliverySemantics::not_delivered();
            source.into()
        }
        other => AdapterError::new(ErrorCode::Internal, other.to_string())
            .with_details(json!({ "batch_index": index, "batch_command": command }))
            .with_disposition(DeliverySemantics::not_delivered())
            .into(),
    }
}

fn limit_error(message: &str, details: Value) -> AppError {
    AdapterError::new(ErrorCode::InvalidArgs, message)
        .with_suggestion("Split the batch or narrow commands that return large payloads")
        .with_details(details)
        .with_disposition(DeliverySemantics::not_delivered())
        .into()
}
