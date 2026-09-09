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
| ARO-P0A | P0 | Semantic AppProfile Cache | verified baseline | CODE COMPLETE | PR #11; implementation checkpoint `bebec4046a380477415d0f347a508f134fc8807a`; final gates pending |
| ARO-P0B | P0 | Compound Execution Engine | P0A foundation | PLANNED | |
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

Status: CODE COMPLETE - VERIFYING

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
- [ ] exact-head CI PASS.
- [ ] exact-head CodeQL PASS.
- [ ] exact-head Supply Chain PASS.
- [ ] guarded merge.
- [ ] post-merge CI PASS.
- [ ] post-merge CodeQL PASS.
- [ ] post-merge Supply Chain PASS.
- [ ] post-merge Release PASS.

### Verification checkpoint

- PR: #11
- code implementation checkpoint before documentation sync: `bebec4046a380477415d0f347a508f134fc8807a`
- previous run evidence used for root-cause fixing:
  - Supply Chain #45: PASS
  - Linux tests #45: PASS, confirming the prior session-GC failure was transient and unrelated to P0A
  - stale-ref constructor policy #45: PASS
  - remaining #45 P0A failure: two `unused_mut` Clippy findings, fixed at the implementation checkpoint above
- final exact-head CI / CodeQL / Supply Chain remain intentionally unchecked until the documentation-synced head passes.
- no merge has occurred; P0B/P0C remain untouched.

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
