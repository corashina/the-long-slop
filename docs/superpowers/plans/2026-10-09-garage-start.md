# Garage Start Implementation Plan

> **For agentic workers:** Use `superpowers:subagent-driven-development` for delegated execution, or `superpowers:executing-plans` for primary-agent execution. Track steps with checkboxes.

**Goal:** Start inside the garage, prepare the fixed Lada 2101, and drive onto the existing desert road.

**Architecture:** Export installed Unity references into local glTF assets and a typed manifest. Keep one Rapier world and authoritative Rust state. Import visuals through Bevy and bind stable gameplay IDs once.

**Tech Stack:** Rust 1.99, Bevy `0.19.1`, Rapier `0.36.0`, wasm-bindgen `0.2.129`, UnityPy `1.25.4`, TypeTreeGeneratorAPI `0.0.10`, pygltflib, Python, and the existing browser shell.

**Spec:** [Approved garage specification](../specs/2026-10-09-garage-start-design.md).

## Global Constraints

- Reference: installed Steam build `11140890`; fixed Lada 2101; garage spawn.
- Retain Rust, WebAssembly, Bevy, Rapier, and localhost hosting.
- Keep Steam files unchanged. Keep recovered C# and generated original assets outside version control.
- Match the installed home's layout, object transforms, selected vehicle assets, and specified preparation behaviors.
- Do not claim full-game or identical Unity physics parity.
- Stop a recovery checkpoint if a required field cannot be recovered. Do not invent its value.
- Use one physics world. Run gameplay at the existing fixed 60 Hz.
- Reuse application buffers, handles, and assets during frame updates.
- Remove obsolete startup and refuel paths. Add no compatibility modes or procedural asset fallback.
- Add only critical tests. Add no survival, weapons, other vehicles, multiplayer, saves, deployment, or reduced-motion behavior.
- Use short UI instructions and documentation. Apply the repository's stop-slop writing rules.

## Review Focus

1. Invalid asset identity or missing dependency must keep Start disabled and name the failure. Tasks 1 and 7 test this.
2. Reflected wheel geometry and scaled door parents must keep correct size, facing, pivots, and collision. Tasks 1, 6, and 7 test this.
3. Catch-up steps must not repeat an ignition, seat, pickup, or handbrake edge. Tasks 3, 4, and 8 test this.
4. A wall must block both movement and an aimed interaction. Tasks 4 and 5 test this.
5. Restart after carrying, detaching, and rebasing must restore one initial state without duplicate bodies. Task 8 tests this.

## Files and ownership

| Files | Owner and purpose |
| --- | --- |
| `tools/reference/{export.py,gltf.py,parameters.py,requirements.txt}` | Asset agent: offline extraction, conversion, and parameter recovery |
| `tools/reference/tests/test_export.py` | Asset agent: identity and transform checks |
| `src/reference.rs` | Primary: manifest and shared IDs; freeze before delegation |
| `src/vehicle.rs`, `tests/vehicle.rs` | Primary: physics owner, query access, vehicle forces, and wheel support |
| `src/world.rs`, `tests/world.rs` | Primary: active focus, driveway terrain, exclusions, and origin |
| `src/fluids.rs`, `src/simulation/engine.rs`, `tests/{fluids,engine}.rs` | Fluid agent: quantities, transfer, spills, and recovered engine rules |
| `src/player.rs`, `tests/player.rs` | Player agent: movement, camera state, seating, and clearance |
| `src/interaction.rs`, `src/interaction/{items,wheels}.rs`, `tests/interaction.rs` | Interaction agent: item identity, aim, hinges, carry, caps, and attachments |
| `src/starting_area.rs`, `src/presentation/reference.rs` | Primary: imported area, body creation, and render bindings |
| `src/{app,browser,simulation,lib}.rs`, `src/presentation/{camera,car,mod,world}.rs` | Primary: integration, schedule, lifecycle, and obsolete-path removal |
| `web/{main.js,index.html,style.css}`, `scripts/{serve.mjs,build.ps1,check.ps1}` | Primary: browser input, loading, UI, and build checks |
| `tests/{reference,starting_area,simulation}.rs`, `tests/common/{mod.rs,reference.json}` | Primary: contract, world, and lifecycle checks |
| `.gitignore`, `Cargo.toml`, `Cargo.lock`, `README.md`, `PRODUCT.md`, `DESIGN.md`, `docs/verification.md` | Primary: dependencies and final product record |

