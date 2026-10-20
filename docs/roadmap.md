# Project roadmap

Updated: 2026-10-19. Approved scope: one complete garage repair and driving experience.
Behavior authority: Steam build `11140890` and its decompiled code in `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs`.
Save format authority: [save schema](../src/save/schema.rs).

## Current scope

Use one complete Lada, the starting home, garage, yard, and nearby terrain.
Include required parts, tools, fluids, controls, local driving, and Save/Load.
One sample means one complete car with all required wheels, parts, and controls.
Existing extra cars and world code do not expand this milestone.
Clothing, survival, weapons, and general cargo expansion are deferred.

Code exists for most garage systems. Current browser acceptance remains open.
A build, static check, or scripted route does not prove normal gameplay.
Historical loading failures require fresh reproduction before repair.

## Task rules

Work in the order below. Keep each layer usable before starting the next.
Recheck the code and source before each task. Complete missing behavior and verify existing behavior.
Use the completion goal as the task boundary. Split large tasks during design.
Difficulty estimates cover completion, including integration and verification.
Moderate means limited changes. Hard means several connected systems. Very hard means complex physics, rendering, or performance work.

Follow [AGENTS.md](../AGENTS.md) for checks. Run only critical tests.
Release builds, full checks, and browser acceptance require an explicit request or an agreed milestone.
Use isolated `codex/` worktrees for architecture and performance work. Keep changes unmerged unless authorized.
Measure source driving before changing tire, rim, hub, suspension, or brake driving physics.
Measure performance with matched release A/B/A runs. Report startup, garage, and local driving separately.

Use this prompt template:

> Complete roadmap task <ID>: <task>. Recheck its current state against build 11140890.
> Limit work to the current garage scope and the task's completion goal.
> Follow the task rules and applicable design approval requirements.
> Report changes, checks, failures, deferred checks, and unverified runtime behavior.

## Current tasks

Statuses describe code coverage. No row claims current browser acceptance.
Verify = code exists. Partial = known behavior gaps. Missing = no complete gameplay path.

| ID | Task and starting code | Difficulty | Status | Completion goal |
| --- | --- | --- | --- | --- |
| G01 | Reliable session start. [Session](../src/app/save.rs), [menu](../web/menu.js) | Hard | Verify | Use matching assets and WASM. Verify New game, mouse capture, pause, Continue, restart, and cold Load. |
| G02 | Remove and install Lada parts. [Parts](../src/interaction/parts.rs) | Hard | Verify | Remove and reinstall the engine, radiator, body parts, and plates. Reject removal of source-fixed seats. Preserve identities, fluids, mass, and collision. |
| G04 | Fill and drain fluids. [Fluids](../src/fluids.rs), [interaction](../src/interaction.rs) | Hard | Verify | Operate caps and containers. Verify pouring, spilling, mixtures, empty supplies, and fluid retention after part removal. |
| G05 | Start and maintain the engine. [Engine](../src/simulation/engine.rs) | Hard | Verify | Verify ignition, starter, fuel use, cooling, oil effects, stalling, and missing-part behavior. |
| G06 | Clean, restore, and paint. [Tools](../src/interaction/cleaning.rs), [conditions](../src/conditions.rs) | Hard | Partial | Verify rubbing, repair stages, paint, tool depletion, and visible changes. Complete spray refraction and soft-particle depth fading. |
| G07 | Carry, handle, and push objects. [Interaction](../src/interaction.rs), [carry](../src/interaction/carry.rs) | Hard | Partial | Verify pickup, drop, throw, rotation, distance, recenter, and pushing. Complete source action priority and resolve control conflicts. |
| G08 | Equip and use inventory. [Inventory](../src/inventory.rs), [equipment](../src/interaction/equipment.rs) | Hard | Partial | Complete explicit equip and release, source Tab behavior, slot selection, swapping, and ownership rules. |
| G09 | Inspect and move around the car. [Player](../src/player.rs), [input](../web/input.js) | Hard | Partial | Verify walking, running, jumping, crouching, and seat transitions. Add source lean, zoom, and full low-posture behavior. |
| G10 | Operate garage and car controls. [Mechanisms](../src/interaction/mechanisms.rs), [instruments](../src/simulation/instruments.rs), [verification](g10-controls.md) | Hard | Verify | Source reactions, handbrake inputs, secondary-seat lights, dashboard lighting, and radio visuals implemented. Targeted checks pass. Computer Use URL policy stopped browser checks. All twelve normal-input scenarios remain open. Gauge readability and audible output remain unverified. G09 owns seat transitions. G13 owns missing radio persistence. |
| G11 | Complete Lada driving physics. [Vehicle](../src/vehicle.rs), [drivetrain](../src/simulation/drivetrain.rs) | Very hard | Partial | Complete launch, tire grip, bare rims, empty hubs, suspension, wheel spin, and combined braking. Verify gears, clutch, and assists. |
| G12 | Drive outside and return. [Area](../src/starting_area.rs), [world](../src/world.rs) | Hard | Verify | Drive out, turn, brake, park, and return. Verify nearby collision, ground seams, re-entry, and loose-object stability. |
| G13 | Preserve garage progress. [Save](../src/save/schema.rs), [restore](../src/starting_area/save.rs) | Hard | Verify | Preserve parts, fluids, tools, conditions, mechanisms, inventory, and car state through save, browser reload, load, and continued driving. |
| G14 | Add working Lada mirrors. [Renderer](../src/presentation/frame_graph.rs) | Very hard | Missing | Render live views for the selected car's mirrors. Preserve adjustment and removal behavior. Verify orientation, visibility, and lifecycle. |
| G15 | Finish usability and performance. [Presentation](../src/presentation/mod.rs), [audio](../web/audio.js) | Very hard | Partial | Fix reproduced visual and audio defects. Agree performance targets, measure them, and fix measured startup or frame-time problems. |
| G16 | Accept the complete garage experience | Hard | Verify | Repair the Lada, fill it, start it, drive outside, return, save, reload, and continue through normal input. |

