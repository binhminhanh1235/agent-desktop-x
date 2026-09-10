# Agent Runtime Optimization Plan

Status: ARO-P0A DONE / VERIFIED
Branch: `feat/agent-runtime-optimization`
Verified baseline main: `72772258cca0471fed3eb8603eba0185eced55c2`
Verified baseline tree: `5c5f9aa475064e56783cb9c7d46aea1b7060ee2b`

## Mission

Evolve agent-desktop from a collection of desktop automation commands into a compact local execution runtime for AI agents and agent harnesses.

The external model should stay deliberately small:

- `observe`: understand only the state that matters.
- `execute`: run one atomic or compound semantic action with conditions and verification.
- `run`: execute a reusable skill/workflow, including durable work later.

Internally the runtime may use cached semantic knowledge, accessibility, native app protocols, MCP, APIs, events, and eventually vision. The caller should not need to micromanage those transports.

Core optimization loop:

```text
DISCOVER ONCE
    -> CACHE SEMANTICS
    -> EXECUTE MANY
    -> VERIFY ONCE
```

## Design principles

1. Cache semantic knowledge, never trust raw element instances as persistent identity.
2. Prefer scoped revalidation over full-tree rescans.
3. Prefer semantic compound actions over coordinate/script batches.
4. Keep the harness round-trip count small without hiding ambiguity or destructive risk.
5. Keep existing CLI, MCP tools, Skills, delivery semantics, and platform contracts backward compatible.
6. Complex inside, simple outside.
7. Every optimization must be measurable on cold, warm, and hot paths.
8. Native semantic interfaces win over accessibility when available; accessibility wins over vision.

## Success metrics

Track these per benchmark scenario:

- full-tree reads
- provider/accessibility calls
- time to resolve a target
- agent/harness round trips
- bytes/tokens returned to the harness
- stale-cache recovery rate
- false-positive semantic recovery rate
- action verification rate
- cold / warm / hot path latency

No fixed performance claim is accepted without benchmark evidence.

## P0A - Semantic AppProfile Cache

Goal: learn an application's stable semantic structure once, then resolve targets cheaply on later calls.

Deliver:

- `AppProfile` model separated from live element references.
- semantic selector recipe using stable signals such as app/process identity class, window signature, automation/accessibility id, role/control type, normalized name, ancestor path, and supported actions.
- live ref cache with generation/lifetime boundaries.
- semantic cache that can survive navigation and process restart when its selector remains valid.
- scoped revalidation.
- cache invalidation hooks for process/window generation changes.
- diagnostics that explain cache hit, miss, stale, revalidated, and fallback behavior.
- benchmark counters for cold vs warm target resolution.

Must not:

- persist raw UIA/AX/AT-SPI runtime handles as durable identity.
- silently select an ambiguous mutating target.
- change existing stale-ref or delivery-semantics guarantees.

Acceptance:

- first resolution may scan a scope and produce a selector recipe.
- repeated resolution of the same stable target uses fewer provider/tree reads.
- process restart invalidates live refs but permits semantic re-resolution.
- moved element remains resolvable if semantic identity is stable.
- duplicate semantic candidates refuse rather than guess.
- cache corruption/mismatch fails closed.
- Linux/macOS/Windows contracts remain compatible.

Implementation checkpoint for the production-real P0A vertical slice:

- integration seam: core read-only live `find`; mutating ref actions keep the existing strict resolution and delivery-semantics path unchanged.
- cache lifetime: bounded process-local typed memory only, capped at 256 profiles; no SQLite or durable selector persistence in P0A.
- live cache: generation-bound `RefEntry` metadata only. `NativeHandle` and raw runtime handles are never stored as durable identity.
- durable identifier admission: AutomationId, AXIdentifier, and AXDOMIdentifier may participate; RuntimeId is explicitly excluded.
- semantic recipe: canonical role, normalized stable text, semantic ancestor labels, app/window metadata, supported actions, and stable identifiers when available. Bounds and child-index path remain transient live-ref evidence.
- window recreation: app + surface are the hard semantic scope; current pid/process-instance/window id form the live generation. Window title is retained as profile metadata but does not block equivalent-window re-resolution after recreation.
- warm priority: valid generation-bound live ref -> stable identifier -> semantic path -> role/type + normalized text -> bounded one-edit fuzzy candidate -> scoped cold window discovery.
- fuzzy/scoped recovery: candidate enumeration is capped at 16 and ambiguity fails closed.
- cache admission is intentionally narrow for this slice: exact identity queries with name, description, native id, or value; broad role-only/state/containment/text queries keep the existing cold path so outward `find` semantics do not narrow silently.
- machine-readable trace event `app_profile.resolve` reports `live_hit`, `semantic_hit`, `revalidated`, `cache_miss`, `ambiguous`, or `invalidated`, plus resolution/provider/tree and cold/warm counters.
- focused tests cover first-learn/warm reuse, reduced tree work, moved bounds, process+window recreation, duplicate refusal, corrupted cache, and identity mismatch.


Final ARO-P0A verification evidence:

- final branch head: `72da253b00e498215a32a15034877d32aa30474e`
- final branch tree: `3ba6283b8cbdfcf98f215a5f2cc9c6459a9ab7e6`
- exact-head verification used a temporary `master` ref pointing at the final branch head because the repository workflows accept push on `main`/`master`; runner checkout logs confirm the exact commit rather than a PR merge ref.
- exact-head CI #49 / run `34377740049`: PASS
- exact-head CodeQL #49 / run `34377740077`: PASS
- exact-head Supply Chain #49 / run `34377740102`: PASS
- PR #11: guarded squash merge with expected head `72da253b00e498215a32a15034877d32aa30474e`
- merge commit: `69d071450f10780779ff327c74382748df99d306`
- merge tree: `3ba6283b8cbdfcf98f215a5f2cc9c6459a9ab7e6`
- post-merge CI #50 / run `34379682644`: PASS
- post-merge CodeQL #50 / run `34379682651`: PASS
- post-merge Supply Chain #50 / run `34379682629`: PASS
- post-merge Release #12 / run `34379682645`: PASS
- ARO-P0B and ARO-P0C remain PLANNED and were not started.

