# Agent Runtime Optimization Tracking

Initiative branch: `feat/agent-runtime-optimization`
Plan: `docs/agent-runtime-optimization-plan.md`

GitHub Issues are disabled for this repository, so this file is the canonical task tracker until repository issue tracking is enabled.

## Status vocabulary

- PLANNED: specified but not ready to implement.
- READY: dependencies satisfied and may be started.
- IN PROGRESS: implementation has started.
- BLOCKED: cannot progress without an external dependency.
- CODE COMPLETE: implementation is done but verification is incomplete.
- DONE / VERIFIED: exact-head gates and post-merge gates have passed.

## Task board

| ID | Priority | Task | Depends on | Status | Evidence |
|---|---|---|---|---|---|
| ARO-P0A | P0 | Semantic AppProfile Cache | verified baseline | DONE / VERIFIED | PR #11; final branch `72da253b00e498215a32a15034877d32aa30474e`; merge `69d071450f10780779ff327c74382748df99d306`; exact-head CI/CodeQL/Supply Chain and post-merge CI/CodeQL/Supply Chain/Release PASS |
| ARO-P0B | P0 | Compound Execution Engine | P0A foundation | DONE / VERIFIED | manual acceptance 12/12 PASS; feature `9f0ce7c0850e941fc33988d495410a4ff86f785d` / tree `03913a552af5ec8bfcb99f6922b1646fc1ab54f4`; PR #12; exact-head CI #58 / CodeQL #58 / Supply Chain #58 PASS; merge `e2356cb984695c21f992619bd918d850bbecdd7d`; post-merge CI #59 / CodeQL #59 / Supply Chain #59 / Release #14 PASS |
| ARO-P0C | P0 | Compact Agent API: observe/execute/run | P0A, P0B contracts | PLANNED | |
| ARO-P1A | P1 | View Handles + State Delta | P0A, P0C | PLANNED | |
| ARO-P1B | P1 | Event Bus + Cache Invalidation | P0A | PLANNED | |
| ARO-P1C | P1 | Verification + Recovery + Safety | P0A, P0B | PLANNED | |
| ARO-P2A | P2 | Capability Discovery + Router | P0C, P1C | PLANNED | |
| ARO-P2B | P2 | Learned Actions + Record-to-Skill | P0B, P1C | PLANNED | |
| ARO-P2C | P2 | Durable Jobs + MCP Tasks | P0B, P0C | PLANNED | |
| ARO-P3A | P3 | A2A Surface | P2C | PLANNED | |
| ARO-P3B | P3 | Vision Fallback | P1C, P2A | PLANNED | |

## ARO-P0A - Semantic AppProfile Cache

Status: DONE / VERIFIED

Implementation prompt: `docs/prompts/agent-runtime-optimization-p0a.md`

### Objective

Reduce repeated accessibility/provider work by caching semantic selector knowledge while keeping live platform element references short-lived and generation-bound.

### Required implementation

- [x] inventory the existing element/ref/cache identity model across core and native adapters.
- [x] define `AppProfile`, semantic selector recipe, live-ref generation, and cache diagnostics.
- [x] use stable semantic signals rather than bounds/runtime ids as durable identity.
- [x] implement one vertical slice through an existing read/resolve path before generalizing.
- [x] add scoped revalidation.
- [x] invalidate live refs on process/window generation change.
- [x] fail closed on ambiguous candidates.
- [x] preserve current delivery semantics and stale-ref behavior.
- [x] expose benchmark counters for cold and warm resolution.
- [x] add Linux/macOS/Windows-compatible contracts where applicable.
- [x] document storage/lifetime boundaries.

### Acceptance

- [x] first lookup can learn a semantic selector.
- [x] repeated stable lookup performs fewer provider/tree reads.
- [x] process restart makes the live ref stale but semantic re-resolution can recover.
- [x] moved element can resolve without depending on old bounds.
- [x] duplicate candidates refuse.
- [x] corrupted/mismatched cache fails closed.
- [x] existing CLI/MCP/Skills behavior is unchanged.
- [x] exact-head CI PASS.
- [x] exact-head CodeQL PASS.
- [x] exact-head Supply Chain PASS.
- [x] guarded merge.
- [x] post-merge CI PASS.
- [x] post-merge CodeQL PASS.
- [x] post-merge Supply Chain PASS.
- [x] post-merge Release PASS.

### Verification checkpoint

- PR: #11
- final branch head: `72da253b00e498215a32a15034877d32aa30474e`
- final branch tree: `3ba6283b8cbdfcf98f215a5f2cc9c6459a9ab7e6`
- exact-head CI #49 / run `34377740049`: PASS
- exact-head CodeQL #49 / run `34377740077`: PASS
- exact-head Supply Chain #49 / run `34377740102`: PASS
- guarded squash merge expected head: `72da253b00e498215a32a15034877d32aa30474e`
- merge commit: `69d071450f10780779ff327c74382748df99d306`
- merge tree: `3ba6283b8cbdfcf98f215a5f2cc9c6459a9ab7e6`
- post-merge CI #50 / run `34379682644`: PASS
- post-merge CodeQL #50 / run `34379682651`: PASS
- post-merge Supply Chain #50 / run `34379682629`: PASS
- post-merge Release #12 / run `34379682645`: PASS
- exact-head push verification used a temporary `master` ref that pointed at the final branch head; checkout logs recorded `72da253b00e498215a32a15034877d32aa30474e`, not `refs/pull/11/merge`.

