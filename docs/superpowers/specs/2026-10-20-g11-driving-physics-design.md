# G11 Lada driving physics design

Date: 2026-10-20.
Scope: roadmap G11 for the garage Lada and nearby terrain.
Base: `cc7540b255d683722c67e09103a66bfa789c0fe7`.
Status: written specification approved by the user on 2026-10-20.

## Intended result

The player can launch, steer, shift, coast, and brake the Lada through normal input.
Tires, bare rims, and empty hubs have their source behavior.
Suspension travel and wheel rotation respond to load, contact, and torque.
Service braking, engine braking, and the handbrake work together.

The source authority is Steam build `11140890`.
The authority includes checked records, decompiled control flow, and native driving measurements.
Compilation and scripted movement do not establish handling parity.

The user approved a measurement-first design with an isolated worktree and subagents.
Existing edits in the primary checkout remain outside this work.
The work remains unmerged.

## Current gaps

| Area | Current behavior | Required change |
| --- | --- | --- |
| Empty hub | Removing the rim removes the controller wheel. | Keep a hub contact model in that slot. |
| Bare rim | Radius changes, but grip uses full tire quality. | Use the source rim parameters. |
| Tire grip | One Rapier friction value represents both axes. | Use measured longitudinal and lateral response. |
| Suspension | The adapter converts source values into Rapier tuning. | Match measured travel, settling, and load response. |
| Wheel rotation | RPM comes from controller rotation derived from body motion. | Integrate independent wheel angular speed. |
| Combined braking | The stock controller ignores its brake impulse during motor force. | Solve motor and brake torque together. |
| Automatic launch | Clamped curve evaluation can retain neutral at zero wheel RPM. | Resolve Unity curve boundaries from native measurements. |

The current exporter already stores hub parameters, wheel mass, damping, force height, and both friction curves.
It does not provide a complete runtime rim profile.
The retained reports contain no qualifying source driving measurements.
The existing formula probe explicitly excludes native driving traces.

Current drivetrain control flow remains the starting point.
A source method without a live caller does not justify a new assist.
A positive torque floor is unsupported.

## Measurement requirement

Measurement precedes changes to tire, rim, hub, suspension, or brake physics.
The first executable delivery is a repeatable source capture and comparison tool.
The tool uses the installed build's native Unity runtime and checked managed code.
A Rust formula probe or recreated Unity formula does not qualify.

Capture runs in a separate ignored scratch directory within the G11 worktree.
The installed game, normal saves, and other worktrees remain unchanged.
Existing source-capture code may supply process setup and provenance checks.
It does not supply driving observations.

Each run records these inputs:

- Steam build, source assembly hash, native engine hash, and capture-tool revision.
- Lada identity, component identities, mass, center of mass, and wheel parameters.
- Tire condition, fluids, engine state, assist choices, selected gear, and handbrake state.
- Surface geometry, initial pose, body velocity, wheel state, and fixed timestep.
- The complete frame-indexed control sequence and any test setup changes.
- Render-frame times, physics-step times, callback order, and the sampling phase.

Each fixed step records body pose, body velocity, engine RPM, gear, and clutch engagement.
Each wheel records contact, suspension travel, angular speed, motor torque, and brake torque.
Native contact and slip values are recorded when the runtime exposes them.
Any unavailable value is identified. An unavailable value is not inferred and labeled native.

The capture has two levels.
Isolated native probes measure curve boundaries and WheelCollider response.
Assembled Lada runs confirm launch, suspension, and combined braking in the original runtime.
A standalone WheelCollider probe cannot replace assembled Lada evidence.

The scenario set includes:

| Scenario | Observation |
| --- | --- |
| Curve boundary | Evaluate zero RPM, each curve endpoint, and values outside both endpoints. |
| Launch | Start from rest with default assists, then with manual gears and clutch control. |
| Gear and direction | Select forward, neutral, and reverse. Change direction after braking. |
| Grip | Measure longitudinal slip, lateral slip, and combined slip for full-condition tires, worn tires, bare rims, and empty hubs. |
| Rim removal | Remove one driven rim, then both. Record the divisor, motor torque, engine-brake torque, and RPM feedback. |
| Wheel spin | Apply torque with ground contact and with driven wheels clear of the ground. |
| Suspension | Record settled height, full travel, load response, and settling after a controlled drop. |
| Braking | Measure service brake, engine brake, and handbrake separately and in combination. |
| Motor and brake | Apply throttle with each eligible brake and with all brakes together. |
| Coast and hold | Release throttle, disengage clutch, stop, and hold on a measured local slope. |

