# ARO-P1C — Verification + Recovery + Safety

## Exact baseline

- Repository: `binhminhanh1235/agent-desktop-x`
- Base branch: `main`
- Base SHA: `59c96f916246ed34598a9a35a61255d1433f253a`
- Base tree: `08f356d7a040886f95f02790c4ffb54d57d4af47`
- Working branch: `feat/agent-runtime-optimization-p1c`
- Dependency state: ARO-P0A/P0B/P0C/P1A/P1B DONE / VERIFIED

## Objective

Distinguish action delivery from desired UI outcome and add bounded semantic recovery without introducing blind mutation replay or a parallel execution engine.

P1C extends the existing P0B semantic compound execution path. It reuses P0A semantic selector/re-resolution knowledge, existing delivery semantics, existing postcondition assertions, P1B invalidation signals, and current adapter boundaries.

## Existing execution seam

The canonical path is:

1. compact `desktop.execute` / `desktop.run` validates bounded semantic steps;
2. compact API constructs `Commands::Batch(... semantic: true ...)`;
3. `src/batch/execution.rs` prepares and executes each step;
4. the action is dispatched once through the existing dispatcher;
5. `apply_verification` executes a read-only postcondition after the action;
6. action delivery semantics are carried into verification failures;
7. existing post-action wait failures use `delivered_unverified`;
8. batch compound metadata currently states `mutation_replay: false`.

P1C must preserve this ownership. Recovery is a bounded phase inside this engine, not another executor.

## Hard safety invariants

- Never replay a mutating command after delivery is `delivered`, `delivered_unverified`, or otherwise uncertain.
- A mutation may be retried only when the failed attempt is provably `not_delivered` and recovery remains semantically unambiguous.
- Verification recovery may re-observe/re-resolve read-only state, but cannot redispatch the mutation.
- Raw/native element or window handles never become recovery identity.
- Moved-element recovery uses semantic re-resolution, never historical coordinates/bounds.
- Ambiguous recovery fails closed.
- Permission/state drift never auto-grants permission and never converts uncertainty into retry safety.
- Recovery attempts are bounded and share the existing step/plan deadline.
- Legacy non-semantic batch behavior stays unchanged.

## P1C vertical slice

Implement bounded recovery for semantic compound steps with observable recovery metadata:

- postcondition/outcome verification remains authoritative;
- stale/not-found semantic target recovery is allowed only before delivery and only when semantic re-resolution is unique;
- focus/window drift may perform bounded read-only revalidation before a retry that is still provably not-delivered;
- verification failure after delivery may perform bounded read-only verification recovery but never mutation replay;
- permission/state drift degrades safely with recovery guidance, not privileged mutation;
- expose mutation-attempt count, recovery-attempt count, retry safety, recovery outcome/reason, and whether any mutation replay occurred.

Default maximum automatic recovery attempts for this slice: 1 per semantic step. Any later expansion requires a separate reviewed slice.

## Acceptance mapping

1. Successful mutation reports delivered + verified.
2. Verification failure after delivery is distinct from action not delivered.
3. Stale semantic ref can recover when an equivalent target is unambiguous and the failed mutation attempt was not delivered.
4. Moved element recovery uses semantic re-resolution rather than old bounds.
5. Ambiguous recovery refuses deterministically.
6. Delivered or delivery-uncertain non-idempotent mutation is never replayed.
7. Focus/window drift recovery is bounded, deadline-aware, and revalidates state.
8. Permission/state drift degrades safely without automatic permission escalation.
9. Mutation attempts, recovery attempts, recovery outcome, delivery, verification and retry safety are observable.
10. P0A/P0B/P0C/P1A/P1B regressions remain green; legacy non-semantic batch behavior remains compatible.
11. Windows x64 and ARM64 full lanes pass.

## Evidence requirements

Tests must prove both happy paths and refusal paths. Evidence must distinguish deterministic counters/contracts from measured wall-clock performance. Do not claim latency or percentage speed-up without a dedicated benchmark.

## Gate and merge discipline

Before merge:

- record exact feature SHA and tree;
- exact-head CI PASS;
- exact-head CodeQL PASS;
- exact-head Supply Chain PASS.

Merge only with a guarded squash merge against that exact feature head.

After merge:

- record exact `main` SHA and tree;
- exact post-merge CI, CodeQL, Supply Chain and Release PASS;
- update canonical tracking on `main` with final evidence;
- because closure docs change `main`, require CI, CodeQL, Supply Chain and Release to PASS again on the exact closure SHA before declaring DONE / VERIFIED.

Do not begin P2 work until this closure is complete.
