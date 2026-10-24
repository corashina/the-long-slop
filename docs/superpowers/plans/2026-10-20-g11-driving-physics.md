# G11 Lada driving physics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete source-measured Lada launch, wheel support, suspension, wheel spin, and combined braking within the garage scope.

**Architecture:** Retain Rapier bodies, queries, and integration. Replace the stock vehicle controller with one local four-slot wheel solver. Preserve engine, drivetrain, assembly, and save ownership.

**Tech Stack:** Rust 2024, pinned Rapier 0.36.0, existing serde, Python 3.12, PowerShell, shipped Unity 2021.3.18f1, and existing Docker checks.

**Spec:** [Approved G11 design](../specs/2026-10-20-g11-driving-physics-design.md), commit `6b6462f`.

## Global Constraints

- The source authority is Steam build `11140890`.
- Existing edits in the primary checkout remain outside this work.
- The work remains unmerged.
- Measurement precedes changes to tire, rim, hub, suspension, or brake physics.
- Use three fresh source runs per scenario with identical initial state.
- There is one active wheel solver. There is no compatibility mode.
- Four stable wheel slots exist for each current four-wheel vehicle record.
- The torque divisor is the count of installed driven rims, clamped to one.
- The existing fixed-step duration remains unchanged.
- The physics step performs no new heap allocation.
- Validation permits source-supported zero values.
- Rust changes require targeted native tests and both configured WASM compile checks.
- Release builds, full checks, and browser acceptance remain unapproved.
- G11 is accepted only after that browser milestone passes.
- Use GPT-6.1 Sol with high reasoning for implementation and review. Never use GPT-6 Astra.
- Apply the repository's brief English rules and pstack unslop to documentation.

## Review Focus

These risks require explicit tests in the owning tasks.

1. A failed or truncated capture must never replace accepted fixtures. Task 1 owns this test.
2. Removing driven rims must preserve the source torque and RPM divisor. Tasks 2 and 5 own this test.
3. A moving or deleted contact body must not leave stale impulses or query bounds. Tasks 4 and 5 own this test.
4. Load and origin changes must preserve wheel motion without using stale contacts. Task 5 owns this test.
5. Held brake torque must hold zero angular speed and must coexist with motor torque. Tasks 2 and 4 own this test.

## Execution boundaries

Use the existing worktree:

`C:/Users/Corashina/.codex/worktrees/g11-driving-physics/the-long-slop`

Use branch `codex/g11-driving-physics`.
Start from the committed specification. Do not merge newer primary-checkout work implicitly.
Before each task, inspect that branch's status and the preceding task's committed interface.
Use the app's existing worktree attachment. Do not create another worktree.

The user already selected subagents.
A source-capture worker owns Tasks 1 and 2.
A profile worker owns Task 3.
A wheel worker owns Task 4.
One integration owner performs Task 5 and owns shared vehicle interfaces.
A drivetrain worker owns Task 6.
The integration owner performs Task 7.
Fresh reviewers check each task and the complete branch.
Tasks are sequential where they share interfaces. Do not run source captures concurrently.
Do not let two workers edit the same file.

Tasks 1 and 2 are the evidence requirement.
Do not start Tasks 3 through 7 until that requirement passes.
Do not choose tire equations, inertia rules, curve wrapping, or suspension rules before native observations.
Task 2 records those choices in a measured model document before any solver implementation.
If measurements require another architecture or engine scheduling, return for a design revision.
A missing native runtime or an unstable trace stops work at the evidence requirement.

## File map

| Files | Responsibility |
| --- | --- |
| `tools/reference/driving_probe.py` | Capture preflight, orchestration, receipt validation, fixture publication, and trace comparison. |
| `tools/reference/driving_capture.cs` | Original-runtime setup, observation hooks, native probes, and buffered samples. |
| `tools/reference/driving_capture.ps1` | Scratch staging, Roslyn compilation, Cecil hooks, and bounded player execution. |
| `tools/reference/tests/test_driving_probe.py` | Critical capture and publication failure tests. |
| `tests/fixtures/driving/` | Versioned scenario inputs, numeric observations, timing maps, and envelopes. |
| `docs/g11-driving-measurements.md` | Qualifying evidence and the selected measured model. |
| `tools/reference/driving.py`, `tools/reference/manifest.py` | Complete source wheel-profile extraction. |
| `src/reference.rs`, `src/reference/driving.rs` | Typed profile records and boundary validation. |
| `src/vehicle/wheel.rs` | Fixed-size support, wheel state, and solver owner. |
| `src/vehicle/wheel/contact.rs` | Contact queries, relative velocity, and force points. |
| `src/vehicle/wheel/solve.rs` | Measured angular, suspension, and tire response. |
| `src/vehicle/wheel/tests.rs` | Native solver and profile fixtures. |
| `tests/support/driving_trace.rs` | Test-only candidate traces with run and revision provenance. |
| `src/vehicle.rs`, `src/interaction/wheels.rs` | Atomic solver replacement, assembly support, lifecycle, and existing snapshots. |
| `src/vehicle/query_bounds/tests.rs` | Query invalidation after wheel impulses. |
| `src/simulation/engine.rs`, `src/simulation/drivetrain.rs` | Measured curve boundaries and existing source drivetrain flow. |
| `src/simulation/driving_tests.rs` | Critical drivetrain and engine tests within the crate. |
| `tests/driving_physics.rs` | Exported-Lada route and wheel save continuity. |
| `docs/g11-driving-verification.md`, `docs/roadmap.md`, `docs/VEHICLES.md` | Exact checks, evidence, remaining limits, and code status. |

