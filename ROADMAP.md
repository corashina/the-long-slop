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

Known failures on master c0da737f (2026-10-30 browser retest), fix before the next run:

- Appearance preview shows only the legs and shoes. The camera framing is wrong for Male and Female.
- Looking down shows hollow shirt sleeves, floating hands and hollow pant legs. It also shows at normal pitch while walking.
- Outside the Lada, E on the door handle does not open it. The HUD shows "[LMB]+[Mouse]: Move | [F]: Dismount".
- First page load can stall at the start of object loading with dependency staging full (7.7/7.8 MiB). A reload clears it.
- In game, pause > Load > Load last once stayed on "Loading..." for over 6 minutes, with no console messages.
- 30 of 91 ignored simulation tests fail in release: MC2, BL, stale test data. Debug overflows the stack.
- SA4: `hazard_trigger_damages_player_until_death` fails. The hazard trigger does not kill the player.
- W6: `tp_w6_building_keeps_the_pole_with_the_nearest_target`. A building links to the pole whose target 0 is nearest, not the pole with the nearest target (`roadGenScript.CheckCableConnections`).
- W12: `tp_w12_dig_up_refuse_pickup_and_save`. A buried item loads with a dynamic body. The source freezes it.
- W13: `tp_w13_detector_beeps_near_metal_and_saves_on`. A metal detector saved on loads off.
- W15: `w02_long_trip_keeps_far_edits`. The required publication stalls after the car moves out.
- CR2: `tp_cr2_halloween_bats_near_buildings`. On Halloween, the area near the 2nd road building (about 14.6 km out) fails with "Generated condition: state". The same area loads in June.
- W17: `tp_w17_far_move_to_the_first_gas_station_loads`. A far move to the first gas station on the road (about 7.7 km out) never loads: the road plan does not finish.

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
- PB9: Die on foot and in a moving car. The body falls with joint limits and carries the car speed.
- AR3: Drive the Lada at speed. The dashboard, wheel and hands stay sharp. The road and terrain behind them have no ghost copies.
- AR12: Park three cars near the garage. Each car body reflects its own surroundings, and the reflections update as the cars and the sun move.

#### BL: the player cannot exit the car in the garage

- PB1: Look down standing, crouching, walking forward and back. Carry a light and a heavy (>10 kg) item. Sit in the Lada (driver and passenger), on the WC and on the sled. Check legs, arms and hands on the wheel.

#### S3: Load last hangs

- S1: Load, Load last and New game after death do not fail with "Asset CPU budget exceeded". Reload the page, then Load last from the start menu: the game starts, no "Source building metadata exceeds prototype CPU budget".
- S2: Turn on "Load last save at start" and reload the page. The last save loads.

#### Appearance camera: the preview shows only the legs

- PB16: Open Appearance from the start menu and the pause menu. The preview figure shows next to the panel with the export camera and lights. Check the figure pose, the lights and the colors. Changes show at once.
- PB17: Set Male and Female with custom colors and disabled parts. Check them in game, in the shadow and on the ragdoll. Check the submesh colors of each segment.

#### Guns from equip(): sandbox guns do not load

- CC14: Blow up a tank next to a car. Parts fall off.
- WD8: Reload and unload with R and E from aimed or carried ammo. Round totals stay the same. The left hand moves to the gun.
- WD9: Shotguns and BB gun: hold R and move the mouse to break the barrel. Closed barrels fire; open barrels reload. The BB gun gains power.
- WD10: Revolver: hold R and scroll to turn the cylinder. Empty chambers click. Bullets show in full chambers.
- WD11: SMG: load from a magazine item. No magazine: no fire. Save and load keeps ammo, barrel, chambers, power and magazine.
- WD12: SMG: unload with E at loose ammo, or with R while you hold ammo. A magazine drops at the gun with the gun rounds, and the gun is empty. Load it back with R. Load a held magazine into a loaded SMG: the old magazine drops. Save and load: a dropped magazine keeps its rounds.
- WD15: Shoot the full 20 L garage fuel can. It explodes, and the can and its cap go away. Save and load: they stay gone.
- WD17: Shoot a tank that does not explode, and the garage water tank, with a shotgun. Holes that face down leak, and the fluid amount falls.
- WD19: Drive into a cactus, shoot one until it breaks, and blow one up. Cactus pieces fly off and the cactus stays, as in the original. It does not break again until its area streams in again.
- WD20: Hit a tumbleweed hard with the car, a shot and a melee swing. It breaks with its gib and sound. Save and load: it stays gone.

