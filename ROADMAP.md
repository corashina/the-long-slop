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
| X04a | New game options | M | Peaceful, survival and homesick toggles, seed field. Homesick builds home copies along the road. |
| X04b | Settings screen | M | Settings store and screen: sensitivities, invert, FOV, crosshair, head bob, help, always-show stats, volume, FPS limit, reset per tab. |
| X04c | Rebinding | M | Original action list and default keys. Rebind on the Controls screen. HUD hints show bound keys. After X04b. |
| X04d | Languages and help text | M | 10 language files, language dropdown, word table for all menu and HUD text. Help text follows the assists. After X04c. |
| X04e | Menu figure IK | S | Start-menu figure turns its head, body and hands towards the mouse, as `menuIkScript`. |
| X06a | Live mirrors | L | Car mirror cameras render to textures. Only the 5 nearest run. FOV and far plane follow distance. Mirrors toggle and Mirror draw distance slider. |
| X06c | Live reflection probes | M | Only 1 probe is live. Make every building and car probe live, with the 5 s disable distance check and the Reflections toggle. |
| X06e | Motion blur | M | Uses the motion target of `motion_vectors.rs`. Shutter angle follows car speed. Off where the source turns it off. |
| X06f | Depth of field and binocular lens | M | Main DoF with focus ray, night clamp and Dof toggle. Binocular DoF, lens distortion and chromatic aberration. |
| X06g | Heat haze and LUT options | M | Heat haze pass weighted by temperature, old particle heat haze, LUT option (Default, C64, Sepia), grayscale. |

Not planned: multiplayer, VR, stereo 3D, mod events, Steam features.

## Test pass

Run these checks on Tomasz's PC in one run. Nothing below has passed yet.
Check F3 in each scenario below. Compare with the frame rate rule.

Known failures on master c0da737f (2026-10-30 browser retest), fix before the next run:

- Appearance preview shows only the legs and shoes. The camera framing is wrong for Male and Female.
- Looking down shows hollow shirt sleeves, floating hands and hollow pant legs. It also shows at normal pitch while walking.
- Outside the Lada, E on the door handle does not open it. The HUD shows "[LMB]+[Mouse]: Move | [F]: Dismount".
- The inspection pit traps the player: about 2 m deep, no way to climb out.
- First page load can stall at the start of object loading with dependency staging full (7.7/7.8 MiB). A reload clears it.
- In game, pause > Load > Load last once stayed on "Loading..." for over 6 minutes, with no console messages.
- Most ignored cargo tests fail: no start vehicle after `load_garage`, restore stays pending, stack overflows in debug.

Prepare once:

1. Pull master. Re-export, then make a release build. Old exports and old saves do not load. Only the current save schema loads.
2. Paste the acquisition hash that the export prints into `CONTRACT_SHA256` if it changed.
3. Run all ignored tests: `cargo test --release --lib -- --ignored --nocapture`.
4. In that output, note the `w02_` bytes per km and the 5,000 km estimate. Above 200 MB, compact spawn boundaries.
5. Claude runs the steps in Chrome with the test sandbox: see `tests/sandbox/README.md`. First check the sandbox itself: `spawn('Bus02Full', 10)` places a bus, and `teleport`, `building`, `spawn`, `hand`, `fill` and `cycle` work. Use `tests/sandbox/buses.js` for More cars steps 16-20.

### Starting area

