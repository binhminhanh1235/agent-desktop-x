use clap::CommandFactory;
use serde_json::{Value, json};

use crate::cli::Cli;

#[derive(Debug, Clone)]
pub(super) struct ToolDescriptor {
    pub command: String,
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

pub(super) fn tools() -> Vec<ToolDescriptor> {
    let command = Cli::command();
    let mut tools: Vec<_> = command
        .get_subcommands()
        .filter(|subcommand| is_mcp_callable(subcommand.get_name()))
        .map(|subcommand| {
            let command_name = subcommand.get_name().to_string();
            ToolDescriptor {
                name: format!("desktop_{}", command_name.replace('-', "_")),
                description: subcommand
                    .get_about()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("Run agent-desktop {command_name}")),
                input_schema: input_schema(&command_name),
                command: command_name,
            }
        })
        .collect();
    tools.sort_by(|left, right| left.name.cmp(&right.name));
    tools
}

pub(super) fn command_for_tool(tool_name: &str) -> Option<String> {
    tools()
        .into_iter()
        .find(|tool| tool.name == tool_name)
        .map(|tool| tool.command)
}

pub(super) fn list_result() -> Value {
    let tools = tools()
        .into_iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "title": format!("agent-desktop {}", tool.command),
                "description": format!(
                    "{} Arguments are the same structured JSON fields accepted by the matching batch command.",
                    tool.description
                ),
                "inputSchema": tool.input_schema,
            })
        })
        .collect::<Vec<_>>();
    json!({ "tools": tools })
}

fn is_mcp_callable(name: &str) -> bool {
    !matches!(name, "batch" | "cursor-overlay")
}

fn input_schema(command: &str) -> Value {
    if command == "skills" {
        return json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "path"],
                    "description": "Skill operation. Omit for list."
                },
                "name": {
                    "type": "string",
                    "description": "Skill name or alias for action=get."
                },
                "reference": {
                    "type": "string",
                    "description": "Optional bundled reference path/name for action=get."
                },
                "full": {
                    "type": "boolean",
                    "description": "Append all references when reading a skill."
                }
            },
            "additionalProperties": false
        });
    }

    if matches!(
        command,
        "list-displays" | "clipboard-clear" | "status" | "version"
    ) {
        return json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        });
    }

    json!({
        "type": "object",
        "description": "Structured arguments matching the agent-desktop batch JSON shape for this command.",
        "additionalProperties": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_deterministic_and_contains_skills() {
        let first = tools();
        let second = tools();
        let names = first.iter().map(|tool| tool.name.as_str()).collect::<Vec<_>>();
        assert_eq!(
            names,
            second
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>()
        );
        assert!(names.contains(&"desktop_snapshot"));
        assert!(names.contains(&"desktop_skills"));
        assert!(!names.contains(&"desktop_batch"));
        assert!(!names.contains(&"desktop_cursor_overlay"));
    }

    #[test]
    fn tool_name_round_trips_to_cli_command() {
        assert_eq!(
            command_for_tool("desktop_list_windows").as_deref(),
            Some("list-windows")
        );
    }
}
