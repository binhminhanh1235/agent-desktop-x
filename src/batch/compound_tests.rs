use std::{
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use agent_desktop_core::{
    AdapterError, ClipboardContent, ClipboardFormat, CommandContext, InteractionLease,
    PermissionReport,
};
use serde_json::json;

use super::execute;
use crate::cli_args::batch::BatchArgs;

struct CompoundAdapter {
    clears: AtomicUsize,
    clipboard: Mutex<String>,
}

impl CompoundAdapter {
    fn new(text: &str) -> Self {
        Self {
            clears: AtomicUsize::new(0),
            clipboard: Mutex::new(text.into()),
        }
    }
}

impl agent_desktop_core::ObservationOps for CompoundAdapter {}
impl agent_desktop_core::ActionOps for CompoundAdapter {}

impl agent_desktop_core::InputOps for CompoundAdapter {
    fn clear_clipboard(&self, _lease: &InteractionLease) -> Result<(), AdapterError> {
        self.clears.fetch_add(1, Ordering::SeqCst);
        self.clipboard.lock().expect("clipboard").clear();
        Ok(())
    }

    fn get_clipboard_content(
        &self,
        format: ClipboardFormat,
        _deadline: agent_desktop_core::Deadline,
    ) -> Result<Option<ClipboardContent>, AdapterError> {
        assert_eq!(format, ClipboardFormat::Text);
        Ok(Some(ClipboardContent::Text(
            self.clipboard.lock().expect("clipboard").clone(),
        )))
    }
}

impl agent_desktop_core::SystemOps for CompoundAdapter {
    fn acquire_interaction_lease(
        &self,
        deadline: agent_desktop_core::Deadline,
    ) -> Result<InteractionLease, AdapterError> {
        InteractionLease::guarded(deadline, ())
    }
}

fn args(commands: serde_json::Value) -> BatchArgs {
    BatchArgs {
        commands_json: commands.to_string(),
        stop_on_error: true,
        semantic: true,
        timeout_ms: 60_000,
    }
}

#[test]
fn condition_mismatch_prevents_mutation() {
    let adapter = CompoundAdapter::new("ready");
    let output = execute(
        args(json!([{
            "command": "clipboard-clear",
            "args": {},
            "condition": {
                "command": "clipboard-get",
                "args": {},
                "json_pointer": "/text",
                "equals": "go"
            }
        }])),
        &adapter,
        &PermissionReport::default(),
        &CommandContext::default(),
    )
    .expect("condition failure is structured");

    assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);
    assert_eq!(output["results"][0]["execution"], "not_started");
    assert_eq!(output["results"][0]["not_started_reason"], "condition");
    assert_eq!(
        output["results"][0]["error"]["disposition"]["delivery"],
        "not_delivered"
    );
    assert_eq!(output["plan_trace"][0]["phase"], "condition");
}

#[test]
fn verification_passes_without_replaying_mutation() {
    let adapter = CompoundAdapter::new("payload");
    let output = execute(
        args(json!([{
            "command": "clipboard-clear",
            "args": {},
            "verify": {
                "command": "clipboard-get",
                "args": {},
                "json_pointer": "/text",
                "equals": ""
            }
        }])),
        &adapter,
        &PermissionReport::default(),
        &CommandContext::default(),
    )
    .expect("verification passes");

    assert_eq!(adapter.clears.load(Ordering::SeqCst), 1);
    assert_eq!(
        output["results"][0]["data"]["compound_verification"]["verified"],
        true
    );
    assert_eq!(output["compound"]["mutation_replay"], false);
}

#[test]
fn verification_failure_is_distinct_and_never_replays_action() {
    let adapter = CompoundAdapter::new("payload");
    let output = execute(
        args(json!([{
            "command": "clipboard-clear",
            "args": {},
            "verify": {
                "command": "clipboard-get",
                "args": {},
                "json_pointer": "/text",
                "equals": "still-present"
            }
        }])),
        &adapter,
        &PermissionReport::default(),
        &CommandContext::default(),
    )
    .expect("verification failure is structured");

    assert_eq!(adapter.clears.load(Ordering::SeqCst), 1);
    assert_eq!(output["results"][0]["ok"], false);
    assert_eq!(
        output["results"][0]["error"]["details"]["kind"],
        "compound_verification_failed"
    );
    assert_eq!(
        output["results"][0]["error"]["disposition"]["delivery"],
        "delivered_unverified"
    );
    assert_eq!(
        output["results"][0]["error"]["disposition"]["retry"],
        "unsafe"
    );
    assert_eq!(output["stopped"]["phase"], "verification");
}

#[test]
fn semantic_mode_rejects_coordinate_only_steps_before_side_effects() {
    let adapter = CompoundAdapter::new("payload");
    let error = execute(
        args(json!([
            {"command": "clipboard-clear", "args": {}},
            {"command": "mouse-click", "args": {"xy": "10,10"}}
        ])),
        &adapter,
        &PermissionReport::default(),
        &CommandContext::default(),
    )
    .expect_err("coordinate-only command must fail preflight");

    assert_eq!(error.code(), "INVALID_ARGS");
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);
}

#[test]
fn legacy_batch_without_compound_metadata_keeps_old_surface() {
    let adapter = CompoundAdapter::new("payload");
    let output = execute(
        BatchArgs {
            commands_json: json!([{"command": "clipboard-clear", "args": {}}]).to_string(),
            stop_on_error: false,
            semantic: false,
            timeout_ms: 60_000,
        },
        &adapter,
        &PermissionReport::default(),
        &CommandContext::default(),
    )
    .expect("legacy batch succeeds");

    assert!(output.get("compound").is_none());
    assert!(output.get("plan_trace").is_none());
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 1);
}

#[test]
fn invalid_nested_assertion_rejects_plan_before_any_side_effect() {
    let adapter = CompoundAdapter::new("payload");
    let error = execute(
        args(json!([
            {"command": "clipboard-clear", "args": {}},
            {
                "command": "version",
                "args": {},
                "verify": {
                    "command": "clipboard-clear",
                    "args": {},
                    "json_pointer": "/cleared",
                    "equals": true
                }
            }
        ])),
        &adapter,
        &PermissionReport::default(),
        &CommandContext::default(),
    )
    .expect_err("mutating verification must fail plan preparation");

    assert_eq!(error.code(), "INVALID_ARGS");
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);
}

#[test]
fn per_step_timeout_is_capped_and_reported_without_exceeding_plan_budget() {
    let adapter = CompoundAdapter::new("payload");
    let started = Instant::now();
    let output = execute(
        args(json!([{
            "command": "wait",
            "args": {"ms": 5000},
            "timeout_ms": 25
        }])),
        &adapter,
        &PermissionReport::default(),
        &CommandContext::default(),
    )
    .expect("step timeout remains a structured compound result");

    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(output["results"][0]["error"]["code"], "TIMEOUT");
    assert_eq!(output["plan_trace"][0]["outcome"], "failed");
}

#[test]
fn compact_plan_trace_never_copies_command_payload_values() {
    let secret = "payload-value-that-must-not-enter-plan-trace";
    let adapter = CompoundAdapter::new(secret);
    let output = execute(
        args(json!([{
            "command": "clipboard-get",
            "args": {}
        }])),
        &adapter,
        &PermissionReport::default(),
        &CommandContext::default(),
    )
    .expect("read-only compound step succeeds");

    assert_eq!(output["results"][0]["data"]["text"], secret);
    let trace = serde_json::to_string(&output["plan_trace"]).expect("trace JSON");
    assert!(!trace.contains(secret));
    assert!(trace.contains("clipboard-get"));
}
