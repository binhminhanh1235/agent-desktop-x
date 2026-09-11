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
| ARO-P0C | P0 | Compact Agent API: observe/execute/run | P0A, P0B contracts | DONE / VERIFIED | PR #13 compact API; PR #14 runtime-bound hardening; PR #15 Windows file-lock repair; final code baseline `8e9f30a8dc21beac9c64d2a6651d2c0733afd3ac`; CI/CodeQL/Supply/Release #73 PASS |
| ARO-P1A | P1 | View Handles + State Delta | P0A, P0C | DONE / VERIFIED | final branch `b874e4fafad4b2312ec3a87a6ab0ed4b37cdfe8e` / tree `e0a571dbf853132760333e9bf2056a1ceb415a2f`; exact-head CI/CodeQL/Supply Chain #86 PASS; PR #16; merge `8c7f4bcc4c46c956f06b101fdb329562d3ddc8c7`; post-merge CI/CodeQL/Supply Chain #88 + Release #20 PASS |
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

## ARO-P0C - Compact Agent API

Status: DONE / VERIFIED

Real-machine runbook: `docs/agent-runtime-optimization-p0c-real-machine-test.md`

### Required implementation

- [x] expose exactly three compact high-level tools: `desktop.observe`, `desktop.execute`, and `desktop.run`.
- [x] retain existing granular `desktop_*` MCP tools.
- [x] keep `desktop.observe` read-only and exclude full snapshot/screenshot from the compact enum.
- [x] route `desktop.execute` and `desktop.run` through the existing semantic compound engine.
- [x] preserve full-plan semantic preflight before side effects.
- [x] preserve permission/deadline/condition/verification/delivery/no-replay contracts.
- [x] return compact provenance, verification, and state-change summaries.
- [x] enforce schema/runtime parity for step count, timeouts, and assertion pointer length before dispatch.
- [x] repair the Windows x64 file-lock deadline regression discovered by post-hardening CI without increasing the lock budget or weakening the test.

### Focused acceptance

- [x] compact schemas are deterministic and bounded.
- [x] compact tools and granular tools coexist in `tools/list`.
- [x] observe refuses mutation.
- [x] execute rejects coordinate-only semantic mutation during preflight before earlier side effects.
- [x] run preserves verification and no mutation replay.
- [x] more than 64 steps fail before side effects.
- [x] zero top-level/per-step timeout fails before side effects.
- [x] assertion JSON Pointer longer than 256 Unicode characters fails before side effects.
- [x] Windows x64 file-lock acquisition performs one immediate non-blocking attempt even if setup consumed the caller budget; contention waiting remains deadline-bounded.
- [x] exact-head and final code-baseline CI/CodeQL/Supply Chain/Release PASS.

### Verification checkpoint

Original compact API, PR #13:

- final feature head: `3723a9c090e3deaed5acf626c8e7dae10cfb0746`
- final feature tree: `6b6b67bc4838def6d0b490e795257795929254f7`
- exact-head CI #65 / run `34452517347`: PASS
- exact-head CodeQL #65 / run `34452517345`: PASS
- exact-head Supply Chain #65 / run `34452517356`: PASS
- guarded squash merge: `c29d291b0f5cf7c6712165ac8204cd9725647233`
- merge tree: `6b6b67bc4838def6d0b490e795257795929254f7`
- post-merge CI #66 / run `34454566703`: PASS
- post-merge CodeQL #66 / run `34454566815`: PASS
- post-merge Supply Chain #66 / run `34454566702`: PASS
- post-merge Release #16 / run `34454566772`: PASS

Runtime-bound hardening, PR #14:

- final head: `bfea5c4ccc9f8b97284a7d86f4873e656d21c420`
- final tree: `352b7cd34766a6b33437ca493c57148fec468ca3`
- exact-head CI #68 / run `34455540410`: PASS
- exact-head CodeQL #68 / run `34455540510`: PASS
- exact-head Supply Chain #68 / run `34455540574`: PASS
- guarded squash merge: `7d934135491e220740d6c1af6d5449e2324b9560`
- merge tree: `352b7cd34766a6b33437ca493c57148fec468ca3`
- post-hardening Supply Chain #69 / run `34458017873`: PASS
- post-hardening CodeQL #69 / run `34458017888`: PASS
- post-hardening Release #17 / run `34458017912`: PASS
- post-hardening CI #69 / run `34458017919`: FAIL on Windows x64; the regression was treated as a blocker rather than waived.

