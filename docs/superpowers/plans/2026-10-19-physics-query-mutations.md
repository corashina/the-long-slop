# Physics query mutation tracking implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking. Preserve the execution method selected by the user.

**Goal:** Replace full query-bound discovery scans with complete mutation tracking while preserving physics and query behavior.

**Architecture:** `PhysicsWorld` owns scoped mutation methods and reusable queues. `QueryBounds` processes affected handles and removals. Simulation movement joins that work before the existing post-step refresh.

**Tech Stack:** Rust 2024, Rapier 0.36.0, Parry 0.31.1 through Rapier, PowerShell, Docker, wasm32-unknown-unknown.

**Spec:** [Approved design](../specs/2026-10-19-physics-query-mutations-design.md), approved on 2026-10-19.

## Global constraints

- Work in `C:/Users/Corashina/.codex/worktrees/physics-query-mutations/the-long-drive-wasm`.
- Keep branch `codex/physics-query-mutations` unmerged.
- Implementation base is `8eca150d90e7d0fa216823276c01825f8b1189d1`. Specification commit is `a2c4544`.
- Steam build `11140890` remains the behavior authority.
- Keep query results, physics notifications, and refresh timing unchanged.
- No physics tuning, new gameplay, dependency fork, or save format change is included.
- Raw mutable access stays private to the vehicle module.
- No refresh consumes `take_modified` or `take_removed`.
- There is no full-scan fallback in production.
- The old scan remains only as a test oracle after replacement.
- Queue storage is reused. Avoid per-frame allocation and setup.
- Add only critical tests. Use source-independent native fixtures for mutation contracts.
- Run targeted native checks and a WASM compile check after Rust changes.
- Full checks, release builds, and browser acceptance require an explicit request or an agreed milestone under `AGENTS.md`.
- Reduced scan counts alone do not establish a performance gain.
- Use GPT-6.1 Sol with high reasoning for implementation and any authorized review. Never use GPT-6 Astra.

## Review focus

Each condition below has a named test in its owning task.

1. In-place shape edits and local attachment edits must change query bounds before simulation. Task 3 owns this test.
2. Remove, replace, and remove again in one slot must preserve the live generation. Task 3 owns this test.
3. Fixed-parent enablement must survive an intervening refresh before Rapier processes notifications. Task 3 owns this test.
4. Waking, sleeping, kinematic motion, and controller impulses must retain contacts and post-step queries. Task 3 owns these tests.
5. Rebase, retirement, and world replacement must not reuse pending handles from the previous world. Task 4 owns these tests.

## File responsibilities

| File | Change |
| --- | --- |
| `src/vehicle.rs` | Integrate mutation ownership, constructor seeding, simulation tracking, and private controller adapters. |
| `src/vehicle/mutations.rs` | Add scoped edits, lifecycle operations, joint operations, and bulk edits. |
| `src/vehicle/query_bounds.rs` | Own queues, generation markers, incremental BVH updates, and retained-byte accounting. |
| `src/vehicle/query_bounds/reference.rs` | Keep the previous scan under `cfg(test)` as `ScanBounds`. |
| `src/vehicle/query_bounds/tests.rs` | Hold existing tests, the shared query comparison fixture, and critical mutation tests. |
| `src/player.rs` | Migrate edits and use the character impulse adapter. |
| `src/interaction/{attachments,carry,cleaning,equipment,mechanisms,parts,push,storage,targets,wheels}.rs` | Migrate body, collider, parent, mass, and joint edits. |
| `src/starting_area.rs` | Migrate construction, compound shapes, mass, and activity changes. |
| `src/starting_area/{inventory,recovery,save,survival}.rs` | Migrate equipment, recovery, restoration, and object lifecycle changes. |
| `src/save/world.rs` | Migrate inactivity, recovery, and restored body motion. |
| `src/world/{changes,content_bindings,content_containers,content_lamps,content_mechanisms,content_parts,content_slots,content_wheels,instances,stream,stream_tests}.rs` | Migrate publication, retirement, mechanisms, metadata, activity, and test edits. |
| `docs/performance/2026-10-19-physics-query-mutations.md` | Record actual checks, limitations, and any separately authorized measurements. |
| `docs/roadmap.md` | Update item 10 from recorded evidence. |

