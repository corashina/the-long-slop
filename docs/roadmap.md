# Project roadmap

Updated: 2026-10-20. Approved scope: one complete garage repair and driving experience.
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
Verify = code exists; only acceptance remains. Partial = known behavior gaps. Missing = no complete gameplay path.

| ID | Task and starting code | Difficulty | Status | Remaining work |
| --- | --- | --- | --- | --- |
| G01 | Reliable session start. [Session](../src/app/save.rs), [menu](../web/menu.js) | Hard | Verify | Accept New game, mouse capture, pause, Continue, restart, and cold Load in a browser with working mouse capture. |
| G02 | Remove and install Lada parts. [Parts](../src/interaction/parts.rs), [verification](g02-parts.md) | Hard | Verify | Accept part removal and installation through normal input in a browser. |
| G04 | Fill and drain fluids. [Fluids](../src/fluids.rs), [evidence](superpowers/reports/2026-10-20-g04-fluids.md) | Hard | Verify | Accept aiming, visible flow, audio, and browser reload. Run the two ignored reference-loading runtime cases. |
| G05 | Start and maintain the engine. [Engine](../src/simulation/engine.rs), [specification](superpowers/specs/2026-10-20-g05-engine-design.md) | Hard | Verify | Accept input capture, dashboard, engine audio, and driving in a browser. |
| G06 | Clean, restore, and paint. [Tools](../src/interaction/cleaning.rs), [conditions](../src/conditions.rs) | Hard | Partial | Verify rubbing, repair stages, paint, tool depletion, and visible changes. Complete spray refraction and soft-particle depth fading. |
| G07 | Carry, handle, and push objects. [Interaction](../src/interaction.rs), [carry](../src/interaction/carry.rs) | Hard | Verify | Accept pickup, drop, throw, rotation, push, and reset through normal input in a browser. |
| G08 | Equip and use inventory. [Inventory](../src/inventory.rs), [verification](g08-inventory.md) | Hard | Verify | Accept equip, release, Tab, slot selection, swapping, and saved ownership in a browser. |
| G09 | Inspect and move around the car. [Player](../src/player.rs), [input](../web/input.js) | Hard | Partial | Fix Rapier slope normals over flat floors near 2000 units. Accept movement, posture, seats, and schema 9 save/load in a browser. |
| G10 | Operate garage and car controls. [Mechanisms](../src/interaction/mechanisms.rs), [verification](g10-controls.md) | Hard | Verify | Accept the twelve normal-input scenarios. Verify gauge readability and audible output. |
| G11 | Complete Lada driving physics. [Vehicle](../src/vehicle.rs), [drivetrain](../src/simulation/drivetrain.rs) | Very hard | Partial | Complete launch, tire grip, bare rims, empty hubs, suspension, wheel spin, and combined braking. Verify gears, clutch, and assists. |
| G12 | Drive outside and return. [Area](../src/starting_area.rs), [world](../src/world.rs) | Hard | Verify | Drive out, turn, brake, park, and return. Verify nearby collision, ground seams, re-entry, and loose-object stability. |
| G13 | Preserve garage progress. [Save](../src/save/schema.rs), [restore](../src/starting_area/save.rs) | Hard | Partial | Save radio controls. Preserve all garage and car state through save, browser reload, load, and continued driving. |
| G14 | Add working Lada mirrors. [Renderer](../src/presentation/frame_graph.rs) | Very hard | Missing | Render live views for the selected car's mirrors. Preserve adjustment and removal behavior. Verify orientation, visibility, and lifecycle. |
| G15 | Finish usability and performance. [Presentation](../src/presentation/mod.rs), [audio](../web/audio.js) | Very hard | Partial | Fix reproduced visual and audio defects. Agree performance targets, measure them, and fix measured startup or frame-time problems. |
| G16 | Accept the complete garage experience | Hard | Verify | Repair the Lada, fill it, start it, drive outside, return, save, reload, and continue through normal input. |

G11 includes service braking, engine braking, and the handbrake under simultaneous inputs.
G12 requires a reliable local driving area. It does not require extended road generation or distant travel.
G13 follows the approved garage state. Saved survival needs remain deferred.
G15 covers defects in the selected experience. It does not reopen the old optimization proposal list.
G16 requires real input, active physics, visible results, and submitted frames on the same tested package.
It also requires the seven deferred G03 wheel checks in the [wheel audit](g03-wheel-audit.md).

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
