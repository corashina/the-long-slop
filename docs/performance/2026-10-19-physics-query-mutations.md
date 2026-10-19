# Physics query mutation tracking

Date: 2026-10-19.
Branch: `codex/physics-query-mutations`, unmerged.
Implementation base: `8eca150d90e7d0fa216823276c01825f8b1189d1`.
Design: [Approved specification](../superpowers/specs/2026-10-19-physics-query-mutations-design.md).
Plan: [Implementation plan](../superpowers/plans/2026-10-19-physics-query-mutations.md).
Steam build `11140890` remains the behavior authority.

## Implemented behavior

`PhysicsWorld` owns scoped body and collider edits, insertion, removal, parenting, mass changes, and bulk edits.
All callers use these methods. The old `rapier_mut` and `queries_mut` methods are absent.
The three mutable query pipelines remain private to the vehicle and character impulse operations.

`QueryBounds::refresh` processes affected handles instead of scanning every collider and cached slot.
Full handles include arena generations. Removal processing cannot erase a replacement generation.
Every collider edit conservatively marks geometry as changed, including in-place shape edits.
Body edits expand into current attached collider handles.
Attachment changes synchronize the collider's world pose before queries.

A refresh keeps explicit mutations queued until the next physics step finishes.
The step collects active bodies before and after Rapier advances, replays explicit mutations, and refreshes queries.
It then clears the explicit step queues.
Existing caller refresh boundaries remain in place.

The refresh preserves `propagate_modified_body_positions_to_colliders` and Rapier's notification lists.
It consumes no `take_modified` or `take_removed` list and performs no collision detection.
The previous scan remains under `cfg(test)` as an independent comparison index.
No production scan fallback remains.

## Native contract coverage

The query filter runs 14 source-independent tests with the product `FIXED_DT`, approximately 20 ms.
Comparisons check ray values, every shape-hit field, sorted overlap handles, collider generations, and bounds.
They require equal candidate and reference leaves and containment of current geometry.
Pinned Parry can retain an enclosing leaf when geometry shrinks, even with margin zero.

The tests cover direct pose and shape edits, local pose edits, enablement, attachments, and repeated slot reuse.
They also cover dynamic and kinematic motion, sleep, contact wake, retained contacts, and private controller impulses.
Post-step assertions use public queries before an extra caller refresh.
The 240-step settling test checks both indexes at every boundary and observes natural sleep.

`wheel_door_part_and_retirement_sequences_match_scan` replays the current write order from wheel, mechanism, and part synchronization.
It uses a loose dynamic body, a fixed parent, a kinematic door body, local poses, and box geometry.
It transfers the part, moves the door, toggles the collider, recomputes mass, detaches the part, and retires its body.
Queries match the scan after each refresh boundary. The retired collider has no query leaf or ray hit.
This test checks the mutation contract. It does not run the complete gameplay systems.

`rebase_and_save_replacement_keep_query_state_local` starts a body at x=12 and a standalone collider at x=20.
It shifts physics and `WorldOrigin` by x=8, then captures, serializes, and deserializes a `BodySnapshot`.
The absolute saved body position remains `[12.0, 0.0, 0.0]`.
A fresh world restores pose, recomputes mass, then restores velocity and sleep.
It tests both awake and sleeping snapshots.
The restored body is at local x=4. The old standalone collider remains queryable at local x=12.
Fresh arenas reuse the old complete handles while the old world retains an unflushed removal.
Separate queues and indexes keep both worlds correct before and after the old removal flushes.
This test checks snapshot reconstruction and world ownership. It does not run `GarageRuntime::from_save`.

## Owned work counts

`RefreshWork` counts actual owned refresh-loop visits and explicit `compute_aabb()` calls.
The body count includes pending-body visits, including stale handles.
The collider count includes retirement-cache visits and pending-collider visits, including unsuccessful lookups.
A pending-collider visit includes its live lookup and cached-state comparison.
Multiple arena reads within that visit do not count as separate visits.
Body expansion reads attached collider handles without inspecting each collider object there.