Brace lists identify individual files, not new aggregate modules.
No dependency, web, save schema, or build script change is planned.

## Check commands

Run commands from the worktree root. Use `scripts/check.ps1` for the configured Docker toolchain.
It runs formatting, the requested native tests, and the WASM compile check.

```powershell
./scripts/check.ps1 -Filter query_bounds
```

Expected result is exit code 0, executed native tests passing, and a successful WASM library check.
Check that the filter executes the intended tests. Zero executed tests is not acceptance.
Do not repeat a successful check without another change or an unresolved concern.

## Task 1: Preserve the scan and establish query comparisons

**Files:** Modify `src/vehicle/query_bounds.rs`. Create `src/vehicle/query_bounds/reference.rs` and `src/vehicle/query_bounds/tests.rs`.

**Interfaces:**

- Consume the current `QueryBounds::refresh(&mut self, &mut rapier3d::prelude::PhysicsWorld) -> usize`.
- Produce test-only `ScanBounds`, including its `bvh: Bvh`, with `Default` and the same `refresh` signature.
- Produce test helpers `empty_world() -> crate::vehicle::PhysicsWorld` and `assert_matches_scan(world: &mut crate::vehicle::PhysicsWorld, scan: &mut ScanBounds, samples: &[Vector])`.
- Produce `ray_hit(world: &crate::vehicle::PhysicsWorld, x: f32, z: f32) -> Option<ColliderHandle>`.

- [ ] **Step 1: Record a clean baseline.**

Run `git status --short --branch` and `./scripts/check.ps1 -Filter query_bounds`.
Expect the current four query-bound tests to pass. Preserve unrelated changes and record any baseline failure.

- [ ] **Step 2: Move the existing tests and copy the reference scan.**

Move the current inline tests into `tests.rs` without changing their assertions.
Copy the pre-change index algorithm and state into `reference.rs` as `ScanBounds`.
Compile the reference module only under `cfg(test)`.
Keep the production implementation unchanged during this task.

- [ ] **Step 3: Add `scan_comparison_covers_hits_overlaps_and_bounds`.**

Construct two separate, non-overlapping balls and one attached box in `empty_world`.
Use sample points at the old positions, the new positions, and empty space.
The comparison helper refreshes the candidate first and the scan second on the same world.
It then compares vertical ray casts, downward ball casts, shape overlap membership, and enabled leaf AABBs.
Use a ball radius of `0.25`, ray distance `20.0`, and sample origins `10.0` units above each point.
Sort overlap handles by raw parts. Avoid equal-distance nearest-hit ties.
Read leaves through the pinned Parry API `Bvh::leaf_node(slot)` and `BvhNode::aabb()`.
Assert that both indexes contain exactly the live, effectively enabled collider generations.

```rust
assert_eq!(ray_hit(&world, 0.0, 0.0), Some(first));
assert_eq!(ray_hit(&world, 8.0, 0.0), Some(second));
assert_eq!(ray_hit(&world, 20.0, 0.0), None);
assert_matches_scan(&mut world, &mut scan, &samples);
```

These are baseline characterization checks. They are expected to pass before optimization.
Do not manufacture a failing implementation for this extraction.

- [ ] **Step 4: Verify and commit.**

Run `./scripts/check.ps1 -Filter query_bounds` and `git diff --check`.
Expect the four preserved tests and the new comparison test to pass.
Stage the three task files. Commit as `Preserve query scan as a test oracle`.

## Task 2: Enforce tracked mutation access and migrate every caller

**Files:** Create `src/vehicle/mutations.rs`. Modify all source files in the file table and the query-bound tests. Leave documentation changes for Task 4.

**Interfaces:**

Use Rapier's `RigidBody`, `Collider`, `GenericJoint`, `ImpulseJoint`, and associated handle types.
Add these owner methods in `mutations.rs`:

```rust
pub fn edit_body<R>(&mut self, handle: RigidBodyHandle, edit: impl FnOnce(&mut RigidBody) -> R) -> R;
pub fn try_edit_body<R>(&mut self, handle: RigidBodyHandle, edit: impl FnOnce(&mut RigidBody) -> R) -> Option<R>;
pub fn edit_collider<R>(&mut self, handle: ColliderHandle, edit: impl FnOnce(&mut Collider) -> R) -> R;
pub fn try_edit_collider<R>(&mut self, handle: ColliderHandle, edit: impl FnOnce(&mut Collider) -> R) -> Option<R>;
pub fn insert_body(&mut self, body: impl Into<RigidBody>) -> RigidBodyHandle;
pub fn insert(&mut self, body: impl Into<RigidBody>, collider: impl Into<Collider>) -> (RigidBodyHandle, ColliderHandle);
pub fn insert_collider(&mut self, collider: impl Into<Collider>) -> ColliderHandle;
pub fn insert_collider_with_parent(&mut self, collider: impl Into<Collider>, parent: RigidBodyHandle) -> ColliderHandle;
pub fn set_collider_parent(&mut self, collider: ColliderHandle, parent: Option<RigidBodyHandle>);
pub fn remove_body(&mut self, body: RigidBodyHandle) -> Option<RigidBody>;
pub fn recompute_body_mass(&mut self, body: RigidBodyHandle);
pub fn edit_bodies(&mut self, edit: impl FnMut(RigidBodyHandle, &mut RigidBody, &ColliderSet));
pub fn edit_colliders(&mut self, edit: impl FnMut(ColliderHandle, &mut Collider));
pub fn insert_impulse_joint(&mut self, first: RigidBodyHandle, second: RigidBodyHandle, joint: impl Into<GenericJoint>, wake_up: bool) -> ImpulseJointHandle;
pub fn remove_impulse_joint(&mut self, joint: ImpulseJointHandle, wake_up: bool) -> Option<ImpulseJoint>;
```

Keep the existing `remove_collider(&mut self, ColliderHandle) -> ()` contract, including wheel-support wake-up and immediate refresh.
Add `#[cfg(test)] pub(super) fn set_gravity(&mut self, gravity: Vector)` for the current zero-gravity tests.
Production uses the existing constructor setting.
Do not add unused integration-setting methods.

Add `PhysicsWorld::solve_character_collision_impulses(&mut self, controller: &KinematicCharacterController, dt: f32, filter: QueryFilter<'_>, shape: &dyn Shape, mass: f32, collisions: &[CharacterCollision]) -> ()`.
This adapter owns the temporary mutable query pipeline and calls the existing Rapier solver unchanged.

Add these `pub(super)` methods on `QueryBounds`:

```rust
fn mark_body(&mut self, body: RigidBodyHandle);
fn mark_collider(&mut self, collider: ColliderHandle, geometry_changed: bool);
fn retire_collider(&mut self, collider: ColliderHandle);
fn pending_counts(&self) -> (usize, usize, usize); // cfg(test): bodies, colliders, removals
fn step_counts(&self) -> (usize, usize);          // cfg(test): bodies, colliders
```

- [ ] **Step 1: Write `tracked_edits_cover_optional_handles_and_lifecycle`.**

Create a fixed parent and a child through the new insertion methods.
Run one refresh, then edit the same body and collider repeatedly.
Assert deduplicated pending counts and retained since-step mutation counts.
Remove the body. Assert its child handle is recorded for removal before Rapier deletes it.
Test invalid optional handles without calling either closure.

```rust
assert_eq!(world.try_edit_body(RigidBodyHandle::invalid(), |_| panic!("invalid body closure")), None::<()>);
assert_eq!(world.try_edit_collider(ColliderHandle::invalid(), |_| panic!("invalid collider closure")), None::<()>);
assert!(!world.rapier().colliders.contains(child));
```

Run `./scripts/check.ps1 -Filter query_bounds`.
Expect compilation to fail because the new owner methods do not exist yet.

- [ ] **Step 2: Add reusable mutation queues and the owner methods.**

