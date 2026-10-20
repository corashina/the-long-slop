# G07 object handling implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task by task. Track steps with checkboxes.

**Goal:** Complete source-faithful object handling for the garage Lada experience defined by G07.

**Architecture:** `InteractionState` selects source action priority and input ownership. Carry and push retain separate physics owners. One temporary mass record preserves authored mass during carry.

**Tech stack:** Rust 2024, Rapier 0.36.0, existing JavaScript input, Node tests, PowerShell checks, and the existing Docker toolchain.

**Spec:** [Approved G07 design](../specs/2026-10-20-g07-object-handling-design.md), commit `42eee50`, approved on 2026-10-20.

## Global constraints

- Work in `C:\Users\Corashina\.codex\worktrees\g07-object-handling\the-long-drive-wasm` on `codex/g07-object-handling`.
- Base product code is `cc7540b255d683722c67e09103a66bfa789c0fe7`.
- Build `11140890` defines behavior. Authored pickup and push records define object-specific values.
- The selection and result use fixed-size values without per-frame allocation.
- G08 slot selection, inventory swapping, and broader equipment work remain deferred.
- Driving physics, general cargo, clothing, survival, weapons, and renderer changes remain outside G07.
- G07 adds no pivot metadata or screen-edge camera system for unused variants.
- The branch remains unmerged unless the user authorizes integration.
- Use GPT-6.1 Sol with high reasoning for implementation and review. Never use GPT-6 Astra.
- Give each task one writer. Run tasks in order because they share action and physics owners.
- Use subagents for implementation and independent task review. The controller owns integration and final checks.
- Add only critical tests. Remove obsolete paths after migrating callers.
- Use the existing dependencies. Add no package or save-schema change.
- Full checks, release builds, and browser acceptance require separate authorization or an agreed milestone.

## Review focus

Each condition has a regression in the named task.

1. A press and release within one render frame must not drop a newly acquired object. Tasks 1 and 3.
2. A carried container can lose fluid before release. Physical mass stays 1; released mass uses current contents. Task 4.
3. A `noRot` container can survive save and load, then rotate manually without inheriting head pitch. Tasks 3 and 4.
4. Capture loss during F, Tab, or grip must not trigger a delayed action after resume. Tasks 2, 5, and 6.
5. An invalid slot and an exact throw threshold must leave a valid release path. Tasks 2 and 3.

## File responsibilities and interfaces

| File | Responsibility |
| --- | --- |
| `web/input.js`, `tests/web/input.test.mjs` | Correct raw button mapping and preserve input edges and reset behavior. |
| New `src/interaction/input.rs` | Select input ownership and retain cancellation state without physics mutations. |
| `src/interaction.rs`, `feedback.rs`, `targets.rs` | Apply source priority, transfer ownership, and report available actions. |
| `src/starting_area.rs` | Pass the resolved look delta to the player and use the common mass boundary. |
| `src/interaction/carry.rs` | Carry gestures, pose, temporary body state, release, and restore. |
| New `src/vehicle/mass.rs`, `src/vehicle.rs`, `mutations.rs` | Preserve authored mass while one body has a physical mass override. |
| `src/interaction/parts.rs`, `wheels.rs` | Publish authored assembly mass through that boundary. |
| New `src/interaction/hand_controls.rs`, `equipment.rs` | Resolve the shared Tab gesture through the existing equipment owner. |
| `src/interaction/push.rs` | Ordinary push and persistent grip. |
| New `tests/garage_handling.rs` | Source-backed G07 input, physics, ownership, and restore regressions. |
| `scripts/check.ps1` | Run the selected native tests with ignored export-backed cases enabled. |
| `tests/garage_parts.rs`, `wheel_input.rs`, `inventory.rs` | Preserve existing installation and inventory boundaries. |
| `docs/roadmap.md` | Record code coverage and current verification limits. |

### Task 1: Correct raw carry controls

**Files:** Modify `web/input.js:1` and `tests/web/input.test.mjs`.

**Interfaces:** Keep `InputState.code`, `takeFrame`, and `clear` unchanged. Keep the existing `FrameInput` bit values.

- [ ] Add `source_carry_buttons_and_frame_edges` with these assertions:

