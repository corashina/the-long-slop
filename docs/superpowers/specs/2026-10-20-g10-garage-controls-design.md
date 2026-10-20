# G10 garage and car controls design

Date: 2026-10-20.
Status: approved in-chat design, written spec awaiting review.
Base: `master`, commit `cc7540b255d683722c67e09103a66bfa789c0fe7`.
Branch: `codex/g10-garage-controls`.

## Purpose and completion

G10 completes the controls for the garage Lada and starting home.
The player must operate these controls through normal game input.
The behavior authority is Steam build `11140890` and its decompiled code.

The approved design extends the existing modules and fixes five source differences:

- Radio display materials and tuning needle position.
- Dashboard and gauge needle lighting.
- Door, hood, and trunk endpoint reactions.
- LMB operation of the handbrake lever.
- L-key light control from source secondary control seats.

G10 also verifies the existing controls through normal input.
Those controls include four doors, the hood, the trunk, windows, seats, home switches, car lights, horn, radio, and gauges.
A passing build does not establish gameplay acceptance.

The user approved this design and requested a spec in an isolated worktree.
Implementation, the written plan, and integration remain separate stages.
The final implementation milestone includes a release build and browser acceptance.
This spec-writing stage does not run those checks.

## Scope and boundaries

Use the selected Lada, starting home, garage, yard, and nearby terrain.
Keep existing drag rules, action priority, part ownership, and collider bindings.
Use the existing radio playback, lighting, instrument, and physics systems.
Do not add a shared control subsystem.

Custom music, additional vehicles, driving physics changes, and new control animations are outside G10.
The selected Lada has no exported horn or light-switch animation targets.
Do not add animations for those empty source arrays.

Seat entry, exit, and reachable seat controls remain acceptance requirements.
G09 owns the direct seat-switching correction and player posture work.
G10 owns the L-key eligibility correction for source secondary control seats.
Record a failed seat transition as a dependency on G09 instead of duplicating its implementation.
G10 remains unaccepted if that dependency prevents the required seat scenario.

G13 owns broader save acceptance.
G10 must preserve the existing saved control state and restore derived visuals without side effects.
No G10 save format change is expected.

## Source evidence and current state

The audit used `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs` from the primary checkout.
That ignored source cache is not part of the new worktree.
Implementation must use the same accepted source archive and build identity.
Line numbers below refer to the audited source file.

| Behavior | Source evidence | Current difference |
| --- | --- | --- |
| Endpoint reactions | `usablescript.RotSound` and `StartEndStuff`, lines 66787-66850 | `tools/reference/mechanisms.py` omits force fields. `MechanismState::step` changes transforms without reactions. |
| Handbrake input | Player usable action dispatch, lines 17743-17767 | `MechanismState::primary_switch` excludes the handbrake from LMB use. |
| Secondary control seats | `carscript.GetPlayerInput`, lines 8515-8548 | `GarageRuntime` gates the L shortcut on `driving()` alone. |
| Radio display | `radioscript.Update`, lines 49975-49998 | Radio export and runtime schema omit display materials and tuning needle links. |
| Dashboard lighting | `carscript` meter update, lines 8865-8888, and `meterscript.TurnOn` and `TurnOff`, lines 34097-34132 | Instrument export omits dashboard and needle material links. Presentation changes digit materials only. |
| Gauge needle lighting | Needle `TurnOn` and `TurnOff`, lines 35593-35600 | Needle rotations exist, but their material switching is absent. |
| Direct seat switching | Player `GetIn`, lines 19076-19089 | Occupied-seat entry and action dispatch block source switching. G09 owns this correction. |

Home and cabin switches already have an input path through `InteractionAction::LightControl` and `LightingState::use_control`.
Ignored mechanism light actions alone do not establish a current failure.
Remove an obsolete path only if the G10 change replaces its callers.

These findings come from static source and code inspection.
No current browser result establishes control reachability, readable gauges, audible playback, or source physics parity.

## Reference data contract

Extend the existing reference export and Rust records together.
Use the existing `NodeId`, `SourceId`, `BodyId`, and `MaterialLink` types.
Validate source pointers and material links at the export and manifest boundaries.
Do not add defaults for fields missing from an old export.
Regenerate the runtime package after schema changes.

A source-null component may have an explicit absent value.
A missing required Lada component is an export error.
The exporter must distinguish those cases.

### Endpoint reaction records

Extend `MechanismReference` with source reaction data for applicable rotation controls.
Each record contains the source control node, local force axes, inversion flag, force magnitude, and configured endpoint events.
The event set distinguishes entering and leaving each minimum or maximum endpoint.
Endpoint clip presence defines whether the source calls `StartEndStuff` for that event.
Do not use the source `startForce` or `endForce` fields as gates. The audited method does not use them.

