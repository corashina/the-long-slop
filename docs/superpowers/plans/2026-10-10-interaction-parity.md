# Home and Lada Interactions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the current home, Lada, and supplies usable through recovered interactions and common visible surfaces.

**Architecture:** Bind source components to collider actions during loading. Keep action resolution, carry, mechanisms, and parts separate. Execute actions in fixed steps and use the same action result for prompts.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, Rapier 0.36.0, WebAssembly, serde, Python 3.12, UnityPy, and Docker.

**Spec:** [Approved interaction specification](../specs/2026-10-10-interaction-parity-design.md).

## Global Constraints

- Use the separate `interaction-parity` worktree. Leave the primary checkout and other chats' changes untouched.
- Keep Rust, Bevy, Rapier, WebAssembly, and the selected installed assets.
- Keep decompiled C#, generated assets, and recovery evidence outside version control.
- Keep Steam files unchanged.
- Do not add old-manifest readers, migrations, or fallback exports.
- Cache bindings and query storage. Avoid per-frame sorting, heap allocation, and hierarchy scans.
- Do not add unrelated items, other vehicles, survival, weapons, inventory slots, saves, multiplayer, or deployment.
- Radio playback, horn audio, and live mirror reflections remain excluded.
- Do not add reduced-motion behavior.
- Write only critical tests. Reuse current physics, fluid, wheel, and lifecycle tests.
- Keep the preparation and driving route working after each layer.
- Use short sentences and the stop-slop skill for controls and documentation.

## Review Focus

- A cap or hubcap has its own use action but also belongs to a carryable or removable parent. Test Tasks 1 and 5.
- A pickup-only surface lies behind a solid wall with another action mask. Test Task 1.
- F starts pickup, then its release reaches a held-item handler. It must not drop or throw the new item. Test Task 6.
- A detached door carries its window, mirror, and colliders across an origin change. Test Tasks 5 and 7.
- A failed slot action or focus loss occurs during a held-input sequence. Keep identity and cancel stale actions. Test Tasks 5–7.

## Workspace and checks

Worktree: `C:/Users/Corashina/.codex/worktrees/interaction-parity/the-long-drive-wasm`.
Primary source cache: `C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/.cache/reference`.
Installed game: `C:/Program Files (x86)/Steam/steamapps/common/The Long Drive`.

All product writes, exports, build output, screenshots, and commits belong in the worktree.
Read the primary source cache. Do not reuse its browser package as the new build.
Use the using-git-worktrees skill to verify the existing worktree before execution. Do not create another checkout.
Inspect the Docker daemon and image before running the existing scripts.
Use the existing pinned Python packages when available. Otherwise create the documented worktree environment.
Initialize ignored assets by exporting from Steam into the worktree. Do not copy a stale manifest after changing its contract.

Use `./scripts/check.ps1 -Test interaction` for interaction regressions.
Use the matching target name for `reference`, `wheels`, `starting_area`, `engine`, and other suites.
Use `./scripts/check.ps1` for formatting, the complete native suite, and the release WASM check.
Run exporter tests with `python -m unittest discover -s tools/reference/tests` in the prepared Python environment.
Run installed-source gates through the existing Docker image with `cargo test --locked --test <target> -- --ignored`.
Mount this worktree at `/workspace`. Reuse `tld-wasm-cargo` and `tld-wasm-target` volumes.
Run `./scripts/build.ps1` for the browser package and `node scripts/serve.mjs` for localhost acceptance.

Each task must record the failing regression, passing checks, source evidence, and commit.
The assertion contracts below belong in the named regression tests. Reuse existing fixture setup.
Do not claim a task passes from an earlier task's output.

## File map and ownership

| File | Responsibility | Owner |
| --- | --- | --- |
| `tools/reference/actions.py` | Resolve source action and carry owners | Export implementer |
| `tools/reference/mechanisms.py` | Recover rotation, sliding, switches, and lamps | Export implementer |
| `tools/reference/parts.py` | Recover slots, covers, loose bodies, and part colliders | Export implementer |
| `tools/reference/manifest.py` | Assemble the typed contract | Coordinator |
| `src/reference.rs` | Validate shared records and IDs | Coordinator |
| `src/interaction/targets.rs` | Resolve actions and short prompt states | Gameplay implementer |
| `src/interaction/carry.rs` | Carry constraint, movement, recenter, and throwing | Gameplay implementer |
| `src/interaction/mechanisms.rs` | Rotation, sliding, and switches | Gameplay implementer |
| `src/interaction/parts.rs` | Part and slot state changes | Gameplay implementer |
| `src/presentation/mechanisms.rs` | Apply node movement and lamp state | Presentation implementer |
| `src/interaction.rs`, `src/starting_area.rs`, `src/app.rs` | Fixed-step and HUD integration | Coordinator |
| Existing tests and fixtures | Critical behavior and source acceptance | Task implementer |

