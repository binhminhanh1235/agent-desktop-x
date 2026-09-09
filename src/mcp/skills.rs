use agent_desktop_core::{AppError, commands::skills};
use serde_json::{Value, json};

const INDEX_URI: &str = "agent-desktop://skills";

pub(super) fn list_resources() -> Result<Value, AppError> {
    let index = skills::list()?;
    let mut resources = vec![json!({
        "uri": INDEX_URI,
        "name": "agent-desktop skills index",
        "title": "Bundled agent-desktop skills",
        "description": "Version-matched skill catalog compiled into the agent-desktop executable.",
        "mimeType": "application/json"
    })];

    if let Some(entries) = index.get("skills").and_then(Value::as_array) {
        for entry in entries {
            let Some(name) = entry.get("name").and_then(Value::as_str) else {
                continue;
            };
            resources.push(json!({
                "uri": format!("{INDEX_URI}/{name}"),
                "name": name,
                "title": format!("{name} skill"),
                "description": entry.get("summary").and_then(Value::as_str).unwrap_or("Bundled agent-desktop skill"),
                "mimeType": "text/markdown"
            }));
        }
    }

    Ok(json!({ "resources": resources }))
}

pub(super) fn read_resource(uri: &str) -> Result<Value, AppError> {
    if uri == INDEX_URI {
        return Ok(json!({
            "contents": [{
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&skills::list()?)
                    .map_err(|error| AppError::Internal(error.to_string()))?
            }]
        }));
    }

    let tail = uri
        .strip_prefix(&format!("{INDEX_URI}/"))
        .ok_or_else(|| AppError::invalid_input(format!("Unknown MCP skill resource '{uri}'")))?;
    let mut parts = tail.splitn(2, '/');
    let name = parts.next().unwrap_or_default();
    let selector = parts.next();

    let (full, reference) = match selector {
        None => (false, None),
        Some("full") => (true, None),
        Some(reference) => (false, Some(reference.to_string())),
    };
    let value = skills::get(skills::GetArgs {
        name: name.to_string(),
        full,
        reference,
    })?;
    let content = value
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::Internal("Bundled skill did not return markdown content".to_string())
        })?;

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "text/markdown",
            "text": content
        }]
    }))
}

pub(super) fn list_prompts() -> Value {
    json!({
        "prompts": [{
            "name": "agent-desktop-skill",
            "title": "Load agent-desktop skill",
            "description": "Load a version-matched bundled skill before operating the desktop.",
            "arguments": [
                {
                    "name": "name",
                    "description": "Skill name or alias (agent-desktop, windows, ffi).",
                    "required": false
                },
                {
                    "name": "full",
                    "description": "Set to true to append every bundled reference.",
                    "required": false
                }
            ]
        }]
    })
}

pub(super) fn get_prompt(params: &Value) -> Result<Value, AppError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if name != "agent-desktop-skill" {
        return Err(AppError::invalid_input(format!(
            "Unknown MCP prompt '{name}'"
        )));
    }
    let arguments = params.get("arguments").and_then(Value::as_object);
    let skill_name = arguments
        .and_then(|args| args.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("agent-desktop");
    let full = arguments
        .and_then(|args| args.get("full"))
        .and_then(Value::as_str)
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));

    let value = skills::get(skills::GetArgs {
        name: skill_name.to_string(),
        full,
        reference: None,
    })?;
    let content = value
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Internal("Bundled skill did not return markdown content".to_string()))?;

    Ok(json!({
        "description": format!("Bundled skill '{skill_name}'"),
        "messages": [{
            "role": "user",
            "content": {
                "type": "text",
                "text": content
            }
        }]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_resources_are_embedded_and_readable() {
        let listed = list_resources().expect("list resources");
        let resources = listed["resources"].as_array().expect("resources array");
        assert!(
            resources
                .iter()
                .any(|resource| resource["uri"] == "agent-desktop://skills/agent-desktop")
        );

        let read =
            read_resource("agent-desktop://skills/agent-desktop").expect("read desktop skill");
        assert!(
            read["contents"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("# agent-desktop"))
        );
    }
}