Use `web/assets/reference/` for generated assets. Ignore that directory and the local extraction environment.
Use external PNG textures and binary buffers with `home.gltf`, `lada.gltf`, `items.gltf`, and `manifest.json`.
Asset paths inside Bevy start with `reference/`. Browser fetch paths start with `./assets/reference/`.

The primary owns shared interfaces and integration files. A module agent must request an interface change before editing them.
After Tasks 1 and 2, Tasks 3 and 4 can run in parallel. Task 5 follows the player and fluid contracts.
Task 6 follows Task 5. Tasks 7 through 9 run through the primary integrator.
Keep the current road game working until Task 7 replaces startup with a complete garage checkpoint.

## Shared contracts

Task 1 freezes reference IDs and records. Task 2 freezes physics access and `FrameInput` in `simulation.rs`.
Tasks 3 through 5 define their module contracts below. These are planned APIs, not existing APIs.

- `ItemId(pub u32)`, `AssetId(pub u32)`, `SeatId(pub u32)`, `OpeningId(pub u32)`, and `WheelSlotId(pub u8)`.
- `SourceId { file: String, path_id: i64 }` uses a resolved asset file, not a relative Unity pointer index.
- `ReferenceManifest::from_json(bytes: &[u8]) -> Result<Self, ReferenceError>` validates the full selected-build contract.
- `FrameInput { held: u32, pressed: u32, released: u32, mouse: [f32; 2], wheel: f32 }` separates edges and held input.
- `VehiclePose` keeps its existing position and quaternion arrays.
- `FluidMixture { quantities_liters: [f32; 6] }` stores gasoline, oil, alcohol, water, blood, and diesel in that order.
- `FluidContainer { capacity_liters: f32, mixture: FluidMixture }` owns quantities without frame-time allocation.
- `EngineInput` contains ignition press/release/hold, throttle, drivetrain RPM, clutch, effective electronics, and ambient temperature.
- `EngineOutput { torque_nm: f32, brake_torque_nm: f32, rpm: f32, running: bool }` replaces an enabled-engine flag.
- `AimTarget { item: ItemId, action: InteractionAction, hit: [f32; 3], distance: f32 }` represents the nearest usable surface.
- `ItemStore` owns one preallocated record per manifest item: location, condition, pose, fluid contents, and attachment.

The manifest has typed records for assets, nodes, bodies, colliders, anchors, hinges, wheel slots, items, tanks, engine, player, and seats.
Records carry source IDs and hashes. Runtime records use numeric IDs and resolved handles.
Bake collider geometry into body-local meters. Rapier body poses contain translation and rotation only.
Do not add manifest versions, migration logic, optional legacy records, or string-based queries during play.

## Task 1: Export validated assets and reference parameters

**Files:** Create `tools/reference/` files, `src/reference.rs`, and `tests/reference.rs`. Modify `.gitignore`, `Cargo.toml`, and `Cargo.lock`.
**Consumes:** The installed reference and recovered source in `.cache/reference/`.
**Produces:** `ReferenceManifest`, validated local glTF assets, source hashes, and a parameter evidence report.