Use the coordinator as the only writer for shared integration files during parallel work.
An implementer may own a complete task when no parallel writer uses its files.
Publish shared record fields before dispatching exporter, runtime, or presentation work.
Do not dispatch tasks with unstabilized dependencies.

### Task 1: Resolve source action owners and fix cap pickup

**Files:** Create `tools/reference/actions.py`, `src/interaction/targets.rs`. Modify `tools/reference/manifest.py`, `src/reference.rs`, `src/interaction.rs`, `src/starting_area.rs`, `src/app.rs`, and native fixtures. Test `tests/interaction.rs`, `tests/reference.rs`, and `tools/reference/tests/test_export.py`.

**Interfaces:**
- Add `ActionMasks` with `use_mask`, `pickup_mask`, `attach_mask`, and `push_mask`, all `u32`.
- Add `ActionBinding` with `collider: ColliderId` and optional use, pickup, removal, and push `ItemId` owners.
- Add `collider: ColliderHandle` to `AimTarget`. Preserve the hit collider independently from its resolved action owner.
- Add `ResolvedAim` with optional `AimTarget` values for use, grab, installation, filling, and push.
- Add `InteractionBlock` for covered, incompatible, occupied, and unavailable actions. Keep it copyable.
- Add `InteractionState::resolve(&self, physics: &PhysicsWorld, view: VehiclePose, items: &ItemStore) -> ResolvedAim`.
- Add `InteractionState::prompt(&self, aim: &ResolvedAim, items: &ItemStore) -> &'static str`.
- Replace `GarageRuntime.target` with the resolved result. Update existing callers; retain no legacy aim path.

- [ ] Add `cap_surface_resolves_use_and_carry_owner`. Assert cap use resolves to the cap and pickup resolves to its container.
- [ ] Add `pickup_mask_does_not_cross_a_solid_wall`. Place a pickup-only collider behind a wall and assert no grab target.
- [ ] Add exporter assertions for masks 12747265, 8388609, and 8552449. Recover the push mask from the same player component.

```rust
// cap_surface_resolves_use_and_carry_owner
assert_eq!(aim.use_target.map(|hit| hit.item), Some(cap));
assert_eq!(aim.grab_target.map(|hit| hit.item), Some(can));
// pickup_mask_does_not_cross_a_solid_wall
assert!(blocked_aim.grab_target.is_none());
```

- [ ] Run the interaction and exporter regressions. Confirm the cap and mask failures before changing production behavior.
- [ ] Implement source-owner extraction. Follow `pickupable`, `layerScript`, part links, and usable ancestors; validate links and masks.
- [ ] Bind collider handles to records during loading. Use action-specific queries with solid occlusion and cached predicate data.
- [ ] Route GRAB and USE through resolved owners. Replace HUD item-kind rules with the shared result. Register streamed pump bindings explicitly. Publish a general part-removal prompt only when Task 5 implements that action.
- [ ] Run interaction, input, reference, and existing source preparation checks. Require passing checks, then commit `Fix source interaction target ownership`.

### Task 2: Complete carry controls and wheel removal into hand

**Files:** Create `src/interaction/carry.rs`. Modify `src/interaction.rs`, `src/simulation.rs`, `src/starting_area.rs`, `src/reference.rs`, `tools/reference/actions.py`, `web/main.js`, and fixtures. Test `tests/interaction.rs`, `tests/input.rs`, and `tests/wheels.rs`.

