# G09 player movement implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete source-faithful movement, low posture, lean, zoom, and Lada seat transitions within the garage scope.

**Architecture:** Extend the existing Rapier player controller. Resolve head pose, camera pose, and effective field of view from one player state. Keep posture, collision, saved state, and recovery consistent within each task.

**Tech Stack:** Rust, Rapier 0.36, Bevy 0.19, browser JavaScript, Python 3.12, unittest, Node test runner, Docker toolchain.

**Spec:** [Approved G09 specification](../specs/2026-10-20-g09-player-movement-design.md), commit `5dbf3b9`.

## Global constraints

- Steam build `11140890` and its recovered records define behavior.
- This task excludes clothing, body inverse kinematics, general settings, new vehicles, and driving-physics changes.
- Reuse Rapier queries and the existing controller. Add no dependencies.
- Update collider dimensions only when posture changes. Avoid per-frame shape allocation and repeated setup.
- Reject old schema versions. Add no compatibility paths or migrations.
- Full checks, release builds, and browser acceptance require a separate explicit request or agreed milestone.
- No such milestone is approved by this specification.
- Use the existing `codex/g09-player-movement` worktree. Keep it unmerged.
- Use GPT-6.1 Sol with high reasoning for implementation and review subagents.
- Apply AGENTS.md, technical-writing, and unslop to text changes.

## Review focus

- A clear standing capsule can hide a blocked intermediate sensor. Task 3 tests ordered sensor clearance and contact retention.
- The foot sphere can hit an obstacle that misses the capsule. Task 3 tests movement against both enabled shapes.
- Q cancellation can arrive without a release edge. Tasks 2 and 4 test cancellation without a lean toggle.
- Camera-forward and head-forward differ during Q look. Task 4 tests targeting, carried anchors, and exit search separately.
- Load preparation can use live-session FOV accidentally. Task 5 tests candidate projection independent from the active session.

## Execution and shared files

The user selected subagent execution. The plan still requires review before implementation starts.
Use one implementer per task, followed by specification and code review before proceeding.
Use separate reviewers as required by subagent-driven-development.
The parent owns scheduling and the final branch review.
Tasks are ordered. Do not run competing implementers in this worktree.
`src/player.rs`, `src/starting_area.rs`, and `src/app.rs` have one writer at a time.
Read-only source audits may run in parallel.

Each task has a failing check, implementation, passing check, and focused commit.
Do not activate posture before its save and recovery paths work.
Do not activate lean before its camera and head consumers migrate.
Do not activate zoom before its projection consumers migrate.

All commands below run from the G09 worktree.
Rust check commands include formatting, targeted native tests, and both WASM compile checks.
Format changed Rust before each targeted check.
A failing check must expose the planned missing behavior or API, not formatting or missing test inputs.
A successful WASM compile does not establish browser behavior.

## Files and responsibility

| Files | Responsibility | Task |
| --- | --- | --- |
| `tools/reference/player_movement.py`, `tools/reference/manifest.py`, `src/reference/player.rs`, `src/reference.rs` | Recover and validate player geometry, source values, and Lada view settings. | 1, 3, 5 |
| `tools/reference/tests/test_player_movement.py`, `tests/reference_loading.rs` | Source recovery and negative contract probes. | 1 |
| `web/input.js`, `web/main.js`, `src/browser.rs`, `src/simulation.rs`, `src/app.rs` | Sample player actions and preserve input edges. | 2 |
| `src/player/posture.rs`, `src/player/collision.rs`, `src/player.rs`, `src/starting_area.rs` | Continuous posture, complete collision geometry, movement, and source predicates. | 3 |
| `src/save/schema.rs`, `src/save/validate.rs`, `src/starting_area/save.rs`, `src/starting_area/recovery.rs` | Preserve posture through saves and recovery. | 3, 4 |
| `src/player/view.rs`, `src/player/pose.rs`, `src/interaction.rs`, `src/interaction/carry.rs`, `src/interaction/push.rs`, `src/audio.rs` | Resolve views and migrate source head or camera consumers. | 4 |
| `src/starting_area/inventory.rs`, `src/starting_area/survival.rs`, `src/app.rs`, `src/app/save.rs`, `src/presentation/save.rs` | Migrate existing view consumers without adding deferred features. | 4 |
| `src/presentation/camera_projection.rs`, `src/presentation/camera.rs`, `src/presentation/frame_graph.rs`, `src/presentation/view_frame.rs`, `src/presentation/save.rs`, `src/presentation/mod.rs` | Use effective FOV consistently across rendering and preparation. | 5 |
| `tests/player_movement.rs`, `tests/player_projection.rs`, `tests/web/input.test.mjs`, `tests/web/menu.test.mjs` | Critical behavior checks. | 2-5 |
| `web/index.html`, `web/feedback.js`, `docs/roadmap.md` | Controls and evidence report. | 2, 4, 6 |