- [ ] Create the local extractor environment. Pin UnityPy `1.25.4` and TypeTreeGeneratorAPI `0.0.10`.
- [ ] Start Docker Desktop if needed. Confirm `docker info` succeeds before running the repository checks.
- [ ] Resolve and pin pygltflib from its official package metadata. Add serde `=1.0.229` with derive and pin serde_json.
- [ ] Write `test_build_and_conversion`: reject build mismatch; convert positive Z to negative Z; preserve scaled and reflected transforms.
- [ ] Assert converted triangle normals face the converted surface. Assert collider points match converted visual points.
- [ ] Run `python -m unittest discover -s tools/reference/tests`. Confirm these checks fail before conversion exists.
- [ ] Implement `export_reference(steam_root: Path, output: Path) -> None` in `export.py`.
- [ ] Implement `export_gltf(roots, output: Path) -> dict` and `recover_parameters(env, roots) -> dict` in their owner files.
- [ ] Use UnityPy `MeshHandler.process()`, submesh triangles, normals, tangents, UVs, and texture images. Deduplicate resolved source identities.
- [ ] Export one primitive per submesh/material pair and node extras containing `reference_id`.
- [ ] Keep hierarchy, signed scale, texture color space, UV transforms, and alpha modes. Write glTF through pygltflib.
- [ ] Apply `C = diag(1,1,-1)`, `M' = C M C`, and quaternion `(-x,-y,z,w)` once.
- [ ] Reverse triangle winding and tangent handedness once. Do not add another reversal for retained negative node scale.
- [ ] Export static triangle collision and moving primitive or convex collision. Preserve sensor flags, groups, friction, and restitution.
- [ ] Trace selected home linkage, starter fluid callers, wheel condition changes, and `massScript` aggregation before accepting startup parameters.
- [ ] Decode only required custom components with `parse_as_dict(check_read=True)` and the loaded `TypeTreeGenerator`.
- [ ] Replace generated base-header nodes with UnityPy's standard MonoBehaviour nodes before reading custom fields.
- [ ] Compare decoded class links with `parse_monobehaviour_head()`. Reject a header mismatch or incomplete byte consumption.
- [ ] Use the exact targets below. Follow their references for caps, seats, containers, colliders, and required globals.
- [ ] Validate required records, positive capacities, finite transforms, bounded arrays, and source byte consumption.
- [ ] Run extraction for build `11140890`. Confirm matching hashes, valid glTF references, complete bindings, and a parameter provenance report.
- [ ] Run exporter tests and `./scripts/check.ps1 -Test reference`. Confirm both pass before committing the exporter and contract.

The transform test uses these assertions in `test_export.py`:

```python
self.assertEqual(convert_point((0.0, 0.0, 1.0)), (0.0, 0.0, -1.0))
self.assertEqual(convert_point((100.0, 0.0, 0.0)), (100.0, 0.0, 0.0))
```

Define `convert_point(value: tuple[float, float, float]) -> tuple[float, float, float]` in `tools/reference/gltf.py`.
Define the remaining typed records in `reference.rs`; reject unknown fields, missing bindings, duplicate IDs, and non-finite geometry.
Add `tests/common/reference.json` with small synthetic collider geometry and the recovered scalar values needed by native tests.
The fixture must not contain original meshes or substitute for production extraction.

| Source | Target | Known serialized values; startup still needs tracing |
| --- | --- | --- |
| `level1` | Home GO `32`, Transform `21400` | `haz02`, 434 transforms |
| `sharedassets1.assets` | Lada GO `40132`, Transform `82556` | `Car02Full`, 697 transforms; root mass `717 kg` |
| `sharedassets1.assets` | Fuel tank `154417` | `39 L` capacity; `0.5 L` gasoline |
| `sharedassets1.assets` | Engine `136815` | Start chance `1`; starter `700`, idle `800`, maximum `6600`, shutdown `550 RPM`; torque `87 Nm` |
| `sharedassets1.assets` | Sump `138629` | `5 L` capacity; `2.5 L` oil |
| `sharedassets1.assets` | Coolant `144880` | `5 L` capacity; `3.6 L` water |
| `level1` | Player `80125`, capsule `58456`, camera `40241` | Speed `1.75`, run multiplier `2.5`, aim reach `1.85`; capsule radius `0.2`, height `1.75`; FOV `65°` |

`tankscript.FStart()` registers mass and updates fluids. It does not randomize them.
`RandomFluid()` replaces quantities. Trace its selected starter caller before using its randomization rules.
Export engine torque curve keys, tangents, weights, weighted modes, and wrap modes.
Trace runtime overrides such as `startingPushForce = maxNm * 9`.
The four road wheel radii are `0.300`, suspension distances `0.300`, and center offsets Y `-0.100`.
Audit `PlusWheelToKeepAwake` separately. It is not a fifth road wheel item.

