# Browser save and load

Date: 2026-10-12
Branch: `codex/save-load`
Status: Approved specification; implementation plan awaits review.

## Purpose

Let players save a browser session and restore its logical state after a reload.
Preserve the previous save when a write fails.
Preserve the active session when a load fails.

Use one manual save slot and IndexedDB.
Support only the current schema and current source export.
Reject incompatible saves. Do not add migrations or compatibility paths.

The current build has one Lada, its parts, home items, roadside pumps, mechanisms, and lamps.
It has no survival or weather system.
This work adds no survival, weather, vehicles, autosave, export, import, or save-slot management.

## Current state inventory

| Owner | Persistent logical state | Rebuilt or transient state |
| --- | --- | --- |
| `PlayerState` | Absolute capsule pose, yaw, pitch, seat, crouch, vertical and recoil velocities, running state | Controller, shapes, body, collider, contacts, support-body binding |
| `PhysicsWorld` and `GarageRuntime` | Vehicle pose, linear and angular velocities, handbrake, auto gear | Body and collider handles, wheel controller, queries, contact solver caches |
| `EngineState` | Ignition, RPM, temperature, run timing, start timing, random-start state, fuel supply, fan state | Held-key and starter gesture state; computed output |
| `StartingArea` | Poses and velocities of moved source bodies; dynamic-body sleep state | Body map, compound colliders, attachment cache, mass cache, render-pose buffers |
| `ItemStore` | Item identity, pose, location, condition, temperature, fluid quantities | Body handles, name cache, source capacities, source rules |
| `PartAttachments` | Installed part links and detached assembly poses | Shapes, body handles, slot scan bindings, mass cache |
| `WheelAssemblies` | Rim/tire links, wheel-slot links, free assembly pose and velocity, wheel rotation | Colliders, sensors, support tuning, handle maps |
| `MechanismState` | Rotation degrees, slide position, enabled state, turn state | Source frames, body bindings, active drag gesture |
| `InteractionState` | Opening valves and held item | Aim, collider bindings, fill gesture, push gesture, input latches |
| `CarryState` | Held instance, carry offset, rotation, local grip pose | Anchor body, joint handle, throw gesture |
| `WorldStream` | World seed and stable identities of generated interactive objects | Origin, chunks, terrain, pump colliders, support colliders |
| `CycleState` | Active elapsed time, cycle time, evaluated-cycle time, cloud coordinate, star rotation | Evaluated colors, fog, sky, and light signals |
| `LightingState` | Requested headlights, controller state, fade energy, refresh state, cabin modes, switch movement | Mounted-state cache, renderer outputs, source material and light bindings |
| `Drive` | Camera mode | Input, step accumulator, interpolation, readiness and feedback caches |
| `InstrumentState` | None beyond saved trip distance and vehicle signals | Needle smoothing, digit interpolation, lamp outputs |

Persist values that affect the next logical update.
Recompute values derived from source rules or other saved state.
Store independent bodies once, even when multiple items share a body.
Keep attachment-relative poses separate from absolute world poses.

## Save contract

Use a typed Rust snapshot with strict field validation.
Use the existing `serde` and `serde_json` dependencies.
Keep browser storage and asynchronous operation control in a separate JavaScript module.

The stored record contains a `payload` object and a `checksum` string.
The payload contains the fields below, except `checksum`.

| Field | Contract |
| --- | --- |
| `schema_version` | Exactly `1` |
| `saved_at` | Save time for menu feedback; no simulation effect |
| `source` | App ID, build ID, exact runtime manifest SHA256, and source-file hashes |
| `world` | Seed and records for existing world changes |
| `player` | Player and carry state from the inventory |
| `vehicles` | The existing vehicle instance, body state, engine state, gear, and handbrake |
| `bodies` | Independent changed source bodies and detached assembly bodies |
| `items` | Current item state, including fluids, condition, temperature, and logical location |
| `attachments` | Part-slot, wheel-slot, and tire/rim links |
| `mechanisms` | Mechanism values and opening valves |
| `progress` | Trip distance |
| `lighting` | Cycle state, lamp controller state, and switch movement |
| `view` | Camera mode |
| `checksum` | SHA256 of the serialized snapshot payload |