Store pending query work and since-step explicit mutations separately within `QueryBounds`.
Use full handles, slot markers, and reused vectors. Upgrade an existing collider's geometry flag with logical OR.
Store removal handles separately. Clear markers through queued handles rather than scanning every allocated slot.
Mark valid objects before giving a closure mutable access.
Bulk edits mark each yielded object and preserve the existing iteration order.
Parent changes mark the collider, the old parent, and the new parent.
Body removal records attached colliders without allocating a temporary child list.
Keep existing failure behavior, refresh timing, mass updates, and joint wake-up arguments.
`edit*`, `insert*`, `set_collider_parent`, and `remove_body` enqueue changes without adding a refresh.
Existing mesh and box helpers retain their current refresh calls.
The current production scan continues to serve queries until Task 3 replaces it.
A refresh clears pending query work but retains since-step mutation queues.

- [ ] **Step 3: Migrate all callers and remove the old mutable methods in the same change.**

Use scoped edits for body and collider setters. Read needed values before entering an edit closure.
Use optional edit methods where current code uses `get_mut` or an optional handle.
Replace parenting, insertion, removal, mass recomputation, and joint access with their owner methods.
Replace raw mutable iteration with `edit_bodies` or `edit_colliders`.
Keep `set_collider_enabled` inside the tracked collider closure so its solver-group behavior survives.
Keep all current `refresh_queries` calls.
Replace the player's mutable query access with the character impulse adapter.
Audit direct writes inside `vehicle.rs`, constructor code, local aliases, and test setup.
Remove `rapier_mut` and public `queries_mut`. Do not leave a compatibility path.

Run `rg -n 'rapier_mut|queries_mut' src tests`.
Expect no matches and exit code 1. This is a structural check, not proof of complete tracking.
Review every remaining `QueryPipelineMut` construction as a private controller operation or test fixture.

- [ ] **Step 4: Verify and commit the complete caller migration.**

Run `./scripts/check.ps1 -Filter query_bounds` and `git diff --check`.
Expect all library callers to compile, critical native tests to pass, and the WASM check to pass.
Stage only the migration files listed above. Commit as `Track physics edits through scoped owner methods`.

## Task 3: Replace scanning with complete incremental refresh

**Files:** Modify `src/vehicle.rs`, `src/vehicle/mutations.rs`, `src/vehicle/query_bounds.rs`, and `src/vehicle/query_bounds/tests.rs`.

**Interfaces:**

- Consume Task 2's mutation methods and Task 1's comparison fixture.
- Keep `QueryBounds::refresh(&mut self, &mut rapier3d::prelude::PhysicsWorld) -> usize`.
- Add `QueryBounds::mark_simulated_body(&mut self, RigidBodyHandle)` for pending work only.
- Add `QueryBounds::requeue_step_mutations(&mut self)` and `QueryBounds::finish_step(&mut self)`.
- Add test-only `RefreshWork { bodies: usize, colliders: usize, aabbs: usize }` with equality and debug support.
- Count every inspected cached slot or live collider under `colliders`. Count actual AABB computations under `aabbs`.
- Add test-only `QueryBounds::last_work(&self) -> RefreshWork` and `queue_capacities(&self) -> Vec<usize>`.
- Include every production queue and marker capacity in `retained_bytes()`.

- [ ] **Step 1: Write the incremental-work and differential tests.**

Instrument the current scan with test-only work counters before changing its algorithm.
Add `idle_refresh_and_sparse_edits_visit_only_queued_handles`.
Create 1,024 standalone balls, separated by four units. Warm all queues before recording capacities.
A second unchanged refresh must perform zero body visits, collider visits, and AABB computations.
Edit one collider twice before refresh. Assert one collider visit and one AABB computation.
Assert that capacities stay equal across eight unchanged refreshes.

```rust
assert_eq!(world.query_bounds.last_work(), RefreshWork { bodies: 0, colliders: 0, aabbs: 0 });
assert_eq!(world.query_bounds.last_work().colliders, 1);
assert_eq!(world.query_bounds.last_work().aabbs, 1);
assert_eq!(before, world.query_bounds.queue_capacities());
```

