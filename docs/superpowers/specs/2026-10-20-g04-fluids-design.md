# G04 fluid handling design

Date: 2026-10-20.
Status: Approved and implemented on 2026-10-20. G04 remains `Verify`.
Normal-input browser acceptance and scoped final-fix review remain pending.
Base: `cc7540b255d683722c67e09103a66bfa789c0fe7`.
Branch: `codex/g04-fluids`.

## Purpose and scope

G04 completes fluid handling for the current garage Lada experience.
The player can operate caps, fill containers, pour fluid, and spill fluid.
Mixtures and empty supplies follow Steam build `11140890`.
Engine oil and radiator coolant remain with their parts after removal.

The task uses existing ownership, interaction, physics, and particle systems.
It adds no fluid ownership system or package dependency.
Clothing, survival expansion, general cargo, extra vehicles, and hose gameplay remain outside this task.
Shared fluid consumers receive only the changes required by the fluid contract.
Engine tuning and driving physics remain outside this task.

The main checkout has unrelated local edits.
All G04 files remain in the separate worktree.
The branch remains unmerged until the user authorizes integration.

## Authority and original gaps

Behavior authority is `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs` in the main checkout.
The file belongs to build `11140890`.
The implementation must use the matching installed garage export for integration checks.

| Source method | Source lines | Required behavior |
| --- | --- | --- |
| `fluidcontainer.pour` | 28321-28343 | Process entries in source list order and recalculate each fraction. |
| `fluidcontainer.ChangeOne` | 28346-28365 | Limit additions by remaining capacity and retain source subtraction behavior. |
| `fluidcontainer.ChangeWithoutMix` | 28368-28386 | Recalculate each fraction after previous mutations. |
| Player fill selection and `DrinkFill` | 17068-17085, 18878-18918 | Fill the selected receiver from the held tank. |
| `tankcapparticledetectionscript` | 62975-63041 | Receive emitted fluid through nearby caps without another source debit. |
| `tankcapscript.Update` | 63407-63475 | Apply tilt, valve, fill fraction, emission, and depletion rules. |
| `tankscript.Upd` | 63615-63624 | Remove the first nonpositive entry on each update call. |
| Source tank save and restore | 39861-39874, 40241-40251 | Preserve ordered quantities and run source load cleanup. |

Before G04, `src/fluids.rs` used simultaneous proportional transfer and clamped spilling.
Those calculations differed from the ordered source mutations.
Particle collision handling reported only player contact.
It could not fill another tank.
Supply targeting could reverse direct fill direction.
Export and save quantity arrays lost fluid list order.
Save capture clamped signed quantities and rescaled totals.

## Selected approach

Extend the existing fluid and particle paths.
A calculation-only fix leaves particle filling incomplete.
A replacement fluid subsystem adds unnecessary ownership and integration work.

`ItemStore` remains the owner of fluid state.
`FluidContainer` remains the unit for mixture operations.
`InteractionState` owns cap state, action selection, and receipt application.
`FluidContact` owns particle motion and collision reports.
Physics reports collisions without changing tank contents.

## Ordered fluid state

Runtime state retains the six fluid quantities and their source entry order.
Order includes retained zero entries and finite negative entries until source cleanup removes them.
New fluid types enter at the end, as in `ChangeOne`.
The installed garage export has one entry per tank and no duplicate types.
Export validation establishes unique fluid types as a supported garage invariant.
This invariant is narrower than the source list type.
Per-frame operations use fixed storage with a maximum of six unique fluid types.

Export preserves the serialized source list order.
Generated fluid construction preserves the order of its existing source definition.
No constructor or restore path rebuilds order by sorting fluid indices.

Transfer iterates source entries in order.
Each entry reads the current source fraction after previous changes.
The destination accepts its source-defined amount before the source subtraction.
Spill uses the same ordered recalculation as `ChangeWithoutMix`.
The implementation does not clamp source debit to an initial total.

Cleanup follows source `Tank.Upd()` calls.
Each call removes at most one nonpositive entry.
A helper must not remove all depleted entries as an extra cleanup step.

Valid source arithmetic can produce nonfinite runtime results near depletion.
Runtime fluid operations must not silently replace those results with zero.
Physics and presentation receive finite derived values where their APIs require them.
Those derived values never overwrite authoritative fluid state.

## Direct fill and caps

Direct fill transfers the held tank into the selected receiver.
A target classified as a supply does not reverse that direction.
The held tank cap can remain closed, as in the source direct-fill path.

Fill requires distinct tanks, remaining source fluid, receiver space, and an open receiver cap.
The source `noFillFrom` and receiver `noFillThis` flags apply to direct fill.
Receiver opening area, receiver valve, and source oil rules determine flow.
Action feedback and execution use the same eligibility rules.

Cap toggle and tuning retain their existing source rules.
Closing a receiver blocks fill.
An empty source stops fill and emission without creating fluid.
Full receivers accept no additional fluid.

## Free spill and particle receipts

Spill retains source tilt, fill fraction, valve, `noPour`, and tube gates.
Direct fill does not add a source-spill suppression rule absent from source code.
Each emitter records raw `poured = speed * dt` before sleep scaling.
Spill debit applies sleep scaling once.
A contact receipt applies sleep scaling once through the source pour operation.

Particle motion reports struck collider identity and contact position.
Interaction resolves the struck GameObject and its current transform-root tanks through existing source bindings.
Receiver caps use their current poses, including detached part poses.
The existing player contact path remains functional without adding survival behavior.

