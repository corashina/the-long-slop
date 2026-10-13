# Portable Flashlights Design

Status: approved on 2026-10-13. Product implementation awaits plan review.
Date: 2026-10-13. Branch: codex/equipment. Base: 542be787ee131861920c09c4ad37ba05a103b090.

## Goal

Restore source portable flashlight behavior through the existing carry, interaction, physics, and lighting systems.
Keep the same item and switch state through carry, drop, storage locking, pause, and origin changes.
Restart must restore source startup state and complete renderer preparation before play.
The user excluded further automated tests. Use release builds, code review, and visible browser checks.

## Source authority

Use installed Steam build 11140890 and Unity 2021.3.18f1.
The accepted library manifest receipt is 92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99.
Managed source SHA-256 is 22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee.
Keep installed files and the primary source library unchanged.
Recovered records are in .cache/equipment-source/flashlights/{source,details,startup,mounts,settings}.json.
Keep these records, models, textures, cookies, and browser probes outside Git.

## Models and startup

| Model | Source controller | Pickup | Startup evidence |
| --- | --- | --- | --- |
| Older regizseblampa | level1:74834 | level1:69340 | GameObject 12118 is active under StartSpawnItems117; terrain startup enables117 |
| Newer ujlamp | level1:74835 | level1:69341 | GameObject 12119 is inactive under the same startup root |
| Newer player model | level1:74880 | level1:69384 | Player80125 references it in alternate startup inventory groups |

Recover both model hierarchies and their source materials.
Instantiate the older source-enabled garage flashlight at its authored pose.
Keep the inactive newer scene object inactive. Do not add a second garage flashlight.
The selected player record has all three startWithItems flags false.
Recover newer prefab and inventory links as provenance. Do not invent an acquisition route.
This feature does not add inventory, loot, purchase, or cheat interfaces.
A labelled diagnostic may exercise a checked newer model without claiming gameplay acquisition.
Exclude PumpKin controllers from handheld flashlight selection.

The shared prefab controllers are sharedassets1.assets:148780 for the older model and148779 for the newer model.
Their pickup links are138785 and138784. Resolve all links through source pointers.
Use source IDs for binding. Do not bind through current runtime item numbers.

## Controls and power

The controller is flashlightscript. Its Refresh method applies source ON/OFF materials and light activation.
Both handheld models start with ON=false and randomStart=false.
Their electronics links are null. Electronics() returns true for that source case.
No depletion rule exists in these controllers. Add no battery, charge, consumption, or recharge behavior.

Player held-item handling calls pickup.lampa.TURN() on left mouse press.
An aimed world flashlight uses the existing use press or left mouse press when source LMB precedence permits it.
Keep held-item use ahead of world-target LMB use. One press must toggle once.
Use the existing switch feedback and controls. Add no new control key.
Preserve source pickup restrictions, reach, layers, and source controller links.
Source pickup, save, and lamp references must identify the same item.

## Beam settings

| Setting | Older model | Newer model |
| --- | --- | --- |
| Source Light | level1:56881 | level1:56882 |
| Kind | Spot | Spot |
| Intensity |0.8500000238418579|1.5|
| Range, metres |12|15|
| Full cone, degrees |90|80|
| Inner cone, degrees |8|62.188602447509766|
| Source RGB |1,0.8154796361923218,0.6009999513626099|1,1,1|
| Culling mask |4294967295|4294967295|
| Shadows |Hard, strength1|Hard, strength1|

Both lights use source bias0.05, normal bias0.4, and near plane0.2.
Recover the checked source cookies, shadow records, samplers, and light-node transforms.
Do not replace cookies or apply another coordinate conversion after scene import.
Use the existing light uniform, cookie atlas, source shadow atlas, and spotlight rendering path.
The current renderer already supports spot lights and one shadow face per shadowed spot.
Keep capacity checks explicit. Do not allocate a new renderer or per-frame beam resources.

## Carry and storage locking

Both source rigid bodies have mass 1, linear drag0, angular drag0.05, gravity enabled, and no kinematic flag.
Carry uses the existing retained dynamic body and joint. Drop restores source gravity and mass.
Beam pose follows the imported light node under the moving item hierarchy.
Do not attach the beam to the player camera independently of its model.

These flashlight pickups have no attachablescript or partscript link.
Their mounted state uses mountStuff vehicle storage locking, rather than plate targets or part slots.
Do not invent a flashlight mounting socket or a manual attachment action.

Recover the eight selected Lada storage volumes:

| Source mountStuff | Source path | Storage type |
| --- | --- | --- |
| level1:65838 | Car02Full/Hamutarto/Mount (1) | Glove storage |
| level1:65854 | Car02Full/GloveStoreMount | Glove storage |
| level1:67560 | Car02Full/PassengerMount | Passenger room |
| level1:79410 | Car02Full/PassengerMount (1) | Passenger room |
| level1:77806 | Car02Full/PassengerMount (2) | Passenger room |
| level1:78464 | Car02Full/PassengerMount (3) | Passenger room |
| level1:79301 | Car02Full/TrunkMount | Trunk |
| level1:69971 | Car02Full/EngineMount | Engine bay |

All eight link parentTosave to level1:73405 and have onlyManual=false and noPhysicsLock=false.
Recover collider pose, centre, size, scale, source ID, owner, and door or slot restrictions.
Do not substitute axis-aligned world boxes for source oriented overlap rules.
Source mountLayerMask is8388609.

Source neededDropTime is1 second.
The source requires membership in the preceding and current overlap scans before Lock.
It excludes held, already locked, restricted, and incompatible attached items.
Lock reparents the item, removes its independent rigid body, and updates containing mass.
Represent this with the existing retained body disabled while locked and unique collider ownership.
Keep source-relative pose. Retain one item and its lamp state.
Pickup must preflight carry, then unlock without losing pose, body ownership, or switch state.
A rejected transaction must retain its prior state.
Rebase must preserve local storage pose and world continuity.

Settings source level1:65886 links physics dropdown level1:70023.
That dropdown uses defInt=2 and useDefInt=true.
Use the recovered fresh-default policy 2; do not use the pre-initialization serialized S.IPhysicsLock=0 as its default.
Policy2 permits glove storage, trunk, and engine bay item locking, but excludes passenger-room item locking.
Keep the policy in the source contract. Add no settings interface in this feature.
All selected volumes use the non-basket scheduler: scan when elapsed time is strictly greater than1.5 seconds.
Source usable values above0.1 or missing required slot parts disable storage.
Apply the recovered initialization sequence: two yielded frames, then three QuickFreeze passes with only the first prescanned.
Restrict new locking integration to eligible flashlight items. Other cargo behavior remains outside this feature.

## Architecture

Extend checked recovery and the typed reference contract with portable model, pickup, controller, and storage links.
Require a fresh export. Add no old-manifest reader or compatibility default.
Keep source data recovery separate from item interaction and presentation.

Reuse the existing lamp switch state and light/material outputs.
Add portable item-to-controller binding and source held-use dispatch.
Keep storage-lock state in a focused interaction module using ItemStore, CarryState, and PhysicsWorld.
Use current body ownership for vehicle mass changes. Count each mounted flashlight once.
Keep imported item synchronization as the pose authority for models and their light children.

Prepare source geometry, material choices, cookies, and shadow variants during loading.
Keep assets shared through ownership transitions. Warm normal and mirror variants before readiness.
Use the plate restart readiness fix and current mirror instance guards.
Preserve all three mirror captures, their feedback guard, and their bounded targets.
Coordinate new source light bindings with the rendering completion plan.
Full deferred rendering replacement remains outside this feature.

## Failures and acceptance

Reject missing required source links, bad body data, unsupported startup rules, and capacity overflow with source IDs.
Keep export staging and promotion transactional.
Reject failed pickup or locking work before changing body, collider, or carry ownership.
Use current loading diagnostics for asset and pipeline failures.

Do not add or run automated tests unless the user changes the current instruction.
Build a matching production package and inspect the actual imported source models in the browser.
Check carried, dropped, locked, switched, paused, resumed, restarted, and rebased states.
Check both beam profiles and source material changes in labelled diagnostics.
Check a failed carry transition by source review or visible diagnostic evidence.
Compare matching ready-state asset, pipeline, buffer, and target counts across repeated transitions.
Keep three mirror capture targets and zero browser or GL errors.
Restore production after diagnostic builds. Record code, package, source, and asset hashes together.
Label scripted placement, inactive prototype use, trusted-input limits, and any unsupported source effect.
Do not claim matched Unity appearance, exact native motion, or unimplemented acquisition behavior.

## Exclusions

Battery depletion, recharge, new inventory, new loot, purchases, cheat interfaces, other vehicles, and general cargo locking are excluded.
Save-format changes, multiplayer, free-surface gluing, and full deferred rendering completion are excluded.
