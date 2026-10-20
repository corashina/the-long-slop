# G07 object handling design

Date: 2026-10-20.
Status: Approved by the user on 2026-10-20.
Branch: `codex/g07-object-handling`.
Base: `cc7540b255d683722c67e09103a66bfa789c0fe7`.

## Purpose and scope

G07 completes object handling for the existing garage Lada experience.
The player can pick up, carry, drop, throw, rotate, move, recenter, and push supported objects.
Build `11140890` defines behavior. Authored pickup and push records define object-specific values.
The approved design makes `InteractionState` own action priority and input consumption.
Carry and push retain separate physics owners.

This task includes conflicts with mechanisms, caps, part installation, equipment, and player look.
It includes only the shared Tab behavior needed for recenter.
G08 slot selection, inventory swapping, and broader equipment work remain deferred.
Driving physics, general cargo, clothing, survival, weapons, and renderer changes remain outside G07.

## Evidence and current gaps

The local behavior source is `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs`.
The source cache belongs to the main checkout and is absent from a fresh worktree.
Line references below refer to that file at design time.
Raw `level1` transforms identify normal target `39812` under head `39811`.
The `noRot` target `23837` belongs to Player root `39843`.
The main export has 50 scene pickup records.
Their raw source records have null `ScrollPos` links and false `dontAffectGravity` values.
Five pickups use `noRot`: fuel cans 116, 118, and 122, OilCan 120, and barrel 124.
Their source records are `level1:74273`, `74274`, `74271`, `76005`, and `76219`.
All selected scene pickup records set `move_with_rotation` to false.

| Source lines | Behavior |
| --- | --- |
| 54996-55002, 55017-55018 | E uses. F handles pickup, installation, and push. Tab handles equip and recenter. |
| 17530-17568, 17636-17687 | Interaction gates, active usable drag, attachment scan, and part actions precede later passes. |
| 17697-17828, 17963-17985 | Usable actions precede sitting and free pickup. |
| 18181-18255 | Push runs after source use, carry, fill, sit, pickup, and mount gates. |
| 16714-16725, 18467-18477 | Mount requires charge below minimum. Throw requires charge above minimum. Equality gives a plain drop. |
| 16820-16845, 16876-16879, 18414-18425 | Acquisition release arms later drop and throw. Charge rises, falls, and can disarm release. |
| 17385-17509 | Source flags select carry targets, handling axes, wheel movement, and distance limits. |
| 48794-48814, 48858-48864 | Pickup applies temporary physical mass and gravity. Drop restores authored state. |
| 15773-15787, 16427-16444, 16942-16970, 28559 | Tab timing, recenter pose, and the shared 0.25-second threshold. |
| 16689-16706, 18211-18253, 49786-49799 | Push eligibility, grip lifecycle, push projection, force, and recoil. |

Current code reverses RMB and MMB in `web/input.js`.
`InteractionState::step` can apply a use action before an ungated F action overwrites its result.
Mechanism handling removes wheel input from carry but retains mouse input.
`GarageRuntime` separately guesses mouse ownership before player look.
Invalid installation targets currently block drop and throw.
`CarryState` recenters immediately on Tab and stops release differently after a complete throw-charge cycle.
The existing four input tests do not cover these paths.

## Ownership and data flow

`web/input.js` owns raw browser events, held buttons, input edges, and reset events.
`FrameInput` retains its current accumulation boundary.
The browser does not decide which gameplay action owns an input.

`InteractionState` resolves source eligibility and priority before each affected state change.
Ordered attachment, use, pickup, and push passes remain distinct.
A single nearest-target scan must not replace these passes.
An active usable drag retains source priority.
Lower-priority actions cannot overwrite an accepted action or reuse its consumed input.
Separate input channels can act together only where the source gates permit them.
Explicit source exceptions, including RMB wheel handling, remain visible in the selection rules.

A small interaction result carries the selected action and the remaining player-look delta.
The remaining delta contains one value per axis for source camera exceptions.
Carry, mechanisms, and player look receive only their resolved mouse and wheel input.
`GarageRuntime` passes the remaining look delta to `PlayerState`.
The separate mouse-ownership conditions in `GarageRuntime` are removed.

Action eligibility serves dispatch and feedback.
Feedback may show several available controls, but it must not advertise an action that source gates reject.
Continuous carry constraints still update when another action owns mouse input.
The selection and result use fixed-size values without per-frame allocation.
Obsolete guards and duplicated ownership decisions are removed after their callers change.

## Input and handling contract

