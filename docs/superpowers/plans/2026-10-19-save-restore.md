# Save Restore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give save restoration one finalization owner while preserving both availability phases and current saved behavior.

**Architecture:** A stack-borrowed `RestoreFinalizer` provides separate initial and deferred entry points. Private helpers own mass, body lookup, motion, and world overlays. Existing runtime, physics, stream, and assembly owners remain responsible for their state.

**Tech Stack:** Rust 2024, Rapier `0.36.0`, existing save types, PowerShell, and the existing Docker Rust toolchain.

**Spec:** [Approved save restore design](../specs/2026-10-19-save-restore-design.md), approved document commit `8ce8b0c`.

Date: 2026-10-19. Branch: `codex/codebase-architecture`.
Worktree: `C:/Users/Corashina/.codex/worktrees/codebase-architecture/the-long-drive-wasm`.
Plan base: `8ce8b0c`. Execute save, startup, terrain, pipeline, then readiness. Keep shared production edits serial.
Product source base: `97783a10444c9d5faef9d1c5a10b2ff21469d33c`.
Status: Implementation plan awaiting user review. Subagent-driven development is already selected. Do not implement before review.

## Global Constraints

- Separate entry points preserve the initial and deferred sequences.
- `GarageRuntime` keeps session ownership. `PhysicsWorld` keeps physics ownership.
- The finalizer borrows both owners' fields for one call.
- It adds no retained state, frame allocation, or repeated setup during unchanged restore polls.
- The save schema, source identities, gameplay rules, and build `11140890` authority remain unchanged.
- Errors retain their type, message, propagation order, and existing partial mutations.
- The refactor adds no rollback or new validation rules.
- `PendingRestore` keeps its existing fields. World changes remain in `WorldStream`.
- Wheel and part owners keep assembly rules and collider changes.
- Delete replaced code. Add no compatibility path, migration, operation graph, or dependency.
- Do not broaden the wheel fixture or construct an export-free full runtime fixture.
- Root alone commits. Workers return reviewed changes for root to commit.
- Keep the worktree unmerged. Do not merge changes from `master`.
- Full checks, release builds, and browser acceptance remain deferred without a separate request or agreed milestone.
- Source parity and browser behavior remain unverified.

## Review Focus

1. Assembly or mass changes wake saved sleeping bodies. Pin final sleep and query state in tests A and D.
2. Deferred bodies can be absent while identities remain required. Pin errors and inactive-body skipping in test B.
3. Changed-world motion must win once. Pin overlay filtering, record clearing, and subsequent motion in test C.
4. A generated owner can arrive after required restore. Pin pending gates, recapture, and late binding in test E.
5. Ready polls must preserve gameplay motion and published queries. Pin repeat polls in tests D and E.

## File map

| File | Responsibility |
| --- | --- |
| Create `src/starting_area/save/finalize.rs` | Borrowed context, fixed phase sequences, and private shared helpers. |
| Create `src/starting_area/save/finalize/tests.rs` | Three nonignored tests against production helpers. |
| Modify `src/starting_area/save.rs:175-243,355-432` | Replace both finalization blocks. Preserve surrounding orchestration and capture. |
| Modify `src/vehicle.rs` | Add one empty `cfg(test)` physics constructor for real helper tests. |
| Modify `src/interaction/items.rs` | Add one independent `cfg(test)` item-store constructor for real helper tests. |
| Modify `src/world/stream_tests.rs` | Add export-backed initial and delayed-owner tests using existing source preparation support. |

Existing wheel fixtures and query scan fixtures remain private and unchanged.
The two test constructors create real owner values. They are not a runtime fixture framework.

---

### Task 1: Centralize both restore finalization paths

**Files:** Use the complete file map above. This is one testable ownership change.

**Interfaces:**

The finalizer module imports existing types. The new interfaces are:

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
fn recompute_dynamic_mass(physics: &mut PhysicsWorld);
fn restore_body_records<'a>(records: impl Iterator<Item = &'a BodySnapshot>,
	lookup: BodyLookup, ids: &IdentityMap,
	bodies: &HashMap<BodyId, RigidBodyHandle>, items: &ItemStore,
	physics: &mut PhysicsWorld) -> Result<(), SaveError>;
fn restore_world_motion(motion: &mut Vec<BodySnapshot>, changes: &ChangeRecords,
	items: &ItemStore, physics: &mut PhysicsWorld);
