# Project roadmap

Updated: 2026-10-29. Goal: recreate The Long Drive for the web.
Behavior authority: Steam build `11140890` and its decompiled code in `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs`.
Save format authority: [save schema](crates/simulation/src/save/schema.rs).

## Frame rate rule

The game runs at about 60 FPS now. Every new feature must keep it there.
Check the F3 overlay before and after each change. Report the frame-time cost.
If a feature drops the frame rate, fix the cost before merge or agree a cut.

## Task rules

Work section by section, in the order below. Keep the game playable after each task.
Recheck the code and the original before each task. Complete missing behavior and verify existing behavior.
Follow [AGENTS.md](AGENTS.md) for checks.
Remove a row when its task is done. Do not mark rows done.
Add the manual checks of each task to the test pass below. Remove them after they pass.

Size: S is up to a day. M is a few days. L is about a week. XL is several weeks.

## 1. Starting area, player, and movement

Finish the home, garage, Lada, and nearby driving first. The 2026-10-29 audit found these gaps.

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| S3 | Tune icons | S | Radio knobs and the sink tap show their own mouse wheel image (`usablescript.img`, `tankcapscript.img`). Today they show the cap image. |
| S4 | Garage creep and names | S | Barrel `BARREL (7)` creeps and wire brush `Drotkefe (1)` spins on its shelf. The car hover name shows "Car02kFull". |

## 2. Car parts and repair

Fix part repair, electronics, the siphon hose, exhaust smoke, and cargo storage. All tasks are done.

## 3. World and terrain

All tasks are done.

## 4. More cars

All tasks are done.

## 5. Later

| ID | Task | Size | Remaining work |
| --- | --- | --- | --- |
| X03 | Creatures | XL | Ground UFO, EMP and green-eyed evil rabbits, human monster (attacks hurt the player), breakablescript `DamageFrom` for AI. Rabbits do not yet sense the player hum. |
| X04 | Menus, settings, and saves | L | New game options, game modes, homesick mode, menu player figure with IK, settings, rebinding, languages, help text. |
| X06 | Advanced rendering | XL | Live mirrors, lamp shadows, motion blur, TAA, live reflection probes, depth of field, binocular lens distortion and chromatic aberration, heat haze, LUT options, MSAA 8x. Each one must pass the frame rate rule. |

Not planned: multiplayer, VR, stereo 3D, mod events, Steam features, developer tools.

## Test pass

Run these checks on Tomasz's PC in one run. Nothing below has run in the browser yet.

Prepare once:

1. Pull master. Re-export, then make a release build. Old exports and old saves do not load. Only the current save schema loads.
2. Paste the acquisition hash that the export prints into `CONTRACT_SHA256` if it changed.
3. Check the export log for the "Biome materials" line (set count and marked nodes), the melee weapon list, the gun list and the explosion record. The clothing preview export runs without the assets: if it fails, read its error.

### Starting area (section 1)

1. Check F3 near the garage and on the road. Compare with the frame rate rule.
2. Play the garage loop in the browser: repair, fill, start, drive out, return, save, reload.
3. Check the 20 L can stays on the ground after drive-out and return.
4. Check paint spray and sponge particles (P02): refraction and soft edges.
5. Check slope normals far from origin, ladders, head bob, sleep hint, hazards.
6. Check player sounds: hits, footsteps, ambience, muffling in buildings.
7. Check the music box, radios, compass, clock, and small items.
8. Crosshair icons (S1): look at a car door, a sliding and a vertical sliding usable, a light switch, a radio knob, a tuneable cap and a seat. Push the Lada, hold the hose at a cap and a trailer grip at the hitch, fill a can. Carry a light, a medium and a heavy item. Each case shows the source icons together, and the heavy item shows the too heavy icon.
9. Binoculars (S2): pick up the garage binoculars and hold LMB. The view zooms through the lens with the black two-circle mask, the binoculars and the crosshair hide, and look speed drops. Scroll changes the zoom between 3 and 4 degrees. Release LMB to return. Try it with the binoculars equipped too.

### Car parts and repair (section 2)