**Interfaces:**
- Move `HeldState` and reusable constraint management into `CarryState`.
- Add `CarryState::new(reference: &ReferenceManifest) -> Result<Self, ReferenceError>`.
- Add `CarryState::pickup(&mut self, item: ItemId, point: [f32; 3], player: &PlayerState, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<(), ReferenceError>`.
- Add `CarryState::step(&mut self, input: FrameInput, player: &PlayerState, physics: &mut PhysicsWorld, dt: f32) -> bool` for forced release.
- Add `CarryState::drop(&mut self, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Option<ItemId>`.
- Add `WheelAssemblies::can_install(&self, item: ItemId, slot: WheelSlotId, items: &ItemStore) -> Result<(), ReferenceError>` before changing held state.
- Preserve `InteractionState::held()` and `drop_held()` as the public carry API. Remove the old private carry implementation.
- Add input bits for carry translation and recenter. Keep ROTATE for MMB; use RMB for translation and MMB+RMB for roll.
- Add typed item recenter position, rotation, distance, and recovered carry restrictions to source pickup records.

- [ ] Add `carry_translation_roll_and_recenter_respect_collision`. Assert each mode changes only the requested pose and cannot cross a wall.
- [ ] Add `removed_wheel_is_held_with_the_same_identity`. Assert missing support, held rim identity, attached tire identity, and unchanged condition.

```rust
// removed_wheel_is_held_with_the_same_identity
assert_eq!(interaction.held(), Some(rim));
assert!(!wheels.installed(slot));
assert_eq!(items.get(tire).unwrap().location, ItemLocation::Attached(rim));
assert_eq!(items.get(rim).unwrap().condition, condition_before);
```

- [ ] Run interaction, input, and wheel regressions. Confirm failure for the missing controls and held-wheel behavior.
- [ ] Extract the carry code. Bind MMB events and Tab in `web/main.js`; clear their input bits with the existing input reset.
- [ ] Apply source carry sensitivities and recenter offsets. Consume carry mouse motion before applying camera look.
- [ ] After successful wheel removal, call carry pickup before completing the fixed step. Preserve the acquisition press until release.
- [ ] Validate wheel installation before dropping its constraint. Rejected slots must keep the held item.
- [ ] Run the three suites and source fluid preparation checks, then commit `Complete carry controls and wheel pickup`.

### Task 3: Add home and cabin rotation and sliding

**Files:** Create `tools/reference/mechanisms.py`, `src/interaction/mechanisms.rs`, `src/presentation/mechanisms.rs`. Modify `tools/reference/manifest.py`, `src/reference.rs`, `src/interaction.rs`, `src/starting_area.rs`, and presentation registration. Test `tests/interaction.rs`, `tests/reference.rs`, `tests/starting_area.rs`, and exporter tests.

**Interfaces:**
- Add `MechanismId(u32)` and `MechanismReference` with source ID, use item, parent item, driven nodes, and typed rotation or slide data.
- Rotation data carries both authored axes, pivots, limits, inversion, and relative drag anchors.
- Slide data carries authored path transforms, initial value, limits, speed, and enabled rules.
- Add `MechanismState::new(reference: &ReferenceManifest) -> Result<Self, ReferenceError>`.
- Add `MechanismState::step(&mut self, item: ItemId, input: FrameInput, view: VehiclePose, dt: f32)`.
- Add `MechanismState::node_pose(&self, node: NodeId) -> Option<VehiclePose>` and `cancel_input(&mut self)`.
- Use mechanism poses for both collider placement and presentation. Replace the old hinge-state path when it covers the same mechanism.

- [ ] Add `mechanism_limits_move_visual_and_collision_pose`. Assert rotation clamps, sliding stops at endpoints, and both consumers read the same pose.
- [ ] Add `seated_window_drag_preserves_seat_and_driver_state`. Assert USE release does not enter or exit a seat after dragging.
- [ ] Add exporter source assertions for garage doors, windows, glovebox, ashtray, visors, and mirrors. Identify records by component source IDs.

```rust
// mechanism_limits_move_visual_and_collision_pose
assert!(angle >= min_degrees && angle <= max_degrees);
assert_eq!(visual_pose, collider_pose);
// seated_window_drag_preserves_seat_and_driver_state
assert_eq!(player.seat(), seat_before);
assert_eq!(player.driving(), driving_before);
```