```javascript
const input = new InputState([]);
input.code('Mouse1', true);
assert.equal(input.takeFrame().held, 16384);
input.code('Mouse2', true);
assert.equal(input.takeFrame().held, 16384 | 4096);
input.clear();
input.code('KeyF', true);
input.code('KeyF', false);
const frame = input.takeFrame();
assert.equal(frame.held, 0);
assert.equal(frame.pressed, 256);
assert.equal(frame.released, 256);
```

- [ ] Run `node --test tests/web/input.test.mjs`. Confirm the mapping assertion fails on the baseline.
- [ ] Map `Mouse1` to TRANSLATE `16384` and `Mouse2` to ROTATE `4096` in `web/input.js`.
- [ ] Extend the test to assert that combined-button release preserves the remaining held button. Assert `clear()` discards edges and deltas.
- [ ] Run `.\scripts\check.ps1 -WebTest input`. Require all tests to pass.
- [ ] Commit only these files with `fix: use source carry mouse controls`.

### Task 2: Make action priority and input ownership explicit

**Files:** Create `src/interaction/input.rs` and `tests/garage_handling.rs`. Modify `src/interaction.rs:1173`, `feedback.rs:111`, `targets.rs:538`, `src/starting_area.rs:964`, `scripts/check.ps1`, and existing direct-step callers in `tests/garage_parts.rs` and `tests/wheel_input.rs`.

**Interfaces produced:**

```rust
pub struct InteractionOutcome {
    pub action: Option<InteractionAction>,
    pub look_mouse: [f32; 2],
}
```

Re-export this type from `interaction.rs`. Derive `Clone`, `Copy`, `Debug`, and `Default`.
`InteractionState::step` retains every current parameter and returns `InteractionOutcome` instead of `Option<InteractionAction>`.
The private `ResolvedInput` has `use_input`, `grab_input`, `carry_input`, and `push_input` fields of type `FrameInput`.
It also has `look_mouse: [f32; 2]`.
`InteractionState::filter_cancelled_input(&mut self, input: FrameInput) -> FrameInput` applies cancellation once per step.
`InteractionState::resolve_input(&self, input: FrameInput, available: &feedback::ActionAvailability) -> ResolvedInput` selects owners without mutations.
Both methods live in `input.rs`.

- [ ] Provision only the installed manifest before native tests. Create the ignored destination directory, then copy this file:

```powershell
Copy-Item -LiteralPath 'C:\Users\Corashina\Documents\Projects\the-long-drive-wasm\web\assets\reference\manifest.json' -Destination 'web\assets\reference\manifest.json'
```

Compare both SHA256 hashes with `Get-FileHash`. Require equality and manifest build `11140890`.
Do not regenerate assets, copy the source cache, or add the export to Git.

- [ ] Add `-IncludeIgnored` to the `NativeTarget` and `NativeFilter` parameter sets in `scripts/check.ps1`.
Append `-- --include-ignored` after the selected Cargo filter. Keep `-Full` and web parameter sets unchanged.
Run the existing wheel and part suites with this switch. Record baseline failures before changing gameplay.

- [ ] Add source-backed `action_priority_and_input_ownership` in `tests/garage_handling.rs`.
Use `ReferenceManifest::from_runtime_json` and `GarageRuntime::new`.
Keep fixture helpers in this test file:
`source_scene() -> (ReferenceManifest, GarageRuntime)` and `step_input(&mut GarageRuntime, FrameInput, ResolvedAim) -> InteractionOutcome`.
Find targets through source names and bindings. Reuse the six-direction ray method in `tests/wheel_input.rs`.
Assert active drag wins, a later F action cannot overwrite a claimed action, and ordinary pickup accepts source-held F.
Assert the resulting `look_mouse` is zero for an accepted drag and unchanged for an unclaimed look delta.

```rust
assert_eq!(outcome.action, Some(InteractionAction::Mechanism(door)));
assert_eq!(outcome.look_mouse, [0.0; 2]);
assert!(run.area.interaction.held().is_none());
```

Assert invalid installation permits a later drop. Assert real ray resolution keeps separate action passes.