1. Play the garage loop in the browser: repair, fill, start, drive out, return, save, reload.
2. Check the 20 L can stays on the ground after drive-out and return.
3. Check paint spray and sponge particles: refraction and soft edges.
4. Check slope normals far from origin, ladders, head bob, sleep hint, hazards.
5. Check player sounds: hits, footsteps, ambience, muffling in buildings.
6. Check the music box, radios, compass, clock, and small items.
7. Crosshair icons: look at a car door, a sliding and a vertical sliding usable, a light switch, a radio knob, a tuneable cap and a seat. Push the Lada, hold the hose at a cap and a trailer grip at the hitch, fill a can. Carry a light, a medium and a heavy item. Each case shows the source icons together, and the heavy item shows the too heavy icon. No name or condition line shows above the crosshair.
8. Binoculars: pick up the garage binoculars and hold LMB. The view zooms through the lens with the black two-circle mask, the binoculars and the crosshair hide, and look speed drops. Scroll changes the zoom between 3 and 4 degrees. Release LMB to return. Try it with the binoculars equipped too.
9. Tune icons: look at the radio volume and tune knobs, the compass and clock knobs, and a tuneable sink tap. Each shows its own mouse wheel image. A tuneable cap without an image shows the cap image.
10. Garage rest: the garage barrel and the wire brush stay still. Drive the Lada and check that parked cars and dropped items still settle and sleep.
11. Stairs: walk up stairs without a jump. Duck and walk up again. Prone does not step up.
12. Press B many times on foot and in the Lada until a turd spawns. The game does not stop. Cut a food with the knife: the piece shows.

### Car parts and repair

1. Repair: swap the hood and the trunk lid, repair rust and dirt.
2. Electronics: check the Lada radiator fan spins while the engine runs, and stops with the engine. Swap in a loot radiator: its fan spins too.
3. Exhaust smoke: rate follows rpm, color changes after a fuel mix change, 5x5 sheet animates. Check the particle quads are not upside down.
4. Siphon hose: first check the garage hose spawns clear of the walls. Attach it to the Lada tank and a can. Siphon by height, drain with LMB, coil with R, store it, tear it off by driving.
5. Cargo storage: lock items in the Lada trunk, glove box, and engine bay, and in a fridge, a pallet, and a coffin. Open the lid to release them. The source box is manual only: items in it when its area loads lock, items put in it later stay loose. Items in a fridge, pallet or coffin when its area loads lock at once, not after 3 s.
6. Save and load with the hose attached and with cargo locked.
7. Park the Lada with the wheels turned. Save and load: the wheels stay turned.
8. Brush rust off a Lada part and a roadside car part. Rub dirt off with the sponge. The material changes at once.
9. Paint a Lada door and a loose part with spray paint. The color changes while you spray.
10. Repair a tire. The HUD condition and the wheel grip follow it.
11. A state change binds a new material. Check the part does not blink for a frame.
12. Paint and brush a roadside car part. Drive away until it unloads, come back: the part keeps its look. Save and load: it still keeps it.
13. Find a loot wire brush, sponge and spray paint. Pick up each one and hold LMB to use it.
14. Clean rust and dirt with the loot wire brush and sponge. The brush model moves on the surface, with particles and sound.
15. Paint a car part with loot spray paint. The can shows its color, sprays particles with sound, and runs out.
16. Use a loot brush until it wears out. Its scrub part hides and it stops working.
17. Save and load. Loot tool paint left, color and wear stay. Drive away and back: they stay.
18. Start a new game. The Lada body is Used to Old, not 100%, and rust shows. Every new game draws the same state: the startup random is fixed.
19. Pick two other start cars in the game mode menu. Each starts worn the same way.
20. Look at the Lada oil cap and coolant cap. Each shows its name and level, like the fuel cap. E opens and closes it.

### World and terrain

1. Check the Game mode button position on the start menu.
2. Start a new game in each biome. Check terrain, props, roads, lamps, fog, sky and swapped materials.
3. Load a Mixed save with Winter set. The world must load as winter.
4. Check the winter color: colder view in Winter, changes at biome borders in Mixed. Check F3 in Mixed while the color grading rebakes.
5. Sandstorms: spawn a storm near the player with a temporary hook (never commit it). Check sky, fog, light, sound, dust, wind on loose items, and that dust and wind stop inside buildings and cars. Check the F3 storm line. Save and load inside a storm.
6. Power cables: check poles link to the next pole on curves and tilted poles, road buildings and the home link to a pole, and the rope meshes are not flipped.
7. Draw distance: check F3 at 500, 2500 and 5000 m near the garage and on the road.
8. Check far terrain, buildings and fog at 2500 m against the original.
9. Check loot in a few building types: gas and oil cans, beer, barrels, buckets and the WC now spawn. Flashlights in loot must not stop the world, and their beams show the cookie on walls and terrain.
10. Near the Mansion: the crow sound fades in from 1000 m to full at 200 m, and the day and night ambience fades out by the same amount.
11. Digging: find a buried item (underground spawn). Swing the shovel in each direction (LMB with back, left, right keys). Check the swing sound, hit sounds, hit particles on metal, wood, plants and sand, and that hits push loose items.
12. Dig a buried item up with the shovel. It must lift, become loose, and then allow pickup. A buried item must refuse pickup. Save and load with an item buried and one dug up.
13. Metal detector: turn it on and off with use. Check the beep pitch and volume and the needle near metal, buried metal, and behind a wall. Save and load with it on.
14. Tumbleweeds: check they spawn near the player, roll in gusts, stop near walls, and save with the world.
15. Drop an item, drive about 10 km out and back, save and reload. The item stays where it was dropped.
16. Load the game at 500 m and at the largest draw distance, then drive for 5 minutes. The game does not stop with "Source renderer allocation exceeds its resource budget".
17. Move far in one step (sandbox building('Benzinkut01'), or load a far save). Wait while the area loads. The game does not stop with "validated authored material choice".

