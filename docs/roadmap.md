# Project roadmap

Updated: 2026-10-25. Goal: recreate The Long Drive for the web.
Behavior authority: Steam build `11140890` and its decompiled code in `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs`.
Save format authority: [save schema](../src/save/schema.rs).

## Frame rate rule

The game runs at about 60 FPS now. Every new feature must keep it there.
Check the F3 overlay before and after each change. Report the frame-time cost.
If a feature drops the frame rate, fix the cost before merge or agree a cut.

## Task rules

Work section by section, in the order below. Keep the game playable after each task.
Recheck the code and the original before each task. Complete missing behavior and verify existing behavior.
Follow [AGENTS.md](../AGENTS.md) for checks.
Measure original driving before changing tire, rim, hub, suspension, or brake physics. See [G11 measurements](g11-driving-measurements.md).
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
6. Siphon hose (R03): first check the garage hose spawns clear of the walls. Attach it to the Lada tank and a can. Siphon by height, drain with LMB, coil with R, store it, tear it off by driving.
7. Cargo storage (R05): lock items in the Lada trunk, glove box, and engine bay, and in a box, a fridge, a pallet, and a coffin. Open the lid to release them.
8. Save and load with the hose attached and with cargo locked.
9. Check F3 with smoke, near a hose, and while driving with cargo. Compare with the frame rate rule.

## 3. World and terrain

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| W01 | Restore world range. [World](../src/world.rs) | M | Raise draw distance and item spawn distance toward the original within the frame rate rule. Verify freeze distances. [I01-I03] |
| W02 | Complete long-distance travel. | XL | Distant revisits, memory limits, far object records, and their saved state. [I04] |
| W03 | Add biomes. [Terrain](../crates/terrain) | M | Biome selection, grass and winter terrain, edge of the world, winter color. [I05, I06, K10] |
| W04 | Expand buildings. [Generation](../src/world/generation.rs) | L | Villages, big stops, maze, power cables, breakable lamps, random lights and doors. [I07-I09, I11-I13] |
| W05 | Complete world population. | L | Loot rules for every building type, roadside car condition, ambience zones. [I14, C07, L02] |
| W06 | Add digging and detection. | L | Shovel holes, buried items, metal detector. [F10, F03] |
| W07 | Add weather. | L | Sandstorms and their saved state, tumbleweeds, snowfall in the winter biome. [J01-J04] |
| W08 | Add grass tracks. [Grass](../web/grass.js) | M | Grass painter and regrowth after tracks. [I15] |

## 4. More cars

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| V01 | Add a general vehicle loader. [Vehicles](vehicles.md) | L | One export record per car, per-car instruments and lights. Needed by V02-V06. [B01, B19] |
| V02 | Add the other cars. | L | Skoda (with its interior muffling), Lada wagon, two-stroke cars, Car07 automatic, Car08. [B02-B04, B06, B07, L06] |
| V03 | Add buses. | XL | Diesel rules, three buses, doors, door lamps, bus trailer. [B05, B08-B11] |
| V04 | Add bikes. | XL | Motorcycle balance and kick start, pedal bicycle, sled. [B12-B14] |
| V05 | Add towing. | L | Trailers, hitch, trailer lights and brakes, rope towing. [B15, B16] |
| V06 | Complete driving input and effects. | M | Gear stick animation, mouse driving, wheel and joystick axes, wheel dust and ground sounds. [B18, B20-B22] |

## 5. Later

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| X01 | Player body and appearance | XL | Body model, hand and foot IK, ragdoll, wearables, sunglasses, appearance editor. [D01-D03, D05, D06, M10] |
| X02 | Weapons and damage | XL | Damage model, guns, ammo, melee, explosions, hit effects, gibs, wounds, explosive fluids. [G01-G11, E04, C06] |
| X03 | Creatures | XL | AI senses and movement, rabbits, monsters, vultures, sandworm, flyers, airplanes, UFO, EMP. [H01-H11, B17, F21] |
| X04 | Menus, settings, and saves | L | New game options, game modes, homesick mode, save slots, autosave, settings, rebinding, languages, help text. [M01-M12, M14, M16, I10, L05] |
| X05 | Save every remaining state | XL | Grow the save with each feature: items, buildings, wear, colors. [M13] |
| X06 | Advanced rendering | XL | Live mirrors, lamp shadows, motion blur, TAA, live reflection probes, depth of field, binoculars, heat haze, LUT options, MSAA 8x. Each one must pass the frame rate rule. [K01-K09, K11-K14, F04] |
| X07 | Other items | M | Computer and display, speed-writing device, paintings, holiday items. Cigarette ember, smoke and burn-down; spray bottle start-area copies, refill and prompt text; genie lamp glow; cleaned wash triggers turn their object off. [F07, F18, F19, F22, E05, F13, F14] |
| X08 | Custom radio music | M | Add user music to the radio. [L07] |

Not planned: multiplayer, VR, stereo 3D, mod events, Steam features, developer tools. [N01-N08]
