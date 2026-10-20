# G05 engine design

Date: 2026-10-20.
Status: Design approved in chat. Written specification awaits review.
Base commit: `cc7540b255d683722c67e09103a66bfa789c0fe7`.
Branch: `codex/g05-engine`.

## Goal and scope

Complete roadmap G05 for the garage Lada from Steam build `11140890`.
Cover ignition, starter, fuel use, cooling, oil effects, stalling, and missing parts.
Preserve the existing fixed gameplay step and item identities.

The user approved a focused ownership split instead of a full engine rewrite.
Use an isolated worktree and subagents. Keep the branch unmerged.

Driving physics, extra engines, new audio systems, and general fluid transfer changes remain outside G05.
Full checks, release builds, and browser acceptance require separate authorization.

## Current defects

`src/simulation/engine.rs` combines chassis ignition with removable engine mechanics.
Its missing-engine branch resets ignition, RPM, fuel supply, and run grace.
Source chassis ignition persists. Detached engines continue their own update.

Starter chance checks reuse one sample throughout a held press.
Source takes a new sample at each eligible interval.
Engine drains use `spill()`. Source uses sequential `ChangeWithoutMix` changes.
Fuel suitability and heat penalties also enter the run decision in the wrong update order.

The existing starter release behavior matches source.
A long-hold release retains the starter command for one update.
The next neutral update clears the command. Preserve this behavior.

## Ownership and data flow

A small `IgnitionState` owns the chassis ignition latch and starter transitions.
`VehicleRuntime` owns this state. It remains usable without an installed engine.
Held keys, hold duration, and prior held state are transient input.

`EngineState` owns RPM, temperature, run grace, fuel supply, penalties, start timing, and random draw state.
Keep the current engine instance associated with its source engine item after removal.
The Lada engine retains its known fuel-tank binding. No general connection system is needed.

`ItemStore` continues to own fuel, sump, and radiator containers.
Removal and installation retain the same containers, quantities, order, identities, and temperature.
Do not create replacement fluid state during these transactions.

Each fixed step resolves attachment and chassis controls before engine input.
Only an installed engine receives chassis ignition, starter, throttle, clutch, wheel RPM, and speed.
A detached engine receives zero chassis controls and continues its mechanical update.
Define removal as the first fixed step after the part transaction changes attachment.
Do not reproduce an unproven final queued Unity input during that step.

Wheel torque and dashboard RPM require an installed engine.
Chassis ignition, dashboard ignition, ignition sounds, and vehicle lights read `IgnitionState`.
The drivetrain, instruments, diagnostics, and restored lighting must use their new owners directly.
Remove obsolete engine-owned ignition fields and readers. Do not add compatibility wrappers.

## Engine rules

Preserve the source shutdown threshold, oil threshold, heat threshold, fuel threshold, and 0.2-second run grace.
Preserve the torque curve, starter torque, engine braking, thermostat, radiator condition factors, and fan behavior.
Do not change the source starter release delay or fan behavior to make them appear smoother.

The run decision reads the previous engine fuel suitability and heat penalty.
The engine then refreshes fuel suitability and the heat penalty at their source-local stages.
Store these values explicitly. Refresh oil effects only through the mounted service step.
Initialize cached fuel suitability, heat penalty, and oil penalty to zero.
The Lada record `level1:68282` stores these values as zero.
Initialize the start sample to one, as assigned by `enginescript.Start`.

Cross-script Unity execution order is unproven.
Keep the existing fixed-step order for mounted oil service, evaporation, and radiator cooling.
Document the chosen sequence in the implementation tests without claiming exact Unity frame timing.

A detached engine keeps RPM, temperature, run grace, supplied fuel, and its last fuel binding.
During run grace, it can consume fuel from that binding and gain heat.
After grace, supplied fuel clears and RPM approaches zero.
Passive cooling continues with zero chassis speed.
Detached engines do not refresh oil effects, consume sump fluid, or receive radiator cooling.

A missing radiator disables radiator cooling, leakage, and coolant evaporation.
An installed hot engine can evaporate coolant while stopped.
An empty radiator cannot remove coolant or provide fluid cooling.

Reuse `FluidContainer::change_without_mix` for source engine drains.
Apply each source quantity and sleep factor once.
Preserve retained insertion order and source cleanup behavior.
Each initial Lada container has one fluid type. Later mixed contents retain their insertion order.
Empty and depleted containers must produce finite values and valid quantities at the publication boundary.
Do not change general transfer behavior or silently substitute proportional drains.
Keep shared fluid changes limited to the retained-order metadata required by this save contract.
Reconcile conflicting G04 changes before integration. Do not absorb unrelated G04 behavior.

Advance a separate engine draw counter only when a start-chance check consumes a new sample.
Use the existing deterministic hash to obtain each sample without per-frame allocation.
Keep `progress.start_attempts` for its existing attempt and start-impulse roles.
A save and restore must preserve the next sample.

## Audio consumers