### More cars

1. Sit on the WC and on each sled seat. Stand up. Check the view limits and the prompt.
2. Sled: slide down a slope, steer, reverse, stop. Check the slide sound follows speed and stops in the air. Load the sled with items above its slide mass: it slides without a rider.
3. Ride a sled away from its building: it must not freeze or vanish. You cannot pick up or store a sled you sit on.
4. Save and load seated on the sled and on the WC.
5. New game. The Lada stands at its garage spot. Drive it, use lights, gauges, radio, horn, wheels off and on, cargo in the trunk. Save and load.
6. Drive the Lada far from the garage and back: it must not vanish.
7. Gear lever: the Lada lever moves through each gear in manual and automatic.
8. Wheel effects: dust on sand and none on asphalt, grass trails, rolling, spring and skid sounds, smoke and sand bursts while skidding.
9. Mouse driving: U toggles it in the car. Mouse X steers, mouse up is gas, down is brake, mouse look is off.
10. Steering wheel: with a real wheel or pad, bind every axis and gear button. Check H-shifter on and off and the live values. Steer a sled with the pad.
11. Find a small trailer and a bus trailer. Pick up the grip and put it on the Lada hitch. Drive, turn, reverse and brake.
12. Check the trailer rear and brake lamps follow the Lada lights, ignition and brake. Check the bus trailer brake switch.
13. Pick the grip off the hitch while the car stands and while it rolls. Save and load with the trailer on the hitch.
14. Tow a trailer far from where it spawned and back: car, trailer and grip must not vanish or freeze apart.
15. Check the trailer reflection probe.
16. Find Bus01, Bus02 and Bus03. Fill a bus with diesel and drive it. With petrol the engine must fail to start.
17. Bus01: both gauge parts in their kmh slots read speed. Check the counters and the gauge lamps.
18. Bus doors: the signal button lights green, the open button swings the leaves and lights red. Check the leaf sounds and the stretching colliders. Buttons do nothing without electronics power.
19. Bus02 trailer: put its grip on the bus hitch. Drive and turn. Its brake switch sets brake force 100 on each wheel. Its rear lamps follow the bus lights and brake.
20. Save and load with bus doors open and signalled and with the trailer on the bus.
21. Find Bike01 and Bike03. Ride Bike01: it stays upright, leans into turns, and falls over when nobody rides or pushes it.
22. Kick start Bike01: ignition on, roll it fast enough, the engine starts with its start sound. No starter motor.
23. Push a bike and the Lada by hand with their grips. The bike stays upright while you push it.
24. Bike03: pedal with W from rest. The pedals and feet turn. Kick start the engine by pedaling; pedals stop while the engine runs. Fuel comes from its tank parts.
25. Swap bike parts: wheels, mirrors, headlight, basket, bag, gauge, tanks. Save and load seated on each bike.
26. Pick each start car in the Game mode screen, then Random a few times. Each car starts, drives, saves and loads. StartCar06 diesel comes with 0.5 L diesel.
27. Two-stroke cars (car03, car09, car10): run them on a 2.5 % oil mix, then on plain petrol. The engine weakens without oil. Check the oil lamp.
28. Find roadside bare and full cars, also Car07 and a bare Lada, and no Car06P. Parts, fluids, rust and body variants differ between copies. No bus spawns as a roadside car.
29. Car07 shifts by itself in manual mode. Its dashboard clock shows game time, and its knob sets the time.
30. Drive past groups of roadside cars for 2 km. F3 bodies and generated objects fall back after you pass. Your start car and last car keep their area.
31. Tow a small trailer and drive Bus02 with its trailer: wheel meshes turn and steer. Bus02 leaf springs, axles and tipper rams follow the wheels.
32. Look at the Lada, a roadside car, a bus and each bike. The hover text shows the game name, such as "Volna Kombi", not "Car02kFull". The Game mode start car list shows the same names.

