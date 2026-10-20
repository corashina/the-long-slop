# G09 player movement and inspection

Date: 2026-10-20.
Status: Written specification approved by the user on 2026-10-20. Implementation planning authorized.
Branch: `codex/g09-player-movement`.
Base commit: `cc7540b255d683722c67e09103a66bfa789c0fe7`.

## Goal and scope

Complete G09 from `docs/roadmap.md` for the garage Lada.
The player can move around the car, inspect low parts, lean, zoom, and enter or leave seats.
Walking, running, jumping, crouching, and seat transitions remain usable throughout implementation.
Steam build `11140890` and its recovered records define behavior.

The approved design extends `PlayerState` and the existing Rapier controller.
A replacement controller would enlarge the work without resolving the input and camera contracts.
Camera-only lean would leave interaction rays and carried objects at the wrong position.

This task excludes clothing, body inverse kinematics, general settings, new vehicles, and driving-physics changes.
Existing chase and sleep cameras remain supported. G09 adds no new camera modes.

## Current gaps

`src/player.rs` has two capsule shapes and one `crouched` flag.
Posture changes immediately. The view uses half the selected capsule height.
Lean, independent look, and variable camera field of view are absent.
The dynamic push-grip branch returns before posture updates.
Recovery assumes a standing player.

`FrameInput` carries a primary `u32` action mask. Only bit 13 is unused.
Inventory already uses separate fields.
Browser mouse buttons follow DOM indexes. MMB rotates items, and RMB moves items, as the source controls and help require.
Save schema 6 stores crouch as a Boolean.

## Source contract

Source locations below refer to `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs`.
Authored player values come from `.cache/reference-library/records/level1/80125.json`.
The source rigid-body mass is 72 kg, from `records/level1/50702.json`.

| Behavior | Required result | Source lines |
| --- | --- | --- |
| Movement | WASD uses body axes. Combined direction has magnitude at most one. | 18692-18767 |
| Speed | Base speed is 1.75. Run multiplier is 2.5. | 18543, player record |
| Posture speed | Interpolate through multipliers 0.3, 0.6, and 1 at posture 0, 0.3, and 1. | 18543 |
| Ground jump | Space press applies source force 10000 through the source force interval and player mass. | 18313-18318, 18539-18541 |
| Crouch | Ctrl requests posture 0.3. Release requests standing when the source transition permits it. | 16772-16807, 18770-18797 |
| Wheel posture | Wheel changes posture continuously. Wheel-selected posture persists after the wheel stops. | 16789-16807 |
| Clearance | Ten ordered sensors limit upward posture movement. | 15141-15144, 18823-18834 |
| Free look | Held Q gives independent pitch and yaw, each limited to 90 degrees in either direction. | 16526-16546 |
| Standing lean | A Q release before 0.2 seconds toggles positional lean. | 16522-16559 |
| Seated lean | Held Q moves the head, unless the seat selects independent look. | 16482-16511 |
| Recenter | Seated Tab resets head position and approaches default seat angles when no carried-object action takes priority. | 16440-16451 |
| Zoom | Held Z or Y zooms. Empty-hand RMB also zooms. | 16047-16051, 54982-55021 |
| Seat use | E release enters. E press attempts exit. | 17812-17827, 17320-17363 |
| Saved posture | Source save captures and restores `duckValue`. | 40887, 41074 |

### Continuous posture

Represent posture as a finite value in `[0, 1]`.
Zero is the fully low posture. One is standing. Ctrl crouch is 0.3.
Store the source transition state separately from the posture value.
This state distinguishes held Ctrl, wheel-selected posture, rising, and airborne duck actions.

Wheel adjustment uses the source scroll convention and `duckSens`, whose authored value is 0.05.
Do not apply that value to raw browser wheel units without the existing scroll conversion.
When posture exceeds 0.3, Ctrl snaps it downward to 0.3.
When posture is below 0.3, Ctrl requests an increase to 0.3 through clearance checks.
Automatic rising advances by `dt * 5`, subject to the source transition and clearance rules.
Head position interpolates between the recovered prone and standing anchors.
Collider radius interpolates between source limits 0.1 and 0.3.
Collider center and height follow the source anchor geometry.
At the low endpoint, use Rapier's valid sphere or capsule representation for Unity's minimum capsule extent.
Recover the additional `Feet2` sphere from source record `level1:57258`.
It is enabled only above posture 0.3 and uses the same interpolated body radius.
Preserve its source local pose and collision filtering, at lines 16810-16814.

Recover the ten sensor positions, shapes, and masks. Preserve their ordered clearance behavior.
A clear final standing capsule alone does not replace intermediate clearance checks.
Posture changes preserve the correct lower anchor and do not move the player through a ceiling or floor.

The source speed formula uses continuous posture, run, and existing carried-weight modifiers.
Verify jump, landing, and source airborne duck transitions against the decompiled force calls.
The source permits one duck jump after leaving the ground and applies an inverse force on release.
Its authored multipliers are 1.25 and 0.5. Ground contact resets the relevant availability state.
Retain the existing controller and change movement behavior only where G09 source evidence establishes a discrepancy.