Windows file-lock baseline repair, PR #15:

- final head: `225a897bc26bc7335aeb324a6d4f81456447e253`
- final tree: `8b7dc61fcd3f2aa1a05f807901ecf854ce831ff8`
- exact-head CI #72 / run `34469010952`: PASS
- exact-head CodeQL #72 / run `34469010963`: PASS
- exact-head Supply Chain #72 / run `34469010945`: PASS
- guarded squash merge: `8e9f30a8dc21beac9c64d2a6651d2c0733afd3ac`
- merge tree: `8b7dc61fcd3f2aa1a05f807901ecf854ce831ff8`
- final code-baseline CI #73 / run `34470409683`: PASS
- final code-baseline CodeQL #73 / run `34470409728`: PASS
- final code-baseline Supply Chain #73 / run `34470409795`: PASS
- final code-baseline Release #18 / run `34470409691`: PASS
- Windows x64 unit tests and full E2E/safety lane: PASS
- Windows ARM64 full lane: PASS

Optimization evidence retained across P0A-P0C:

- P0A cold resolution `tree_reads: 28` -> warm resolution `tree_reads: 0`.
- P0B/P0C can carry condition/action/verification in one top-level runtime/MCP call instead of requiring separate harness calls.
- no fixed latency speed-up is claimed without a measured wall-clock benchmark.

## ARO-P1A - View Handles + State Delta

Status: DONE / VERIFIED

Implementation prompt: `docs/prompts/agent-runtime-optimization-p1a.md`

Exact branch base:

- main SHA: `89bc5ff2fcd2e5d1db935a50d440e82221accbf2`
- tree: `9892105cf0f29c26e472dd2a75323841074fcff7`
- working branch: `feat/agent-runtime-optimization-p1a`

Verified boundary and implementation:

- compact observe still calls the existing granular dispatch path after parsing a `BatchCommand` and enforcing read-only semantics;
- P1A view flow is opt-in and currently supports only `list-windows`; legacy observe without `view` keeps the existing response contract;
- views are process-local, thread-safe, bounded to 64 entries, TTL-bound to 30 seconds, and deterministically oldest-first evicted;
- canonical view state and deltas are bounded, deterministically ordered, and use semantic comparison keys derived from app, PID, process generation, and title;
- raw/native `WindowInfo.id` is explicitly excluded from canonical view state and comparison identity, so runtime OS handles never become durable semantic identity;
- missing or ambiguous safe semantic identity fails closed rather than falling back to runtime handles or provider array order;
- optional top-level `expected_view_id` is available on `desktop.execute` and `desktop.run`, bounded to exactly 19 characters with schema pattern `^v1-[0-9a-fA-F]{16}$`;
- before any mutation, `expected_view_id` resolves the short-lived stored view, performs a fresh provider `list_windows` observation in the same scope, canonicalizes current state, and compares it to the stored state;
- unknown/expired views fail closed and changed state returns `VIEW_STALE` before side effects; freshness re-observation consumes the same whole-plan timeout budget rather than adding a second timeout budget;
- view evidence is observation evidence, not authorization: a fresh expected view still enters the existing P0B/P0C semantic compound engine and cannot authorize coordinate-only mutation;
- delivery disposition and mutation no-replay semantics remain owned by the unchanged P0B/P0C execution path;
- baseline-to-feature changed files are limited to P1A compact/view code, focused tests, the Windows menu fixture, prompt/tracking docs; no P1B event bus, P1C recovery/confidence expansion, P2 router/jobs/vision work, or granular MCP semantic changes are present.

### Acceptance mapping

