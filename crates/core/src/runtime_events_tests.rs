use super::*;
use crate::{AppInfo, ProcessId, SignalCompleteness, WindowInfo, WindowState};

fn app(name: &str, pid: u32, generation: &str) -> AppInfo {
    AppInfo {
        name: name.into(),
        pid: ProcessId::new(pid),
        bundle_id: Some(format!("com.example.{}", name.to_lowercase())),
        process_instance: Some(generation.into()),
        presentation: None,
    }
}

fn window(app: &str, title: &str, pid: u32, generation: &str, id: &str) -> WindowInfo {
    WindowInfo {
        id: id.into(),
        title: title.into(),
        app: app.into(),
        pid: ProcessId::new(pid),
        process_instance: Some(generation.into()),
        bounds: None,
        state: WindowState::default(),
    }
}

fn baseline(apps: Vec<AppInfo>, windows: Vec<WindowInfo>) -> SignalBaseline {
    SignalBaseline {
        apps,
        windows,
        surfaces: Vec::new(),
        completeness: SignalCompleteness {
            apps: true,
            windows: true,
            surfaces: false,
        },
    }
}

#[test]
fn process_replacement_and_window_recreation_are_semantic_and_handle_free() {
    let before = baseline(
        vec![app("Editor", 41, "proc-a")],
        vec![window("Editor", "Document", 41, "proc-a", "0xDEADBEEF")],
    );
    let after = baseline(
        vec![app("Editor", 57, "proc-b")],
        vec![window("Editor", "Document", 57, "proc-b", "0xCAFEBABE")],
    );

    let events = runtime_invalidation_diff(&before, &after);
    assert!(events.iter().any(|event| matches!(
        event,
        RuntimeEvent::ProcessReplaced { previous, current }
            if previous.process_instance() == "proc-a"
                && current.process_instance() == "proc-b"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        RuntimeEvent::WindowGenerationChanged { previous, current }
            if previous.title() == "Document" && current.title() == "Document"
    )));

    let debug = format!("{events:?}");
    assert!(!debug.contains("DEADBEEF"));
    assert!(!debug.contains("CAFEBABE"));
}

#[test]
fn unrelated_app_change_stays_scoped() {
    let before = baseline(
        vec![
            app("Editor", 41, "editor-a"),
            app("Browser", 50, "browser-a"),
        ],
        vec![window("Editor", "Document", 41, "editor-a", "w1")],
    );
    let after = baseline(
        vec![
            app("Editor", 41, "editor-a"),
            app("Browser", 51, "browser-b"),
        ],
        vec![window("Editor", "Document", 41, "editor-a", "w1")],
    );

    let events = runtime_invalidation_diff(&before, &after);
    assert_eq!(events.len(), 1);
    match &events[0] {
        RuntimeEvent::ProcessReplaced { previous, current } => {
            assert_eq!(previous.app(), "Browser");
            assert_eq!(current.app(), "Browser");
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn incomplete_provider_snapshot_does_not_fabricate_realtime_events() {
    let mut before = baseline(vec![app("Editor", 41, "a")], Vec::new());
    let after = baseline(Vec::new(), Vec::new());
    before.completeness.apps = false;
    assert!(runtime_invalidation_diff(&before, &after).is_empty());
}

#[test]
fn oversized_native_metadata_fails_safe_to_provider_reset() {
    let oversized = "x".repeat(MAX_SCOPE_TEXT_BYTES + 1);
    let before = baseline(vec![app(&oversized, 41, "a")], Vec::new());
    let after = baseline(Vec::new(), Vec::new());
    assert_eq!(
        runtime_invalidation_diff(&before, &after),
        vec![RuntimeEvent::ProviderReset]
    );
}

#[test]
fn bounded_bus_has_deterministic_drop_oldest_overflow_semantics() {
    let mut bus = RuntimeEventBus::new(2);
    let cursor = bus.cursor();
    bus.publish(RuntimeEvent::ProviderReset);
    bus.publish(RuntimeEvent::ProviderReset);
    bus.publish(RuntimeEvent::ProviderReset);

    let batch = bus.read_since(cursor);
    assert!(batch.overflowed);
    assert_eq!(batch.events.len(), 2);
    assert_eq!(batch.metrics.capacity, 2);
    assert_eq!(batch.metrics.depth, 2);
    assert_eq!(batch.metrics.max_depth, 2);
    assert_eq!(batch.metrics.published, 3);
    assert_eq!(batch.metrics.dropped, 1);
}

#[test]
fn duplicate_events_preserve_order_without_unbounded_growth() {
    let process = RuntimeProcessScope::new("Editor", ProcessId::new(41), "proc-a")
        .expect("bounded process scope");
    let event = RuntimeEvent::AccessibilityTreeInvalidated { process };
    let mut bus = RuntimeEventBus::new(4);
    let cursor = bus.cursor();
    for _ in 0..20 {
        bus.publish(event.clone());
    }
    let batch = bus.read_since(cursor);
    assert!(batch.overflowed);
    assert_eq!(batch.events, vec![event; 4]);
    assert_eq!(batch.metrics.depth, 4);
    assert_eq!(batch.metrics.dropped, 16);
}
