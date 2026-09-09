use super::FindArgs;
use crate::{
    AdapterError, AppError, ErrorCode, LocatorQuery, WindowInfo,
    adapter::PlatformAdapter,
    app_profile_cache::{self, AppProfile, AppProfileKey, CacheLookup},
    context::CommandContext,
    live_locator::{
        LocatorMatch, LocatorMatchData, LocatorMaterialization, LocatorResolution,
        LocatorResolutionMeta, LocatorResolveRequest, LocatorSelection, LocatorStats,
        ObservationRoot, resolve_query,
    },
    refs::{RefMap, RefPath},
};
use serde::Serialize;
use serde_json::json;

const MAX_REVALIDATION_CANDIDATES: u32 = 16;

#[derive(Debug, Clone, Default, Serialize)]
struct LookupStats {
    resolution_calls: u64,
    provider_reads: u64,
    tree_reads: u64,
    cold_lookups: u64,
    warm_lookups: u64,
}

impl LookupStats {
    fn add_locator(&mut self, stats: &LocatorStats) {
        self.tree_reads += stats.reads.counts.observation_attempts;
        self.provider_reads += stats.reads.counts.attributes_requested
            + stats.reads.counts.child_reads
            + stats.reads.counts.action_reads
            + stats.reads.counts.fallback_reads
            + stats.semantic_reads.child_label_reads
            + stats.semantic_reads.promotion_reads
            + stats.semantic_reads.settable_reads;
    }
}

struct WarmResolved {
    resolution: LocatorResolution,
    refreshed: AppProfile,
    stats: LookupStats,
}

enum WarmAttempt {
    Resolved(WarmResolved),
    Miss {
        status: &'static str,
        reason: &'static str,
        stats: LookupStats,
    },
}

struct FreshCandidate {
    entry: crate::RefEntry,
    display_path: Vec<String>,
}

pub(super) fn resolve(
    args: &FindArgs,
    query: &LocatorQuery,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
    window: &WindowInfo,
    request: &LocatorResolveRequest,
) -> Result<LocatorResolution, AppError> {
    let key = if args.root.is_none() && args.snapshot.is_none() {
        app_profile_cache::key_for_find(
            query,
            args.app.as_deref(),
            args.window_id.as_deref(),
            args.surface,
        )
    } else {
        None
    };
    let Some(key) = key else {
        return cold(query, adapter, window, request);
    };

    match app_profile_cache::lookup(&key)? {
        CacheLookup::Miss => {
            let resolution = cold(query, adapter, window, request)?;
            learn_after_cold(
                &key,
                window,
                args,
                &resolution,
                context,
                "cache_miss",
                "no_profile",
            )?;
            Ok(resolution)
        }
        CacheLookup::Invalidated => {
            emit(
                context,
                "invalidated",
                "cache_validation_failed",
                &LookupStats::default(),
            )?;
            let resolution = cold(query, adapter, window, request)?;
            learn_after_cold(
                &key,
                window,
                args,
                &resolution,
                context,
                "revalidated",
                "cold_after_invalid_cache",
            )?;
            Ok(resolution)
        }
        CacheLookup::Hit(profile) => {
            if !profile.window_matches(window, args.surface) {
                app_profile_cache::invalidate(&key)?;
                emit(
                    context,
                    "invalidated",
                    "window_signature_changed",
                    &LookupStats::default(),
                )?;
                let resolution = cold(query, adapter, window, request)?;
                learn_after_cold(
                    &key,
                    window,
                    args,
                    &resolution,
                    context,
                    "revalidated",
                    "cold_after_window_change",
                )?;
                return Ok(resolution);
            }

            let same_generation = profile.live_generation_matches(window);
            emit(
                context,
                if same_generation {
                    "live_hit"
                } else {
                    "semantic_hit"
                },
                if same_generation {
                    "generation_match"
                } else {
                    "generation_changed"
                },
                &LookupStats {
                    resolution_calls: 1,
                    warm_lookups: 1,
                    ..LookupStats::default()
                },
            )?;
            if !same_generation {
                emit(
                    context,
                    "invalidated",
                    "live_generation_changed",
                    &LookupStats::default(),
                )?;
            }

            match try_warm(&profile, window, args, adapter, request, same_generation)? {
                WarmAttempt::Resolved(warm) => {
                    app_profile_cache::store(key, warm.refreshed)?;
                    emit(context, "revalidated", "warm_semantic_target", &warm.stats)?;
                    Ok(warm.resolution)
                }
                WarmAttempt::Miss {
                    status,
                    reason,
                    stats,
                } => {
                    if status == "ambiguous" {
                        app_profile_cache::invalidate(&key)?;
                    }
                    emit(context, status, reason, &stats)?;
                    let resolution = cold(query, adapter, window, request)?;
                    learn_after_cold(
                        &key,
                        window,
                        args,
                        &resolution,
                        context,
                        if resolution.meta.total_matches > 1 {
                            "ambiguous"
                        } else {
                            "revalidated"
                        },
                        "scoped_cold_revalidation",
                    )?;
                    Ok(resolution)
                }
            }
        }
    }
}