Chassis ignition sounds follow the chassis switch.
Existing idle, motor, starter, and stop sounds follow engine state and the engine emitter.
Attachment must not suppress the detached engine's run grace or its genuine stop transition.
Detachment alone must not generate a false stop sound.
The starter receives no new chassis command after detachment.
Do not introduce a new audio subsystem or new cooling-tick behavior in G05.

## Save contract and errors

Increase save schema version 6 to version 7 on this baseline.
Move the ignition latch from `EngineSnapshot` to `VehicleSnapshot`.
Keep engine mechanics and cached penalties in `EngineSnapshot`, including detached state.
Save retained fluid order with the quantities for each item container.
Restore both values together. This metadata is required because sequential drains depend on order.
Order entries must be unique fluid indices between zero and five.
The order must contain every positive fluid entry and preserve valid retained zero entries.
Items without containers must not carry fluid-order metadata.
This changes save metadata only. It does not replace the existing fluid algorithm.

Store the engine draw counter as a lossless decimal string.
Use the existing unsigned integer parser and strict field validation.
Reject version 6. Do not add migration, defaults for obsolete fields, or compatibility paths.

Save validation requires finite mechanical values and valid penalty ranges.
Elapsed time is nonnegative. Run time is between minus one and elapsed time.
Start-chance time is between zero and elapsed time.
Random samples remain between zero and one. Supplied fuel remains nonnegative.
Permit detached RPM and residual run grace.

Load restores attachment, chassis ignition, and engine mechanics before deriving instruments and audio state.
Do not save held keys, hold duration, prior held state, or an active starter command.
Pause, seat changes, death, recovery, and load clear transient commands immediately.
Input cancellation preserves ignition and mechanics. It must not create a short-release ignition toggle.
Clear published starter state even when no simulation step follows.

Keep validation at existing reference and save boundaries.
Do not hide missing references with fallback values or add frame-path allocations.

## Critical verification

Use installed build `11140890` records for integration tests.
The managed worktree does not contain ignored source files or runtime assets initially.
Make the required installed export available explicitly before export-backed tests.
Do not generate a replacement fixture and treat it as source evidence.

The native checks cover these cases:

| Case | Required result |
| --- | --- |
| Short tap, long hold, release, and cancellation | Correct ignition and starter transitions, including the source release delay. |
| One held start with failed and successful draws | New interval samples and repeatable continuation after save and restore. |
| Empty, wrong, mixed, and depleted fuel | Source suitability, sequential consumption, finite depletion, and correct run grace. |
| Low oil and excessive heat | Source power penalties and stalls at the defined thresholds. |
| Missing, empty, and worn radiator | Correct cooling, leakage, evaporation, and hot stopped behavior. |
| Engine removal and reinstallation | Retained state and fluids, cleared chassis input, retained fuel binding, and zero wheel torque while detached. |
| Ignition without an engine | Working chassis switch and dashboard ignition, with zero dashboard RPM. |
| Engine audio after removal | Engine emitter, grace playback, and one genuine stop transition. |
| Save validation and restore | Strict version rejection, valid counters and fluid order, preserved mechanics, and cleared starter input. |
| Mixed-fluid save and restore | The same fluid order, quantities, and subsequent engine consumption as an uninterrupted run. |

Run targeted native tests and both existing WASM backend compile checks after product changes.
Run targeted web tests only if web behavior changes.
At the specification stage, check the document and Git diff only.
Do not run full checks, release builds, or browser acceptance at this stage.

Update `docs/roadmap.md` with changes, checks, failures, deferred checks, and runtime limits after implementation.
Keep G05 at `Verify` while browser acceptance remains open.
Native tests and compilation do not prove normal gameplay.

## Source evidence

Behavior authority is `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs` from build `11140890`.
The original checkout currently holds that ignored file and `web/assets/reference/manifest.json`.
Initial cached values come from `.cache/reference-library/records/level1/68282.json`.
The startup manifest confirms one initial fluid type in each Lada engine-bound container.

| Source method or block | Lines | Contract |
| --- | --- | --- |
| `carscript.Update` ignition | 7768-7817 | Chassis latch, starter delay, and release behavior. |
| `enginescript.inputs` | 12023-12045 | Controls clear, but the fuel binding persists. |
| `enginescript.Running` and `Update` | 12048-12145 | Run grace, decision order, fuel use, start samples, and RPM. |
| `enginescript.Update` | 12170-12213 | Heat penalty, heat gain, passive cooling, and engine audio. |
| `enginescript.Upd` | 12262-12279 | Mounted oil effects and sump consumption. |
| `carscript.Update` engine slot | 7558-7592 | Engine attachment and mounted service. |
| `carscript.Update` evaporation | 7371-7393 | Hot installed engine and positive radiator contents. |
| `coolantTankscript.Cool` | 9953-10012 | Thermostat, cooling, leakage, condition, and fan behavior. |
| `fluidcontainer.ChangeWithoutMix` | 28381-28386 | Sequential changes in retained fluid order. |
| `fluidcontainer.canRun` | 28430-28457 | Fuel suitability. The modifier argument is unused. |

## Review and next stage

The user reviews this committed specification before implementation planning.
The next stage uses the writing-plans skill.
The user then reviews the written plan and selects execution before product edits begin.
