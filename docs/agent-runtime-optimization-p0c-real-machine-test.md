# ARO-P0C Real-Machine Acceptance Runbook

Purpose: verify the compact Agent Runtime Optimization API on a real desktop without weakening the normal agent-desktop safety model.

P0C verified code baseline:

- main: `8e9f30a8dc21beac9c64d2a6651d2c0733afd3ac`
- tree: `8b7dc61fcd3f2aa1a05f807901ecf854ce831ff8`
- CI #73 / `34470409683`: PASS
- CodeQL #73 / `34470409728`: PASS
- Supply Chain #73 / `34470409795`: PASS
- Release #18 / `34470409691`: PASS

The documentation-closure commit may have a newer main SHA while keeping the same runtime code. Record the exact binary/source SHA used for the machine test in the evidence table at the end.

## 1. Preconditions

Use a build from the exact main revision you intend to accept.

Check the binary and permissions first:

```powershell
agent-desktop --version
agent-desktop permissions
```

For read-only tests, normal headless MCP mode is enough:

```powershell
agent-desktop --mcp
```

For tests that intentionally exercise physical pointer/keyboard paths, start the MCP process with headed policy:

```powershell
agent-desktop --mcp --headed
```

Do not use destructive applications or real user data for this acceptance. Clipboard text and an empty Notepad window are sufficient harmless sentinels.

## 2. MCP lifecycle

The MCP transport is newline-delimited JSON-RPC over stdin/stdout. For the legacy handshake used by this runbook, send:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"aro-p0c-acceptance","version":"1"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

Every compact tool invocation uses the same envelope:

```json
{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"desktop.observe","arguments":{}}}
```

Use a new `id` for each request so evidence can be correlated without ambiguity.

## 3. A - Catalog and backward compatibility

Call `tools/list` after initialization.

Required compact tools:

- `desktop.observe`
- `desktop.execute`
- `desktop.run`

Also confirm existing granular tools are still present, for example `desktop_list_windows`, `desktop_snapshot`, and `desktop_click` where supported by the build.

Pass criteria:

- all three compact tools are advertised;
- granular tools remain advertised;
- compact input schemas are structured JSON schemas, not free-form command strings.

## 4. B - Observe is read-only and bounded

First run a normal targeted observation:

```json
{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"desktop.observe","arguments":{"command":"list-windows","args":{}}}}
```

Expected result includes:

- `api_version: 1`
- `operation: "observe"`
- provenance with `surface: "mcp"`
- `verification.state: "not_applicable"`
- the targeted command result

Then attempt to route a mutation through observe:

```json
{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"desktop.observe","arguments":{"command":"clipboard-clear","args":{}}}}
```

Expected: `INVALID_ARGS` before mutation.

Also inspect the `desktop.observe` schema from `tools/list` and confirm full `snapshot` and `screenshot` are not in its compact command enum. Those granular tools remain available explicitly when a full payload is truly needed.

## 5. C - Semantic preflight happens before any side effect

Set a harmless clipboard sentinel from PowerShell:

```powershell
Set-Clipboard -Value 'ARO-P0C-SENTINEL'
Get-Clipboard
```

Send one compact execution containing an initially valid mutation followed by a coordinate-only mutation:

```json
{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"desktop.execute","arguments":{"steps":[{"command":"clipboard-clear","args":{}},{"command":"mouse-click","args":{"xy":"10,10"}}]}}}
```

P0C forces semantic compound mode. The full plan must be preflighted before step 1 can mutate anything.

Expected:

- request fails with `INVALID_ARGS` because the coordinate-only mutation is not allowed in semantic mode;
- `clipboard-clear` was not executed;
- `Get-Clipboard` still returns `ARO-P0C-SENTINEL`.

Re-check:

```powershell
Get-Clipboard
```

This is the key proof that grouping operations into one harness call does not trade away fail-closed preflight.

## 6. D - Named run preserves verification and no-replay

Clear the clipboard through one named compact workflow and verify the resulting state in the same runtime call:

```json
{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"desktop.run","arguments":{"workflow":"clear-clipboard","steps":[{"command":"clipboard-clear","args":{},"verify":{"command":"clipboard-get","args":{},"json_pointer":"/text","equals":""}}]}}}
```

Expected wrapper evidence:

