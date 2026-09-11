use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

use agent_desktop_core::{
    ActionOps, AdapterError, ClipboardContent, ClipboardFormat, Deadline, InputOps,
    InteractionLease, ObservationOps, PermissionReport, PermissionState, ProcessId, Rect,
    SystemOps, WindowFilter, WindowInfo, WindowState,
};
use serde_json::{Value, json};

use super::*;

struct FreshnessAdapter {
    windows: Mutex<Vec<WindowInfo>>,
    clears: AtomicUsize,
    list_calls: AtomicUsize,
}

impl FreshnessAdapter {
    fn new(windows: Vec<WindowInfo>) -> Self {
        Self {
            windows: Mutex::new(windows),
            clears: AtomicUsize::new(0),
            list_calls: AtomicUsize::new(0),
        }
    }

    fn set_windows(&self, windows: Vec<WindowInfo>) {
        *self.windows.lock().expect("windows lock") = windows;
    }
}

impl ObservationOps for FreshnessAdapter {
    fn list_windows(
        &self,
        filter: &WindowFilter,
        _deadline: Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        self.list_calls.fetch_add(1, Ordering::SeqCst);
        let windows = self.windows.lock().expect("windows lock");
        Ok(windows
            .iter()
            .filter(|window| {
                filter
                    .app
                    .as_deref()
                    .is_none_or(|app| window.app.eq_ignore_ascii_case(app))
            })
            .cloned()
            .collect())
    }
}

impl ActionOps for FreshnessAdapter {}

impl InputOps for FreshnessAdapter {
    fn clear_clipboard(&self, _lease: &InteractionLease) -> Result<(), AdapterError> {
        self.clears.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn get_clipboard_content(
        &self,
        _format: ClipboardFormat,
        _deadline: Deadline,
    ) -> Result<Option<ClipboardContent>, AdapterError> {
        Ok(Some(ClipboardContent::Text(String::new())))
    }
}

impl SystemOps for FreshnessAdapter {
    fn permission_report(&self, _deadline: Deadline) -> Result<PermissionReport, AdapterError> {
        Ok(PermissionReport {
            accessibility: PermissionState::Granted,
            screen_recording: PermissionState::Granted,
            automation: PermissionState::NotRequired,
        })
    }

    fn acquire_interaction_lease(
        &self,
        deadline: Deadline,
    ) -> Result<InteractionLease, AdapterError> {
        InteractionLease::guarded(deadline, ())
    }
}

fn window(x: f64) -> WindowInfo {
    WindowInfo {
        id: "native-runtime-window".to_string(),
        title: "Document".to_string(),
        app: "Editor".to_string(),
        pid: ProcessId::from(42_u32),
        process_instance: Some("process-generation-1".to_string()),
        bounds: Some(Rect {
            x,
            y: 20.0,
            width: 800.0,
            height: 600.0,
        }),
        state: WindowState {
            is_focused: true,
            accessible: true,
            minimized: Some(false),
            visible: Some(true),
        },
    }
}

fn create_view(adapter: &FreshnessAdapter) -> Value {
    invoke(
        "desktop.observe",
        json!({
            "command": "list-windows",
            "args": {"app": "Editor"},
            "view": {}
        }),
        adapter,
        false,
    )
    .expect("view observation")
}

fn view_id(output: &Value) -> &str {
    output["view"]["view_id"].as_str().expect("view id")
}

#[test]
fn execute_and_run_schemas_expose_bounded_expected_view_id() {
    for tool in ["desktop.execute", "desktop.run"] {
        let schema = input_schema(tool);
        let expected = &schema["properties"]["expected_view_id"];
        assert_eq!(expected["minLength"], 19);
        assert_eq!(expected["maxLength"], 19);
        assert_eq!(expected["pattern"], "^v1-[0-9a-fA-F]{16}$");
    }
}

#[test]
fn fresh_expected_view_is_reobserved_before_mutation() {
    let adapter = FreshnessAdapter::new(vec![window(10.0)]);
    let observed = create_view(&adapter);
    let output = invoke(
        "desktop.execute",
        json!({
            "expected_view_id": view_id(&observed),
            "steps": [{"command": "clipboard-clear", "args": {}}]
        }),
        &adapter,
        false,
    )
    .expect("fresh expected view should allow normal semantic execution");

    assert_eq!(adapter.list_calls.load(Ordering::SeqCst), 2);
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 1);
    assert_eq!(output["view_freshness"]["state"], "passed");
    assert_eq!(
        output["view_freshness"]["expected_view_id"],
        view_id(&observed)
    );
}

#[test]
fn stale_expected_view_refuses_before_side_effects() {
    let adapter = FreshnessAdapter::new(vec![window(10.0)]);
    let observed = create_view(&adapter);
    adapter.set_windows(vec![window(99.0)]);

    let error = invoke(
        "desktop.execute",
        json!({
            "expected_view_id": view_id(&observed),
            "steps": [{"command": "clipboard-clear", "args": {}}]
        }),
        &adapter,
        false,
    )
    .expect_err("stale expected view must refuse before mutation");

    assert!(error.to_string().contains("VIEW_STALE"));
    assert_eq!(adapter.list_calls.load(Ordering::SeqCst), 2);
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);
}

#[test]
fn expected_view_never_bypasses_existing_semantic_preflight() {
    let adapter = FreshnessAdapter::new(vec![window(10.0)]);
    let observed = create_view(&adapter);

    let error = invoke(
        "desktop.execute",
        json!({
            "expected_view_id": view_id(&observed),
            "steps": [
                {"command": "clipboard-clear", "args": {}},
                {"command": "mouse-click", "args": {"xy": "10,10"}}
            ]
        }),
        &adapter,
        false,
    )
    .expect_err("fresh observation evidence must not authorize coordinate mutation");

    assert_eq!(error.code(), "INVALID_ARGS");
    assert_eq!(adapter.list_calls.load(Ordering::SeqCst), 2);
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);
}

#[test]
fn unknown_expected_view_refuses_before_side_effects() {
    let adapter = FreshnessAdapter::new(vec![window(10.0)]);
    let error = invoke(
        "desktop.run",
        json!({
            "workflow": "clear-clipboard",
            "expected_view_id": "v1-00000000ffffffff",
            "steps": [{"command": "clipboard-clear", "args": {}}]
        }),
        &adapter,
        false,
    )
    .expect_err("unknown expected view must refuse");

    assert!(error.to_string().contains("VIEW_UNKNOWN"));
    assert_eq!(adapter.list_calls.load(Ordering::SeqCst), 0);
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);
}