## Task 1: Recover the source player contract

**Files:** Create `tools/reference/player_movement.py`, `tools/reference/tests/test_player_movement.py`, and `src/reference/player.rs`. Modify `tools/reference/manifest.py`, `src/reference.rs`, and `tests/reference_loading.rs`.

**Interfaces:**

- Python `recover_player_movement(source) -> dict` returns the new player fields.
- Python `recover_seat_view(source, reader) -> dict` returns the selected seat view fields.
- Python `validate_player_movement(value: dict) -> None` rejects missing or invalid data.
- `src/reference.rs` re-exports the new types from `reference::player`.
- `PlayerReference.posture: PlayerPostureReference` has `standing_head: Transform`, `prone_head: Transform`, `minimum_radius: f32`, `maximum_radius: f32`, `ctrl_value: f32`, `scroll_step: f32`, `rise_rate: f32`, `feet: PlayerSphereReference`, and `clearance: [PlayerClearanceReference; 10]`. Use the existing `reference::Transform` type.
- `PlayerReference.view: PlayerViewReference` has `head_lerp: f32`, `lean_sensitivity: f32`, `lean_distance: f32`, `height_limits: [f32; 2]`, `foot_fov_degrees: f32`, and `car_fov_degrees: f32`.
- Its source literal fields are `tap_seconds`, `return_window_seconds`, `return_rate`, `free_look_limit_degrees`, `zoom_multiplier`, `zoom_in_rate`, `zoom_out_rate`, `lean_scroll_multiplier`, and `height_scroll_step`, all `f32`.
- Validate their source values 0.2, 0.5, 10, 90, 0.4, 10, 20, 5, and 0.05 respectively.
- Add `prone_multiplier`, `duck_jump_multiplier`, `inverse_duck_multiplier`, and `max_fall_speed` as `f32` player fields.
- `PlayerSphereReference` contains `source: SourceId`, `center: [f32; 3]`, `radius: f32`, `membership: u32`, and `filter: u32`.
- `PlayerClearanceReference` contains `posture: f32` and `sphere: PlayerSphereReference`.
- `PlayerPostureReference.clearance` is `[PlayerClearanceReference; 10]`. Its `sensor_hold_steps: u8` is source value 3.
- `SeatReference` gains `default_yaw_degrees: f32` and `pitch_limits_degrees: [f32; 2]`. Validate the selected records' positional-lean mode.

- [ ] **Step 1: Check the isolated baseline and accepted inputs.**

Run `git status --short` and `git branch --show-current`. Require a clean `codex/g09-player-movement` checkout.
Run `./scripts/check.ps1 -WebTest input,menu` and `./scripts/check.ps1 -Filter simulation::step_clock_tests`.
Record existing failures before product edits. Do not repair unrelated defects.

Use the main checkout's accepted library and Python environment as read-only inputs.
Define these PowerShell variables for the remaining source commands:

```powershell
$g09Python = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/tools/reference/.venv/Scripts/python.exe'
$g09Library = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/web/assets/library'
$g09Output = Join-Path (Get-Location) 'web/assets/reference'
$env:G09_LIBRARY_ROOT = $g09Library
```