- `operation: "run"`
- `provenance.workflow: "clear-clipboard"`
- `verification.state: "passed"`
- `verification.requested: 1`
- `verification.passed: 1`

Expected underlying compound evidence includes successful verification and `mutation_replay: false`.

If verification is deliberately changed to an impossible value, the failure must remain a verification failure after the mutation delivery boundary. The runtime must not replay `clipboard-clear` merely because verification failed.

## 7. E - Runtime bounds match advertised schema

All invalid bounds below must fail before any candidate mutation.

### E1. Zero whole-plan timeout

```json
{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"desktop.execute","arguments":{"steps":[{"command":"clipboard-clear","args":{}}],"timeout_ms":0}}}
```

Expected: `INVALID_ARGS`.

### E2. Zero per-step timeout

```json
{"jsonrpc":"2.0","id":51,"method":"tools/call","params":{"name":"desktop.execute","arguments":{"steps":[{"command":"clipboard-clear","args":{},"timeout_ms":0}]}}}
```

Expected: `INVALID_ARGS`.

### E3. More than 64 steps

Construct an array containing 65 harmless candidate steps, for example 65 `clipboard-clear` objects, and send it to `desktop.execute`.

Expected: `INVALID_ARGS` before step 1 executes.

Use a clipboard sentinel before the call if you want direct side-effect evidence:

```powershell
Set-Clipboard -Value 'ARO-P0C-BOUND-SENTINEL'
```

After the rejected request:

```powershell
Get-Clipboard
```

Expected: `ARO-P0C-BOUND-SENTINEL` remains unchanged.

### E4. Assertion JSON Pointer longer than 256 Unicode characters

Send a condition or verification assertion whose `json_pointer` contains more than 256 characters.

Expected: `INVALID_ARGS` before any candidate mutation.

Repeat once with `condition` and once with `verify` if you are doing the full acceptance matrix.

## 8. F - Harness round-trip optimization evidence

Measure top-level MCP `tools/call` requests, not internal provider calls.

Baseline granular harness pattern for a condition/action/verification flow normally requires separate calls such as:

```text
read condition
-> mutate
-> read verification state
```

P0C can carry the same bounded condition/action/verification plan inside one `desktop.execute` or `desktop.run` top-level `tools/call`.

Record:

- number of top-level MCP requests in the granular version;
- number of top-level MCP requests in the compact version;
- payload bytes if your harness exposes them;
- wall-clock time only if you actually measure it.

Do not convert lower round-trip count into an unmeasured latency/speed-up claim. Provider work and OS scheduling can dominate wall-clock time.

## 9. G - Optional P0A integration check with Notepad

This check is useful when validating the full optimization stack rather than P0C alone.

Open a clean Notepad window and identify a stable semantic target. Resolve the same target twice through the normal semantic `find` path while collecting the `app_profile.resolve` trace/counters.

Expected behavior from the previously verified P0A acceptance:

- cold lookup can learn the target profile;
- warm lookup reuses valid semantic/live knowledge;
- previously measured reference case reduced `tree_reads` from 28 cold to 0 warm;
- moving the window does not invalidate semantic identity merely because bounds changed;
- restarting the process invalidates the old live generation and requires semantic re-resolution rather than reuse of the old native handle.

Treat `28 -> 0` as retained evidence from the verified benchmark scenario, not as a guarantee that every application produces exactly those counts.

## 10. H - Evidence capture

Record one row per acceptance case.

| Field | Value |
|---|---|
| exact main/source SHA | |
| exact tree | |
| binary version | |
| OS / architecture | |
| MCP mode | headless / headed |
| test ID | A / B / C / D / E1 / E2 / E3 / E4 / F / G |
| top-level `tools/call` count | |
| result | PASS / FAIL |
| error code/details when expected | |
| mutation sentinel preserved when required | yes / no / n/a |
| P0A cold `tree_reads` | |
| P0A warm `tree_reads` | |
| notes | |

## 11. Final acceptance rule

P0C real-machine acceptance is PASS only when:

- compact and granular tool surfaces coexist;
- observe cannot mutate;
- semantic full-plan preflight prevents earlier mutations when any later semantic step is invalid;
- verification is distinct from action delivery and delivered mutations are not replayed;
- runtime bounds reject invalid requests before side effects;
- the exact binary/source SHA is recorded.

A machine-specific failure is evidence to investigate, not a reason to weaken an existing timeout, safety guard, or regression test.
