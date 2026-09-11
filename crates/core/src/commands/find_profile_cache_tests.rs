use super::{
    execute,
    profile_cache_test_support::{ProfileCacheAdapter, TEST_LOCK, cache_key, find_args},
};
use crate::{
    ProcessId,
    app_profile_cache::{self, CacheLookup},
    context::CommandContext,
    refs_test_support::HomeGuard,
    runtime_events::{RuntimeEvent, RuntimeProcessScope, publish_runtime_event},
};

#[test]
fn first_lookup_learns_semantics_and_second_lookup_avoids_tree_walk() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();
    let request_args = find_args();

    let first = execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let after_cold = adapter.counts();
    assert_eq!(first["matches"].as_array().expect("matches").len(), 1);
    assert!(after_cold.0 >= 1);

    match app_profile_cache::lookup(&cache_key(&request_args)).expect("cache lookup") {
        CacheLookup::Hit(profile) => {
            assert!(profile.selector().has_stable_identifier());
            assert!(profile.selector().has_semantic_path());
        }
        other => panic!("expected learned AppProfile, got {other:?}"),
    }

    let second = execute(find_args(), &adapter, &CommandContext::default()).expect("warm find");
    let after_warm = adapter.counts();
    assert_eq!(second["matches"].as_array().expect("matches").len(), 1);
    assert_eq!(
        after_warm.0, after_cold.0,
        "warm semantic lookup must avoid a full tree observation"
    );
    assert!(after_warm.1 > after_cold.1);
    assert!(after_warm.2 > after_cold.2);
}

#[test]
fn bounds_motion_does_not_break_semantic_identity() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();

    execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let before = adapter.counts();
    adapter.mutate(|state| state.bounds_x = 640.0);

    let response = execute(find_args(), &adapter, &CommandContext::default()).expect("warm find");
    assert_eq!(response["matches"].as_array().expect("matches").len(), 1);
    assert_eq!(adapter.counts().0, before.0);
}

#[test]
fn process_and_window_recreation_invalidates_live_generation_but_semantics_reresolve() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();

    execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let key = cache_key(&find_args());
    let before_profile = match app_profile_cache::lookup(&key).expect("cache lookup") {
        CacheLookup::Hit(profile) => profile,
        _ => panic!("profile must exist"),
    };
    adapter.mutate(|state| {
        state.pid = 411;
        state.process_instance = "proc-b".into();
        state.window_id = "w-profile-2".into();
    });
    let current_window = ProfileCacheAdapter::window(&adapter.state.lock().expect("fixture lock"));
    assert!(!before_profile.live_generation_matches(&current_window));

    let before = adapter.counts();
    let response = execute(find_args(), &adapter, &CommandContext::default()).expect("warm find");
    assert_eq!(response["matches"].as_array().expect("matches").len(), 1);
    assert_eq!(
        adapter.counts().0,
        before.0,
        "restart should reuse semantics without replaying the cold tree walk"
    );
}

#[test]
fn runtime_invalidation_stales_live_ref_but_preserves_semantic_profile() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();
    execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let key = cache_key(&find_args());
    let current_window = ProfileCacheAdapter::window(&adapter.state.lock().expect("fixture lock"));

    let before = match app_profile_cache::lookup(&key).expect("profile before event") {
        CacheLookup::Hit(profile) => profile,
        other => panic!("expected profile, got {other:?}"),
    };
    assert!(before.live_generation_matches(&current_window));

    let process = RuntimeProcessScope::new("ProfileApp", ProcessId::new(410), "proc-a")
        .expect("bounded process scope");
    publish_runtime_event(RuntimeEvent::AccessibilityTreeInvalidated { process });

    let after = match app_profile_cache::lookup(&key).expect("profile after event") {
        CacheLookup::Hit(profile) => profile,
        other => panic!("semantic profile must survive, got {other:?}"),
    };
    assert!(!after.live_generation_matches(&current_window));
    assert!(after.selector().has_stable_identifier());
    assert!(after.selector().has_semantic_path());
    assert_eq!(app_profile_cache::invalidation_metrics_for_tests().0, 1);
    assert_eq!(app_profile_cache::invalidation_metrics_for_tests().1, 1);
}

