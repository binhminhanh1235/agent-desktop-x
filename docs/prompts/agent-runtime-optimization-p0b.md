# ARO-P0B — Compound Execution Engine

## Scope

Implement only ARO-P0B on branch `feat/agent-runtime-optimization-p0b`.

Do not start ARO-P0C.

Verified baseline:

- main: `1c2c5ccbd92f9641f9a7c10e93d692a94bc63a99`
- tree: `112f2ccd9f19cda7a849703ea21f5ecce741feca`
- ARO-P0A: DONE / VERIFIED

## Goal

Evolve the existing sequential batch executor into the reusable compound-execution foundation for later `desktop.execute` without creating a second parallel engine.

Core model:

`ACTION + CONDITION + WAIT + VERIFICATION + BOUNDED RECOVERY`

P0B must reduce harness round trips while preserving existing delivery semantics and refusing unsafe recovery.

## Required architecture

- reuse `src/batch`, existing command dispatch, action/ref pipelines, wait/event baseline support, and command policy preflight.
- keep legacy batch behavior backward compatible by default.
- add an opt-in semantic compound mode instead of silently changing existing coordinate-capable batches.
- every command and nested assertion must be parsed and policy-preflighted before the first side effect.
- conditions and verification commands must be read-only.
- support a per-step deadline capped by the whole-plan deadline.
- keep the existing whole-plan deadline.
- never replay a mutating command after it may have been delivered.
- verification failure must be distinct from transport/action failure.
- carry the action delivery disposition through verification failure.
- reject coordinate-only targeting in semantic compound mode.
- retain existing pre-action event baseline behavior for action -> wait flows.
- emit a compact machine-readable plan trace without copying user data into trace metadata.

## Initial vertical slice

Extend batch items with optional `timeout_ms`, `condition`, and `verify`.

Assertion shape:

```json
{
  "command": "is",
  "args": {"ref_id": "@snapshot:e1", "property": "enabled"},
  "json_pointer": "/result",
  "equals": true
}
```

A false condition means the main command is not started and delivery is `not_delivered`.

Verification runs once after successful main-command dispatch. A failed verification does not replay the main command and must preserve an unsafe delivery disposition for already-delivered mutation.

## Acceptance

- legacy batch JSON keeps old behavior when semantic mode and compound metadata are absent.
- semantic mode rejects coordinate-only mutating steps before any side effect.
- all nested condition/verification commands are validated before execution starts.
- condition mismatch prevents the mutation.
- action executes at most once even when verification fails.
- verification failure reports the exact batch index/command/phase.
- verification failure after mutation is delivered-unverified or stricter and retry-unsafe.
- verification success is machine-readable.
- per-step timeout cannot exceed the whole-plan deadline.
- action -> event wait still uses a pre-action baseline.
- compact plan trace records index, command, outcome, phase, elapsed time, and disposition without payload values.
- Linux/macOS/Windows contracts remain compatible.

## Verification policy

Run targeted tests, then exact-head CI, CodeQL, and Supply Chain. Fix exact failed jobs. Guarded merge only after all three pass. Then verify exact main CI, CodeQL, Supply Chain, and Release. Only then mark ARO-P0B DONE / VERIFIED. Do not automatically start ARO-P0C.
