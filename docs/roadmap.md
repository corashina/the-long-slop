# Project roadmap

Updated: 2026-10-24. Goal: recreate The Long Drive for the web.
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

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| P03 | Finish Lada driving physics. [Vehicle](../src/vehicle.rs), [drivetrain](../src/simulation/drivetrain.rs) | L | Launch, tire grip, bare rims, empty hubs, suspension, wheel spin, combined braking. Verify gears, clutch, and assists. [A04-A07, M15] |
| P04 | Drive outside and return. [Area](../src/starting_area.rs), [world](../src/world.rs) | M | Verify nearby collision, ground seams, re-entry, and loose-object stability. [A08] |
| P05 | Save garage progress. [Save](../src/save/schema.rs), [restore](../src/starting_area/save.rs) | S | Save radio controls. Keep all garage and car state through save, reload, and load. [A09] |
| P06 | Complete player movement. [Player](../src/player) | M | Ladders, head bob on foot and in cars, sleep eligibility, hazard triggers. [D04, D07, D08, E06] |
| P07 | Add player sounds. [Audio](../web/audio.js) | M | Footsteps by surface, prop hit sounds, time-of-day ambience, interior muffling, audio priority limits. [L01, L03, L04, L06, L08] |
| P08 | Complete survival. [Survival](../src/survival.rs) | M | Verify needs, healing, death, alcohol, car-fluid damage, all food values. Save player needs. [E01-E03, E07, F16] |
| P09 | Add portable electronics. [Items](../src/interaction/items.rs) | M | Shared on/off and battery base, then compasses, clock, portable radios, music box. [F01, F02, F05, F06, F08, F20] |
| P10 | Add small item actions. [Items](../src/interaction/items.rs) | M | Spirit level, scale, spray bottle, genie lamp, cigarettes, box contents, fluid containers. [F09, F11, F13-F15, E05] |
| P11 | Accept the starting area | M | Repair the Lada, fill it, start it, drive out, return, save, reload, and continue with normal input. [A11] |

## 2. Car parts and repair

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| R01 | Complete repair rules. [Conditions](../src/conditions.rs), [parts](../src/interaction/parts.rs) | M | Repair values per condition type, covered-part rules, cross-fitting where the original allows it. [C01-C03] |
| R02 | Complete electronics. | M | Battery and alternator load for every powered part. Verify coolant fan and thermostat. [C08, C09] |
| R03 | Add the siphon hose. [Fluids](../src/fluids.rs) | L | Attach to caps, coil, and move fluid between tanks. [C04] |
| R04 | Add fire and burning fluids. | M | Fire and embers, burnt-fluid smoke color. [F17, C05] |
| R05 | Add cargo storage. [Storage](../src/interaction/storage.rs) | XL | Lock items to car beds and shelves. Keep mass, collision, and saved state. [F12] |

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
| V02 | Add the other cars. | L | Skoda, Lada wagon, two-stroke cars, Car07 automatic, Car08. [B02-B04, B06, B07] |
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
| X07 | Other items | M | Computer and display, speed-writing device, paintings, holiday items. [F07, F18, F19, F22] |
| X08 | Custom radio music | M | Add user music to the radio. [L07] |

Not planned: multiplayer, VR, stereo 3D, mod events, Steam features, developer tools. [N01-N08]
