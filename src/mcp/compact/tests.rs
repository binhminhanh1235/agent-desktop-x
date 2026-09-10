use std::sync::atomic::{AtomicUsize, Ordering};

use agent_desktop_core::{
    AdapterError, ClipboardContent, ClipboardFormat, InteractionLease, PermissionReport,
    PermissionState,
};
use serde_json::json;

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
    assert_eq!(
        input_schema("desktop.execute")["properties"]["steps"]["maxItems"],
        MAX_STEPS
    );
    assert_eq!(
        input_schema("desktop.execute")["properties"]["steps"]["items"]["properties"]
            ["verify"]["properties"]["json_pointer"]["maxLength"],
        MAX_ASSERTION_JSON_POINTER_CHARS
    );
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
fn execute_enforces_runtime_step_and_timeout_bounds_before_side_effects() {
    let adapter = CompactAdapter {
        clears: AtomicUsize::new(0),
    };
    let too_many_steps = (0..=MAX_STEPS)
        .map(|_| json!({ "command": "clipboard-clear", "args": {} }))
        .collect::<Vec<_>>();
    let error = invoke(
        "desktop.execute",
        json!({ "steps": too_many_steps }),
        &adapter,
        false,
    )
    .expect_err("step count must be runtime bounded");
    assert_eq!(error.code(), "INVALID_ARGS");
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);

    let error = invoke(
        "desktop.execute",
        json!({
            "steps": [{ "command": "clipboard-clear", "args": {} }],
            "timeout_ms": 0
        }),
        &adapter,
        false,
    )
    .expect_err("zero plan timeout must be rejected");
    assert_eq!(error.code(), "INVALID_ARGS");
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);

    let error = invoke(
        "desktop.execute",
        json!({
            "steps": [{
                "command": "clipboard-clear",
                "args": {},
                "timeout_ms": 0
            }]
        }),
        &adapter,
        false,
    )
    .expect_err("zero step timeout must be rejected");
    assert_eq!(error.code(), "INVALID_ARGS");
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);
}

#[test]
fn execute_enforces_assertion_pointer_bound_before_side_effects() {
    let adapter = CompactAdapter {
        clears: AtomicUsize::new(0),
    };
    let overlong_pointer = format!("/{}", "x".repeat(MAX_ASSERTION_JSON_POINTER_CHARS));

    for assertion_name in ["condition", "verify"] {
        let mut step = json!({
            "command": "clipboard-clear",
            "args": {}
        });
        step[assertion_name] = json!({
            "command": "clipboard-get",
            "args": {},
            "json_pointer": overlong_pointer,
            "equals": ""
        });
        let error = invoke(
            "desktop.execute",
            json!({ "steps": [step] }),
            &adapter,
            false,
        )
        .expect_err("overlong assertion pointer must be rejected");
        assert_eq!(error.code(), "INVALID_ARGS");
        assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);
    }
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
