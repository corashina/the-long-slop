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
- W6: `tp_w6_building_keeps_the_pole_with_the_nearest_target`. A building links to the pole whose target 0 is nearest, not the pole with the nearest target (`roadGenScript.CheckCableConnections`).
- W12: `tp_w12_dig_up_refuse_pickup_and_save`. A buried item loads with a dynamic body. The source freezes it.
- W13: `tp_w13_detector_beeps_near_metal_and_saves_on`. A metal detector saved on loads off.
- W15, SA2: `w02_long_trip_keeps_far_edits`, `w02_garage_can_returns_after_far_trip`. The required publication stalls after the car moves out.
- MC2 on the road: `tp_sa1_fill_start_drive_return_save_reload`. The Lada on the road, engine running in gear 2 at 2427 rpm, moves 0 m. MC2 is not only the garage floor.
- CP17: `tp_cp17_loot_tool_state_saves_and_streams`. After save and load, a loot spray can has 100 paint again and its default color.
- CP12: `tp_cp12_roadside_part_keeps_paint_and_repair`. Paint on a placed bus part stays live and after a 3 km move. Then save fails with "Invalid source integer". A save before the paint works.
- WD19: `tp_wd19_cactus_breaks_once_and_stays`. A car hit, a shot and a blast break cacti once, with one gib each, and they stay. After an 8 km move the broken cactus is still bound, so its area does not stream in again.
- CP15: `tp_cp13_cp16_loot_tools_work_from_the_hand`. The loot spray paints, shows its color, sprays particles and runs out, but plays no sound. Its prototype tool has no `voice`, so the spray signal never plays and its tool loop never starts.
- WD2: `tp_wd2_shovel_breaks_a_building_lamp`. The shovel breaks the lamps and they stay dark. Save and load at the Mansion: the restore stays pending.
- CR2: `tp_cr2_halloween_bats_near_buildings`. On Halloween, the area near the 2nd road building (about 14.6 km out) fails with "Generated condition: state". The same area loads in June.
- W17: `tp_w17_far_move_to_the_first_gas_station_loads`. A far move to the first gas station on the road (about 7.7 km out) never loads: the road plan does not finish.
- CP5, CP6: `r05_cargo_locks_in_trunk`. The driver cannot exit the Lada in the garage (BL), so the cargo checks do not run.
- CP7: `tp_cp7_turned_wheels_stay_after_save_and_load`. The seated driver holds steer left, but the garage Lada steer stays 0.
- CP8, CP9, CP11: `lada_part_paint_and_repair_change_its_appearance_and_save`. Paint on a Lada part is not a world change, so it does not save.
- WD1: `tp_wd1_bottle_vase_and_box_keep_soft_drops_break_on_hard_ones_and_stay_broken`. After a bottle breaks, save fails with "Removed or missing attachment owner".
- WD3: `tp_wd3_each_melee_weapon_swings_with_each_key_and_breaks_a_bottle`. The export has melee weapon records only for the crowbar and the shovel. The katana, fly swatter and leg have none.
- WD14, WD15, CC14: `tp_wd14_a_shot_fuel_can_explodes_breaks_a_bottle_sets_off_a_second_and_stays_gone`, `tp_wd15_the_shot_garage_can_explodes_and_stays_gone`, `tp_cc14_a_tank_blast_next_to_a_car_knocks_parts_off`. The frame after a tank blast panics at `save/world.rs:408` "no entry found for key".
- WD20: `tp_wd20_a_hard_hit_breaks_a_tumbleweed_with_its_gib_and_it_stays_gone`. A broken tumbleweed spawns no gib.

Blocked checks. Each waits for the fix of its bug. Run them in the next build's test pass.

#### MC2: the garage Lada does not drive

- SA1: Play the garage loop in the browser: repair, fill, start, drive out, return, save, reload.
- SA2: Check the 20 L can stays on the ground after drive-out and return.
- SA10: Drive the Lada and check that parked cars and dropped items still settle and sleep.
- CP4: Siphon hose: first check the garage hose spawns clear of the walls. Attach it to the Lada tank and a can. Siphon by height, drain with LMB, coil with R, store it, tear it off by driving.
- CP12: Paint and brush a roadside car part. Drive away until it unloads, come back: the part keeps its look. Save and load: it still keeps it.
- CP17: Save and load. Loot tool paint left, color and wear stay. Drive away and back: they stay.
- W15: Drop an item, drive about 10 km out and back, save and reload. The item stays where it was dropped.
- AR3: Drive the Lada at speed. The dashboard, wheel and hands stay sharp. The road and terrain behind them have no ghost copies.
- AR12: Park three cars near the garage. Each car body reflects its own surroundings, and the reflections update as the cars and the sun move.

#### BL: the player cannot exit the car in the garage

- PB1: Look down standing, crouching, walking forward and back. Carry a light and a heavy (>10 kg) item. Sit in the Lada (driver and passenger), on the WC and on the sled. Check legs, arms and hands on the wheel.

#### Appearance camera: the preview shows only the legs

- PB16: Open Appearance from the start menu and the pause menu. The preview figure shows next to the panel with the export camera and lights. Check the figure pose, the lights and the colors. Changes show at once.
- PB17: Set Male and Female with custom colors and disabled parts. Check them in game, in the shadow and on the ragdoll. Check the submesh colors of each segment.

#### Guns from equip(): sandbox guns do not load

- CC14: Blow up a tank next to a car. Parts fall off.
- WD8: Reload and unload with R and E from carried ammo. Round totals stay the same.
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
15. Paint a car part with loot spray paint. The can shows its color, sprays particles with sound, and runs out.

### World and terrain

6. Power cables: check poles link to the next pole on curves and tilted poles, road buildings and the home link to a pole, and the rope meshes are not flipped.
12. Dig a buried item up with the shovel. It must lift, become loose, and then allow pickup. A buried item must refuse pickup. Save and load with an item buried and one dug up.
13. Metal detector: turn it on and off with use. Check the beep pitch and volume and the needle near metal, buried metal, and behind a wall. Save and load with it on.
17. Move far in one step (sandbox building('Benzinkut01'), or load a far save). Wait while the area loads. The game does not stop with "validated authored material choice".

### Player body and outfit

6. Use doors and handles. Push a car from a seat.
8. Hold Left Alt to rub dirt off a car and to wash a wash trigger.
14. Worn items are hidden in first person but cast shadows. The worn pumpkin inner mesh stays visible, as in the source.
21. The dark lens (profile 0) dims the view by 2 stops.

### Weapons and damage

1. Drop and throw beer bottles, the vase and a box. Soft drops keep them whole. Hard hits break them with pieces and a break sound. The pieces scatter and fade.
2. Hit a building lamp with the shovel. It goes dark and stays dark when switched. Save and load: it stays broken.
3. Swing the katana, crowbar, fly swatter and leg with each attack key. Each plays its own animation and sounds and breaks a bottle. The hands follow the weapon grips.
14. Shoot a world fuel can or tank with gas, oil or diesel over 40%. It explodes with particles and sound, breaks nearby bottles, pushes items and sets off a second tank. Save and load: the exploded items stay gone.

### Creatures

2. Bats: set the PC date between 15 Oct and 15 Nov, start a new game and drive to a gas station. In 65% of stations, 5 to 30 bats fly near it, flap and squeak. Pick one up: it keeps squeaking and does not fly out of the hand. Save and load near the bats.