1. Run the ignored export tests: `cargo test --test garage_parts -- --ignored` and `cargo test --lib r05_cargo_locks_in_trunk -- --ignored`.
2. Repair (R01): swap the hood and the trunk lid, repair rust and dirt.
3. Electronics (R02): check the Lada radiator fan spins while the engine runs, and stops with the engine. Swap in a loot radiator: its fan spins too.
4. Exhaust smoke (R04): rate follows rpm, color changes after a fuel mix change, 5x5 sheet animates. Check the particle quads are not upside down.
5. Siphon hose (R03): first check the garage hose spawns clear of the walls. Attach it to the Lada tank and a can. Siphon by height (both ends are main, so the source runs the siphon twice per frame), drain with LMB, coil with R, store it, tear it off by driving.
6. Cargo storage (R05): lock items in the Lada trunk, glove box, and engine bay, and in a fridge, a pallet, and a coffin. Open the lid to release them. The source box is manual only: items in it when its area loads lock, items put in it later stay loose. Items in a fridge, pallet or coffin when its area loads lock at once, not after 3 s.
7. Save and load with the hose attached and with cargo locked.
8. Check F3 with smoke, near a hose, and while driving with cargo. Compare with the frame rate rule.

### World and terrain (section 3)

1. Check the Game mode button position on the start menu.
2. Start a new game in each biome. Check terrain, props, roads, lamps, fog, sky and swapped materials.
3. Load a Mixed save with Winter set. The world must load as winter.
4. Check the winter color: colder view in Winter, changes at biome borders in Mixed.
5. Check F3 near the garage and on the road in Mixed while the color grading rebakes.
6. Sandstorms (W07): run `cargo test --lib terrain_sample_matches_published_ground -- --ignored`. Spawn a storm near the player with a temporary hook (never commit it). Check sky, fog, light, sound, dust, wind on loose items, and that dust and wind stop inside buildings and cars. Check the F3 storm line and the frame rate in a storm. Save and load inside a storm.
7. Power cables (W04): check poles link to the next pole on curves and tilted poles, road buildings and the home link to a pole, and the rope meshes are not flipped. Check F3 along the road.
8. Draw distance (W01): check F3 at 500, 2500 and 5000 m near the garage and on the road. Compare with the frame rate rule.
9. Set a draw distance that does not fit (above about 5500 m). New game must refuse it and name the largest value. Reload: the slider keeps the last value that started.
10. Check the slider is locked in the pause menu. Check far terrain, buildings and fog at 2500 m against the original.
11. Run `cargo test --test world_range -- --ignored --nocapture`. Peaks must stay below the limits.
12. World population (W05): run `cargo test --release --lib w05_loot_matches_source_rules -- --ignored --nocapture` (about 5 min plus build). All 11 building types must pass. The `unsupported_part_type` drops must be gone.
13. Check loot in a few building types: gas and oil cans, beer, barrels, buckets and the WC now spawn. Flashlights in loot must not stop the world, and their beams show the cookie on walls and terrain.
14. Near the Mansion: the crow sound fades in from 1000 m to full at 200 m, and the day and night ambience fades out by the same amount. Check F3 near the Mansion.
15. Digging (W06): find a buried item (underground spawn). Swing the shovel in each direction (LMB with back, left, right keys). Check the swing sound, hit sounds, hit particles on metal, wood, plants and sand, and that hits push loose items.
16. Dig a buried item up with the shovel. It must lift, become loose, and then allow pickup. A buried item must refuse pickup. Save and load with an item buried and one dug up.
17. Metal detector (W06): turn it on and off with use. Check the beep pitch and volume and the needle near metal, buried metal, and behind a wall. Save and load with it on. Check F3 while it scans.
18. Tumbleweeds (W07): check they spawn near the player, roll in gusts, stop near walls, and save with the world. Check F3 with several tumbleweeds.
19. Long trips (W02): run `cargo test --release --lib w02_ -- --ignored --nocapture`. Note the printed bytes per km and the 5,000 km estimate. Above 200 MB, compact spawn boundaries.
20. Also run the roadside tests: `cargo test --lib roadside_ -- --ignored`. Far edits, consumed items and lamp states must stay after their area unloads.
21. In the browser: drop an item, drive about 10 km out and back, save and reload. The item stays where it was dropped. Check F3 before and after the drive.

