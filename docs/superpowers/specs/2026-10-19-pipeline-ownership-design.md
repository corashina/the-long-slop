# Pipeline ownership

Date: 2026-10-19.
Status: Written specification awaiting user review.
Branch: `codex/codebase-architecture`.
Base: `97783a10444c9d5faef9d1c5a10b2ff21469d33c`.

## Intent and scope

Give the existing `PipelineOwners` module one complete candidate pipeline lifecycle.
Preserve current behavior and the complete Lada garage scope.
The user approved the design in chat. Written approval is required before an implementation plan or product changes.
The stated base is authoritative. Changes on `master` are outside this work.

## Current ownership

`world/pipelines.rs` admits work, specializes bases, claims candidates, and reclaims active families.
`surface_pass.rs` and `caster.rs` open guards, build descriptors, queue companions, and register owners.
`pipeline_ownership.rs` retains owners, prepares held IDs, retires cancellation, and releases promotion.
These callers currently share lifecycle policy.

An existing active specialization hit can have no candidate owner.
Candidate ownership never grants exclusive ownership of an accepted shared ID.

## Selected design

Expand `src/presentation/pipeline_ownership.rs`. Keep the `PipelineOwners` name.
Keep descriptor edits, discovery cursors, classification, and draw lookup tables in the pass modules.
Keep generic descriptor admission and cache operations in the existing vendor patch.
The owner controls guards, specialization dispatch, claims, companion registration, isolation, preparation, cancellation, active reuse, and promotion.

Use these product interfaces. All signatures below are new or changed interfaces.

```rust
pub(crate) enum PipelineUse<'a> {
	Active,
	Candidate(&'a GpuPreparation),
}

pub(crate) fn specialize_base(
	world: &mut World,
	usage: PipelineUse<'_>,
	specialize: impl FnOnce(&mut World)
		-> Result<CachedRenderPipelineId, ReferenceError>,
) -> Result<Option<CachedRenderPipelineId>, ReferenceError>;

impl PipelineOwners {
	pub(crate) fn queue_companion(
		&self,
		cache: &PipelineCache,
		base: CachedRenderPipelineId,
		build: impl FnOnce() -> Option<RenderPipelineDescriptor>,
	) -> Result<Option<CachedRenderPipelineId>, ReferenceError>;

	pub(crate) fn isolated(&self, id: CachedRenderPipelineId) -> bool;
	pub(crate) fn base_isolated(&self, base: CachedRenderPipelineId) -> bool;
	pub(crate) fn cancelled(&self, base: CachedRenderPipelineId) -> bool;
	pub(crate) fn fail(&self, base: CachedRenderPipelineId, message: String) -> bool;
}

pub(crate) fn refresh_isolation(world: &mut World);
pub(crate) fn prepare(world: &mut World);
```

For base specialization, `None` means candidate cancellation prevented the operation. The caller stops that job.
For companion creation, `None` also covers an omitted source pass. The caller records no companion ID.
`Err` preserves the current failure route and message. It never reports cancellation as a shader failure.
`fail` records a candidate failure and returns `true`. It returns `false` when the active status must receive the failure.
Base callers map Bevy specialization errors to the current `ReferenceError` message inside the closure.
The closure still selects `base_specialize` or the material's `prepass_specialize` function.

Raw `queue_scope`, `claim`, `companion`, `owner`, and `reclaim_active` become private after caller migration.
No caller can queue a candidate companion without ownership registration.

## Operation order

Base specialization follows this order.

1. Check candidate cancellation before acquiring a guard or invoking the closure.
2. For candidates, acquire the existing one-slot guard before invoking specialization.
3. Invoke the specialization closure once. Active work uses the normal cache queue.
4. On a candidate result, claim only a held ID. Do not hold an accepted active-cache hit.
5. On a new claim, transfer all registered companions and release the previous live owner's counts once.
6. Reset retired family entries for the new owner. Retain each new live entry once.
7. Revisit both pass discovery tables when the claim changes.
8. Record base isolation when the family remains owned.
9. For active work, reclaim the complete family through the existing cache release operations.
10. Revisit both pass discovery tables after active reclaim.

