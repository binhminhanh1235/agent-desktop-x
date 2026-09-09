use super::*;
use crate::{
    AccessibilityNode, AdapterError, ElementIdentifier, IdentifierEvidence, IdentifierKind,
    LiveElement, LiveIdentity, LocatorField, NodeIdentity, NodePresentation, Rect, WindowInfo,
    adapter::{ActionOps, InputOps, NativeHandle, ObservationOps, SystemOps, WindowFilter},
    app_profile_cache::{self, CacheLookup},
    live_locator::{ObservationRequest, ObservationRoot},
    refs_test_support::HomeGuard,
};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone)]
struct FixtureState {
    pid: u32,
    process_instance: String,
    bounds_x: f64,
    duplicates: usize,
    stable_id: String,
    name: String,
}

struct ProfileCacheAdapter {
    state: Mutex<FixtureState>,
    tree_reads: AtomicU64,
    strict_resolves: AtomicU64,
    live_reads: AtomicU64,
}

impl ProfileCacheAdapter {
    fn new() -> Self {
        Self {
            state: Mutex::new(FixtureState {
                pid: 410,
                process_instance: "proc-a".into(),
                bounds_x: 10.0,
                duplicates: 1,
                stable_id: "run-primary".into(),
                name: "Run".into(),
            }),
            tree_reads: AtomicU64::new(0),
            strict_resolves: AtomicU64::new(0),
            live_reads: AtomicU64::new(0),
        }
    }

    fn mutate(&self, update: impl FnOnce(&mut FixtureState)) {
        update(&mut self.state.lock().unwrap());
    }

    fn counts(&self) -> (u64, u64, u64) {
        (
            self.tree_reads.load(Ordering::SeqCst),
            self.strict_resolves.load(Ordering::SeqCst),
            self.live_reads.load(Ordering::SeqCst),
        )
    }

    fn window(state: &FixtureState) -> WindowInfo {
        WindowInfo {
            id: "w-profile".into(),
            title: "Profile Fixture".into(),
            app: "ProfileApp".into(),
            pid: crate::ProcessId::new(state.pid),
            process_instance: Some(state.process_instance.clone()),
            bounds: None,
            state: crate::WindowState {
                is_focused: true,
                ..Default::default()
            },
        }
    }

    fn button(state: &FixtureState) -> AccessibilityNode {
        AccessibilityNode {
            ref_id: None,
            role: "button".into(),
            identity: NodeIdentity {
                name: Some(state.name.clone()),
                value: None,
                description: None,
                native_id: Some(ElementIdentifier {
                    kind: IdentifierKind::AutomationId,
                    value: state.stable_id.clone(),
                }),
            },
            presentation: NodePresentation {
                hint: None,
                states: Vec::new(),
                available_actions: vec![crate::capability::CLICK.into()],
                bounds: Some(Rect {
                    x: state.bounds_x,
                    y: 10.0,
                    width: 50.0,
                    height: 20.0,
                }),
                descriptors: Default::default(),
            },
            children_count: None,
            subtree_truncated: false,
            children: Vec::new(),
        }
    }

    fn tree(state: &FixtureState) -> AccessibilityNode {
        let mut buttons = Vec::new();
        for _ in 0..state.duplicates {
            buttons.push(Self::button(state));
        }
        AccessibilityNode {
            ref_id: None,
            role: "window".into(),
            identity: NodeIdentity {
                name: Some("Profile Fixture".into()),
                ..NodeIdentity::default()
            },
            presentation: NodePresentation {
                hint: None,
                states: Vec::new(),
                available_actions: Vec::new(),
                bounds: None,
                descriptors: Default::default(),
            },
            children_count: None,
            subtree_truncated: false,
            children: vec![AccessibilityNode {
                ref_id: None,
                role: "group".into(),
                identity: NodeIdentity {
                    name: Some("Primary".into()),
                    ..NodeIdentity::default()
                },
                presentation: NodePresentation {
                    hint: None,
                    states: Vec::new(),
                    available_actions: Vec::new(),
                    bounds: None,
                    descriptors: Default::default(),
                },
                children_count: None,
                subtree_truncated: false,
                children: buttons,
            }],
        }
    }