`G09_LIBRARY_ROOT` is a required installed-test input, not a product fallback.
Source tests mutate copied dictionaries, never accepted files.
Do not create shared cache writers or edit accepted manifest JSON manually.

- [ ] **Step 2: Add source recovery and native contract assertions.**

Python tests are `test_recover_fov_defaults`, `test_ordered_clearance_spheres`, `test_feet_parent_chain`, and `test_missing_data_prevents_promotion`.
Use the installed library fixture root supplied above. Reuse `LibrarySource` and the existing source-test patterns.
Assert these recovered values:

```python
self.assertEqual(value['view']['foot_fov_degrees'], 65.0)
self.assertEqual(value['view']['car_fov_degrees'], 65.0)
self.assertEqual(len(value['posture']['clearance']), 10)
self.assertEqual(value['posture']['sensor_hold_steps'], 3)
self.assertEqual(value['posture']['feet']['filter'], 4172190207)
self.assertEqual(value['posture']['clearance'][0]['sphere']['filter'], 4037970431)
```

Add ignored native test `installed_player_movement_contract` in `tests/reference_loading.rs`.
Check source IDs, ordered fractions 0.1 through 1, sensor radius 0.15, and all five Lada seat limits.
Mutate sensor order, radius, mask, missing view data, and nonfinite geometry. Require Rust rejection.
Require Python rejection for the corresponding mutated fields.

- [ ] **Step 3: Run the source tests and confirm the missing contract fails.**

Run `& $g09Python -m unittest tools.reference.tests.test_player_movement -v`.
Require a failure for missing recovery behavior, rather than an unavailable library or tool.
The new ignored native test must later run explicitly. A skipped test is not a passing result.

- [ ] **Step 4: Recover and validate the exact source data.**

Read settings sliders `level0:16865.tabMisc.sliders[1]` and `[2]`, not the authored Camera FOV.
Recover foot sphere `level1:57258` through its parent chain with the existing transform conversion.
Recover clearance prefab `sharedassets1.assets:42627`, its sphere `124758`, and source layer 14.
Recover layer masks from `globalgamemanagers:10`. Preserve source layer 12 for the foot sphere.
Generate sensor centers with the source fractions and head anchors.
Use `survival.managed_evidence(source, assertions)` for managed literals, including the `sens` counter.
Keep source IDs and current coordinate-conversion conventions.
Validate all five Lada seats as positional lean, yaw limits ±179, default yaw 0, pitch limits ±89, and exit angle 65.
Keep existing active player fields until Tasks 3 and 5 migrate their consumers. Add no compatibility parser.

- [ ] **Step 5: Derive branch-local runtime assets and run the contract checks.**

```powershell
& $g09Python -m tools.reference.export --stage runtime --library-root $g09Library --output $g09Output
& $g09Python -m tools.reference.export --stage runtime --library-root $g09Library --output $g09Output --verify
& $g09Python -m unittest tools.reference.tests.test_player_movement -v
./scripts/check.ps1
```

The last script performs formatting and both WASM checks. Run the ignored native test explicitly in the toolchain:

```powershell
. ./scripts/cargo-target.ps1
$g09Target = Get-CargoTargetDirectory (Get-Location).Path
docker run --rm --mount "type=bind,source=$((Get-Location).Path),target=/workspace" --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' --mount 'type=volume,source=tld-wasm-target,target=/target' --env "CARGO_TARGET_DIR=$g09Target" my-summer-slop-toolchain cargo test --locked --test reference_loading installed_player_movement_contract -- --ignored
```

Require one executed native contract test, zero failures, and both WASM checks passing.
This exporter verifies the accepted library before derivation. Any provenance failure stops export.
Do not invoke source, catalog, or library stages. Do not bypass failed provenance or rebuild the library without approval.
Only branch-local `web/assets/reference` and `web/assets/runtime/<package-id>` may change.

- [ ] **Step 6: Review and commit the source contract.**

Stage only this task's source and tests. Commit `feat: recover source player movement contract`.

## Task 2: Carry player input through the browser bridge