fn cold(
    query: &LocatorQuery,
    adapter: &dyn PlatformAdapter,
    window: &WindowInfo,
    request: &LocatorResolveRequest,
) -> Result<LocatorResolution, AppError> {
    resolve_query(adapter, query, ObservationRoot::Window(window), request)
}

fn learn_after_cold(
    key: &AppProfileKey,
    window: &WindowInfo,
    args: &FindArgs,
    resolution: &LocatorResolution,
    context: &CommandContext,
    status: &'static str,
    reason: &'static str,
) -> Result<(), AppError> {
    let mut stats = LookupStats {
        resolution_calls: 1,
        cold_lookups: 1,
        ..LookupStats::default()
    };
    stats.add_locator(&resolution.stats);

    if resolution.meta.complete
        && resolution.meta.total_matches == 1
        && resolution.matches.len() == 1
    {
        let found = &resolution.matches[0];
        if let Some(profile) =
            AppProfile::from_match(window, args.surface, &found.entry, &found.data.path)
            && app_profile_cache::store(key.clone(), profile)?
        {
            emit(context, status, reason, &stats)?;
            return Ok(());
        }
    } else if resolution.meta.total_matches > 1 {
        app_profile_cache::invalidate(key)?;
    }
    emit(context, status, reason, &stats)
}

fn try_warm(
    profile: &AppProfile,
    window: &WindowInfo,
    args: &FindArgs,
    adapter: &dyn PlatformAdapter,
    request: &LocatorResolveRequest,
    same_generation: bool,
) -> Result<WarmAttempt, AppError> {
    let mut stats = LookupStats {
        resolution_calls: 1,
        warm_lookups: 1,
        ..LookupStats::default()
    };
    let selector = profile.selector();

    if selector.has_stable_identifier() {
        if let Some(entry) = profile.semantic_entry(window) {
            match adapter.resolve_element_strict(&entry, request.deadline) {
                Ok(handle) => {
                    stats.provider_reads += 1;
                    return finish_handle(
                        profile,
                        window,
                        args,
                        adapter,
                        request,
                        handle,
                        same_generation.then(|| profile.live_path()).unwrap_or_else(RefPath::new),
                        same_generation
                            .then(|| profile.selector().display_path().to_vec())
                            .unwrap_or_default(),
                        false,
                        stats,
                    );
                }
                Err(error) if error.code == ErrorCode::AmbiguousTarget => {
                    if selector.has_semantic_path()
                        && let Some(candidate) = semantic_path_candidate(
                            profile,
                            window,
                            adapter,
                            request,
                            false,
                            &mut stats,
                        )?
                    {
                        return finish_candidate(
                            profile,
                            window,
                            args,
                            adapter,
                            request,
                            candidate,
                            true,
                            stats,
                        );
                    }
                    return Ok(WarmAttempt::Miss {
                        status: "ambiguous",
                        reason: "stable_identifier_not_unique",
                        stats,
                    });
                }
                Err(error) if stale_or_unsupported(&error) => {}
                Err(error) => return Err(error.into()),
            }
        }
    } else if selector.has_semantic_path()
        && let Some(candidate) =
            semantic_path_candidate(profile, window, adapter, request, true, &mut stats)?
    {
        return finish_candidate(
            profile,
            window,
            args,
            adapter,
            request,
            candidate,
            false,
            stats,
        );
    } else if let Some(entry) = profile.semantic_entry(window) {
        match adapter.resolve_element_strict(&entry, request.deadline) {
            Ok(handle) => {
                stats.provider_reads += 1;
                return finish_handle(
                    profile,
                    window,
                    args,
                    adapter,
                    request,
                    handle,
                    same_generation.then(|| profile.live_path()).unwrap_or_else(RefPath::new),
                    same_generation
                        .then(|| profile.selector().display_path().to_vec())
                        .unwrap_or_default(),
                    false,
                    stats,
                );
            }
            Err(error) if error.code == ErrorCode::AmbiguousTarget => {
                return Ok(WarmAttempt::Miss {
                    status: "ambiguous",
                    reason: "role_name_not_unique",
                    stats,
                });
            }
            Err(error) if stale_or_unsupported(&error) => {}
            Err(error) => return Err(error.into()),
        }
    }

    if let Some(candidate) =
        semantic_path_candidate(profile, window, adapter, request, false, &mut stats)?
    {
        return finish_candidate(
            profile,
            window,
            args,
            adapter,
            request,
            candidate,
            true,
            stats,
        );
    }

    if let Some(candidate) = fuzzy_candidate(profile, window, adapter, request, &mut stats)? {
        return finish_candidate(
            profile,
            window,
            args,
            adapter,
            request,
            candidate,
            true,
            stats,
        );
    }

    Ok(WarmAttempt::Miss {
        status: "cache_miss",
        reason: "semantic_target_stale",
        stats,
    })
}

