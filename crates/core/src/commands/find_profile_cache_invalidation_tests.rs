use super::{
    execute,
    profile_cache_test_support::{ProfileCacheAdapter, TEST_LOCK, cache_key, find_args},
};
use crate::{
    ProcessId,
    app_profile_cache::{self, CacheLookup},
    context::CommandContext,
    refs_test_support::HomeGuard,
    runtime_events::{
        RUNTIME_EVENT_CAPACITY, RuntimeEvent, RuntimeProcessScope, RuntimeWindowScope,
        publish_runtime_event,
    },
};

#[test]
fn window_generation_event_invalidates_live_ref_and_preserves_semantics() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();
    execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let key = cache_key(&find_args());
    let current_window =
        ProfileCacheAdapter::window(&adapter.state.lock().expect("fixture lock"));

    let process = RuntimeProcessScope::new("ProfileApp", ProcessId::new(410), "proc-a")
        .expect("bounded process scope");
    let previous = RuntimeWindowScope::new(process.clone(), "Profile Fixture")
        .expect("bounded window scope");
    let current = RuntimeWindowScope::new(process, "Profile Fixture")
        .expect("bounded window scope");
    publish_runtime_event(RuntimeEvent::WindowGenerationChanged { previous, current });

    let profile = match app_profile_cache::lookup(&key).expect("profile after window event") {
        CacheLookup::Hit(profile) => profile,
        other => panic!("semantic profile must survive, got {other:?}"),
    };
    assert!(!profile.live_generation_matches(&current_window));
    assert!(profile.selector().has_stable_identifier());
    assert!(profile.selector().has_semantic_path());
    let metrics = app_profile_cache::invalidation_metrics_for_tests();
    assert_eq!(metrics.0, 1);
    assert_eq!(metrics.1, 1);
}

#[test]
fn dropped_events_force_conservative_live_ref_invalidation() {
    let _serial = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let _guard = HomeGuard::new();
    app_profile_cache::clear_for_tests();
    let adapter = ProfileCacheAdapter::new();
    execute(find_args(), &adapter, &CommandContext::default()).expect("cold find");
    let key = cache_key(&find_args());
    let current_window =
        ProfileCacheAdapter::window(&adapter.state.lock().expect("fixture lock"));

    let before = match app_profile_cache::lookup(&key).expect("profile before overflow") {
        CacheLookup::Hit(profile) => profile,
        other => panic!("expected profile, got {other:?}"),
    };
    assert!(before.live_generation_matches(&current_window));

    for _ in 0..=RUNTIME_EVENT_CAPACITY {
        publish_runtime_event(RuntimeEvent::ProviderReset);
    }

    let after = match app_profile_cache::lookup(&key).expect("profile after overflow") {
        CacheLookup::Hit(profile) => profile,
        other => panic!("semantic profile must survive overflow, got {other:?}"),
    };
    assert!(!after.live_generation_matches(&current_window));
    assert!(after.selector().has_stable_identifier());
    assert!(after.selector().has_semantic_path());
    let metrics = app_profile_cache::invalidation_metrics_for_tests();
    assert_eq!(metrics.1, 1);
    assert_eq!(metrics.3, 1);
}
