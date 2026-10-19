# Wheel action ownership implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete wheel actions through one interaction method without changing current behavior.

**Architecture:** Keep action selection in `InteractionState::step` and execution after that step returns. The interaction executor owns carry, assembly calls, sensors, pickup, notices, and completion feedback. `GarageRuntime` keeps `WheelAssemblies` ownership.

**Tech Stack:** Rust 2024, Rapier 0.36.0, existing Bevy 0.19.1 integration, native unit tests, and WASM compilation.

**Spec:** [Approved wheel action specification](../specs/2026-10-19-wheel-actions-design.md).

Status: Complete. Independent reviews passed. Browser acceptance remains deferred.
Worktree: `C:/Users/Corashina/.codex/worktrees/codebase-architecture/the-long-drive-wasm`.
Branch: `codex/codebase-architecture`.
Starting commit: `3157f5a`.
Execution method: Subagents, as requested by the user.

## Global constraints

- Execution must remain after the complete interaction step.
- Use the original frame input and resolved aim.
- `GarageRuntime` continues to own `WheelAssemblies`.
- Delete the obsolete variants. Add no conversion or compatibility path.
- Keep the existing completion queue, capacity, order, and drain point.
- The executor adds no per-frame allocation or repeated setup.
- Do not add a production test interface or restore a full manifest fixture.
- Steam build `11140890` and its decompiled behavior remain authoritative.
- The branch remains unmerged until authorized.

No dependency, save schema, browser, UI text, physics rule, or action-priority changes are included.
Full checks, release builds, and browser acceptance remain deferred.

## Review focus

Each condition has a named test in Step 1.

1. Removal targets an attached tire. Completion must identify the returned rim.
2. Removal has no slot. The existing notice must remain unchanged.
3. Removal succeeds but pickup fails. The mutation and completion event must remain.
4. Split receives different input and aim combinations. Pickup must use the original GRAB press and point priority.
5. Carry fails after split selection. The final Drop action must prevent wheel execution.

## File map

| File | Change |
| --- | --- |
| `src/interaction/wheel_actions.rs` | New private module. Define `WheelAction` and the executor. |
| `src/interaction/wheel_actions/tests.rs` | Critical native state and feedback tests. |
| `src/interaction/wheel_actions/fixture.rs` | Small test-only combined fixture. |
| `src/interaction.rs` | Register the module, re-export the enum, replace variants, and migrate five selection assignments. |
| `src/starting_area.rs` | Replace four wheel execution branches with one executor call. |
| `src/audio.rs` | Migrate wheel completion matching without changing emitted item identities. |
| `src/interaction/wheels.rs` | Test-only assembly constructor. No assembly algorithm changes. |
| `src/interaction/items.rs` | Test-only item store constructor. |
| `src/interaction/carry.rs` | Test-only carry constructor using player and pickup records. |
| `src/interaction/push.rs` | Test-only push constructor using a player record. |
| `src/interaction/cleaning.rs` | Empty test-only cleaning state. |
| `src/interaction/storage.rs` | Empty test-only storage state. |
| `src/interaction/parts.rs` | Empty test-only part attachment state. |
| `src/interaction/attachments.rs` | Empty test-only plate attachment state. |
| `src/interaction/mechanisms.rs` | Empty test-only mechanism state. |
| `src/interaction/equipment.rs` | Empty test-only equipment state with an explicit `NodeId`. |
| `src/vehicle.rs` | Test-only real chassis and wheel adapter constructor. |

Test setup touches eleven existing owner files. Every constructor stays under `#[cfg(test)]`.
Keep those helpers beside their owners. Do not add general defaults or widen production visibility.
The combined fixture initializes `InteractionState` through its existing parent-module privacy rules.
Use `PlayerState::new` for the player.

## Task 1: Complete wheel action ownership

The enum migration, executor, garage dispatch, and audio migration form one atomic task.
A separate enum-only task would add an intermediate interface without completing the ownership change.
Fixture setup belongs to this task because its critical tests require that setup.

**Interfaces consumed:**