fn semantic_path_candidate(
    profile: &AppProfile,
    window: &WindowInfo,
    adapter: &dyn PlatformAdapter,
    request: &LocatorResolveRequest,
    include_identifier: bool,
    stats: &mut LookupStats,
) -> Result<Option<FreshCandidate>, AppError> {
    if !profile.selector().has_semantic_path() {
        return Ok(None);
    }
    let query = profile.selector().exact_query(include_identifier);
    let resolution = scoped_scan(adapter, window, request, &query)?;
    stats.add_locator(&resolution.stats);
    if !resolution.meta.complete || resolution.meta.truncated {
        return Ok(None);
    }
    let mut candidates = resolution
        .matches
        .into_iter()
        .filter(|candidate| profile.selector().path_matches(&candidate.data.path))
        .map(|candidate| FreshCandidate {
            entry: candidate.entry,
            display_path: candidate.data.path,
        })
        .collect::<Vec<_>>();
    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.pop()),
        _ => Err(AdapterError::ambiguous_target(
            "Semantic AppProfile path matched multiple live candidates",
        )
        .into()),
    }
}

fn fuzzy_candidate(
    profile: &AppProfile,
    window: &WindowInfo,
    adapter: &dyn PlatformAdapter,
    request: &LocatorResolveRequest,
    stats: &mut LookupStats,
) -> Result<Option<FreshCandidate>, AppError> {
    let query = profile.selector().role_query();
    let resolution = scoped_scan(adapter, window, request, &query)?;
    stats.add_locator(&resolution.stats);
    if !resolution.meta.complete || resolution.meta.truncated {
        return Ok(None);
    }
    let mut candidates = resolution
        .matches
        .into_iter()
        .filter(|candidate| {
            (!profile.selector().has_semantic_path()
                || profile.selector().path_matches(&candidate.data.path))
                && profile.selector().fuzzy_name_matches(&candidate.data.name)
        })
        .map(|candidate| FreshCandidate {
            entry: candidate.entry,
            display_path: candidate.data.path,
        })
        .collect::<Vec<_>>();
    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.pop()),
        _ => Err(AdapterError::ambiguous_target(
            "Bounded fuzzy AppProfile match is ambiguous",
        )
        .into()),
    }
}

fn scoped_scan(
    adapter: &dyn PlatformAdapter,
    window: &WindowInfo,
    request: &LocatorResolveRequest,
    query: &LocatorQuery,
) -> Result<LocatorResolution, AppError> {
    let scan_request = LocatorResolveRequest {
        selection: LocatorSelection::All {
            limit: Some(MAX_REVALIDATION_CANDIDATES),
        },
        materialization: LocatorMaterialization::None,
        ..*request
    };
    resolve_query(
        adapter,
        query,
        ObservationRoot::Window(window),
        &scan_request,
    )
}