The counters exclude Rapier propagation, BVH internals, simulation, active-set enumeration, and the comparison scan.
They also exclude handle queue marking, clearing, and replay loops.
Preserved Rapier propagation can revisit modified bodies across refreshes before a physics step.
Zero owned discovery visits therefore does not mean zero work throughout the dependency.

| Case | Body visits | Collider visits | AABB calls |
| --- | ---: | ---: | ---: |
| Previous scan, unchanged scene with 1,024 standalone balls | 0 | 2,048 | 0 |
| Incremental refresh, unchanged scene | 0 | 0 | 0 |
| Two edits to one standalone collider before refresh | 0 | 1 | 1 |
| Body metadata edit with unchanged attached bounds | 1 | 1 | 0 |
| Collider metadata edit with unchanged attached bounds | 0 | 1 | 1 |
| Live fixed body after repeated same-slot generation replay | 1 | 1 | 0 |

The idle test warms body, collider, removal, and explicit step storage.
Queue capacities remain unchanged across eight idle refreshes with fixed membership.
The retained-byte stability test also passes across eight unchanged refreshes.
An enabled collider metadata edit still computes one AABB because the scoped edit could change geometry.
This conservative cost avoids storing old shared shapes merely to detect geometry writes.
These counts do not establish a frame-time improvement.

`QueryBounds::retained_bytes` includes the following retained heap storage:

- Handle and slot-marker capacities for pending bodies, pending colliders, removals, step bodies, and step colliders.
- Geometry-flag capacity multiplied by `size_of::<bool>()`.
- Cached-state capacity multiplied by `size_of::<Option<ColliderState>>()`.
- BVH storage reported by `heap_memory_size()`.

Each queue adds handle capacity times handle size and marker capacity times `size_of::<Option<(H, usize)>>()`.
The cache retains no shape references or second changed-collider queue.
This is query-index accounting, not whole-process memory or allocator overhead.
No matched runtime retained-byte measurement has run.

## Dependency and write audit

The implementation audit used pinned Rapier `0.36.0` and Parry `0.31.1`.
The remaining direct vehicle writes follow these tracking rules:

| Write path | Tracking |
| --- | --- |
| Constructor body and collider insertion | Owner methods seed pending and explicit step queues. |
| Scoped edits, parenting, mass, lifecycle, and bulk edits | `vehicle/mutations.rs` marks each affected handle. |
| Vehicle input wake, wheel replacement, mass, reset, and support retirement | Internal methods mark the chassis before writes. |
| Origin shift | Existing mutation loops mark moved bodies and standalone colliders. |
| Attachment pose synchronization | The pending-collider refresh loop owns the write. |
| Vehicle and character controller impulses | Private adapters change velocity and wake. Simulation tracking covers subsequent movement. |
| Rapier simulation and final collider propagation | Pre-step and post-step active-body union plus retained explicit mutations. |
| Joint removal wake | No immediate bounds write. Active-body tracking covers subsequent movement. |

Pinned `pipeline/user_changes.rs` processes parent, pose, type, and enable notifications.
Explicit step queues retain these edits after an intervening query refresh, including edits to fixed or disabled bodies.
Pinned `pipeline/physics_pipeline/substep.rs` propagates final poses, including CCD-clamped motion and kinematic movement.
Quarantine paths are covered by explicit edits and active-body expansion.

Pinned sleeping rules matter to the active-body union.
The product uses `FIXED_DT = 0.01999999955` and does not override the default 0.5-second sleep timer.
`physics_pipeline/solve.rs` commits sleep before integration.
`island_manager/sleep.rs` strongly resets sleeping persistent-island members when they wake.
A newly reset timer cannot expire in one product timestep.
Initial persistent-island bootstrap can weakly wake a sleeping member, but insertion entries remain queued through its first step.
This audit covers the pinned dependency and current product timestep, sleep settings, and owner methods.
Future raw-world restoration, changed sleep settings, or new simulation types require another mutation-path audit.