- [ ] Run `.\scripts\check.ps1 -Test garage_handling -Filter action_priority_and_input_ownership -IncludeIgnored`.
The initial failure is the missing `InteractionOutcome` contract.
After introducing the result type, confirm a priority assertion fails before fixing its handler.
A missing export or skipped test is not the intended failure.

- [ ] Implement the ownership resolver and migrate all direct-step callers in the same change.
Resolve attachment and part actions, use actions, sitting, free pickup, and push in source order.
An active usable drag retains priority. Preserve separately permitted channels.
For wheel distance, tuning or look blocks carry unless RMB is held. MMB always blocks carry wheel input.
Keep this RMB exception explicit rather than applying a blanket one-consumer rule.
Remove `blocked_install` as a carry-release veto while preserving fit rejection and notices.
Use the same eligibility rules for `available_actions` and dispatch.
Replace the mouse conditions near `starting_area.rs:1095` with `walking_input.mouse = outcome.look_mouse`.
Remove obsolete mouse-ownership fields when their last caller disappears.

- [ ] Retain cancellation suppression in the input owner. F, Tab, RMB, and MMB must reach release or neutral state before reuse.
Add a reset sequence to the test. Assert no delayed pickup, release, or handling after reset.
Continuous carry constraints still update with mouse deltas removed.

- [ ] Run the focused G07 test, then `wheel_input` and `garage_parts`, each with `-IncludeIgnored`.
Require named tests to execute and pass. The native script also checks format and both WASM backends.
- [ ] Commit this task with `fix: resolve source interaction priority and input ownership`.

### Task 3: Complete carry gestures and pose

**Files:** Modify `src/interaction/carry.rs:137-190,326-477`, `src/interaction.rs`, `input.rs`, `feedback.rs:449`, `targets.rs`, and `tests/garage_handling.rs`.

**Interfaces:** Keep `CarryState::{begin_throw, charge_throw, release_throw, acquisition_held, release_acquisition}`.
Keep the current `CarryState::step` parameters until Task 5 adds an explicit recenter request.
`InteractionOutcome` remains the result consumed by runtime and tests.

- [ ] Add `carry_release_and_pose_follow_source` to the G07 suite.
Assert acquisition release retains identity, a later F gesture drops, and invalid installation permits both drop and throw.
Assert combined F-down and F-up in one frame cannot drop the acquisition.
Assert charge rises to its maximum, falls, disarms release at or below minimum, and retains the object on that release.
Assert a later gesture rearms release. Verify throw adds velocity rather than replacing existing velocity.
For the canceled falling-cycle release, assert:

```rust
assert_eq!(outcome.action, None);
assert_eq!(run.area.interaction.held(), Some(item));
```

Assert forced-distance drop at the authored `carry_drop_distance`, repeated pickup cycles, and collision constraints.

- [ ] Add a private export-backed regression in `carry.rs` for the exact charge boundary.
After normal acquisition and `begin_throw`, set the private `throw_charge` to the authored minimum.
Assert interaction release reports `Drop`, produces no throw impulse, and leaves the object in `ItemLocation::World`.
Assert the same charge does not advertise MOUNT. Test just below and above the boundary too.
Use `#[ignore]` for this installed-export test. Do not add public test setters.

- [ ] Run the new G07 test and `.\scripts\check.ps1 -Filter interaction::carry::tests -IncludeIgnored`.
Confirm the falling-cycle and pose assertions expose the baseline gaps.

- [ ] Correct release arming and strict comparisons in the existing carry owner.
Mount requires charge `< minimum`. Throw requires `> minimum`. Equality drops only when release is armed.
A falling-cycle cancellation retains the object and differs from input-reset suppression.
Correct MMB yaw, pitch, and combined roll signs against the decompiled source and coordinate conversion.
Preserve authored distance sensitivity, offset radius, and physical collision checks.
Keep the existing recenter offset formula. Do not add another `+1`.

- [ ] Represent `noRot` rotation relative to `PlayerState::body_pose`, while ordinary rotation uses the view frame.
Manual MMB rotation remains available for `noRot` objects.
Update acquisition, pose, restore interpretation, and ROTATE feedback consistently.
Test OilCan `120`: changing head pitch preserves root-relative rotation, root yaw follows, and MMB still changes rotation.
Assert RMB moves, MMB beats translation, and MMB plus RMB rolls.
Test wheel blocking, RMB override, and combined RMB plus MMB blocking with real tuning targets.