### Car camera and crash damage

1. Sit in the Lada driver seat. Tap C: the outside view orbits the car with the mouse. Pitch stops at about 89 degrees.
2. Hold Q and move the mouse: the view pans up to 1.5 m. Shift+Q pans forward and back.
3. Scroll: zoom from 1 to 30 m. Hold Tab: the view goes back to 5 m and the default angle.
4. Drive: the view turns behind the car, faster at higher speed.
5. Tap C: back to the cockpit. Repeat on the passenger seat and on a sled or bike seat.
6. Hold C for 0.4 s: the camera stays at its spot and turns with the mouse. Hold C again: it follows the car. Tap C: back to the view before.
7. Press M in a seat: the camera rises over 1/3 s and looks down. Scroll sets the height from 2.5 to 200 m. Q and the mouse pan. Tab recenters. Press M again: the camera comes back down.
8. In each other view: E does not get out or use things, prompts and the crosshair hide, worn items show, and the sunglasses tint is off. Sleep is not possible.
9. Drive the Lada into a wall at low, medium and high speed. Light parts (mirrors, hubcaps) fall off first. Fast crashes drop more parts. Worn parts fall off sooner.
10. Fallen parts do not lock into the trunk until you pick them up. Refit them with F.
11. A crash can sound the horn. A hard crash puts blood in the car with a sound. A second hard crash kills you.
12. Wash the blood off with the sponge. The next hard crash only puts blood in the car again.
13. Save and load with blood in the Lada and in a world car. The blood stays. Drive away from the world car and back: the blood stays.
14. Blow up a tank next to a car. Parts fall off.
15. Overheat a car engine above 110 degrees with coolant: steam rises under the hood and the boil sound grows with the heat. Check that steam is not lost inside the hood colliders. Car08 and Car08c have no steam.

### Saves

1. Save under two names. Each tile shows a game thumbnail, date, seed, km and version.
2. Overwrite, Load and Delete ask Yes/No in game. Load last loads the newest save.
3. Load, Load last and New game after death do not fail with "Asset CPU budget exceeded". Reload the page, then Load last from the start menu: the game starts, no "Source building metadata exceeds prototype CPU budget".
4. Play 2 minutes on foot. The autosave tile appears. Check the F3 cost of the autosave frame.
5. Turn on "Load last save at start" and reload the page. The last save loads.
6. Sleep, wake, save and load: the sleep hint stays off until the cooldown ends.

### Player body and outfit

