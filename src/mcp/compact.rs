use agent_desktop_core::{
    AppError, PlatformAdapter, commands::batch::BatchCommand, context::CommandContext,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::{cli::Commands, cli_args::batch::BatchArgs};

mod schema;

pub(super) const TOOL_NAMES: [&str; 3] = ["desktop.observe", "desktop.execute", "desktop.run"];

const API_VERSION: u64 = 1;
const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const MAX_WORKFLOW_NAME_BYTES: usize = 128;
const OBSERVE_COMMANDS: &[&str] = &[
    "find",
    "get",
    "is",
    "list-windows",
    "list-apps",
    "list-surfaces",
    "list-notifications",
    "clipboard-get",
    "status",
    "permissions",
    "version",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObserveRequest {
    command: String,
    #[serde(default)]
    args: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecuteRequest {
    steps: Vec<Value>,
    #[serde(default = "default_true")]
    stop_on_error: bool,
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunRequest {
    workflow: String,
    steps: Vec<Value>,
    #[serde(default = "default_true")]
    stop_on_error: bool,
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
}

pub(super) fn is_compact_tool(name: &str) -> bool {
    TOOL_NAMES.contains(&name)
}

pub(super) fn description(name: &str) -> &'static str {
    match name {
        "desktop.observe" => {
            "Read a bounded, targeted slice of desktop state without returning a full accessibility tree by default."
        }
        "desktop.execute" => {
            "Execute one or more semantic desktop steps through the compound safety engine with conditions, deadlines, and verification."
        }
        "desktop.run" => {
            "Run a named reusable semantic workflow through the same compound safety engine; persistence and durable jobs are intentionally out of scope for this API version."
        }
        _ => "Unknown compact agent API tool.",
    }
}

pub(super) fn input_schema(name: &str) -> Value {
    match name {
        "desktop.observe" => schema::observe_schema(),
        "desktop.execute" => schema::execute_schema(false),
        "desktop.run" => schema::execute_schema(true),
        _ => json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
    }
}

pub(super) fn invoke(
    tool_name: &str,
    arguments: Value,
    adapter: &dyn PlatformAdapter,
    headed: bool,
) -> Result<Value, AppError> {
    match tool_name {
        "desktop.observe" => observe(arguments, adapter, headed),
        "desktop.execute" => execute(arguments, adapter, headed),
        "desktop.run" => run(arguments, adapter, headed),
        _ => Err(AppError::invalid_input(format!(
            "Unknown compact agent API tool '{tool_name}'"
        ))),
    }
}

fn observe(
    arguments: Value,
    adapter: &dyn PlatformAdapter,
    headed: bool,
) -> Result<Value, AppError> {
    let request: ObserveRequest = decode("desktop.observe", arguments)?;
    if !OBSERVE_COMMANDS.contains(&request.command.as_str()) {
        return Err(AppError::invalid_input_with_suggestion(
            format!(
                "desktop.observe does not allow command '{}'",
                request.command
            ),
            "Use a targeted read-only observe command, or call the existing granular MCP tool explicitly when a full snapshot/screenshot is required.",
        ));
    }
    let args = object_or_empty(request.args, "desktop.observe args")?;
    let command = crate::batch::parse_command(BatchCommand {
        command: request.command.clone(),
        session: None,
        args,
        timeout_ms: None,
        condition: None,
        verify: None,
    })?;
    if command.is_mutating() {
        return Err(AppError::invalid_input_with_suggestion(
            "desktop.observe only accepts read-only commands",
            "Use desktop.execute for mutations.",
        ));
    }

    let context = CommandContext::default().with_headed(headed);
    let result = crate::execute_with_adapter(command, adapter, &context)?;
    let mut output = json!({
        "api_version": API_VERSION,
        "operation": "observe",
        "provenance": {
            "surface": "mcp",
            "engine": "granular-dispatch",
            "command": request.command,
        },
        "verification": {
            "state": "not_applicable"
        },
        "result": result,
    });
    if let Some(confidence) = output["result"].get("confidence").cloned() {
        output["confidence"] = confidence;
    }
    Ok(output)
}

fn execute(
    arguments: Value,
    adapter: &dyn PlatformAdapter,
    headed: bool,
) -> Result<Value, AppError> {
    let request: ExecuteRequest = decode("desktop.execute", arguments)?;
    execute_steps(
        "execute",
        None,
        request.steps,
        request.stop_on_error,
        request.timeout_ms,
        adapter,
        headed,
    )
}

fn run(arguments: Value, adapter: &dyn PlatformAdapter, headed: bool) -> Result<Value, AppError> {
    let request: RunRequest = decode("desktop.run", arguments)?;
    let workflow = validate_workflow_name(request.workflow)?;
    execute_steps(
        "run",
        Some(workflow),
        request.steps,
        request.stop_on_error,
        request.timeout_ms,
        adapter,
        headed,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_steps(
    operation: &'static str,
    workflow: Option<String>,
    steps: Vec<Value>,
    stop_on_error: bool,
    timeout_ms: u64,
    adapter: &dyn PlatformAdapter,
    headed: bool,
) -> Result<Value, AppError> {
    if steps.is_empty() {
        return Err(AppError::invalid_input_with_suggestion(
            format!("desktop.{operation} requires at least one step"),
            "Provide one or more semantic command steps.",
        ));
    }
    let commands_json = serde_json::to_string(&steps).map_err(|error| {
        AppError::invalid_input(format!(
            "desktop.{operation} could not encode steps: {error}"
        ))
    })?;
    let command = Commands::Batch(BatchArgs {
        commands_json,
        stop_on_error,
        semantic: true,
        timeout_ms,
    });
    let context = CommandContext::default().with_headed(headed);
    let result = crate::execute_with_adapter(command, adapter, &context)?;
    let verification = verification_summary(&steps, &result);
    let state_change = state_change_summary(&result);

    let mut provenance = json!({
        "surface": "mcp",
        "engine": "compound-v1",
        "semantic_only": true,
    });
    if let Some(workflow) = workflow {
        provenance["workflow"] = json!(workflow);
    }

    Ok(json!({
        "api_version": API_VERSION,
        "operation": operation,
        "provenance": provenance,
        "verification": verification,
        "state_change": state_change,
        "result": result,
    }))
}

fn verification_summary(steps: &[Value], result: &Value) -> Value {
    let requested = steps
        .iter()
        .filter(|step| step.get("verify").is_some_and(|verify| !verify.is_null()))
        .count();
    if requested == 0 {
        return json!({
            "state": "not_requested",
            "requested": 0,
            "passed": 0,
            "failed": 0,
        });
    }

    let results = result
        .get("results")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let passed = results
        .iter()
        .filter(|entry| {
            entry
                .pointer("/data/compound_verification/verified")
                .and_then(Value::as_bool)
                == Some(true)
        })
        .count();
    let failed = results
        .iter()
        .filter(|entry| {
            entry
                .pointer("/error/details/kind")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.starts_with("compound_verification_"))
        })
        .count();
    let state = if passed == requested {
        "passed"
    } else if failed > 0 {
        "failed"
    } else {
        "incomplete"
    };
    json!({
        "state": state,
        "requested": requested,
        "passed": passed,
        "failed": failed,
    })
}

fn state_change_summary(result: &Value) -> Value {
    let mut summary = json!({
        "completed_entries": result
            .get("completed_entries")
            .cloned()
            .unwrap_or_else(|| json!(0)),
        "not_started_entries": result
            .get("not_started_entries")
            .cloned()
            .unwrap_or_else(|| json!(0)),
    });
    if let Some(stopped) = result.get("stopped") {
        summary["stopped"] = stopped.clone();
    }
    summary
}

fn validate_workflow_name(workflow: String) -> Result<String, AppError> {
    let trimmed = workflow.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_WORKFLOW_NAME_BYTES {
        return Err(AppError::invalid_input_with_suggestion(
            "desktop.run workflow must be 1..=128 bytes after trimming",
            "Use a short stable workflow identifier such as 'search-and-export'.",
        ));
    }
    Ok(trimmed.to_owned())
}

fn object_or_empty(value: Option<Value>, label: &str) -> Result<Value, AppError> {
    match value.unwrap_or_else(|| json!({})) {
        Value::Null => Ok(json!({})),
        Value::Object(map) => Ok(Value::Object(map)),
        _ => Err(AppError::invalid_input(format!(
            "{label} must be a JSON object"
        ))),
    }
}

fn decode<T: DeserializeOwned>(tool: &str, value: Value) -> Result<T, AppError> {
    serde_json::from_value(value).map_err(|error| {
        AppError::invalid_input_with_suggestion(
            format!("Invalid {tool} arguments: {error}"),
            "Use the deterministic input schema returned by tools/list.",
        )
    })
}

fn default_true() -> bool {
    true
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

#[cfg(test)]
#[path = "compact/tests.rs"]
mod tests;
