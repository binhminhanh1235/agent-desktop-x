use super::{RuntimeEvent, RuntimeProcessScope, RuntimeWindowScope};
use crate::{AppInfo, SignalBaseline, WindowInfo, search_text};
use std::collections::BTreeMap;

pub(super) fn runtime_invalidation_diff(
    previous: &SignalBaseline,
    current: &SignalBaseline,
) -> Vec<RuntimeEvent> {
    let mut events = Vec::new();
    if previous.completeness.apps && current.completeness.apps {
        diff_apps(previous, current, &mut events);
    }
    if previous.completeness.windows && current.completeness.windows {
        diff_windows(previous, current, &mut events);
    }
    events.sort_by_key(sort_key);
    events.dedup();
    events
}

fn diff_apps(previous: &SignalBaseline, current: &SignalBaseline, events: &mut Vec<RuntimeEvent>) {
    let previous_groups = group_apps(&previous.apps);
    let current_groups = group_apps(&current.apps);
    let mut keys = previous_groups
        .keys()
        .chain(current_groups.keys())
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();

    for key in keys {
        let old = previous_groups.get(&key).cloned().unwrap_or_default();
        let new = current_groups.get(&key).cloned().unwrap_or_default();
        if old.len() == 1 && new.len() == 1 {
            let old_app = &previous.apps[old[0]];
            let new_app = &current.apps[new[0]];
            if app_runtime_identity(old_app) != app_runtime_identity(new_app) {
                if let (Some(previous), Some(current)) =
                    (process_scope(old_app), process_scope(new_app))
                {
                    events.push(RuntimeEvent::ProcessReplaced { previous, current });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
            continue;
        }

        for index in old {
            let old_app = &previous.apps[index];
            if !new.iter().any(|candidate| {
                app_runtime_identity(old_app) == app_runtime_identity(&current.apps[*candidate])
            }) {
                if let Some(previous) = process_scope(old_app) {
                    events.push(RuntimeEvent::ProcessExited { previous });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
        }
        for index in new {
            let new_app = &current.apps[index];
            if !previous_groups
                .get(&key)
                .into_iter()
                .flatten()
                .any(|candidate| {
                    app_runtime_identity(new_app)
                        == app_runtime_identity(&previous.apps[*candidate])
                })
            {
                if let Some(current) = process_scope(new_app) {
                    events.push(RuntimeEvent::ProcessStarted { current });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
        }
    }
}

fn diff_windows(
    previous: &SignalBaseline,
    current: &SignalBaseline,
    events: &mut Vec<RuntimeEvent>,
) {
    let previous_groups = group_windows(&previous.windows);
    let current_groups = group_windows(&current.windows);
    let mut keys = previous_groups
        .keys()
        .chain(current_groups.keys())
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();

    for key in keys {
        let old = previous_groups.get(&key).cloned().unwrap_or_default();
        let new = current_groups.get(&key).cloned().unwrap_or_default();
        if old.len() == 1 && new.len() == 1 {
            let old_window = &previous.windows[old[0]];
            let new_window = &current.windows[new[0]];
            if window_runtime_identity(old_window) != window_runtime_identity(new_window) {
                if let (Some(previous), Some(current)) =
                    (window_scope(old_window), window_scope(new_window))
                {
                    events.push(RuntimeEvent::WindowGenerationChanged { previous, current });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
            continue;
        }

        for index in old {
            let old_window = &previous.windows[index];
            if !new.iter().any(|candidate| {
                window_runtime_identity(old_window)
                    == window_runtime_identity(&current.windows[*candidate])
            }) {
                if let Some(previous) = window_scope(old_window) {
                    events.push(RuntimeEvent::WindowDestroyed { previous });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
        }
        for index in new {
            let new_window = &current.windows[index];
            if !previous_groups
                .get(&key)
                .into_iter()
                .flatten()
                .any(|candidate| {
                    window_runtime_identity(new_window)
                        == window_runtime_identity(&previous.windows[*candidate])
                })
            {
                if let Some(current) = window_scope(new_window) {
                    events.push(RuntimeEvent::WindowCreated { current });
                } else {
                    events.push(RuntimeEvent::ProviderReset);
                }
            }
        }
    }
}

fn group_apps(apps: &[AppInfo]) -> BTreeMap<String, Vec<usize>> {
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (index, app) in apps.iter().enumerate() {
        groups.entry(app_semantic_key(app)).or_default().push(index);
    }
    groups
}

fn group_windows(windows: &[WindowInfo]) -> BTreeMap<String, Vec<usize>> {
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (index, window) in windows.iter().enumerate() {
        groups
            .entry(format!(
                "{}\u{0}{}",
                search_text::normalize(&window.app),
                search_text::normalize(&window.title)
            ))
            .or_default()
            .push(index);
    }
    groups
}

fn app_semantic_key(app: &AppInfo) -> String {
    app.bundle_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| format!("bundle:{}", search_text::normalize(value)))
        .unwrap_or_else(|| format!("name:{}", search_text::normalize(&app.name)))
}

fn app_runtime_identity(app: &AppInfo) -> Option<(u32, &str)> {
    app.process_instance
        .as_deref()
        .map(|instance| (app.pid.get(), instance))
}

fn window_runtime_identity(window: &WindowInfo) -> Option<(u32, &str, &str)> {
    window
        .process_instance
        .as_deref()
        .map(|instance| (window.pid.get(), instance, window.id.as_str()))
}

fn process_scope(app: &AppInfo) -> Option<RuntimeProcessScope> {
    RuntimeProcessScope::new(
        app.name.clone(),
        app.pid,
        app.process_instance.as_deref()?.to_string(),
    )
}

fn window_scope(window: &WindowInfo) -> Option<RuntimeWindowScope> {
    let process = RuntimeProcessScope::new(
        window.app.clone(),
        window.pid,
        window.process_instance.as_deref()?.to_string(),
    )?;
    RuntimeWindowScope::new(process, window.title.clone())
}

fn sort_key(event: &RuntimeEvent) -> (u8, String) {
    match event {
        RuntimeEvent::ProcessExited { previous } => (0, process_key(previous)),
        RuntimeEvent::ProcessReplaced { previous, .. } => (1, process_key(previous)),
        RuntimeEvent::WindowDestroyed { previous } => (2, window_key(previous)),
        RuntimeEvent::WindowGenerationChanged { previous, .. } => (3, window_key(previous)),
        RuntimeEvent::AccessibilityTreeInvalidated { process } => (4, process_key(process)),
        RuntimeEvent::WindowCreated { current } => (5, window_key(current)),
        RuntimeEvent::ProcessStarted { current } => (6, process_key(current)),
        RuntimeEvent::ProviderReset => (7, String::new()),
    }
}

fn process_key(scope: &RuntimeProcessScope) -> String {
    format!(
        "{}|{}|{}",
        search_text::normalize(scope.app()),
        scope.pid().get(),
        scope.process_instance()
    )
}

fn window_key(scope: &RuntimeWindowScope) -> String {
    format!(
        "{}|{}",
        process_key(scope.process()),
        search_text::normalize(scope.title())
    )
}