1. Look down standing, crouching, walking forward and back. Carry a light and a heavy (>10 kg) item. Sit in the Lada (driver and passenger), on the WC and on the sled. Check legs, arms and hands on the wheel.
2. Check you cannot see inside the head.
3. Check the body shadow and the eye shadows. Check the siphon hose shadow follows the hose.
4. Compare F3 shadow draw counts and frame time near the garage and on the road with the previous test pass.
5. Drive the Lada. Use the horn, lights, ignition (tap and hold), handbrake and gear stick: a hand moves to each. Press gas, brake and clutch: the feet move to the pedals.
6. Use doors and handles. Push a car on foot and from a seat.
7. Carry a two-hand item.
8. Hold Left Alt: the hand points at the aimed surface. Rub dirt off a car and wash a wash trigger. Hold Left Alt at the sky: the hand waves.
9. Die on foot and in a moving car. The body falls with joint limits and carries the car speed.
10. The camera stays in the head. Mouse look fades out.
11. The death screen effect fades in over 14 s, then the menu opens. Read the console for DeadV parameters the port ignores.
12. Load a save: no ragdoll is left.
13. Put on and take off each wearable type (glasses, hat, cap, helmet, mask) and the cigarette with Tab and X. The cigarette comes off first.
14. Worn items are hidden in first person but cast shadows.
15. Save and load with worn items: they stay worn. Die: each worn item drops.
16. Open Appearance from the start menu and the pause menu. The preview figure shows next to the panel with the export camera and lights. Check the figure pose, the lights and the colors. Changes show at once. Back and Escape save. Reload the page: the outfit stays.
17. Set Male and Female with custom colors and disabled parts. Check them in game, in the shadow and on the ragdoll. Check the submesh colors of each segment.
18. Set the model modes Off, Shadow and Visible. Shadow hides the body but keeps its shadow.
19. Check the world view and its shadows do not draw the preview figure.
20. Put on glasses with each lens color. Each lens tints the view like the original, and the tint matches the visible lens color.
21. The dark lens (profile 0) dims the view by 2 stops.
22. Bright highlights look untonemapped, like the original.
23. Lenses with the flare flag hide the sun flare. Other lenses keep it.
24. Turn off "Sunglasses effects" in the display controls: the tint goes away. The flare rule stays. Reload the page: the setting stays.
25. Take the glasses off: the tint clears. Die with them on: the tint clears and the death effect fades in.
26. Save and load with the glasses worn: the tint comes back. Read the console for a warning about more than one active lens.

### Player movement

1. Park a bus on a slope, release the handbrake and stand inside while it rolls. The player rides inside.
2. Stand on a trailer bed or the Lada roof while it moves fast: you slide off. While it moves slowly: you ride.
3. Jump inside a moving bus: you land in the same place.
4. Run, jump, release W: you keep flying forward. Standing jump: straight up. Keys in the air turn you slowly.
5. Jump at a wall: one extra push. Jump at a ledge: a second push. Hold jump below the ledge: you grip and stop falling, then jump up.
6. Save mid-air and load: the jump continues.

### Weapons and damage

1. Drop and throw beer bottles, the vase and a box. Soft drops keep them whole. Hard hits break them with pieces and a break sound. The pieces scatter and fade.
2. Hit a building lamp with the shovel. It goes dark and stays dark when switched. Save and load: it stays broken.
3. Swing the katana, crowbar, fly swatter and leg with each attack key. Each plays its own animation and sounds and breaks a bottle. The hands follow the weapon grips.
4. Hit a buried item with the crowbar: it does not dig. The shovel still digs.
5. Hit a sausage with the shovel or a hurtFood weapon: it splits into pieces. Save and load: the pieces keep their bites.
6. Fire each gun: revolver, BB gun, SMG and the three shotguns. Check the shot sound, muzzle flash, recoil, tracer, sand or spark particles and hit sounds. A shot breaks a bottle. Shotgun pellets push a crowbar.
7. Check the HUD crosshair per gun, and its empty color.
8. Reload and unload with R and E from aimed or carried ammo. Round totals stay the same. The left hand moves to the gun.
9. Shotguns and BB gun: hold R and move the mouse to break the barrel. Closed barrels fire; open barrels reload. The BB gun gains power.
10. Revolver: hold R and scroll to turn the cylinder. Empty chambers click. Bullets show in full chambers.
11. SMG: load from a magazine item. No magazine: no fire. Save and load keeps ammo, barrel, chambers, power and magazine.
12. SMG: unload with E at loose ammo, or with R while you hold ammo. A magazine drops at the gun with the gun rounds, and the gun is empty. Load it back with R. Load a held magazine into a loaded SMG: the old magazine drops. Save and load: a dropped magazine keeps its rounds.
13. Shoot metal, wood, sand, glass and flesh: bullet holes show with the right texture, face out, move with items and fade per lifetime. Check the first hole for a hitch.
14. Shoot a world fuel can or tank with gas, oil or diesel over 40%. It explodes with particles and sound, breaks nearby bottles, pushes items and sets off a second tank. Save and load: the exploded items stay gone.
15. Shoot the full 20 L garage fuel can. It explodes, and the can and its cap go away. Save and load: they stay gone.
16. Hit an explosive tank with a melee weapon. It explodes as with a shot.
17. Shoot a tank that does not explode, and the garage water tank, with a shotgun. Holes that face down leak, and the fluid amount falls.
18. Stand near a blast: sound goes dull, an ear ring plays from the blast side, and both fade. Save and load keeps the ringing. Stand close: the player dies with the ragdoll.
19. Drive into a cactus, shoot one until it breaks, and blow one up. Cactus pieces fly off and the cactus stays, as in the original. It does not break again until its area streams in again.
20. Hit a tumbleweed hard with the car, a shot and a melee swing. It breaks with its gib and sound. Save and load: it stays gone.