Count actual loop visits and AABB calls. Do not report queue sizes as work while retaining hidden scans.

Add `in_place_shape_and_local_pose_edits_match_scan`.
Grow a ball from radius `1.0` to `3.0` through `shape_mut`, then move an attached box locally without moving its parent.
Compare the old location, the new location, and the enlarged shape edge before and after simulation.

Add `generation_reuse_does_not_remove_replacement`.
Perform removal, same-slot insertion, and another replacement before refresh.
Assert equal slot indices, distinct full handles, no old-location hit, and a hit on the final replacement.
Repeat through body removal after reparenting its collider.

Add `fixed_parent_enablement_survives_intervening_refresh`.
Disable and enable a fixed parent, with a refresh between each edit and step.
Also reparent a collider previously disabled by its old parent to an enabled fixed parent.
Compare against the scan after every boundary.

Add `simulation_motion_sleep_wake_and_kinematic_queries_match_scan`.
Exercise dynamic motion, a velocity-based kinematic body, a position-based kinematic target, sleeping, and waking through contact.
Assert a hit at the post-step pose without an extra caller refresh.
Keep the existing 240-step settling and active-contact assertions.

Add `private_controller_impulses_keep_post_step_queries_current`.
Populate one `VehicleBody` in the fixture with a minimal chassis and one ray-cast wheel.
Call `PhysicsWorld::apply_vehicle_forces(0, DriveInput::default(), DriveTorques::default())` while the wheel supports the chassis.
Use a separate character collision fixture to call the character impulse adapter.
Assert changed body velocity, retained contact behavior, and post-step equality with the scan.
Use primitive shapes and fixed values. Load no source export.

- [ ] **Step 2: Run the tests before replacing the scan.**

Run `./scripts/check.ps1 -Filter query_bounds`.
Expect the new work-count assertions to fail against the current full scans.
Differential correctness checks may already pass. Preserve that baseline rather than forcing unrelated failures.

- [ ] **Step 3: Implement incremental query refresh and simulation coverage.**

Seed every initial collider once before the constructor's first refresh.
Reuse existing insertion loops where possible. Any separate seed iteration runs only during construction.

Retain body-position propagation. Expand pending body handles into their attached collider handles.
Process removals before live updates. Check cached full handles before deleting a reused slot.
For each dirty collider, resolve the live generation and current attachment.
Recompute attachment world pose only when its cached attachment changed, matching the old scan.
Compute effective enablement from the collider and its parent.
Skip body-only work when cached pose, attachment, and effective enablement remain equal.
Recompute AABBs for geometry-marked colliders even when pose and shape identity appear unchanged.
Use the existing `Bvh::reinsert_or_update_with_change_detection` call and `0.0` margin.
Remove production global scans and retained shape references.

Before stepping, enqueue `islands.active_bodies()` through `mark_simulated_body`.
After stepping, enqueue active bodies again and requeue retained explicit mutations.
Refresh queries, then clear the since-step queues with `finish_step`.
Keep fixed-body and collider notification changes in that post-step reconsideration.
Do not consume Rapier notification lists or invoke collision detection during query refresh.

Audit the pinned pipeline's position, enablement, and collider update paths against these queues.
Give any uncovered write a precise tracking path before proceeding.
Do not add a periodic scan or weaken the differential assertions.

- [ ] **Step 4: Verify and commit.**

Run `./scripts/check.ps1 -Filter query_bounds` and `git diff --check`.
Expect all differential cases and work-count assertions to pass.
Review the production refresh for global body, collider, or cached-state discovery loops.
Stage the four task files. Commit as `Refresh physics query bounds from tracked mutations`.

## Task 4: Check gameplay mutation sequences and record acceptance limits

**Files:** Modify `src/vehicle/query_bounds/tests.rs` and `docs/roadmap.md`. Create `docs/performance/2026-10-19-physics-query-mutations.md`.

**Interfaces:** Consume the tracked owner API, comparison fixture, `PhysicsWorld::rebase`, `capture_body_save`, and `WorldOrigin`.
No new public runtime API is required.