### Lean and independent look

Maintain separate body angles, independent look angles, and positional lean.
Q free look does not turn the walking axes.
A short Q release toggles standing positional lean. A longer hold does not toggle it.
While standing lean is active and Q is released, mouse motion moves the head.
In a normal seated view, held Q moves the head.
All five Lada seats have `lookInsteadOfLean=false`. G09 implements their positional lean path.
Do not add the alternate seat mode for this milestone.

Positional lean uses source sensitivity 0.15 and the gameplay step duration.
Mouse X moves along head-right. Mouse Y uses head-forward projected onto the body plane.
Shift replaces that forward direction with body-up. Wheel motion contributes body-up times five.
Clamp lean displacement to source distance 0.6.
Standing lean uses the mean of the standing, prone, and current head positions as its center.
Seated lean uses the standing head anchor as its center.
Normal head tracking uses interpolation factor `dt * 15`.
Independent look returns with the source `dt * 10` rule and its 0.5-second return window.

Apply source lean bounds. Do not invent a camera collision rule absent from the source.
Positional lean moves the source head pose and its child cameras.
Q independent look adds a camera transform without turning the body or base head direction.
Use, pickup, attachment, and crosshair targeting use the resolved camera pose.
Source ray locations are lines 17636, 17963, 17546, and 17292-17315.
Carried-object anchors and head audio sensors retain their source head-space transforms.
The shared view result exposes both poses. Consumers select the source head or camera pose explicitly.
Held seated positional lean blocks `RayStuff` actions, at lines 17530-17534.
Foot independent look does not set that seated-lean block.

### Zoom and projection

Select the recovered foot field of view when walking and car field of view when seated.
Recover both source settings defaults. This adds no settings interface.
Zoom approaches 0.4 times that base field of view with interpolation factor `dt * 10`.
Release approaches the configured field of view with factor `dt * 20`.
Normal look sensitivity uses multiplier 0.4 while the source zoom predicate is true.
Independent Q look retains its own source sensitivity rules.

One effective field of view feeds all player-facing world cameras and their source snapshots.
It also feeds sky projection, LOD selection, required-view preparation, and save staging.
Source `SetAllCamFov` updates the player, far, sky, main, and screenshot cameras at lines 19041-19068.
Reflection-probe cube faces keep their 90-degree projection.
Existing chase and sleep selection remains explicit, with no first-person lean transform added to those views.

## Components and data flow

`PlayerReference` owns recovered anchors, sensor records, movement constants, lean constants, and view constants.
`SeatReference` owns the selected seats' source angle limits and default view angles.
Their records are `level1:77843`, `76381`, `65840`, `69296`, and `66466`.
All five seats use yaw limits of -179 and 179 degrees, with default pitch zero.
Seated Tab approaches yaw zero and the recovered default pitch.
Seated pitch is limited to -89 and 89 degrees.
Exit requires an angle of at least 65 degrees from seat-forward.
No entry tilt limit applies. The source `BcantSitAngle` field is false.
Driver seat `level1:69296` has both driver flags enabled.
Unused authored seat fields do not introduce behavior.
The exporter and Rust validator require these fields. Missing source data is an export or load error.
Remove replaced derived-only fields after their consumers move to the new contract.

`FrameInput` gains compact player held, pressed, and released fields for Q and dedicated zoom.
The browser bridge carries these fields separately from driving and inventory masks.
Frame merge preserves edges until one gameplay step consumes them.
Input clear removes all pending player fields. Multiple zoom keys preserve held state until all are released.
Preserve RMB movement and MMB rotation. Test these actions through DOM mouse events.

`PlayerState` owns body movement, posture, mode transitions, and one authoritative gameplay view.
A focused `src/player/posture.rs` module owns posture transitions and dimensions.
A focused `src/player/view.rs` module owns lean, independent look, and zoom transitions.
These modules return plain values. They do not own rendering resources or browser state.

`GarageRuntime` supplies interaction context and routes shared mouse and wheel input.
The player view result contains head pose, camera pose, and effective field of view.
Use, pickup, attachment, crosshair targeting, and rendering use camera pose.
Carried-object anchors and head audio sensors use head pose.
Streaming and load staging use camera pose and effective field of view.
All consumers share the same resolved state. No consumer reconstructs lean independently.
Render interpolation must not apply source head smoothing a second time.
World-origin shifts move world positions without altering local lean offsets.

Reuse Rapier queries and the existing controller. Add no dependencies.
Create sensor records and reusable query storage at setup.
Update collider dimensions only when posture changes. Avoid per-frame shape allocation and repeated setup.

## Input priority

Retain source context predicates instead of assigning wheel input an unconditional consumer.
Mechanism tuning and active object handling keep their source mouse and wheel actions.
Normal look is suppressed when those actions consume mouse motion.
Q look or positional lean suppresses ordinary body look when its source mode requires that suppression.