Use three fresh source runs per scenario with identical initial state.
Store the raw samples and hashes in ignored storage.
Commit compact numeric fixtures, scenario definitions, and provenance receipts.
Do not commit copied source assemblies or asset bundles.

Longitudinal, lateral, and combined-slip responses each require qualifying native observations before implementation.
The capture records car and engine Update callbacks as well as physics callbacks.
The source AutoShift call precedes Stuff and reads the preceding average-RPM sample.
The timing receipt maps each control update, engine output, and RPM sample to its consuming physics step.
The comparison uses that recorded mapping. It does not assume one render update per physics step.

The source runs define a comparison envelope before candidate tuning.
Each fixture records units, sample times, repeat variation, and capture precision.
Continuous comparisons use the source repeat envelope plus recorded measurement precision.
Discrete events use the observed source event window.
Do not expand an envelope to accept a candidate result.
Unstable captures require diagnosis and another capture.

If native capture cannot produce qualifying data, stop physics implementation at this requirement.
Report the missing evidence and retain G11 as Partial.
Do not substitute guessed tire equations or generic vehicle tuning.

## Component boundaries

Rapier retains rigid bodies, colliders, spatial queries, and body integration.
A local module under `src/vehicle/` owns wheel contact, suspension, tire forces, and wheel angular state.
The stock `DynamicRayCastVehicleController` path is removed when the new module replaces it.
There is one active wheel solver. There is no compatibility mode.

`WheelAssemblies` retains item identity, attachment rules, conditions, mass ownership, and collider ownership.
It resolves each slot into an explicit hub, rim, or tire support profile.
An empty slot has a hub profile, rather than an absent physics wheel.
Hubcaps remain assembly state. They do not create another physics wheel.

Each profile contains the measured radius, angular mass inputs, damping, and friction parameters.
The source curves remain distinct for longitudinal and lateral response.
Native evidence determines their evaluation and combination rules.
Source stiffness is not copied into Rapier's unrelated `friction_slip` field.

Four stable wheel slots exist for each current four-wheel vehicle record.
Each slot owns its support profile and dynamic wheel state.
Contact state and assembly state are separate.
A wheel without ground contact can still rotate under torque.

The existing engine owner retains combustion, engine RPM, and clutch attenuation.
The drivetrain retains gear selection, assist rules, and final-drive multiplication.
The wheel solver receives motor, engine-brake, service-brake, and handbrake torque.
The source wheel roles determine which torque reaches each slot.
The torque divisor is the count of installed driven rims, clamped to one.
Tire presence and ground contact do not change that divisor.
Every driven slot, including a hub, receives the source motor and engine-brake assignment.
RPM feedback sums every driven collider's signed RPM and divides by that same installed-rim divisor.
These rules remain explicit when one driven rim or both driven rims are absent.

No new library or general vehicle framework is required.

## Fixed-step data flow

The existing fixed-step duration remains unchanged.
The sequence below describes the current runtime integration, rather than an assumed Unity callback schedule.
Native traces must validate its input and output timing before the wheel solver is accepted.
Preserve the source delay between automatic gear selection and refreshed average RPM.
This design does not authorize an engine or drivetrain scheduling rewrite.
A measured timing mismatch requires a design revision before that scheduling changes.

1. Resolve controls, stored handbrake state, assist choices, and selected gear.
2. Read source-defined RPM feedback from the preceding solved wheel state, using the installed-driven-rim divisor.
3. Step the existing drivetrain and engine in their current source order.
4. Calculate axle torque with clutch engagement and gear ratios applied once.
5. Assign torque using the source wheel roles and installed-driven-rim divisor, including hub and rim states.
6. Query contacts and solve suspension, tire response, and wheel angular motion.
7. Apply body impulses at the measured force points.
8. Integrate Rapier bodies and publish wheel poses and RPM.

The solver uses contact-point velocity, rather than chassis speed alone.
Contacts with dynamic bodies use relative motion and matching reaction impulses.
The existing filters continue to exclude the owning chassis and sensors.

Brake torque opposes angular motion and can hold a stopped wheel.
Brake torque cannot create reverse rotation by crossing zero.
Motor torque and all eligible brake contributions remain present in the same step.
Clutch release removes engine coupling according to source control flow.

Angular speed drives wheel graphics and drivetrain feedback.
The solver does not derive angular speed from the displayed wheel angle.
Suspension pose uses solved travel, including unloaded and airborne states.

## State and lifecycle

Support changes update the affected slot without rebuilding all four wheels.
Other slots retain their angular state and suspension state.
The affected slot follows the measured source transition rule.
Assembly transitions continue to preserve item identity, condition, mass, and collision ownership.