### Creatures

1. See planes cross the sky at 600 m with their contrails and engine sound. The first plane comes at start, then one every 50 to 500 s, at most 3. About one in ten is a UFO: it spins and hums.
2. Bats: set the PC date between 15 Oct and 15 Nov, start a new game and drive to a gas station. In 65% of stations, 5 to 30 bats fly near it, flap and squeak. Pick one up: it keeps squeaking and does not fly out of the hand. Save and load near the bats.
3. Rabbits spawn 30 to 100 m away, first at start, then every 50 to 100 s, at most 10. Big ones are larger and slower.
4. They roam, run from the player and from shots, and jump over small obstacles. Their ears twitch.
5. They die on hard hits, falls and head shots, with a die sound. A head shot can take the eyes (eyes hide) or the ears.
6. Dead rabbits fall as ragdolls. Pick up and carry a bone: the body follows.
7. Save and load with live and dead rabbits, and with a rabbit that lost its eyes.
8. About one rabbit in four (the big ones) is evil and has glowing red eyes.
9. An evil rabbit that sees the player roars and stops, then runs at the player and attacks with a bite animation and sound.
10. A bite of a big evil rabbit hurts the player. It also damages loose items and other rabbits near the bite.
11. Close to an evil rabbit (about 4 m), it growls every 10 to 30 s.
12. Start a car engine or hold the horn. Normal rabbits flee from it. Evil rabbits flee from a loud sound far away, but still attack up close.
13. Turn on a portable radio near rabbits: they react to it like a quiet engine.
14. Save and load: big rabbits are still evil.

### Small effects

1. Drop and throw food and cut food pieces. Each makes its hit sound.
2. Eat each food. Crumbs, meat, turd and chocolate bits fall from the food. Smoke a cigarette: ash falls now and then.
3. Stay up at night for a few nights. The horror sound plays for 5 to 15 s on the first night, then every 1 to 5 nights. It stops at day.
4. New game between 15 October and 15 November: the start pumpkin glows and flickers at night. Hold it and wear it: the light follows.

### Advanced rendering

1. Antialiasing on (default): stand still in the garage. Edges of cars, wires and fences do not crawl or shimmer.
2. Turn quickly and walk sideways past wires and poles. Edges stay sharp, with no trails behind thin objects.
3. Drive the Lada at speed. The dashboard, wheel and hands stay sharp. The road and terrain behind them have no ghost copies.
4. Watch another moving car or a falling item from a standing view. It shows no trail.
5. Teleport (sandbox `building`), load a save and switch the car camera with C. The first frame after each shows no smear.
6. Turn Antialiasing off and reload. The image has hard edges and no TAA. Check F3 frame time with it on and off, standing and driving.
7. Lamp shadows: at night, turn on the Lada headlights, a road lamp and a house lamp near the car. Objects cast shadows from each lamp. Compare with the original: hard and soft edges, strength, fade with distance.
8. Look for shadow acne and gaps at contact points. Tune `BIAS_TEXELS` in `src/presentation/lamp_shadows.rs` if needed.
9. Stand near 5 or more shadowed lamps: only the 4 nearest cast shadows. No shadow pops when two lamps swap order.
10. Turn "Lamp shadows" off and on. Reload: the setting stays. Check the F3 cost with it on and off.
11. Check the page log for "layout requires ... textures": the lamp atlas adds one texture to each surface pipeline.
