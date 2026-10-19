# Physics query mutation tracking

Date: 2026-10-19
Branch: `codex/physics-query-mutations`
Base: `8eca150d90e7d0fa216823276c01825f8b1189d1`
Status: Design direction approved. Written specification awaits review.

## Goal and scope

Implement performance improvement 10 from `docs/roadmap.md`.
Replace repeated full collider scans with complete application-owned mutation tracking.
Keep query results, physics notifications, and refresh timing unchanged.
Steam build `11140890` remains the behavior authority.

The user approved the design direction on 2026-10-19.
This specification defines that direction before implementation planning.
The branch remains unmerged.

No physics tuning, new gameplay, dependency fork, or save format change is included.
No performance gain is assumed.

## Current behavior and evidence

`src/vehicle.rs` owns Rapier's world and a separate `QueryBounds` index.
`src/vehicle/query_bounds.rs::refresh` scans every cached slot and every live collider.
It detects position, attachment, enabled state, shape, insertion, removal, and handle generation changes.
It also keeps a shared shape reference to detect edits through `shape_mut`.

The refresh first calls `propagate_modified_body_positions_to_colliders`.
It retains Rapier's notifications for the next simulation step.
Attachment edits need separate synchronization because their bodies might not move.
Effective enablement includes both the collider and its parent body.

`step_physics` refreshes queries immediately after Rapier advances.
`starting_area.rs::step` also refreshes after mechanisms, attachments, parts, and wheels synchronize.
Other callers refresh between edits and queries within the same simulation step.
These refresh points are part of the contract.

Commit `28845d19b1f89287447a779ed42c1b757df96d7d` introduced this owned index.
Its tests cover direct edits, attachments, handle reuse, contacts, and queries after a step.
Those tests establish constraints for this change.

The cached Rapier 0.36.0 source confirms these dependency contracts:

- `ColliderSet::take_modified` and `take_removed` consume notifications used by the physics pipeline.
- `RigidBodySet::propagate_modified_body_positions_to_colliders` preserves the body notification list.
- `IslandManager::active_bodies` includes active dynamic and kinematic bodies.
- The vehicle controller changes body velocity through impulses.
- The character impulse solver changes body velocity and can wake bodies.

Dependency evidence comes from `geometry/collider_set.rs`, `dynamics/rigid_body_set.rs`,
`dynamics/island_manager/manager.rs`, and the two controllers under `control`.

## Caller contract

Callers supply a handle and a scoped edit. They do not mark query state themselves.

```rust
physics.edit_body(body, |body| body.set_position(pose, true));
physics.edit_collider(collider, |collider| collider.set_shape(shape));
physics.set_collider_parent(collider, Some(parent));
physics.refresh_queries();
let hit = physics.queries(filter).cast_ray(&ray, distance, true);
```

The method names above define the intended API.
Edits to invalid handles retain the existing caller's failure behavior.
Required-handle edits fail for invalid handles.
`try_edit_body` and `try_edit_collider` return `None` for invalid handles without calling the closure.
Closures cannot return borrowed mutable physics objects.

`rapier()` remains a read-only view.
`rapier_mut()` is removed after all callers migrate.
The public `queries_mut()` method is removed.
No replacement exposes mutable body sets, collider sets, or the whole Rapier world.

Insertion, removal, parenting, joint operations, mass recomputation, and bulk edits have explicit owner methods.
Only operations used by current callers are added.
`recompute_body_mass` owns the mutable body and read-only collider borrows needed for mass recomputation.
Joint operations preserve their current wake-up arguments.
Gravity and integration settings use owner methods only where current callers need them.

The character impulse solver gets a specific owner method.
The vehicle controller keeps its current internal integration point.
Both use a private mutable query pipeline for their audited impulse operations.
They cannot expose that pipeline to callers.
Their velocity changes affect bounds when simulation moves the bodies.

## Ownership and modules

`vehicle::PhysicsWorld` owns the raw Rapier world, the mutation queues, and `QueryBounds`.
Raw mutable access stays private to the vehicle module.
Internal vehicle writes follow the same tracking contract as external callers.

