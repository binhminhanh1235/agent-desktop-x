use super::*;
use agent_desktop_core::ProcessId;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

fn scoped_store(app: &str) -> (ViewStore, StoredView) {
    let mut store = ViewStore::new(8, Duration::from_secs(30));
    let view = store.insert(
        ObservationScope {
            command: "list-windows".into(),
            app: Some(app.to_lowercase()),
        },
        BTreeMap::new(),
        None,
        Instant::now(),
    );
    (store, view)
}

#[test]
fn matching_runtime_event_makes_view_stale_without_touching_unrelated_scope() {
    let now = Instant::now();
    let mut store = ViewStore::new(8, Duration::from_secs(30));
    let editor = store.insert(
        ObservationScope {
            command: "list-windows".into(),
            app: Some("editor".into()),
        },
        BTreeMap::new(),
        None,
        now,
    );
    let browser = store.insert(
        ObservationScope {
            command: "list-windows".into(),
            app: Some("browser".into()),
        },
        BTreeMap::new(),
        None,
        now,
    );
    let previous = agent_desktop_core::runtime_events::RuntimeProcessScope::new(
        "Editor",
        ProcessId::new(42),
        "proc-generation-1",
    )
    .expect("bounded scope");
    let current = agent_desktop_core::runtime_events::RuntimeProcessScope::new(
        "Editor",
        ProcessId::new(43),
        "proc-generation-2",
    )
    .expect("bounded scope");
    store.apply_event(
        &agent_desktop_core::runtime_events::RuntimeEvent::ProcessReplaced { previous, current },
    );

    let error = store
        .live(&editor.id.0, now)
        .expect_err("editor view must be stale");
    assert!(error.to_string().contains("VIEW_STALE"));
    assert!(store.live(&browser.id.0, now).is_ok());
    assert_eq!(store.invalidation.views_invalidated, 1);
    assert_eq!(store.invalidation.unrelated_views_retained, 1);
}

#[test]
fn duplicate_view_invalidation_is_idempotent() {
    let (mut store, view) = scoped_store("Editor");
    let process = agent_desktop_core::runtime_events::RuntimeProcessScope::new(
        "Editor",
        ProcessId::new(42),
        "proc-generation-1",
    )
    .expect("bounded scope");
    let event =
        agent_desktop_core::runtime_events::RuntimeEvent::AccessibilityTreeInvalidated { process };
    store.apply_event(&event);
    store.apply_event(&event);
    assert!(
        store
            .entries
            .get(&view.id)
            .is_some_and(|view| view.invalidated)
    );
    assert_eq!(store.invalidation.events_applied, 2);
    assert_eq!(store.invalidation.views_invalidated, 1);
}

#[test]
fn overflow_marks_all_existing_views_stale_but_keeps_bounded_store() {
    let now = Instant::now();
    let mut store = ViewStore::new(2, Duration::from_secs(30));
    let first = store.insert(
        ObservationScope {
            command: "list-windows".into(),
            app: Some("editor".into()),
        },
        BTreeMap::new(),
        None,
        now,
    );
    let second = store.insert(
        ObservationScope {
            command: "list-windows".into(),
            app: Some("browser".into()),
        },
        BTreeMap::new(),
        None,
        now,
    );
    store.invalidate_all_for_overflow();
    assert_eq!(store.entries.len(), 2);
    assert!(store.live(&first.id.0, now).is_err());
    assert!(store.live(&second.id.0, now).is_err());
    assert_eq!(store.invalidation.views_invalidated, 2);
    assert_eq!(store.invalidation.overflow_resets, 1);
}

#[test]
fn expiry_keeps_precedence_after_event_invalidation() {
    let start = Instant::now();
    let mut store = ViewStore::new(2, Duration::from_millis(1));
    let view = store.insert(
        ObservationScope {
            command: "list-windows".into(),
            app: Some("editor".into()),
        },
        BTreeMap::new(),
        None,
        start,
    );
    let process = agent_desktop_core::runtime_events::RuntimeProcessScope::new(
        "Editor",
        ProcessId::new(42),
        "proc-generation-1",
    )
    .expect("bounded scope");
    store.apply_event(
        &agent_desktop_core::runtime_events::RuntimeEvent::ProcessExited { previous: process },
    );
    let error = store
        .live(&view.id.0, start + Duration::from_millis(2))
        .expect_err("expired view");
    assert!(error.to_string().contains("VIEW_EXPIRED"));
}
