use crate::{AppError, context::CommandContext, live_locator::LocatorStats};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, Default, Serialize)]
pub(super) struct LookupStats {
    pub(super) resolution_calls: u64,
    pub(super) provider_reads: u64,
    pub(super) tree_reads: u64,
    pub(super) cold_lookups: u64,
    pub(super) warm_lookups: u64,
}

impl LookupStats {
    pub(super) fn add_locator(&mut self, stats: &LocatorStats) {
        self.tree_reads += stats.reads.counts.observation_attempts.max(1);
        self.provider_reads += stats.reads.counts.attributes_requested
            + stats.reads.counts.child_reads
            + stats.reads.counts.action_reads
            + stats.reads.counts.fallback_reads
            + stats.semantic_reads.child_label_reads
            + stats.semantic_reads.promotion_reads
            + stats.semantic_reads.settable_reads;
    }
}

pub(super) fn emit(
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