- `InteractionState::step(...) -> Option<InteractionAction>`.
- `InteractionState::drop_held`, `pickup_item`, `completed_action`, and `bind_wheel_sensor`.
- Existing `WheelAssemblies` fit, remove, install, split, assemble, and sensor methods.
- Existing `PhysicsWorld` mutation methods and `PlayerState::new`.

**Interfaces produced:**

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelAction {
    Remove(ItemId),
    Install { item: ItemId, target: WheelSlotId },
    Split(ItemId),
    Join { rim: ItemId, tire: ItemId },
}

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

`InteractionAction` exposes one `Wheel(WheelAction)` variant.
`None` preserves the notice. `Some("")` clears it. Other values replace it.
The notice result does not encode operation success.

- [x] **Step 1: Write the critical failing tests and their small fixture.**

Register the private `wheel_actions` module from `src/interaction.rs` for the failing-test run.
Register `tests.rs` and `fixture.rs` under that module with `#[cfg(test)]`.
All test names include `wheel_actions`.
Use real pickup, drop, assembly, collider binding, and feedback methods.
Do not mock the operation sequence.

| Test name | Assertions |
| --- | --- |
| `wheel_actions_invalid_install_retains_carry` | The notice is `This wheel does not fit the slot.` Carry and its joint remain. No completion is queued. |
| `wheel_actions_install_failure_after_release_keeps_drop` | Fit passes, but real binding fails. Carry remains released. The install error notice appears without completion. |
| `wheel_actions_install_rebinds_sensor` | Install a compatible spare rim into the original rim's empty slot. Sensor ownership changes to the spare. Identities and expected mass remain correct. |
| `wheel_actions_remove_tire_keeps_completion_after_pickup_failure` | Target the attached tire. The returned rim becomes loose. Pickup failure returns `Cannot carry this wheel.` One removal event names the rim. |
| `wheel_actions_remove_without_slot_preserves_notice` | The executor returns `None`. Wheel state and the completion queue remain unchanged. |
| `wheel_actions_failed_split_and_join_keep_drop` | Test each rejected operation with a fresh held fixture. Carry is released. The exact spec notice appears. No completion is queued. |
| `wheel_actions_split_and_join_preserve_state` | Check identities, collider parents, body ownership, expected mass, and motion. Split emits `PartRemove(tire)`. Join completion names the rim and tire. Join adds no pickup. |
| `wheel_actions_split_uses_original_input_and_aim` | No GRAB press means no pickup. A GRAB press uses focus before grab, then the tire pose. Failed pickup keeps the split and its event. |
| `wheel_actions_late_drop_replaces_split` | Real `step` returns Drop after carry-distance failure. Tire attachment remains intact. No split completion is queued. |

The fixture uses a chassis, a tested slot sensor, a rim, a tire, and a spare rim.
Its vehicle adapter needs four distinct wheel references, with only the tested slot active.
Use finite positive masses and valid pickup records. No values change production behavior.

Give wheel colliders source IDs through `user_data`.
Leave assembly binding pending so successful cases exercise the real binding path.
Keep item states, indices, body IDs, and names aligned.
Read existing private state from child tests instead of adding production getters.

Use `pub(super)` for test helpers in interaction owners. Use `pub(crate)` for the physics test helper.
The owner-local helper contracts are:

- `PhysicsWorld::wheel_actions_fixture(slot: WheelSlotReference, base_mass: f32) -> Self`.
- `ItemStore::wheel_actions_fixture(states: Vec<ItemState>) -> Self`.
- `WheelAssemblies::wheel_actions_fixture(rim: ItemId, tire: ItemId, spare: ItemId, slot: WheelSlotId, spare_fits: bool) -> Self`.
- `CarryState::wheel_actions_fixture(player: PlayerReference, rules: &[(ItemId, PickupReference)]) -> Self`.
- `PushState::wheel_actions_fixture(player: PlayerReference) -> Self`.
- `empty_for_wheel_actions() -> Self` on each remaining inactive owner in the file map.

The combined fixture may wrap the executor to borrow its fields.
That wrapper must not select actions or duplicate executor behavior.

For late action replacement, hold an assembled loose rim through real pickup.
Move its body beyond `carry_drop_distance`, then submit an ASSEMBLE press.
Run the real interaction step. Check the returned Drop and unchanged tire attachment.
Dispatch a wheel action only if the final returned action is `InteractionAction::Wheel`.

