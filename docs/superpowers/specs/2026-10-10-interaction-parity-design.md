# Home and Lada interactions

Date: 2026-10-10, Europe/Warsaw.
Status: The user approved the in-chat design. Written specification review is pending.
Reference: installed The Long Drive Steam build 11140890.
Base commit: d4cdefc.

## Intent and scope

The player must interact with the current home, Lada, and preparation supplies through common visible surfaces.
The user requested missing interactions based on the decompiled game.
The user approved action resolution, complete carry movement, cabin mechanisms, compatible part changes, pushing, and throwing.
Success means a continuous preparation and driving route with useful prompts and working actions.

Use the separate `interaction-parity` worktree. Leave the primary checkout and other chats' changes untouched.
Use subagents for independent tasks after defining interfaces and file ownership in the implementation plan.
Keep Rust, Bevy, Rapier, WebAssembly, and the selected installed assets.

This specification extends the earlier garage scope for the selected home and Lada.
It adds recovered mechanical controls and removable parts within those imported hierarchies.
It also covers the existing cans, barrel, faucet, and spare wheel.
Do not add unrelated items, other vehicles, survival, weapons, inventory slots, saves, multiplayer, or deployment.
Radio playback, horn audio, and live mirror reflections remain excluded.
Mirror adjustment must move the imported mirror. It does not require a reflection camera.
Do not add reduced-motion behavior.

## Evidence and recovery

The [comparison](../../research/2026-10-10-interaction-reference.md) records the current gaps and recovered methods.
Keep decompiled C#, generated assets, and recovery evidence outside version control.
Keep Steam files unchanged.

The original separates use, pickup, attachment, and push masks.
Recovered use, pickup, and attachment masks are 12747265, 8388609, and 8552449.
Recover the selected push mask before adding push targets.
Original pickup searches parent `pickupable` components instead of accepting the nearest item label.
Original part removal calls `UnCraft`, then picks up the removed part.
Original mechanisms use rotation, sliding, and switch state through `usablescript`.

Recover component links, collider ownership, pivots, axes, limits, speeds, and initial states from the installed hierarchy.
Recover slot types, cover rules, craft restrictions, mass, damping, and collider states for removable parts.
Retain source IDs and byte-consumption checks for each added record.
Fail export on unresolved required links or unsupported in-scope mechanism data. Name the component and field.
Do not guess a value or silently omit a required action.

Extend the existing typed manifest with explicit action and part records.
Validate references and finite values before Start becomes available.
Replace obsolete item-kind action rules. Require a fresh export after the manifest contract changes.
Do not add old-manifest readers, migrations, or fallback exports.

## Action resolution and prompts

Keep collider identity through aim selection.
Resolve separate owners for use, pickup, part installation, part removal, and push.
Build these links from source components and ancestor ownership during loading.
Use the recovered mask for each action. An item label alone does not permit an action.

A solid surface must block targets behind it.
Parent resolution may follow the hit object's hierarchy. It must not search through unrelated geometry.
Use action masks without losing pickup-only surfaces excluded from the use mask.
Keep target reach and seat-clearance rules from the existing recovered parameters.
Exclude the player body and respect carried-body query rules.
Cache bindings and query storage. Avoid per-frame sorting, heap allocation, and hierarchy scans.

Aiming at a can cap must allow E for the cap and F for the can.
Resolve wheel surfaces through actual part ownership and slot rules.
A removable hubcap must offer its own removal. Resolve the wheel after removing a covering hubcap.
A nonremovable child surface must route removal to the correct parent part.
Do not skip a removable covering part to remove its covered parent.

Return resolved actions and blocked-action reasons from one interaction resolver.
Use this result for both execution and HUD prompts.
Show the target name and valid input. Show short reasons for covered parts, occupied slots, or incompatible parts.
Keep fluid quantities and condition available for the resolved object.
Clear stale notices after target or action changes.

## Controls and carry behavior

Keep movement, look, seating, ignition, handbrake, camera, pause, and recovery controls.
Use these controls for added actions:

| Input | Action |
| --- | --- |
| E | Toggle an aimed cap or mechanical switch; enter or leave a seat |
| Hold E and mouse | Move the aimed rotation or sliding mechanism |
| F with empty hands | Pick up a loose item or remove an eligible mounted part |
| F with a held part and compatible target | Install the part under the recovered mount rules |
| Tap F with a held item | Drop when no installation applies |
| Hold F with a held item, then release | Charge and throw |
| Hold F on an eligible push target with empty hands | Push |
| Mouse wheel | Change carry distance or tune the aimed source valve |
| Hold RMB and mouse | Move the held item sideways and vertically |
| Hold MMB and mouse | Rotate the held item in yaw and pitch |
| Hold MMB and RMB, then move mouse | Roll the held item |
| Tab with a held item | Recenter using the recovered item offsets |
| X | Split or assemble compatible wheel parts |
| Hold LMB | Fill an open destination through the existing fluid rules |

Use source eligibility and input order to distinguish pickup, mounting, pushing, dropping, and throwing.
Do not drop on F press before deciding whether mounting or throwing applies.
The pickup press that acquires an item must not also throw it.
An aimed incompatible slot must retain the held item and show a reason. It must not trigger drop.
F must not pick up the complete chassis or home body.

Recover carry sensitivities and item recenter fields. Retain the existing collision-safe carry constraint.
Recover throw force rules from `fpscontroller` and `pickupable.Drop`.
The player parameters record minimum throw force 0.2, gain 1.5, and multiplier 1000.
Verify the source force application before converting it to Rapier impulses or velocity changes.
Match source charge limits and release behavior. Display charge only while throwing can occur.

Cancel drag, push, and throw input on pause, capture loss, focus loss, restart, or invalid target removal.
Resume must not apply a delayed throw, push, or mouse delta.

## Home and cabin mechanisms

Export active mechanical controls in the selected home and Lada.
Include authored doors, window controls, glovebox, ashtray, sun visors, and adjustable mirrors.
Replace the exporter name filters and hardcoded home-control allowlist with supported component records.
Do not infer behavior from object names.

Support one-axis and two-axis rotation, authored sliding paths, and discrete switches required by these objects.
Preserve source child links, inversion, relative drag axes, and applicable enabled states.
Use source pivots and limits. Preserve mechanism position across pickup and attachment changes where the source rules require it.
Update visible nodes and solid colliders from the same committed state.
Keep caps and valves in the existing fluid module.

Implement aimed handbrake controls with the same authoritative state as Space.
Implement recovered headlight and interior-light controls with visible light states and source electronics gates.
Reuse existing Bevy light support. Recover lamp transforms and light parameters during export.
Allocate light entities and materials during loading. Toggle or update them during play.
Do not implement an audio-only switch as a successful action with no result.

While seated, give a valid aimed mechanism priority over seat exit.
Only seat exit clears occupancy. Mechanism use must preserve driving state.
Detached mechanisms must follow recovered enabled-state rules.

## Parts and physical state

Support source-removable Lada parts, including doors, lids, engine, radiator, seats, bumpers, lights, mirrors, plates, and hubcaps.
Include existing cabin parts when their source slot permits removal.
Keep structural roots fixed to their identity. Respect `cantCraft`, `cantUnCraft`, cover rules, and pickup restrictions.

Use stable part and slot IDs. Represent each part once in world, held, or attached state.
Keep nested ownership explicit. Children move with their parent until a permitted removal separates them.
Removing a part must place that part in hand in one state change.
Reject removal before changing state if its source rules or carry constraints prevent the action.

Installation requires an available compatible slot and all source cover rules.
Use source type intersections and mount rotation. Do not infer compatibility from names or distance alone.
Reject invalid installation without releasing the held item or changing occupancy.
Keep wheel geometry, support, tire assembly, and grip behavior in the existing wheel module.
Share slot eligibility with general parts. Do not duplicate wheel ownership.

Prepare reusable body, collider, and visual bindings during load.
Activate the loose representation after removal and the attached representation after installation.
Maintain solid collision without overlapping duplicate colliders.
Update attachment anchors and moving-child transforms with the current parent pose.

Preserve fluid contents, condition, and item identity through changes.
Recalculate vehicle mass from installed parts and fluids. Preserve source total-mass rules.
Removing the engine must stop engine drive and starting. Keep the removed engine's oil and temperature with the item.
Removing the radiator must remove its cooling contribution. Keep its coolant with the item.
Installation must restore the installed component bindings and effective electronics or cooling state.
Do not drain or refill a removed component unless the source action requires it.