The browser computes the runtime manifest hash from the exact bytes used during startup.
The browser computes and checks the payload checksum outside the IndexedDB transaction.
The database stores the snapshot as structured data with its checksum and metadata.
Compute the checksum from UTF-8 bytes of `JSON.stringify(payload)`.
Preserve Rust struct field order when creating the payload object.
Read the stored payload without changing its field order before checking its checksum.
Sort identity-indexed records during capture.

A checksum detects damaged data. It is not a security boundary.
Rust validates the entire snapshot after the browser checks its envelope and checksum.

### Identity

Use each source record's file and path ID as its source identity.
Use an instance identity separate from the source identity.
The starting car, home, parts, and supplies each have one deterministic initial instance identity.
Resolve runtime numeric IDs through the validated manifest; do not use array indices as persisted identities.

Identify each roadside pump by world seed, signed station index, and its role within that station.
Replace pump identity allocation based on visit order.
Generated pump bindings can use runtime numeric IDs, but saves must not depend on those IDs.
Pumps retain unlimited source stock; regenerate their reservoir instead of saving artificial depletion.

Reference attachment owners, slots, seats, mechanisms, and lamps through stable identities.
Reject duplicate instances, missing references, ownership conflicts, and attachment cycles.
Validate part compatibility and wheel compatibility against the current manifest.

### Coordinates

Store world positions as absolute `[f64; 3]` coordinates.
Convert runtime positions through `WorldOrigin::absolute` during capture.
Store rotations as finite normalized quaternions and velocities in world axes.
Store attachment and grip offsets in their declared local frame.

Choose a fresh origin near the saved focus during reconstruction.
Create streamed terrain around the saved player and vehicle before restoring dynamic bodies.
Keep distant object positions in absolute form; never narrow the full absolute position to `f32`.
Use bounded local coordinates for active bodies.
Recreate distant changed bodies when their region enters the active world.
Retain their logical records while their region is inactive.

## Save operation

1. Accept Save from the pause menu after runtime readiness.
2. Clear input gestures while retaining held-item ownership.
3. Capture the snapshot between fixed simulation steps.
4. Validate the snapshot and compute its checksum.
5. Open one IndexedDB read/write transaction for the save slot.
6. Replace the complete record within that transaction.
7. Report success only after the transaction completion event.

Prepare serialization, validation, and hashing before opening the write transaction.
Request strict transaction durability using the current browser API.
Do not delete the previous record before writing its replacement.
Abort on write errors. Keep the previous committed save and the active session.
Report quota, unavailable storage, blocked database access, and transaction failure in the menu.
Do not route expected storage errors through the fatal game-start error handler.

Permit one pending save, load, or new-game operation per page.
Disable conflicting actions during an operation.
Use an operation token to reject stale asynchronous results.

## Load operation

Accept Load from the start menu or pause menu after runtime readiness.
Keep the active session paused throughout preparation.

1. Read the complete stored record without changing the active world.
2. Validate the envelope, checksum, schema, and exact source compatibility.
3. Validate numerical values, required records, identities, and relationships in Rust.
4. Confirm required assets and their loader dependencies are available.
5. Construct a separate candidate runtime with a fresh physics world.
6. Recreate terrain, source bodies, parts, wheel assemblies, mechanisms, seats, and carried items.
7. Restore fluid quantities, condition, engine state, trip distance, cycle state, and lamps.
8. Rebuild collider bindings, mass data, queries, body maps, and attachment bindings.
9. Prepare candidate presentation bindings using the existing loaded asset library.
10. Commit the prepared runtime and presentation bindings at one frame boundary.

All fallible reconstruction and asset work occurs before commit.
Commit replaces the prepared state without a further fallible asset request.
Release the old runtime after the replacement has its required bindings.
Discard the candidate after any preparation failure.

On failure, retain the old runtime, camera, origin, items, attachments, fluids, and progress.
Show the specific failure in the menu. Keep Continue available for an active session.
For an incompatible record, show: "This save uses a different schema or source export."
For a missing record, show: "No saved game is available."
For missing assets, identify the required source asset in the error.

