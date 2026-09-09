use super::{
    FindArgs,
    profile_diagnostics::LookupStats,
    profile_result::{resolution_from_live, stale_or_unsupported},
};
use crate::{
    AppError, ErrorCode, LocatorQuery, WindowInfo,
    adapter::PlatformAdapter,
    app_profile_cache::{self, AppProfile},
    live_locator::{
        LocatorMaterialization, LocatorResolution, LocatorResolveRequest, LocatorSelection,
        ObservationRoot, resolve_query,
    },
    refs::RefPath,
};

const MAX_REVALIDATION_CANDIDATES: u32 = 16;

pub(super) struct WarmResolved {
    pub(super) resolution: LocatorResolution,
    pub(super) refreshed: AppProfile,
    pub(super) stats: LookupStats,
}

pub(super) enum WarmAttempt {
    Resolved(Box<WarmResolved>),
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

enum CandidateSearch {
    None,
    One(Box<FreshCandidate>),
    Ambiguous,
}

struct LiveLocation {
    path: RefPath,
    display_path: Vec<String>,
    allow_identifier_refresh: bool,
}

pub(super) struct WarmContext<'a> {
    pub(super) profile: &'a AppProfile,
    pub(super) window: &'a WindowInfo,
    pub(super) args: &'a FindArgs,
    pub(super) adapter: &'a dyn PlatformAdapter,
    pub(super) request: &'a LocatorResolveRequest,
}