    fn live(state: &FixtureState) -> LiveElement {
        LiveElement {
            identity: LiveIdentity {
                name: LocatorField::Known(state.name.clone()),
                description: LocatorField::Absent,
                identifiers: IdentifierEvidence::typed(
                    [ElementIdentifier {
                        kind: IdentifierKind::AutomationId,
                        value: state.stable_id.clone(),
                    }],
                    Some(0),
                    true,
                ),
            },
            state: crate::ElementState {
                role: "button".into(),
                states: Vec::new(),
                value: None,
                enabled: Some(true),
                hidden: Some(false),
                offscreen: Some(false),
            },
            states_complete: true,
            bounds: Some(Rect {
                x: state.bounds_x,
                y: 10.0,
                width: 50.0,
                height: 20.0,
            }),
            available_actions: vec![crate::capability::CLICK.into()],
        }
    }
}

impl ObservationOps for ProfileCacheAdapter {
    fn observe_tree(
        &self,
        root: ObservationRoot<'_>,
        _request: &ObservationRequest,
    ) -> Result<crate::ObservedTree, AdapterError> {
        self.tree_reads.fetch_add(1, Ordering::SeqCst);
        let state = self.state.lock().unwrap().clone();
        let node = match root {
            ObservationRoot::Window(_) => Self::tree(&state),
            ObservationRoot::Element { .. } => Self::button(&state),
        };
        crate::adapter::observed_tree(&root, node)
    }

    fn list_windows(
        &self,
        _filter: &WindowFilter,
        _deadline: crate::Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        Ok(vec![Self::window(&self.state.lock().unwrap())])
    }

    fn resolve_element_strict(
        &self,
        entry: &crate::RefEntry,
        _deadline: crate::Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        self.strict_resolves.fetch_add(1, Ordering::SeqCst);
        let state = self.state.lock().unwrap();
        let id_matches = entry
            .identity
            .native_id
            .as_ref()
            .is_some_and(|id| id.value == state.stable_id);
        let text_matches = entry.identity.role == "button"
            && entry.identity.name.as_deref() == Some(state.name.as_str());
        if !id_matches && !text_matches {
            return Err(AdapterError::stale_ref("@profile"));
        }
        if state.duplicates > 1 {
            return Err(AdapterError::ambiguous_target(
                "fixture contains duplicate semantic candidates",
            ));
        }
        Ok(NativeHandle::null())
    }

    fn resolve_locator_anchor(
        &self,
        _entry: &crate::RefEntry,
        _deadline: crate::Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        Ok(NativeHandle::null())
    }

    fn get_live_element(
        &self,
        _handle: &NativeHandle,
        _deadline: crate::Deadline,
    ) -> Result<LiveElement, AdapterError> {
        self.live_reads.fetch_add(1, Ordering::SeqCst);
        Ok(Self::live(&self.state.lock().unwrap()))
    }
}

impl ActionOps for ProfileCacheAdapter {}
impl InputOps for ProfileCacheAdapter {}
impl SystemOps for ProfileCacheAdapter {
    fn supported_surfaces(&self) -> Vec<crate::SnapshotSurface> {
        vec![crate::SnapshotSurface::Window]
    }
}

fn args() -> FindArgs {
    FindArgs {
        app: Some("ProfileApp".into()),
        window_id: Some("w-profile".into()),
        root: None,
        snapshot: None,
        surface: crate::SnapshotSurface::Window,
        filter: FindFilterArgs {
            role: Some("button".into()),
            name: Some("Run".into()),
            description: None,
            native_id: None,
            value: None,
            text: None,
            exact: true,
        },
        states: Vec::new(),
        selection: FindSelectionArgs {
            count: false,
            first: false,
            last: false,
            nth: None,
            limit: None,
        },
        timeout_ms: Some(1_000),
    }
}

