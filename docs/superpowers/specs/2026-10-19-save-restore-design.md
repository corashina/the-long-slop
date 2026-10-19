# Save restore finalization

Date: 2026-10-19.
Status: Approved by the user on 2026-10-19.
Branch: `codex/codebase-architecture`.
Base: `97783a10444c9d5faef9d1c5a10b2ff21469d33c`.

## Approved result

A private `RestoreFinalizer` owns assembly finalization, mass recomputation, motion restoration, and query publication.
Separate entry points preserve the initial and deferred sequences.
`GarageRuntime` keeps session ownership. `PhysicsWorld` keeps physics ownership.
The finalizer borrows both owners' fields for one call.
It adds no retained state, frame allocation, or repeated setup during unchanged restore polls.

This is a refactor of current behavior.
The save schema, source identities, gameplay rules, and build `11140890` authority remain unchanged.
Source parity and browser behavior remain unverified.
The user approved this written specification. Implementation planning is authorized.

## Current evidence

All paths and lines below refer to the base commit.

- [Initial reconstruction](../../../src/starting_area/save.rs#L245) validates before candidate creation and restores item frames after assembly setup.
- [Initial finalization](../../../src/starting_area/save.rs#L355) synchronizes assembly state before mass, motion, wash colliders, and queries.
- [Deferred restore](../../../src/starting_area/save.rs#L35) waits for generated owners before it restores required links.
- [Deferred finalization](../../../src/starting_area/save.rs#L175) applies body records before unattached world motion overlays.
- [Pending state](../../../src/starting_area/save.rs#L17) distinguishes required restore from remaining deferred links.
- [Snapshot capture](../../../src/starting_area/save.rs#L694) preserves unresolved attachment records and excludes their temporary assembly body records.
- [Body validation](../../../src/save/validate.rs#L730) checks source identities and independent assembly roots.
- [Inactive body activation](../../../src/save/world.rs#L385) retains absolute poses and restores motion when bodies become active.
- [World motion restoration](../../../src/world/changes.rs#L573) restores pose, linear velocity, angular velocity, and sleep.
- [Physics edits](../../../src/vehicle/mutations.rs#L12) mark query changes through owner methods.
- [Candidate readiness](../../../src/presentation/save.rs#L360) requires deferred restore before publication.
- [Runtime publication](../../../src/app.rs#L852) advances deferred restore after source publication and before world activity synchronization.

## Private interface and ownership

`src/starting_area/save.rs` declares `mod finalize;`.
The child module exposes only its context and two entry points to its parent.
The fields below have `pub(super)` visibility. Other modules cannot construct the context.

```rust
pub(super) struct RestoreFinalizer<'a> {
	pub(super) area: &'a mut StartingArea,
	pub(super) physics: &'a mut PhysicsWorld,
	pub(super) wheels: &'a mut WheelAssemblies,
	pub(super) persistent: &'a mut PersistentWorld,
	pub(super) stream: &'a mut WorldStream,
}

impl RestoreFinalizer<'_> {
	pub(super) fn initial(&mut self, reference: &ReferenceManifest,
		ids: &IdentityMap, saved: &SaveSnapshot) -> Result<(), SaveError>;

	pub(super) fn deferred(&mut self, reference: &ReferenceManifest,
		ids: &IdentityMap, motion: &mut Vec<BodySnapshot>,
		unresolved: &[ItemSnapshot]) -> Result<(), SaveError>;
}

enum BodyLookup { Required, Available }
```

Private helpers recompute dynamic mass and restore body records.
The record helper takes an iterator of borrowed `BodySnapshot` values and a `BodyLookup` value.
`initial` selects `Required`. `deferred` selects `Available`.
The entry points contain fixed sequences. They do not construct an operation graph.

`PendingRestore` keeps its existing items, carry, inventory, identity map, ready flag, and motion records.
`PersistentWorld` keeps inactive poses, activation rules, and its existing synchronization behavior.
World change records remain in `WorldStream`.
Wheel and part owners keep assembly rules and collider changes.

## Initial caller and sequence

`from_save` retains validation, candidate creation, conditions, tools, item restoration, assembly restoration, and initial inventory selection.
It retains stream restoration, saved owner protection, origin setup, requests, and `PersistentWorld::restore`.
At the current first persistent synchronization, it constructs a borrowed finalizer and calls `initial`.

The initial entry point preserves this order:

1. Synchronize `PersistentWorld`.
2. Synchronize mechanisms.
3. Synchronize attachments with the existing `false` argument.
4. Synchronize compound colliders.
5. Synchronize parts.
6. Synchronize portable mass.
7. Synchronize wheels.
8. Reapply saved item frames with `items.restore_save`.
9. Synchronize item poses.
10. Recompute mass for every dynamic body.
11. Restore saved bodies, then vehicle bodies, with `Required` lookup.
12. Synchronize wash colliders.
13. Refresh physics queries.

The caller then retains player restoration, initial carry, equipment, consumables, vehicle state, lighting, final pose commit, and audio.
It retains pending restore creation, input clearing, and the paused session state.
The finalizer does not step physics.

## Deferred caller and sequence

`advance_saved_restore` retains owner checks and imported attachment restoration.
It retains required-distance checks, unchanged-ready early return, and saved-binding checks.
It binds generated identities before generated inventory, equipment, and carry restoration.
Unavailable generated identities and carry bodies retain their existing pending results.

The caller borrows disjoint runtime fields and pending fields directly.
It calls `deferred` after generated ownership restoration.
It does not remove `PendingRestore` from the runtime to work around borrowing.

The deferred entry point preserves this order:

1. Synchronize `PersistentWorld`.
2. Synchronize parts.
3. Refresh wheel conditions.
4. Synchronize wheels.
5. Synchronize item poses.
6. Commit poses with the existing `true` argument.
7. Recompute mass for every dynamic body.
8. Restore pending body records with `Available` lookup.
9. If motion records remain, restore unattached world change snapshots after those records.
10. Clear pending motion records.
11. Release saved item locations against unresolved attachment records.
12. Refresh physics queries.

The caller then clears pending carry and inventory and sets `ready`.
It removes `PendingRestore` only when no unresolved attachment records remain.
An unchanged ready poll returns before finalization.
A later owner can resolve an attachment after required restore becomes ready.
That pass does not replay cleared motion or overwrite motion from later gameplay.

## Lookup, errors, and publication

Source identity indexing retains its validated invariant in both phases.
Initial runtime source-body indexing remains strict.
Deferred runtime source-body lookup permits an absent handle.
Assembly identity lookup uses `IdentityMap::item` in both phases and propagates its error.
`Required` returns the current `ReconstructionFailure` for a missing assembly body.
`Available` skips a missing assembly item or body after identity resolution.
Both policies skip inactive body handles.

Mass recomputation precedes saved motion because assembly and mass changes can wake bodies.
Motion restoration writes linear velocity, then angular velocity, then sleep or wake state.
Deferred unattached world snapshots retain precedence over body records.
Attached world snapshots remain excluded from that overlay pass.
The finalizer retains tracked physics edit methods and explicit query refreshes.
`PersistentWorld::sync` can also refresh queries internally. Those refreshes remain unchanged.
Errors retain their type, message, propagation order, and existing partial mutations.
The refactor adds no rollback or new validation rules.

## File changes

- `src/starting_area/save/finalize.rs` contains the private context, phase entry points, lookup policy, and shared helpers.
- `src/starting_area/save.rs` replaces both finalization blocks with context calls and retains restore orchestration and capture.
- `src/starting_area/save/finalize/tests.rs` contains focused tests of the real finalizer and real physics state.
- `src/world/stream_tests.rs` retains full restore cases for generated owners and inventory.
- Existing query tests retain query comparison support. Wheel fixtures remain private and unchanged.

The old finalization blocks are deleted. There is no compatibility path or schema migration.

## Critical verification contract

1. An initial assembled and detached wheel restore preserves mass, motion, sleep, and query hits before any physics step.
2. Real absent assembly bodies fail under `Required` and skip under `Available`. Identity errors still propagate.
3. A generated owner delay blocks required restore. Binding restores ownership and publishes the resulting collider queries.
4. Unattached world motion overrides body records. Attached snapshots do not enter that pass. Repeated ready polls preserve later motion.
5. A distant unresolved attachment survives recapture and later binding without a duplicate body or stale query hit.

New focused test names use the `restore_finalization` prefix.
Tests call the real entry points and inspect bodies, colliders, items, and queries.
Existing runtime and stream preparation support supplies complete save scenarios. No new fixture framework is added.

The implementation check commands are:

```powershell
./scripts/check.ps1 -Filter restore_finalization
```

This command includes native tests, formatting, and the WASM compile check.
The two existing restore tests require installed source exports and are ignored by default.
Run them explicitly in the configured Docker toolchain when their exports are available:

```text
cargo test --locked --lib fixed_decoration_imported_inventory_reload_waits_for_real_readiness -- --ignored
cargo test --locked --lib roadside_possession_survives_owner_retirement_without_duplicate_spawn -- --ignored
```

`check.ps1 -Filter` cannot enable ignored tests. Report unavailable exports as a verification limit.
The test output must show the intended tests ran.
Full checks, release builds, and browser acceptance remain deferred without a separate request or agreed milestone.
No tests or builds run for this specification.

## Completion and rejected alternatives

Completion requires both callers to use the finalizer and all duplicated finalization blocks to be absent.
Critical tests must preserve both phase sequences, lookup policies, overlays, late owners, and repeated polls.
The schema and public runtime ownership must remain unchanged.
The finalizer adds no retained allocation or unchanged-poll work.
Runtime parity requires separate acceptance evidence.

An owned restore session would move candidate construction and capture state into a new lifecycle owner.
That design changes more ownership than this refactor requires.
A generic operation graph would encode two fixed sequences as runtime data and add unnecessary state.
A shared sequence would change phase-specific order or add initial work to deferred restore.
The approved finalizer preserves both sequences with one owner for their shared rules.