Prepare once:

1. Pull master. Re-export, then make a release build. Old exports and old saves do not load. Only the current save schema loads.
2. Paste the acquisition hash that the export prints into `CONTRACT_SHA256` if it changed.
3. Run all ignored tests: `cargo test --release --lib -- --ignored --nocapture`.
4. In that output, note the `w02_` bytes per km and the 5,000 km estimate. Above 200 MB, compact spawn boundaries.
5. Claude runs the steps in Chrome with the test sandbox: see `tests/sandbox/README.md`. First check the sandbox itself: `spawn('Bus02Full', 10)` places a bus, and `teleport`, `building`, `spawn`, `hand`, `fill` and `cycle` work. Use `tests/sandbox/buses.js` for More cars steps 16-20.

### Starting area

3. Check paint spray and sponge particles: refraction and soft edges.
4. Check hazards: a hazard trigger hurts the player until death.
5. Listen to player sounds: hits, footsteps, ambience, muffling in buildings.
6. Listen to the music box and the radios. The compass and clock needles look right.
7. Crosshair icons: look at a car door, a sliding and a vertical sliding usable, a light switch, a radio knob, a tuneable cap and a seat. Push the Lada, hold the hose at a cap and a trailer grip at the hitch, fill a can, carry a heavy item. Each case shows the source images together. No name or condition line shows above the crosshair.
8. Binoculars: the lens view shows the black two-circle mask. Try it with the binoculars equipped too.

### Car parts and repair

1. Repair: swap the hood and the trunk lid, repair rust and dirt.
2. Electronics: check the Lada radiator fan spins while the engine runs, and stops with the engine. Swap in a loot radiator: its fan spins too.
3. Exhaust smoke: rate follows rpm, color changes after a fuel mix change, 5x5 sheet animates. Check the particle quads are not upside down.
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
18. Start a new game. The Lada body is Used to Old, not 100%, and rust shows. Every new game draws the same state: the startup random is fixed.
19. Pick two other start cars in the game mode menu. Each starts worn the same way.
20. Look at the Lada oil cap and coolant cap. Each shows its name and level, like the fuel cap. E opens and closes it.

### World and terrain

4. Check the winter color looks colder in Winter and at biome borders in Mixed. Check F3 in Mixed while the color grading rebakes.
5. Sandstorms: spawn a storm near the player with sandbox `storm(50)`. Check sky, fog, light, sound and dust, and that dust stops inside buildings and cars. Check that wind does not move items inside a car. Check the F3 storm line.
6. Power cables: check poles link to the next pole on curves and tilted poles, road buildings and the home link to a pole, and the rope meshes are not flipped.
7. Draw distance: check F3 at 500, 2500 and 5000 m near the garage and on the road.
8. Check far terrain, buildings and fog at 2500 m against the original.
9. Loot flashlight beams show the cookie on walls and terrain.
10. Near the Mansion: listen to the crow sound fade in and the day and night ambience fade out.
11. Swing the shovel at metal, wood, plants and sand. Check the hit particles of each.
12. Dig a buried item up with the shovel. It must lift, become loose, and then allow pickup. A buried item must refuse pickup. Save and load with an item buried and one dug up.
13. Metal detector: turn it on and off with use. Check the beep pitch and volume and the needle near metal, buried metal, and behind a wall. Save and load with it on.
17. Move far in one step (sandbox building('Benzinkut01'), or load a far save). Wait while the area loads. The game does not stop with "validated authored material choice".