### More cars (section 4)

1. Run `cargo test --lib item_seat -- --ignored` and `cargo test --lib sled -- --ignored`.
2. Sit on the WC and on each sled seat. Stand up. Check the view limits and the prompt.
3. Sled (V04): slide down a slope, steer, reverse, stop. Check the slide sound follows speed and stops in the air. Load the sled with items above its slide mass: it slides without a rider.
4. Ride a sled away from its building: it must not freeze or vanish. You cannot pick up or store a sled you sit on.
5. Save and load seated on the sled and on the WC.
6. Check F3 near a parked sled and while sliding. Compare with the frame rate rule.
7. Loader (V03 PR 1): New game. The Lada stands at its garage spot. Drive it, use lights, gauges, radio, horn, wheels off and on, cargo in the trunk. Save and load.
8. Run `cargo test --lib two_ladas -- --ignored` and `cargo test --lib installed_lada -- --ignored`.
9. Drive the Lada far from the garage and back: it must not vanish. Check F3 at the start and while driving.
10. Gear lever (V06): the Lada lever moves through each gear in manual and automatic.
11. Wheel effects (V06): dust on sand and none on asphalt, grass trails, rolling, spring and skid sounds, smoke and sand bursts while skidding.
12. Mouse driving (V06): U toggles it in the car. Mouse X steers, mouse up is gas, down is brake, mouse look is off.
13. Steering wheel (V06): with a real wheel or pad, bind every axis and gear button. Check H-shifter on and off and the live values. Steer a sled with the pad.
14. Check F3 before and after, driving on sand at speed. Compare with the frame rate rule.
15. Towing (V05): run `cargo test --lib lada_tows -- --ignored`.
16. Find a small trailer and a bus trailer. Pick up the grip and put it on the Lada hitch. Drive, turn, reverse and brake.
17. Check the trailer rear and brake lamps follow the Lada lights, ignition and brake. Check the bus trailer brake switch.
18. Pick the grip off the hitch while the car stands and while it rolls. Save and load with the trailer on the hitch.
19. Tow a trailer far from where it spawned and back: car, trailer and grip must not vanish or freeze apart.
20. Check the trailer reflection probe. `PlusWheelToKeepAwake` is not ported.
21. Check F3 while towing. Compare with the frame rate rule.
22. Buses (V03): run `cargo test --lib v03_bus_pool -- --ignored`.
23. Check F3 at the start and near a bus. Compare with the frame rate rule.
24. Find Bus01, Bus02 and Bus03. Fill a bus with diesel and drive it. With petrol the engine must fail to start.
25. Bus01: both gauge parts in their kmh slots read speed. Check the counters and the gauge lamps.
26. Bus doors: the signal button lights green, the open button swings the leaves and lights red. Check the leaf sounds and the stretching colliders. Buttons do nothing without electronics power.
27. Bus02 trailer: put its grip on the bus hitch. Drive and turn. Its brake switch sets brake force 100 on each wheel. Its rear lamps follow the bus lights and brake.
28. Save and load with bus doors open and signalled and with the trailer on the bus.
29. Bikes (V04): run `cargo test --lib bike -- --ignored`.
30. Find Bike01 and Bike03. Ride Bike01: it stays upright, leans into turns, and falls over when nobody rides or pushes it.
31. Kick start Bike01: ignition on, roll it fast enough, the engine starts with its start sound. No starter motor.
32. Push a bike and the Lada by hand with their grips. The bike stays upright while you push it.
33. Bike03: pedal with W from rest. The pedals and feet turn. Kick start the engine by pedaling; pedals stop while the engine runs. Fuel comes from its tank parts.
34. Swap bike parts: wheels, mirrors, headlight, basket, bag, gauge, tanks. Save and load seated on each bike.
35. Check F3 while riding each bike. Compare with the frame rate rule.
36. Other cars (V02): run `cargo test --lib car_types -- --ignored`.
37. Pick each start car in the Game mode screen, then Random a few times. Each car starts, drives, saves and loads. StartCar06 diesel comes with 0.5 L diesel.
38. Two-stroke cars (car03, car09, car10): run them on a 2.5 % oil mix, then on plain petrol. The engine weakens without oil. Check the oil lamp.
39. Find roadside bare and full cars, also Car07 and a bare Lada, and no Car06P. Parts, fluids, rust and body variants differ between copies.
40. Car07 shifts by itself in manual mode. Its dashboard clock shows game time, and its knob sets the time.
41. Run `cargo test --release --lib w05_loot_matches_source_rules -- --ignored --nocapture`. The car pool totals print and no V03 car spawns.
42. Drive past groups of roadside cars for 2 km. F3 bodies and generated objects fall back after you pass. Your start car and last car keep their area.
43. Check F3 with several cars in view. Compare with the frame rate rule.