- [ ] Run the regressions and confirm unsupported two-axis rotation and sliding failures.
- [ ] Recover linked transforms and paths. Remove name filters, the home usable allowlist, and the separate obsolete hinge representation.
- [ ] Implement source child-value links and enabled-state rules. Preserve cap and faucet handling in the existing fluid path.
- [ ] Drive both render nodes and moving collider poses through mechanism state. Keep signed scale and handedness conversion unchanged.
- [ ] Regenerate assets and run interaction, reference, exporter, and installed-source tests. Replace obsolete fixed hinge counts with required-source assertions.
- [ ] Verify the preparation route still runs, then commit `Add source home and cabin mechanisms`.

### Task 4: Add switches and visible vehicle lighting

**Files:** Modify `tools/reference/mechanisms.py`, `src/reference.rs`, `src/interaction/mechanisms.rs`, `src/presentation/mechanisms.rs`, `src/starting_area.rs`, and `src/app.rs`. Test interaction, reference, exporter, and browser source acceptance.

**Interfaces:**
- Add `ControlAction::{Handbrake(bool), Headlights(u8), InteriorLight(bool)}`.
- Add `MechanismState::take_control_action(&mut self) -> Option<ControlAction>`; process it once in `GarageRuntime::step`.
- Add typed lamp records for source parent item, transform, lamp type, color, intensity, range, cone, and control group.
- Add `VehicleControlState` with `headlights: u8` and `interior_light: bool`. Keep handbrake in the existing authoritative runtime field.
- Bind reusable Bevy lamp entities once. Presentation reads controls and attachment state; it does not change them.

- [ ] Add `aimed_handbrake_uses_the_space_state`. Assert either input changes the same value and neither input exits the seat.
- [ ] Add `light_switch_respects_electronics_and_source_states`. Assert off, low, high, and interior-light states follow recovered control data.

```rust
// aimed_handbrake_uses_the_space_state
assert_eq!(runtime.handbrake, expected_handbrake);
assert_eq!(runtime.area.player.seat(), seat_before);
// light_switch_respects_electronics_and_source_states
assert_eq!(controls.headlights, expected_source_state);
assert_eq!(effective_lamp_on, requested_on && electronics_enabled);
```

- [ ] Run interaction and exporter regressions before adding switch handling.
- [ ] Recover discrete switch links, lamp parameters, startup state, and electronics gates. Reject unresolved required lamp links.
- [ ] Implement switch actions and visible lamp changes. Allocate entities and materials during load, then reuse them.
- [ ] Keep audio-only controls excluded. Connect the HUD to the same switch eligibility result.
- [ ] Run native and exporter checks, build WASM, and verify light changes in the browser. Commit `Add aimed switches and vehicle lights`.

### Task 5: Add compatible Lada part removal and installation

**Files:** Create `tools/reference/parts.py`, `src/interaction/parts.rs`, and `tests/parts.rs`. Modify `src/reference.rs`, `tools/reference/manifest.py`, `src/interaction/items.rs`, `src/interaction/wheels.rs`, `src/interaction.rs`, `src/starting_area.rs`, `src/vehicle.rs`, `src/simulation/engine.rs`, and item presentation bindings.

**Interfaces:**
- Add `PartSlotId(u32)`, `PartReference`, and `PartSlotReference` with source IDs, type lists, cover links, collider states, and mount transforms.
- Add `SlotTarget::{Wheel(WheelSlotId), Part(PartSlotId)}`. Store this on resolved installation targets.
- Add `PartAttachments::new(reference: &ReferenceManifest) -> Result<Self, ReferenceError>`.
- Add `PartAttachments::bind(&mut self, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<(), ReferenceError>` for startup body allocation.
- Add `PartAttachments::can_install(&self, item: ItemId, slot: PartSlotId, items: &ItemStore) -> Result<(), InteractionBlock>`.
- Add `PartAttachments::can_remove(&self, item: ItemId, player: &PlayerState, items: &ItemStore) -> Result<(), InteractionBlock>`.
- Add `PartAttachments::remove(&mut self, item: ItemId, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<ItemId, ReferenceError>`.
- Add `PartAttachments::install(&mut self, item: ItemId, slot: PartSlotId, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<(), ReferenceError>`.
- Add `PartAttachments::installed(&self, item: ItemId) -> bool` and `sync(&mut self, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<(), ReferenceError>`.
- Keep one location per item. General parts use slot ownership; wheel support and rim/tire assembly remain in `WheelAssemblies`.
- Add engine and radiator presence to engine input. Store detached engine temperature with its stable item state.

