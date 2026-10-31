# Project roadmap

Goal: recreate The Long Drive for the web.
Behavior authority: Steam build `11140890` and its decompiled code in `.cache/reference-library/records/managed/Assembly-CSharp.cs`.
Save format authority: [save schema](crates/simulation/src/save/schema.rs).

## Frame rate rule

The game runs at about 60 FPS now. Every new feature must keep it there.
Check the F3 overlay before and after each change. Report the frame-time cost.
If a feature drops the frame rate, fix the cost before merge or agree a cut.

## Task rules

Keep the game playable after each task.
Recheck the code and the original before each task. Complete missing behavior and verify existing behavior.
Remove a row when its task is done. Do not mark rows done.
Add the manual checks of each task to the test pass below. Remove them after they pass.

Size: S is up to a day. M is a few days. L is about a week. XL is several weeks.

## Open tasks

The starting area, car parts, world and more cars sections are done. Only their test pass remains.

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| X03 | Creatures | XL | Ground UFO, EMP and green-eyed evil rabbits, human monster (attacks hurt the player), breakablescript `DamageFrom` for AI. Rabbits do not yet sense the player hum. |
| MC2 | Garage car does not move | M | A Volna chassis collider face (world z -70.28, 1.3 m left of the center line, 5 m long) sits 0.15 m into the garage floor and holds the car. Name it with `garage_start_car_drives_in_automatic` (it prints the chassis colliders on failure), then fix its geometry or layer. |
| X06c | Building reflection probes | S | Car probes are live. Check the original building prefabs for realtime `ReflectionProbe`s. Export any as prototype probes. The live probe slots then take them. |

### Multiplayer

Match the source Steam multiplayer (`sns`, `snl`, `snTempPlayerSync`). WebRTC peer to peer through PeerJS. The host browser runs the world. Guests talk only to the host. The host forwards.

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| M01 | Join and see each other | S | Remote player collision: export the `mpplayerscript` capsule (`minBodyRadius`, `maxBodyRadius`, head and prone head nodes), add a kinematic capsule per figure, 3 s spawn delay and 1 s after standing up. |
| M02 | Shared cars | L | Driver input sync (`carfloats`, gears, ignition, lamps, handbrake), passengers. Seated figures follow the seat of the car by save id (source `UpdPos` sitting). |
| M03 | Shared items and world | XL | `claim`, item poses and velocity, inventory, tanks and caps, parts, usables, attachables, food, weapons. |
| M04 | Everything else | XL | Creatures, time and sleep, weather, radio channels, sounds, voice chat. |

Not planned: VR, stereo 3D, mod events, Steam features.

## Test pass

Run these checks on Tomasz's PC in one run. Passed checks are removed.
Check F3 in each scenario below. Compare with the frame rate rule.
Look, listen and F3-only checks are on Tomasz's eyes list (project file `plans/tp-eyes-list.md`), not here.

Known failures on master c0da737f (2026-10-30 browser retest), fix before the next run:

- Appearance preview shows only the legs and shoes. The camera framing is wrong for Male and Female.
- Looking down shows hollow shirt sleeves, floating hands and hollow pant legs. It also shows at normal pitch while walking.
- K3: `tp_k3_e_on_the_outside_door_handle_opens_the_door`. Outside the Lada, the use ray takes the door and the door action shows, but E does not turn the door.
- First page load can stall at the start of object loading with dependency staging full (7.7/7.8 MiB). A reload clears it.
- In game, pause > Load > Load last once stayed on "Loading..." for over 6 minutes, with no console messages. The simulation and the save bridge finish the load (`tp_k5_s1_a_stored_save_starts_the_session_and_its_restore_finishes`), so the hang is in the browser.
- 29 of 90 ignored simulation tests fail in release: MC2, BL, stale test data. Debug overflows the stack.
- W15: `w02_long_trip_keeps_far_edits`. No loose roadside item lies within 60 m of a road stop, so no far edit is made. `w02_garage_can_returns_after_far_trip`: the dropped can is not active again after the return.
- CR2: `tp_cr2_halloween_bats_near_buildings`. On Halloween the first three road buildings load, but none has bats. The source spawns them at each building with a 65% chance.
- CP5, CP6: `r05_cargo_locks_in_trunk`. The driver cannot exit the Lada in the garage (BL), so the cargo checks do not run.
- CP7: `tp_cp7_turned_wheels_stay_after_save_and_load`. The seated driver holds steer left, but the garage Lada steer stays 0.
- CP8, CP9, CP11: `lada_part_paint_and_repair_change_its_appearance_and_save`. Paint on a Lada part is not a world change, so it does not save.
- WD1: `tp_wd1_bottle_vase_and_box_keep_soft_drops_break_on_hard_ones_and_stay_broken`. After a bottle breaks, save fails with "Removed or missing attachment owner".
- WD3: `tp_wd3_each_melee_weapon_swings_with_each_key_and_breaks_a_bottle`. The export has melee weapon records only for the crowbar and the shovel. The katana, fly swatter and leg have none.
- WD14, WD15, CC14: `tp_wd14_a_shot_fuel_can_explodes_breaks_a_bottle_sets_off_a_second_and_stays_gone`, `tp_wd15_the_shot_garage_can_explodes_and_stays_gone`, `tp_cc14_a_tank_blast_next_to_a_car_knocks_parts_off`. The blast does not break a nearby bottle or set off a second tank, and knocks no car parts off. After the garage can explodes, save fails with "Fixed source attachment changed".
- WD20: `tp_wd20_a_hard_hit_breaks_a_tumbleweed_with_its_gib_and_it_stays_gone`. A broken tumbleweed spawns no gib.