No change to `src/save/schema.rs` is planned.
Existing wheel angle, RPM, and previous-angle fields remain sufficient.
Existing exported-wheel tests receive only changes required by the new internal wheel records.

## Command setup

Run commands from the G11 worktree in PowerShell.
These variables identify existing read-only dependencies:

```powershell
$g11ReferenceProject = 'C:/Users/Corashina/Documents/Projects/the-long-slop'
$g11Python = "$g11ReferenceProject/tools/reference/.venv/Scripts/python.exe"
$g11Source = "$g11ReferenceProject/.cache/reference-library/source"
$g11Raw = '.cache/driving-source/capture-001'
```

Use a fresh capture directory for each attempt. Never delete failed evidence to reuse a name.
The old Steam installation path is absent at planning time.
The archived source tree contains the player and its managed and native dependencies.
Validate the complete archive receipt before using that tree.

The old `.cache/world-source/` helpers contain hard-coded primary-checkout and Steam paths.
They also change random calls, world scheduling, and startup exit behavior.
Read them for setup patterns. Do not execute or copy their patch set wholesale.

Only documentation checks run while writing this plan.
Commands below are execution instructions. They have not run for G11.

### Task 1: Build a repeatable native capture tool

**Files:** Create the three capture files and `tools/reference/tests/test_driving_probe.py`.
Create `tests/fixtures/driving/scenarios.json`.
Read `tools/reference/library/archive.py`, `tools/reference/export.py`, and the ignored world-capture helpers.

**Interfaces:**

```python
def preflight(source_root: Path, reference_root: Path) -> dict: ...
def capture(source_root: Path, reference_root: Path,
            output_root: Path, scenarios: Path, repeats: int) -> dict: ...
def verify(output_root: Path) -> dict: ...
```

The CLI exposes `preflight`, `capture`, and `verify` with those arguments.
A completed capture writes `receipt.json` only after all required native observations pass.
`capture` requires `repeats == 3` and a new output directory inside the G11 worktree.
`reference_root` supplies the accepted source and library receipts. It is read-only.
Use `load_snapshot`, `verify_snapshot`, `native_engine_evidence`, and the existing managed-source hash constants.

- [x] Write tests named `test_preflight_rejects_changed_source`, `test_incomplete_capture_never_accepts`, and `test_capture_requires_native_phases`.
  Assert that a changed archive hash fails.

  Assert that missing repeats, missing samples, nonfinite values, and missing completion markers fail.
  Assert that failure cannot create a completed-capture receipt.
  Assert that a formula-only trace cannot satisfy either native-probe or assembled-car evidence.

```python
with self.assertRaises(ValueError):
    preflight(changed_source, reference_root)
self.assertFalse((failed_output / 'receipt.json').exists())
```

- [x] Run `& $g11Python -B -m unittest tools.reference.tests.test_driving_probe`.
  Expect failure because `driving_probe` or its required behavior is absent.
- [x] Implement `preflight` and scratch setup in the three capture files.
  Verify the complete source manifest and retained Steam receipt, not only two DLL hashes.
  Check `TheLongDrive.exe`, `MonoBleedingEdge`, Roslyn, Cecil, and the shipped physics module.
  Compile original observer code against the shipped assemblies.
  Parameterize all source, reference, scratch, and profile paths.
  Use isolated in-memory PlayerPrefs and a scratch save path.
  Record original hashes, patched hashes, changed methods, and observer revision.
  Keep physics, engine, gear, random, and world-generation formulas unchanged.
  Suppress Steam initialization only in the scratch copy and record that patch.