### Saved state (X05 part 1)

1. Run `cargo test --lib two_ladas -- --ignored`.
2. Park the Lada with the wheels turned. Save and load: the wheels stay turned.
3. Sleep, wake, save and load: the sleep hint stays off until the cooldown ends.

### World part repair and paint (X05 part 2)

1. Run `cargo test --lib lada_part_paint_and_repair -- --ignored`. It also saves and loads the painted part.
2. Brush rust off a Lada part and a roadside car part. Rub dirt off with the sponge. The material changes at once.
3. Paint a Lada door and a loose part with spray paint. The color changes while you spray.
4. Repair a tire. The HUD condition and the wheel grip follow it.
5. A state change binds a new material. Check the part does not blink for a frame.
6. Check F3 while spraying and near many roadside cars. Compare with the frame rate rule.
7. Paint and brush a roadside car part. Drive away until it unloads, come back: the part keeps its look. Save and load: it still keeps it.

### Save slots and autosave (X04b)

1. Run `cargo test --lib x04b -- --ignored`.
2. Save under two names. Each tile shows a game thumbnail (WebGPU canvas copy), date, seed, km and version.
3. Overwrite, Load and Delete ask Yes/No in game. Load last loads the newest save.
4. Play 2 minutes on foot. The autosave tile appears. Check the F3 frame cost of the autosave frame.
5. Turn on "Load last save at start" and reload the page. The last save loads.

### Player body (X01a)

