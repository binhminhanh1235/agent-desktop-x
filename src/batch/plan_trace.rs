use agent_desktop_core::DeliverySemantics;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct PlanTraceEntry {
    index: usize,
    command: String,
    outcome: &'static str,
    elapsed_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    phase: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disposition: Option<DeliverySemantics>,
}

impl PlanTraceEntry {
    pub(super) fn completed(
        index: usize,
        command: &str,
        ok: bool,
        elapsed_ms: u128,
        phase: Option<&'static str>,
        disposition: Option<DeliverySemantics>,
    ) -> Self {
        Self {
            index,
            command: command.into(),
            outcome: if ok { "succeeded" } else { "failed" },
            elapsed_ms,
            phase,
            disposition,
        }
    }

    pub(super) fn not_started(
        index: usize,
        command: &str,
        elapsed_ms: u128,
        phase: &'static str,
    ) -> Self {
        Self {
            index,
            command: command.into(),
            outcome: "not_started",
            elapsed_ms,
            phase: Some(phase),
            disposition: Some(DeliverySemantics::not_delivered()),
        }
    }
}