- [x] Implement native capture at the original car, engine, and physics callback boundaries.
  Use observation hooks with explicit entry and exit phases.
  Inject controls at the checked source input boundary without replacing its smoothing logic.
  Require the original menu and selected Lada initialization to finish.
  Verify source control capture is active and physics steps advance.
  Capture isolated Unity curve and WheelCollider probes separately from assembled-Lada traces.
  Buffer samples, then flush them after the finite scenario.
  Launch the scratch player hidden. Use a 180-second per-run timeout.
  On timeout, terminate only the exact process handle created for that run.
  Preserve partial samples and logs under the failed run directory.
- [x] Define the scenario JSON fields.
  Include scenario identity, probe kind, source object identities, setup, frame schedule, control schedule, and duration.
  Include initial body, wheel, engine, fluid, assist, and surface state.
  Include all scenario rows in the approved specification.
  Use source-recovered values. Do not fill missing measurements with estimates.
- [x] Run the Python tests again. Expect all named tests to pass.
  Run `& $g11Python -B -m tools.reference.driving_probe preflight --source-root $g11Source --reference-root $g11ReferenceProject`.
  Expect a verified receipt with app `1017180`, build `11140890`, Unity `2021.3.18f1`, and checked source hashes.
  Preflight does not launch the player.
- [x] Commit the capture tool, its tests, and scenario definitions as `feat: add checked G11 native driving capture`.
  Do not stage copied assemblies or raw runs.

### Task 2: Capture source behavior and freeze the measured model

**Files:** Modify `driving_probe.py` and its tests.
Create numeric files under `tests/fixtures/driving/`.
Create `docs/g11-driving-measurements.md`.

**Interfaces:**

```python
def publish_fixtures(output_root: Path, destination: Path) -> dict: ...
def compare(fixture_root: Path, candidate: Path, group: str, expected: dict) -> dict: ...
```

Add CLI commands `publish` and `compare`.
Capture receipts use `schema_version`, `source`, `observer`, `patches`, `scenarios`, and `runs`.
Each run names its scenario, repeat index, setup hash, raw files, callback map, and completion marker.
Each scenario also declares its immutable comparison group: `wheel`, `drivetrain`, or `assembled_lada`.
Samples carry render-frame index, physics-step index, callback, phase, control-input revision, and output revision.
They carry body state, engine state, wheel state, and metric availability in explicit SI units.
Numeric fixtures retain their source sample hashes and immutable comparison envelopes.
`verify` returns the same receipt whether called directly or before publication.
`compare` consumes a candidate trace with the source trace's scenario and metric names.
Its `group` selects a fixture-defined required scenario set, rather than a candidate-selected subset.
Reject missing scenarios or metrics within that group.
Its `expected` dictionary contains `run_id`, `tested_revision`, and `asset_manifest_sha256`.
Require exact matches plus the current fixture receipt hash and a completed-trace marker.
Expose CLI flags `--group`, `--expected-run-id`, `--expected-revision`, and `--expected-asset-manifest`.
It returns `passed`, plus failures with `scenario`, `metric`, `source_envelope`, and `candidate_value`.
Its CLI exits nonzero for a missing metric, bad provenance, or failed comparison.

- [ ] Write `test_envelopes_come_only_from_source_repeats`, `test_comparison_rejects_missing_and_outside_samples`, and `test_failed_publication_keeps_prior_fixtures`.
  Assert that all three source repeats contribute to each continuous envelope.
  Assert that event windows use observed native event times.
  Assert that candidate values cannot change those envelopes.
  Assert that a missing scenario, stale run ID, wrong revision, or wrong asset manifest fails.
  Assert that unavailable native metrics remain explicitly unavailable.
  Assert that failed publication preserves the prior fixture set byte-for-byte.

```python
self.assertFalse(compare(fixtures, incomplete_candidate, 'wheel', expected)['passed'])
self.assertEqual(before_bytes, (fixtures / 'receipt.json').read_bytes())
```

- [ ] Run the capture tests. Expect the two new tests to fail.
- [ ] Implement publication and comparison.
  Use source minimum and maximum values plus recorded capture precision.
  Bind each fixture to its raw sample hashes, scenario hash, observer revision, and sampling phase.
  Validate all required repeats and phases before publishing a complete fixture set.
  Keep a failed publication from replacing the previous accepted set.
- [ ] Run the capture command.
  `& $g11Python -B -m tools.reference.driving_probe capture --source-root $g11Source --reference-root $g11ReferenceProject --output-root $g11Raw --scenarios tests/fixtures/driving/scenarios.json --repeats 3`
  Expect three complete native runs for every required scenario.
  Then run `& $g11Python -B -m tools.reference.driving_probe verify --output-root $g11Raw`.
  Expect separate qualifying results for native component probes and the assembled Lada.