- [ ] Add `hubcap_cover_blocks_wheel_removal_until_removed`. Assert the first F removes a source-removable cover; the next permits its wheel.
- [ ] Add `wrong_or_occupied_slot_keeps_held_part_and_contents`. Assert rejection preserves pose, fluid quantities, condition, and ownership.
- [ ] Add `occupied_seat_cannot_be_removed` and `nested_part_moves_with_parent`. Assert rejection and one live collider representation per part.
- [ ] Add `engine_and_radiator_removal_preserve_contents_and_change_operation`. Assert no drive or start without engine, no radiator cooling without radiator, and restored bindings after installation.

```rust
// wrong_or_occupied_slot_keeps_held_part_and_contents
assert_eq!(interaction.held(), Some(part));
assert_eq!(items.get(part).unwrap().location, location_before);
assert_eq!(items.get(part).unwrap().fluid, fluid_before);
// engine_and_radiator_removal_preserve_contents_and_change_operation
assert!(!output.running);
assert_eq!(output.torque_nm, 0.0);
assert_eq!(total_oil_after, total_oil_before);
assert_eq!(total_coolant_after, total_coolant_before);
```

- [ ] Run parts, wheel, and engine regressions. Confirm failures before implementing the attachment path.
- [ ] Recover all permitted selected-Lada slots and parts. Include doors, lids, seats, bumpers, mirrors, lamps, plates, hubcaps, engine, radiator, and eligible cabin parts.
- [ ] Precompute type compatibility and cover dependencies. Reuse source loose-body mass, damping, and collider groups.
- [ ] Prepare loose body and collider bindings at load. Commit removal or installation only after validation; acquire removed parts through `CarryState`.
- [ ] Move nested mechanisms and part visuals with parent ownership. Avoid duplicate chassis and loose colliders.
- [ ] Rebind vehicle fluids, engine presence, radiator effects, electronics, lamp parenting, and mass after each successful part change.
- [ ] Run parts, wheel, engine, fluid, exporter, and installed-source checks, then commit `Add source-compatible Lada part changes`.

### Task 6: Add push and charged throw

**Files:** Modify `tools/reference/actions.py`, `src/reference.rs`, `src/interaction/carry.rs`, `src/interaction/targets.rs`, `src/interaction.rs`, `src/starting_area.rs`, and input fixtures. Test `tests/interaction.rs`, `tests/input.rs`, and `tests/parts.rs`.

**Interfaces:**
- Add typed push records with owner, mask, force, grounded restrictions, and required grip data.
- Recover `pushablescript`, `pushgripable`, and `pickupable.Drop` through the existing analysis tools when their bodies are absent from the cache.
- Add `CarryState::throw_charge(&self) -> f32` and `cancel_input(&mut self)`.
- Add `CarryState::release_throw(&mut self, player: &PlayerState, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Option<ItemId>`.
- Add `InteractionState::cancel_input(&mut self)` for carry, mechanism, and push latches.
- Use `PhysicsWorld` and existing Rapier force-at-point support. Do not add a second physics world.

- [ ] Add `pickup_release_does_not_drop_or_throw_acquired_item`. Assert the acquisition F sequence retains the item.
- [ ] Add `held_f_charges_and_release_throws_once`. Assert charge starts only after acquisition release, follows source gain, and applies one release action.
- [ ] Add `source_push_wakes_car_without_bypassing_handbrake`. Assert point force and sleeping-body wake with the handbrake still set.

```rust
// pickup_release_does_not_drop_or_throw_acquired_item
assert_eq!(interaction.held(), Some(acquired_item));
assert_eq!(carry.throw_charge(), 0.0);
// source_push_wakes_car_without_bypassing_handbrake
assert!(!physics.rapier().bodies[physics.chassis_handle()].is_sleeping());
assert!(runtime.handbrake);
```

- [ ] Run regressions. Confirm missing throw and push behavior before adding it.
- [ ] Recover force semantics and source restrictions. Pin throw minimum 0.2, gain 1.5, and multiplier 1000 in the typed data.
- [ ] Implement F arbitration: compatible installation, allowed removal, pickup, push, then held-item drop or throw.
- [ ] Preserve an aimed incompatible-slot rejection. Require a new held-item F sequence before charging.
- [ ] Add required source grip behavior only if a selected push target needs it. Cancel push after release, lost target, or lost capture.
- [ ] Run interaction, input, parts, and native preparation checks, then commit `Add source push and throw actions`.

