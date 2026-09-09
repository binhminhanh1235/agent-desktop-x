use crate::{
    ElementIdentifier, IdentifierEvidence, IdentifierKind, LiveElement, LocatorQuery, RefEntry,
    SnapshotSurface, WindowInfo, refs::RefPath, search_text,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, OnceLock},
};

const MAX_PROFILES: usize = 256;
const MAX_SEMANTIC_PATH: usize = 64;
const MAX_FUZZY_CHARS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AppProfileKey(String);

#[derive(Debug, Clone)]
pub(crate) struct AppProfile {
    window: WindowSignature,
    selector: SemanticSelectorRecipe,
    live: LiveRefCacheEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowSignature {
    app: String,
    title: String,
    surface: SnapshotSurface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveGeneration {
    pid: u32,
    process_instance: String,
    window_id: String,
}

#[derive(Debug, Clone)]
struct LiveRefCacheEntry {
    generation: LiveGeneration,
    entry: RefEntry,
}

#[derive(Debug, Clone)]
pub(crate) struct SemanticSelectorRecipe {
    role: String,
    name: Option<String>,
    normalized_name: Option<String>,
    description: Option<String>,
    normalized_description: Option<String>,
    value: Option<String>,
    normalized_value: Option<String>,
    stable_identifier: Option<ElementIdentifier>,
    display_ancestor_path: Vec<String>,
    semantic_ancestor_path: Vec<String>,
    supported_actions: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum CacheLookup {
    Miss,
    Hit(AppProfile),
    Invalidated,
}

#[derive(Default)]
struct CacheState {
    entries: HashMap<AppProfileKey, AppProfile>,
    order: VecDeque<AppProfileKey>,
}

static CACHE: OnceLock<Mutex<CacheState>> = OnceLock::new();

fn cache() -> &'static Mutex<CacheState> {
    CACHE.get_or_init(|| Mutex::new(CacheState::default()))
}

pub(crate) fn key_for_find(
    query: &LocatorQuery,
    app: Option<&str>,
    window_id: Option<&str>,
    surface: SnapshotSurface,
) -> Option<AppProfileKey> {
    if !query_is_cacheable(query) {
        return None;
    }
    let query_json = serde_json::to_string(query).ok()?;
    Some(AppProfileKey(format!(
        "v1|{}|{}|{}|{}",
        surface.as_str(),
        app.map(search_text::normalize).unwrap_or_default(),
        window_id.unwrap_or_default(),
        query_json
    )))
}

fn query_is_cacheable(query: &LocatorQuery) -> bool {
    query.has_text.is_none()
        && query.states.is_empty()
        && query.containment.has.is_none()
        && query.containment.has_not.is_none()
        && !query.is_empty()
}

pub(crate) fn lookup(key: &AppProfileKey) -> Result<CacheLookup, crate::AppError> {
    let mut state = cache()
        .lock()
        .map_err(|_| crate::AppError::Internal("AppProfile cache lock is poisoned".into()))?;
    let Some(profile) = state.entries.get(key).cloned() else {
        return Ok(CacheLookup::Miss);
    };
    if !profile.validate() {
        state.entries.remove(key);
        state.order.retain(|candidate| candidate != key);
        return Ok(CacheLookup::Invalidated);
    }
    state.order.retain(|candidate| candidate != key);
    state.order.push_back(key.clone());
    Ok(CacheLookup::Hit(profile))
}

pub(crate) fn store(
    key: AppProfileKey,
    profile: AppProfile,
) -> Result<bool, crate::AppError> {
    if !profile.validate() {
        return Ok(false);
    }
    let mut state = cache()
        .lock()
        .map_err(|_| crate::AppError::Internal("AppProfile cache lock is poisoned".into()))?;
    state.order.retain(|candidate| candidate != &key);
    state.order.push_back(key.clone());
    state.entries.insert(key, profile);
    while state.entries.len() > MAX_PROFILES {
        let Some(oldest) = state.order.pop_front() else {
            break;
        };
        state.entries.remove(&oldest);
    }
    Ok(true)
}

pub(crate) fn invalidate(key: &AppProfileKey) -> Result<(), crate::AppError> {
    let mut state = cache()
        .lock()
        .map_err(|_| crate::AppError::Internal("AppProfile cache lock is poisoned".into()))?;
    state.entries.remove(key);
    state.order.retain(|candidate| candidate != key);
    Ok(())
}

impl AppProfile {
    pub(crate) fn from_match(
        window: &WindowInfo,
        surface: SnapshotSurface,
        entry: &RefEntry,
        display_path: &[String],
    ) -> Option<Self> {
        let process_instance = window
            .process_instance
            .as_deref()
            .filter(|value| !value.is_empty())?
            .to_string();
        let selector = SemanticSelectorRecipe::from_entry(entry, display_path)?;
        Some(Self {
            window: WindowSignature::new(window, surface),
            selector,
            live: LiveRefCacheEntry {
                generation: LiveGeneration {
                    pid: window.pid.get(),
                    process_instance,
                    window_id: window.id.clone(),
                },
                entry: entry.clone(),
            },
        })
    }

    pub(crate) fn window_matches(&self, window: &WindowInfo, surface: SnapshotSurface) -> bool {
        self.window == WindowSignature::new(window, surface)
    }

    pub(crate) fn live_generation_matches(&self, window: &WindowInfo) -> bool {
        let Some(process_instance) = window.process_instance.as_deref() else {
            return false;
        };
        self.live.generation.pid == window.pid.get()
            && self.live.generation.process_instance == process_instance
            && self.live.generation.window_id == window.id
    }

    pub(crate) fn semantic_entry(&self, window: &WindowInfo) -> Option<RefEntry> {
        self.selector.to_entry(window, self.window.surface)
    }

    pub(crate) fn selector(&self) -> &SemanticSelectorRecipe {
        &self.selector
    }

    pub(crate) fn live_path(&self) -> RefPath {
        self.live.entry.scope.path.clone()
    }

    pub(crate) fn refreshed(
        &self,
        window: &WindowInfo,
        entry: &RefEntry,
        observed_display_path: Option<&[String]>,
    ) -> Option<Self> {
        let display_path = observed_display_path
            .map(<[String]>::to_vec)
            .unwrap_or_else(|| self.selector.display_ancestor_path.clone());
        Self::from_match(window, self.window.surface, entry, &display_path)
    }

    fn validate(&self) -> bool {
        self.selector.validate()
            && !self.live.generation.process_instance.is_empty()
            && self.live.generation.pid != 0
            && self.live.entry.process.pid.get() == self.live.generation.pid
            && self.live.entry.process.process_instance.as_deref()
                == Some(self.live.generation.process_instance.as_str())
            && self.live.entry.source.source_window_id.as_deref()
                == Some(self.live.generation.window_id.as_str())
            && self.live.entry.source.source_surface == self.window.surface
            && self
                .live
                .entry
                .source
                .source_app
                .as_deref()
                .is_some_and(|app| search_text::normalize(app) == self.window.app)
    }
}

impl WindowSignature {
    fn new(window: &WindowInfo, surface: SnapshotSurface) -> Self {
        Self {
            app: search_text::normalize(&window.app),
            title: search_text::normalize(&window.title),
            surface,
        }
    }
}

impl SemanticSelectorRecipe {
    fn from_entry(entry: &RefEntry, display_path: &[String]) -> Option<Self> {
        if !crate::Role::is_canonical(&entry.identity.role) || entry.identity.role == "unknown" {
            return None;
        }
        let name = meaningful(entry.identity.name.as_deref());
        let description = meaningful(entry.identity.description.as_deref());
        let value = meaningful(entry.identity.value.as_deref());
        let stable_identifier = durable_identifier(entry.identity.native_id.as_ref());
        let display_ancestor_path = display_path
            .iter()
            .take(MAX_SEMANTIC_PATH)
            .cloned()
            .collect::<Vec<_>>();
        let semantic_ancestor_path = display_ancestor_path
            .iter()
            .map(|segment| search_text::normalize(segment))
            .collect::<Vec<_>>();
        if stable_identifier.is_none()
            && name.is_none()
            && description.is_none()
            && value.is_none()
            && semantic_ancestor_path.is_empty()
        {
            return None;
        }
        let mut supported_actions = entry.capabilities.available_actions.clone();
        supported_actions.sort();
        supported_actions.dedup();
        Some(Self {
            role: entry.identity.role.clone(),
            normalized_name: name.as_deref().map(search_text::normalize),
            name,
            normalized_description: description.as_deref().map(search_text::normalize),
            description,
            normalized_value: value.as_deref().map(search_text::normalize),
            value,
            stable_identifier,
            display_ancestor_path,
            semantic_ancestor_path,
            supported_actions,
        })
    }

    fn validate(&self) -> bool {
        crate::Role::is_canonical(&self.role)
            && self.role != "unknown"
            && self
                .stable_identifier
                .as_ref()
                .is_none_or(|identifier| {
                    durable_identifier(Some(identifier)).as_ref() == Some(identifier)
                })
            && normalized_matches(self.name.as_deref(), self.normalized_name.as_deref())
            && normalized_matches(
                self.description.as_deref(),
                self.normalized_description.as_deref(),
            )
            && normalized_matches(self.value.as_deref(), self.normalized_value.as_deref())
            && self.semantic_ancestor_path.len() <= MAX_SEMANTIC_PATH
            && self.display_ancestor_path.len() == self.semantic_ancestor_path.len()
            && self
                .display_ancestor_path
                .iter()
                .zip(&self.semantic_ancestor_path)
                .all(|(display, semantic)| search_text::normalize(display) == *semantic)
            && self.supported_actions.len() <= 256
            && self.supported_actions.iter().all(|action| action.len() <= 256)
            && (self.stable_identifier.is_some()
                || self.normalized_name.is_some()
                || self.normalized_description.is_some()
                || self.normalized_value.is_some()
                || !self.semantic_ancestor_path.is_empty())
    }

    pub(crate) fn has_stable_identifier(&self) -> bool {
        self.stable_identifier.is_some()
    }

    pub(crate) fn has_semantic_path(&self) -> bool {
        !self.semantic_ancestor_path.is_empty()
    }

    pub(crate) fn display_path(&self) -> &[String] {
        &self.display_ancestor_path
    }

    pub(crate) fn path_matches(&self, candidate: &[String]) -> bool {
        candidate.len() == self.semantic_ancestor_path.len()
            && candidate
                .iter()
                .map(|segment| search_text::normalize(segment))
                .eq(self.semantic_ancestor_path.iter().cloned())
    }

    pub(crate) fn exact_query(&self, include_identifier: bool) -> LocatorQuery {
        LocatorQuery {
            identity: crate::IdentityPredicate {
                role: Some(self.role.clone()),
                name: self.normalized_name.clone(),
                description: self
                    .normalized_name
                    .is_none()
                    .then(|| self.normalized_description.clone())
                    .flatten(),
                native_id: include_identifier
                    .then(|| self.stable_identifier.as_ref().map(|id| id.value.clone()))
                    .flatten(),
                value: (self.normalized_name.is_none() && self.normalized_description.is_none())
                    .then(|| self.normalized_value.clone())
                    .flatten(),
            },
            exact: true,
            ..LocatorQuery::default()
        }
    }

    pub(crate) fn role_query(&self) -> LocatorQuery {
        LocatorQuery {
            identity: crate::IdentityPredicate {
                role: Some(self.role.clone()),
                ..crate::IdentityPredicate::default()
            },
            exact: true,
            ..LocatorQuery::default()
        }
    }

    pub(crate) fn matches_live(&self, live: &LiveElement, allow_identifier_refresh: bool) -> bool {
        if live.state.role != self.role {
            return false;
        }
        if let Some(expected) = &self.stable_identifier {
            let id_matches = live
                .identity
                .identifiers
                .identifiers()
                .iter()
                .any(|identifier| identifier == expected);
            if !id_matches && !allow_identifier_refresh {
                return false;
            }
            if !id_matches
                && allow_identifier_refresh
                && self.normalized_name.is_none()
                && self.normalized_description.is_none()
                && self.normalized_value.is_none()
            {
                return false;
            }
        }
        if let Some(expected) = self.normalized_name.as_deref() {
            return live
                .identity
                .name
                .known()
                .is_some_and(|actual| search_text::normalize(actual) == expected);
        }
        if let Some(expected) = self.normalized_value.as_deref() {
            return live
                .state
                .value
                .as_deref()
                .is_some_and(|actual| search_text::normalize(actual) == expected);
        }
        if let Some(expected) = self.normalized_description.as_deref() {
            return live
                .identity
                .description
                .known()
                .is_some_and(|actual| search_text::normalize(actual) == expected);
        }
        self.stable_identifier.is_some() && !allow_identifier_refresh
    }

    pub(crate) fn fuzzy_name_matches(&self, candidate: &str) -> bool {
        let Some(expected) = self.normalized_name.as_deref() else {
            return false;
        };
        let actual = search_text::normalize(candidate);
        one_edit_or_less(expected, &actual)
    }

    fn to_entry(&self, window: &WindowInfo, surface: SnapshotSurface) -> Option<RefEntry> {
        let process_instance = window
            .process_instance
            .as_deref()
            .filter(|value| !value.is_empty())?
            .to_string();
        Some(RefEntry {
            process: crate::RefProcess {
                pid: window.pid,
                process_instance: Some(process_instance),
            },
            identity: crate::RefEntryIdentity {
                role: self.role.clone(),
                name: self.name.clone(),
                value: self.value.clone(),
                description: self.description.clone(),
                native_id: self.stable_identifier.clone(),
            },
            geometry: crate::RefGeometry {
                bounds: None,
                bounds_hash: None,
            },
            capabilities: crate::RefCapabilities {
                states: Vec::new(),
                available_actions: self.supported_actions.clone(),
            },
            source: crate::RefSource {
                source_app: Some(window.app.clone()),
                source_window_id: Some(window.id.clone()),
                source_window_title: Some(window.title.clone()),
                source_window_bounds_hash: None,
                source_surface: surface,
            },
            scope: crate::RefScope {
                root_ref: None,
                path_is_absolute: false,
                path: RefPath::new(),
            },
        })
    }
}

pub(crate) fn entry_from_live(
    window: &WindowInfo,
    surface: SnapshotSurface,
    live: &LiveElement,
    path: RefPath,
) -> Option<RefEntry> {
    let process_instance = window
        .process_instance
        .as_deref()
        .filter(|value| !value.is_empty())?
        .to_string();
    Some(RefEntry {
        process: crate::RefProcess {
            pid: window.pid,
            process_instance: Some(process_instance),
        },
        identity: crate::RefEntryIdentity {
            role: live.state.role.clone(),
            name: live.identity.name.known().cloned().filter(|value| !value.trim().is_empty()),
            value: live.state.value.clone().filter(|value| !value.trim().is_empty()),
            description: live
                .identity
                .description
                .known()
                .cloned()
                .filter(|value| !value.trim().is_empty()),
            native_id: durable_identifier_from_evidence(&live.identity.identifiers),
        },
        geometry: crate::RefGeometry {
            bounds: None,
            bounds_hash: None,
        },
        capabilities: crate::RefCapabilities {
            states: live.state.states.clone(),
            available_actions: live.available_actions.clone(),
        },
        source: crate::RefSource {
            source_app: Some(window.app.clone()),
            source_window_id: Some(window.id.clone()),
            source_window_title: Some(window.title.clone()),
            source_window_bounds_hash: None,
            source_surface: surface,
        },
        scope: crate::RefScope {
            root_ref: None,
            path_is_absolute: false,
            path,
        },
    })
}

fn durable_identifier(identifier: Option<&ElementIdentifier>) -> Option<ElementIdentifier> {
    identifier
        .filter(|identifier| {
            !identifier.value.trim().is_empty()
                && matches!(
                    identifier.kind,
                    IdentifierKind::AutomationId
                        | IdentifierKind::AxIdentifier
                        | IdentifierKind::AxDomIdentifier
                )
        })
        .cloned()
}

fn durable_identifier_from_evidence(evidence: &IdentifierEvidence) -> Option<ElementIdentifier> {
    evidence
        .preferred_identifier()
        .and_then(|identifier| durable_identifier(Some(identifier)))
        .or_else(|| {
            evidence
                .identifiers()
                .iter()
                .find_map(|identifier| durable_identifier(Some(identifier)))
        })
}

fn meaningful(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn normalized_matches(raw: Option<&str>, normalized: Option<&str>) -> bool {
    match (raw, normalized) {
        (Some(raw), Some(normalized)) => search_text::normalize(raw) == normalized,
        (None, None) => true,
        _ => false,
    }
}

fn one_edit_or_less(expected: &str, actual: &str) -> bool {
    let expected = expected.chars().collect::<Vec<_>>();
    let actual = actual.chars().collect::<Vec<_>>();
    if expected.len() < 5
        || actual.len() < 5
        || expected.len() > MAX_FUZZY_CHARS
        || actual.len() > MAX_FUZZY_CHARS
        || expected.len().abs_diff(actual.len()) > 1
    {
        return false;
    }
    let (mut left, mut right, mut edits) = (0_usize, 0_usize, 0_u8);
    while left < expected.len() && right < actual.len() {
        if expected[left] == actual[right] {
            left += 1;
            right += 1;
            continue;
        }
        edits += 1;
        if edits > 1 {
            return false;
        }
        match expected.len().cmp(&actual.len()) {
            std::cmp::Ordering::Equal => {
                left += 1;
                right += 1;
            }
            std::cmp::Ordering::Greater => left += 1,
            std::cmp::Ordering::Less => right += 1,
        }
    }
    edits + u8::from(left < expected.len() || right < actual.len()) <= 1
}

#[cfg(test)]
pub(crate) fn clear_for_tests() {
    if let Ok(mut state) = cache().lock() {
        state.entries.clear();
        state.order.clear();
    }
}

#[cfg(test)]
pub(crate) fn corrupt_for_tests(key: &AppProfileKey) {
    if let Ok(mut state) = cache().lock()
        && let Some(profile) = state.entries.get_mut(key)
    {
        profile.selector.role = "unknown".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_identifier_is_never_promoted_to_durable_semantics() {
        let entry = crate::refs_test_support::sample_ref_entry_with_id(
            crate::IdentifierKind::RuntimeId,
            "runtime-42",
        );
        let selector = SemanticSelectorRecipe::from_entry(&entry, &["group:Editor".into()])
            .expect("semantic text/path still makes the selector cacheable");
        assert!(!selector.has_stable_identifier());
        assert!(selector.has_semantic_path());
    }

    #[test]
    fn fuzzy_matching_is_bounded_to_one_edit() {
        let mut entry = crate::refs_test_support::sample_ref_entry();
        entry.identity.name = Some("Settings".into());
        let selector = SemanticSelectorRecipe::from_entry(&entry, &[]).unwrap();
        assert!(selector.fuzzy_name_matches("Setting"));
        assert!(!selector.fuzzy_name_matches("Sessions"));
    }
}