Keep the guard alive through the operation. Each guard contains at most one queue call.
A specialization cache hit consumes no descriptor slot.
A guard failure prevents specialization or descriptor construction.

Companion creation first resolves the base owner and rejects cancellation.
For an owned base, acquire the guard before the descriptor builder runs.
Run the descriptor builder only after that guard succeeds.
The builder preserves descriptor cloning and source pass checks in their current order.
It can return `None`, as transparent camera-depth creation does today.
The guard still precedes this optional-pass decision. A capacity error retains its current priority.
For a descriptor, queue, hold, and register the companion in that order.
Retain its owner once. Replace any prior live companion count once.
For an active base, build and queue normally without a candidate guard or hold.
Do not change the existing pass discovery order or optional pass decisions.

## Isolation and lifecycle

Store the current base isolation set inside `PipelineOwners`.
Remove the separate `PipelineIsolation` resource and per-pass isolation copies.
`refresh_isolation` retains the current compile-stage timing and removes IDs owned by the active bank.
New candidate claims add base isolation at the current specialization point.
Pass readiness reads `base_isolated`. Global shader readiness reads `isolated` for every owned or retired ID.
Both pass types expose `ready(&self, cache: &PipelineCache, owners: &PipelineOwners) -> bool`.
Existing draw lookup methods remain in the pass types.
Do not create a vector or retain a new isolation snapshot each frame.

Cancellation retires every live held ID and releases its owner count once.
Retired entries retain stable descriptor IDs and remain isolated.
Repeated cancellation cannot release the same count again.
A later candidate can claim the retained family. Active reuse can reclaim it.
Neither path releases a retired count twice.

Promotion releases only IDs whose real cache pipeline is ready.
Keep the assertion for an incomplete promoted ID.
Successful release removes the ownership entry and releases its live count once.
Preparation stays synchronous per held ID inside that candidate's GPU scope.
Preserve the current preparation gate, `owner.ready().is_ok()`.
An owner error blocks preparation. Outstanding error scopes alone do not block another held ID.
Held and retired IDs remain excluded from automatic processing, including shader reload retries.

## Borrow and schedule rules

The vendor queue guard owns an `Arc`. It does not retain a cache borrow.
Acquire the guard through a short resource borrow before passing `&mut World` to specialization.
Clone the candidate owner and required material inputs before entering the wrapper.
Do not capture a resource reference from `World` in the specialization closure.
Use stack closures. Do not store closures or create boxed callbacks.
Use `World::resource_scope` for ownership plus mutable cache access during active reclaim and preparation.
Do not hold the ownership mutex while the specialization closure runs.
Companion builders can borrow published descriptors because their cache access is shared.

Preserve `surface_pass::prepare_surface_passes` before `caster::prepare_passes` in `RenderSystems::Prepare`.
Preserve compile after `PrepareBindGroups`.
Preserve ownership preparation after compile and before `RenderSystems::Render`.

## Readiness and physical drain

Keep base publication, base compilation, companion compilation, and caster classification as separate conditions.
Keep failure reporting separate from physical drain.
A failure flag cannot release descriptor staging, counted ownership, or a pending submission receipt.
Cancellation waits for owner drain before the batch receipt can complete.
Keep the existing submitted-work callback and batch accounting.

Readiness code consumes ownership and pass outcomes. It never specializes, queues, claims, holds, or acquires pipelines.
Pass lookup tables retain their existing IDs and source pass decisions.
The renderer readiness work can change aggregation, but cannot weaken these family conditions.

## Files and ownership

| File | Responsibility |
| --- | --- |
| `src/presentation/pipeline_ownership.rs` | Concrete lifecycle entry points, family entries, isolation, and Bevy cache operations. |
| `src/world/pipeline_ownership.rs` | Small native-readable held-entry transition policy used by the Bevy owner. |
| `src/world.rs` | Declare the transition policy module. |
| `src/presentation/world/pipelines.rs` | Admission, material keys, batch dispatch, receipts, and family readiness checks. |
| `src/presentation/surface_pass.rs` | Additive descriptor edits, discovery, readiness, and draw lookup. |
| `src/presentation/caster.rs` | Camera depth and shadow descriptor edits, classification, readiness, and draw lookup. |
| `src/presentation/lighting.rs` | Read full ID isolation through the owner. |
| `src/presentation/frame_graph.rs` and `src/presentation/surface_graph.rs` | Pass the owner to pass readiness queries. |
| `src/presentation/source_lighting.rs` | Preserve pass registration and schedule order. |
| `vendor/bevy_render` | Unchanged generic cache mechanisms and documented ownership contract. |