1. Look down standing, crouching, walking forward and back. Carry a light and a heavy (>10 kg) item. Sit in the Lada (driver and passenger), on the WC and on the sled. Check legs, arms (twist around the bone axis is a known risk) and hands on the wheel.
2. Check you cannot see inside the head.
3. Check the body shadow and the eye shadows. Check the siphon hose shadow follows the hose.
4. ShadowsOnly renderers now cast (#138). Compare F3 shadow draw counts and frame time before and after, near the garage and on the road.
5. Check the F3 cost of the body.

### Hand and foot reach (X01b)

1. Drive the Lada. Use the horn, lights, ignition (tap and hold), handbrake and gear stick: a hand moves to each. Press gas, brake and clutch: the feet move to the pedals.
2. Use doors and handles. Push a car on foot and from a seat.
3. Carry a two-hand item.
4. Hold Left Alt: the hand points at the aimed surface. Rub dirt off a car and wash a wash trigger. Hold Left Alt at the sky: the hand waves.

### Player ragdoll (X01c)

1. Die on foot and in a moving car. The body falls with joint limits and carries the car speed.
2. The camera stays in the head. Mouse look fades out.
3. The death screen effect fades in over 14 s, then the menu opens. Read the console for DeadV parameters the port ignores.
4. Load a save: no ragdoll is left.
5. Press F3 during death. Compare with the frame rate rule.

### Wearables, outfit and appearance editor (X01d)

1. Put on and take off each wearable type (glasses, hat, cap, helmet, mask) and the cigarette with Tab and X. The cigarette comes off first.
2. Worn items are hidden in first person but cast shadows.
3. Save and load with worn items: they stay worn. Die: each worn item drops.
4. Open Appearance from the start menu and the pause menu. The preview figure shows next to the panel with the export camera and lights. Check the figure pose, the lights and the colors. Changes show at once. Back and Escape save. Reload the page: the outfit stays.
5. Set Male and Female with custom colors and disabled parts. Check them in game (female pose follows the male joints by name), in the shadow and on the ragdoll. Check the submesh colors of each segment.
6. Set the model modes Off, Shadow and Visible. Shadow hides the body but keeps its shadow.
7. Check the world view and its shadows do not draw the preview figure. Check F3 with the screen open.

### Sunglasses (X01e)

1. Put on glasses with each lens color. Each lens tints the view like the original.
2. The dark lens (profile 0) dims the view by 2 stops.
3. Bright highlights look untonemapped, like the original.
4. Lenses with the flare flag hide the sun flare. Other lenses keep it.
5. Turn off "Sunglasses effects" in the display controls: the tint goes away. The flare rule stays. Reload the page: the setting stays.
6. Take the glasses off: the tint clears. Die with them on: the tint clears and the death effect fades in.
7. Save and load with the glasses worn: the tint comes back.
8. If possible, compare the NoTonemap LUT kernel with the game DXBC (`KGenLut3D_NoTonemap`). The port follows the open PPv2 `Lut3DBaker.compute`.
9. The worn lens comes from the item selector choices, matched to lens nodes by source id (X01e PR 1). Check that the tint matches the visible lens color. Read the console for a warning about more than one active lens.

### Damage and breaking (X02 part 1)

1. Run `cargo test --lib breakables -- --ignored`.
2. Drop and throw beer bottles, the vase and a box. Soft drops keep them whole. Hard hits break them with pieces and a break sound. The pieces scatter and fade.
3. Hit a building lamp with the shovel. It goes dark and stays dark when switched. Save and load: it stays broken.
4. Check F3 near a box pile and while pieces fall. Compare with the frame rate rule.

### Melee weapons (X02 part 2)

1. Run `cargo test --lib melee -- --ignored`. Fix the katana and crowbar prototype names in the tests if they are not found.
2. Swing the katana, crowbar, fly swatter and leg with each attack key. Each plays its own animation and sounds and breaks a bottle.
3. Hit a buried item with the crowbar: it does not dig. The shovel still digs.
4. Hit a sausage with the shovel or a hurtFood weapon: it splits into pieces. Save and load: the pieces keep their bites. A dropped turd also saves and reloads now.
5. Check the hands follow the weapon grips during attacks.

### Guns (X02 part 3)

1. Run `cargo test --lib guns -- --ignored`. Fix the gun prototype names and the equip helper if guns are not found.
2. Fire each gun: revolver, BB gun, SMG and the three shotguns. Check the shot sound, muzzle flash, recoil, tracer, sand or spark particles and hit sounds. A shot breaks a bottle. Shotgun pellets push a crowbar.
3. Check the HUD crosshair per gun, and its empty color.
4. Reload and unload with R and E from aimed or carried ammo. Round totals stay the same. The left hand moves to the gun.
5. Shotguns and BB gun: hold R and move the mouse to break the barrel. Closed barrels fire; open barrels reload. The BB gun gains power.
6. Revolver: hold R and scroll to turn the cylinder. Empty chambers click. Bullets show in full chambers.
7. SMG: load from a magazine item. No magazine: no fire. Save and load keeps ammo, barrel, chambers, power and magazine.
8. Shoot metal, wood, sand, glass and flesh: bullet holes show with the right texture, face out, move with items and fade per lifetime. Check the first hole for a hitch.
9. Check F3 while firing the SMG with many holes. Compare with the frame rate rule.

### Explosions (X02 part 4)

1. Run `cargo test --lib explosion -- --ignored`. Fix the tank picks in the tests if no explosive tank is found.
2. Shoot a world fuel can or tank with gas, oil or diesel over 40%. It explodes with particles and sound, breaks nearby bottles, pushes items and sets off a second tank. Save and load: the exploded items stay gone.
3. Hit an explosive tank with a melee weapon. It explodes as with a shot.
4. Shoot a tank that does not explode with a shotgun. Holes leak fluid when they face down. Check the fluid amount falls.
5. Stand near a blast: sound goes dull, an ear ring plays from the blast side, and both fade. Save and load keeps the ringing. Stand close: the player dies with the ragdoll.
6. Check F3 during a chain of blasts. Compare with the frame rate rule.

### X02 leftovers

1. Run `cargo test --lib x02_leftovers -- --ignored`.
2. Shoot the full 20 L garage fuel can. It explodes, and the can and its cap go away. Save and load: they stay gone.
3. Shoot the garage water tank with a shotgun. Holes that face down leak.
4. SMG: unload with E at loose ammo, or with R while you hold ammo. A magazine drops at the gun with the gun rounds, and the gun is empty. Load it back with R. Load a held magazine into a loaded SMG: the old magazine drops. Save and load: a dropped magazine keeps its rounds. The export now has the magazine as an item prototype and a new acquisition hash.
5. Drive into a cactus, shoot one until it breaks, and blow one up. Cactus pieces fly off and the cactus stays, as in the original (its Break throws before Destroy). It does not break again until its area streams in again.

### Sky traffic and bats (X03a)

1. See planes cross the sky at 600 m with their contrails and engine sound. The first plane comes at start, then one every 50 to 500 s, at most 3. About one in ten is a UFO: it spins and hums.
2. Check F3 with planes in the sky. Compare with the frame rate rule.
3. Bats (X03a): set the PC date between 15 Oct and 15 Nov, start a new game and drive to a gas station. In 65% of stations, 5 to 30 bats fly near it, flap and squeak. Pick one up: it keeps squeaking and does not fly out of the hand.
4. Save and load near the bats.
5. Check F3 next to the bats. Compare with the frame rate rule.

### Rabbits (X03 part 1)

1. Rabbits spawn 30 to 100 m away, first at start, then every 50 to 100 s, at most 10. Big ones are larger and slower.
2. They roam, run from the player and from shots, and jump over small obstacles. Their ears twitch.
3. They die on hard hits, falls and head shots, with a die sound. A head shot can take the eyes (eyes hide) or the ears.
4. Dead rabbits fall as ragdolls. Pick up and carry a bone: the body follows.
5. Save and load with live and dead rabbits, and with a rabbit that lost its eyes.
6. Check F3 near 10 rabbits. Compare with the frame rate rule.

### Evil rabbits (X03 part 2)

1. About one rabbit in four (the big ones) is evil and has glowing red eyes.
2. An evil rabbit that sees the player roars and stops, then runs at the player and attacks with a bite animation and sound.
3. A bite of a big evil rabbit hurts the player. It also damages loose items and other rabbits near the bite.
4. Close to an evil rabbit (about 4 m), it growls every 10 to 30 s.
5. Start a car engine or hold the horn. Normal rabbits flee from it. Evil rabbits flee from a loud sound far away, but still attack up close.
6. Turn on a portable radio near rabbits: they react to it like a quiet engine.
7. Save and load: big rabbits are still evil.
8. Check F3 near 10 rabbits and a running car. Compare with the frame rate rule.

### Player movement (vehicle carry, air momentum, wall jump)

1. Park a bus on a slope, release the handbrake and stand inside while it rolls. The player rides inside.
2. Stand on a trailer bed or the Lada roof while it moves fast: you slide off. While it moves slowly: you ride.
3. Jump inside a moving bus: you land in the same place.
4. Run, jump, release W: you keep flying forward. Standing jump: straight up. Keys in the air turn you slowly.
5. Jump at a wall: one extra push. Jump at a ledge: a second push. Hold jump below the ledge: you grip and stop falling, then jump up.
6. Save mid-air and load: the jump continues.
7. Check F3 walking and in a bus. Compare with the frame rate rule.

### Car camera modes (X10)

1. Sit in the Lada driver seat. Tap C: the outside view orbits the car with the mouse. Pitch stops at about 89 degrees.
2. Hold Q and move the mouse: the view pans up to 1.5 m. Shift+Q pans forward and back.
3. Scroll: zoom from 1 to 30 m. Hold Tab: the view goes back to 5 m and the default angle.
4. Drive: the view turns behind the car, faster at higher speed.
5. Tap C: back to the cockpit. Repeat on the passenger seat and on a sled or bike seat.
6. Hold C for 0.4 s: the camera stays at its spot and turns with the mouse. Hold C again: it follows the car. Tap C: back to the view before.
7. Press M in a seat: the camera rises over 1/3 s and looks down. Scroll sets the height from 2.5 to 200 m. Q and the mouse pan. Tab recenters. Press M again: the camera comes back down.
8. In each other view: E does not get out or use things, prompts and the crosshair hide, worn items show, and the sunglasses tint is off. Sleep is not possible.
9. Check F3 in each view. Compare with the frame rate rule.

### Car crash damage (X09)

1. Run `cargo test --lib x09_crash -- --ignored`.
2. Drive the Lada into a wall at low, medium and high speed. Light parts (mirrors, hubcaps) fall off first. Fast crashes drop more parts. Worn parts fall off sooner.
3. Fallen parts do not lock into the trunk until you pick them up. Refit them with F.
4. A crash can sound the horn. A hard crash puts blood in the car with a sound. A second hard crash kills you.
5. Wash the blood off with the sponge. The next hard crash only puts blood in the car again.
6. Save and load with blood in the Lada and in a world car. The blood stays. Drive away from the world car and back: the blood stays.
7. Blow up a tank next to a car. Parts fall off.
8. Check F3 while you drive past many roadside cars. Compare with the frame rate rule.

### Small effects

1. Run `cargo test --lib tumbleweed_breaks -- --ignored` and `cargo test --lib spawned_copy_takes -- --ignored`.
2. Hit a tumbleweed hard with the car, a shot and a melee swing. It breaks with its gib and sound. Save and load: it stays gone.
3. Drop and throw food and cut food pieces. Each makes its hit sound.
4. Run `cargo test --lib lada_tows -- --ignored`: trailer wheel meshes turn while the trailer rolls.
5. Stay up at night for a few nights. The horror sound plays for 5 to 15 s on the first night, then every 1 to 5 nights. It stops at day.
6. Overheat a car engine above 110 degrees with coolant: steam rises under the hood and the boil sound grows with the heat. Check that steam is not lost inside the hood colliders. The steam noise is an approximation of Unity's. Car08 and Car08c have no steam.
7. Tow a small trailer and drive Bus02 with its trailer: wheel meshes turn and steer. Bus02 leaf springs, axles and tipper rams follow the wheels.
8. Eat each food. Crumbs, meat, turd and chocolate bits fall from the food. Smoke a cigarette: ash falls now and then.
9. New game between 15 October and 15 November: the start pumpkin glows and flickers at night. Hold it and wear it: the light follows.
10. Check F3 with steam on, near Bus02, and near the pumpkin. Compare with the frame rate rule.

### Loot tools (X11)

1. Re-export. The log shows "Loot tools: 3 prototypes".
2. Run `cargo test --lib loot_tools -- --ignored`.
3. Find a loot wire brush, sponge and spray paint. Pick up each one and hold LMB to use it.
4. Clean rust and dirt with the loot wire brush and sponge. The brush model moves on the surface, with particles and sound.
5. Paint a car part with loot spray paint. The can shows its color, sprays particles with sound, and runs out.
6. Use a loot brush until it wears out. Its scrub part hides and it stops working.
7. Save and load. Paint left, color and wear stay. Drive away and back: they stay.
8. Check F3 while you use each loot tool. Compare with the frame rate rule.

### Worn start cars

1. Run `cargo test --lib lada_part_paint_and_repair -- --ignored`. It checks that the start car starts worn.
2. Start a new game. The Lada body is Used to Old, not 100%, and rust shows. Every new game draws the same state: the startup random is fixed.
3. Pick two other start cars in the game mode menu. Each starts worn the same way.
