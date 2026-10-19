# Wheel action ownership

Date: 2026-10-19.
Status: Written specification awaiting user review.
Branch: `codex/codebase-architecture`.
Base: `d5b9b9835eeece5000eb8eb0c5d5c582afae8837`.

## Intent and scope

Complete wheel actions through one interaction method.
Remove wheel action sequencing from `GarageRuntime::step`.
Preserve current behavior, including failed operations and completion timing.

This section covers wheel removal, installation, tire splitting, and tire joining.
The product scope remains one complete Lada garage experience.
Steam build `11140890` and its decompiled behavior remain authoritative.
The behavior below records the inspected checkout. It does not establish source parity.
Any discovered source mismatch requires a separate behavior change.

The user approved this design in chat.
That approval permits this specification. Implementation planning follows written-spec approval.

## Current behavior

`InteractionState::step` selects an `InteractionAction`.
`GarageRuntime::step` then executes a selected wheel action.

Execution must remain after the complete interaction step.
ASSEMBLE, carry failure, or successful filling can replace an earlier selected action.
Executing a wheel operation during selection would change the final result.

The garage currently knows carry release, assembly calls, sensor binding, pickup, notices, and completion feedback.
`WheelAssemblies` already owns fit rules, collider ownership, mass, and wheel support.

## Selected design