```

`restore_world_motion` owns the existing nonempty-motion guard, unattached overlay loop, and motion clear.
It clears an empty vector too. It never applies overlays when that vector was empty.
`initial` passes saved body records followed by vehicle body records without collecting them.
`deferred` borrows pending motion without another record collection.

The test constructors have these exact signatures:

```rust
// src/vehicle.rs, within #[cfg(test)] impl PhysicsWorld
pub(crate) fn restore_finalization_fixture() -> Self;
// src/interaction/items.rs, within #[cfg(test)] impl ItemStore
pub(crate) fn restore_finalization_fixture(states: Vec<ItemState>) -> Self;
// src/world/stream_tests.rs, ignored export test support
fn generated_owner_fixture()
	-> (ReferenceManifest, SourceStamp, GarageRuntime, ObjectKey, ItemId);
```

The empty physics constructor initializes the same five fields as query tests' `empty_world` at `query_bounds/tests.rs:191`.
It sets `integration_parameters.dt` to `FIXED_DT`. It creates no car, controller, or renderer.
The item constructor initializes indices, names, body IDs, states, and `next_generated` consistently.
It initializes all other collections empty and uses `WorldOrigin::default()`.
Handle an empty state vector with `next_generated = 0`. Do not change the wheel constructor.
Unit tests construct `IdentityMap` through its public fields. Unused role maps remain empty.

`generated_owner_fixture` uses existing `reference()` and `roadside_stamp`. It asserts build `11140890`.
It creates a real generated roadside owner through the existing building selection and preparation route.
Reuse the source setup in `roadside_possession_survives_owner_retirement_without_duplicate_spawn` at `stream_tests.rs:2816`.
Keep that owner in the world. Attach the initial `OilCan` through the existing `content_bindings::restore_item` route.
Return the owner key and imported child ID. Do not make generated inventory or carry state for this fixture.
Use existing `prepare_ready`, `prepare_step_for_views`, `roadside_views`, and `move_player` inside `stream_tests.rs` only.
No helper becomes a production export to share test access.

- [ ] **RED: Add the test module and the two narrow test constructors.**

Declare `mod finalize;` in `save.rs` and `#[cfg(test)] mod tests;` in `finalize.rs`.
Add the three nonignored tests below. They call the named production helpers directly.
No test steps physics. Query checks use `PhysicsWorld::queries` and real collider handles.

A. `restore_finalization_mass_motion_sleep_and_queries` creates two dynamic bodies with independent collider roots.
Use unit-volume cuboids. Give one body additional mass `12.0` and collider density `3.0`.
Give the second body additional mass `3.0` and collider density `2.0`.
Put the roots at x `2.0` and `6.0`. Use initial item IDs `0` and `1`.
Call mass recomputation before `restore_body_records`. Then refresh queries through the real physics owner.
Test awake and sleeping snapshots. Sleeping snapshots contain zero velocities.
Pin these assertions, using each test's actual handles:

```rust
assert_eq!(physics.rapier().bodies[first].mass(), 15.0);
assert_eq!(physics.rapier().bodies[second].mass(), 5.0);
assert_eq!(physics.rapier().bodies[first].is_sleeping(), sleeping);
assert_eq!(physics.rapier().bodies[second].is_sleeping(), sleeping);
assert_eq!(physics.rapier().bodies[first].linvel().to_array(), linear);
assert_eq!(physics.rapier().bodies[first].angvel().to_array(), angular);
```

For awake records use linear `[2.0, 0.0, 1.0]` and angular `[0.0, 0.25, 0.0]`.
Cast vertical rays from y `10.0` with max distance `20.0` at both x positions.
Assert each hit equals its collider handle. Assert a ray at x `10.0` misses.
These assertions prove helper state only. Test D proves the initial finalizer sequence.

B. `restore_finalization_lookup_errors_and_inactive_bodies` exercises real records, maps, items, and bodies.
For an initial item mapped to a state with `body = None`, assert the existing required error:

```rust
assert!(matches!(required, Err(SaveError::ReconstructionFailure(ref s))
	if s == "Missing assembly body"));
assert!(available.is_ok());
assert!(matches!(unknown_identity, Err(SaveError::InvalidLinks(_))));
```

