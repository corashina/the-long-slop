# Correct Driving Physics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete source-based driving controls and wheel behavior for the existing Lada and Skoda.

**Architecture:** Keep Rapier bodies and collision. Give each car one drivetrain state and one four-slot wheel owner.
Connect source records, input, engine output, wheel forces, saves, and browser feedback through the existing runtime.

**Tech Stack:** Rust 1.99, Rapier 0.36.0, Bevy 0.19.1, serde, Python, JavaScript, WASM, and the existing Docker toolchain.

**Spec:** [Approved driving specification](../specs/2026-10-14-correct-driving-physics-design.md).
**Status:** Awaiting plan review and execution-method selection.
**Base:** Master `36ba94b`, including specification commit `36ba94b`.
**Execution recommendation:** Native execution suits the shared drivetrain, wheel, and save interfaces. Review the complete branch after implementation.

## Global Constraints

- Use installed Steam build `11140890`, Unity `2021.3.18f1`, as the source.
- Keep each layer playable before adding the next layer.
- Use the existing 60 Hz simulation step and pinned dependencies.
- Implement in a separate worktree based on the current master.
- Limit menu work to driving assists, driving feedback, and the required control help.
- Exclude control rebinding, controller axes, H-pattern hardware, new cars, towing, and multiplayer.
- Exclude deferred source-world integration and unrelated save gaps.
- Keep merging and pushing outside this implementation request.
- Do not allocate, rebuild maps, or create physics objects during ordinary wheel updates.
- Add no migration, old reader, or fallback path.
- Follow AGENTS.md. Apply stop-slop to documentation and UI text.
- Leave unrelated work, including the inventory and clothing audit, unchanged.

## Review Focus

1. A press and release between physics steps must shift once. Task 2 checks pending edges and unsigned bit 31.
2. Simultaneous pedals must retain both values through reverse assists. Tasks 2 and 3 check inversion and clutch precedence.
3. Removing a tire must change contact configuration without clearing other wheel motion. Task 5 checks rims, hubs, and cross-car exchange.
4. A rejected save must leave the active car unchanged. Task 3 checks gear validation before scene replacement.
5. Contact with another moving body must use relative velocity and exchange impulses. Task 5 checks momentum and self-filtering.

## File boundaries

| Files | Responsibility |
| --- | --- |
| `tools/reference/driving.py`; `tools/reference/manifest.py`; `tools/reference/export.py` | Derive driving records and required resources through the existing exporter |
| `src/reference/driving.rs`; `src/reference.rs` | Define and validate driving, wheel, and binding contracts |
| `src/simulation/input.rs`; `src/simulation.rs`; `web/input.js` | Fixed-step pedal ramps and source keyboard actions |
| `src/simulation/drivetrain.rs`; `src/simulation/engine.rs` | Transmission state, clutch coupling, and torque output |
| `src/starting_area.rs`; `src/starting_area/save.rs` | Per-car runtime connection, lifecycle, and save reconstruction |
| `src/save/schema.rs`; `src/save/validate.rs` | Required driving state and validation |
| `src/vehicle/wheels.rs`; `src/vehicle.rs` | Contact, suspension, tire force, spin, snapshots, and Rapier integration |
| `src/interaction/wheels.rs` | Publish installed tire, rim, hub, mass, and condition changes |
| `src/browser.rs`; `src/app.rs`; `web/main.js`; `web/menu.js`; `web/feedback.js` | Assist requests and authoritative driving feedback |
| `web/index.html`; `web/style.css`; `README.md` | Two assist controls, gear display, and control help |
| Existing tests and `tests/drivetrain.rs` | Critical source, input, drivetrain, wheel, browser, and save checks |

Do not split unrelated runtime or presentation modules during this work.
Keep the current engine, mass, attachment, save-publication, and browser-menu owners.

## Execution setup