- [ ] Review the raw observations before publication.
  Require longitudinal, lateral, and combined-slip runs for tires, rims, and hubs.
  Require one-rim and two-rim removal, airborne spin, suspension settling, and dynamic support.
  Require brake hold at zero, throttle with each brake, all brakes together, and clutch release.
  Require curve boundaries at zero, both endpoints, and outside both endpoints.
  Include source load and support-transition observations.
- [ ] Write the measured model document from those observations.
  Specify curve evaluation, combined slip, angular inertia, damping, suspension travel, and force points.
  Specify affected-slot angular state after hub, rim, tire, or condition changes.
  Define the callback-to-physics timing map and preserve the AutoShift-to-Stuff RPM delay.
  Define initial states, sample times, comparison metrics, and exact source-derived envelopes.
  Identify every model parameter's source record or native observation.
  Do not leave equation choice to the solver worker.
  If these rules cannot be established, stop before Task 3.
  If source timing requires an engine scheduling rewrite, request a design revision.
- [ ] Run `publish --output-root $g11Raw --destination tests/fixtures/driving` through the module CLI.
  Run the capture tests again. Expect all named tests to pass.
  Replay each retained source repeat through the same group comparator with its recorded provenance. Expect all required scenarios to pass.
- [ ] Commit the model document, fixtures, and comparison code as `test: retain native G11 driving evidence`.
  Record the qualifying receipt hash in the model document.
  This commit is the prerequisite for Task 3.

### Task 3: Resolve complete hub, rim, and tire profiles

**Files:** Modify `tools/reference/driving.py`, `tools/reference/manifest.py`, and `tools/reference/tests/test_driving.py`.
Modify `src/reference.rs` and `src/reference/driving.rs`.

Create `src/vehicle/wheel.rs` and `src/vehicle/wheel/tests.rs`. Declare `mod wheel;` in `src/vehicle.rs`.

**Interfaces:**

Add `rim_forward_stiffness: f32` and `rim_sideways_stiffness: f32` to `WheelSlotReference`.
Recover their values from checked managed behavior. Do not hard-code rounded audit numbers.

Define these copyable internal types in `wheel.rs`:

```rust
pub enum WheelSupport { Hub(WheelProfile), Rim(WheelProfile), Tire(WheelProfile) }
pub struct WheelProfile {
    pub radius_m: f32,
    pub angular_inertia_kg_m2: f32,
    pub rotational_damping: f32,
    pub forward: WheelFrictionCurve,
    pub sideways: WheelFrictionCurve,
}
pub enum WheelAssembly {
    Hub,
    Rim { radius_m: f32, total_mass_kg: f32 },
    Tire { radius_m: f32, total_mass_kg: f32, quality: f32 },
}
pub fn resolve_support(
    slot: &WheelSlotReference, assembly: WheelAssembly, wheel_slip: [f32; 4],
) -> WheelSupport;
```

`WheelAssembly` describes the current rim and tire assembly without invalid combinations.
Its total mass follows the source `mass.Mass()` owner, including the mounted tire when present.
The hub uses the existing source hub mass.
The four `wheel_slip` values are forward minimum, forward maximum, sideways minimum, and sideways maximum.
Use the measured inertia and stiffness rules from Task 2.
Keep suspension and force-height parameters in the existing slot record.
Use `pub(crate)` for `WheelSupport`, `WheelProfile`, `WheelAssembly`, and `resolve_support`.
Re-export the types and resolver through `vehicle` for assembly code.
Keep the solver owner visible only to its parent module.
Add `PartialEq` to value records needed by the assembly's support cache.

- [ ] Add profile tests using the Task 2 fixture values.
  Assert exact source hub radius and stiffness.
  Assert separate rim stiffness and tire-condition curves.
  Assert that source-supported zero damping and zero stiffness remain valid.
  Add Python failure tests for missing rim parameters and invalid required values.

```rust
let WheelSupport::Hub(profile) = resolve_support(&slot, WheelAssembly::Hub, slip) else {
    panic!("expected hub profile");
};
assert_eq!(profile.radius_m, slot.hub.radius_m);
assert_eq!(profile.forward.stiffness, slot.hub.forward_stiffness);
```

- [ ] Run `& $g11Python -B -m unittest tools.reference.tests.test_driving`.
  Run `./scripts/check.ps1 -Filter vehicle::wheel`.
  Expect failures for the missing profile fields or resolution behavior.
- [ ] Implement export, typed validation, and `resolve_support`.
  Validate finite values, positive radius and angular inertia, and ordered curves at the boundary.
  Recover new parameters before promoting the selected runtime export.
  Reject obsolete manifests without the required fields. Do not add defaults or migration code.
  Preserve existing chassis mass ownership.
  The existing controller remains active until Task 5 replaces it atomically.
