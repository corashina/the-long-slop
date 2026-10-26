# Project roadmap

Updated: 2026-10-26. Goal: recreate The Long Drive for the web.
Behavior authority: Steam build `11140890` and its decompiled code in `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs`.
Save format authority: [save schema](src/save/schema.rs).

## Frame rate rule

The game runs at about 60 FPS now. Every new feature must keep it there.
Check the F3 overlay before and after each change. Report the frame-time cost.
If a feature drops the frame rate, fix the cost before merge or agree a cut.

## Task rules

Work section by section, in the order below. Keep the game playable after each task.
Recheck the code and the original before each task. Complete missing behavior and verify existing behavior.
Follow [AGENTS.md](AGENTS.md) for checks.
Remove a row when its task is done. Do not mark rows done.

IDs in brackets are points in the 1:1 parity list (`plans/parity-1to1.md` in the project files).
Size: S is up to a day. M is a few days. L is about a week. XL is several weeks.

## 1. Starting area, player, and movement

Finish the home, garage, Lada, and nearby driving first.

All tasks are done. Run this test pass on Tomasz's PC next:

1. Pull master. Re-export, then rebuild. Old exports and saves do not load.
2. Check F3 near the garage and on the road. Compare with the frame rate rule.
3. Play the garage loop in the browser: repair, fill, start, drive out, return, save, reload.
4. Check the 20 L can stays on the ground after drive-out and return.
5. Check paint spray and sponge particles (P02): refraction and soft edges.
6. Check slope normals far from origin, ladders, head bob, sleep hint, hazards.
7. Check player sounds: hits, footsteps, ambience, muffling in buildings.
8. Check the music box, radios, compass, clock, and small items.

## 2. Car parts and repair

Fix part repair, electronics, the siphon hose, exhaust smoke, and cargo storage.

All tasks are done. Run this test pass on Tomasz's PC after section 1:

1. Pull master. Re-export, then rebuild. Old exports and saves do not load.
2. Run the ignored export tests: `cargo test --test garage_parts -- --ignored` and `cargo test --lib r05_cargo_locks_in_trunk -- --ignored`.
3. Repair (R01): swap the hood and the trunk lid, repair rust and dirt.
4. Electronics (R02): check the Lada radiator fan spins while the engine runs.
5. Exhaust smoke (R04): rate follows rpm, color changes after a fuel mix change, 5x5 sheet animates. Check the particle quads are not upside down.
6. Siphon hose (R03): first check the garage hose spawns clear of the walls. Attach it to the Lada tank and a can. Siphon by height (both ends are main, so the source runs the siphon twice per frame), drain with LMB, coil with R, store it, tear it off by driving.
7. Cargo storage (R05): lock items in the Lada trunk, glove box, and engine bay, and in a fridge, a pallet, and a coffin. Open the lid to release them. The source box is manual only: items put in it later stay loose.
8. Save and load with the hose attached and with cargo locked.
9. Check F3 with smoke, near a hose, and while driving with cargo. Compare with the frame rate rule.

## 3. World and terrain

Run these checks in the section 3 test pass on Tomasz's PC:

1. Re-export, then rebuild. Old exports do not load (W03 changed the world and lighting records).
2. Check the export log line "Biome materials" (set count and marked nodes).
3. Check the Game mode button position on the start menu.
4. Start a new game in each biome. Check terrain, props, roads, lamps, fog, sky and swapped materials.
5. Load a Mixed save with Winter set. The world must load as winter.
6. Check the winter color: colder view in Winter, changes at biome borders in Mixed.
7. Check F3 near the garage and on the road in Mixed while the color grading rebakes.
8. Sandstorms (W07): run `cargo test --lib terrain_sample_matches_published_ground -- --ignored`. Spawn a storm near the player with a temporary hook (never commit it). Check sky, fog, light, sound, dust, wind on loose items, and that dust and wind stop inside buildings and cars. Check the F3 storm line and the frame rate in a storm. Save and load inside a storm.
9. Power cables (W04): check poles link to the next pole on curves and tilted poles, road buildings and the home link to a pole, and the rope meshes are not flipped. Check F3 along the road.
10. Draw distance (W01): check F3 at 500, 2500 and 5000 m near the garage and on the road. Compare with the frame rate rule.
11. Set a draw distance that does not fit (above about 5500 m). New game must refuse it and name the largest value. Reload: the slider keeps the last value that started.
12. Check the slider is locked in the pause menu. Check far terrain, buildings and fog at 2500 m against the original.
13. Run `cargo test --test world_range -- --ignored --nocapture`. Peaks must stay below the limits.
14. World population (W05): run `cargo test --release --lib w05_loot_matches_source_rules -- --ignored --nocapture` (about 5 min plus build). All 11 building types must pass. The `unsupported_part_type` drops must be gone.
15. Check loot in a few building types: gas and oil cans, beer, barrels, buckets and the WC now spawn. Flashlights in loot must not stop the world, and their beams show the cookie on walls and terrain.
16. Near the Mansion: the crow sound fades in from 1000 m to full at 200 m, and the day and night ambience fades out by the same amount. Check F3 near the Mansion.
17. Digging (W06): find a buried item (underground spawn). Swing the shovel in each direction (LMB with back, left, right keys). Check the swing sound, hit sounds, hit particles on metal, wood, plants and sand, and that hits push loose items.
18. Dig a buried item up with the shovel. It must lift, become loose, and then allow pickup. A buried item must refuse pickup. Save and load with an item buried and one dug up.
19. Metal detector (W06): turn it on and off with use. Check the beep pitch and volume and the needle near metal, buried metal, and behind a wall. Save and load with it on. Check F3 while it scans.
20. Tumbleweeds (W07): check they spawn near the player, roll in gusts, stop near walls, and save with the world. Check F3 with several tumbleweeds.
21. Long trips (W02): run `cargo test --release --lib w02_ -- --ignored --nocapture`. Note the printed bytes per km and the 5,000 km estimate. Above 200 MB, compact spawn boundaries.
22. Also run the roadside tests: `cargo test --lib roadside_ -- --ignored`. Far edits, consumed items and lamp states must stay after their area unloads.
23. In the browser: drop an item, drive about 10 km out and back, save and reload. The item stays where it was dropped. Check F3 before and after the drive.

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |

## 4. More cars

Run these checks in the section 4 test pass on Tomasz's PC:

1. Re-export, then rebuild. Old exports and saves do not load (V04 changed wheel slots, world bodies and the save schema). V06 added wheel effect, skid, lever, mouse driving and steering wheel records.
2. Run `cargo test --lib item_seat -- --ignored` and `cargo test --lib sled -- --ignored`.
3. Sit on the WC and on each sled seat. Stand up. Check the view limits and the prompt.
4. Sled (V04): slide down a slope, steer, reverse, stop. Check the slide sound follows speed and stops in the air. Load the sled with items above its slide mass: it slides without a rider.
5. Ride a sled away from its building: it must not freeze or vanish. You cannot pick up or store a sled you sit on.
6. Save and load seated on the sled and on the WC.
7. Check F3 near a parked sled and while sliding. Compare with the frame rate rule.
8. Loader (V03 PR 1): New game. The Lada stands at its garage spot. Drive it, use lights, gauges, radio, horn, wheels off and on, cargo in the trunk. Save and load.
9. Run `cargo test --lib two_ladas -- --ignored` and `cargo test --lib installed_lada -- --ignored`.
10. Drive the Lada far from the garage and back: it must not vanish. Check F3 at the start and while driving.
11. Gear lever (V06): the Lada lever moves through each gear in manual and automatic.
12. Wheel effects (V06): dust on sand and none on asphalt, grass trails, rolling, spring and skid sounds, smoke and sand bursts while skidding.
13. Mouse driving (V06): U toggles it in the car. Mouse X steers, mouse up is gas, down is brake, mouse look is off.
14. Steering wheel (V06): with a real wheel or pad, bind every axis and gear button. Check H-shifter on and off and the live values. Steer a sled with the pad.
15. Check F3 before and after, driving on sand at speed. Compare with the frame rate rule.
16. Towing (V05): run `cargo test --lib lada_tows -- --ignored`. Re-export first: prototypes now carry `trailer` and `hitches`.
17. Find a small trailer and a bus trailer. Pick up the grip and put it on the Lada hitch. Drive, turn, reverse and brake.
18. Check the trailer rear and brake lamps follow the Lada lights, ignition and brake. Check the bus trailer brake switch.
19. Pick the grip off the hitch while the car stands and while it rolls. Save and load with the trailer on the hitch.
20. Tow a trailer far from where it spawned and back: car, trailer and grip must not vanish or freeze apart.
21. Check the trailer reflection probe and trailer wheel meshes. Trailer wheel meshes do not turn yet; `PlusWheelToKeepAwake` is not ported.
22. Check F3 while towing. Compare with the frame rate rule.
23. Buses (V03): re-export first: the bus pool, bus gauges and bus doors are new. Run `cargo test --lib v03_bus_pool -- --ignored`.
24. Check F3 at the start and near a bus. Compare with the frame rate rule.
25. Find Bus01, Bus02 and Bus03. Fill a bus with diesel and drive it. With petrol the engine must fail to start.
26. Bus01: both gauge parts in their kmh slots read speed. Check the counters and the gauge lamps.
27. Bus doors: the signal button lights green, the open button swings the leaves and lights red. Check the leaf sounds and the stretching colliders. Buttons do nothing without electronics power.
28. Bus02 trailer: put its grip on the bus hitch. Drive and turn. Its brake switch sets brake force 100 on each wheel. Its rear lamps follow the bus lights and brake.
29. Save and load with bus doors open and signalled and with the trailer on the bus.

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| V02 | Add the other cars. | L | Skoda (with its interior muffling), Lada wagon, two-stroke cars, Car07 automatic, Car08. Roadside cars from building loot with random condition, fluids and missing parts. [B02-B04, B06, B07, L06, C07] |
| V04 | Add bikes. | XL | Motorcycle balance and kick start, pedal bicycle. Builds on the V03 loader. [B12, B13] |

