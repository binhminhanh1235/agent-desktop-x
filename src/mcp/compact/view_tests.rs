use std::sync::{Mutex, atomic::{AtomicUsize, Ordering}};

use agent_desktop_core::{
    ActionOps, AdapterError, ClipboardContent, ClipboardFormat, Deadline, InputOps, InteractionLease,
    ObservationOps, PermissionReport, PermissionState, ProcessId, Rect, SystemOps, WindowFilter,
    WindowInfo, WindowState,
};
use serde_json::{Value, json};

use super::*;
use crate::mcp::compact::invoke;

struct ViewAdapter {
    windows: Mutex<Vec<WindowInfo>>,
    clears: AtomicUsize,
    list_calls: AtomicUsize,
}

impl ViewAdapter {
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

impl ObservationOps for ViewAdapter {
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

impl ActionOps for ViewAdapter {}

impl InputOps for ViewAdapter {
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

impl SystemOps for ViewAdapter {
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

fn window(title: &str, focused: bool, x: f64) -> WindowInfo {
    WindowInfo {
        id: format!("w-{}", 0xdead_beefu64 + title.len() as u64),
        title: title.to_string(),
        app: "Editor".to_string(),
        pid: ProcessId::from(42_u32),
        process_instance: Some("proc-generation-1".to_string()),
        bounds: Some(Rect {
            x,
            y: 20.0,
            width: 800.0,
            height: 600.0,
        }),
        state: WindowState {
            is_focused: focused,
            accessible: true,
            minimized: Some(false),
            visible: Some(true),
        },
    }
}

fn observe(adapter: &ViewAdapter, previous: Option<&str>, app: Option<&str>) -> Value {
    let mut view = json!({});
    if let Some(previous) = previous {
        view["previous_view_id"] = json!(previous);
    }
    let mut args = json!({});
    if let Some(app) = app {
        args["app"] = json!(app);
    }
    invoke(
        "desktop.observe",
        json!({ "command": "list-windows", "args": args, "view": view }),
        adapter,
        false,
    )
    .expect("view observation")
}

fn view_id(output: &Value) -> &str {
    output["view"]["view_id"].as_str().expect("view id")
}

#[test]
fn first_observation_creates_bounded_view_and_keeps_normal_result() {
    let adapter = ViewAdapter::new(vec![window("A", true, 10.0)]);
    let output = observe(&adapter, None, None);
    assert!(view_id(&output).len() <= 32);
    assert_eq!(output["view"]["generation"], 1);
    assert_eq!(output["result"].as_array().map(Vec::len), Some(1));
    assert!(output.get("delta").is_none());
    assert_eq!(output["view"]["metrics"]["full_entries"], 1);
}

#[test]
fn identical_second_observation_is_empty_delta_and_performs_fresh_read() {
    let adapter = ViewAdapter::new(vec![window("A", true, 10.0)]);
    let first = observe(&adapter, None, None);
    let second = observe(&adapter, Some(view_id(&first)), None);
    assert_eq!(second["delta"], json!({"added":[], "removed":[], "changed":[]}));
    assert!(second.get("result").is_none());
    assert_eq!(adapter.list_calls.load(Ordering::SeqCst), 2);
    assert_eq!(second["view"]["metrics"]["delta_entries"], 0);
}

#[test]
fn addition_removal_and_change_emit_only_the_relevant_entity() {
    let adapter = ViewAdapter::new(vec![window("A", true, 10.0)]);
    let first = observe(&adapter, None, None);

    adapter.set_windows(vec![window("A", true, 10.0), window("B", false, 30.0)]);
    let added = observe(&adapter, Some(view_id(&first)), None);
    assert_eq!(added["delta"]["added"].as_array().map(Vec::len), Some(1));
    assert_eq!(added["delta"]["removed"].as_array().map(Vec::len), Some(0));
    assert_eq!(added["delta"]["changed"].as_array().map(Vec::len), Some(0));

    adapter.set_windows(vec![window("B", false, 30.0)]);
    let removed = observe(&adapter, Some(view_id(&added)), None);
    assert_eq!(removed["delta"]["removed"].as_array().map(Vec::len), Some(1));
    assert_eq!(removed["delta"]["added"].as_array().map(Vec::len), Some(0));

    adapter.set_windows(vec![window("B", true, 99.0)]);
    let changed = observe(&adapter, Some(view_id(&removed)), None);
    assert_eq!(changed["delta"]["changed"].as_array().map(Vec::len), Some(1));
    assert_eq!(changed["delta"]["added"].as_array().map(Vec::len), Some(0));
    assert_eq!(changed["delta"]["removed"].as_array().map(Vec::len), Some(0));
}

#[test]
fn provider_order_does_not_change_delta_order_or_create_false_changes() {
    let adapter = ViewAdapter::new(vec![window("B", false, 30.0), window("A", true, 10.0)]);
    let first = observe(&adapter, None, None);
    adapter.set_windows(vec![window("A", true, 10.0), window("B", false, 30.0)]);
    let second = observe(&adapter, Some(view_id(&first)), None);
    assert_eq!(second["delta"], json!({"added":[], "removed":[], "changed":[]}));

    adapter.set_windows(vec![window("D", false, 40.0), window("C", false, 35.0), window("A", true, 10.0), window("B", false, 30.0)]);
    let third = observe(&adapter, Some(view_id(&second)), None);
    let keys = third["delta"]["added"]
        .as_array()
        .expect("added")
        .iter()
        .map(|entry| entry["key"].as_str().expect("key"))
        .collect::<Vec<_>>();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted);
}

#[test]
fn unknown_and_incompatible_views_refuse_deterministically() {
    let adapter = ViewAdapter::new(vec![window("A", true, 10.0)]);
    let unknown = invoke(
        "desktop.observe",
        json!({
            "command":"list-windows",
            "args":{},
            "view":{"previous_view_id":"v1-00000000ffffffff"}
        }),
        &adapter,
        false,
    )
    .expect_err("unknown view must refuse");
    assert!(unknown.to_string().contains("VIEW_UNKNOWN"));

    let first = observe(&adapter, None, Some("Editor"));
    let mismatch = invoke(
        "desktop.observe",
        json!({
            "command":"list-windows",
            "args":{"app":"Browser"},
            "view":{"previous_view_id":view_id(&first)}
        }),
        &adapter,
        false,
    )
    .expect_err("scope mismatch must refuse");
    assert!(mismatch.to_string().contains("VIEW_SCOPE_MISMATCH"));
}

#[test]
fn legacy_observe_without_view_stays_backward_compatible() {
    let adapter = ViewAdapter::new(vec![window("A", true, 10.0)]);
    let output = invoke(
        "desktop.observe",
        json!({"command":"list-windows","args":{}}),
        &adapter,
        false,
    )
    .expect("legacy observe");
    assert!(output.get("result").is_some());
    assert!(output.get("view").is_none());
    assert!(output.get("delta").is_none());
}

#[test]
fn stale_view_field_cannot_authorize_mutation() {
    let adapter = ViewAdapter::new(vec![]);
    let error = invoke(
        "desktop.execute",
        json!({
            "view":{"previous_view_id":"v1-0000000000000001"},
            "steps":[{"command":"clipboard-clear","args":{}}]
        }),
        &adapter,
        false,
    )
    .expect_err("execute must not accept view as authorization");
    assert_eq!(error.code(), "INVALID_ARGS");
    assert_eq!(adapter.clears.load(Ordering::SeqCst), 0);
}

#[test]
fn canonical_view_never_persists_native_window_id() {
    let native = "w-3735928559";
    let result = json!([{
        "id": native,
        "title":"A",
        "app_name":"Editor",
        "pid":42,
        "process_instance":"proc-generation-1",
        "bounds":{"x":10.0,"y":20.0,"width":800.0,"height":600.0},
        "is_focused":true,
        "accessible":true,
        "minimized":false,
        "visible":true
    }]);
    let state = canonical_windows(&result).expect("canonical state");
    let encoded = serde_json::to_string(&state).expect("encode state");
    assert!(!encoded.contains(native));
    assert!(!encoded.contains("\"id\""));
}

#[test]
fn expired_view_refuses_explicitly() {
    let start = Instant::now();
    let scope = ObservationScope { command: "list-windows".into(), app: None };
    let mut store = ViewStore::new(2, Duration::from_millis(1));
    let inserted = store.insert(scope.clone(), BTreeMap::new(), None, start);
    let error = store
        .previous(&inserted.id.0, &scope, start + Duration::from_millis(2))
        .expect_err("expired view");
    assert!(error.to_string().contains("VIEW_EXPIRED"));
}

#[test]
fn capacity_eviction_is_oldest_first_and_deterministic() {
    let now = Instant::now();
    let scope = ObservationScope { command: "list-windows".into(), app: None };
    let mut store = ViewStore::new(2, Duration::from_secs(10));
    let first = store.insert(scope.clone(), BTreeMap::new(), None, now);
    let second = store.insert(scope.clone(), BTreeMap::new(), None, now);
    let third = store.insert(scope.clone(), BTreeMap::new(), None, now);
    assert!(!store.entries.contains_key(&first.id));
    assert!(store.entries.contains_key(&second.id));
    assert!(store.entries.contains_key(&third.id));
}

#[test]
fn unsafe_or_ambiguous_semantic_identity_fails_closed() {
    let missing_generation = json!([{
        "id":"w-1","title":"A","app_name":"Editor","pid":42
    }]);
    let error = canonical_windows(&missing_generation).expect_err("missing generation");
    assert!(error.to_string().contains("VIEW_IDENTITY_UNSAFE"));

    let duplicate = json!([
        {"id":"w-1","title":"A","app_name":"Editor","pid":42,"process_instance":"g"},
        {"id":"w-2","title":"A","app_name":"Editor","pid":42,"process_instance":"g"}
    ]);
    let error = canonical_windows(&duplicate).expect_err("duplicate identity");
    assert!(error.to_string().contains("VIEW_IDENTITY_AMBIGUOUS"));
}

#[test]
fn one_change_delta_is_smaller_than_full_observation_for_many_windows() {
    let initial = (0..30)
        .map(|index| window(&format!("Window {index:02}"), index == 0, index as f64))
        .collect::<Vec<_>>();
    let adapter = ViewAdapter::new(initial.clone());
    let first = observe(&adapter, None, None);
    let mut changed = initial;
    changed[17].bounds.as_mut().expect("bounds").x += 25.0;
    adapter.set_windows(changed);
    let second = observe(&adapter, Some(view_id(&first)), None);

    assert_eq!(second["view"]["metrics"]["full_entries"], 30);
    assert_eq!(second["view"]["metrics"]["delta_entries"], 1);
    assert_eq!(second["view"]["metrics"]["harness_calls_this_observation"], 1);
    assert!(
        second["view"]["metrics"]["delta_payload_bytes"].as_u64().unwrap()
            < second["view"]["metrics"]["full_result_bytes"].as_u64().unwrap()
    );
    assert!(serde_json::to_vec(&second).unwrap().len() < serde_json::to_vec(&first).unwrap().len());
}