- [ ] Derive the selected Lada export into this worktree.
  Run `& $g11Python -B -m tools.reference.export --library-root "$g11ReferenceProject/web/assets/library" --output web/assets/reference`.
  Stop if the accepted source receipts or native manifest validation fail.
  Do not modify the primary export.
- [ ] Repeat the Python tests and targeted Rust check. Expect passing tests and both WASM compile checks.
  Update any direct `WheelSlotReference` constructors for the new required fields.
- [ ] Commit as `feat: recover complete Lada wheel support profiles`.

### Task 4: Implement the measured wheel solver without changing the active vehicle path

**Files:** Modify `src/vehicle/wheel.rs` and `src/vehicle/wheel/tests.rs`.
Create `src/vehicle/wheel/contact.rs`, `src/vehicle/wheel/solve.rs`, and `tests/support/driving_trace.rs`.

**Interfaces:**

`WheelSolver` owns `chassis: RigidBodyHandle`, four slot records, four support profiles, and four wheel states.
`WheelState` owns `angle_rad`, `previous_angle_rad`, `omega_rad_s`, `suspension_length_m`, and the measured suspension history.
It stores `contact: Option<WheelContact>`.
`WheelContact` names its collider, optional rigid body, world point, and world normal.
Any additional mathematical state must be defined by the Task 2 measured model.

```rust
pub struct WheelTorques {
    pub motor_nm: f32,
    pub engine_brake_nm: f32,
    pub service_brake_nm: f32,
    pub handbrake_nm: f32,
}
impl WheelSolver {
    pub fn new(chassis: RigidBodyHandle, slots: [WheelSlotReference; 4],
               supports: [WheelSupport; 4]) -> Self;
    pub fn set_support(&mut self, index: usize, support: WheelSupport);
    pub fn step(&mut self, torques: [WheelTorques; 4], steering: [f32; 4],
                queries: QueryPipelineMut<'_>, dt: f32)
                -> [Option<RigidBodyHandle>; 5];
}
```

The returned handles identify the chassis and up to four contact bodies changed by solver impulses.
Use `states: [WheelState; 4]` and `supports: [WheelSupport; 4]` as the solver's field names.
The integration owner marks the returned bodies' query bounds.
The two child modules are private implementation details. Other tasks call only the owner.
No feature flag selects this solver.

Define this test-only helper in `tests/support/driving_trace.rs`:

```rust
pub fn write_trace(group: &str, samples: &serde_json::Value)
    -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>>;
```

Without `G11_TRACE_ROOT`, tests check their assertions without writing candidate artifacts.
With that variable, require `G11_TRACE_RUN`, `G11_TESTED_REVISION`, and `G11_ASSET_MANIFEST`.
Write one completed `<group>.json` with those identities, the fixture receipt hash, and all required group scenarios.
Refuse to overwrite an existing trace. Do not publish a completed marker after a failed assertion.
Include this helper by path from library test modules and the exported integration test.
It has no dependency on crate-private production types.

- [ ] Add table test `g11_native_wheel_scenarios_match` for airborne torque, longitudinal slip, lateral slip, combined slip, and suspension settling.
  Replay every required `wheel` scenario and emit its trace through `write_trace` when tracing is requested.
  Add stopped-wheel brake hold and simultaneous motor/brake cases.
  Add moving-support relative velocity and equal reaction-impulse assertions.
  Use fixture envelopes from Task 2. Do not introduce generic handling tolerances.
  The test setups load the source initial states and step the solver with each captured input.

```rust
assert_ne!(airborne_solver.states[driven].omega_rad_s, 0.0);
assert_eq!(held_solver.states[braked].omega_rad_s, 0.0);
assert!(source_comparison_failures.is_empty());
```

- [ ] Run `./scripts/check.ps1 -Filter vehicle::wheel`.
  Expect failures for the missing solver behavior.
- [ ] Implement the measured model from Task 2 through `WheelSolver::step`.
  Use the existing BVH query pipeline and full current wheel filter predicate.
  Exclude sensors, the owning chassis, and source-ineligible vehicle colliders.
  Use relative contact-point velocity for dynamic support.
  Integrate angular speed from torque and tire reaction.
  Evaluate longitudinal and lateral response separately, then apply the measured combined-slip rule.
  Solve full source suspension travel, including airborne extension and noncentral target positions.
  Apply impulses at the measured force points.
  Keep signed motor torque and all brake inputs in the same solve.
  Brakes can hold zero speed. Brakes alone cannot reverse wheel rotation.
