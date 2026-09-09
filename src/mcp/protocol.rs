use agent_desktop_core::{AppError, ErrorPayload, PlatformAdapter};
use serde_json::{Map, Value, json};

const MODERN_PROTOCOL: &str = "2026-07-28";
const LEGACY_PROTOCOLS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

pub(super) fn parse_error() -> Value {
    error_response(Value::Null, -32700, "Parse error", None)
}

pub(super) fn invalid_request() -> Value {
    error_response(Value::Null, -32600, "Invalid Request", None)
}

pub(super) fn handle(request: Value, adapter: &dyn PlatformAdapter, headed: bool) -> Option<Value> {
    let object = match request.as_object() {
        Some(object) => object,
        None => return Some(invalid_request()),
    };
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Some(invalid_request());
    }

    let id = object.get("id").cloned();
    let method = match object.get("method").and_then(Value::as_str) {
        Some(method) => method,
        None => return id.map(|id| error_response(id, -32600, "Invalid Request", None)),
    };
    let params = object.get("params").cloned().unwrap_or_else(|| json!({}));
    let modern = method == "server/discover" || request_is_modern(&params);

    let result = match method {
        "server/discover" => Ok(discover_result()),
        "initialize" => Ok(initialize_result(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(cacheable(super::catalog::list_result(), modern)),
        "tools/call" => call_tool(&params, adapter, headed),
        "resources/list" => super::skills::list_resources()
            .map(|value| cacheable(value, modern))
            .map_err(app_error),
        "resources/templates/list" => Ok(cacheable(json!({ "resourceTemplates": [] }), modern)),
        "resources/read" => read_resource(&params, modern),
        "prompts/list" => Ok(cacheable(super::skills::list_prompts(), modern)),
        "prompts/get" => super::skills::get_prompt(&params).map_err(app_error),
        "notifications/initialized" => return None,
        _ => {
            return id.map(|id| {
                error_response(
                    id,
                    -32601,
                    "Method not found",
                    Some(json!({ "method": method })),
                )
            });
        }
    };

    let id = id?;
    Some(match result {
        Ok(result) => success_response(id, result, modern),
        Err(error) => error_response(id, error.code, &error.message, error.data),
    })
}

fn call_tool(
    params: &Value,
    adapter: &dyn PlatformAdapter,
    headed: bool,
) -> Result<Value, ProtocolError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| ProtocolError::invalid_params("tools/call requires string 'name'"))?;
    if super::catalog::command_for_tool(name).is_none() {
        return Err(ProtocolError::invalid_params(format!(
            "Unknown MCP tool '{name}'"
        )));
    }
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !arguments.is_object() && !arguments.is_null() {
        return Err(ProtocolError::invalid_params(
            "tools/call arguments must be an object",
        ));
    }

    match super::invoke_tool(name, arguments, adapter, headed) {
        Ok(value) => Ok(tool_result(value, false)),
        Err(error) => Ok(tool_error_result(error)),
    }
}

fn read_resource(params: &Value, modern: bool) -> Result<Value, ProtocolError> {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| ProtocolError::invalid_params("resources/read requires string 'uri'"))?;
    super::skills::read_resource(uri)
        .map(|value| cacheable(value, modern))
        .map_err(app_error)
}

fn tool_result(value: Value, is_error: bool) -> Value {
    let text = serde_json::to_string(&value).unwrap_or_else(|_| value.to_string());
    let structured = if value.is_object() {
        value
    } else {
        json!({ "value": value })
    };
    json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "structuredContent": structured,
        "isError": is_error
    })
}

fn tool_error_result(error: AppError) -> Value {
    let payload = ErrorPayload::from_app_error(&error);
    let payload = serde_json::to_value(payload).unwrap_or_else(|_| {
        json!({
            "code": error.code(),
            "message": error.to_string()
        })
    });
    let text = serde_json::to_string(&payload).unwrap_or_else(|_| error.to_string());
    json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "structuredContent": {
            "error": payload
        },
        "isError": true
    })
}

fn initialize_result(params: &Value) -> Value {
    let requested = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or("2025-11-25");
    let protocol_version = if LEGACY_PROTOCOLS.contains(&requested) {
        requested
    } else {
        "2025-11-25"
    };
    json!({
        "protocolVersion": protocol_version,
        "capabilities": legacy_capabilities(),
        "serverInfo": server_info(),
        "instructions": instructions()
    })
}

fn discover_result() -> Value {
    let mut result = json!({
        "supportedVersions": [MODERN_PROTOCOL],
        "capabilities": modern_capabilities(),
        "instructions": instructions(),
        "ttlMs": 3_600_000,
        "cacheScope": "public"
    });
    attach_modern_metadata(&mut result);
    result
}

fn legacy_capabilities() -> Value {
    json!({
        "tools": { "listChanged": false },
        "resources": { "subscribe": false, "listChanged": false },
        "prompts": { "listChanged": false }
    })
}

fn modern_capabilities() -> Value {
    json!({
        "tools": {},
        "resources": {},
        "prompts": {}
    })
}

fn instructions() -> &'static str {
    "Native Rust desktop automation. Read agent-desktop://skills/agent-desktop before complex workflows. Tool names mirror CLI commands with a desktop_ prefix; arguments use the same structured JSON object as batch commands."
}

fn server_info() -> Value {
    json!({
        "name": "agent-desktop",
        "version": env!("CARGO_PKG_VERSION")
    })
}

fn request_is_modern(params: &Value) -> bool {
    params
        .get("_meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("io.modelcontextprotocol/protocolVersion"))
        .and_then(Value::as_str)
        == Some(MODERN_PROTOCOL)
}

fn cacheable(mut result: Value, modern: bool) -> Value {
    if modern {
        if let Some(object) = result.as_object_mut() {
            object.insert("ttlMs".into(), json!(3_600_000));
            object.insert("cacheScope".into(), json!("public"));
        }
    }
    result
}

fn success_response(id: Value, mut result: Value, modern: bool) -> Value {
    if modern {
        if let Some(object) = result.as_object_mut() {
            object
                .entry("resultType".to_string())
                .or_insert_with(|| json!("complete"));
        }
        attach_modern_metadata(&mut result);
    }
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn attach_modern_metadata(result: &mut Value) {
    let Some(object) = result.as_object_mut() else {
        return;
    };
    let meta = object
        .entry("_meta".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(meta) = meta.as_object_mut() {
        meta.insert("io.modelcontextprotocol/serverInfo".into(), server_info());
    }
}

fn error_response(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut error = json!({
        "code": code,
        "message": message
    });
    if let (Some(object), Some(data)) = (error.as_object_mut(), data) {
        object.insert("data".into(), data);
    }
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error
    })
}

fn app_error(error: AppError) -> ProtocolError {
    ProtocolError {
        code: -32602,
        message: error.to_string(),
        data: Some(json!({
            "code": error.code(),
            "suggestion": error.suggestion()
        })),
    }
}

struct ProtocolError {
    code: i64,
    message: String,
    data: Option<Value>,
}

impl ProtocolError {
    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