#[test]
fn unrelated_runtime_event_retains_profile_live_ref() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();
    execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let key = cache_key(&find_args());
    let current_window = ProfileCacheAdapter::window(&adapter.state.lock().expect("fixture lock"));

    let other = RuntimeProcessScope::new("Browser", ProcessId::new(900), "browser-a")
        .expect("bounded process scope");
    publish_runtime_event(RuntimeEvent::ProcessStarted { current: other });

    let profile = match app_profile_cache::lookup(&key).expect("profile after unrelated event") {
        CacheLookup::Hit(profile) => profile,
        other => panic!("expected profile, got {other:?}"),
    };
    assert!(profile.live_generation_matches(&current_window));
    let metrics = app_profile_cache::invalidation_metrics_for_tests();
    assert_eq!(metrics.0, 1);
    assert_eq!(metrics.1, 0);
    assert_eq!(metrics.2, 1);
}

#[test]
fn duplicate_and_reordered_events_never_resurrect_invalidated_live_ref() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();
    execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let key = cache_key(&find_args());
    let current_window = ProfileCacheAdapter::window(&adapter.state.lock().expect("fixture lock"));
    let process = RuntimeProcessScope::new("ProfileApp", ProcessId::new(410), "proc-a")
        .expect("bounded process scope");

    publish_runtime_event(RuntimeEvent::ProcessExited {
        previous: process.clone(),
    });
    publish_runtime_event(RuntimeEvent::ProcessStarted {
        current: process.clone(),
    });
    publish_runtime_event(RuntimeEvent::ProcessExited { previous: process });

    let profile = match app_profile_cache::lookup(&key).expect("profile after reordered events") {
        CacheLookup::Hit(profile) => profile,
        other => panic!("semantic profile must survive, got {other:?}"),
    };
    assert!(!profile.live_generation_matches(&current_window));
    let metrics = app_profile_cache::invalidation_metrics_for_tests();
    assert_eq!(metrics.0, 3);
    assert_eq!(metrics.1, 1, "duplicate invalidation must be idempotent");
}

#[test]
fn duplicate_semantic_candidates_refuse_cached_single_target() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();

    execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let before = adapter.counts();
    adapter.mutate(|state| state.duplicates = 2);

    let response =
        execute(find_args(), &adapter, &CommandContext::default()).expect("cold fallback");
    assert_eq!(response["matches"].as_array().expect("matches").len(), 2);
    assert!(
        adapter.counts().0 > before.0,
        "ambiguous warm resolution must fall back to authoritative scoped discovery"
    );
}

#[test]
fn corrupted_cache_is_never_used_as_a_target() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();
    let request_args = find_args();

    execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let before = adapter.counts();
    let key = cache_key(&request_args);
    app_profile_cache::corrupt_for_tests(&key);

    let response =
        execute(find_args(), &adapter, &CommandContext::default()).expect("cold revalidation");
    assert_eq!(response["matches"].as_array().expect("matches").len(), 1);
    assert!(
        adapter.counts().0 > before.0,
        "invalid cache data must fail closed into fresh scoped discovery"
    );
}

#[test]
fn mismatched_cached_identity_fails_closed() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();

    execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let before = adapter.counts();
    adapter.mutate(|state| state.name = "Stop".into());

    let response =
        execute(find_args(), &adapter, &CommandContext::default()).expect("cold fallback");
    assert!(response["matches"].as_array().expect("matches").is_empty());
    assert!(
        adapter.counts().0 > before.0,
        "identity mismatch must fall back to authoritative scoped discovery"
    );
}