## Task 2: Establish one physics owner and the shared item store

**Files:** Modify `src/{vehicle,world,app,browser,simulation,lib}.rs`, `web/main.js`, and `tests/{vehicle,world,simulation}.rs`. Create `src/interaction/items.rs` and `tests/common/mod.rs`.
**Consumes:** Task 1 reference records and shared IDs.
**Produces:** Stable physics access, force/step separation, active-focus rebasing, and preallocated item identity.

Interfaces:
`PhysicsWorld::new(reference: &VehicleReference, pose: VehiclePose) -> Self` replaces the procedural physics defaults.
`PhysicsWorld::queries(filter: QueryFilter<'_>) -> QueryPipeline<'_>` and its mutable counterpart expose Rapier queries.
`rapier(&self)` and `rapier_mut(&mut self)` expose the single owned Rapier world to native modules.
`apply_vehicle_forces(input: DriveInput, drive_force: f32)` applies forces without stepping physics.
`step_physics(&mut self)` integrates exactly once. `refresh_queries(&mut self)` updates collision queries after structural changes.
`WorldStream::rebase(focus_local: [f32; 3], physics: &mut PhysicsWorld) -> Option<[f32; 3]>` replaces chassis-only focus.
`ItemStore::new(reference: &ReferenceManifest) -> Self` creates records once.
`run(reference_json: &str) -> Result<(), JsValue>` parses the manifest once before app construction.
`common::reference() -> ReferenceManifest` and `common::world() -> PhysicsWorld` supply native test fixtures.

- [ ] Write `force_application_does_not_step`: applying forces leaves poses unchanged until `step_physics`.
- [ ] Write `walking_focus_rebases_parked_car`: player focus triggers rebase while the chassis remains near its old location.
- [ ] Assert absolute positions, body velocities, and item identities remain unchanged by the shift.
- [ ] Run `./scripts/check.ps1 -Test vehicle` and `./scripts/check.ps1 -Test world`. Confirm new assertions fail.
- [ ] Split existing force application from physics integration. Replace the old `step(input, engine_enabled)` callers in the same commit.
- [ ] Pass fetched manifest text into `run`. Keep the current road scene while creating its physics from recovered Lada parameters.
- [ ] Keep current road behavior by sending its existing force calculation through the new interface until Task 7 replaces fuel state.
- [ ] Add query access and item records. Resolve source identity into gameplay IDs during construction, never during frame updates.
- [ ] Add collider/body creation and cleanup through the shared physics owner. Refresh queries after changes, not per setter.
- [ ] Run vehicle and world tests, then `./scripts/check.ps1`. Confirm the current browser driving loop still works.
- [ ] Commit this working refactor. Freeze shared structs and signatures before dispatching Tasks 3 and 4.

The force test checks the real physics pose:

```rust
let before = world.snapshot().pose;
world.apply_vehicle_forces(input, 2600.0);
assert_eq!(world.snapshot().pose, before);
world.step_physics();
assert_ne!(world.snapshot().pose, before);
```

Define named mask constants for movement, Space, Use, Run, Crouch, Grab, Fill, Ignition, and Camera in `simulation.rs`.
Use those constants in the final browser bridge. Keep press, release, and held masks separate.

## Task 3: Implement fluid and engine rules

**Files:** Create `src/fluids.rs`, `src/simulation/engine.rs`, `tests/fluids.rs`, and `tests/engine.rs`. Add module declarations through the primary.
**Consumes:** `FluidMixture`, recovered `EngineReference`, tank records, and `EngineInput`.
**Produces:** Fluid operations and `EngineState::step(input, fluids, reference, random_sample, dt) -> EngineOutput`.

