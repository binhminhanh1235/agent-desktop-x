# ARO-P1A — View Handles + State Delta

## Exact base

- branch: `feat/agent-runtime-optimization-p1a`
- base main SHA: `89bc5ff2fcd2e5d1db935a50d440e82221accbf2`
- base tree: `9892105cf0f29c26e472dd2a75323841074fcff7`
- dependencies: ARO-P0A / P0B / P0C DONE / VERIFIED

## Objective

Add short-lived, bounded, process-local observation view handles and deterministic state delta support to the compact MCP API, starting with the `list-windows` read surface.

## Core invariant

**A view is observation evidence, never authorization.**

A `view_id` must never become a raw UI handle, durable element identity, mutation capability, or bypass for semantic preflight, live target resolution, permission checks, delivery/no-replay semantics, or stale-ref/process-generation validation.

## Actual runtime path

The implementation must stay aligned with the verified runtime path:

`desktop.observe` → compact request decode → batch command parser → read-only command check → `execute_with_adapter` → granular dispatch → core `list_windows` → `PlatformAdapter::list_windows` → platform inventory.

`desktop.execute` / `desktop.run` remain on the existing P0B/P0C semantic compound path.

## Identity boundary

Windows `WindowInfo.id` is currently emitted as `w-<HWND>`. It is therefore runtime/native identity and must not be persisted inside a P1A view or used as the canonical delta key.

The initial `list-windows` canonical state must instead use bounded semantic/process-generation evidence. If a stable unique semantic key cannot be produced, the view path fails closed rather than inventing identity from array order, bounds, or native/runtime IDs.

## Initial vertical slice

- optional compact observe view request, backward compatible with existing requests;
- first view observation performs the normal fresh read, canonicalizes bounded state, stores it process-locally, and returns the normal result plus view metadata;
- subsequent observation with a previous view performs another fresh read, validates scope compatibility, creates a new view, and returns only deterministic `added` / `removed` / `changed` delta state;
- identical observations produce an empty delta;
- unchanged entities are omitted;
- store capacity, TTL, canonical entry count, canonical bytes, delta entry count, and delta bytes are bounded;
- eviction ordering is deterministic;
- unknown, expired, incompatible, and ambiguous views fail deterministically;
- no SQLite, durable storage, event bus, jobs, capability router, or P1B/P1C/P2 work.

## Required acceptance

1. first targeted observation creates a bounded `view_id`;
2. identical second observation returns deterministic empty delta;
3. one addition returns only that addition;
4. one removal returns only that removal;
5. one changed semantic entity returns only changed state;
6. ordering is deterministic where provider order is not contractual;
7. unknown view refuses deterministically;
8. expired view refuses explicitly;
9. incompatible scope cannot be diffed;
10. capacity eviction is deterministic;
11. canonical views contain no raw/native runtime handles as persistent identity;
12. stale view cannot authorize mutation;
13. mutation still traverses existing semantic preflight and delivery/no-replay path;
14. compact calls without view fields remain backward compatible;
15. existing granular MCP tools remain unchanged.

## Optimization evidence

Record at minimum full observation bytes, subsequent delta bytes, full entry count, delta entry count, omission of unchanged entities, and harness call count. Do not claim wall-clock or percentage latency improvement without measuring it.

Retain P0A evidence `tree_reads: 28 -> 0` only as P0A evidence, never as P1A performance evidence.

## Verification rule

Do not merge to `main` until exact-head CI, CodeQL, and Supply Chain pass. Merge with exact expected head SHA, then verify exact post-merge CI, CodeQL, Supply Chain, and Release. If a closure commit changes `main`, verify the final main SHA again before marking ARO-P1A DONE / VERIFIED.
