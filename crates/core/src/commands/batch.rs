use crate::AppError;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchAssertion {
    pub command: String,
    #[serde(default)]
    pub args: Value,
    #[serde(default)]
    pub json_pointer: String,
    pub equals: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchCommand {
    pub command: String,
    pub session: Option<String>,
    #[serde(default)]
    pub args: Value,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub condition: Option<BatchAssertion>,
    #[serde(default)]
    pub verify: Option<BatchAssertion>,
}

pub fn parse_commands(json_str: &str) -> Result<Vec<BatchCommand>, AppError> {
    serde_json::from_str(json_str)
        .map_err(|e| AppError::invalid_input(format!("Invalid batch JSON: {e}")))
}