`transfer(source: &mut FluidContainer, destination: &mut FluidContainer, liters: f32) -> f32` returns accepted volume.
`spill(container: &mut FluidContainer, liters: f32) -> f32` returns removed volume.
`suitability(container: &FluidContainer, required: &FluidMixture) -> f32` follows recovered required-fluid ratios.
`VehicleFluids<'a>` borrows the fuel, sump, and coolant containers from `ItemStore`; it must not duplicate quantities.
`EngineState::step(&mut self, input: EngineInput, fluids: VehicleFluids<'_>, reference: &EngineReference, random_sample: f32, dt: f32) -> EngineOutput`.
`ItemStore::vehicle_fluids(&mut self, reference: &VehicleReference) -> Result<VehicleFluids<'_>, ReferenceError>` returns disjoint mutable references.
Define `FluidMixture::total(&self) -> f32` and an indexed `amount` getter in `fluids.rs`.

- [ ] Write `transfer_conserves_each_type`: a capacity-limited transfer preserves each source-plus-destination quantity within `0.00001 L`.
- [ ] Write `ignition_edges_and_start`: one tap toggles ignition once; held ignition engages the starter after the recovered delay.
- [ ] Assert ignition off, unsuitable fuel, insufficient oil, and excessive heat prevent running.
- [ ] Assert missing coolant affects cooling without adding a start lock. Use fixed randomness and recovered engine values.
- [ ] Run `./scripts/check.ps1 -Test fluids` and `./scripts/check.ps1 -Test engine`. Confirm new tests fail.
- [ ] Implement mixture transfer, spills, engine RPM, ignition, starter, torque curve, consumption, oil, and temperature from recovered rules.
- [ ] Apply fuel suitability to supplied fuel and target RPM. Apply oil and heat penalties to output.
- [ ] Use running thresholds `fuel > 0.3`, `oil penalty < 0.99`, and `heat penalty < 0.99`.
- [ ] Reuse fixed fluid arrays and caller state. Add no frame-time collection construction.
- [ ] Run fluid and engine tests. Commit these native modules without editing app integration files.

The transfer test asserts each type:

```rust
let accepted = transfer(&mut source, &mut destination, 10.0);
assert!((accepted - available_capacity).abs() < 0.00001);
for index in 0..6 {
    assert!((source.mixture.quantities_liters[index]
        + destination.mixture.quantities_liters[index] - before[index]).abs() < 0.00001);
}
assert!(!output_with_ignition_off.running);
```

## Task 4: Implement player movement, look, and seating

**Files:** Create `src/player.rs` and `tests/player.rs`. The primary adds the module declaration.
**Consumes:** Frozen physics queries, `FrameInput`, `PlayerReference`, seat records, and `VehiclePose`.
**Produces:** `PlayerState` with movement, look, occupancy, clearance, and view poses.

Interfaces:
`PlayerState::new(reference: &PlayerReference, pose: VehiclePose, physics: &mut PhysicsWorld) -> Self`.
`step(input: FrameInput, physics: &mut PhysicsWorld, dt: f32)` updates native player state.
`enter(seat: SeatId, pose: VehiclePose, physics: &mut PhysicsWorld)` changes occupancy.
`try_exit(physics: &mut PhysicsWorld, limits: &ExitReference) -> bool` preserves occupancy when blocked.
`view_pose(physics: &PhysicsWorld) -> VehiclePose`, `focus_position(physics: &PhysicsWorld) -> [f32; 3]`, and `rebase(shift)` support integration.

- [ ] Write `capsule_cannot_cross_wall` and `crouch_cannot_stand_under_roof` using real Rapier geometry.
- [ ] Write `blocked_exit_retains_seat`: a blocked capsule leaves occupancy and input mode unchanged.
- [ ] Assert seated and walking pitch clamp to ±89 degrees. Assert seated yaw follows recovered limits.
- [ ] Run `./scripts/check.ps1 -Test player`. Confirm new checks fail.
- [ ] Use `KinematicCharacterController::move_shape` for movement and Rapier collision queries for standing and exit clearance.
- [ ] Preserve ground, jump, run, and crouch state. Enter and exit only through explicit occupancy transitions.
- [ ] Reuse a collision scratch buffer. Invoke dynamic-body impulse solving only for relevant dynamic contacts.
- [ ] Create standing and crouched capsule shapes once. Reuse them for movement and clearance queries.
- [ ] Measure Rapier's internal impulse-solver allocation cost. Application scratch does not remove the library's manifold allocations.
- [ ] Run player tests. Commit the native player module without editing browser or app files.