Blocked checks. Each waits for the fix of its bug. Run them in the next build's test pass.

#### MC2: the garage Lada does not drive

- SA1: Play the garage loop in the browser: repair, fill, start, drive out, return, save, reload.
- SA2: Check the 20 L can stays on the ground after drive-out and return.
- SA10: Garage rest: the garage barrel and the wire brush stay still. Drive the Lada and check that parked cars and dropped items still settle and sleep.
- CP4: Siphon hose: first check the garage hose spawns clear of the walls. Attach it to the Lada tank and a can. Siphon by height, drain with LMB, coil with R, store it, tear it off by driving.
- CP10: Repair a tire. The HUD condition and the wheel grip follow it.
- CP12: Paint and brush a roadside car part. Drive away until it unloads, come back: the part keeps its look. Save and load: it still keeps it.
- CP17: Save and load. Loot tool paint left, color and wear stay. Drive away and back: they stay.
- W15: Drop an item, drive about 10 km out and back, save and reload. The item stays where it was dropped.
- PB5: Drive the Lada. Use the horn, lights, ignition (tap and hold), handbrake and gear stick: a hand moves to each. Press gas, brake and clutch: the feet move to the pedals.
- AR3: Drive the Lada at speed. The dashboard, wheel and hands stay sharp. The road and terrain behind them have no ghost copies.
- AR12: Park three cars near the garage. Each car body reflects its own surroundings, and the reflections update as the cars and the sun move.

#### BL: the player cannot exit the car in the garage

- PB1: Look down standing, crouching, walking forward and back. Carry a light and a heavy (>10 kg) item. Sit in the Lada (driver and passenger), on the WC and on the sled. Check legs, arms and hands on the wheel.

#### Appearance camera: the preview shows only the legs

- PB16: Open Appearance from the start menu and the pause menu. The preview figure shows next to the panel with the export camera and lights. Check the figure pose, the lights and the colors. Changes show at once.
- PB17: Set Male and Female with custom colors and disabled parts. Check them in game, in the shadow and on the ragdoll. Check the submesh colors of each segment.

#### Guns from equip(): sandbox guns do not load

- CC14: Blow up a tank next to a car. Parts fall off.
- WD8: Reload and unload with R and E from aimed or carried ammo. Round totals stay the same. The left hand moves to the gun.
- WD15: Shoot the full 20 L garage fuel can. It explodes, and the can and its cap go away. Save and load: they stay gone.
- WD19: Drive into a cactus, shoot one until it breaks, and blow one up. Cactus pieces fly off and the cactus stays, as in the original. It does not break again until its area streams in again.
- WD20: Hit a tumbleweed hard with the car, a shot and a melee swing. It breaks with its gib and sound. Save and load: it stays gone.

Prepare once:

1. Pull master. Re-export, then make a release build. Old exports and old saves do not load. Only the current save schema loads.
2. Paste the acquisition hash that the export prints into `CONTRACT_SHA256` if it changed.
3. Run all ignored tests: `cargo test --release --lib -- --ignored --nocapture`.
4. In that output, note the `w02_` bytes per km and the 5,000 km estimate. Above 200 MB, compact spawn boundaries.
5. Claude runs the steps in Chrome with the test sandbox: see `tests/sandbox/README.md`. First check the sandbox itself: `spawn('Bus02Full', 10)` places a bus, and `teleport`, `building`, `spawn`, `hand`, `fill` and `cycle` work. Use `tests/sandbox/buses.js` for More cars steps 16-20.

### Car parts and repair

2. Swap a loot radiator into the Lada: its fan spins while the engine runs.
5. Cargo storage: lock items in the Lada trunk, glove box, and engine bay, and in a fridge, a pallet, and a coffin. Open the lid to release them. The source box is manual only: items in it when its area loads lock, items put in it later stay loose. Items in a fridge, pallet or coffin when its area loads lock at once, not after 3 s.
6. Save and load with the hose attached and with cargo locked.
7. Park the Lada with the wheels turned. Save and load: the wheels stay turned.
8. Brush rust off a Lada part and a roadside car part. Rub dirt off with the sponge. The material changes at once.
9. Paint a Lada door and a loose part with spray paint. The color changes while you spray.
11. A state change binds a new material. Check the part does not blink for a frame.
13. Find a loot wire brush, sponge and spray paint. Pick up each one and hold LMB to use it.
14. Clean rust and dirt with the loot wire brush and sponge. The brush model moves on the surface, with particles and sound.
15. Paint a car part with loot spray paint. The can shows its color, sprays particles with sound, and runs out.
16. Use a loot brush until it wears out. Its scrub part hides and it stops working.

### World and terrain

5. Sandstorms: spawn a storm near the player with sandbox `storm(50)`. Check sky, fog, light, sound and dust, and that dust stops inside buildings and cars. Check that wind does not move items inside a car. Check the F3 storm line.
6. Power cables: check poles link to the next pole on curves and tilted poles, road buildings and the home link to a pole, and the rope meshes are not flipped.
12. Dig a buried item up with the shovel. It must lift, become loose, and then allow pickup. A buried item must refuse pickup. Save and load with an item buried and one dug up.
13. Metal detector: turn it on and off with use. Check the beep pitch and volume and the needle near metal, buried metal, and behind a wall. Save and load with it on.
17. Move far in one step (sandbox building('Benzinkut01'), or load a far save). Wait while the area loads. The game does not stop with "validated authored material choice".

### Player body and outfit

6. Use doors and handles. Push a car on foot and from a seat.
7. Carry a two-hand item.
8. Hold Left Alt: the hand points at the aimed surface. Rub dirt off a car and wash a wash trigger. Hold Left Alt at the sky: the hand waves.
14. Worn items are hidden in first person but cast shadows. The worn pumpkin inner mesh stays visible, as in the source.
21. The dark lens (profile 0) dims the view by 2 stops.
23. Lenses with the flare flag hide the sun flare. Other lenses keep it.
25. Take the glasses off: the tint clears. Die with them on: the tint clears and the death effect fades in.
26. Save and load with the glasses worn: the tint comes back. Read the console for a warning about more than one active lens.

### Weapons and damage

1. Drop and throw beer bottles, the vase and a box. Soft drops keep them whole. Hard hits break them with pieces and a break sound. The pieces scatter and fade.
2. Hit a building lamp with the shovel. It goes dark and stays dark when switched. Save and load: it stays broken.
3. Swing the katana, crowbar, fly swatter and leg with each attack key. Each plays its own animation and sounds and breaks a bottle. The hands follow the weapon grips.
5. Hit a sausage with the shovel or a hurtFood weapon: it splits into pieces. Save and load: the pieces keep their bites.
13. Shoot metal, wood, sand, glass and flesh: bullet holes show with the right texture, face out, move with items and fade per lifetime. Check the first hole for a hitch.
14. Shoot a world fuel can or tank with gas, oil or diesel over 40%. It explodes with particles and sound, breaks nearby bottles, pushes items and sets off a second tank. Save and load: the exploded items stay gone.

### Creatures

2. Bats: set the PC date between 15 Oct and 15 Nov, start a new game and drive to a gas station. In 65% of stations, 5 to 30 bats fly near it, flap and squeak. Pick one up: it keeps squeaking and does not fly out of the hand. Save and load near the bats.
10. A bite of a big evil rabbit damages loose items and other rabbits near the bite.
12. Start a car engine near rabbits. Normal rabbits flee from it.