## Evidence template

When a task moves to DONE / VERIFIED, record:

```text
final branch head:
final tree:
CI run:
CodeQL run:
Supply Chain run:
PR:
merge commit:
merge tree:
post-merge CI:
post-merge CodeQL:
post-merge Supply Chain:
post-merge Release:
```

## Working rule

Only one vertical slice should be IN PROGRESS at a time unless two slices are proven independent. Do not broaden scope merely because adjacent architecture is attractive.

## ARO-P0B - Compound Execution Engine

Status: DONE / VERIFIED

Implementation prompt: `docs/prompts/agent-runtime-optimization-p0b.md`

### Initial vertical slice

- [x] reuse the existing batch executor rather than create a parallel engine.
- [x] keep legacy batch behavior unchanged by default.
- [x] add opt-in semantic compound guardrails.
- [x] add typed per-item condition and verification assertions.
- [x] preflight nested assertions before side effects.
- [x] add per-step deadlines capped by the whole-plan deadline.
- [x] preserve action -> event-wait baseline behavior.
- [x] carry mutation delivery semantics through verification failures.
- [x] never replay the main mutation after verification failure.
- [x] add compact plan trace metadata without payload values.
- [x] targeted P0B tests PASS.
- [x] exact-head CI PASS.
- [x] exact-head CodeQL PASS.
- [x] exact-head Supply Chain PASS.
- [x] guarded merge.
- [x] post-merge CI PASS.
- [x] post-merge CodeQL PASS.
- [x] post-merge Supply Chain PASS.
- [x] post-merge Release PASS.

### Manual optimization and safety acceptance

All requested Windows acceptance cases passed on the P0A/P0B optimization implementation.

P0A evidence:

- A3 cold vs warm lookup: PASS. Cold `cache_miss` used `tree_reads: 28`, `cold_lookups: 1`; warm `live_hit` + `revalidated` used `tree_reads: 0`, `warm_lookups: 1`.
- A4 window moved: PASS. Bounds movement preserved semantic identity and the following lookup remained warm with `tree_reads: 0`.
- A5 process restart and semantic re-resolution: PASS. Live generation invalidated and equivalent semantic target was resolved without reusing the old UI handle.
- A6 automated P0A regression suite: PASS 6/6, including warm reuse, bounds motion, process/window recreation, duplicate candidates, corrupted cache, and identity mismatch.

P0B evidence:

- B1 condition prevents side effect: PASS. Failed condition produced `not_started`, `not_delivered`, retry-safe, and preserved clipboard state.
- B2 action plus verification in one request: PASS. `compound_verification.verified: true`, `outcome: succeeded`, `mutation_replay: false`.
- B3 verification failure differs from action failure: PASS. `compound_verification_failed`, `delivered_unverified`, retry-unsafe.
- B4 no mutation replay after verification failure: PASS. Mutation counter remained exactly 1.
- B5 per-step deadline: PASS. A 5000 ms wait with `timeout_ms: 25` terminated at the step deadline with `TIMEOUT` instead of waiting for the requested sleep.
- B6 semantic mode rejects coordinate mutation during preflight: PASS. `INVALID_ARGS` occurred before any side effect.
- B7 nested assertion preflight: PASS. Mutating assertion was rejected before any side effect.
- B8 compact trace payload privacy: PASS. Secret payload was absent from `plan_trace`; trace contained only bounded metadata.

Manual acceptance summary: PASS 12/12.

### Final verification checkpoint

- PR: #12
- final feature head: `9f0ce7c0850e941fc33988d495410a4ff86f785d`
- final feature tree: `03913a552af5ec8bfcb99f6922b1646fc1ab54f4`
- exact-head CI #58 / run `34440064110`: PASS
- exact-head CodeQL #58 / run `34440064150`: PASS
- exact-head Supply Chain #58 / run `34440064114`: PASS
- guarded squash merge expected head: `9f0ce7c0850e941fc33988d495410a4ff86f785d`
- merge commit: `e2356cb984695c21f992619bd918d850bbecdd7d`
- merge tree: `03913a552af5ec8bfcb99f6922b1646fc1ab54f4`
- post-merge CI #59 / run `34441106403`: PASS
- post-merge CodeQL #59 / run `34441106418`: PASS
- post-merge Supply Chain #59 / run `34441106394`: PASS
- post-merge Release #14 / run `34441106399`: PASS
- Windows x64 full Test: PASS
- Windows ARM64 full Test: PASS
- Windows E2E contract gate: PASS
- refusal guard: PASS
- capture redaction: PASS
- fixture compile smoke: PASS
- shipped binary size: PASS
- profile isolation: PASS
- cleanup/post-job steps: PASS
- P0A optimization evidence retained: cold `tree_reads: 28` -> warm `tree_reads: 0`.
- P0B compound execution reduces multiple harness round trips into one runtime call while preserving delivery/no-replay safety semantics.
- ARO-P0C remains PLANNED and untouched.