`src/vehicle/mutations.rs` contains the tracked edit and lifecycle methods.
`src/vehicle/query_bounds.rs` owns the query BVH, cached bound state, and pending collider work.
`src/vehicle.rs` retains vehicle control and the simulation-step boundary.

Current callers migrate in `player`, `interaction`, `starting_area`, `save`, and `world`.
Tests that use the removed methods migrate in the same change.
No compatibility layer remains.

## Mutation data

Pending body and collider entries use full Rapier handles, including their generations.
Slot-indexed markers deduplicate queued work without a per-frame hash allocation.
Queue storage grows when world membership grows and is reused across refreshes.

Separate queues retain explicit body and collider mutations since the previous physics step.
A query refresh clears pending query work but keeps these simulation mutation queues.
This retains coverage when Rapier later processes an already-refreshed edit.

A pending collider entry records whether geometry could have changed.
Every scoped collider edit sets that flag conservatively, including `shape_mut` edits.
Body movement alone does not set the geometry flag.

Each cached collider state retains its handle, world pose, attachment pose, and effective enabled state.
It no longer retains a shared shape merely to discover unreported geometry changes.
Geometry mutations are reported by the edit boundary instead.

Removal entries remain distinct from live dirty entries.
Removing one generation cannot erase a replacement that uses the same slot.
A fresh world owns fresh queues and a fresh query index.
No pending handles carry across Save/Load world replacement.

## Refresh flow

1. Preserve the existing body-position propagation call and its notification behavior.
2. Expand dirty body handles into their current attached collider handles.
3. Apply recorded removals before inserting any replacement generation.
4. Resolve each pending live collider against the current Rapier sets.
5. Synchronize changed attachment poses using the current parent body pose.
6. Compute effective enablement from the collider and its parent.
7. Update the BVH when geometry, pose, attachment, or enablement requires an update.
8. Record the new cached state and clear pending work.

The propagation call can write colliders without passing through the scoped edit methods.
Dirty body expansion covers those writes.
Parent changes mark the collider and both affected bodies.
Body removal records its collider handles before Rapier removes them.
Insertion always marks the new collider generation.

An unchanged refresh performs no global collider or cached-slot scan.
Body-only edits with unchanged bound state need no AABB computation.
Collider edits can cause a conservative AABB computation even when only metadata changed.
This trade-off keeps mutable geometry access complete without retaining every old shape.

Whole-world operations still visit every object they actually change.
Origin shifts mark moved bodies and standalone colliders during their existing loops.
There is no full-scan fallback in production.
The old scan remains only as a test oracle.

## Simulation flow

`step_physics` queues the active body handles before calling Rapier.
It queues active body handles again after the step.
It also queues bodies and colliders retained in the simulation mutation queues.
It then refreshes queries before returning, as today.
Only then does it clear the simulation mutation queues.

The union covers bodies that sleep during the step and bodies that wake during the step.
Dynamic and kinematic motion use the same path.
Explicit body edits and new bodies also enter the pending body queue.
The simulation mutation queues cover edits refreshed before the step, even when the body is fixed or disabled.
For example, enabling a fixed parent can change its collider enablement during Rapier notification processing.
Step-time collider propagation is covered by expanding those body handles after simulation.

The implementation must verify this coverage against the pinned Rapier source and differential tests.
Any uncovered simulation write must receive a precise tracking path before the scan is removed.
A missed path cannot be hidden with a periodic scan.

No refresh consumes `take_modified` or `take_removed`.
No refresh runs collision detection or advances simulation as a substitute for updating queries.
Physics contact, island, wake-up, mass, and CCD notifications keep their current behavior.

## Required mutation coverage