Wheel angular mass does not add duplicate physical mass to the chassis.
The existing assembly mass owner remains authoritative.
The design keeps angular inertia and chassis mass accounting separate.

Existing save fields hold wheel angle, RPM, and previous angle.
The solver restores angular speed from saved RPM and preserves those fields' meanings.
Contact samples are rebuilt from geometry after load.
The first restored solve must not use stale contacts or a false angular-speed spike.
Gear, engine state, assist choices, and stored handbrake state keep their existing owners.

Recovery resets body and wheel dynamics through the existing recovery path.
Origin changes translate contact positions without changing speed or wheel angle.
Pause and capture loss retain drivetrain state and clear input through existing paths.

All internal callers migrate to the new support and wheel-state interfaces together.
Remove obsolete active-slot maps, resting-controller copies, and the old controller reconstruction path.
Rename count methods when their old installed-wheel meaning no longer applies.
Tests distinguish slot count, installed-rim count, and current contact count.

## Allocation and error rules

Wheel state uses fixed-size arrays.
Contact work uses fixed storage and the existing query infrastructure.
Profile resolution runs on assembly or condition changes.
Curve setup does not run every frame.
The physics step performs no new heap allocation.

The exporter and manifest loader reject missing or invalid required parameters.
They check finite values, curve ordering, radii, and angular mass inputs.
Validation permits source-supported zero values.
Internal solver code trusts validated records.

A source hash mismatch invalidates a capture.
Missing samples or nonfinite results fail the comparison.
Invalid physics values do not silently fall back to generic tuning.

## Verification and completion

Critical native tests cover these contracts:

- Hub, rim, and tire transitions retain four slots and correct assembly ownership.
- Airborne torque changes angular speed without requiring chassis movement.
- Combined motor and brake torque follows the captured source response.
- Suspension travel and settling remain within the captured source envelope.
- Default automatic launch, manual shifts, clutch thresholds, and reverse transitions follow source behavior.
- Removing one driven rim or both preserves the source torque divisor and RPM feedback rule.
- Save, load, recovery, and origin changes preserve the required wheel state.

One exported-Lada integration route checks launch, coasting, braking, and wheel-profile changes.
Fixture comparisons report the scenario, metric, source envelope, and candidate value on failure.
A scripted route proves only the tested simulation behavior.

Rust changes require targeted native tests and both configured WASM compile checks.
Exporter changes require focused Python tests.
Web input tests run only if web controls change.

Release builds, full checks, and browser acceptance remain unapproved.
An agreed browser milestone must test normal input, active physics, visible wheel behavior, and submitted frames.
That milestone includes launch, shifting, clutch use, rims, hubs, suspension, wheel spin, and combined braking.
Browser results must identify the tested commit and matching asset package.

The roadmap records code coverage separately from browser acceptance.
G11 remains Partial while required behavior or qualifying source comparisons are missing.
G11 changes to Verify when those code and comparison requirements pass.
Browser acceptance remains explicitly open until its agreed milestone passes.
G11 is accepted only after that browser milestone passes.
No result claims full PhysX solver equivalence.

## Work sequence and exclusions

The implementation plan starts with measurement capture and its evidence requirement.
It then adds support profiles, the measured wheel solver, integration, and critical verification.
Each layer leaves a usable product before the next layer begins.
Subagents receive separate file ownership for measurement, wheel physics, and drivetrain verification.
One integration owner controls shared vehicle interfaces.

This task excludes extra vehicles, long-distance world generation, cargo, towing, and renderer optimization.
It does not alter unrelated engine maintenance, inventory, or player controls.
A required change outside these boundaries needs a design revision.

The user approved this written specification on 2026-10-20.
Implementation remains subject to implementation-plan review.
The selected execution method uses subagents.

## Source and code pointers

- `docs/roadmap.md`: G11, combined braking, garage scope, and the native measurement requirement.
- `docs/VEHICLES.md`: current drivetrain ownership and unresolved adapter limits.
- `tools/reference/driving.py`: checked managed hashes, source methods, and wheel export.
- `src/reference.rs`: `WheelSlotReference`, hub records, and friction curves.
- `src/interaction/wheels.rs`: assembly resolution and current radius-only support.
- `src/vehicle.rs`: stock controller setup, wheel forces, RPM feedback, save state, and support replacement.
- `src/simulation/drivetrain.rs`: source shift order and transmitted axle torque.
- `src/simulation/engine.rs`: torque curve evaluation and clutch coupling.
- `src/save/schema.rs`: `WheelAttachment` angle and RPM fields.
- `.cache/reference-library/records/managed/Assembly-CSharp.cs`: checked native-build managed behavior.