Use the using-git-worktrees skill after the user approves this plan and selects an execution method.
Inspect attached worktrees first. Use the native worktree tools for a new checkout on `codex/correct-driving-physics`.
Start from the committed plan revision on current master, including later independent master changes that precede execution.
Inspect those changes before choosing the exact base. Preserve unrelated dirty files in the primary checkout.
Record the selected base and isolated checkout path before editing product code.
Prepare the accepted ignored reference data and browser assets for that checkout using the existing reference workflow.
Keep writable export output inside the new checkout. Preserve the primary source library and installed game files.
Run the existing baseline checks before Task 1. Report baseline failures before treating them as task regressions.

## Verification commands

Run commands from the implementation worktree in PowerShell.
Reuse the checked primary reference Python environment without installing packages.
Python imports the implementation checkout's modules because commands run from that checkout.

```powershell
. ./scripts/cargo-target.ps1
$drivingRoot = (Get-Location).Path
$drivingTarget = Get-CargoTargetDirectory $drivingRoot
$drivingSourceRoot = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm'
$drivingPython = Join-Path $drivingSourceRoot 'tools/reference/.venv/Scripts/python.exe'
```

For each Rust target below, use `./scripts/check.ps1 -Test <target>`.
The script selects the worktree target cache and returns a nonzero code on failure.
Run `./scripts/check.ps1` for the final native, browser, formatting, and WASM gate.
Run `./scripts/build.ps1` to package the release browser build.

Run source and exporter checks with:

```powershell
& $drivingPython -m unittest tools.reference.tests.test_driving
& $drivingPython -m tools.reference.export --stage runtime
& $drivingPython -m tools.reference.export --stage runtime --verify
```

Run required ignored source tests in the existing container:

```powershell
docker run --rm --pull=never --mount "type=bind,source=$drivingRoot,target=/workspace,readonly" --mount type=volume,source=tld-wasm-cargo,target=/cargo --mount type=volume,source=tld-wasm-target,target=/target --env "CARGO_TARGET_DIR=$drivingTarget" my-summer-slop-toolchain cargo test --locked -- --ignored
```

Require exit code zero and no failed tests. Report skipped or unavailable checks as such.
Do not infer runtime acceptance from these commands.

## Task 1: Recover and validate the driving contract

**Files:** Create `tools/reference/driving.py`, `src/reference/driving.rs`, and `tools/reference/tests/test_driving.py`.
Modify `tools/reference/manifest.py`, `tools/reference/export.py`, `src/reference.rs`, `tests/reference.rs`, and `tests/common/reference.json`.

**Interfaces:**
- Add `ReferenceManifest.driving: DrivingReference` and `VehicleReference.driving: DrivingVehicleReference`.
- `DrivingReference` contains `source: SourceId`, `pedal_rate: f32`, `force_manual: bool`, `min_motor_torque_nm: f32`, `default_assists: DrivingAssists`, and `bindings: Vec<DrivingBinding>`.
- `DrivingAssists` contains `automatic_clutch: bool` and `automatic_gears: bool`.
- `DrivingBinding` contains `action: DrivingAction` and `codes: Vec<String>` for checked browser keyboard codes.
- `DrivingAction` covers clutch, shift up/down, reverse, neutral, and gears one through six.
- `DrivingVehicleReference` contains `initial_gear: usize`, `force_automatic: bool`, and `play_shift_sounds_in_automatic: bool`.
- Add `WheelFrictionCurve` with `extremum_slip`, `extremum_value`, `asymptote_slip`, `asymptote_value`, and `stiffness`, all `f32`.
- Replace the five-value friction arrays with required `WheelFrictionCurve` fields; replace positional Python field iteration with named reads.
- Add `WheelHubReference` with `radius_m`, `mass_kg`, `forward_stiffness`, and `sideways_stiffness`, all `f32`.
- Extend `WheelSlotReference` with `wheel_mass_kg: f32`, `rotational_damping: f32`, `force_application_distance_m: f32`, and `hub: WheelHubReference`.
- Keep existing spring, damper, target, friction curves, wheel roles, and engine fields authoritative.
- `recover_driving(source, car_readers, wheel_readers) -> dict` returns global, per-car, and per-slot records with provenance.
- Reuse the accepted library reader. Do not read unchecked guessed IDs or add default values for missing records.