Test an absent assembly state with its item identity still present in `IdentityMap`.
Test a source identity present in `ids.bodies` but absent from the runtime body map under `Available`.
An unmapped initial identity must error under both policies. Do not turn identity failure into an absent body.
Insert one real body handle into `physics.inactive_bodies` and assert its velocity and sleep state remain unchanged.
Keep source-map indexing strict under `Required`. Do not replace its validated invariant with a fallback.

C. `restore_finalization_overlay_precedence_and_single_application` creates two imported item roots and real `ChangeRecords`.
Create records through `ChangeRecords::record`, with baseline velocities zero and changed linear velocity `[3.0, 0.0, 0.0]`.
Give the second snapshot an attachment. Set both body-record velocities to `[1.0, 0.0, 0.0]`.
Apply body records, then call `restore_world_motion` with nonempty motion.
Assert the unattached root has velocity `[3.0, 0.0, 0.0]` and the attached snapshot did not override its body record.
Assert `motion.is_empty()`.
Set the first body's velocity to `[7.0, 0.0, 0.0]` and call the overlay helper again.
Assert that velocity remains `[7.0, 0.0, 0.0]`.
Use a zero local pose and assert overlay position follows the saved item pose before query refresh.

- [ ] **RED: Run the focused native command and confirm the missing production helpers cause the failure.**

```powershell
./scripts/check.ps1 -Filter restore_finalization
```

Expected: compilation fails for the absent helper interfaces. A missing export is not the intended failure.

- [ ] **Implement the private helpers with the current lookup and motion rules.**

Move the dynamic-body mass loop and body-record logic from `save.rs:191-216,387-426` into the stated helpers.
Keep `PhysicsWorld::edit_bodies` and `edit_body`. Preserve body-record order and skip inactive handles.
Move the world overlay and clear from `save.rs:217-233` into `restore_world_motion`.
Keep `world::changes::restore_motion`, attachment exclusion, pose handling, and the nonempty-motion condition.

- [ ] **Implement `RestoreFinalizer::initial` and replace its caller block.**

At `save.rs:355`, keep `PersistentWorld::restore` in `from_save`.
Construct the finalizer from disjoint candidate fields. Replace only the following finalization block.
Preserve this exact sequence inside `initial`:

1. Persistent synchronization, mechanisms, attachments with `false`, and compound colliders.
2. Parts, portable mass, wheels, and saved item-frame restoration.
3. Item pose synchronization, dynamic mass recomputation, and required body-record motion.
4. Wash collider synchronization, then query refresh.

Keep all operations after the original query refresh in their current positions.
Player, initial carry, equipment, consumables, vehicle state, lighting, pose commit, audio, and pending creation stay in `from_save`.

- [ ] **Implement `RestoreFinalizer::deferred` and replace its caller block.**

Borrow disjoint runtime and pending fields. Do not take `PendingRestore` out of the runtime.
Keep owner availability, required-distance checks, unchanged-ready return, binding checks, inventory, equipment, and carry in the caller.
Preserve this exact sequence inside `deferred`:

1. Persistent synchronization, parts, wheel conditions, and wheels.
2. Item pose synchronization and pose commit with `true`.
3. Dynamic mass recomputation, available body-record motion, world overlays, and motion clear.
4. Saved location release against unresolved records, then query refresh.

Keep pending carry/inventory clearing, `ready = true`, and final pending removal after the call.
Delete the two old finalization blocks. Keep existing error order and partial mutations.
Do not step physics or move `PersistentWorld::sync` internal query refreshes.

- [ ] **GREEN: Run the focused native tests and WASM compile check.**

Run `./scripts/check.ps1 -Filter restore_finalization`.
Expected: tests A, B, and C run and pass. Formatting and WASM compilation pass.
A zero-test result fails this step.

- [ ] **Add export-backed tests D and E without adding a complete native fixture framework.**