`CanDuck` requires captured input, no tuning availability, no reload, and no humming.
Without a picked-up object, wheel posture is permitted by those conditions.
With a picked-up object, wheel posture additionally requires Q and no RMB.
A slideable rotating mechanism adds the source wheel-posture restriction at lines 16790-16806.
Do not add deferred reload or humming gameplay. Use existing applicable state without inventing new systems.
Source permits some shared scroll effects. Do not impose exclusive consumption where source branches both apply.
Standing lean and posture can both change during one wheel update.
Positive wheel input above posture 0.95 also changes the source free-look head-height offset.
That offset uses `scroll * 0.05` within limits -0.1 and 0.375, at lines 16518-16520.

RMB zoom requires an empty picked-up slot.
Dedicated Z or Y zoom does not require that slot to be empty.
Tab retains carried-object recenter priority before seated recenter.
Ctrl remains the clutch input while driving. Walking posture input cannot move the seated body.
Update control help with Q hold, Q tap, zoom, and continuous posture behavior.

## Lifecycle and save state

Seat entry clears standing lean and walking posture transitions according to source `GetIn`.
Use recovered seat angles and positional lean. Do not substitute a generic seat limit.
Seated posture is always one, as in source line 15905.
Seat entry compares the usable component on release with the component recorded on press.
Null components compare equal, including different seats with no usable component.
For seats with a usable component, the hold duration must be strictly below 0.2 seconds, from source line 17825.
Seats without a usable component have no hold-duration limit.
Source entry predicates are at lines 17819-17827.
Seat exit angle permission uses camera-forward. Its placement search uses head position and head-forward.
Preserve the existing source capsule search and masks, at lines 17247-17279.
A blocked exit keeps the player seated. Successful exit restores valid standing geometry.
Apply 0.3 times the parent linear velocity, including horizontal components, as in source lines 17344-17359.

Push grip retains its existing dynamic-body path.
View and posture processing must not disappear through the current early return.
Apply source posture permissions and shape changes without replacing push forces or G07 ownership rules.

Pause and capture loss cancel pending Q charge, independent look input, positional lean, and zoom input.
Cancellation is not a Q release and must not toggle standing lean.
Neutralize temporary view offsets and zoom without changing the persistent posture value.
Resume starts with cleared input. It must not force a low player through an obstacle.

Recovery preserves the posture value and uses its actual collider and head geometry during placement.
Recovery resets temporary view state and transforms carried bodies from the resulting gameplay view.
A failed recovery placement leaves the existing player state unchanged.

Replace saved `crouched` with continuous posture. Increment `SAVE_SCHEMA_VERSION` from 6 to 7.
Validate finite posture in `[0, 1]` and rebuild matching collision geometry before activation.
Source saves posture, body angles, and position. Lean mode, Q charge, and zoom are temporary view state.
Do not serialize those temporary states. Initialize them neutrally after load.
Preserve the existing velocity, seat, ownership, and carry snapshot contracts.
Restore a wheel-selected low posture without an automatic stand request from stale input.
Reject old schema versions. Add no compatibility paths or migrations.
Prepared load views must use the restored posture and neutral zoom before the candidate commits.

## Verification and completion

The implementation keeps each layer usable and uses separate subagent file ownership.
The implementation plan defines task order and review boundaries after this specification receives approval.

Critical checks cover these observable contracts:

- Source export and Rust validation agree on new player and seat fields.
- Walking, running, diagonal speed, jumping, landing, and continuous posture follow the recovered contract.
- Intermediate obstructions limit rising. The fully low capsule remains valid.
- The additional foot sphere follows its source posture threshold, pose, radius, and collision mask.
- Q tap differs from Q hold. Seated lean and recenter use recovered seat values.
- Zoom changes every affected projection consistently and preserves probe projections.
- Camera interaction rays and carried-object head anchors follow their source transforms.
- Q independent look redirects camera targeting while exit search retains head direction.
- Held seated lean blocks source ray actions. Foot free look does not.
- Carry, tuning, wheel posture, and RMB zoom follow source predicates.
- Seat entry, blocked exit, successful exit, push grip, recovery, and rebase preserve valid state.
- Save round trips preserve intermediate posture. Invalid posture and schema 6 are rejected.
- Pause, capture loss, and load discard temporary input without producing a Q tap.

Use targeted Python checks for changed source recovery.
Use targeted native player, save, and transition checks for Rust changes.
Run the required WASM compile checks for WebGL2 and WebGPU.
Run the input and menu web suites for changed browser behavior.
Installed-reference tests must run explicitly if marked ignored. A skipped test is not evidence.

Full checks, release builds, and browser acceptance require a separate explicit request or agreed milestone.
No such milestone is approved by this specification.
Report code coverage separately from normal-input runtime acceptance.
Keep G09 Partial until all scoped behavior exists. Do not mark browser acceptance complete without direct evidence.
Record completed checks, failures, deferred checks, and unverified runtime behavior in the roadmap update.
