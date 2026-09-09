use crate::{
    AppError, ErrorCode,
    live_locator::{
        LocatorMatch, LocatorMatchData, LocatorResolution, LocatorResolutionMeta, LocatorStats,
    },
    refs::RefMap,
};

pub(super) fn resolution_from_live(
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
    let interactive = crate::ref_alloc::is_ref_able_role_actions(&role, &live.available_actions);
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

pub(super) fn stale_or_unsupported(error: &crate::AdapterError) -> bool {
    matches!(
        error.code,
        ErrorCode::StaleRef
            | ErrorCode::ElementNotFound
            | ErrorCode::PlatformNotSupported
            | ErrorCode::ActionNotSupported
    )
}