| Mutation source | Tracking rule | Required comparison |
| --- | --- | --- |
| Body pose, rotation, type, or enabled state | Mark the body and expand its attached colliders. | Queries before and after a step. |
| Body impulses, velocity, gravity scale, or kinematic target | Track the edit and simulation movement. | Active, sleeping, waking, and kinematic bodies. |
| Collider world pose, local pose, enabled state, or shape | Mark the collider. Treat geometry as possibly changed. | Ray casts, shape casts, and overlaps. |
| Parenting and detachment | Mark the collider and affected bodies. | Old and new attachment locations. |
| Body or collider insertion and removal | Record complete handles and removals. | Same-slot reuse before a refresh. |
| Wheel, door, part, and storage synchronization | Use tracked edits at every current write. | Queries between existing refresh points. |
| Terrain and structure publication or retirement | Use tracked lifecycle methods. | Streaming admission and retirement. |
| Origin shifts and recovery | Mark objects inside the existing edit loops. | Relative geometry and wheel support. |
| Save capture and restore | Preserve capture order. Restore through tracked methods. | Same-world restore and world replacement. |
| Vehicle and character controller impulses | Keep mutable query access private to audited operations. | Motion, contacts, and waking after simulation. |

Bulk mutable iteration is confined to owner methods that mark every yielded object.
No unchecked mutable iterator escapes the owner.
A source audit covers aliases such as `let world = physics.rapier_mut()` before migration.
The audit also covers direct writes inside `vehicle.rs` and test setup.

## Critical verification

Keep the current scan as a test-only reference index.
Run both indexes against the same physics world at each existing refresh boundary.
Compare ray hits, shape hits, overlap membership, and AABBs.
Compare handles and query values, rather than BVH layout or unspecified tie order.
Use scenes without equal-distance ties for exact nearest-hit comparisons.

Critical cases cover the following behaviors:

- Position, rotation, local pose, shape replacement, and in-place shape edits.
- Parenting, detachment, enablement, removal, and repeated handle-slot reuse.
- Multiple edits and multiple refreshes before a physics step.
- Fixed-parent enablement and reparenting, with a query refresh before Rapier processes the notifications.
- Active motion, kinematic targets, sleeping, waking, and contacts that remain in the physics pipeline.
- Wheel support, door motion, part attachment, streaming retirement, origin shifts, recovery, and Save/Load.
- Queries immediately after `step_physics`, without an additional caller refresh.

Native test fixtures cover the targeted gameplay cases without requiring installed source assets.
Existing source-dependent checks remain separate and are reported when unavailable.
Test-only work counts prove that idle refreshes inspect no unrelated colliders.
Sparse edits inspect only queued bodies, their attached colliders, and queued collider handles.
Repeated refreshes with fixed membership reuse queue capacities.

Implementation checks include `scripts/check.ps1 -Filter query_bounds` and affected native integration targets.
These commands include a WASM compile check.
Full checks, release builds, and browser acceptance require an explicit request or an agreed milestone under `AGENTS.md`.
They do not run as part of this specification stage.

At an approved performance milestone, use matched release A/B/A runs in exclusive benchmark windows.
Record garage and driving average, p95, and p99 frame times separately.
Record query preparation CPU time, affected-object counts, AABB computations, and retained bytes.
Record instrumented counts separately from uninstrumented frame times.
Check real input, active physics, submitted frames, streaming, and Save/Load.
Reduced scan counts alone do not establish a performance gain.

## Alternatives and risks

A Rapier fork could expose independent mutation notifications.
That approach adds dependency maintenance and still needs complete simulation coverage.
Application-owned tracking uses the existing dependency and keeps the invariant with the owning module.

A world-level dirty flag would avoid some idle scans.
It would retain full scans whenever any object changes and does not meet item 10.

The main risk is an untracked write through an internal physics operation.
Private mutation access, a complete caller migration, and differential tests address that risk.
Conservative collider marking can increase AABB work for unchanged edits.
The work counters and later matched measurements must report that cost.

## Completion criteria

All mutation paths use tracked ownership, including internal controller and simulation paths.
Production refreshes have no global discovery scan.
The old mutable access methods are absent.
Critical native checks and the WASM compile check pass.
No unverified runtime behavior or performance gain is reported as verified.
The branch remains isolated and unmerged.