| Input | Required behavior |
| --- | --- |
| F held with empty hands | Acquire an eligible free object under source action gates. Ordinary push uses the source held-key rule. |
| Acquisition F release | Keep the acquired object. Arm a later drop or throw gesture. |
| Later F release | Install when eligible and below minimum charge. Otherwise, drop unless a source attachment or tube action owns release. |
| F held while carrying | Charge throw power through the source rise and fall cycle. Preserve source release disarming. |
| MMB and mouse | Rotate the source-selected carry target. |
| RMB and mouse | Move the carry target. |
| MMB plus RMB and mouse | Apply source roll behavior. MMB has priority over translation. |
| Wheel | Tuning or look blocks carry distance changes unless RMB is held. MMB always blocks distance changes. |
| Tab | Use source eligibility and timing for recenter without also triggering equip. |

An invalid installation target can produce a notice but cannot trap a carried object.
Charge equal to the minimum neither mounts nor throws. It drops when release is armed.
Falling charge at or below minimum disarms release and retains the held object.
That release rearms a later gesture without dropping the object.
Drop retains existing velocity. Throw adds the source forward impulse after release.

Carry uses authored sensitivity, weight response, offset limits, release distance, and recenter pose.
The existing relative offset representation already accounts for `recenterDist + 1`.
No extra distance increment is added.
`noRot` keeps the carry rotation relative to the Player root instead of the head.
It inherits root movement and still permits manual MMB rotation.
The selected scene pickups use neither `moveWithRot` nor `ScrollPos`.
G07 adds no pivot metadata or screen-edge camera system for unused variants.
Behavior outside the selected garage sample remains unverified.

`carry.rs` owns acquisition, the physical carry constraint, pose, charge, release, and forced drop.
`push.rs` owns ordinary push, grip, steering, recoil, and grip release.
The existing Rapier constraint remains unless it cannot express a verified source requirement.
Pickup sets physical mass to 1 and disables gravity for the selected garage pickups.
Handling gains use authored mass rather than temporary physical mass.
Drop restores authored mass and gravity before applying any throw impulse.
The selected pickup records contain no gravity exception.
Authored item state remains available for release and save reconstruction.
Temporary held-body changes must not corrupt authored mass, fluids, identity, or ownership.
Mass synchronization must preserve the held override and restore current authored contents on release.

## Push and cancellation contract

Only source pushable components permit ordinary push.
Source support-body rules prevent pushing the body that supports the player unless explicitly permitted.
Ordinary push applies force at the hit point with source mass, projection, and recoil rules.
Grip requires the source grip component, grounded state, free grip, and allowed support body.
One F-down starts grip. A later F-down releases grip.
Grip ends when its source distance or body eligibility fails.

Pause, capture loss, visibility loss, and input reset clear pending gestures.
Cancellation must not cause a delayed throw, equip, pickup, or push when input resumes.
Grip joints are removed through the existing physics owner.
Save restoration clears transient gestures while preserving the saved carried object and pose.

## Critical verification

One focused native G07 suite verifies real item state and Rapier bodies through input sequences.
Pure priority tests supplement that suite only where a full scene adds no useful evidence.

Required cases cover:

- Acquisition release, held-key acquisition, later drop, forced drop, and repeated pickup cycles.
- Throw charge rise, fall, exact minimum, disarmed release, added velocity, and cancellation.
- MMB rotation, RMB movement, combined roll, recenter pose, and source handling flags.
- Wheel blocked during tuning or look, allowed with RMB, and blocked with RMB plus MMB.
- Source Tab timing, occupied-hand cases, and prevention of simultaneous recenter and equip.
- Invalid installation followed by drop or throw. Valid installation retains existing wheel and part behavior.
- Use and F conflicts, active drag priority, input consumption, and explicit source sharing exceptions.
- Ordinary push, support-body rejection, airborne recoil, grip start, steering, range loss, and release.
- Identity, fluid retention, mass restoration, collision ownership, and carry save restoration.

Native suites that require the installed export run explicitly with their ignored tests enabled.
A skipped installed-export test is not passing evidence for G07.
The worktree must use a matching local source export before those tests run.
The export is test data and is not added to Git.

Targeted commands use `scripts/check.ps1` for web input and the relevant native suites.
Rust changes require format checks and WASM compile checks for both configured backends.
Existing `wheel_input` and `garage_parts` tests protect installation behavior.
Changed export recovery also receives focused exporter tests.
Full checks, release builds, and browser acceptance require separate authorization or an agreed milestone.

## Completion and reported limits

Implementation completion requires passing targeted checks and source-backed requirements in this specification.
The report identifies changed behavior, checks, failures, deferred checks, and unverified runtime behavior.
Browser acceptance remains open until normal input demonstrates the complete G07 scenario with active physics and submitted frames.
Static checks and synthetic input do not establish browser acceptance.
The roadmap keeps code coverage separate from runtime acceptance.
The branch remains unmerged unless the user authorizes integration.

## Design-stage verification

Both read-only subagent audits completed before this specification.
The four existing web input tests passed in the new worktree.
No product code changed during design.
No native tests, WASM checks, release builds, full checks, or browser acceptance ran during design.
