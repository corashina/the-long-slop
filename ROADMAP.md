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

Known failures on master e927770 (2026-10-31 headless run, export ee6554d), fix before the next run:

- K1 (browser): Appearance preview shows only the legs and shoes. The camera framing is wrong for Male and Female.
- K2 (browser): Looking down shows hollow shirt sleeves, floating hands and hollow pant legs. It also shows at normal pitch while walking.
- K4 (browser): First page load can stall at the start of object loading with dependency staging full (7.7/7.8 MiB). A reload clears it.
- K5 (browser): In game, pause > Load > Load last once stayed on "Loading..." for over 6 minutes, with no console messages. The simulation and the save bridge finish the load (`tp_k5_s1_a_stored_save_starts_the_session_and_its_restore_finishes`), so the hang is in the browser.
- 32 of 169 ignored simulation tests fail in release. Debug overflows the stack. The lines below name each one.
- MC2 on the road: `tp_sa1_fill_start_drive_return_save_reload`. The Lada on the road, engine running in gear 2 at 2427 rpm, moves 0 m. MC2 is not only the garage floor.
- W15: `w02_long_trip_keeps_far_edits` is flaky. In 2 of 3 runs, 1 or 2 of 52 revisited building items moved.
- WD19: `tp_wd19_cactus_breaks_once_and_stays`. A car hit, a shot and a blast break cacti once, with one gib each, and they stay. After an 8 km move the broken cactus stayed bound, because a structure unload did not rescan the breakables. Fixed in code (#WD): the rescan now follows structure changes. Not run yet: run it after the next export.
- CP15: `tp_cp13_cp16_loot_tools_work_from_the_hand`. The loot spray played no sound: its signal waited for a fixed-tool `voice`, which a prototype tool does not have. Fixed in code (#WD): an emitting spray plays its prototype `sound` loop. Not run yet: run it after the next export.
- WD2: `tp_wd2_shovel_breaks_a_building_lamp`. The shovel breaks the lamps and they stay dark. Save and load at the Mansion: the restore stays pending.
- CP5, CP6: `r05_cargo_locks_in_trunk`. The driver cannot exit the Lada in the garage (BL), so the cargo checks do not run.
- CP7: `tp_cp7_turned_wheels_stay_after_save_and_load`. The seated driver holds steer left, but the garage Lada steer stays 0.
- WD3: `tp_wd3_each_melee_weapon_swings_with_each_key_and_breaks_a_bottle`. The export has melee weapon records only for the crowbar and the shovel. The katana, fly swatter and leg have none. The exporter skips a `meleeweaponscript` without a pickupable (the leg kick), and finds weapons only in selected world prototypes. Check the katana and fly swatter exclusion reasons in the export catalog.
- WD20: `tp_wd20_a_hard_hit_breaks_a_tumbleweed_with_its_gib_and_it_stays_gone`. The tumbleweed gib prototype holds only a particle system: no body, pickup or renderer. So the gib spawns no item, and the test finds none. The export has no gib particle data. `tumbleweed_breaks_on_a_hard_drop_and_leaves_the_spawned_list` fails the same way.
- No test yet (needs an export to write and run): CP2 loot radiator fan, PB6 doors and push from a seat, PB8 rub and wash, WD8 reload from carried ammo.
- MC2, older tests: `garage_start_car_drives_in_automatic`, `car07_drives_in_forced_automatic`, `start_cars_drive_save_and_load`, `two_ladas_drive_save_and_load`, `lada_tows_the_trailer_and_the_hitch_saves`. The car or trailer does not move.
- BL, older test: `start_car_driver_exits_in_garage` finds no clear exit.
- Stale test data: these tests do not find their fixture near the start (bottle, lamp trigger, buried item, natural item, bike prototype): `bottle_breaks_on_a_hard_drop_once_and_stays_removed_after_reload`, `shovel_broken_lamp_stays_broken_after_reload`, `revolver_shot_breaks_a_bottle`, `katana_swing_breaks_a_bottle`, `shotgun_pellets_push_an_item`, `x02_leftovers_smg_unload_drops_the_magazine`, `crowbar_hit_does_not_lift_a_buried_item`, `roadside_possession_survives_owner_retirement_without_duplicate_spawn`, `bike03_pedals_then_kick_starts`.
- `bike01_kick_starts_and_rides_upright`: panic "validated neutral" at `simulation/drivetrain.rs:149`.
- `a_blast_sets_off_a_second_tank`: the second tank is not removed. `a_shot_tank_explodes_and_breaks_a_nearby_bottle`: the blast does not break the bottle. The `tp_wd14` and `tp_cc14` tests with the full 20 L can pass.
- `fixed_decoration_possessions_restore_on_item_path`: the restored item is not an interaction part.
- `fixed_decorations_preserve_collision_without_item_bindings`: "Missing source rule wrong.assets:84448".
- `road_building_cable_is_in_building_appearance`: no building cables.
- `required_publication_reaches_ready_and_withholds_dependencies`: the world is not supported.

Blocked checks. Each waits for the fix of its bug. Run them in the next build's test pass.

#### MC2: the Lada does not drive

- SA1: Play the garage loop in the browser: repair, fill, start, drive out, return, save, reload.
- SA10: Drive the Lada and check that parked cars and dropped items still settle and sleep.
- CP4: Siphon hose: first check the garage hose spawns clear of the walls. Attach it to the Lada tank and a can. Siphon by height, drain with LMB, coil with R, store it, tear it off by driving.
- W15: Drop an item, drive about 10 km out and back, save and reload. The item stays where it was dropped.

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
15. Paint a car part with loot spray paint. The can shows its color, sprays particles with sound, and runs out.

### Player body and outfit

6. Use doors and handles. Push a car from a seat.
8. Hold Left Alt to rub dirt off a car and to wash a wash trigger.

### Weapons and damage

2. Hit a building lamp with the shovel. It goes dark and stays dark when switched. Save and load: it stays broken.
3. Swing the katana, crowbar, fly swatter and leg with each attack key. Each plays its own animation and sounds and breaks a bottle. The hands follow the weapon grips.
8. Reload and unload with R and E from carried ammo. Round totals stay the same.
19. Drive into a cactus, shoot one until it breaks, and blow one up. Cactus pieces fly off and the cactus stays, as in the original. It does not break again until its area streams in again.
20. Hit a tumbleweed hard with the car, a shot and a melee swing. It breaks with its gib and sound. Save and load: it stays gone.