**Files:** Modify `web/input.js`, `web/main.js`, `src/browser.rs`, `src/simulation.rs`, `src/app.rs`, `tests/web/input.test.mjs`, `tests/web/menu.test.mjs`, `web/index.html`, and `web/feedback.js`.

**Interfaces:**

- `simulation::player_keys::{LEAN, ZOOM}: u8` have values 1 and 2.
- `FrameInput` gains `player_held`, `player_pressed`, and `player_released`, all `u8`.
- Browser frame fields are `playerHeld`, `playerPressed`, and `playerReleased`.
- `window.gamePlayerKeys() -> number` packs held in bits 0-1, pressed in 2-3, and released in 4-5.
- `browser::game_player_keys() -> u32` imports that getter. Sample `game_keys()` first, then read the packed value once.
- Preserve existing primary, driving, inventory, mouse, and wheel interfaces.

- [ ] **Step 1: Add failing input and cancellation assertions.**

Add `zoom_aliases_release_only_after_last_key`, `clear_cancels_q_without_release`, and `source_item_mouse_buttons`.
Extend menu capture-loss tests with Q held for 0.1 seconds, cancellation, and a later physical key release.

```js
const i=new InputState([]);
i.code('KeyZ',true);i.takeFrame();i.code('KeyY',true);i.takeFrame();
i.code('KeyZ',false);
assert.equal(i.takeFrame().playerHeld,2);
i.code('KeyY',false);
assert.equal(i.takeFrame().playerReleased,2);
i.code('KeyQ',true);i.clear();i.code('KeyQ',false);
assert.equal(i.takeFrame().playerReleased,0);
```

Add native `simulation::frame_input_tests::player_edges_wait_for_one_step_and_clear_cancels_them`.
Merge Q press, then zoom press and Q release. Assert `(held, pressed, released) == (2, 3, 1)`.
The next step must contain `(2, 0, 0)`. Clear must return `FrameInput::default()`.

- [ ] **Step 2: Run `./scripts/check.ps1 -WebTest input,menu` and confirm the missing fields fail.**

- [ ] **Step 3: Implement player sampling and correct mouse mappings.**

Map Q to LEAN and Z/Y to ZOOM. Recompute aliases from physical key state.
Merge edge fields with OR. Replace held state. `take_step` retains held and clears edges.
`InputState.clear()` discards edges without creating a Q release.
Existing menu blur, visibility, and pointer-loss handlers must continue to request runtime pause.
Set `Mouse1` to existing TRANSLATE bit 16384 and `Mouse2` to ROTATE bit 4096.
Update mouse-control help atomically. Add Q and zoom help when Task 4 or Task 5 activates those actions.

- [ ] **Step 4: Run the web suites and `./scripts/check.ps1 -Filter simulation::frame_input_tests`.**

Require input alias, clear, menu cancellation, native edge buffering, and both WASM checks to pass.

- [ ] **Step 5: Review and commit `feat: carry source player actions through browser input`.**

## Task 3: Add continuous posture, collision, movement, and saved state

**Files:** Create `src/player/posture.rs`, `src/player/collision.rs`, and `tests/player_movement.rs`. Modify `src/player.rs`, `src/starting_area.rs`, `src/starting_area/recovery.rs`, `src/starting_area/save.rs`, `src/save/schema.rs`, `src/save/validate.rs`, and the reference files from Task 1.

**Interfaces:**

