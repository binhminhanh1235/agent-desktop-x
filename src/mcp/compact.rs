use agent_desktop_core::{
    AppError, PlatformAdapter,
    commands::batch::BatchCommand,
    context::CommandContext,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::{cli::Commands, cli_args::batch::BatchArgs};

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
        "desktop.observe" => observe_schema(),
        "desktop.execute" => execute_schema(false),
        "desktop.run" => execute_schema(true),
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

fn run(
    arguments: Value,
    adapter: &dyn PlatformAdapter,
    headed: bool,
) -> Result<Value, AppError> {
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

fn observe_schema() -> Value {
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

fn execute_schema(include_workflow: bool) -> Value {
    let assertion = json!({
        "type": "object",
        "required": ["command", "json_pointer", "equals"],
        "properties": {
            "command": { "type": "string" },
            "args": { "type": "object", "additionalProperties": true },
            "json_pointer": { "type": "string", "maxLength": 256 },
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
            "maxItems": 64,
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use agent_desktop_core::{
        AdapterError, ClipboardContent, ClipboardFormat, InteractionLease, PermissionReport,
        PermissionState,
    };

    use super::*;

    struct CompactAdapter {
        clears: AtomicUsize,
    }

    impl agent_desktop_core::ObservationOps for CompactAdapter {}
    impl agent_desktop_core::ActionOps for CompactAdapter {}

    impl agent_desktop_core::InputOps for CompactAdapter {
        fn clear_clipboard(&self, _lease: &InteractionLease) -> Result<(), AdapterError> {
            self.clears.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn get_clipboard_content(
            &self,
            format: ClipboardFormat,
            _deadline: agent_desktop_core::Deadline,
        ) -> Result<Option<ClipboardContent>, AdapterError> {
            assert_eq!(format, ClipboardFormat::Text);
            Ok(Some(ClipboardContent::Text(String::new())))
        }
    }

    impl agent_desktop_core::SystemOps for CompactAdapter {
        fn permission_report(
            &self,
            _deadline: agent_desktop_core::Deadline,
        ) -> Result<PermissionReport, AdapterError> {
            Ok(PermissionReport {
                accessibility: PermissionState::Granted,
                screen_recording: PermissionState::Granted,
                automation: PermissionState::NotRequired,
            })
        }

        fn acquire_interaction_lease(
            &self,
            deadline: agent_desktop_core::Deadline,
        ) -> Result<InteractionLease, AdapterError> {
            InteractionLease::guarded(deadline, ())
        }
    }

    #[test]
    fn schemas_are_bounded_and_observe_excludes_full_tree() {
        let schema = input_schema("desktop.observe");
        let commands = schema["properties"]["command"]["enum"]
            .as_array()
            .expect("observe command enum");
        assert!(!commands.iter().any(|value| value == "snapshot"));
        assert!(!commands.iter().any(|value| value == "screenshot"));
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(input_schema("desktop.execute")["properties"]["steps"]["maxItems"], 64);
    }

    #[test]
    fn observe_rejects_mutation_surface() {
        let error = invoke(
            "desktop.observe",
            json!({ "command": "clipboard-clear", "args": {} }),
            &CompactAdapter {
                clears: AtomicUsize::new(0),
            },
            false,
        )
        .expect_err("observe mutation must be rejected");
        assert_eq!(error.code(), "INVALID_ARGS");
    }

    #[test]
    fn execute_reuses_semantic_preflight_before_any_side_effect() {
        let adapter = CompactAdapter {
            clears: AtomicUsize::new(0),
        };
        let error = invoke(
            "desktop.execute",
            json!({
                "steps": [
                    { "command": "clipboard-clear", "args": {} },
                    { "command": "mouse-click", "args": { "xy": "10,10" } }
                ]
            }),
            &adapter,
            false,
        )
        .expect_err("coordinate-only mutation must fail semantic preflight");
        assert_eq!(error.code(), "INVALID_ARGS");
        assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn run_preserves_compound_verification_and_no_replay_contract() {
        let adapter = CompactAdapter {
            clears: AtomicUsize::new(0),
        };
        let output = invoke(
            "desktop.run",
            json!({
                "workflow": "clear-clipboard",
                "steps": [{
                    "command": "clipboard-clear",
                    "args": {},
                    "verify": {
                        "command": "clipboard-get",
                        "args": {},
                        "json_pointer": "/text",
                        "equals": ""
                    }
                }]
            }),
            &adapter,
            false,
        )
        .expect("workflow should pass");

        assert_eq!(adapter.clears.load(Ordering::SeqCst), 1);
        assert_eq!(output["operation"], "run");
        assert_eq!(output["provenance"]["workflow"], "clear-clipboard");
        assert_eq!(output["verification"]["state"], "passed");
        assert_eq!(output["result"]["compound"]["mutation_replay"], false);
    }
}