Keep current cache limits, persistent cache reserve admission, descriptor staging charges, and batch bounds.
Update direct callers together. Remove obsolete resources and entry points without compatibility paths.

## Verification contract

The transition policy uses these exact types and function.

```rust
pub(crate) enum HeldEntryPhase { Held, Retired }
pub(crate) enum HeldEntryAction { Wait, Prepare, Retire, Release }
pub(crate) struct HeldEntryObservation {
	pub cancelled: bool,
	pub isolated: bool,
	pub pipeline_ready: bool,
	pub preparation_valid: bool,
}
pub(crate) fn held_entry_step(
	phase: &mut HeldEntryPhase,
	observation: HeldEntryObservation,
) -> HeldEntryAction;
```

Retired entries return `Wait`. Cancellation changes `Held` to `Retired` and returns `Retire`.
Promotion asserts pipeline readiness and returns `Release`. The Bevy caller then removes that entry.
An isolated incomplete ID returns `Prepare` when preparation is valid. All other observations return `Wait`.
The Bevy owner performs the selected cache operation. This is production policy, not a test model or cache abstraction.

Add four critical native tests against this policy.

1. Cancellation returns `Retire` once. Repeated processing of the retained entry returns `Wait`.
2. Cancellation takes priority over promotion, readiness, and preparation.
3. Promotion releases a ready ID. An incomplete promoted ID triggers the invariant assertion.
4. Preparation respects readiness and owner errors. An error-free pending scope still permits preparation.

Run `scripts/check.ps1 -Filter world::pipeline_ownership::tests` for the new policy and WASM compilation.
Run `scripts/check.ps1 -Filter world::renderer_pending::tests` for existing accounting and receipt contracts.
Native policy tests prove new transition decisions. Existing tests prove accounting, stale ownership, batch, and receipt rules.
WASM compilation proves that the real closures, guards, resources, and borrow rules type-check.
These checks do not execute the real Bevy cache, GPU scopes, or render schedule.

Real specialization guards, family claim transfer, active reuse, shader reload exclusion, and GPU completion remain unverified at runtime.
Companion discovery, real retain counts, and readiness across frames also remain unverified at runtime.
Browser acceptance, full checks, and release builds are not authorized by this specification.

## Alternatives and completion

A complete family registry would also move pass IDs, classification, and readiness into ownership records.
Reject that approach because current pass modules already own those source pass decisions and draw lookups.
Do not add a generic cache interface or change vendor mechanisms without a concrete missing capability.

Implementation is complete when all lifecycle callers use the concrete APIs and obsolete policy paths are removed.
Targeted checks must pass. The report must retain the stated runtime verification limits.
Written specification approval authorizes the next planning stage, not browser acceptance or integration into `master`.

## Source evidence

- `docs/roadmap.md:3-15` defines garage scope and open browser acceptance.
- `src/presentation/world/pipelines.rs:244-277,602-686,704-800` defines accounting, claims, readiness, and physical drain.
- `src/presentation/pipeline_ownership.rs:31-110,116-149` defines transfer, active reclaim, cancellation, promotion, and preparation.
- `src/presentation/surface_pass.rs:114-157` and `src/presentation/caster.rs:230-303` define companion guards and descriptor construction.
- `src/presentation/source_lighting.rs:507-512` and `src/presentation/world/pipelines.rs:104-110` define the current schedule.
- `vendor/bevy_render/SOURCE_PATCH.md:12-19` defines the product and vendor boundary.
- `vendor/bevy_render/src/render_resource/pipeline_cache.rs:229-235,277-293,736-785` defines guard lifetime, admission, holds, retirement, and release.
- `src/lib.rs:29-30` and `Cargo.toml:19-32` limit presentation and Bevy dependencies to WASM.
