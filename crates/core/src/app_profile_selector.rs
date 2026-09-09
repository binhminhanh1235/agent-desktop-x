use crate::{
    ElementIdentifier, IdentifierEvidence, IdentifierKind, LiveElement, LocatorQuery, RefEntry,
    SnapshotSurface, WindowInfo, refs::RefPath, search_text,
};

const MAX_SEMANTIC_PATH: usize = 64;
const MAX_FUZZY_CHARS: usize = 128;

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

impl SemanticSelectorRecipe {
    pub(super) fn from_entry(entry: &RefEntry, display_path: &[String]) -> Option<Self> {
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

    pub(super) fn validate(&self) -> bool {
        crate::Role::is_canonical(&self.role)
            && self.role != "unknown"
            && self.stable_identifier.as_ref().is_none_or(|identifier| {
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
            && self
                .supported_actions
                .iter()
                .all(|action| action.len() <= 256)
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
                description: if self.normalized_name.is_none() {
                    self.normalized_description.clone()
                } else {
                    None
                },
                native_id: if include_identifier {
                    self.stable_identifier
                        .as_ref()
                        .map(|identifier| identifier.value.clone())
                } else {
                    None
                },
                value: if self.normalized_name.is_none() && self.normalized_description.is_none() {
                    self.normalized_value.clone()
                } else {
                    None
                },
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
        one_edit_or_less(expected, &search_text::normalize(candidate))
    }

    pub(super) fn to_entry(
        &self,
        window: &WindowInfo,
        surface: SnapshotSurface,
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

    #[cfg(test)]
    pub(super) fn invalidate_for_test(&mut self) {
        self.role = "unknown".into();
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
            name: live
                .identity
                .name
                .known()
                .cloned()
                .filter(|value| !value.trim().is_empty()),
            value: live
                .state
                .value
                .clone()
                .filter(|value| !value.trim().is_empty()),
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