## P0B - Compound Execution Engine

Goal: let a harness send a semantic action group in one call.

Execution model:

```text
ACTION
  + CONDITION
  + WAIT
  + VERIFICATION
  + BOUNDED RECOVERY
```

Deliver:

- typed compound plan.
- ordered steps.
- stop-on-error.
- per-step and whole-plan deadlines.
- semantic targets, not coordinate-only batches.
- event-aware waits where available.
- final verification.
- compact execution trace.
- no replay of already delivered mutating actions after an uncertain failure.

Acceptance:

- a form-fill/search/export style flow executes in one runtime call.
- failure identifies exact step and delivery semantics.
- timeout does not exceed caller budget.
- verification failure is distinct from transport success.
- destructive ambiguity refuses.

## P0C - Compact Agent API

Goal: make deep harness integration require only a tiny stable surface.

Add a high-level API alongside existing commands:

- `desktop.observe`
- `desktop.execute`
- `desktop.run`

Requirements:

- existing granular MCP tools remain supported.
- high-level calls use the same policy/preflight/dispatch safety path.
- schemas are deterministic and compact.
- results include provenance, confidence where applicable, verification state, and minimal relevant state changes.
- no giant accessibility-tree dump by default.

## P1A - View Handles and State Delta

Goal: avoid repeatedly returning full desktop state.

Deliver:

- short-lived `view_id` / view generation.
- scoped semantic snapshots.
- delta between views.
- relevant additions/removals/changes only.
- stale view revalidation.
- progressive observation query by scope/intent.

Acceptance:

- a navigation can return a small state delta instead of a full tree.
- stale views cannot authorize unsafe actions.
- result ordering is deterministic.

## P1B - Event Bus and Cache Invalidation

Goal: replace polling and blanket cache eviction with native event-driven invalidation.

Targets:

- process started/exited
- window opened/closed/recreated
- focus/navigation changes
- relevant element/property changes
- file/artifact completion where platform support permits

Acceptance:

- affected subtree/profile entries invalidate without clearing unrelated app knowledge.
- waits can consume events with bounded fallback polling where necessary.
- event loss cannot create false success.

## P1C - Verification, Recovery, and Safety

Goal: distinguish technical action delivery from business outcome.

Deliver:

- expected-state assertions.
- verification outcomes.
- bounded self-healing target re-resolution.
- confidence/ambiguity model.
- stronger threshold for mutating/destructive actions.
- recovery explanation and alternatives.

Acceptance:

- click delivered but validation dialog shown => action is not reported as business success.
- stale semantic target can heal only when confidence is sufficiently high.
- destructive low-confidence recovery refuses.

## P2A - Capability Discovery and Router

Goal: use the best available application interface automatically.

Priority:

1. native structured application capability
2. MCP / app-specific protocol
3. vendor API
4. browser semantic interface
5. accessibility
6. vision fallback

Deliver:

- capability registry.
- provenance.
- routing explanation.
- explicit fallback policy.
- seams for MCP client, Windows App Actions, Apple App Intents, Android/AppFunctions where applicable to the host/platform architecture.

The runtime must not reimplement vendor integrations when a reliable native protocol already exists.

## P2B - Learned Actions and Record-to-Skill

Goal: turn successful repeated semantic traces into reusable actions.

Deliver:

- semantic trace capture.
- parameter detection.
- compound-action proposal.
- deterministic skill/workflow representation.
- explicit review before promoting learned behavior to reusable skill.
- no persistent coordinate macros as the primary representation.

## P2C - Durable Jobs and MCP Tasks

Goal: allow long-running desktop work to survive harness/LLM disconnects.

Deliver:

- job id.
- step/attempt state.
- checkpoints.
- artifacts.
- resume semantics.
- MCP Tasks mapping where protocol support is available.
- no automatic replay across uncertain mutation boundaries.

## P3A - A2A Surface

Goal: let another agent delegate a desktop outcome rather than individual tool calls.

Expose:

- Agent Card/capabilities.
- task lifecycle.
- progress.
- artifacts.
- cancellation/resume.
- policy boundaries.

MCP remains the tool-level integration; A2A is outcome delegation.

## P3B - Vision Fallback

Goal: handle surfaces without usable semantic/native interfaces.

Vision is a fallback only.

Requirements:

- unified semantic candidate shape.
- confidence and provenance.
- no silent destructive action on low-confidence visual targets.
- no requirement for a separate Python runtime in the default distribution.

## Public API philosophy

Do not delete granular tools. They are important for compatibility and debugging.

Preferred harness path:

```text
observe -> execute -> run
```

Granular path remains available:

```text
snapshot / list / click / type / wait / ...
```

## Storage

Preferred default remains a single native executable.

Persistent runtime data may use an embedded local store such as SQLite under a single agent-desktop data root. Storage format must be versioned and recoverable. No helper daemon is required for the default path.

## CI / merge policy

For every vertical slice:

1. start from exact verified branch head.
2. implement the smallest coherent slice.
3. add unit/contract tests before broadening scope.
4. run/re-check exact-head CI.
5. require CodeQL and Supply Chain success before merge.
6. guarded merge using exact expected head SHA.
7. verify post-merge CI, CodeQL, Supply Chain, and Release on exact main head.
8. only then mark the tracking task DONE / VERIFIED.