Reject removal of an occupied seat. Retain clearance checks for normal seat exit.
Do not leave the player assigned to a removed seat.

## Pushing and lifecycle

Push only source-eligible objects and the selected Lada through recovered push components.
Apply force at the aimed point. Respect source force limits and grounded restrictions.
Wake affected sleeping bodies. Do not create thrust or bypass the handbrake.
Reuse Rapier queries and existing player movement for collision and body contact.
Do not add push-and-steer grip unless required by a selected push target's source component.

Keep gameplay changes in fixed steps. Rendering reads committed state.
Pause must freeze mechanisms, attachment changes, pushing, throwing, and fluids.
Restart restores source startup attachments and mechanism states, plus the existing intentional garage-start choices.
Clear constraints, input latches, pending actions, and stale bindings during restart.
Recovery preserves removed parts, cargo, fluids, condition, and trip distance.
Floating-origin changes must move all loose and attached bodies, anchors, lights, and stored drag points together.
Returning to the home must retain moved items and mechanisms throughout the session.

## Module boundaries and implementation layers

Keep the existing modules. Add focused files inside `interaction` when needed.

| Module | Responsibility |
| --- | --- |
| `tools/reference` and `reference` | Recover and validate typed component, action, light, and part records |
| `interaction` | Resolve actions, process input, carry, mechanisms, pushing, and throwing |
| `interaction/items` | Keep stable identity, ownership, item fluids, and condition |
| Part and wheel modules | Validate slots and commit physical attachment changes |
| `vehicle` and `simulation` | Apply mass, engine presence, cooling, forces, and fixed-step order |
| `starting_area` | Bind imported bodies and reset persistent area state |
| `presentation` | Apply mechanism and part poses; update lamps |
| `app` and `browser` | Deliver input and display the shared action result |

Build in this order:

1. Repair source owner and mask resolution, with shared prompts.
2. Complete carry controls and wheel removal into hand.
3. Add rotation, sliding, and switches for home and cabin mechanisms.
4. Add general part attachment and vehicle effects.
5. Add push and throw, then verify the continuous browser route.

Keep the preparation and driving route working after each layer.
Do not add configuration, plugin registries, scripting engines, or speculative component abstractions.
Update controls and verification documentation with the stop-slop skill.

## Acceptance

Write only critical tests. Reuse current physics, fluid, wheel, and lifecycle tests.
Add failing regressions before fixing the confirmed targeting failures.

Native and exporter checks must establish:

- Can-cap surfaces permit can pickup without losing cap use.
- Wheel-center actions follow hubcap ownership and covering rules.
- Solid geometry blocks use, pickup, attachment, and push targets behind it.
- Prompts describe the same action that input performs.
- Carry translation, rotation, recenter, and throw cancel without stale input.
- Mechanisms clamp to source limits and move their colliders with their visuals.
- Part removal and installation preserve identity, contents, condition, mass, and slot occupancy.
- Wrong slots, covers, and occupied seats reject actions without partial changes.
- Removing engine or radiator changes drive or cooling, and reinstalling restores the binding.
- Pause, restart, recovery, and origin changes preserve the required state.

Run formatting, the native suite, exporter checks, installed-source gates, and the release WASM build.
Use the actual regenerated export for installed-source acceptance.
Keep synthetic test geometry separate from production assets.

Verify a continuous browser route from the garage spawn.
Walk to a can and pick it up through its cap surface. Adjust its pose and fill the car.
Use the garage doors and cabin mechanisms. Remove and install a wheel through normal visible surfaces.
Remove and install an engine or radiator. Verify the resulting vehicle state.
Push the car, drop and throw an item, then enter, start, and drive through the garage exit.
Check pause and restart with a held item and removed part.
Record screenshots and action results. Record asset size, loading failures, and frame pacing against the current baseline.

Source tests that stage the player near each target do not replace the continuous browser route.
Report any unavailable browser check with its exact cause. Do not claim full acceptance without that check.
Recovered rules do not establish identical Unity physics or full-game parity.