- [x] 1. first observation creates a bounded view id and returns the normal full result plus view metadata.
- [x] 2. identical second observation performs a fresh provider read and returns an empty `added` / `removed` / `changed` delta.
- [x] 3. one added semantic entity produces exactly one `added` entry.
- [x] 4. one removed semantic entity produces exactly one `removed` entry.
- [x] 5. one non-identity semantic state change produces exactly one `changed` entry.
- [x] 6. provider/input ordering does not change canonical ordering or create false deltas.
- [x] 7. unknown previous view ids refuse deterministically with `VIEW_UNKNOWN`.
- [x] 8. expired views refuse explicitly with `VIEW_EXPIRED`.
- [x] 9. incompatible observation scope refuses with `VIEW_SCOPE_MISMATCH`.
- [x] 10. capacity eviction is deterministic and oldest-first.
- [x] 11. canonical view state never persists raw/native runtime window ids; missing/ambiguous semantic identity fails closed.
- [x] 12. execute/run schemas expose only bounded `expected_view_id`; fresh expected view is re-observed before mutation, while unknown/stale expected views refuse before any side effect with `VIEW_UNKNOWN` / `VIEW_STALE`.
- [x] 13. `expected_view_id` is never mutation authorization and does not make coordinate-only mutation valid.
- [x] 14. fresh expected view still traverses the P0B/P0C semantic preflight; focused tests prove the freshness read cannot bypass compound preflight.
- [x] 15. delivery/no-replay semantics remain on the unchanged P0B/P0C path; P1A introduces no replay engine or alternate mutation path.
- [x] 16. existing P0C calls without view fields remain backward compatible; legacy observe returns `result` without `view`/`delta`, and execute/run regression tests remain green.
- [x] 17. existing granular MCP tools are unchanged; baseline-to-feature diff contains no granular MCP dispatch/tool implementation changes and full CI remains green.
- [x] 18. scope stayed inside P1A: no P1B event bus/invalidation, no P1C recovery/confidence expansion, and no P2 capability router/durable jobs/vision implementation.

### Optimization evidence

Deterministic focused scenario: 30 semantic windows are observed, then exactly one existing window changes bounds.

- full entries: 30.
- subsequent delta entries: 1.
- full result bytes: 7130 for the deterministic 30-window fixture serialization measured by the same `serde_json::to_vec` representation used by `full_result_bytes`.
- delta payload bytes: 295 for the one-window changed delta fixture serialization measured by the same representation used by `delta_payload_bytes`.
- unchanged entities: 29 omitted from the delta.
- harness calls for each observation: 1.
- focused acceptance asserts the one-change delta payload is smaller than the full result and the serialized subsequent delta response is smaller than the first full-response envelope.
- no wall-clock latency or percentage speed-up claim is made because P1A has not added a wall-clock benchmark.
- P0A `tree_reads: 28 -> 0` is retained as separate P0A evidence and is not presented as P1A delta evidence.

### Final verification checkpoint

- final branch head: `b874e4fafad4b2312ec3a87a6ab0ed4b37cdfe8e`
- final branch tree: `e0a571dbf853132760333e9bf2056a1ceb415a2f`
- exact-head CI #86 / run `34560362822`: PASS.
- exact-head CodeQL #86 / run `34560362795`: PASS.
- exact-head Supply Chain #86 / run `34560362814`: PASS.
- PR: #16 `feat(aro): add P1A view handles and state delta`.
- guarded squash merge expected head: `b874e4fafad4b2312ec3a87a6ab0ed4b37cdfe8e`.
- merge commit: `8c7f4bcc4c46c956f06b101fdb329562d3ddc8c7`.
- merge tree: `e0a571dbf853132760333e9bf2056a1ceb415a2f`.
- post-merge CI #88 / run `34561325014`: PASS.
- post-merge CodeQL #88 / run `34561325011`: PASS.
- post-merge Supply Chain #88 / run `34561325003`: PASS.
- post-merge Release #20 / run `34561324981`: PASS.
- post-merge Windows x64 full lane: PASS, including Clippy, core/unit tests, examples, binary command tests, FFI integration, stripped release binary, Windows E2E contract, seeded-failure, capture redaction, citation gate, refusal guard, fixture compile, binary-size, profile isolation, cleanup/post-job, and x64-vs-ARM64 lib-test parity.
- Windows ARM64 full lane: PASS.

The final Windows live-menu parity failure was traced to a fixture readiness race: the fixture previously announced menu readiness before Windows had actually entered the nested menu loop, allowing sequential detectors to observe different live snapshots. The fixture now emits menu `UP` / `DOWN` from `WM_ENTERMENULOOP` / `WM_EXITMENULOOP`. Production menu detection semantics, assertions, retry behavior, and timeout contracts were not weakened or inflated.

This closure tracking commit changes `main`; the resulting final-main SHA must pass CI, CodeQL, Supply Chain, and Release before P1A is called finally closed outside this document.

P1B/P1C/P2 remain PLANNED until that exact final-main closure verification passes.