The exit test uses the imported-size capsule and a blocking fixture:

```rust
assert!(!player.try_exit(&mut physics, &exit_limits));
assert_eq!(player.seat(), Some(driver_seat));
```

Define `PlayerState::seat(&self) -> Option<SeatId>` for occupancy checks and interaction dispatch.

## Task 5: Implement aimed use, hinges, carrying, and filling

**Files:** Create `src/interaction.rs` and `tests/interaction.rs`. Extend `src/interaction/items.rs`.
**Consumes:** Player state, shared item records, physics queries, hinges, openings, and Task 3 fluid operations.
**Produces:** `InteractionState::aim(physics, view, reach) -> Option<AimTarget>` and `step(input, target, player, items, physics, dt)`.

- [ ] Write `wall_blocks_target`: a nearer solid wall hides the usable opening behind it.
- [ ] Write `fill_requires_destination_open`: a closed destination accepts zero fluid; source tilt does not gate direct filling.
- [ ] Write `spill_requires_open_tilted_source`: closing the source or removing tilt stops spill loss.
- [ ] Assert pickup/drop keeps the same ID and does not create another item. Assert a hinge cannot exceed its recovered limits.
- [ ] Run `./scripts/check.ps1 -Test interaction`. Confirm new checks fail.
- [ ] Cast against blockers and usable surfaces together. Dispatch the nearest valid action by stable ID.
- [ ] Implement bounded hinge target/state movement and collider updates from the same pose.
- [ ] Implement cap valves, constrained carry distance, rotation, drop, direct fill, and physical spilling.
- [ ] Stop filling on release, lost aim, empty source, or full destination. Use source proportions and accepted volume.
- [ ] Exclude the carried body's own surface from aim where required. Retain collision with the environment and car.
- [ ] Implement pump-to-container filling through this action path. Remove no old browser refuel path until Task 7 integrates its replacement.
- [ ] Run interaction tests. Commit the complete native interaction layer.

The filling test checks these outcomes through `InteractionState::step`:

```rust
assert_eq!(closed_destination_liters, before_destination_liters);
assert!(open_destination_liters > before_destination_liters);
assert_eq!(dropped_item_id, original_item_id);
```

## Task 6: Implement wheel assemblies and vehicle parameter binding

**Files:** Create `src/interaction/wheels.rs`. Modify `src/vehicle.rs`, `tests/vehicle.rs`, and `tests/interaction.rs`.
**Consumes:** Item identity, reference wheel slots, condition, and Task 5 attachment actions.
**Produces:** `detach_wheel(slot, items, physics) -> Result<ItemId, InteractionError>` and `install_wheel(item, slot, items, physics) -> Result<(), InteractionError>`.
`PhysicsWorld::set_wheel(slot: WheelSlotId, wheel: Option<InstalledWheel>)` maintains a stable slot/controller map.
`drive_force(engine: EngineOutput, reference: &VehicleReference) -> f32` uses recovered drivetrain ratios and wheel radius.

- [ ] Write `removed_wheel_has_no_support`: detachment removes the road wheel from the active raycast controller.
- [ ] Assert the detached item retains condition and identity. Assert installation restores support without duplication.
- [ ] Assert tire/rim split and compatible assembly conserve all component IDs. Reject incompatible or occupied slots.
- [ ] Run vehicle and interaction tests. Confirm new checks fail.
- [ ] Bind actual Lada wheel anchors, radii, suspension, mass, and drivetrain parameters. Remove procedural vehicle constants when integration switches visuals.
- [ ] Rebuild active controller wheels only on attachment changes. Preserve remaining wheel state and reserve controller scratch.
- [ ] Make detached parts dynamic bodies. Bake reflected scale into collider geometry and preserve visual reflection parity on reparenting.
- [ ] Apply condition to grip. Update mass after accepted part and fluid changes through the recovered aggregation rules.
- [ ] Run tests, including wheel contact after rebase and reattachment. Commit the wheel layer.