- [ ] **Step 1: Add critical failing contract checks.**

Add `test_driving_source_links_and_defaults` and `test_missing_driving_records_fail_without_promotion` to the Python target.
Check both car identities, four slots each, pedal rate `5.0`, steering rate `1.25`, and resolved source control bindings.
Check assist dependencies and each new field against checked source records.
Add `driving_reference_rejects_invalid_gear_curve_and_mass` to `tests/reference.rs`.
Reject nonfinite values, invalid startup gears, zero wheel mass, malformed curves, and unresolved hub links.

- [ ] **Step 2: Run the Python target and Rust `reference` target.**

Require failure at the missing contract or missing recovery behavior.
Do not count environment failure as a demonstrated regression.

- [ ] **Step 3: Derive required records and validate them.**

Recheck assembly hash `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
Read the methods and source owners listed in the specification.
Recover assist toggle defaults through `settingsscript.RefreshSettings`, including startup changes.
Recover selected hub properties, wheel damping, force application geometry, and lateral modifiers through active source pointers.
Keep source constants and method evidence beside derived runtime records.
Replace obsolete manifest fields when the new contract supersedes them.

- [ ] **Step 4: Regenerate the runtime export, replace the fixture records, and rerun the checks.**

Run runtime verification before accepting the export. Require no new source failures.
Run `./scripts/build.ps1` and confirm both cars load on the current road.
Record the export and package hashes in `.cache/driving-physics/layer-1/`.

- [ ] **Step 5: Commit the source contract and critical checks.**

Commit message: `feat: recover source driving controls and wheel records`.

## Task 2: Carry independent pedal values through the current runtime

**Files:** Modify `src/simulation/input.rs`, `src/simulation.rs`, `src/starting_area.rs`, `src/vehicle.rs`, `web/input.js`, and `web/main.js`.
Update `tests/input.rs`, `tests/vehicle.rs`, and `tests/web/input.test.mjs` for the required contract change.

**Interfaces:**
- Replace `KeyboardSteering` with `KeyboardDriving` in `src/simulation/input.rs`.
- `KeyboardDriving::new(steering_rate: f32, pedal_rate: f32) -> Self`.
- `KeyboardDriving::step(&mut self, seat: Option<SeatId>, held: u32) -> DrivingControls`; `clear(&mut self)` clears ramps.
- `DrivingControls` contains `steer`, `throttle`, `brake`, and `clutch` as `f32` values.
- Replace boolean `DriveInput` fields with `steer: f32`, `throttle: f32`, `brake: f32`, and `handbrake: f32`.
- Add `DriveTorques { motor_nm: f32, engine_brake_nm: f32 }` in `src/vehicle.rs`; motor torque has a sign.
- `PhysicsWorld::apply_vehicle_forces(&mut self, index: usize, input: DriveInput, torques: DriveTorques)` becomes the sole actuation entry.
- Allocate driving action bits 21 through 31 in `simulation::keys`; keep `FrameInput` fields as `u32`.
- Assign clutch bit 21, shift-up bit 22, shift-down bit 23, reverse bit 24, and neutral bit 25.
- Assign gears one through six to bits 26 through 31. Reuse these bits in source binding expansion.
- `InputState(bindings)` merges source driving bindings with existing actions during construction. Normalize masks with unsigned conversion.
- Pass required bindings from the loaded manifest. Remove the zero-argument constructor and update its callers.

- [ ] **Step 1: Add failing input and runtime checks.**

Use this test shape in `tests/input.rs`:

```rust
let mut input = KeyboardDriving::new(1.25, 5.0);
let seat = Some(SeatId(1));
let mut sample = DrivingControls::default();
for _ in 0..6 { sample = input.step(seat, keys::FORWARD | keys::BACK | keys::CLUTCH); }
assert!((sample.throttle - 0.5).abs() < 1e-5);
assert!((sample.brake - 0.5).abs() < 1e-5);
assert!((sample.clutch - 0.5).abs() < 1e-5);
```

Derive `Default` for `DrivingControls`. Check release, seat changes, and explicit clear in the same critical check.
Extend the render-rate test to 30, 60, and 120 FPS with equal fixed-step samples.
Add `driving_edges_survive_no_step_and_run_once_during_catch_up`.
Add a browser test for gear-six bit `2147483648`, release edges, and a shared on-foot/driving key.

- [ ] **Step 2: Run `input`, `vehicle`, and the browser input target; confirm the intended failures.**

Browser command: `node --test tests/web/input.test.mjs`.

- [ ] **Step 3: Connect ramps, unsigned keys, and the new torque interface.**

Keep steering timing unchanged. Pass smoothed throttle into `EngineInput`; keep service braking as a separate float.
Adapt the existing automatic flow to source input thresholds and preserve simultaneous pedal values.
Convert source torque to stock-controller force and brake impulse at one boundary during this layer.
Do not multiply engine output by throttle or clutch a second time.
Keep the stock wheel controller's current motor/brake limitation explicit until Task 5 replaces it.
Remove `drive_force`, `set_engine_brake`, and boolean-braking helpers after updating their callers.
Route brake lamps from the effective service-brake value.

- [ ] **Step 4: Rerun the checks and drive both cars with normal browser input.**

Confirm throttle rise and release, brake rise, reverse assist, pause clearing, and push-grip steering.
Record the remaining stock-controller limit in `.cache/driving-physics/layer-2/`.

- [ ] **Step 5: Commit the input layer.**

Commit message: `feat: apply source pedal ramps through driving simulation`.

## Task 3: Add manual transmission, assists, feedback, and persistent state

**Files:** Create `src/simulation/drivetrain.rs` and `tests/drivetrain.rs`.
Modify `src/simulation.rs`, `src/starting_area.rs`, `src/starting_area/save.rs`, `src/save/schema.rs`, `src/save/validate.rs`, and `src/vehicle.rs`.
Modify `src/browser.rs`, `src/app.rs`, `web/main.js`, `web/menu.js`, `web/feedback.js`, `web/index.html`, `web/style.css`, and `README.md`.
Update `tests/engine.rs`, `tests/save_schema.rs`, `tests/save_roundtrip.rs`, and the existing browser input/menu/feedback checks.

**Interfaces:**
- `DrivetrainState::new(vehicle: &VehicleReference) -> Self`; `gear(&self) -> usize` exposes the manifest index.
- `DrivetrainState::step(&mut self, controls: DrivingControls, edges: u32, assists: DrivingAssists, engine: EngineOutput, wheel_rpm: f32, direction: f32, vehicle: &VehicleReference) -> DrivetrainOutput`.
- `DrivetrainOutput` contains `gear: usize`, effective `throttle`, `brake`, `engagement`, `wheel_rpm`, `free_run`, and `ignition_request`.
- `transmitted_torques(engine: EngineOutput, vehicle: &VehicleReference, gear: usize) -> DriveTorques` applies ratios once.
- `VehicleRuntime.drivetrain` replaces `auto_gear`; `GarageRuntime.driving_assists` owns the player's two assist choices.
- `GarageRuntime::set_driving_assists(&mut self, requested: DrivingAssists) -> Result<(), ReferenceError>` accepts paused changes and normalizes dependencies.
- Replace saved `VehicleSnapshot.auto_gear` with `gear: usize`; add `SaveSnapshot.driving_assists: DrivingAssists`.
- Increase `SAVE_SCHEMA_VERSION` from `1` to `2`. Update current fixtures and schema consumers; reject version `1`.
- `gameDrivingAssists() -> u32` consumes a pending checkbox request while paused: changed bit 2, clutch bit 0, gears bit 1.
- `gameDrivingState(gear: i32, assists: u32, seated: bool)` publishes the accepted state; gear uses source display numbering.
- Rust publishes restored/default state before polling requests. Programmatic checkbox updates must not queue a new request.
- The browser renders Rust output. It does not select gears or simulate clutch behavior.

- [ ] **Step 1: Add critical failing drivetrain and save checks.**

Add `manual_shift_requires_source_clutch_threshold`: reject depression `0.5`, accept `0.5001`, and reject an unavailable gear.
Add `reverse_neutral_and_clutch_route_signed_torque_once`: check reverse sign, neutral zero, and one clutch attenuation.
Add `assists_keep_simultaneous_pedals_and_source_direction_rules` using recovered source thresholds and default flags.
Add `release_and_seat_exit_keep_selected_gear`; include zero-input updates of the unoccupied car.
Add `driving_save_restores_both_gears_and_assists_without_shift_edges` and `invalid_driving_save_keeps_active_scene`.
Extend browser checks for checkbox dependency, paused application, unsigned shift input, and `R`/`N`/forward display.

- [ ] **Step 2: Run the drivetrain, engine, save, and relevant browser targets; confirm intended failures.**

Use `./scripts/check.ps1 -Test drivetrain`, then `engine`, `save_schema`, and `save_roundtrip`.
Run `node --test tests/web/input.test.mjs tests/web/menu.test.mjs tests/web/feedback.test.mjs`.

- [ ] **Step 3: Connect the complete transmission flow and current save schema.**

Port active source shift, assist inversion, simultaneous-pedal clutch, starter, stall, and free-run rules in source order.
Keep pedal depression separate from engine engagement. Retain signed driven-wheel RPM.
Use the existing engine's clutch attenuation; change engine code only where source comparison proves a mismatch.
Remove `select_gear` and the unconditional automatic approximation after routing automatic behavior through `DrivetrainState`.
Wire source keyboard actions, contextual key conflicts, two assist controls, and the gear display.
Use the exact labels `Automatic clutch` and `Automatic gears`.
Clear ramps and pending edges on lifecycle boundaries without changing the selected gear.
Validate saved driving state before candidate creation or scene publication.
Update README controls using exported bindings and preserve existing menu behavior.

- [ ] **Step 4: Rerun checks, build the package, and complete the manual browser route for both cars.**

Check launch, shifts, neutral coast, reverse, stall/restart, assists, seat switch, pause, and save/full-reload continuation.
Confirm both cars retain their own gears and engine state after a seat switch.

- [ ] **Step 5: Commit the complete transmission layer.**

Commit message: `feat: add source manual gears clutch and driving assists`.

## Task 4: Establish the wheel equations and fixed source comparison bounds

**Files:** Create `docs/research/2026-10-14-driving-wheel-contract.md`.
Keep raw recordings, telemetry, analysis scripts, and full source extracts under ignored `.cache/driving-physics/source/`.
Update this plan's evidence status after collecting the required results.

**Interfaces:**
- The contract records each equation's variables, units, source citation, numerical bounds, and fixed-step order.
- It defines longitudinal and lateral slip, curve interpolation, load coupling, rotational inertia, damping, suspension force, and contact impulses.
- It defines empty-hub support and force application points from selected source records.
- It supplies fixed comparison bounds for both cars before wheel implementation or tuning.
- Task 5 consumes this completed contract. Do not dispatch or start Task 5 before this gate passes.

- [ ] **Step 1: Record matched source conditions and repeat each route three times per car.**

Record build, source hashes, car, parts, fluids, tire condition, gear, assists, starting pose, surface, and input timing.
Measure acceleration, coast-down, service braking, combined braking, turning, and suspension settling.
Repeat tire-to-rim and empty-hub checks under matched conditions.
Use available telemetry or timed video observations; record each metric's measurement resolution.
Preserve source files unchanged. Do not install or inject a source-game measurement plugin.

- [ ] **Step 2: Define and freeze the comparison bounds from those recordings.**

For each scalar metric, set the reference centre to the median of three runs.
Set tolerance to the maximum absolute deviation from that median plus the recorded measurement resolution.
Record the resulting numeric intervals with units for each car and configuration.
Use the same fixed sampling times for speed, yaw, and suspension observations.
Keep the intervals fixed during target tuning. Record any changed source conditions as a new dataset.

- [ ] **Step 3: Derive and document the wheel adapter equations.**

Use checked Unity 2021.3 properties and source gameplay rules; inspect upstream physics evidence when needed.
Label solver choices that the source does not expose as adapter choices.
State the low-speed treatment, combined-slip limits, brake zero-crossing rule, and spin update order.
Show dimensional checks and hand-calculated rest, reverse, locked-wheel, airborne, and dynamic-ground cases.
Require the proposed equations to reproduce the recorded source ranges before accepting the contract.

- [ ] **Step 4: Review the evidence gate and record its result.**

Require source receipts, numeric comparison intervals, explicit equations, and a link to each raw measurement.
Missing source measurements leave this task incomplete. Continue unrelated approved work; keep Task 5 unscheduled.
Report the missing measurement and required access or user action without claiming matching handling.
Request a specification correction if the evidence contradicts the approved architecture.

- [ ] **Step 5: Commit the checked wheel contract and evidence summary.**

Commit message: `docs: define measured source wheel contract`.
Keep raw source files, recordings, and temporary tools outside Git.

## Task 5: Replace the stock wheel layer and connect installed wheel state

**Prerequisite:** Task 4 has passed. Read its equations and fixed comparison intervals before editing wheel code.

**Files:** Create `src/vehicle/wheels.rs`.
Modify `src/vehicle.rs`, `src/interaction/wheels.rs`, `src/starting_area.rs`, `src/starting_area/save.rs`, `src/save/schema.rs`, and `src/save/validate.rs`.
Update `tests/vehicle.rs`, `tests/wheels.rs`, `tests/save_capture.rs`, and `tests/save_roundtrip.rs`.

**Interfaces:**
- `WheelConfiguration` contains source radius, mass, rotational damping, independent friction curves, and tire/rim/hub kind.
- `WheelSlotState` contains spin angle, angular speed, suspension length, and the completed contact sample.
- `VehicleWheels` stores `[WheelSlotState; 4]` and `[WheelConfiguration; 4]`; it has no per-frame heap storage.
- `VehicleWheels::new(vehicle: &VehicleReference, slots: [&WheelSlotReference; 4], chassis: RigidBodyHandle) -> Self`.
- `VehicleWheels::step(&mut self, physics: &mut rapier3d::prelude::PhysicsWorld, input: DriveInput, torques: DriveTorques)` applies the checked contract at `FIXED_DT`.
- `VehicleWheels::set_configuration(&mut self, slot: usize, configuration: WheelConfiguration)` preserves the other slots.
- `PhysicsWorld::apply_vehicle_forces` retains Task 2's signature and delegates to this owner.
- Keep `PhysicsWorld::snapshot`, `driven_wheel_rpm`, recovery, rebase, and wheel attachment save entry points as public runtime boundaries.
- Replace `InstalledWheel { radius, condition }` with the required `WheelConfiguration` path; an empty slot selects checked hub configuration.
- Save spin angle and angular speed for each stable slot. Replace obsolete previous-rotation and inferred-RPM state.
- Raise the save schema version for this later breaking change; keep one current reader.

- [ ] **Step 1: Add critical failing physical checks.**

Add `motor_and_all_eligible_brakes_act_on_the_same_step`, including separate driven and handbrake wheel roles.
Add `powered_and_airborne_wheels_keep_independent_spin` and `brakes_do_not_reverse_a_stopped_wheel`.
Replace `removed_slot_has_no_raycast_support_and_restores_without_duplicates` with source-checked tire/rim/hub behavior.
Add `wheel_exchange_preserves_other_slots_and_source_roles` for both cars.
Extend the spring test for off-centre target position, full travel, and changed mass.
Add `dynamic_ground_uses_relative_velocity_and_equal_opposite_impulses` and retain self-collision exclusion checks.
Extend save/recovery/rebase checks for selected gear, wheel spin, part-derived configuration, and cleared contact history.

- [ ] **Step 2: Run vehicle, wheel, and affected save targets; confirm the intended failures.**

Require failures in the identified stock-controller limits, not unrelated fixture setup.

- [ ] **Step 3: Implement Task 4's wheel contract through Rapier queries and body APIs.**

Query contact-relative velocity at the source force application point.
Compute suspension forces, motor torque, summed brake torque, independent spin, and two friction curves in the checked order.
Convert torque to force and force to impulse once. Exchange impulses with dynamic ground bodies.
Use full source travel and preserve physical spring and damper coefficients after mass changes.
Publish stable wheel poses and signed RPM through existing consumers, including engine, audio, and gauges.
Connect tire condition, bare-rim stiffness, empty hubs, radius, and installed mass through the existing attachment owner.
Restore attachment-derived configuration before spin state on load.
Reset contact history after recovery; transport contact positions during rebasing without changing spin or velocities.
Remove `DynamicRayCastVehicleController`, `WheelTuning`, controller reconstruction, and obsolete rotation-derived RPM bookkeeping.
Retain license and source provenance for reused upstream code.

- [ ] **Step 4: Rerun critical checks and complete both normal-input browser routes.**

Require finite state at rest, reverse, lost contact, combined braking, and part exchange.
Run the fixed source comparison routes against Task 4's frozen intervals.
Check garage exit and current-road seams, save/full reload, recovery, restart, and both origin directions.
Inspect steady updates for allocation and repeated setup; inspect mount updates for bounded retained state.
Do not accept wheel replacement while either car loses its garage-to-road route.

- [ ] **Step 5: Commit the wheel layer and its critical checks.**

Commit message: `feat: replace stock wheels with checked source wheel adapter`.

## Task 6: Verify the complete branch and record acceptance

**Files:** Update `docs/verification.md`, `docs/roadmap.md`, `docs/vehicles.md`, and the driving research record.
Keep captures, route logs, package hashes, and raw measurements under `.cache/driving-physics/acceptance/`.

**Interfaces:** The report separates gameplay rules, physical checks, source comparison, browser input, and unresolved limits.

- [ ] **Step 1: Run the final command gates once against the final implementation.**

Run `./scripts/check.ps1`, source/exporter checks, ignored source tests, runtime verification, and `./scripts/build.ps1`.
Require exit code zero. Repeat only affected checks after a new fix.

- [ ] **Step 2: Run the final normal-input browser route at 30, 60, and 120 FPS.**

Check both cars, manual and assisted driving, all pedal releases, shifts, neutral, reverse, stall, and combined brakes.
Check lights, instruments, audio inputs, part exchange, pause, capture loss, seat switch, save/load, recovery, restart, and rebasing.
Use real pointer capture where available. Record unavailable capture as a coverage limit.

- [ ] **Step 3: Complete source comparisons and a fresh whole-branch review.**

Compare all required metrics with Task 4's frozen intervals and retain measured deltas.
Use the execution method selected by the user for review dispatch.
Check the complete diff for duplicate clutch/ratio conversion, obsolete paths, per-frame allocation, and save publication order.
Fix actionable findings and repeat their affected checks.

- [ ] **Step 4: Update current documentation and roadmap status from the results.**

Record exact code/export/package identities, commands, browser conditions, source comparisons, and unresolved differences.
Keep driving marked partial if any required implementation or acceptance criterion remains unverified.
Document source-backed manual controls and the retained limitations of the wheel adapter.
Apply stop-slop and check Markdown links and whitespace.

- [ ] **Step 5: Commit the verification record and present the branch for review.**

Commit message: `docs: record driving physics verification and limits`.
Retain the isolated worktree and evidence. Do not merge, push, or remove other worktrees.

## Plan self-review

- Scope coverage: Tasks 1–3 cover source data, ramps, transmission, controls, lifecycle, and persistent state.
- Physics coverage: Tasks 4–5 cover measured equations, tires, rims, hubs, suspension, additive braking, and wheel spin.
- Acceptance coverage: Task 6 covers final builds, browser routes, source comparison, documentation, and branch review.
- Boundary checks: Review Focus names the owning task and its required check for each listed failure mode.
- Source gate: Task 4 must pass before Task 5 can be scheduled. No measured source handling result exists in this plan.
- Execution: The user must review this plan and choose Native or Subagent-driven execution before implementation starts.