- [x] **Step 2: Confirm the tests fail for the missing wheel action implementation.**

Run `./scripts/check.ps1 -Filter wheel_actions`.
Resolve formatting errors before treating the run as a failing behavior check.
Expected initial failure: the new wheel action type or executor is missing.
Confirm that the compiler error points into the registered wheel module or its tests.
A successful zero-test run is not an accepted initial failure.
An unavailable toolchain or invalid fixture is not the expected failure.

- [x] **Step 3: Implement the enum and executor.**

Add the exact interfaces above in `src/interaction/wheel_actions.rs`.
Keep `mod wheel_actions` and add `pub use wheel_actions::WheelAction` in `src/interaction.rs`.
Replace the four obsolete variants with the nested wheel variant.
Migrate both split selection paths, plus install, join, and removal selection.
Keep every selection condition and its ordering intact.

Move the operation sequences from `src/starting_area.rs` into the executor.
Preserve every notice string, early return, pickup point, completion item, and sensor condition from the specification.
Call `drop_held` rather than its lower-level carry method.
Keep preflight before carry release for installation.
Keep release before attempted split or join. Add no rollback.

- [x] **Step 4: Migrate runtime dispatch and audio in the same change.**

Replace the four runtime wheel branches with one nested variant branch.
Pass the original input and aim, plus existing borrowed owners.
Apply the notice only when the executor returns `Some`.
Keep dispatch after the full interaction step and before the existing later garage work.

Update audio matching for the nested variant.
Remove and install use the wheel item. Join uses the tire.
Keep split completion as `PartRemove(tire)` and keep the current audio drain point.

- [x] **Step 5: Run targeted checks and inspect the complete diff.**

Run `./scripts/check.ps1 -Filter wheel_actions`.
Expected: the nine named tests execute and pass, formatting passes, and the workspace WASM compile check passes.
Do not accept a successful command that runs zero matching tests.

Run `git diff --check`. Expected: no whitespace errors.
Run `rg -n 'WheelRemove|WheelInstall|WheelSplit|WheelJoin' src crates tests web`.
Expected: no matches. Exit code 1 means no matches for this search.
Run `rg -n 'InteractionAction::Wheel|WheelAction::|complete_wheel_action' src tests`.
Review every match and the complete branch diff.

Check that generated-wheel registration, save reconstruction, and assembly algorithms retain their existing ownership.
Check that only test code changed in the eleven fixture-support owner files.
Do not run full checks, release builds, or browser acceptance for this task.

- [x] **Step 6: Commit the complete working change.**

Stage only the files in this plan that changed.
Commit with `refactor: complete wheel actions within interaction`.
Keep the branch unmerged.

- [x] **Step 7: Complete independent reviews.**

Use one fresh subagent for specification compliance and one fresh subagent for code quality.
Reviewers must inspect the complete change and actual check output.
Correct findings in the same worktree. Repeat only checks affected by those corrections.

- [x] **Step 8: Report the section result.**

Report commits, changed ownership, executed checks, failures, and deferred checks.
State that browser behavior and source parity remain unverified.
Stop after this section. The other architecture sections need their own design cycle.

## Execution arrangement

Use GPT-6.1 Sol with high reasoning for the implementer and reviewers.
Use one implementer because the production migration and fixture state share files and interfaces.
Do not allow parallel writers to those files.
Reviewers can read independently after the complete implementation is ready.

Implementation starts after the user reviews this plan.
Keep the user's selected subagent method. Do not ask for that choice again.

## Section result

Implementation commit: `8179297c430c5d237288ebd5fee8ab3baf25151f`.
The fourteen baseline query-bound tests passed before product edits.
The expected RED compile failed on missing wheel interfaces.
All nine wheel-action tests passed after implementation.
Formatting, workspace WASM compilation, scope checks, and whole-branch whitespace checks passed.
Compiler warning messages and source files match the unchanged baseline.
Specification, code-quality, and final whole-branch reviews passed without findings.
Full checks, release builds, and browser acceptance remain deferred.
Browser behavior, audible playback, and Steam build parity remain unverified.
The branch and managed worktree remain unmerged and preserved.
