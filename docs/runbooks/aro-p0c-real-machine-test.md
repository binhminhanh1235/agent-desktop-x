# ARO-P0C real-machine acceptance

Use this after the P0C merge is verified on `main`. It is intentionally a host-level acceptance, not a replacement for CI.

## What this proves

The test checks the shipped native executable rather than an in-process unit-test adapter:

1. MCP starts over stdio.
2. `tools/list` contains `desktop.observe`, `desktop.execute`, and `desktop.run` while granular tools remain available.
3. `desktop.observe` can perform a targeted read and refuses mutation/full-tree commands.
4. `desktop.execute` uses semantic compound preflight, verification, delivery semantics, and mutation no-replay behavior.
5. `desktop.run` uses the same engine while preserving a workflow identifier.
6. The release binary still works through the normal Windows/macOS/Linux build path.

## Prerequisites

- Check out the exact final `main` SHA reported in the P0C verification evidence.
- Rust toolchain from `rust-toolchain.toml` installed.
- On Windows, use a normal PowerShell terminal first. Elevation is only needed when the target application itself runs elevated.
- For UI-targeted follow-up tests, grant/meet the normal platform accessibility requirements documented by agent-desktop.

## 1. Exact source and release build

```powershell
git status --short
git rev-parse HEAD
git rev-parse 'HEAD^{tree}'
cargo test --locked -p agent-desktop --bin agent-desktop compact
cargo build --locked -p agent-desktop --release
.\target\release\agent-desktop.exe --version
```

Expected:

- clean worktree;
- HEAD and tree match the final evidence;
- compact tests PASS;
- release build succeeds;
- version command returns the normal agent-desktop version.

On macOS/Linux replace the executable path with `./target/release/agent-desktop`.

## 2. Helper for one-shot MCP calls on Windows

In PowerShell:

```powershell
$AgentDesktop = (Resolve-Path '.\target\release\agent-desktop.exe').Path

function Invoke-AgentDesktopMcpLines {
    param([string[]]$Lines)
    $raw = $Lines | & $AgentDesktop --mcp
    if ($LASTEXITCODE -ne 0) { throw "agent-desktop MCP exited with $LASTEXITCODE" }
    @($raw | ForEach-Object { $_ | ConvertFrom-Json -Depth 100 })
}
```

Each call closes stdin after the supplied lines, so the native MCP process exits normally after replying.

## 3. Tool catalog compatibility

```powershell
$r = Invoke-AgentDesktopMcpLines @(
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"aro-p0c-real","version":"1"}}}',
  '{"jsonrpc":"2.0","method":"notifications/initialized"}',
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
)

$names = @($r[-1].result.tools | ForEach-Object name)
$required = @('desktop.observe','desktop.execute','desktop.run','desktop_snapshot','desktop_find','desktop_click','desktop_skills')
$missing = @($required | Where-Object { $_ -notin $names })
if ($missing.Count -ne 0) { throw "Missing MCP tools: $($missing -join ', ')" }
'MCP catalog: PASS'
```

Expected: `MCP catalog: PASS`.

## 4. Targeted observe and full-tree refusal

```powershell
$r = Invoke-AgentDesktopMcpLines @(
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"desktop.observe","arguments":{"command":"version","args":{}}}}',
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"desktop.observe","arguments":{"command":"snapshot","args":{}}}}',
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"desktop.observe","arguments":{"command":"clipboard-clear","args":{}}}}'
)

if ($r[0].result.isError) { throw 'desktop.observe version unexpectedly failed' }
if ($r[0].result.structuredContent.operation -ne 'observe') { throw 'observe wrapper missing' }
if (-not $r[1].result.isError) { throw 'desktop.observe incorrectly allowed snapshot' }
if (-not $r[2].result.isError) { throw 'desktop.observe incorrectly allowed mutation' }
'Observe boundary: PASS'
```

Expected: `Observe boundary: PASS`. This proves the compact observation path does not silently become a giant tree dump or mutation surface.

## 5. Semantic preflight occurs before side effects