On success, remain at the pause menu and show: "Game loaded."
Continue performs mouse capture through the existing user action.
Load does not request mouse capture after asynchronous work.

Reset input gestures, the step accumulator, interpolation, aim, and feedback caches during commit.
Restore the saved camera mode and initialize instruments from restored logical signals.
Restore a held item through a new anchor and joint, using its saved grip state.
Restore seat occupancy through the saved vehicle instance and source seat identity.
Cancel drag, push, fill, throw, and starter gestures; no saved key remains pressed.

## Menu and new game

Add Load to the start menu.
Add Save and Load to the pause menu.
Replace Restart with New game in UI text and action names.
Use the existing controls, source styling, and responsive menu patterns.
Provide pending status and operation results through the menu status area.

New game constructs a fresh runtime before replacing the current session.
It restores garage, supplies, car, origin, trip distance, source cycle offset, and lamp state.
It preserves the stored save until the player completes another Save.
Recover continues to preserve current session progress.
Storage failure must not disable New game, Recover, or Continue.

## Component boundaries

| Component | Responsibility |
| --- | --- |
| Rust save module | Schema, identity resolution, state validation, capture, candidate reconstruction |
| Existing state owners | Narrow capture and restore methods for their private logical state |
| Browser storage module | IndexedDB access, integrity checks, operation tokens, storage errors |
| Browser/runtime bridge | Explicit capture, prepare, commit, and result messages |
| App integration | Fixed-step capture and atomic runtime/presentation replacement |
| Menu integration | Save, Load, New game, pending state, and feedback |

Keep serialization out of per-frame render and physics paths.
Allocate snapshot and candidate data only during save or load operations.
Avoid a generic persistence framework or unrelated runtime refactoring.

## Critical verification

| Check | Required evidence |
| --- | --- |
| Driven vehicle | Save and load preserve pose, velocity, engine state, seat, fluids, and trip distance |
| Rebase | Absolute positions survive a rebase before save and a fresh origin during load |
| Removed part | Detached pose, velocity, condition, slot vacancy, and rebuilt collision behavior match |
| Split wheel | Rim/tire ownership and wheel support rebuild without duplicate bodies or colliders |
| Held item | Identity, carry offset, rotation, and rebuilt joint survive the round trip |
| Fluid transfer | Mixture quantities, valve state, and container ownership survive the round trip |
| World changes | Moved home objects and distant dropped parts remain changed after stream-out and reload |
| Station order | Visiting stations in different orders does not change saved pump identity |
| Lighting | Cycle, cloud and star state, lamp modes, and an active fade survive the round trip |
| Interrupted write | Aborting after a put request preserves the preceding committed record |
| Browser interruption | Closing the page during a transaction leaves a complete old or new record |
| Corrupt record | Damaged payload, checksum, fields, and links fail before commit |
| Storage failure | Unavailable storage, quota failure, and blocked access retain the save and active session |
| Source mismatch | Changed schema, build, manifest, or source hashes produce a clear incompatibility error |
| Missing assets | An unavailable required asset rejects preparation and retains the active session |
| Candidate failure | A reconstruction failure discards the candidate and leaves Continue usable |
| New game | Fresh logical state replaces play while the stored save remains loadable |

Compare normalized logical snapshots before and after reconstruction.
Exclude runtime handles, chosen origin, caches, and canceled gestures from equality checks.
Check resumed simulation and interactions after reconstruction, not serialization equality alone.
Use native tests for the contract and reconstruction rules.
Use browser tests for real IndexedDB transactions and menu/runtime integration.
Record any browser input limit; a controlled fixture does not prove uninterrupted mouse-capture play.

## Review checks

The design covers the existing persistent state inventory and user-requested failure cases.
It defines current-schema rejection without migrations.
It separates source identity, instance identity, and runtime handles.
It preserves inactive world changes and uses absolute positions.
It requires candidate preparation before active-session replacement.
Implementation starts after written-spec review, plan review, and execution-method selection.