- [ ] Implement `set_support` with the source transition rule.
  Preserve all other slots' state.
  Use fixed arrays and stack contact storage. Resolve curves outside the per-step path.
  Do not clone slot records or allocate during `step`.
- [ ] Repeat the targeted Rust check. Expect fixture tests and both WASM checks to pass.
  Review every runtime parameter against the measured-model document.
- [ ] Commit as `feat: solve measured Lada wheel forces and spin`.

### Task 5: Replace the stock controller and migrate lifecycle behavior

**Files:** Modify `src/vehicle.rs`, `src/interaction/wheels.rs`, and `src/vehicle/query_bounds/tests.rs`.
Modify existing wheel fixture constructors and required world relocation assertions.

**Interfaces:**

Retain `apply_vehicle_forces(index, DriveInput, DriveTorques)`, `snapshot`, and the existing save method signatures.
Replace `set_wheel` with:

```rust
pub(crate) fn set_wheel_support(

    &mut self, slot: WheelSlotId, support: WheelSupport
) -> Result<(), ReferenceError>;
pub(crate) fn installed_driven_rim_count(&self, vehicle: usize) -> usize;
```

`driven_wheel_rpm(vehicle)` sums every driven slot's signed RPM.
Its divisor is `installed_driven_rim_count(vehicle).max(1)`.
Torque uses the same divisor.
Remove unused `wheel_support_count` and old `driven_wheel_count`.
Assembly code caches `Option<WheelSupport>` for its last resolved support.
`WheelAttachment` retains its current serialized shape.

- [ ] Add native integration-unit tests for four persistent slots with zero, one, and two driven rims.
  Assert source torque distribution and RPM feedback for all three states.
  Assert that tire removal does not change the installed-rim divisor.
  Add save-state restoration at nonzero RPM without a false first-step spike.
  Add contact-body deletion, origin shift, normal reset, and wheel-preserving reset assertions.
  Assert that profile changes leave other wheel states, mass, and collider ownership unchanged.

```rust
assert_eq!(physics.installed_driven_rim_count(0), 0);
assert_eq!(physics.driven_wheel_rpm(0), left_hub_rpm + right_hub_rpm);
assert_eq!(saved.rotation, restored.rotation);
```

- [ ] Run `./scripts/check.ps1 -Filter vehicle`. Expect failures for the missing permanent-slot behavior.
- [ ] Replace the controller in one integration change.
  Initialize all four profiles from assembly state.
  Resolve profiles in `WheelAssemblies::sync_vehicle` only after relevant assembly or condition changes.
  Pass motor, engine-brake, service-brake, and handbrake torque to eligible slots.
  Preserve the source divisor even when driven slots contain hubs.
  Mark query bounds for every changed body returned by the solver.
  Preserve sleep and wake behavior.
  Keep the existing `VehicleSnapshot` shape and use solved angle and suspension travel for its poses.
- [ ] Migrate save, recovery, reset, relocation, rebase, and collider removal.
  Restore angular speed from saved RPM. Rebuild contacts before consuming them.
  Translate cached world contact points during rebase.
  Clear deleted contact handles and wake affected chassis.
  Retain the distinct `reset_preserving_wheels` contract.
  Keep ordinary recovery's dynamics reset.
- [ ] Remove `DynamicRayCastVehicleController`, `InstalledWheel`, active-slot maps, resting-controller copies, and controller prewarming.
  Replace the direct old-controller fixture in `private_controller_impulses_keep_post_step_queries_current`.
  Keep its post-impulse query correctness assertions.
  Update every current caller in this commit. Do not add compatibility wrappers.
- [ ] Repeat `./scripts/check.ps1 -Filter vehicle`. Expect passing targeted tests and both WASM checks.
  Search production vehicle code for removed controller symbols. Expect no obsolete vehicle-controller path.
- [ ] Commit as `feat: integrate permanent Lada wheel physics`.

### Task 6: Verify launch, clutch, gears, and assists against native evidence

**Files:** Modify `src/simulation/engine.rs` and `src/simulation/drivetrain.rs` only where measurements require changes.
Create `src/simulation/driving_tests.rs`. Reuse the test-only trace helper from Task 4.
Declare its test-only module from `src/simulation.rs`.
Use Task 2's compact source-derived vehicle, engine, and driving records.

**Interfaces:**

Retain `DrivetrainState::step`, `EngineState::step`, and `transmitted_torques`.
Retain `torque(curve: &TorqueCurve, rpm: f32) -> f32`.
Task 2 defines the exact measured endpoint and extrapolation behavior.
Engine output already includes clutch engagement.
Axle torque applies the selected gear and final-drive ratio once.