Copy this exact sentinel into the Windows clipboard before running the block:

```text
ARO-P0C-SENTINEL
```

Then run:

```powershell
$r = Invoke-AgentDesktopMcpLines @(
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"desktop.execute","arguments":{"steps":[{"command":"clipboard-clear","args":{}},{"command":"mouse-click","args":{"xy":"10,10"}}]}}}',
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"desktop.observe","arguments":{"command":"clipboard-get","args":{}}}}'
)

if (-not $r[0].result.isError) { throw 'coordinate-only mutation was not rejected' }
$clipboard = $r[1].result.structuredContent.result.text
if ($clipboard -ne 'ARO-P0C-SENTINEL') { throw "preflight was not side-effect free; clipboard=$clipboard" }
'Semantic preflight/no side effect: PASS'
```

Expected: the invalid semantic plan is rejected and the clipboard still contains the sentinel.

## 6. Execute + verification + no replay

```powershell
$r = Invoke-AgentDesktopMcpLines @(
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"desktop.execute","arguments":{"steps":[{"command":"clipboard-clear","args":{},"verify":{"command":"clipboard-get","args":{},"json_pointer":"/text","equals":""}}]}}}'
)

$x = $r[0].result.structuredContent
if ($r[0].result.isError) { throw 'desktop.execute unexpectedly failed' }
if ($x.operation -ne 'execute') { throw 'execute wrapper missing' }
if ($x.provenance.engine -ne 'compound-v1') { throw 'execute did not use compound-v1' }
if (-not $x.provenance.semantic_only) { throw 'execute did not force semantic mode' }
if ($x.verification.state -ne 'passed') { throw "verification state=$($x.verification.state)" }
if ($x.result.compound.mutation_replay) { throw 'mutation replay must remain false' }
'Execute verify/no-replay: PASS'
```

Expected: `Execute verify/no-replay: PASS`.

## 7. Named run path

```powershell
$r = Invoke-AgentDesktopMcpLines @(
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"desktop.run","arguments":{"workflow":"real-machine-clear-clipboard","steps":[{"command":"clipboard-clear","args":{},"verify":{"command":"clipboard-get","args":{},"json_pointer":"/text","equals":""}}]}}}'
)

$x = $r[0].result.structuredContent
if ($r[0].result.isError) { throw 'desktop.run unexpectedly failed' }
if ($x.operation -ne 'run') { throw 'run wrapper missing' }
if ($x.provenance.workflow -ne 'real-machine-clear-clipboard') { throw 'workflow provenance missing' }
if ($x.verification.state -ne 'passed') { throw 'run verification did not pass' }
if ($x.result.compound.mutation_replay) { throw 'run mutation replay must remain false' }
'Run workflow path: PASS'
```

Expected: `Run workflow path: PASS`.

## 8. Optional real application smoke

After the protocol/safety tests above pass, use a normal desktop app you are comfortable modifying:

1. Start the app manually.
2. Use `desktop.observe` with `list-windows`, then `find` for one exact stable semantic target.
3. Use the returned qualified ref in `desktop.execute` with a read-only condition and a read-only verification.
4. Move the window and repeat the same semantic lookup to exercise the P0A warm path.
5. Restart the app and repeat to confirm the old live generation is invalidated rather than reused.

For performance evidence, inspect the existing `app_profile.resolve` trace counters. The previously verified optimization baseline is cold `tree_reads: 28` versus warm `tree_reads: 0`; do not claim the same number for a new application unless the new trace actually reports it.

## PASS criteria

The machine acceptance is PASS only when all of the following are true:

- exact final SHA/tree checked;
- release binary builds and starts MCP;
- compact + granular catalog present;
- observe read succeeds;
- observe snapshot/mutation refusal succeeds;
- semantic preflight proves no earlier side effect;
- execute verification passes with `mutation_replay: false`;
- run workflow provenance and verification pass;
- any application-specific performance number is copied from actual trace evidence, not assumed.

If any step fails, keep the full JSON response and the exact HEAD SHA. Do not retry a mutation when the returned disposition says retry is unsafe.