Export the original source axis convention explicitly.
Convert vectors once through the existing Unity-to-runtime transform rules.
The four doors, hood, and trunk use source force magnitude `10000` with configured axis flags.
Read that value from the source record. Do not hard-code it in runtime logic.

### Radio visual records

Extend the existing radio reference with visual data separate from audio voices.
Include the screen renderer, tuning needle renderer, and their on and off material links.
Include the tuning needle transform and both source endpoint transforms.
Preserve the source identities of each reference.

The visual state uses existing radio power and normalized tuning.
It must not create a second radio clock, frequency state, or power state.

### Instrument visual records

Extend `InstrumentReference` with the dashboard renderer and its on and off materials.
Extend each `NeedleReference` with its renderer and on and off materials.
Keep current needle ranges, smoothing, warning lamps, and odometer records.

Use the source renderer identity when it differs from the moving needle transform.
Validate both identities before package promotion.

## Runtime behavior

### Endpoint reactions

`MechanismState` remains responsible for rotation limits and endpoint transitions.
Compare previous and current source angles after each input update.
Emit an event only when an angle enters or leaves a configured endpoint.
Holding input against an unchanged endpoint emits no event.
Preserve each source endpoint event when one update crosses the full angle range.

Retain the input value before angle clamping for reaction intensity.
Use the source rule:

```text
intensity = clamp(abs(input) / clamp(dt, 0.001, 1000) * 0.01, 0, 1)
force = source_world_axes * source_force * intensity
```

Apply source inversion to the axes.
Reverse the force for an event that leaves an endpoint.
Apply the reaction at the current world position of the source control.
Select the control's current root rigid body, including a detached part where applicable.
A root without a rigid body receives no reaction, as in the source.

Convert source `ForceMode.Force` into one physics-step contribution.
A single-step impulse uses `force * FIXED_DT` and preserves the force application point.
Do not leave a persistent force on a Rapier body.

Keep pending reactions in preallocated state sized from the exported controls.
Consume each pending reaction once within its physics step.
Do not allocate, search the manifest, or create body bindings per frame.
Restore, mount, rebind, and initial setup must not emit endpoint reactions.
Keep rotation, collider, and visible transform updates in the existing synchronization path.

### Input rules

The handbrake lever accepts E and LMB through the existing action dispatch.
Each press toggles once. Holding either input does not repeat the toggle.
Existing held-item and action reservations retain priority.
Space continues to use the existing vehicle handbrake path.

The L shortcut accepts `driving()` or `driving2()` for the selected vehicle.
Ordinary passenger seats gain no control unless the source assigns a control role.
Do not change throttle, brake, clutch, or gear eligibility in G10.
Horn eligibility already covers both control roles and must remain unchanged.

### Radio visuals

Radio screen and tuning needle materials use the existing effective power result.
The source rule is radio ON and electronics output above the exported cutoff.
Do not add an ignition requirement that the source does not have.

Move the tuning needle between the two source endpoint transforms using normalized tuning clamped to zero through one.
Resolve those endpoint transforms in the current car hierarchy.
The needle follows tuning while radio power is off.
Do not derive needle position from rounded frequency or audible station selection.

Reuse the current radio controls, station selection, audio unlock, and bounded audio cache.
Volume zero turns the radio off through the existing source rule.
AM and FM tuning retain their exported ranges, steps, and repeat timing.

### Dashboard lighting

Dashboard, needle, and digit materials share the existing instrument lighting condition.
They light when local ignition is active and the headlight state is low or high.
Local ignition retains the existing electronics cutoff rule.
Warning lamps retain their separate source signal rules.

Extend the existing `InstrumentVisuals` material switching.
Bind renderer entities and material handles at setup.
Update materials when the lighting state changes.
Continue updating needle and odometer transforms through the current instrument state.
Do not clone materials or construct asset handles per frame.

### Pause and restored state

Pause and mouse-capture loss must stop held horn input through the existing input lifecycle.
Resume must not duplicate radio voices or replay control presses.
Restored radio and instrument visuals must derive from the restored runtime state.
Restore must not play endpoint sounds or apply endpoint reactions.
G10 tests cover these side-effect boundaries without expanding G13 save scope.

## Modules and data flow