- [ ] Add table test `g11_native_drivetrain_scenarios_match`.
  Its cases cover native curve boundaries, default launch, manual clutch and gear order, reverse, and assists.
  Replay every required `drivetrain` scenario and emit its trace through `write_trace` when tracing is requested.
  Assert fixture-derived curve values at zero, endpoints, and outside endpoints.
  Replay source launch inputs with the captured initial engine state.
  Cover direct gear keys, sequential shifts, the halfway clutch threshold, and assist dependency.
  Verify reverse input inversion and clutch disengagement under braking.
  Assert single clutch attenuation and single gear/final-drive multiplication.
  Load `expected` values and capture precision from the Task 2 drivetrain fixture.

```rust
assert_eq!(actual.gear, expected.gear);
assert_eq!(actual.ignition_request, expected.ignition_request);
assert!((actual.wheel_rpm - expected.wheel_rpm).abs() <= capture_precision_rpm);
```

- [ ] Run `./scripts/check.ps1 -Filter simulation::driving_tests`.
  Record which assertions fail. A source-confirmed existing behavior needs no code change.
- [ ] Apply only the measured corrections.
  Preserve the existing update ownership and AutoShift RPM delay.
  Do not infer Unity WrapMode behavior from the serialized infinity value.
  Do not add positive minimum torque or an uncalled parking assist.
  If normal source launch requires a timing rewrite, stop and revise the approved design.
- [ ] Repeat the targeted Rust check. Expect all named tests and both WASM checks to pass.
- [ ] Commit tests and any required correction as `fix: match measured Lada launch and drivetrain rules`.

### Task 7: Verify the exported Lada and record G11 status

**Files:** Create `tests/driving_physics.rs` and `docs/g11-driving-verification.md`.
Modify `docs/roadmap.md` and the affected limits in `docs/VEHICLES.md`.
Use the worktree's selected Lada export from Task 3.

**Interfaces:**

Add ignored test `installed_lada_g11_launch_brake_profiles_and_reload`.
Its ignore reason is `requires build 11140890 export`.
Ordinary fixture tests remain unignored in the owning crate modules.
The route uses existing `GarageRuntime`, player seating, assist choices, frame inputs, capture, and restore.
The host compares its `assembled_lada` trace after the Rust test.
Wheel and drivetrain tests produce the two component traces. All three group comparisons are required.
The Docker tests do not require Python.
Use `write_trace` from Task 4. Write only beneath the fresh `G11_TRACE_ROOT` for that verification run.

- [ ] Add the exported-Lada route.
  Iterate every required `assembled_lada` scenario with its captured initial state and emit that complete group.
  Exercise launch, steering, forward and reverse gears, clutch, coasting, and combined braking.
  Exercise tire, rim, and hub states through existing assembly operations.
  Save at nonzero wheel RPM, reload, and continue.
  Verify wheel identity, condition, mass, collision ownership, and state continuity.
  Check the source initial states and timing map from Task 2.
  The host comparison below checks the full source envelopes after this test writes its trace.

```rust
assert_eq!(reference.vehicles.len(), 1);
assert_eq!(reference.vehicles[0].source.path_id, 77839);
assert!(run.physics.snapshot(0).speed_mps.abs() > 0.0);
```

- [ ] Execute this route explicitly. Do not count an ignored test as passed.
  The current `scripts/check.ps1` cannot pass `--ignored`.
  Stage the new test before recording its revision, so the diff hash includes its source.
  After the targeted Rust check has prepared the toolchain image, use:

```powershell
. ./scripts/cargo-target.ps1
$g11Workspace = (Get-Location).Path
$g11CargoTarget = Get-CargoTargetDirectory $g11Workspace
$g11RunId = [guid]::NewGuid().ToString('N')
$g11TraceRoot = ".cache/driving-source/verify-$g11RunId"
$g11Commit = (& git rev-parse HEAD).Trim()
$g11Diff = (& git diff HEAD --binary | Out-String)
$g11DiffBytes = [Text.Encoding]::UTF8.GetBytes($g11Diff)
$g11DiffHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($g11DiffBytes)).ToLowerInvariant()
$g11Revision = "$g11Commit+$g11DiffHash"
$g11AssetManifest = (Get-FileHash web/assets/reference/manifest.json -Algorithm SHA256).Hash.ToLowerInvariant()
$g11DockerArgs = @(
    'run', '--rm',
    '--mount', "type=bind,source=$g11Workspace,target=/workspace",
    '--mount', 'type=volume,source=tld-wasm-cargo,target=/cargo',
    '--mount', 'type=volume,source=tld-wasm-target,target=/target',
    '--env', "CARGO_TARGET_DIR=$g11CargoTarget",
    '--env', "G11_TRACE_ROOT=/workspace/$g11TraceRoot",
    '--env', "G11_TRACE_RUN=$g11RunId",
    '--env', "G11_TESTED_REVISION=$g11Revision",
    '--env', "G11_ASSET_MANIFEST=$g11AssetManifest",

    '--workdir', '/workspace', 'the-long-slop-toolchain'
)
& docker @g11DockerArgs cargo test --locked --lib g11_native
if ($LASTEXITCODE -ne 0) { throw 'G11 component replay failed' }
& docker @g11DockerArgs cargo test --locked --test driving_physics --test wheel_assembly --test wheel_input -- --ignored
if ($LASTEXITCODE -ne 0) { throw 'G11 exported replay failed' }
foreach ($g11Group in @('wheel', 'drivetrain', 'assembled_lada')) {
    & $g11Python -B -m tools.reference.driving_probe compare --fixture-root tests/fixtures/driving --candidate "$g11TraceRoot/$g11Group.json" --group $g11Group --expected-run-id $g11RunId --expected-revision $g11Revision --expected-asset-manifest $g11AssetManifest
    if ($LASTEXITCODE -ne 0) { throw "G11 comparison failed: $g11Group" }
}
```

  Expect both named component tests and all selected ignored integration tests to execute and pass.
  Expect complete, fresh traces for all three groups and no failed comparison.
  Record the tested commit and tracked-diff hash. The hash includes the staged integration test.
  If a run fails, keep its artifacts and allocate another run ID for the retry.
  Never compare after a failed Docker command.
  If a trace is outside its source envelope, fix the solver rather than enlarge the envelope.
  Report independent failures with exact commands. Do not weaken existing assertions.
- [ ] Run final focused Python capture/export tests.
  Run `./scripts/check.ps1 -Filter vehicle` and `./scripts/check.ps1 -Filter simulation::driving_tests`.
  Expect the focused native tests and both configured WASM checks to pass.
  Reuse passing results unless another relevant change requires a rerun.
  Do not run `-Full`, release builds, or browser acceptance.
- [ ] Review wheel-step allocations after warmup.
  Inspect solver arrays, curve setup, contact collection, and query mutation paths.
  Confirm no new heap allocation in the new wheel step.
  If inspection is inconclusive, add one focused allocation probe around the isolated solver step.
  Report the probe's scope. Do not claim whole-frame allocation freedom.
- [ ] Write the verification report.
  Record source receipt, model revision, tested commit, tracked-diff hash, asset-manifest hash, run ID, commands, executed test names, and failures.
  Record release build, full checks, and browser acceptance as deferred.
  Change G11 from Partial to Verify only if every code and source-comparison requirement passes.
  Retain an explicit normal-input browser acceptance requirement.
  Add self-run commands only when a tested package exists.
- [ ] Run `git diff --check` and inspect the complete branch diff against the design base.
  Obtain a fresh whole-branch review.
  Fix confirmed issues and rerun only affected checks.
- [ ] Commit the integration test and documentation as `test: verify measured G11 Lada driving`.
  Keep the branch unmerged and report the remaining browser milestone.

## Completion handoff

Report completed tasks and the qualifying source receipt.
Report exact checks, failures, and deferred checks.
Link the measured-model document and G11 verification report.
Do not describe native fixture comparison as browser acceptance.
Do not mark G11 accepted without the separately agreed normal-input browser milestone.

This plan requires user review before implementation.
The execution method remains subagent-driven, as the user requested.


## Task 2 measurement coverage notes

The current 39 cases do not complete the approved measurement requirement.
The evidence audit is .superpowers/sdd/2026-10-20-g11-driving-physics/task-2-model-evidence-audit.md.
Complete these bounded measurement layers before model publication:

- [ ] Qualify explicit source events, actual free launch, separate braking, and native teardown.
- [ ] Observe live support and condition transitions through original APIs.
- [ ] Observe fresh source save/load through original scene startup, without new-game holds or canonical setters.
- [ ] Identify airborne angular response through actual torque changes and torque-off coast, for each support.
- [ ] Identify pure and combined contact response with separated force and moment observations.
- [ ] Observe actual suspension endpoints and distinguish static load from dynamic damping.
- [ ] Observe relative motion and reaction on a dynamic support body.
- [ ] Freeze an explicit bounded model, its inference procedure, residuals, and confirmation envelopes.

These notes refine Task2 measurements. They do not authorize guessed equations or a product scheduling rewrite.
Task3 still requires the qualifying fixture and measured-model commit.
