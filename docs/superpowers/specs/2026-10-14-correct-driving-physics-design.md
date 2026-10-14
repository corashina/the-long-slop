# Correct driving physics

Date: 2026-10-14, Europe/Warsaw.
Base: local master `c189df36ade62f04d361083f11f09a219f1cc1be`.
Status: The user approved the design direction. This written specification awaits review.

## Goal and scope

Complete the driving physics work in [the roadmap](../../roadmap.md) for the Lada and Skoda.
Use installed Steam build `11140890`, Unity `2021.3.18f1`, as the source.
Include manual gears, clutch control, pedal ramps, combined braking, tire friction, bare rims, and suspension.
Keep each layer playable before adding the next layer.

Keep the current world, fleet, parts, fluids, engine simulation, lighting, audio, and save owners.
Integrate the required driving state with those owners.
Use the existing 60 Hz simulation step and pinned dependencies.
Implement in a separate worktree based on the current master.

Limit menu work to driving assists, driving feedback, and the required control help.
Exclude control rebinding, controller axes, H-pattern hardware, new cars, towing, and multiplayer.
Exclude deferred source-world integration and unrelated save gaps.
Keep merging and pushing outside this implementation request.

## Current behavior and source evidence

The [driving audit](../../research/2026-10-11-rea-driving-audit.md) records the source identity and earlier research.
The [vehicle record](../../vehicles.md) describes the current two-car ownership model.
Check the installed source hashes before extending the recovered evidence.
The audited assembly SHA-256 is:
`9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.

| Area | Current implementation | Required change |
| --- | --- | --- |
| Steering | `KeyboardSteering` applies the exported rate at fixed steps | Extend this owner for pedals; retain steering behavior |
| Pedals | `step_engine` derives a boolean throttle; `DriveInput` uses boolean brake/reverse | Carry separate throttle, brake, and clutch values |
| Transmission | `select_gear` chooses automatic gears; `step_engine` derives automatic clutch engagement | Add source manual shifts and explicit assist state |
| Braking | `apply_vehicle_forces` selects one brake contribution | Add eligible contributions before solving wheel motion |
| Wheel state | `InstalledWheel` carries radius and condition | Carry tire presence, wheel mass, and separate friction data |
| Friction | One `friction_slip` value represents tire and bare-rim grip | Evaluate longitudinal and lateral slip with source curves |
| Suspension | The adapter reduces travel to a symmetric interval | Use full source travel, target position, and force application geometry |

Use these recovered methods as the gameplay authority:

- `inputscript.DriveStuff`: keyboard ramps, shift edges, and assist-dependent input.
- `settingsscript.RefreshSettings`: assist dependencies and defaults from the linked controls.
- `carscript.IsAutoShift`, `AutoShift`, `ControlStuff`, `ShiftTo`, and `Stuff`: transmission and clutch rules.
- `carscript.SetWheel`: signed motor torque and additive brake torque by wheel role.
- `enginescript`: clutch coupling, wheel RPM, free-run behavior, engine output, and engine braking.
- `gumiscript.Refresh`, `wheelscript.Refresh`, and `wheelgraphicsscript.WheelSize`: condition, tire, rim, and hub behavior.

Retain method identities, source field names, and source record identities beside each derived rule.
Check active callers and startup changes before treating serialized values as runtime values.
Keep recovered source and raw captures in ignored local storage.

## Physics approach

Retain Rapier for rigid bodies, collision, queries, joints, and impulse application.
Replace its stock wheel controller with one focused wheel owner for the selected cars.
Reuse Rapier query and body APIs rather than adding another physics engine.

The pinned [Rapier controller](https://raw.githubusercontent.com/dimforge/rapier/v0.36.0/src/control/ray_cast_vehicle_controller.rs) has two relevant limits.
It uses a shared traction limit rather than separate source slip curves.
It also selects motor force or braking during longitudinal impulse calculation.
Changing tuning values cannot provide simultaneous motor torque and braking.
Its wheel rotation follows ground velocity, which cannot represent powered wheel spin.

A tuning-only change leaves those requirements incomplete.
Replacing the complete physics engine would also disturb unrelated collision and interaction code.
Replace only the wheel layer and remove the obsolete controller path after its replacement passes checks.
Retain upstream license and provenance for any reused Rapier code.

Unity documents separate slip curves and suspension properties in its
[friction guide](https://docs.unity3d.com/2021.3/Documentation/Manual/wheel-colliders-friction.html) and
[component reference](https://docs.unity3d.com/2021.3/Documentation/Manual/class-WheelCollider.html).
Those public properties do not reveal the complete Unity solver.
Treat wheel solver equations as an adapter that requires measured comparison.
Do not claim identical Unity physics from copied parameters.

## Ownership and data flow

| Owner | Responsibility |
| --- | --- |
| `tools/reference/driving.py`, called by the existing exporter | Derive driving parameters, defaults, bindings, and source evidence |
| Existing typed reference contract | Require and validate the new records before scene construction |
| `src/simulation/input.rs` | Own steering and pedal ramps and clear transient input |
| `src/simulation/drivetrain.rs` | Own selected gear, shift rules, assists, clutch output, and signed torque routing |
| Existing engine owner | Compute RPM, running state, fuel use, torque, and engine braking |
| `src/vehicle/wheels.rs` | Own four fixed wheel slots, contact, suspension, spin, slip, and wheel forces |
| `src/vehicle.rs` | Connect wheel forces to Rapier bodies and expose wheel snapshots |
| Existing wheel interaction owner | Publish tire, rim, hub, radius, mass, and condition changes |
| `VehicleRuntime` and `GarageRuntime` | Connect per-car state, fixed-step order, lifecycle, and existing consumers |
| Existing save modules | Capture, validate, and restore persistent driving state |
| Existing browser input, menu, and feedback | Send input and assist changes; display controls and authoritative Rust output |

Use one drivetrain and one wheel owner per car.
Keep walking and push-grip steering on their current input paths.
Step unoccupied cars with cleared player input and their retained mechanical state.
Do not change gear to neutral because the player leaves the seat.

Process each fixed step in this order:

1. Consume button edges once and advance pedal and steering ramps.
2. Apply source assist rules, shift requests, and clutch coupling in recovered source order.
3. Step the engine using signed driven-wheel RPM and the selected gear ratio.
4. Compute motor torque and the three brake contributions for each wheel.
5. Query wheel contacts, solve suspension and tire forces, and advance Rapier.
6. Publish wheel poses, RPM, lights, instruments, audio inputs, and trip distance.

Preserve source ordering inside each step, including reads of the previous completed wheel sample.
Use fixed arrays for four slots and allocate query storage during loading.
Mount changes update the affected slots without rebuilding the controller during frame updates.
Do not allocate, rebuild maps, or create physics objects during ordinary wheel updates.

## Export contract

Extend the current runtime export. Read both cars through their existing source links.
Require assist defaults, manual shift rules, gear startup state, pedal rates, and keyboard bindings.
Export active per-car transmission flags and active global constants used by the selected source paths.
Retain gear ratios, differential ratios, free-run flags, torque curves, and wheel-role membership.

Export wheel mass, damping, force application distance, suspension geometry, and separate friction curves.
Include tire condition endpoints, bare-rim stiffness, hub properties, and active lateral modifiers.
Resolve source transforms and runtime overrides before deriving body-local values.
Preserve source wheel roles after part exchange between cars.

Reject missing links, nonfinite values, invalid gear indices, and invalid curve or suspension bounds.
Keep the accepted export intact if derivation or validation fails.
Regenerate required exports and fixtures. Remove obsolete fields and parsing paths.

## Pedals, gears, and assists

Clamp pedal targets to `[0, 1]`.
Move keyboard throttle, brake, and clutch toward their targets at the source rate of five units per second.
Retain the exported steering rate. Do not add unproven speed scaling.
Keep throttle and brake independent so the player can press both.

Provide sequential shifts and direct reverse, neutral, and available forward-gear requests through source keyboard bindings.
Consume shift requests as press edges. Ignore browser key-repeat events.
Validate requested gears against the current car's source gear set.
Use source clutch thresholds and shift rules rather than introducing shift delays or damage rules.
The recovered manual shift path requires clutch depression greater than `0.5`, subject to source assist rules.

Clutch pedal zero means released; one means pressed.
Convert pedal depression to engine engagement at the engine boundary.
Neutral disconnects the engine. Reverse uses signed gear and wheel speeds.
Preserve source free-run, stall, restart, and engine-braking behavior.
Apply clutch coupling to motor output and engine braking through one authoritative path.

Add two controls to the existing Controls screen: `Automatic clutch` and `Automatic gears`.
Initialize them from checked source defaults.
Enabling automatic gears enables automatic clutch. Disabling automatic clutch disables automatic gears.
Apply source automatic input inversion, direction thresholds, clutch rules, and ignition rules when assists require them.
Keep automatic selection as source gameplay behavior through the same drivetrain owner.
Remove the separate unconditional automatic approximation.

Apply assist changes while paused. Clear transient driving input before resuming.
Preserve selected gear through an assist change; apply source selection on the next active step.
Display the selected gear as `R`, `N`, or its forward number in the existing feedback area.
Update control help and README controls with the exported bindings.
Do not invent gear bindings that conflict with existing actions.
Resolve any source binding conflict through seated input context and document that mapping.

## Braking and wheel mechanics

Compute each eligible wheel's brake torque as:

`engine brake torque + service brake torque * brake pedal + handbrake torque * handbrake value`.

Distribute engine torque and engine braking over the source driven-wheel set.
Apply service braking and handbraking to their source wheel sets.
Retain simultaneous motor torque and braking during the wheel solve.
Use metres, seconds, kilograms, radians, and newton metres inside the wheel owner.
Convert torque to force and force to impulse once at the Rapier boundary.
Prevent brake torque from reversing a stopped wheel by numerical overshoot.

Distinguish a tire on a rim, a bare rim, and an empty hub.
Use each configuration's source radius, mass, and friction properties.
Read empty-hub contact behavior from `WheelSize` and its source hierarchy; do not discard the slot by assumption.
Keep the source tire condition mapping, including stiffness endpoints `1.25` and `0.75` for the audited tire.
The audited bare-rim stiffness is about `0.4` forward and `0.3` sideways.
Export the exact source values instead of using rounded constants.

Retain separate forward and sideways curve parameters through the complete wheel update.
Maintain wheel angular speed from drive torque, brake torque, damping, inertia, and ground reaction.
Compute slip from contact-relative velocity and wheel speed, including dynamic ground bodies.
Apply contact forces to both dynamic bodies and preserve self-collision filtering.
Handle rest, reverse, locked wheels, powered spin, lost contact, and contact recovery without nonfinite values.
Use a documented low-speed numerical treatment and verify that treatment against the source stop behavior.

Use full suspension travel and the source target position.
Keep compression and extension bounds distinct when the target position is off centre.
Apply spring and damper forces at the source force application position.
Keep physical spring and damper coefficients independent of changing chassis mass.
Refresh supported mass and inertia after part changes through the existing mass owner.
Preserve wheel identities and unaffected wheel state when the player changes one wheel.

Record the adapter's slip equations, curve interpolation, load coupling, and numerical bounds before implementing the wheel layer.
Derive these from checked Unity/PhysX evidence and matched source measurements.
Flag any remaining solver uncertainty in the verification record.
Do not replace missing evidence with a direct Unity-stiffness-to-Rapier-friction conversion.

## Lifecycle and saves

Clear pedal ramps, steering ramps, queued shift edges, and held ignition on seat changes or input interruption.
Cover pause, capture loss, focus loss, death, recovery, load, and restart.
Input clearing must not create a shift or reset the retained gear.
New game restores source startup gear, handbrake, wheel state, and assist defaults.
Recovery clears stale contact and suspension history while preserving gear and installed-part state.
Floating-origin changes preserve wheel spin, selected gear, and physical velocities.

Replace saved `auto_gear` with validated transmission state and persist the two assist choices.
Persist wheel angular speed and other mechanical state required to continue the solver after load.
Do not save held buttons or pending shift edges.
Restore items and attachment-derived wheel configuration before restoring wheel mechanical state.
Reject inconsistent saved gears, wheel identities, or nonfinite mechanical values before replacing the active scene.
Increase the save schema version for the breaking contract change and reject older snapshots.
Add no migration, old reader, or fallback path.

## Working layers

| Layer | Deliverable | Required check before the next layer |
| --- | --- | --- |
| 1. Inputs and source records | Typed source data and independent pedal ramps through both cars | Ramp, input-reset, source export, and playable startup checks |
| 2. Transmission | Manual gears, clutch, source assists, controls, feedback, and save state | Launch, shift, neutral, reverse, stall, car switch, and load checks |
| 3. Wheel forces | Focused wheel owner with additive braking and independent wheel spin | Simultaneous motor/brake, wheel-role, contact, and playable drive checks |
| 4. Tires and suspension | Source curves, tire/rim/hub state, full suspension, and part changes | Grip, suspension, mass-change, part-exchange, and recovery checks |
| 5. Acceptance | Browser routes, source comparisons, current documentation, and scoped review | Recorded results and explicit unresolved limits |

Each layer must retain a running garage-to-road route for both cars.
Integrate save changes with the layer that introduces the state.
Remove obsolete paths as their replacements pass checks.

## Critical verification

Extend existing input, engine, vehicle, wheel, browser-input, and save tests where they cover the changed behavior.
Add focused drivetrain checks rather than duplicating formula tests across modules.
Check pedal rise and release, clutch shift boundaries, signed reverse torque, and additive braking eligibility.
Check powered spin, airborne spin, braking near rest, bare rims, hub state, and one-wheel replacement.
Check full suspension travel, settling after a bump, and changing mass without changing spring coefficients.
Check pause, seat switch, load, recovery, restart, and floating-origin state.

Run native tests, required installed-source checks, exporter validation, browser unit tests, and the release WASM build.
Use the existing toolchain and a worktree-specific target directory.
Run normal browser input routes at 30, 60, and 120 render frames per second.
Check startup, both cars, pedal release, manual launch, shifts, neutral coast, reverse, braking, and save/load.
Observe collision seams on the current road and the garage exit.
Record any browser capture limitation rather than claiming coverage from synthetic input alone.

Before tuning wheel forces, capture three repeated source runs per car under matched conditions.
Record build, car, parts, fluids, tire condition, pose, gear, assists, input duration, surface, and time step.
Measure acceleration, coast-down, stop distance, turning response, suspension settling, and tire-to-rim changes.
Use telemetry where available. Record video timing and measurement resolution for observed quantities.
Define numeric comparison bounds from source repeatability and measurement resolution before evaluating the changed solver.
Keep those bounds fixed during tuning and retain the raw measurements.
Report missing source measurements as unverified acceptance criteria.

Build success proves compilation. Formula checks prove the checked gameplay rules.
Claim matched handling only for routes with comparable source and browser evidence.
Keep the roadmap status partial until the required implementation and acceptance checks pass.
Update the roadmap with measured results and unresolved differences after implementation.

## Review result

This specification covers the approved driving scope for both existing cars.
The wheel adapter requires measured validation because public Unity properties do not specify its complete solver.
The implementation plan must establish wheel equations and comparison bounds before scheduling the wheel replacement.
The user must review this specification before implementation planning starts.
