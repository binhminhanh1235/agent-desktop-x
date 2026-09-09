use super::{
    FindArgs,
    profile_diagnostics::{LookupStats, emit},
    profile_revalidate::{WarmAttempt, WarmContext},
};
use crate::{
    AppError, LocatorQuery, WindowInfo,
    adapter::PlatformAdapter,
    app_profile_cache::{self, AppProfile, AppProfileKey, CacheLookup},
    context::CommandContext,
    live_locator::{LocatorResolution, LocatorResolveRequest, ObservationRoot, resolve_query},
};

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
        CacheLookup::Hit(profile) => resolve_hit(
            key,
            *profile,
            ResolveHitContext {
                args,
                query,
                adapter,
                context,
                window,
                request,
            },
        ),
    }
}

struct ResolveHitContext<'a> {
    args: &'a FindArgs,
    query: &'a LocatorQuery,
    adapter: &'a dyn PlatformAdapter,
    context: &'a CommandContext,
    window: &'a WindowInfo,
    request: &'a LocatorResolveRequest,
}

fn resolve_hit(
    key: AppProfileKey,
    profile: AppProfile,
    hit: ResolveHitContext<'_>,
) -> Result<LocatorResolution, AppError> {
    if !profile.window_matches(hit.window, hit.args.surface) {
        app_profile_cache::invalidate(&key)?;
        emit(
            hit.context,
            "invalidated",
            "window_signature_changed",
            &LookupStats::default(),
        )?;
        let resolution = cold(hit.query, hit.adapter, hit.window, hit.request)?;
        learn_after_cold(
            &key,
            hit.window,
            hit.args,
            &resolution,
            hit.context,
            "revalidated",
            "cold_after_window_change",
        )?;
        return Ok(resolution);
    }

    let same_generation = profile.live_generation_matches(hit.window);
    emit(
        hit.context,
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
            hit.context,
            "invalidated",
            "live_generation_changed",
            &LookupStats::default(),
        )?;
    }

    let warm_context = WarmContext {
        profile: &profile,
        window: hit.window,
        args: hit.args,
        adapter: hit.adapter,
        request: hit.request,
    };
    match warm_context.resolve(same_generation)? {
        WarmAttempt::Resolved(warm) => {
            let warm = *warm;
            app_profile_cache::store(key, warm.refreshed)?;
            emit(
                hit.context,
                "revalidated",
                "warm_semantic_target",
                &warm.stats,
            )?;
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
            emit(hit.context, status, reason, &stats)?;
            let resolution = cold(hit.query, hit.adapter, hit.window, hit.request)?;
            learn_after_cold(
                &key,
                hit.window,
                hit.args,
                &resolution,
                hit.context,
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