D. Add ignored `restore_finalization_initial_wheel_roundtrip` in `stream_tests.rs` using existing `reference`, `roadside_stamp`, and `prepare_ready`.
Assert build `11140890`. Prepare a real runtime through the existing adapter.
Remove hubcaps through `PartAttachments::remove` if they cover the selected wheel. Use `WheelAssemblies::remove` on that slot.
Keep another wheel installed. Move the detached root x by `10.0`, then synchronize item poses before capture.
Capture with `capture_save`, serialize and deserialize, then load through `GarageRuntime::from_save`.
Before publication, compare root mass within `0.0001`, local poses, and collider ownership.
Inactive roots must retain saved pose, velocity, and sleep through `PersistentWorld::body_snapshot`, rather than their disabled physical bodies.
For active roots compare the real body snapshot. Use `capture_body_save` with the existing identity and origin.
Use awake velocity `[2.0, 0.0, 1.0]` and angular velocity `[0.0, 0.25, 0.0]`. Repeat with sleeping zero-velocity roots.
Prepare the loaded support, then call `advance_saved_restore`, without a physics step.
Assert active root motion and sleep match the save. Query restored collider positions with no assertion-side query refresh.
The old detached-root position must not produce that collider handle. Another source collider can occupy that position.

E. Add ignored `restore_finalization_delayed_owner_recapture` in `stream_tests.rs` using `generated_owner_fixture`.
Save while the generated owner and imported child remain distant from both restored player and vehicle positions.
Use actual pose moves and `capture_save`, rather than bypassing snapshot validation.
Load near the garage and poll existing preparation until required restore is ready, while the distant owner remains unresolved.
Assert `restore_pending()` is false and `has_deferred_save_links()` is true.
Recapture and assert the child's saved attachment is unchanged and its temporary assembly body record is absent.
Move both foci to the owner's saved destination. Advance existing source publication and restore until the owner binds.
Assert shared child/owner body ownership, one generated owner binding, attachment local pose, and collider query publication.
Set owner velocity `[7.0, 0.0, 0.0]` after restore. Repeat unchanged restore polls and assert the velocity remains unchanged.
Retain the existing required-owner pending assertions in `roadside_possession_survives_owner_retirement_without_duplicate_spawn`.
Both new tests use `#[ignore = "requires the installed source export"]` and require explicit execution below.

- [ ] **Run export-backed tests explicitly when the matching fixture exists. Report missing fixtures as deferred coverage.**

The worktree currently has no `web/assets/reference/manifest.json`.
An installed export is a source fixture, not permission to merge code or regenerate assets.
Use an existing export only after its checked manifest confirms build `11140890` and the required source recipes.
The following commands require the fixture under this worktree's `web/assets` directory.
If access uses another location, add a read-only Docker bind for that exact approved asset directory at `/workspace/web/assets`.
Do not copy another branch's product code or silently use a different export.

```powershell
$restoreRoot = (Get-Location).Path
if (!(Test-Path -LiteralPath "$restoreRoot/web/assets/reference/manifest.json")) {
	throw 'Save restore export tests require the installed source export.'
}
. ./scripts/cargo-target.ps1
$restoreTarget = Get-CargoTargetDirectory $restoreRoot
docker build --tag my-summer-slop-toolchain --file tools/Dockerfile $restoreRoot
docker run --rm --mount "type=bind,source=$restoreRoot,target=/workspace" `
	--mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
	--mount 'type=volume,source=tld-wasm-target,target=/target' `
	--env "CARGO_TARGET_DIR=$restoreTarget" my-summer-slop-toolchain `
	sh -c 'cargo test --locked --lib restore_finalization -- --ignored && cargo test --locked --lib fixed_decoration_imported_inventory_reload_waits_for_real_readiness -- --ignored && cargo test --locked --lib roadside_possession_survives_owner_retirement_without_duplicate_spawn -- --ignored'
```

Expected: D and E each run once, and both named existing restore tests run. All pass.
The normal focused filter skips these ignored tests. It cannot prove finalizer or full runtime coverage.
No browser, release, or full check belongs to these commands.

- [ ] **Run the focused native and WASM command again after adding the export-backed tests.**

Run `./scripts/check.ps1 -Filter restore_finalization`. A, B, and C must pass. D and E compile but remain ignored.

- [ ] **Review the final diff against both exact phase sequences and the five Review Focus cases.**

Confirm the schema is unchanged, motion clears once, query publication remains explicit, and capture retains late owners.
Confirm test constructors are `cfg(test)` and no wheel fixture visibility changed.
Record native results, WASM results, export test results, and all unavailable coverage separately.
Do not report full entry-point coverage when only A, B, and C ran.

- [ ] **Root commit after task review.**

Root stages only this task's changed files and creates `refactor: centralize save restore finalization`.
Workers do not stage or commit. Root records deferred export coverage in the review handoff if fixtures remain absent.
The branch remains unmerged. Implementation starts only after the user reviews this plan.