### Saves

### Settings

5. Crosshair off hides it. Old crosshair shows the source dot with the carry bar. Crosshair scale changes its size.
9. Fresh profile: F3 follows the display refresh. FPS limit 60 on holds F3 at 59 to 61 FPS while playing, also on a 144 Hz display.

### Languages and help text

1. Re-export first: the export now holds the word table, the 9 language files and the car help boxes.
2. Pick Deutsch in Settings > Misc: menus, tabs, settings rows, the key list and save screen texts change at once.
3. Pick 日本語 and 中国人: the text shows without empty boxes. The menu font has no CJK glyphs, so the browser fallback font shows them.
4. In game with Deutsch: the action hints use the German words. Port words without a source word (Drop, Grip, Tap, Hold, Distance, Posture, Roll) stay English.
8. Compare the help box place and the line breaks with the original.

### Start menu figure

1. The start menu shows the figure in the menu camera view, under the menu art. It wears the Appearance outfit. Change the outfit: the figure follows.
5. F3 on the start menu: frame time near the previous run.
6. Compare with the original menu: pose, place, head turn speed. Note the port choices: no idle clip, no menu scenery, gray background.

### Player body and outfit

2. Check you cannot see inside the head.
3. Check the body shadow and the eye shadows. Check the siphon hose shadow follows the hose.
4. Compare F3 shadow draw counts and frame time near the garage and on the road with the previous test pass.
6. Use doors and handles. Push a car on foot and from a seat.
7. Carry a two-hand item.
8. Hold Left Alt: the hand points at the aimed surface. Rub dirt off a car and wash a wash trigger. Hold Left Alt at the sky: the hand waves.
10. The camera stays in the head. Mouse look fades out.
11. The death screen effect fades in over 14 s, then the menu opens. Read the console for DeadV parameters the port ignores.
12. Load a save: no ragdoll is left.
13. Put on and take off each wearable type (glasses, hat, cap, helmet, mask) and the cigarette with Tab and X. The cigarette comes off first.
14. Worn items are hidden in first person but cast shadows. The worn pumpkin inner mesh stays visible, as in the source.
15. Save and load with worn items: they stay worn. Die: each worn item drops.
20. Put on glasses with each lens color. Each lens tints the view like the original, and the tint matches the visible lens color.
21. The dark lens (profile 0) dims the view by 2 stops.
22. Bright highlights look untonemapped, like the original.
23. Lenses with the flare flag hide the sun flare. Other lenses keep it.
25. Take the glasses off: the tint clears. Die with them on: the tint clears and the death effect fades in.
26. Save and load with the glasses worn: the tint comes back. Read the console for a warning about more than one active lens.

### Weapons and damage

1. Drop and throw beer bottles, the vase and a box. Soft drops keep them whole. Hard hits break them with pieces and a break sound. The pieces scatter and fade.
2. Hit a building lamp with the shovel. It goes dark and stays dark when switched. Save and load: it stays broken.
3. Swing the katana, crowbar, fly swatter and leg with each attack key. Each plays its own animation and sounds and breaks a bottle. The hands follow the weapon grips.
5. Hit a sausage with the shovel or a hurtFood weapon: it splits into pieces. Save and load: the pieces keep their bites.
6. Fire each gun: revolver, BB gun, SMG and the three shotguns. Check the shot sound, muzzle flash, recoil, tracer, sand or spark particles and hit sounds. A shot breaks a bottle. Shotgun pellets push a crowbar.
7. Check the HUD crosshair per gun, and its empty color.
13. Shoot metal, wood, sand, glass and flesh: bullet holes show with the right texture, face out, move with items and fade per lifetime. Check the first hole for a hitch.
14. Shoot a world fuel can or tank with gas, oil or diesel over 40%. It explodes with particles and sound, breaks nearby bottles, pushes items and sets off a second tank. Save and load: the exploded items stay gone.
18. Stand near a blast: sound goes dull and an ear ring plays from the blast side. Stand close: the player dies with the ragdoll.