- `PostureState::new(value: f32) -> Self`, `value(&self) -> f32`, and `restore(&mut self, value: f32)` are internal.
- `PostureState` owns source Ctrl, rising, wheel persistence, airborne duck, and sensor-retention state.
- `PlayerState::posture(&self) -> f32` replaces the ambiguous Boolean posture accessor.
- `collision::PlayerSweepShape` implements Parry `Shape`, `CompositeShape`, and `TypedCompositeShape` for the fixed capsule and foot-sphere union.
- `PlayerSweepShape::new(reference: &PlayerReference) -> Self` builds its two-leaf BVH once.
- `PlayerSweepShape::set_posture(&mut self, reference: &PlayerReference, value: f32)` updates inline primitives and poses without allocation.
- `PlayerStepContext` has `wheel_posture_allowed: bool`, `mouse_consumed: bool`, `picked_up: bool`, `recenter_consumed: bool`, and `movement_multiplier: f32`.
- Change `PlayerState::step(&mut self, input: FrameInput, context: PlayerStepContext, physics: &mut PhysicsWorld, dt: f32)` and its caller together.
- `PlayerState::head_pose_at(&self, body_pose: VehiclePose) -> VehiclePose` previews posture geometry with neutral temporary view state.
- `PlayerSnapshot.posture: f32` replaces `crouched`. Schema version becomes 7.
- Test-local `garage() -> (ReferenceManifest, GarageRuntime, SourceStamp)` reads the installed branch export. Build its stamp as in `tests/inventory.rs`.

- [ ] **Step 1: Add failing posture and complete-geometry checks.**

Module tests cover posture endpoints, Ctrl rising from 0.2, wheel persistence, and the sensor contact counter.
Use synthetic obstacles in `tests/player_movement.rs` against an installed source player.
One obstacle must miss the main capsule and hit only the enabled foot sphere.
Compare adapter sweeps with newly constructed standard compounds at representative postures.
Include grounding, dynamic-body contact, and the foot-enable threshold. Test-only compound construction may allocate.
Add `source_posture_collision_and_locomotion` and `source_posture_save_and_recovery` integration tests.
Assert empty-handed speed caps 1.75 standing, 4.375 running, 1.05 crouched, and 0.525 fully low.
Check ground jump and duck-jump velocity changes against force divided by mass times the source interval.
Check one airborne duck jump, its inverse-release force, and reset on landing.

```rust
assert!((run.area.player.posture() - 0.17).abs() < 1.0e-6);
let saved = run.capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-20T10:00:00Z").unwrap();
assert_eq!(saved.player.posture, run.area.player.posture());
let restored = GarageRuntime::from_save(&r, &stamp, &saved).unwrap();
assert_eq!(restored.area.player.posture(), saved.player.posture);
```

Drive posture to 0.17 through gameplay input before capture. Do not add a production test setter.
Validate geometry at 0, 0.17, 0.3, and 1. Check foot enablement strictly above 0.3.
Require rejection of schema 6, posture outside `[0,1]`, and NaN or infinity in an in-memory snapshot.
Failed recovery must preserve player pose, posture, and carried-object poses.

- [ ] **Step 2: Run `./scripts/check.ps1 -Test player_movement` and record the missing behavior failure.**

Tests requiring branch-local assets are not ignored. Missing export must fail clearly.

- [ ] **Step 3: Implement posture and complete player geometry.**

Keep body-root, lower anchor, capsule center, head anchor, and foot-sphere pose distinct.
Evaluate ten ordered source spheres with their source masks and fixed-step contact retention.
Match `sens.FixedUpdate` and `OnTriggerStay`, including counter 3 and its clearing step.
Ctrl below 0.3 requests clearance-limited growth. Ctrl above 0.3 lowers immediately.
Wheel-selected posture remains stable without input. Automatic rise uses rate 5.
Mutate unique owned shapes through `PhysicsWorld::edit_collider` and refresh queries after geometry changes.
Use existing `vehicle::set_collider_enabled` so disabled colliders also receive correct solver groups.
Use source membership and filter groups for movement and clearance queries. Exclude the player's own rigid body.
Remove obsolete standing/crouching shape ownership and derived-only fields after migrating all users.
The controller must constrain every enabled source player shape, including Feet2, without per-step allocation.
Parry 0.31.1 standard compounds require reconstruction when children change. Do not rebuild one during posture updates.
Use the fixed two-part `PlayerSweepShape` query adapter with the existing character controller.
Its BVH leaves conservatively enclose both source head endpoints and the maximum radius.
Expose current capsule as part 0. Expose foot sphere as part 1 only when enabled.
Return the tight current AABB. Delegate ray and point queries through Parry composite query support.
Pass the union once to `move_shape` and collision-impulse resolution. Do not combine separate controller movement results.
Physical colliders remain separate and uniquely own their mutable primitive shapes.
Keep total player mass at the source 72 kg. The foot collider must not add mass.
Assert unchanged mass across all tested posture values.