fn finish_candidate(
    profile: &AppProfile,
    window: &WindowInfo,
    args: &FindArgs,
    adapter: &dyn PlatformAdapter,
    request: &LocatorResolveRequest,
    candidate: FreshCandidate,
    allow_identifier_refresh: bool,
    mut stats: LookupStats,
) -> Result<WarmAttempt, AppError> {
    let handle = match adapter.resolve_locator_anchor(&candidate.entry, request.deadline) {
        Ok(handle) => handle,
        Err(error) if stale_or_unsupported(&error) => {
            return Ok(WarmAttempt::Miss {
                status: "cache_miss",
                reason: "fresh_anchor_changed",
                stats,
            });
        }
        Err(error) => return Err(error.into()),
    };
    stats.provider_reads += 1;
    finish_handle(
        profile,
        window,
        args,
        adapter,
        request,
        handle,
        candidate.entry.scope.path,
        candidate.display_path,
        allow_identifier_refresh,
        stats,
    )
}

fn finish_handle(
    profile: &AppProfile,
    window: &WindowInfo,
    args: &FindArgs,
    adapter: &dyn PlatformAdapter,
    request: &LocatorResolveRequest,
    handle: crate::NativeHandle,
    path: RefPath,
    display_path: Vec<String>,
    allow_identifier_refresh: bool,
    mut stats: LookupStats,
) -> Result<WarmAttempt, AppError> {
    let live = match adapter.get_live_element(&handle, request.deadline) {
        Ok(live) => live,
        Err(error) if stale_or_unsupported(&error) => {
            return Ok(WarmAttempt::Miss {
                status: "cache_miss",
                reason: "live_revalidation_unavailable",
                stats,
            });
        }
        Err(error) => return Err(error.into()),
    };
    stats.provider_reads += 1;
    if !profile
        .selector()
        .matches_live(&live, allow_identifier_refresh)
    {
        return Ok(WarmAttempt::Miss {
            status: "invalidated",
            reason: "live_identity_mismatch",
            stats,
        });
    }
    let Some(entry) = app_profile_cache::entry_from_live(window, args.surface, &live, path) else {
        return Ok(WarmAttempt::Miss {
            status: "invalidated",
            reason: "live_generation_unusable",
            stats,
        });
    };
    let Some(refreshed) = profile.refreshed(window, &entry, Some(&display_path)) else {
        return Ok(WarmAttempt::Miss {
            status: "invalidated",
            reason: "refreshed_selector_invalid",
            stats,
        });
    };
    let resolution = resolution_from_live(entry, live, display_path)?;
    Ok(WarmAttempt::Resolved(WarmResolved {
        resolution,
        refreshed,
        stats,
    }))
}

fn resolution_from_live(
    entry: crate::RefEntry,
    live: crate::LiveElement,
    display_path: Vec<String>,
) -> Result<LocatorResolution, AppError> {
    let mut refmap = RefMap::new();
    let ref_id = refmap.try_allocate(entry.clone())?;
    let role = live.state.role.clone();
    let name = live
        .identity
        .name
        .known()
        .cloned()
        .or_else(|| live.state.value.clone())
        .or_else(|| live.identity.description.known().cloned())
        .unwrap_or_else(|| format!("(unnamed {role})"));
    let interactive =
        crate::ref_alloc::is_ref_able_role_actions(&role, &live.available_actions);
    Ok(LocatorResolution {
        matches: vec![LocatorMatch {
            data: LocatorMatchData {
                ref_id: Some(ref_id),
                role,
                name,
                value: live.state.value,
                states: live.state.states,
                interactive,
                path: display_path,
            },
            document_order: 0,
            entry,
        }],
        refmap: Some(refmap),
        stats: LocatorStats::default(),
        meta: LocatorResolutionMeta {
            total_matches: 1,
            complete: true,
            selection_complete: true,
            truncated: false,
            roles_present: Vec::new(),
        },
    })
}

fn stale_or_unsupported(error: &AdapterError) -> bool {
    matches!(
        error.code,
        ErrorCode::StaleRef
            | ErrorCode::ElementNotFound
            | ErrorCode::PlatformNotSupported
            | ErrorCode::ActionNotSupported
    )
}

fn emit(
    context: &CommandContext,
    status: &'static str,
    reason: &'static str,
    stats: &LookupStats,
) -> Result<(), AppError> {
    context.trace_lazy("app_profile.resolve", || {
        json!({
            "status": status,
            "reason": reason,
            "stats": stats,
        })
    })
}
