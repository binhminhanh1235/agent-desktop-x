use super::*;
use serde_json::json;

use super::*;

    #[test]
    fn modern_detection_uses_protocol_meta() {
        assert!(request_is_modern(&json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28"
            }
        })));
    }

    #[test]
    fn legacy_initialize_negotiates_latest_legacy_for_unknown_version() {
        let result = initialize_result(&json!({ "protocolVersion": "2099-01-01" }));
        assert_eq!(result["protocolVersion"], "2025-11-25");
    }

    #[test]
    fn discover_advertises_modern_tools_resources_and_prompts() {
        let response = handle(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "server/discover",
                "params": {
                    "_meta": {
                        "io.modelcontextprotocol/protocolVersion": "2026-07-28"
                    }
                }
            }),
            &crate::test_noop_ops::NoopAdapter,
            false,
        )
        .expect("discover response");

        assert_eq!(response["result"]["supportedVersions"][0], "2026-07-28");
        assert!(response["result"]["capabilities"]["tools"].is_object());
        assert!(response["result"]["capabilities"]["resources"].is_object());
        assert!(response["result"]["capabilities"]["prompts"].is_object());
        assert_eq!(response["result"]["resultType"], "complete");
    }

    #[test]
    fn tools_list_and_skills_call_work_over_wire() {
        let list = handle(
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            }),
            &crate::test_noop_ops::NoopAdapter,
            false,
        )
        .expect("tools/list response");
        let tools = list["result"]["tools"].as_array().expect("tools array");
        assert!(
            tools
                .iter()
                .any(|tool| tool["name"].as_str() == Some("desktop_snapshot"))
        );
        assert!(
            tools
                .iter()
                .any(|tool| tool["name"].as_str() == Some("desktop_skills"))
        );

        let skill = handle(
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "desktop_skills",
                    "arguments": {
                        "action": "get",
                        "name": "agent-desktop"
                    }
                }
            }),
            &crate::test_noop_ops::NoopAdapter,
            false,
        )
        .expect("skills tool response");
        assert_eq!(skill["result"]["isError"], false);
        assert!(
            skill["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("agent-desktop"))
        );
    }

    #[test]
    fn skill_resource_is_readable_over_wire() {
        let response = handle(
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "resources/read",
                "params": {
                    "uri": "agent-desktop://skills/agent-desktop"
                }
            }),
            &crate::test_noop_ops::NoopAdapter,
            false,
        )
        .expect("resources/read response");

        assert!(
            response["result"]["contents"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("# agent-desktop"))
        );
    }