Collision events group by emitter and struck source GameObject before receiver selection.
Several colliders on one GameObject form one callback.
Different GameObjects on one rigid body form separate callbacks.
It scans root tanks, collision events, and tank caps in source order.
Encountering the emitter cap ends that callback.
The first cap within `radius * 0.1` ends that callback, including a closed or full cap.
A closed or full first cap does not redirect fluid to a later cap.
Contact filling does not add the direct-fill permission flags absent from `Tankolas`.
Another cap on the emitter tank can receive fluid.
Only the encountered emitter cap ends the callback through the emitter-cap rule.

Particle count does not multiply the receipt amount.
Separate struck GameObjects can each receive a callback from one emitter step.
Receipts read live source fractions after spill debit and source cleanup.
A receipt adds fluid without another source debit.

A receiver with space and an open valve changes the emitter collision response.
Bounce and dampening interpolate from the receiver defaults to zero through its valve value.
The emitter retains two response values and a fixed five-update countdown.
Five cap updates decrement the countdown. The sixth update restores the emitter defaults.
A closed or full receiver restores the emitter defaults immediately without clearing the countdown.
Old particles can change this response when the current raw poured amount is zero.
Fluid additions and fill flags still require a positive raw request.
Particle motion uses the current emitter response.
These checks establish the local update sequence, not Unity callback timing.

The implementation uses a stable local callback order based on existing source identities.
The decompiled methods do not establish Unity callback timing or ordering.
That ordering remains an explicit runtime acceptance limit.

Contact storage has a fixed capacity derived from installed openings and particle limits.
Construction allocates that storage once.
Steady spill and receipt handling add no per-frame allocation or repeated setup.
Current garage tanks and openings use imported registration.
Generated particle registration remains outside this task.

## Export, save, and restore

Serialized fluid state uses ordered entries with fluid type and exact quantity.
The same contract applies to core item saves and existing world object snapshots.
Core-save and reference lists use `Vec<FluidEntry>` at cold boundaries.
In-memory world snapshots use a fixed `Copy` value to avoid per-frame allocations.
Their serialized entry lists remain unchanged.
Existing generated constructors and restore paths must use that contract.

Input validation rejects unknown types, duplicate types, nonfinite quantities, and invalid capacity.
Initial garage export validation also requires nonnegative quantities within tank capacity.
Finite signed quantities remain valid source state.
Save validation must not reject source state solely because positive quantities exceed capacity after ordered mutation.
Capture must not clamp quantities, rescale totals, or change gameplay state.
Nonfinite capture returns a clear save error.

Restore preserves entry order and exact finite quantities before the source load update.
The source load update removes at most one nonpositive entry.
Repeated restore passes assign complete state without repeating that logical load update.
One explicit load cleanup phase precedes final fluid mass synchronization.
Tests distinguish this source cleanup from an exact snapshot round trip.

The save schema version increases for the new required representation.
Obsolete save and manifest representations are rejected.
There is no migration, inferred order, optional legacy field, or fallback decoder.
Changed export fields require matching export validation and installed assets for integration checks.
Existing source stamps continue to reject mismatched assets and saves.

## Part ownership and mass

Engine removal keeps the same oil tank item and fluid state.
Radiator removal keeps the same coolant tank item and fluid state.
Cap poses follow the removed part.
Removal and installation do not initialize fluid contents.

After transfer or spill, existing dirty-state and mass updates use the changed item contents.
Tests check both detached and installed states.
Portable mass bounds the combined dry mass and derived fluid total to 0.001 kg.
This uses the existing detached-part boundary. It preserves authoritative entries and saved quantity bits.
Part identities, collider ownership, and cap ownership remain stable through repeated cycles.

## File boundaries

| Area | Expected files | Responsibility |
| --- | --- | --- |
| Fluid model | `src/fluids.rs` | Ordered mutations, state capture, and restore input. |
| Reference data | `src/reference.rs`, `src/reference/`, `tools/reference/manifest.py` | Preserve and validate source entry order. |
| Interaction | `src/interaction.rs`, `src/interaction/feedback.rs` | Direct fill gates, spill steps, and receipt application. |
| Particle contacts | `src/interaction/fluid_contact.rs` | Particle motion and collision records. |
| Item lifecycle | `src/interaction/items.rs`, `src/interaction/parts.rs` | Fluid ownership, dirty state, mass, and detached poses. |
| Persistence | `src/save/`, `src/starting_area/save.rs`, `src/world/changes.rs`, existing generated fluid constructors | One ordered persistence contract. |
| Checks | `tests/fluids.rs`, garage fluid integration tests, export tests | Critical source and lifecycle cases. |

The implementation plan assigns separate file ownership to subagents.
Integration and shared-file changes have one writer.

## Verification and completion

Critical native checks cover ordered mixed transfer and spill with fixed source expectations.
They cover near-empty states, capacity limits, retained entry order, and one-entry cleanup.
They also cover nonfinite save rejection without fluid mutation.

Garage checks use the installed single-Lada export.
They cover cap toggle and tuning, direct fill direction, empty supplies, and closed or full receivers.
Particle checks cover tank receipts, contact radius, callback grouping, and no second debit.
Part checks cover filled engine and radiator removal, detached operation, installation, mass, and ownership.
Save checks cover ordered mixtures and the source load cleanup rule.
A load case retains several nonpositive entries and checks detached tank mass after one cleanup.

Native tests and WASM compile checks run for Rust changes.
Targeted Python tests and export validation run for exporter changes.
Targeted web tests run only if web behavior changes.
Full checks, release builds, and browser acceptance require a separate request or agreed milestone.

Code completion requires passing targeted checks and an inspected final diff.
G04 remains `Verify` until normal-input browser acceptance is recorded.
No native test, compile result, or scripted route proves visible runtime behavior.
The final report lists checks, failures, deferred checks, and runtime limits.