Add `src/interaction/wheel_actions.rs`.
Keep its implementation private to interaction.
Re-export `WheelAction` from `interaction` because `InteractionAction` exposes that type.

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelAction {
    Remove(ItemId),
    Install { item: ItemId, target: WheelSlotId },
    Split(ItemId),
    Join { rim: ItemId, tire: ItemId },
}
```

Replace the four flat wheel variants with `InteractionAction::Wheel(WheelAction)`.
Migrate selection, execution, audio matching, and other callers in the same change.
Delete the obsolete variants. Add no conversion or compatibility path.

Add this method on `InteractionState` in the new module:

```rust
pub(crate) fn complete_wheel_action(
    &mut self,
    action: WheelAction,
    input: FrameInput,
    aim: ResolvedAim,
    wheels: &mut WheelAssemblies,
    player: &PlayerState,
    items: &mut ItemStore,
    physics: &mut PhysicsWorld,
) -> Option<&'static str>;
```

The signature uses existing domain types and borrowed owners.
It adds no executor object, stored context, dispatch queue, trait, or dependency.

`None` preserves the current notice.
`Some("")` clears the current notice.
`Some(message)` replaces the notice with the existing message.
These results describe notices, not operation success.

The garage handles one wheel variant at the current match position.
That branch calls `complete_wheel_action` once and applies its optional notice.
The garage retains the selected action for existing downstream decisions.
Other action handling stays at its current position.

## Ownership and flow

| Owner | Responsibility |
| --- | --- |
| `InteractionState::step` | Input arbitration and final action selection. |
| `InteractionState::complete_wheel_action` | Carry transitions, operation sequence, sensor binding, pickup, notices, completion feedback. |
| `WheelAssemblies` | Fit rules, wheel identities, colliders, mass, and wheel support. |
| `GarageRuntime` | Runtime lifetime, fixed-step order, selected-action dispatch, and notice display state. |
| `AudioState` | Existing completion-event interpretation and playback. |

`GarageRuntime` continues to own `WheelAssemblies`.
Save reconstruction and generated wheel registration retain their current ownership.

The fixed-step flow remains:

1. Resolve aim and run the complete interaction step.
2. Dispatch its final selected action.
3. Complete a wheel action through the interaction method.
4. Continue the existing garage step.
5. Drain completion feedback at the existing audio position.

Use the original frame input and resolved aim.
Do not resolve aim again after a wheel mutation.
Do not move wheel execution into an input-selection branch.

## Operation contract

### Remove

Resolve the slot from the input item.
The input can identify the rim or its attached tire.
If no slot exists, preserve the notice and emit no completion event.

Call `WheelAssemblies::remove` for the resolved slot.
On failure, return `Cannot remove this wheel.`
On success, queue removal completion for the returned rim.
Do not use the input item as the completion item.

Attempt pickup after completion is queued.
Use the original grab point when present. Otherwise, use the returned rim pose.
Successful pickup clears the notice.
Failed pickup returns `Cannot carry this wheel.`
The wheel remains removed, and its completion event remains queued.

### Install

Run the existing fit preflight before releasing carry.
Preflight rejection returns `This wheel does not fit the slot.`
Preflight rejection retains the held item.

After preflight succeeds, call the existing `drop_held` operation.
Keep its storage and cleaning side effects.
Call `WheelAssemblies::install` after carry release.
Keep the assembly method's existing checks.

Installation failure returns `This wheel does not fit the slot.`
Do not add rollback after carry release.
Success queues installation completion for the installed item.
Then bind the slot sensor when one exists.
Success clears the notice.

### Split

Call `drop_held` before the split attempt.
A failed split leaves the item dropped and returns `Cannot dismount this tire.`
A successful split queues `InteractionAction::PartRemove` for the returned tire.
It does not queue a wheel split completion event.

Attempt tire pickup only when the original frame contains a GRAB press.
Choose the pickup point from focus, then grab, then the returned tire pose.
Failed pickup returns `Cannot carry this tire.`
The split remains complete, and its completion event remains queued.
Other successful splits clear the notice.

### Join

Call `drop_held` before the join attempt.
A failed join leaves the item dropped and returns `This tire does not fit the rim.`
Success queues join completion for the original rim and tire.
Success clears the notice. Do not add automatic pickup.

## Feedback, state, and allocation limits

Keep the existing completion queue, capacity, order, and drain point.
Removal and installation audio continue to use the wheel item.
Join audio continues to use the tire.
Split audio continues through `PartRemove(tire)`.

Retain existing physics mutation methods and query refresh behavior.
Preserve each operation's current effects on identity and body ownership.
Preserve its pose, velocity, mass, and sensor behavior.
The executor adds no per-frame allocation or repeated setup.
Existing operation-specific body creation and storage updates remain allowed.

No save schema, manifest schema, browser binding, control, or UI text changes are included.
No wheel physics, compatibility rule, action priority, or failure policy changes are included.

## Alternatives

A garage wheel action module would move the code without giving interaction ownership of completion.
It would leave carry and feedback rules outside their owner.

Moving `WheelAssemblies` into `InteractionState` would change save and generated-world ownership.
That wider lifetime change is unnecessary for this section.

Executing operations during selection would remove dispatch but change same-frame arbitration.
The selected design retains post-step execution.

## Critical verification

Use small native fixtures with real Rapier bodies and the real interaction and assembly methods.
Include one chassis, one slot sensor, a rim, a tire, and an incompatible item.
Use test-only constructors where private state requires them.
Do not add a production test interface or restore a full manifest fixture.

The critical cases are:

| Case | Required result |
| --- | --- |
| Invalid installation preflight | Carry remains intact. Assembly and completion queue remain unchanged. |
| Failed split and failed join | The held item is dropped. No successful completion event is queued. |
| Removal without a slot | The current notice remains unchanged. No completion event is queued. |
| Removal through an attached tire | The rim is removed. Completion names the returned rim. |
| Removal followed by failed pickup | Removal remains complete. The notice reports pickup failure. |
| Successful installation | Existing identities and mass remain correct. The slot sensor names the installed item. |
| Successful split and join | Body ownership, velocity, mass, pickup rules, and completion items remain correct. |
| Selection replaced later in the step | Only the final action executes. A replaced wheel action changes no wheel state. |

Check state and queued events. Do not replace behavior checks with mocked call-order assertions.
Keep test names under the `wheel_actions` filter.

Run these targeted checks after implementation:

```powershell
./scripts/check.ps1 -Filter wheel_actions
git diff --check
```

The first command runs formatting, matching native unit tests, and the workspace WASM compile check.
Review every call site for removal of the obsolete wheel variants.

Full checks, release builds, and browser acceptance remain deferred.
Those checks require an explicit request or an agreed milestone.
Native checks and compilation do not prove browser behavior or source parity.

## Completion criteria

The garage loop contains no wheel-specific carry, assembly, sensor, pickup, or completion sequence.
One interaction method owns each complete wheel action.
All callers use the nested wheel action variant.
The operation contract and targeted checks pass.
The branch remains unmerged until authorized.

## Inspected evidence

Line numbers refer to the base commit named above.

- `src/interaction.rs:79-92`: Existing flat wheel action variants.
- `src/interaction.rs:1050-1068`: Completion queue and FIFO consumption.
- `src/interaction.rs:1135-1143`: Carry release side effects.
- `src/interaction.rs:1432-1489`: Wheel selection and neighboring part actions.
- `src/interaction.rs:1555-1570`: Later split selection and carry failure replacement.
- `src/interaction.rs:1639-1643`: Filling replaces the selected action.
- `src/starting_area.rs:966-982`: Interaction returns before action dispatch.
- `src/starting_area.rs:1028-1144`: Existing wheel execution and notices.
- `src/starting_area.rs:1303-1315`: Existing completion audio drain.
- `src/interaction/wheels.rs:633-643`: Rim or tire slot resolution.
- `src/interaction/wheels.rs:713-908`: Existing assembly operations.
- `src/audio.rs:541-549`: Completion item selection for audio.
- `tests/inventory.rs:10`: Installed-export test is ignored by default.
- `src/vehicle/query_bounds/tests.rs:191-200`: Small real-physics test construction.
