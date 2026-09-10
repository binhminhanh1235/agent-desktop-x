# Native MCP server

agent-desktop can run as an MCP server from the same native Rust executable used by the CLI.

There is no Python runtime, helper daemon, or sidecar process.

## Start

```bash
agent-desktop --mcp
```

The MCP transport uses stdin/stdout. stdout is reserved for newline-delimited JSON-RPC messages; diagnostics go to stderr.

MCP starts headless by default, matching normal CLI safety policy. To permit physical input paths that require headed mode:

```bash
agent-desktop --mcp --headed
```

You can also set `AGENT_DESKTOP_MCP_HEADED=1`.

## Host configuration

A typical MCP host configuration is:

```json
{
  "mcpServers": {
    "agent-desktop": {
      "command": "agent-desktop",
      "args": ["--mcp"]
    }
  }
}
```

For workflows that intentionally need physical pointer or keyboard input:

```json
{
  "mcpServers": {
    "agent-desktop": {
      "command": "agent-desktop",
      "args": ["--mcp", "--headed"]
    }
  }
}
```

Use the absolute executable path if the host does not inherit your shell PATH.

## Protocol compatibility

The stdio server accepts both MCP lifecycle eras:

- MCP `2026-07-28`: stateless `server/discover` and per-request metadata.
- Legacy MCP clients through `2025-11-25` and earlier supported handshake revisions: `initialize` / `notifications/initialized`.

The modern server advertises tools, resources, and prompts. Cacheable list/resource responses include conservative MCP cache metadata.

## Tools

MCP keeps the granular CLI-shaped tools and also exposes the compact Agent Runtime Optimization API.

Granular tools use the `desktop_` prefix. Hyphens become underscores.

| CLI | MCP tool |
|---|---|
| `snapshot` | `desktop_snapshot` |
| `list-windows` | `desktop_list_windows` |
| `click` | `desktop_click` |
| `skills` | `desktop_skills` |

Granular tool arguments are structured JSON with the same field names accepted by the matching command in batch JSON. Tool execution reuses the normal agent-desktop command parser, permission preflight, interaction policy, and dispatcher rather than a second MCP-specific implementation.

`batch` is not recursively exposed as a granular MCP tool because MCP already provides the call boundary. `cursor-overlay` remains a session/configuration command and is not in the MCP tool catalog.

### Compact Agent API

For agent harnesses that want a small stable surface, prefer these three tools:

- `desktop.observe`: one targeted read-only observation. Full `snapshot` and `screenshot` are intentionally excluded from this compact surface so a harness cannot accidentally request a giant accessibility-tree or image payload. Granular tools remain available when explicitly required.
- `desktop.execute`: one or more semantic steps executed through the P0B compound engine. `semantic: true` is enforced by the runtime, so coordinate-only mutating steps fail during preflight before earlier side effects can run.
- `desktop.run`: the same semantic compound execution path plus a bounded caller-defined workflow identifier. P0C does not persist workflow definitions and does not introduce durable jobs or automatic replay.

`desktop.execute` and `desktop.run` preserve the existing command policy, permission preflight, per-step and whole-plan deadlines, condition/verification behavior, delivery disposition, and mutation no-replay contract. Their response adds a compact wrapper with `api_version`, `operation`, `provenance`, `verification`, and `state_change`; the underlying compound result remains available under `result` for exact delivery evidence.

Targeted observation example:

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "tools/call",
  "params": {
    "name": "desktop.observe",
    "arguments": {
      "command": "list-windows",
      "args": {}
    }
  }
}
```

Semantic compound example:

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "tools/call",
  "params": {
    "name": "desktop.execute",
    "arguments": {
      "steps": [
        {
          "command": "clipboard-clear",
          "args": {},
          "verify": {
            "command": "clipboard-get",
            "args": {},
            "json_pointer": "/text",
            "equals": ""
          }
        }
      ]
    }
  }
}
```

Named workflow invocation uses the same step shape:

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "method": "tools/call",
  "params": {
    "name": "desktop.run",
    "arguments": {
      "workflow": "clear-clipboard",
      "steps": [
        { "command": "clipboard-clear", "args": {} }
      ]
    }
  }
}
```

The compact API is additive. Existing hosts can continue using every granular `desktop_*` tool unchanged.

## Skills over MCP

The version-matched skills that are compiled into the executable are exposed three ways.

### Tool

Use `desktop_skills`:

```json
{
  "name": "desktop_skills",
  "arguments": {
    "action": "get",
    "name": "agent-desktop",
    "full": true
  }
}
```

### Resources

Available resource URIs include:

- `agent-desktop://skills`
- `agent-desktop://skills/agent-desktop`
- `agent-desktop://skills/agent-desktop/full`
- `agent-desktop://skills/agent-desktop-windows`
- `agent-desktop://skills/agent-desktop-ffi`

A bundled reference can be read by appending its reference path after the skill name.

### Prompt

The `agent-desktop-skill` prompt loads a bundled skill into the host conversation. It accepts optional `name` and `full` arguments.

For complex desktop work, load `agent-desktop://skills/agent-desktop` before the first observation/action loop. On Windows, also load `agent-desktop://skills/agent-desktop-windows`.

## Wire smoke tests

Legacy discovery of tools:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | agent-desktop --mcp
```

Modern discovery:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}' \
  | agent-desktop --mcp
```

The process remains alive until stdin closes.

## Security and execution policy

MCP does not bypass normal agent-desktop safety behavior.

- Permission checks use the same platform adapter path as CLI execution.
- Headless mode stays the default.
- Physical operations still require headed policy.
- UI refs keep the same snapshot/ref validation and stale-ref behavior.
- Compact execute/run force semantic compound mode and never replay a delivered mutation because verification failed.
- Tool errors are returned as MCP tool errors with the normal agent-desktop error payload in `structuredContent`.

The server is intentionally stdio-first. Remote HTTP exposure is not enabled by this feature.