- [ ] Run both new native test groups. Require passing assertions and both WASM checks.
- [ ] Commit this task with `fix: complete source carry gestures and pose`.

### Task 4: Preserve authored mass through carry and restore

**Files:** Create `src/vehicle/mass.rs`. Modify `src/vehicle.rs:94-135`, `src/vehicle/mutations.rs:109`, `src/interaction/carry.rs`, `src/interaction/parts.rs:1205`, `src/interaction/wheels.rs:1025`, `src/starting_area.rs:436`, `src/starting_area/save.rs`, `src/vehicle/query_bounds/tests.rs:194`, and `tests/garage_handling.rs`.

**Interfaces produced by `PhysicsWorld`:**

```rust
pub fn authored_body_mass(&self, body: RigidBodyHandle) -> f32;
pub fn set_body_mass(&mut self, body: RigidBodyHandle, mass_kg: f32);
pub fn set_body_mass_override(&mut self, body: RigidBodyHandle, mass_kg: Option<f32>);
```

Store one `Option<BodyMassOverride>` in `PhysicsWorld`.
The record contains `body: RigidBodyHandle`, `authored_mass_kg: f32`, and `physical_mass_kg: f32`.
This matches the existing invariant of one carried body.
`HeldState` retains the previous gravity scale for release. No save fields are added.

- [ ] Add `carry_mass_save_and_assembly_lifecycle` using real scene bodies.
Assert pickup gives physical mass `1.0` and gravity scale `0.0`.
Assert `authored_body_mass` retains the pre-pickup value.

```rust
assert_eq!(run.physics.rapier().bodies[body].mass(), 1.0);
assert_eq!(run.physics.rapier().bodies[body].gravity_scale(), 0.0);
assert!((run.physics.authored_body_mass(body) - authored_mass).abs() < 0.0001);
```

Change fluid contents, then run portable, part, and wheel mass synchronization paths while carrying their supported bodies.
Assert physical mass remains `1.0`; drop restores current authored mass and prior gravity before any throw impulse.
Assert identities, fluids, conditions, collider parents, and assembly ownership survive three handling cycles.

- [ ] Run the focused native test. The initial failure is the missing mass API. Once it compiles, confirm the temporary-mass or gravity behavior before finalizing lifecycle.

- [ ] Implement the three mass methods in `mass.rs` and initialize the optional record in all `PhysicsWorld` constructors.
`Some(1.0)` captures authored mass and applies the override.
`set_body_mass` updates authored mass while an override remains active.
`None` restores the latest authored mass immediately.
Use existing Rapier mass APIs and recompute properties before callers apply impulses.
Selected portable colliders have zero density. Preserve their collider mass contract; do not rewrite vehicle inertia handling.
Route all three portable or assembly mass writers through this method and remove duplicate recomputation there.
`remove_body` clears a matching override.

- [ ] Apply the same override on pickup and carry restoration. Clear it before every carry release and equip transfer.
Read `authored_body_mass` for carry movement and rotation gains.
Store and restore gravity through the carry owner. Remove the plate-only gravity special case.
Keep setters change-driven. Do not add a per-frame mass rewrite or a body-mass map.

- [ ] Extend the test through `GarageRuntime::capture_save` and `GarageRuntime::from_save` using the existing `SourceStamp` pattern.
Use `capture_save(&reference, &stamp, CameraSave::FirstPerson, "2026-10-20T12:00:00Z") -> Result<SaveSnapshot, SaveError>`.
Use `GarageRuntime::from_save(&reference, &stamp, &saved) -> Result<GarageRuntime, SaveError>`.
Restore a held OilCan with changed contents and root-relative rotation.
Assert one carry joint, physical mass `1.0`, gravity `0.0`, preserved pose, and no armed transient throw.
Drop after restore. Assert restored authored mass and no duplicate collider ownership.
Publish another authored mass update after restoration, then assert the physical override remains active.
This tests the later-publication order without adding generated items to the garage scenario.

