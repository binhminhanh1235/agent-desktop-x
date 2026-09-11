# ARO-P1B — Event Bus + Cache Invalidation

Status: IN PROGRESS

Working branch: `feat/agent-runtime-optimization-p1b`

Exact base:

- main: `ca848597c016b9eb032f5e08f70234a1167e76e1`
- tree: `f1d27ce10a5f70e6895df26b574e65d298d7810a`
- P0A/P0B/P0C/P1A: DONE / VERIFIED

## Objective

Add a small, typed, bounded, deterministic process-local event/invalidation substrate so semantic cache and short-lived P1A views can be invalidated earlier than TTL alone, while preserving generation-bound correctness and all existing mutation safety contracts.

This phase is not a general-purpose event platform. It must not implement P1C recovery/confidence, P2 routing/learned-actions/jobs, or vision fallback.

## Existing call-path constraints

- P0A semantic profile knowledge lives in core AppProfile cache; live refs are process/window-generation-bound and already have a shorter lifecycle than semantic selector knowledge.
- P1A views live in the compact MCP layer. `expected_view_id` must continue to perform a fresh provider observation before mutation; an event is never proof of freshness or authorization.
- Platform adapters already normalize wait polling into `SignalBaseline` (`AppInfo`, `WindowInfo`, process instance tokens). Core diffs those normalized snapshots rather than depending on Win32/macOS/Linux constants.
- P0B delivery/no-replay, coordinate-mutation safety, permissions, and timeout semantics are out of scope and must remain unchanged.

## Required design

### Typed invalidation model

Define a bounded internal semantic event model covering only invalidation-relevant lifecycle/state transitions, such as:

- process started/replaced/exited;
- window created/destroyed/generation changed;
- accessibility tree invalidated when a provider can make that claim reliably;
- provider/session reset where applicable.

Events must contain bounded semantic scope and generation data only. Raw/native handles and platform constants must never become durable semantic identity.

### Delivery

Use a process-local bounded queue/ring with deterministic ordering and overflow semantics. No unbounded listener registry, no background-thread fanout, and no hidden thread-per-consumer behavior.

Consumers use a monotonic cursor. If a consumer falls behind retained history, it must observe overflow explicitly and conservatively invalidate/revalidate instead of assuming state is fresh. Shutdown/drop behavior must be explicit and deterministic.

### P0A integration

- Process-generation invalidation removes/invalidates live refs for the affected process generation while retaining safe semantic selector knowledge.
- Window-generation invalidation removes only affected window-bound live state.
- Unrelated events must not flush unrelated cache state.
- Duplicate events must be idempotent; reordered/older generation events must never resurrect stale state.
- Overflow must conservatively invalidate live-ref reuse without deleting semantic knowledge unnecessarily.

### P1A integration

- A matching lifecycle/invalidation event makes the matching stored view unusable as fresh.
- Unrelated events leave unrelated views valid until their existing TTL/eviction/provider-observation rules say otherwise.
- Overflow conservatively invalidates view freshness.
- Unknown/expired/evicted semantics remain unchanged.
- `expected_view_id` remains fail-closed and still re-observes provider state.
- Event processing must not authorize mutation or bypass P0B/P0C preflight.

### Adapter boundary and fallback

Native/provider notifications, where present, are classified at the platform/provider boundary before entering core. Current normalized `SignalBaseline` polling is an acceptable bounded fallback for platforms/providers without reliable native lifecycle events; do not claim realtime guarantees for polling.

Unsupported sources must degrade safely and must not fabricate freshness.

## Required acceptance

1. process replacement invalidates correct live refs;
2. window generation change invalidates correct scope;
3. unrelated process/window event does not flush unrelated cache;
4. view scope receives matching invalidation;
5. duplicate events are idempotent;
6. reordered events fail safe;
7. queue overflow fails safe;
8. semantic selector knowledge is retained when safe;
9. raw handles never persist as semantic identity;
10. unsupported event source degrades safely;
11. P0A/P0B/P0C/P1A regressions remain green, including P1A 18/18 acceptance;
12. no P1C recovery engine;
13. no P2 router/job/skill work;
14. Windows x64 + ARM64 paths green;
15. deterministic optimization/invalidation metrics are recorded honestly.

## Evidence

Record deterministic counters such as:

- events published/applied;
- queue current/max depth and dropped/overflow counts;
- cache live entries invalidated per scoped event;
- unrelated cache entries retained;
- views invalidated per scoped event;
- unrelated views retained.

Do not claim wall-clock or percentage speed-up without a measured benchmark.

## Verification discipline

Before merge, record exact branch HEAD/tree and require CI + CodeQL + Supply Chain PASS on that exact head. Merge only by guarded squash using the exact expected head SHA.

After merge, record exact main SHA/tree and require CI + CodeQL + Supply Chain + Release PASS. If a closure documentation commit changes main, repeat all four gates on that exact closure SHA before marking P1B DONE / VERIFIED.

On a failure, inspect the exact failed job, step, and log; fix the narrow root cause. Do not blind-rerun, inflate timeouts, or weaken assertions.
