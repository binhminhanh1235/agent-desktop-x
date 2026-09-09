use super::{FindArgs, FindFilterArgs, FindSelectionArgs, locator_query_from_args};
use crate::{
    AccessibilityNode, AdapterError, ElementIdentifier, IdentifierEvidence, IdentifierKind,
    LiveElement, LiveIdentity, LocatorField, NodeIdentity, NodePresentation, Rect, WindowInfo,
    adapter::{ActionOps, InputOps, NativeHandle, ObservationOps, SystemOps, WindowFilter},
    app_profile_cache::AppProfileKey,
    live_locator::{ObservationRequest, ObservationRoot},
};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

pub(super) static TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone)]
pub(super) struct FixtureState {
    pub(super) pid: u32,
    pub(super) process_instance: String,
    pub(super) window_id: String,
    pub(super) bounds_x: f64,
    pub(super) duplicates: usize,
    pub(super) stable_id: String,
    pub(super) name: String,
}

pub(super) struct ProfileCacheAdapter {
    pub(super) state: Mutex<FixtureState>,
    tree_reads: AtomicU64,
    strict_resolves: AtomicU64,
    live_reads: AtomicU64,
}

impl ProfileCacheAdapter {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(FixtureState {
                pid: 410,
                process_instance: "proc-a".into(),
                window_id: "w-profile".into(),
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

    pub(super) fn mutate(&self, update: impl FnOnce(&mut FixtureState)) {
        update(&mut self.state.lock().expect("fixture lock"));
    }

    pub(super) fn counts(&self) -> (u64, u64, u64) {
        (
            self.tree_reads.load(Ordering::SeqCst),
            self.strict_resolves.load(Ordering::SeqCst),
            self.live_reads.load(Ordering::SeqCst),
        )
    }

    pub(super) fn window(state: &FixtureState) -> WindowInfo {
        WindowInfo {
            id: state.window_id.clone(),
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
                available_actions: vec![crate::capability::CLICK.into()],
                bounds: Some(Rect {
                    x: state.bounds_x,
                    y: 10.0,
                    width: 50.0,
                    height: 20.0,
                }),
                ..NodePresentation::default()
            },
            children_count: None,
            subtree_truncated: false,
            children: Vec::new(),
        }
    }

    fn tree(state: &FixtureState) -> AccessibilityNode {
        let buttons = (0..state.duplicates)
            .map(|_| Self::button(state))
            .collect::<Vec<_>>();
        AccessibilityNode {
            ref_id: None,
            role: "window".into(),
            identity: NodeIdentity {
                name: Some("Profile Fixture".into()),
                ..NodeIdentity::default()
            },
            presentation: NodePresentation::default(),
            children_count: None,
            subtree_truncated: false,
            children: vec![AccessibilityNode {
                ref_id: None,
                role: "group".into(),
                identity: NodeIdentity {
                    name: Some("Primary".into()),
                    ..NodeIdentity::default()
                },
                presentation: NodePresentation::default(),
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
        let state = self.state.lock().expect("fixture lock").clone();
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
        Ok(vec![Self::window(
            &self.state.lock().expect("fixture lock"),
        )])
    }

    fn resolve_element_strict(
        &self,
        entry: &crate::RefEntry,
        _deadline: crate::Deadline,
    ) -> Result<NativeHandle, AdapterError> {
        self.strict_resolves.fetch_add(1, Ordering::SeqCst);
        let state = self.state.lock().expect("fixture lock");
        if entry
            .geometry
            .bounds
            .is_some_and(|bounds| bounds.x != state.bounds_x)
        {
            return Err(AdapterError::stale_ref("@profile"));
        }
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
        Ok(Self::live(&self.state.lock().expect("fixture lock")))
    }
}

impl ActionOps for ProfileCacheAdapter {}
impl InputOps for ProfileCacheAdapter {}
impl SystemOps for ProfileCacheAdapter {
    fn supported_surfaces(&self) -> Vec<crate::SnapshotSurface> {
        vec![crate::SnapshotSurface::Window]
    }
}

pub(super) fn find_args() -> FindArgs {
    FindArgs {
        app: Some("ProfileApp".into()),
        window_id: None,
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

pub(super) fn cache_key(args: &FindArgs) -> AppProfileKey {
    let query = locator_query_from_args(args).expect("query");
    crate::app_profile_cache::key_for_find(
        &query,
        args.app.as_deref(),
        args.window_id.as_deref(),
        args.surface,
    )
    .expect("cache key")
}