- [ ] **Step 4: Integrate locomotion, save schema 7, and recovery atomically.**

Use the continuous source speed curve, base 1.75, run factor 2.5, and normalized movement axes.
Compute carried-weight movement from existing mass bounds and the actual picked-up mass.
Apply jump forces through source mass and force interval. Preserve source airborne duck and inverse-release rules.
Move posture work before push-grip's early return without changing G07 push-force ownership.
Seat entry and seated updates use posture 1. Exit restores valid standing geometry.
Recovery checks complete enabled geometry and previews the resulting neutral head pose before committing transport.
Save capture and restore use continuous posture. Clear temporary transition input after load.
Remove every `crouched` snapshot consumer and reject the obsolete schema without migration.

- [ ] **Step 5: Run `./scripts/check.ps1 -Test player_movement`.**

Include posture module tests with `./scripts/check.ps1 -Filter player::posture` if the integration suite does not execute them.
Require the focused native checks and both WASM checks to pass. Do not claim browser movement parity.

- [ ] **Step 6: Review and commit `feat: preserve continuous source player posture`.**

## Task 4: Add lean, independent look, and seat transitions end to end

**Files:** Create `src/player/view.rs`. Modify `src/player.rs`, `src/player/pose.rs`, `src/starting_area.rs`, `src/interaction.rs`, `src/interaction/carry.rs`, `src/interaction/push.rs`, and `src/audio.rs`.
Also modify `src/starting_area/inventory.rs`, `src/starting_area/survival.rs`, `src/starting_area/recovery.rs`, `src/app.rs`, `src/app/save.rs`, and `src/presentation/save.rs`.
Extend `tests/player_movement.rs`, `web/index.html`, and `web/feedback.js`.

**Interfaces:**

```rust
#[derive(Clone, Copy)]
pub struct PlayerView {
    pub head_pose: VehiclePose,
    pub camera_pose: VehiclePose,
    pub fov_degrees: f32,
    pub ray_actions_blocked: bool,
}
```

- `PlayerState::view(&self, physics: &PhysicsWorld) -> PlayerView` is the only resolved view accessor.
- `PlayerState::neutral_view_at(&self, body_pose: VehiclePose) -> PlayerView` replaces Task 3's head-only preview.
- `PlayerState::cancel_view_input(&mut self)` clears temporary view state while preserving posture.
- Internal `ViewState` owns Q charge, independent angles, lean mode, local offsets, return timing, and head smoothing.
- Initially `fov_degrees` is the selected foot or car base FOV. Task 5 activates zoom atomically with projection changes.

- [ ] **Step 1: Add failing view, routing, and seat assertions.**

Add `q_tap_boundary_and_cancel`, `free_look_preserves_body_and_head_anchors`, `seated_lean_blocks_ray_actions`, and `seat_target_press_release_matches`.
Test Q release at 0.19 and 0.2 seconds, plus a canceled 0.1-second charge.
Check independent look clamps at ±90 and source lean clamps at radius 0.6.
Test all source wheel predicates, including carry plus Q without RMB, tuning, and slideable mechanisms.
One scroll sample must change both standing lean and posture when source allows both.
Check positive wheel above posture 0.95 also changes the bounded head-height offset.

```rust
let view = run.area.player.view(&run.physics);
assert_ne!(view.camera_pose.rotation, view.head_pose.rotation);
assert!(!view.ray_actions_blocked);
assert_eq!(run.area.player.body_pose(&run.physics).rotation, body_before.rotation);
```

For exit, create differing camera and head directions. Test permission at exactly 65 degrees and below it.
Place an obstacle only along head-forward to distinguish the placement search from the permission angle.
Set parent velocity to `[2., 1., -3.]`. Require inherited velocity `[0.6, 0.3, -0.9]` within float tolerance.