The support test must inspect physical contact, not only engine force:

```rust
assert!(!physics.snapshot().wheels_contact[removed_slot_index]);
assert_eq!(detached_item_id, installed_item_id_before_removal);
assert_eq!(condition_after_installation, condition_before_removal);
```

## Task 7: Switch to the playable imported garage

**Files:** Create `src/starting_area.rs` and `src/presentation/reference.rs`. Modify app, browser, presentation, web shell, server, Cargo files, and world generation.
**Consumes:** Completed reference, physics, player, engine, item, and interaction modules.
**Produces:** One working garage-to-road startup path with loading checks, contextual UI, and original imported geometry.

- [ ] Enable Bevy `bevy_gltf` and `png`; retain pinned Bevy `0.19.1`. Update the lockfile with the existing toolchain.
- [ ] Use Task 2's manifest bootstrap. Extend readiness to imported dependencies and complete gameplay bindings.
- [ ] Load scenes with `GltfAssetLabel::Scene(index).from_asset(path)` and spawn `WorldAssetRoot`.
- [ ] Disable both `GltfConvertCoordinates` flags. Disable imported cameras, lights, and animations; use the game camera and lighting.
- [ ] Wait for `WorldInstanceReady`, recursive asset dependencies, node extras, collider creation, and valid initial state before enabling Start.
- [ ] Use `get_load_states()` and dependency readiness to name missing textures, buffers, scenes, and node bindings.
- [ ] Bind node IDs once through `GltfExtras`. Share loaded assets and materials across item instances.
- [ ] Implement `StartingArea::new(reference, physics, origin)` and `sync_origin(origin)` with fixed absolute home placement.
- [ ] Flatten the specified area and driveway. Suppress generated props and the starting station inside the area bounds.
- [ ] Spawn the standing player in the garage, the complete parked Lada, preparation supplies, closed garage doors, and valid seat/opening anchors.
- [ ] Replace the old startup and fuel state. Remove procedural starter car rendering, enabled-engine flags, and seated Hold E refueling.
- [ ] Apply the fixed-step order from the spec. Integrate engine torque into vehicle forces before the single physics step.
- [ ] Derive bounded start samples from the existing seeded hash and a start-attempt counter. Keep selected Lada start chance `1`.
- [ ] Add pointer lock on Start/Resume and accumulated mouse input. Map E, F, LMB, I, Space, and C by context.
- [ ] Update crosshair, target/action prompt, fluid quantity, wheel condition, speed, and engine state through cached DOM writes.
- [ ] Add JSON, glTF, binary-buffer, and PNG MIME types to the localhost server.
- [ ] Run `./scripts/check.ps1` and `./scripts/build.ps1`. Confirm the WASM build passes.
- [ ] In the browser, walk, open the garage, fill fluids, change a wheel, enter, start, and drive onto the road.
- [ ] Check wrong-facing/reflected meshes and mismatched door collision. Block one required texture and verify Start remains disabled with its failure.
- [ ] Commit only after this complete startup flow works. Add no alternate old-game path.

## Task 8: Verify pause, restart, recovery, and origin lifecycle

**Files:** Modify app, browser, world, starting-area lifecycle, `web/main.js`, and `tests/simulation.rs`. Create `tests/starting_area.rs`.
**Consumes:** Task 7 complete game state and shared module lifecycle methods.
**Produces:** Deterministic restart, preserved recovery, active-player streaming, and safe input consumption.