- [ ] Run the G07 mass test, `garage_parts`, `wheel_input`, and `inventory`, each with `-IncludeIgnored`.
Require format and both WASM checks to pass.
- [ ] Commit this task with `fix: preserve authored mass while carrying objects`.

### Task 5: Give Tab one gesture owner

**Files:** Create `src/interaction/hand_controls.rs`. Modify `src/interaction.rs`, `carry.rs:326`, `equipment.rs`, `input.rs`, `feedback.rs`, `targets.rs`, and `tests/garage_handling.rs`.

**Interfaces:** `InteractionState::step_hand_controls(&mut self, input: FrameInput, player: &PlayerState, items: &mut ItemStore, physics: &mut PhysicsWorld, dt: f32) -> bool` returns a recenter request.
Change `CarryState::step` to accept `recenter: bool` after its `input` parameter. Migrate every caller.
Read the existing `reference.inventory.equip_hold_seconds`, exactly `0.25`.
Use `EquipmentState::{item, retrieve, release, sync}`. Do not change `InventoryCommand`.

- [ ] Add `tab_recenter_has_one_owner` with short, exact-threshold, long, non-equipable, occupied-hand, and canceled gestures.
Use carried flashlight `127` for equipable cases. Its authored recenter distance is `-0.5`.
OilCan `120` is non-equipable and has recenter distance `0`. Use it for immediate root-relative recenter checks.
Assert release before `0.25` equips only when the hand is free.
Assert hold beyond `0.25` recenters. At exactly `0.25`, neither strict threshold fires.
A non-equipable object recenters immediately while Tab is held.
With flashlight `126` equipped and `127` carried, short Tab preserves both owners. Long Tab recenters without swapping.

```rust
assert_eq!(run.area.interaction.equipment.item(), Some(ItemId(126)));
assert_eq!(run.area.interaction.held(), Some(ItemId(127)));
assert_eq!(run.area.inventory.slots, slots_before);
```

- [ ] Run the focused native test. Confirm immediate recenter or missing equip behavior fails.

- [ ] Implement a private fixed-size Tab gesture state in `hand_controls.rs`.
Track idle, pressed elapsed time and equip eligibility, and suppression until release.
Process hand controls after `filter_cancelled_input`, before final carry and look selection.
Recompute availability after a hand transfer, then call `resolve_input` with the already filtered frame.
Bind timing to the carried item identity. A change of carried item cannot complete an older gesture.
Keep source threshold comparisons strict and clear the timer on cancellation or ownership change.
Pass the returned recenter request into carry physics. Remove raw Tab interpretation from `CarryState::step`.

- [ ] Transfer carry to equipment by releasing carry state before `EquipmentState::retrieve`.
Transfer equipment to carry through `EquipmentState::release` and the existing carry acquisition path.
Only equipped-item Tab-down on foot, or held MMB, permits that transfer. Tab-down while seated retains equipment.
Suppress equip for the transfer gesture. Preserve inventory membership through existing `sync_inventory`.
Do not add selection, swapping, appearance, or broader equipment behavior.

- [ ] Extend the regression with equipped-to-carry, seated Tab, reset during Tab, and an item change during Tab.
Assert no duplicate owner, slot change, delayed equip, stale recenter, or mass override leak.
Run the G07 Tab test and the existing `inventory` suite with `-IncludeIgnored`.
Require both WASM checks to pass.
- [ ] Commit this task with `fix: share source Tab timing between recenter and equipment`.

### Task 6: Complete push continuity and verify G07 integration

**Files:** Modify `src/interaction/push.rs`, `input.rs`, `src/interaction.rs`, `feedback.rs`, `targets.rs`, `tests/garage_handling.rs`, and `docs/roadmap.md`.

**Interfaces:** Keep `PushState::step`, `cancel_input`, `finish_cancelled_input`, `stop`, `steering`, and `gripping`.
Consume only `ResolvedInput.push_input`. Preserve the `InteractionOutcome` contract.

