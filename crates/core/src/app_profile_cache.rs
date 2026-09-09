use crate::{LocatorQuery, RefEntry, SnapshotSurface, WindowInfo, refs::RefPath, search_text};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, OnceLock},
};

#[path = "app_profile_selector.rs"]
mod selector;

pub(crate) use selector::{SemanticSelectorRecipe, entry_from_live};

const MAX_PROFILES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AppProfileKey(String);

#[derive(Debug, Clone)]
pub(crate) struct AppProfile {
    window: WindowSignature,
    selector: SemanticSelectorRecipe,
    live: LiveRefCacheEntry,
}

#[derive(Debug, Clone)]
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
pub(crate) enum CacheLookup {
    Miss,
    Hit(Box<AppProfile>),
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
    query.exact
        && query.has_text.is_none()
        && query.states.is_empty()
        && query.containment.has.is_none()
        && query.containment.has_not.is_none()
        && (query.identity.name.is_some()
            || query.identity.description.is_some()
            || query.identity.native_id.is_some()
            || query.identity.value.is_some())
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
    Ok(CacheLookup::Hit(Box::new(profile)))
}

pub(crate) fn store(key: AppProfileKey, profile: AppProfile) -> Result<bool, crate::AppError> {
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
        self.window.surface == surface && self.window.app == search_text::normalize(&window.app)
    }

    pub(crate) fn live_generation_matches(&self, window: &WindowInfo) -> bool {
        let Some(process_instance) = window.process_instance.as_deref() else {
            return false;
        };
        self.live.generation.pid == window.pid.get()
            && self.live.generation.process_instance == process_instance
            && self.live.generation.window_id == window.id
    }

    pub(crate) fn live_entry(&self) -> &RefEntry {
        &self.live.entry
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
            .unwrap_or_else(|| self.selector.display_path().to_vec());
        Self::from_match(window, self.window.surface, entry, &display_path)
    }

    fn validate(&self) -> bool {
        self.selector.validate()
            && !self.window.app.is_empty()
            && !self.window.title.is_empty()
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
        profile.selector.invalidate_for_test();
    }
}