- [ ] Write `edges_consumed_once`: six catch-up steps consume a toggle once; held starter input applies across the steps.
- [ ] Write `pause_freezes_complete_state`: bodies, doors, fluids, engine, and trip do not change during pause.
- [ ] Write `restart_after_carry_detach_rebase`: repeated restart restores supplies, attachments, origin, and entity counts without duplicate bodies.
- [ ] Write `recovery_preserves_state`: recovery retains quantities, condition, attachments, and trip, including exhausted-fuel station recovery.
- [ ] Run simulation, starting-area, and world tests. Confirm the new lifecycle checks fail before completing these rules.
- [ ] Consume edges and mouse deltas once. Retain pending deltas if no fixed step occurs; clear them on pause or capture loss.
- [ ] Rebase player, home, car, loose items, constraints, local aim targets, and both interpolation endpoints between physics steps.
- [ ] Keep home state outside chunk ownership. Restore its visuals from preserved state when the player returns.
- [ ] Clear carry constraints before recovery or restart. Remove old bodies/colliders on restart and rebuild one initial state.
- [ ] Restore clear recovery poses without restoring missing parts. Exclude recovery and rebasing from trip accumulation.
- [ ] Check capture rejection, Escape, focus loss, resume, and held input across each transition in the browser.
- [ ] Drive through chunk and origin thresholds, exit, walk away from the parked car, and return to the garage.
- [ ] Run `./scripts/check.ps1`. Commit the verified lifecycle changes.

The restart test captures initial counts and quantities, then repeats the destructive sequence three times:

```rust
assert_eq!(body_count_after_restart, initial_body_count);
assert_eq!(collider_count_after_restart, initial_collider_count);
assert_eq!(item_ids_after_restart, initial_item_ids);
assert_eq!(fluid_quantities_after_restart, initial_fluid_quantities);
assert_eq!(origin_after_restart, [0.0; 3]);
```

## Task 9: Complete acceptance and product records

**Files:** Modify `README.md`, `PRODUCT.md`, `DESIGN.md`, `docs/verification.md`, and any affected UI text.
**Consumes:** The implemented garage build and evidence from all prior tasks.
**Produces:** A tested local browser deliverable with documented limits and current controls.

- [ ] Run exporter tests, `./scripts/check.ps1`, and `./scripts/build.ps1`. Confirm successful outputs, not agent summaries.
- [ ] Serve with `node scripts/serve.mjs`. Test the complete garage-to-road flow through the actual UI.
- [ ] Check blocked exit, crouch clearance, sealed destination, direct filling, spilling, ignition tap/hold, overheating, wheels, pause, and restart.
- [ ] Compare home layout, car facing, wheel placement, doors, and interaction rules with recovered sources. Save screenshots and findings.
- [ ] Measure browser frame pacing at 1280 by 720 after warmup. Record asset sizes, loading time, renderer, and library allocation observations.
- [ ] Fix measured errors before broadening tests. Record remaining material and physics differences without a 1:1 completion claim.
- [ ] Update extraction/build commands, contextual controls, startup behavior, and scope. Remove obsolete road-spawn and direct seated-refuel text.
- [ ] Run a whole-branch review with a fresh reviewer. Resolve actionable findings and repeat only affected checks.
- [ ] Commit the verified deliverable. Use the finishing-branch skill to select integration without publishing or merging by inference.

## Execution handoff

The user authorized subagents where they reduce work time.
Use delegated asset, fluid, and player work with the ownership above. The primary owns physics contracts and integration.
Tasks 3 and 4 share only frozen interfaces and can run together. Serialize changes to the physics owner and app.
Review each completed module against the spec before integration. Use one final independent review after browser acceptance.
The user approved this plan before product implementation.
Implementation and native source acceptance are verified. The user requested that final browser route verification be skipped.
See `docs/verification.md` for results and limits.

## Verified API references

- [UnityPy](https://github.com/K0lb3/UnityPy) and [pygltflib](https://github.com/avaturn/pygltflib).
- [Bevy 0.19.1 glTF loader](https://raw.githubusercontent.com/bevyengine/bevy/v0.19.1/crates/bevy_gltf/src/loader/mod.rs).
- [Bevy 0.19.1 features](https://raw.githubusercontent.com/bevyengine/bevy/v0.19.1/crates/bevy_internal/Cargo.toml).
- [Bevy 0.19 migration](https://bevy.org/learn/migration-guides/0-18-to-0-19/).
- [Rapier 0.36 character controller](https://raw.githubusercontent.com/dimforge/rapier/v0.36.0/src/control/character_controller.rs).
- [Rapier 0.36 queries](https://raw.githubusercontent.com/dimforge/rapier/v0.36.0/src/pipeline/query_pipeline.rs).