- [ ] Add `push_force_grip_and_cancellation` against Lada push owner `27` and source grip collider `398`.
Resolve those identities through actual source bindings, then assert the expected records.
Assert use, pickup, and installation reservations block push as the source requires.
Assert held F can begin ordinary push after the aim changes to a supported pushable target.
Assert ordinary push uses the current hit point, supported projection, authored force, mass, and airborne player recoil.
Assert same-support-body rejection, with the authored override case tested separately.

- [ ] Run the focused test. Confirm held-key target continuity or another demonstrated source gap fails.
Do not modify already-correct push force calculations without a failing source-backed assertion.

- [ ] Fix ordinary push target handling so an extra F-down is not required after a valid held-key aim change.
Keep grip activation edge-triggered. A later F-down releases it once.
Retain grip when source gates permit looking away. End grip on range loss, invalid body, or invalid collider.
Route cancellation through the existing physics owner and remove the grip joint before resume.
Require a fresh gesture after cancellation. Keep source support-body and grounded checks.

- [ ] Extend the regression with WASD steering, repeated grip cycles, out-of-range release, and reset during grip.
Assert joint count returns to baseline after each release.

```rust
assert!(!run.area.interaction.push_gripping());
assert!(run.area.interaction.push_steering().is_none());
assert_eq!(run.physics.rapier().impulse_joints.len(), baseline_joints);
```

Assert no delayed push, stale steering, or duplicate joint after resume.
Run a combined native scenario through `GarageRuntime::step`: acquire, handle, release, push, save, restore, and handle again.
Keep every object in the selected garage sample.

- [ ] Run the final targeted matrix once after the last behavior change:

```powershell
.\scripts\check.ps1 -WebTest input,menu
.\scripts\check.ps1 -Test garage_handling -IncludeIgnored
.\scripts\check.ps1 -Filter interaction::carry::tests -IncludeIgnored
.\scripts\check.ps1 -Test garage_parts -IncludeIgnored
.\scripts\check.ps1 -Test wheel_input -IncludeIgnored
.\scripts\check.ps1 -Test inventory -IncludeIgnored
```

Each native command checks format, the selected tests, WebGL2 WASM, and WebGPU WASM.
All named installed-export tests must execute. Report any pre-existing failure separately with its baseline evidence.
Do not repeat passed commands unless another change or unresolved failure requires it.

- [ ] Update only the G07 row and its verification note in `docs/roadmap.md`.
Record passing check names and unresolved limits. Use `Verify` only when known G07 code gaps are closed.
Keep browser acceptance open. Do not mark G07 accepted from native input or compile evidence.
- [ ] Run `git diff --check`, review changed callers, and obtain the final independent branch review.
- [ ] Commit this task with `fix: complete source push handling and G07 integration`.

## Specification coverage

| Requirement | Task |
| --- | --- |
| Raw controls and frame edges | 1 |
| Ordered action passes, feedback, input ownership, and reset suppression | 2 |
| Pickup, drop, throw, rotation, distance, collision, and strict thresholds | 3 |
| Authored mass, gravity, fluids, assemblies, identity, and save restoration | 4 |
| Recenter timing, occupied hand, and equipment transfer boundary | 5 |
| Push, grip, steering, recoil, cancellation, and complete targeted verification | 6 |
| Browser acceptance limits and unmerged handoff | 6 and execution handoff |

## Execution and handoff

The user selected worktree and subagents. Keep that execution method after plan approval.
Use one implementing subagent and an independent reviewer for each task.
Do not parallelize edits to the shared interaction or physics files.
Each task ends with a usable code layer, passing targeted checks, and a commit.
A baseline failure does not authorize unrelated repairs. Record its command, failure, and effect on G07 verification.
Stop dependency work only when the failure prevents meaningful progress.

Report the final branch, commits, changed behavior, checks, failures, and deferred checks.
Browser input, active scene physics, submitted frames, and visible G07 results remain unverified until browser acceptance runs.
Leave full checks, release builds, browser acceptance, merge, and pull-request creation pending authorization.

## Plan-stage verification

This plan uses read-only source and interface research from two subagents.
The controller self-reviews spec coverage, signatures, task scope, and the five review-focus cases.
Only documentation changes during this stage.
No gameplay tests, native tests, WASM checks, release builds, full checks, or browser acceptance run during planning.