impl WarmContext<'_> {
    pub(super) fn resolve(&self, same_generation: bool) -> Result<WarmAttempt, AppError> {
        let mut stats = LookupStats {
            resolution_calls: 1,
            warm_lookups: 1,
            ..LookupStats::default()
        };

        if same_generation {
            stats.resolution_calls += 1;
            match self
                .adapter
                .resolve_element_strict(self.profile.live_entry(), self.request.deadline)
            {
                Ok(handle) => {
                    stats.provider_reads += 1;
                    return self.finish_handle(handle, self.live_location(false), stats);
                }
                Err(error) if error.code == ErrorCode::AmbiguousTarget => {}
                Err(error) if stale_or_unsupported(&error) => {}
                Err(error) => return Err(error.into()),
            }
        }

        let selector = self.profile.selector();
        if selector.has_stable_identifier()
            && let Some(entry) = self.profile.semantic_entry(self.window)
        {
            stats.resolution_calls += 1;
            match self
                .adapter
                .resolve_element_strict(&entry, self.request.deadline)
            {
                Ok(handle) => {
                    stats.provider_reads += 1;
                    return self.finish_handle(handle, self.live_location(false), stats);
                }
                Err(error) if error.code == ErrorCode::AmbiguousTarget => {
                    if selector.has_semantic_path() {
                        return match self.semantic_path_candidate(false, &mut stats)? {
                            CandidateSearch::One(candidate) => {
                                self.finish_candidate(candidate, true, stats)
                            }
                            CandidateSearch::Ambiguous => Ok(WarmAttempt::Miss {
                                status: "ambiguous",
                                reason: "stable_identifier_and_path_not_unique",
                                stats,
                            }),
                            CandidateSearch::None => Ok(WarmAttempt::Miss {
                                status: "ambiguous",
                                reason: "stable_identifier_not_unique",
                                stats,
                            }),
                        };
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

        if !selector.has_stable_identifier() && selector.has_semantic_path() {
            match self.semantic_path_candidate(true, &mut stats)? {
                CandidateSearch::One(candidate) => {
                    return self.finish_candidate(candidate, false, stats);
                }
                CandidateSearch::Ambiguous => {
                    return Ok(WarmAttempt::Miss {
                        status: "ambiguous",
                        reason: "semantic_path_not_unique",
                        stats,
                    });
                }
                CandidateSearch::None => {}
            }
        } else if !selector.has_stable_identifier()
            && let Some(entry) = self.profile.semantic_entry(self.window)
        {
            stats.resolution_calls += 1;
            match self
                .adapter
                .resolve_element_strict(&entry, self.request.deadline)
            {
                Ok(handle) => {
                    stats.provider_reads += 1;
                    return self.finish_handle(handle, self.live_location(false), stats);
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

        match self.semantic_path_candidate(false, &mut stats)? {
            CandidateSearch::One(candidate) => {
                return self.finish_candidate(candidate, true, stats);
            }
            CandidateSearch::Ambiguous => {
                return Ok(WarmAttempt::Miss {
                    status: "ambiguous",
                    reason: "semantic_path_not_unique",
                    stats,
                });
            }
            CandidateSearch::None => {}
        }

        match self.fuzzy_candidate(&mut stats)? {
            CandidateSearch::One(candidate) => self.finish_candidate(candidate, true, stats),
            CandidateSearch::Ambiguous => Ok(WarmAttempt::Miss {
                status: "ambiguous",
                reason: "bounded_fuzzy_not_unique",
                stats,
            }),
            CandidateSearch::None => Ok(WarmAttempt::Miss {
                status: "cache_miss",
                reason: "semantic_target_stale",
                stats,
            }),
        }
    }

    fn live_location(&self, allow_identifier_refresh: bool) -> LiveLocation {
        LiveLocation {
            path: self.profile.live_path(),
            display_path: self.profile.selector().display_path().to_vec(),
            allow_identifier_refresh,
        }
    }

    fn semantic_path_candidate(
        &self,
        include_identifier: bool,
        stats: &mut LookupStats,
    ) -> Result<CandidateSearch, AppError> {
        if !self.profile.selector().has_semantic_path() {
            return Ok(CandidateSearch::None);
        }
        let query = self.profile.selector().exact_query(include_identifier);
        stats.resolution_calls += 1;
        let resolution = self.scoped_scan(&query)?;
        stats.add_locator(&resolution.stats);
        if !resolution.meta.complete || resolution.meta.truncated {
            return Ok(CandidateSearch::None);
        }
        let candidates = resolution
            .matches
            .into_iter()
            .filter(|candidate| self.profile.selector().path_matches(&candidate.data.path))
            .map(|candidate| FreshCandidate {
                entry: candidate.entry,
                display_path: candidate.data.path,
            })
            .collect::<Vec<_>>();
        Ok(classify_candidates(candidates))
    }

    fn fuzzy_candidate(&self, stats: &mut LookupStats) -> Result<CandidateSearch, AppError> {
        let query = self.profile.selector().role_query();
        stats.resolution_calls += 1;
        let resolution = self.scoped_scan(&query)?;
        stats.add_locator(&resolution.stats);
        if !resolution.meta.complete || resolution.meta.truncated {
            return Ok(CandidateSearch::None);
        }
        let candidates = resolution
            .matches
            .into_iter()
            .filter(|candidate| {
                (!self.profile.selector().has_semantic_path()
                    || self.profile.selector().path_matches(&candidate.data.path))
                    && self
                        .profile
                        .selector()
                        .fuzzy_name_matches(&candidate.data.name)
            })
            .map(|candidate| FreshCandidate {
                entry: candidate.entry,
                display_path: candidate.data.path,
            })
            .collect::<Vec<_>>();
        Ok(classify_candidates(candidates))
    }

    fn scoped_scan(&self, query: &LocatorQuery) -> Result<LocatorResolution, AppError> {
        let scan_request = LocatorResolveRequest {
            selection: LocatorSelection::All {
                limit: Some(MAX_REVALIDATION_CANDIDATES),
            },
            materialization: LocatorMaterialization::None,
            ..*self.request
        };
        resolve_query(
            self.adapter,
            query,
            ObservationRoot::Window(self.window),
            &scan_request,
        )
    }

    fn finish_candidate(
        &self,
        candidate: Box<FreshCandidate>,
        allow_identifier_refresh: bool,
        mut stats: LookupStats,
    ) -> Result<WarmAttempt, AppError> {
        let candidate = *candidate;
        let handle = match self
            .adapter
            .resolve_locator_anchor(&candidate.entry, self.request.deadline)
        {
            Ok(handle) => handle,
            Err(error) if stale_or_unsupported(&error) => {
                return Ok(WarmAttempt::Miss {
                    status: "cache_miss",
                    reason: "fresh_anchor_changed",
                    stats,
                });
            }
            Err(error) if error.code == ErrorCode::AmbiguousTarget => {
                return Ok(WarmAttempt::Miss {
                    status: "ambiguous",
                    reason: "fresh_anchor_ambiguous",
                    stats,
                });
            }
            Err(error) => return Err(error.into()),
        };
        stats.provider_reads += 1;
        self.finish_handle(
            handle,
            LiveLocation {
                path: candidate.entry.scope.path,
                display_path: candidate.display_path,
                allow_identifier_refresh,
            },
            stats,
        )
    }

    fn finish_handle(
        &self,
        handle: crate::NativeHandle,
        location: LiveLocation,
        mut stats: LookupStats,
    ) -> Result<WarmAttempt, AppError> {
        let live = match self
            .adapter
            .get_live_element(&handle, self.request.deadline)
        {
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
        if !self
            .profile
            .selector()
            .matches_live(&live, location.allow_identifier_refresh)
        {
            return Ok(WarmAttempt::Miss {
                status: "invalidated",
                reason: "live_identity_mismatch",
                stats,
            });
        }
        let Some(entry) = app_profile_cache::entry_from_live(
            self.window,
            self.args.surface,
            &live,
            location.path,
        ) else {
            return Ok(WarmAttempt::Miss {
                status: "invalidated",
                reason: "live_generation_unusable",
                stats,
            });
        };
        let Some(refreshed) =
            self.profile
                .refreshed(self.window, &entry, Some(&location.display_path))
        else {
            return Ok(WarmAttempt::Miss {
                status: "invalidated",
                reason: "refreshed_selector_invalid",
                stats,
            });
        };
        let resolution = resolution_from_live(entry, live, location.display_path)?;
        Ok(WarmAttempt::Resolved(Box::new(WarmResolved {
            resolution,
            refreshed,
            stats,
        })))
    }
}

fn classify_candidates(mut candidates: Vec<FreshCandidate>) -> CandidateSearch {
    match candidates.len() {
        0 => CandidateSearch::None,
        1 => {
            if let Some(candidate) = candidates.pop() {
                CandidateSearch::One(Box::new(candidate))
            } else {
                CandidateSearch::None
            }
        }
        _ => CandidateSearch::Ambiguous,
    }
}