| Owner | Responsibility |
| --- | --- |
| `tools/reference/mechanisms.py` and `src/reference.rs` | Export and validate endpoint reaction records. |
| `tools/reference/audio.py` and `src/reference/audio.rs` | Export and validate radio visual links. |
| `tools/reference/instruments.py` and `src/reference/instruments.rs` | Export and validate dashboard and needle material links. |
| `src/interaction/mechanisms.rs` | Detect endpoint transitions and permit the source handbrake inputs. |
| `src/interaction.rs` and `src/starting_area.rs` | Preserve action priority and apply reactions within the physics step. |
| `src/simulation/radio.rs` and `src/audio.rs` | Supply existing radio power and tuning without duplicate state. |
| `src/presentation/instruments.rs` and a focused radio presentation module | Bind and update visual transforms and materials. |
| `src/app.rs` | Connect runtime samples to the presentation systems. |

Data flows from source records to the validated runtime manifest.
Input flows through existing action targeting into mechanism, lighting, and radio state.
Physics consumes endpoint reactions during the same fixed step.
Presentation reads runtime state and changes already-bound transforms and materials.
Audio continues to consume its existing radio and vehicle signals.

## Failure handling

Export fails for invalid source references, non-finite reaction values, or missing required visual materials.
Runtime manifest validation rejects broken node, body, asset, and material links.
Presentation binding waits for valid pending assets through the existing readiness path.
A missing required asset reports the existing reference or save error.
Do not silently replace required visual links or accept stale schema records.

Generated controls must construct the new reference fields explicitly.
They must not gain fabricated Lada reaction data or materials.
If a source effect cannot be verified, record the limit and leave its acceptance open.

## Critical verification

Implementation adds focused tests for changed contracts and behavior.
The written plan selects exact test names and commands.

| Check | Required evidence |
| --- | --- |
| Export and validation | Source fields survive export. Invalid required links fail. Old records fail instead of receiving defaults. |
| Endpoint physics | Each configured event produces the expected direction, intensity, root body, and application point. |
| Endpoint boundaries | Sustained endpoint input does not repeat. Full-range crossings preserve events. Restore and rebind emit none. |
| Handbrake input | E and LMB toggle once per press. Held-item reservations retain priority. |
| Light shortcut | Both source control roles can cycle lights. A non-control seat cannot. |
| Radio visuals | Power selects both material pairs. Tuning reaches both endpoints, including when power is off. |
| Instrument visuals | Dashboard, needles, and digits follow the ignition and headlight matrix. Warning lamps keep their own rules. |
| Lifecycle | Pause, capture loss, resume, and restore do not retain horn input or duplicate radio playback. |

Run targeted Python tests for changed export behavior.
Run targeted native tests and WASM compile checks for Rust changes.
Run targeted web tests if browser input or audio code changes.
Follow `AGENTS.md` and `scripts/check.ps1` for project checks.
Do not run full checks during this task unless separately requested.

The approved final milestone includes `scripts/build.ps1` and normal-input browser acceptance.
Use one matching source export and WASM package for the recorded browser run.
Record the package identity, browser, controls used, result, and any blocker.

Browser acceptance covers these scenarios:

1. Start a session with active physics, mouse capture, and submitted frames.
2. Open and close four doors, the hood, and the trunk. Check visible movement and collider alignment.
3. Reach configured endpoints. Check one reaction per transition and no reaction from a sustained endpoint hold.
4. Operate source window controls and all five home switches through their normal input paths.
5. Enter and exit all five seats. Verify role-specific controls and record any G09 dependency.
6. Operate the handbrake lever with E and LMB. Verify one toggle per press and the existing Space control.
7. Use physical light controls and L. Verify low beam, high beam, cabin modes, and the high-beam indicator.
8. Verify rear lamps under source ignition, brake, and reverse conditions without changing driving physics.
9. Hold and release the horn. Pause, lose capture, and resume without a retained horn input.
10. Operate radio volume, tuning, and band controls. Verify audible playback, display power, and tuning needle movement.
11. Check gauges against the exported ranges and warning thresholds. Verify dashboard lighting for each ignition and headlight state.
12. Restore existing saved control state. Verify correct visuals without an endpoint reaction or duplicate radio voice.

Synthetic input tests may establish state transitions.
They do not replace normal-input browser acceptance.
If capture, assets, or another task blocks a scenario, record the failure and keep that scenario open.
Keep the G10 roadmap status at Verify until the required normal-input scenarios pass.

## Delivery and next stage

Keep the branch unmerged unless the user authorizes integration.
Update the canonical roadmap with changes, checks, failures, dependencies, and remaining runtime limits after implementation.
Do not describe static checks as gameplay parity.

This spec is the current review artifact.
After written-spec approval, use the writing-plans skill to prepare the implementation plan.
The user must review that plan and select its execution method before implementation.