### Task 7: Preserve interactions through pause, restart, recovery, and origin changes

**Files:** Modify `src/starting_area.rs`, `src/interaction.rs`, `src/interaction/items.rs`, `src/interaction/parts.rs`, `src/interaction/mechanisms.rs`, and presentation synchronization. Test `tests/starting_area.rs`, `tests/input.rs`, and `tests/parts.rs`.

**Interfaces:**
- Call `InteractionState::cancel_input` from the existing pause and input-clear boundary.
- Add `PartAttachments::rebase(&mut self, shift: [f32; 3])` and `MechanismState::rebase(&mut self, shift: [f32; 3])` for stored world-space anchors only.
- Keep current `GarageRuntime::restart` and `recover` signatures. Extend their state coverage; add no second reset path.

- [ ] Add `focus_loss_cancels_throw_push_and_drag`. Assert resume produces no delayed force, release, or mouse movement.
- [ ] Add `detached_door_children_survive_rebase_and_recovery`. Assert window and mirror poses, colliders, fluids, ownership, and condition remain consistent.
- [ ] Add `restart_clears_constraints_and_restores_source_parts`. Assert no orphan joints, duplicate bodies, stale occupancy, or changed startup fluids.

```rust
// focus_loss_cancels_throw_push_and_drag
assert_eq!(carry.throw_charge(), 0.0);
assert_eq!(body_velocity_after_resume, control_body_velocity_after_resume);
// detached_door_children_survive_rebase_and_recovery
assert_eq!(items.get(window).unwrap().location, window_location_before);
assert_eq!(items.get(mirror).unwrap().location, mirror_location_before);
```

- [ ] Run the lifecycle regressions and inspect each missing state transition.
- [ ] Extend pause, restart, recovery, and origin handling using existing module boundaries and cached state.
- [ ] Run starting-area, input, parts, world, and native regression suites. Commit `Preserve interaction state through lifecycle changes`.

### Task 8: Verify the complete browser route and deliver the worktree

**Files:** Modify `README.md`, `docs/verification.md`, and the interaction research record. Save screenshots and measurements in ignored worktree evidence paths.

**Interfaces:** The finished browser receives input through `web/main.js`, reads authoritative Rust state, and displays resolved prompts.

- [ ] Run full formatting, native tests, exporter checks, installed-source gates, release WASM, and browser-package build. Require zero failures.
- [ ] Start a distinct localhost server for this worktree. Confirm its served package and manifest belong to this checkout.
- [ ] Walk from garage spawn. Pick up through a can-cap surface, adjust carry pose, and fill the Lada.
- [ ] Use garage doors, windows, mirrors, glovebox, visors, and switches through common visible aim zones.
- [ ] Remove and install a wheel through its source covering rules. Repeat with an engine or radiator and check operation.
- [ ] Push the Lada. Drop and throw an item. Enter, start, and drive through the garage exit.
- [ ] Pause with held input, resume, and restart with a removed part. Check origin changes and return-to-home state.
- [ ] Record screenshots, loading time, asset size, and frame pacing at 1280 by 720. Identify unavailable checks and their exact causes.
- [ ] Update controls and verification with short, source-backed text. Remove obsolete RMB-rotation and wheel-removal instructions.
- [ ] Review the full branch against the specification. Fix actionable findings and rerun checks affected by those fixes.
- [ ] Commit verified code and documentation. Report the worktree path, commits, checks, and remaining limits. Keep the primary checkout unchanged.

## Execution method

Use subagent-driven execution for independently owned exporter, gameplay, and presentation work after shared records are fixed.
The coordinator integrates shared files and verifies each working layer.
Keep dependent tasks sequential. Review each task before starting its dependent task.
Finish with a full-branch review and fresh verification.

## Plan self-review

The tasks cover source evidence, action ownership, carry, mechanisms, lights, general parts, push, throw, and lifecycle state.
Task 8 covers continuous browser acceptance and delivery evidence.
The five Review Focus conditions have named regression checks in their owning tasks.
Shared signatures and IDs are defined before their consumers. Existing wheel support remains one specialized module.
This plan adds no compatibility path, excluded gameplay system, or per-frame component discovery.
Implementation must wait for the user's plan review.