fn cache_key(args: &FindArgs) -> crate::app_profile_cache::AppProfileKey {
    let query = locator_query_from_args(args).unwrap();
    crate::app_profile_cache::key_for_find(
        &query,
        args.app.as_deref(),
        args.window_id.as_deref(),
        args.surface,
    )
    .unwrap()
}

#[test]
fn first_lookup_learns_semantics_and_second_lookup_avoids_tree_walk() {
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();
    let args = args();

    let first = execute(args(), &adapter, &CommandContext::default()).unwrap();
    let after_cold = adapter.counts();
    assert_eq!(first["matches"].as_array().unwrap().len(), 1);
    assert!(after_cold.0 >= 1);

    match app_profile_cache::lookup(&cache_key(&args)).unwrap() {
        CacheLookup::Hit(profile) => {
            assert!(profile.selector().has_stable_identifier());
            assert!(profile.selector().has_semantic_path());
        }
        other => panic!("expected learned AppProfile, got {other:?}"),
    }

    let second = execute(args(), &adapter, &CommandContext::default()).unwrap();
    let after_warm = adapter.counts();
    assert_eq!(second["matches"].as_array().unwrap().len(), 1);
    assert_eq!(
        after_warm.0, after_cold.0,
        "warm semantic lookup must avoid a full tree observation"
    );
    assert!(after_warm.1 > after_cold.1);
    assert!(after_warm.2 > after_cold.2);
}

#[test]
fn bounds_motion_does_not_break_semantic_identity() {
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();

    execute(args(), &adapter, &CommandContext::default()).unwrap();
    let before = adapter.counts();
    adapter.mutate(|state| state.bounds_x = 640.0);

    let response = execute(args(), &adapter, &CommandContext::default()).unwrap();
    assert_eq!(response["matches"].as_array().unwrap().len(), 1);
    assert_eq!(adapter.counts().0, before.0);
}

#[test]
fn process_recreation_invalidates_live_generation_but_semantics_reresolve() {
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();

    execute(args(), &adapter, &CommandContext::default()).unwrap();
    let key = cache_key(&args());
    let before_profile = match app_profile_cache::lookup(&key).unwrap() {
        CacheLookup::Hit(profile) => profile,
        _ => panic!("profile must exist"),
    };
    adapter.mutate(|state| {
        state.pid = 411;
        state.process_instance = "proc-b".into();
    });
    let current_window = ProfileCacheAdapter::window(&adapter.state.lock().unwrap());
    assert!(!before_profile.live_generation_matches(&current_window));

    let before = adapter.counts();
    let response = execute(args(), &adapter, &CommandContext::default()).unwrap();
    assert_eq!(response["matches"].as_array().unwrap().len(), 1);
    assert_eq!(
        adapter.counts().0,
        before.0,
        "restart should reuse semantics without replaying the cold tree walk"
    );
}

#[test]
fn duplicate_semantic_candidates_refuse_the_warm_single_target() {
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();

    execute(args(), &adapter, &CommandContext::default()).unwrap();
    let before = adapter.counts();
    adapter.mutate(|state| state.duplicates = 2);

    let response = execute(args(), &adapter, &CommandContext::default()).unwrap();
    assert_eq!(response["matches"].as_array().unwrap().len(), 2);
    assert!(
        adapter.counts().0 > before.0,
        "ambiguous warm resolution must fall back to authoritative scoped discovery"
    );
}

#[test]
fn corrupted_cache_is_never_used_as_a_target() {
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();
    let args = args();

    execute(args(), &adapter, &CommandContext::default()).unwrap();
    let before = adapter.counts();
    let key = cache_key(&args);
    app_profile_cache::corrupt_for_tests(&key);

    let response = execute(args(), &adapter, &CommandContext::default()).unwrap();
    assert_eq!(response["matches"].as_array().unwrap().len(), 1);
    assert!(
        adapter.counts().0 > before.0,
        "invalid cache data must fail closed into fresh scoped discovery"
    );
}