### Creatures

1. See planes cross the sky with their contrails and engine sound. A UFO spins and hums.
2. Bats: set the PC date between 15 Oct and 15 Nov, start a new game and drive to a gas station. In 65% of stations, 5 to 30 bats fly near it, flap and squeak. Pick one up: it keeps squeaking and does not fly out of the hand. Save and load near the bats.
4. Rabbits roam, run from shots, and jump over small obstacles. Their ears twitch.
8. Evil rabbits have glowing red eyes.
9. Check the evil rabbit roar and bite animations and sounds.
10. A bite of a big evil rabbit damages loose items and other rabbits near the bite.
12. Start a car engine near rabbits. Normal rabbits flee from it.

### Small effects

1. Drop and throw food and cut food pieces. Each makes its hit sound.
2. Eat each food. Crumbs, meat, turd and chocolate bits fall from the food. Smoke a cigarette: ash falls now and then.
4. New game between 15 October and 15 November: the start pumpkin glows and flickers at night. Hold it and wear it: the light follows.

### Advanced rendering

1. Antialiasing on (default): stand still in the garage. Edges of cars, wires and fences do not crawl or shimmer.
2. Turn quickly and walk sideways past wires and poles. Edges stay sharp, with no trails behind thin objects.
4. Watch another moving car or a falling item from a standing view. It shows no trail.
5. Teleport (sandbox `building`), load a save and switch the car camera with C. The first frame after each shows no smear.
6. Turn Antialiasing off and reload. The image has hard edges. Check F3 frame time with it on and off, standing and driving.
7. Lamp shadows: at night, turn on the Lada headlights, a road lamp and a house lamp near the car. Objects cast shadows from each lamp. Compare with the original: hard and soft edges, strength, fade with distance.
8. Look for shadow acne and gaps at contact points. Tune `BIAS_TEXELS` in `src/presentation/lamp_shadows.rs` if needed.
11. Check the page log for "layout requires ... textures": the lamp atlas adds one texture to each surface pipeline.
12. At night, stand still near a lit street lamp, then open a door and drive a car past it. The lamp shadows follow each move with no stale shadow. Compare F3 frame time and "GPU ms" with the previous test pass.
15. Check F3 near four cars with Reflections on and off. Live probes render one 64 px cube face each frame.
16. Re-export first: the export now has the live mirror records.
17. Sit in the Lada with Mirrors on. The side and front mirrors show the world behind the car, not flipped.
18. Turn Mirrors off, or walk more than 4 m from the car. The mirrors show the glass material.
19. Move the Mirror draw distance slider. Far objects appear and disappear in the mirrors.
20. Stand among several cars. Check F3 with 5 live mirrors against the frame rate rule.
21. Turn Depth of field on. Look at the dashboard from the driver seat: it stays sharp and the far road softens a little. Look out of the window: the focus moves out over a short time.
23. Check F3 frame time with Depth of field on and off, with Antialiasing on and off. With Antialiasing on, the soft edges do not flicker. Drive with Motion blur on: the blur still works.
24. Use the binoculars with Depth of field off: near and far things blur with the focus, and scroll moves the sharp distance. The image bulges at the center and edges show color fringes.
25. Release the binoculars: the lens effects stop, and DoF returns to the setting.

### Multiplayer

Re-export first: the export now holds `playeranimationscript.sendInterval`. Use two browser tabs on one PC first, then two PCs.

2. The guest loads the host world and stands where the host stood. A host in a car seat puts the guest in the same seat until M02.
3. Each player sees the other walk, run, duck, prone, look up and down, and carry items. Names show above the heads.
10. Name labels stand above the heads. Check the label height (port choice 2.1 m).
11. Each figure keeps its own outfit colors. The local body colors do not change.
12. F3 with 1 and with 7 remote players. Report the frame time cost per figure.