- [ ] **Step 2: Run `./scripts/check.ps1 -Test player_movement` and confirm the missing view behavior fails.**

- [ ] **Step 3: Implement Q state and migrate all view consumers together.**

Use source sensitivity 0.15, distance 0.6, head tracking rate 15, and free-look return rate 10.
Separate body heading, smoothed head pose, and camera-local independent look.
Use camera pose for use, pickup, attachment, crosshair, push rays, and rendered view where source uses camera.
Use head pose for head-space carry anchors, audio sensors, and existing retained player anchors.
Verify each source consumer before choosing its frame. Do not redirect throw or release directions by assumption.
Exit permission uses camera-forward. Exit search uses head-forward.
Remove `view_pose` and migrate all its callers, including existing survival consumers, without expanding survival behavior.
Keep world rebasing on world positions and local lean offsets unchanged.

- [ ] **Step 4: Integrate context, cancellation, and seat rules.**

Compute Task 3's context from actual interaction state. Preserve shared wheel effects rather than clearing wheel globally.
Suppress source ray actions during held seated positional lean. Foot Q look does not set that block.
Match seat entry to the pressed usable target and source short-hold limit.
Apply seated Tab recenter after carried-object priority, with source default yaw and pitch limits.
Apply exit velocity across every axis. Keep failed entry and failed exit atomic.
Call `cancel_view_input` from `GarageRuntime::clear_input` and successful recovery or load boundaries.
Preserve posture through pause and resume. A later physical Q release must not toggle lean after cancellation.
Add Q-hold, Q-tap, and wheel-posture help text.

- [ ] **Step 5: Run `./scripts/check.ps1 -Test player_movement` and `./scripts/check.ps1 -Filter player::view`.**

Run `./scripts/check.ps1 -WebTest input,menu` after the help and cancellation integration.
Require native source-transition checks, web checks, and both WASM checks to pass.

- [ ] **Step 6: Review and commit `feat: add source lean and independent player look`.**

## Task 5: Apply zoom to every player projection

**Files:** Modify `src/player/view.rs`, `src/player.rs`, `src/app.rs`, `src/app/save.rs`, `src/presentation/camera.rs`, `src/presentation/frame_graph.rs`, `src/presentation/view_frame.rs`, `src/presentation/save.rs`, `src/presentation/mod.rs`, and obsolete FOV reference consumers. Create `src/presentation/camera_projection.rs` and `tests/player_projection.rs`.

**Interfaces:**

- `PlayerView.fov_degrees` becomes the selected base FOV times the source zoom multiplier.
- `world_camera_settings(fov_degrees: f32) -> WorldCameraSettings` retains existing near, far, and mask settings.
- `camera_projection::sync_player_projection(projection: &mut Projection, player_view: bool, fov_degrees: f32)` updates only player views.
- `camera_projection::player_perspective(fov_degrees: f32, aspect_ratio: f32) -> PerspectiveProjection` is shared by live and candidate preparation.
- The small projection module depends only on Bevy camera types. Native tests path-import the actual module, as `tests/early_culling.rs` does.

- [ ] **Step 1: Add failing zoom and real projection tests.**

Extend `player::view` tests for dedicated zoom while carrying and empty-hand-only RMB zoom.
At `dt=0.02`, multiplier 1 approaches 0.4 by factor 0.2, producing 0.88.
A neutral update then approaches 1 by factor 0.4, producing 0.928.
Check normal look sensitivity 0.4 while zoom is requested. Q independent look retains source sensitivity.
In `tests/player_projection.rs`, test the actual sync helper on world, sky, and probe projections.

```rust
sync_player_projection(&mut world, true, 26.);
sync_player_projection(&mut sky, true, 26.);
sync_player_projection(&mut probe, false, 26.);
let Projection::Perspective(world) = world else { panic!("Expected perspective") };
let Projection::Perspective(sky) = sky else { panic!("Expected perspective") };
let Projection::Perspective(probe) = probe else { panic!("Expected perspective") };
assert!((world.fov - 26_f32.to_radians()).abs() < 1.0e-6);
assert!((sky.fov - 26_f32.to_radians()).abs() < 1.0e-6);
assert!((probe.fov - 90_f32.to_radians()).abs() < 1.0e-6);
```