Parent enablement timing remains unchanged.
A query refresh alone does not process Rapier's parent-enable notification.
Reparenting a `DisabledByParent` collider to an enabled fixed body preserves that disabled state.
The differential test requires the scan's absence until later parent disable and enable notifications restore the collider.
The implementation does not change this dependency behavior.

## Verification results

Commands run from the isolated worktree:

```powershell
./scripts/check.ps1 -Filter query_bounds
git diff --check
rg -n 'rapier_mut|queries_mut' src tests
rg -n 'colliders\.iter|bodies\.iter|states\.iter|take_modified|take_removed' src/vehicle/query_bounds.rs
```

The check script used the existing `my-summer-slop-toolchain` Docker image and locked dependencies.
It ran these commands:

```text
cargo fmt --all -- --check
cargo test --locked --lib query_bounds
cargo check --locked --target wasm32-unknown-unknown --lib
```

Final native result:

```text
running 14 tests
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.02s
Finished test profile [unoptimized + debuginfo] target(s) in 13.43s
```

Formatting passed. The WASM library check finished in 0.70s. The check script exited `0`.
Both searches returned no matches, exit `1`, as expected. Diff checks passed, exit `0`.
The branch diff against the recorded base and remaining direct vehicle writes were reviewed.
No additional native integration target changed during Task 4.

Earlier verification established meaningful RED and GREEN evidence:

| Stage | Command | Result |
| --- | --- | --- |
| Scan comparison fixture | `./scripts/check.ps1 -Filter query_bounds` | 5 passed, 0 failed, 0 ignored, 46 filtered out. WASM passed. |
| Owner API test before implementation | Same command | Native compilation failed with 15 missing-method errors. Exit 1. WASM did not run. |
| Completed owner API and migration | Same command | 6 passed, 0 failed, 0 ignored, 46 filtered out. WASM passed. |
| Idle-work assertion against previous scan | Same command | 11 passed, 1 failed, 0 ignored, 46 filtered out. Exit 1. WASM did not run. |
| Generation replay without latest-entry checks | Same command | 11 passed, 1 failed, 0 ignored, 46 filtered out. Exit 1. WASM did not run. |
| Incremental updater and restored replay checks | Same command | 12 passed, 0 failed, 0 ignored, 46 filtered out. WASM passed. |
| Added sequence and snapshot tests | Same command | 14 passed, 0 failed, 0 ignored, 46 filtered out. WASM passed. |

The idle RED observed `RefreshWork { bodies: 0, colliders: 2048, aabbs: 0 }` instead of zero visits.
The generation RED observed pending counts `(6, 6, 0)` instead of `(1, 1, 0)` after repeated replay.
Latest-entry generation checks were restored before GREEN verification.
The two final characterization tests passed on their first run and exposed no new bug.

Four existing native dead-code warnings remain in `src/world/renderer_pending.rs`.
Five existing WASM unused-variable or mutability warnings remain in `src/presentation/materials.rs`.
Git retains existing LF-to-CRLF warnings. No new compiler warning remains.

## Deferred acceptance and measurements

`web/assets/reference/manifest.json` is absent from this isolated worktree.
The ignored inventory and source streaming checks require that installed export.
Those scenarios were unavailable here and were not executed.
The query filter's zero ignored tests does not imply acceptance of those separate scenarios.
No source export was loaded or validated during this task.

Full checks, release builds, and browser acceptance were not requested and did not run.
Complete wheel, door, part, inventory, streaming, recovery, and Save/Load gameplay remain unverified.
Real input, active game physics, submitted frames, and source runtime parity remain unverified.
The snapshot test does not replace Save/Load acceptance through `GarageRuntime::from_save`.

Garage and driving FPS, average frame time, p95, p99, and query preparation CPU time remain unmeasured.
A separately approved milestone must use the roadmap's matched release A/B/A protocol and exclusive benchmark windows.
That milestone must record package and asset hashes, viewport, DPR, profile, starting state, and route.
Instrumented work counts must remain separate from uninstrumented frame times.
The branch remains unmerged. Reduced owned scan work alone does not establish a performance gain.