- [ ] **Step 1: Add source-independent mutation replay tests.**

Add `wheel_door_part_and_retirement_sequences_match_scan`.
Replay the write order from `WheelAssemblies::place_part`, `MechanismState::sync`, and `PartAttachments` synchronization.
Use a loose dynamic body, a fixed parent, a local door pose, and primitive colliders.
Reparent the part, change its local pose, toggle enablement, recompute mass, detach it, and retire its body.
Compare queries after each current refresh boundary. Assert that the retired collider cannot be hit.
These tests cover the mutation contract. They do not establish complete gameplay acceptance.

Add `rebase_and_save_replacement_keep_query_state_local`.
Start with a body at `[12.0, 0.0, 0.0]` and a standalone collider at `[20.0, 0.0, 0.0]`.
Rebase by `[8.0, 0.0, 0.0]` and shift the `WorldOrigin` by the same value.
Capture a `BodySnapshot` through `capture_body_save`, serialize it, and deserialize it.
Reconstruct that body in a fresh `empty_world` through tracked insertion and edits.
Restore pose, velocity, and sleeping state in the current restoration order.
Keep unflushed removals in the old world while initializing the replacement.
Compare against a new scan and assert the following values:

```rust
assert_eq!(snapshot.pose.position, [12.0, 0.0, 0.0]);
assert_eq!(restored.rapier().bodies[restored_body].translation().x, 4.0);
assert_eq!(ray_hit(&world, 12.0, 0.0), Some(standalone));
assert_eq!(ray_hit(&restored, 4.0, 0.0), Some(restored_collider));
```

This checks body snapshot reconstruction and world ownership. It does not substitute for `GarageRuntime::from_save` acceptance.
Use existing ignored source-dependent inventory and streaming checks only when their assets are available and execution is authorized.
Report unavailable scenarios explicitly.

- [ ] **Step 2: Run targeted verification.**

Run `./scripts/check.ps1 -Filter query_bounds` and `git diff --check`.
Run `rg -n 'rapier_mut|queries_mut' src tests` and expect no matches.
Inspect all remaining direct writes inside the vehicle module against the tracking rules.
Do not run full checks, release builds, or browser acceptance in this plan's default execution.
If another integration target changed during implementation, run its relevant native filter through `scripts/check.ps1 -Test`.

- [ ] **Step 3: Record evidence and update item 10.**

Record exact commands, pass counts, ignored cases, failures, and deferred checks in the performance note.
Record retained-byte accounting and deterministic idle and sparse work counts.
Describe the conservative cost of collider edits that change only metadata.
Mark item 10 implemented only after the targeted checks pass.
State that runtime behavior and frame-time improvement remain unverified without the required measurements.
Keep the branch unmerged.

- [ ] **Step 4: Review the diff and commit.**

Run `git diff --check` and inspect the complete branch diff against the recorded base.
Confirm that obsolete mutable APIs and production scan discovery are absent.
Stage the test file and the two documentation files.
Commit as `Verify physics query mutation scenarios and record limits`.

## Deferred performance milestone

This plan does not authorize release builds, full checks, or browser acceptance.
At a separately approved milestone, use the roadmap's matched release A/B/A protocol.
Record package and asset hashes, viewport, DPR, build profile, and exclusive benchmark windows.
Report garage and driving average, p95, and p99 frame times separately.
Measure query preparation time, retained bytes, affected objects, and AABB computations.
Collect instrumented counts separately from uninstrumented frame times.
Verify real input, active physics, submitted frames, streaming, recovery, and Save/Load.
Record blockers without treating menu readiness or compilation as runtime acceptance.

## Plan self-review and handoff

The four tasks cover every mutation class and completion criterion in the approved specification.
The five review conditions each have a named native test above.
Signatures are defined before their consumers. Test helpers remain under `cfg(test)`.
The production scan remains functional during caller migration, then is removed in Task 3.
No compatibility path or runtime fallback is added.

Implementation starts after plan review and execution-method selection.
Inline execution is recommended because the tasks share mutation state and depend on the preceding API changes.
The user can instead select subagent-driven execution with the permitted model.