Test live FOV 26 against a separately restored candidate FOV 65. Candidate preparation must use its own view.
Assert unchanged near/far planes, world mask, shadow settings, and probe projection.

- [ ] **Step 2: Run `./scripts/check.ps1 -Test player_projection` and confirm the missing projection behavior fails.**

- [ ] **Step 3: Activate zoom and migrate every projection consumer together.**

Use factors 10 while zooming and 20 while releasing. Apply source interpolation clamping.
Compute zoom request from dedicated Z/Y or RMB with an empty picked-up slot.
Use distinct recovered foot and car base FOVs, both currently 65.
Sync world and sky Bevy projections before camera/frustum publication.
Update `view_frame` world snapshots and both sky matrices using the same effective FOV.
Use effective FOV for live LOD, required views, and candidate load preparation.
Keep probe faces at 90 degrees. Preserve chase and sleep pose selection without first-person lean offsets.
Prepared loads use the candidate's restored posture and neutral zoom, independent of active-session zoom.
Remove the old single `PlayerReference.field_of_view_degrees` field and remaining exporter consumers.
Add Z/Y and empty-hand RMB zoom help.

- [ ] **Step 4: Run `./scripts/check.ps1 -Test player_projection` and `./scripts/check.ps1 -Filter player::view`.**

Rerun affected Python recovery tests after the reference field removal. Derive and verify the branch-local runtime again.
Require projection-function tests, source recovery checks, and both WASM checks to pass.
These checks establish code contracts. Browser appearance and normal-input acceptance remain unverified.

- [ ] **Step 5: Review and commit `feat: synchronize source zoom across player views`.**

## Task 6: Verify the final branch and record evidence

**Files:** Modify `docs/roadmap.md`. Modify implementation files only to fix failures from this task's review.

**Interfaces:** No new product interfaces. Use the accumulated targeted suites and source contracts.

- [ ] **Step 1: Compare the final diff with every approved specification section.**

Check `rg -n 'view_pose|crouched|field_of_view_degrees|standing_shape' src tests tools/reference --glob '*.rs' --glob '*.py'`.
Inspect each match. Remove obsolete G09 paths without deleting unrelated FOV fields or source names.
Require one resolved view result, complete collision geometry, schema 7, and source input priority.

- [ ] **Step 2: Run only the targeted checks still needed for the final combined state.**

Reuse recorded results when no later change affects the checked contract.
After new fixes, rerun affected source, input, posture, transition, save, or projection checks.
Run `./scripts/check.ps1 -Test player_movement` against the final source contract.
Run `./scripts/check.ps1 -WebTest input,menu` if runtime cancellation or browser integration changed since its last check.
Record exact commands, executed test counts, failures, and required WASM results.
Do not run `scripts/check.ps1 -Full`, `scripts/build.ps1`, or browser acceptance without separate authorization.

- [ ] **Step 3: Get a whole-branch review with a fresh subagent.**

Review source parity, head/camera consumers, geometry mutation tracking, input cancellation, and private load-candidate state.
Fix actionable failures and rerun their targeted checks before closing the review.

- [ ] **Step 4: Update the G09 roadmap evidence.**

Describe implemented controls, continuous posture, seat behavior, and saved posture.
Record source checks and limits. Keep runtime acceptance explicitly open.
Change G09 from Partial to Verify only if no scoped code gap remains.
A build, native test, or scripted state is not normal-input browser acceptance.

- [ ] **Step 5: Commit `docs: record G09 implementation checks and runtime limits`.**

Leave the branch unmerged. Report changes, checks, failures, deferred checks, and remaining runtime limits.

## Plan review handoff

The user has already selected worktree and subagent execution.
Review and approve this written plan before implementation starts.
After approval, use subagent-driven-development with the task order and file ownership above.