## 5. Later

Run the X05 part 1 test pass on Tomasz's PC:

1. Run `cargo test --lib two_ladas -- --ignored`.
2. Park the Lada with the wheels turned. Save and load: the wheels stay turned.
3. Sleep, wake, save and load: the sleep hint stays off until the cooldown ends.
4. Old saves do not load (save schema 23).

Run the X04b test pass (save slots, autosave) in the same run:

1. Run `cargo test --lib x04b -- --ignored`.
2. Save under two names. Each tile shows a game thumbnail (WebGPU canvas copy), date, seed, km and version.
3. Overwrite, Load and Delete ask Yes/No in game. Load last loads the newest save.
4. Play 2 minutes on foot. The autosave tile appears. Check the F3 frame cost of the autosave frame.
5. Turn on "Load last save at start" and reload the page. The last save loads.
6. Old saves do not load (save schema 24).

Run the X01a test pass (player body) in the same run:

1. Re-export, then rebuild (X01a added seat, drive and `Th` frames).
2. Look down standing, crouching, walking forward and back. Carry a light and a heavy (>10 kg) item. Sit in the Lada (driver and passenger), on the WC and on the sled. Check legs, arms (twist around the bone axis is a known risk) and hands on the wheel.
3. Check you cannot see inside the head.
4. Check the body shadow and the eye shadows. Check the siphon hose shadow follows the hose.
5. ShadowsOnly renderers now cast (#138). Compare F3 shadow draw counts and frame time before and after, near the garage and on the road.
6. Check the F3 cost of the body.

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| X01 | Player body and appearance | XL | Hand and foot reach (car controls, usables, item grips, pointing), ragdoll, wearables, sunglasses, appearance editor, female body. [D02, D03, D05, D06, M10] |
| X02 | Weapons and damage | XL | Damage model, guns, ammo, melee, explosions, hit effects, gibs, wounds, explosive fluids, lamps that break when hit. [G01-G11, E04, C06, I12] |
| X03 | Creatures | XL | AI senses and movement, rabbits, monsters, vultures, sandworm, flyers, airplanes, UFO, EMP. [H01-H11, B17, F21] |
| X04 | Menus, settings, and saves | L | New game options, game modes, homesick mode, settings, rebinding, languages, help text. [M01-M03, M06-M12, M14, M16, I10, L05] |
| X05 | Save every remaining state | XL | World-part wear and colors (`save_partcondition` on generated items) after section 4. Other fields grow with their features. [M13] |
| X06 | Advanced rendering | XL | Live mirrors, lamp shadows, motion blur, TAA, live reflection probes, depth of field, binoculars, heat haze, LUT options, MSAA 8x. Each one must pass the frame rate rule. [K01-K09, K11-K14, F04] |
| X07 | Other items | M | Computer and display. [F07] |
| X08 | Custom radio music | M | Add user music to the radio. [L07] |

Not planned: multiplayer, VR, stereo 3D, mod events, Steam features, developer tools. [N01-N08]