G02 update, 2026-10-19: the incompatible-slot scan no longer blocks dropping a held part.
Three targeted native tests passed. They cover 20 general parts and both plates through three cycles each.
Checks cover identity, fluids, mass, collision ownership, invalid fits, duplicate attachment, and source-fixed seats.
The WASM compile check passed. G01 session input and G02 browser acceptance remain open.
See [G02 verification](g02-parts.md) for source evidence, checks, and limits. G02 stays Verify.

G11 includes service braking, engine braking, and the handbrake under simultaneous inputs.
G12 requires a reliable local driving area. It does not require extended road generation or distant travel.
G13 follows the approved garage state. Saved survival needs remain deferred.
G15 covers defects in the selected experience. It does not reopen the old optimization proposal list.
G16 requires real input, active physics, visible results, and submitted frames on the same tested package.
It also requires the seven deferred G03 wheel checks in the [wheel audit](g03-wheel-audit.md).
Verify hubcaps, complete wheels, tire separation, compatible assembly, fit rejection, step requirements, and repeated cycles.
Check identity, condition, ownership, mass, and collision through each transition.

Startup selection, 2026-10-19: the runtime export contains only the garage Lada.
The yard Skoda and its parking code are removed. The Lada record and pose are unchanged.
Export validation, 16 Python tests, two native tests, and the WASM compile check passed.
Full-library verification stopped at missing managed coverage. Targeted runtime derivation passed.
Full checks, release build, and browser acceptance were deferred for this change.

G01 implementation merged, 2026-10-19: Escape in the pause menu uses the existing Continue path.
Targeted web tests cover resume, pause, capture loss, failed capture, and canceled capture.
The baseline reached prepared New game, restart, and cold Load states with matching assets and WASM.
The in-app browser rejected capture in the game and a standalone page without game code.
Browser control timed out during the modified package recheck. Its readiness remains unverified.
All six browser paths require acceptance with working mouse capture. G01 stays Verify.

## Deferred local tasks

These tasks require separate scope approval. Their difficulty estimates require review before planning.

| ID | Task | Difficulty | Remaining scope |
| --- | --- | --- | --- |
| L01 | Clothing and appearance | Very hard | Wearables, appearance controls, player skins, full body pose, and inverse kinematics. |
| L02 | Complete survival | Hard | Verify needs, consumption, sleep, and bodily actions. Save player needs and verify death behavior. |
| L03 | General cargo and storage | Very hard | Extend flashlight storage locks to supported cargo. Preserve ownership, mass, collision, and saved state. |
| L04 | Towing and trailers | Very hard | Complete supported connections, towing physics, controls, detachment, and saved state. |
| L05 | Weapons and combat | Very hard | Complete firing, reload, melee, damage, effects, and saved state. |
| L06 | Extra item actions | Hard | Cigarettes, digging, buried-item recovery, and one-time box-content release. |
| L07 | General settings and save management | Hard | Rebinding, audio settings, save slots, and autosave. Appearance controls belong to L01. |
| L08 | Custom radio music | Moderate | Add user music to the existing radio playback system. |

## Deferred full-game tasks

Existing implementations may be partial. Re-audit each task before design.

| ID | Task | Difficulty | Remaining scope |
| --- | --- | --- | --- |
| W01 | Expand the vehicle range | Very hard | Additional cars, engine types, automatic gearboxes, motorcycles, and buses. |
| W02 | Complete long-distance travel | Very hard | Extended road generation, streaming, origin shifts, distant revisits, memory limits, and persistence. |
| W03 | Expand buildings and exploration | Very hard | More structures, interiors, interactions, and exploration routes. |
| W04 | Complete world population | Very hard | Source loot selection, random placement, supported population rules, and broader item distribution. |
| W05 | Add creatures and threats | Very hard | Animals, enemies, encounters, and related combat behavior. |
| W06 | Add weather | Hard | Source weather behavior and its visual, audio, and gameplay effects. |
| W07 | Add multiplayer | Very hard | Shared ownership, synchronization, sessions, and persistence. |
| W08 | Complete advanced visual effects | Very hard | Remaining source effects outside the garage milestone, including lamp shadows, motion blur, and TAA. |
