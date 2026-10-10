# Day cycle headlights and lamps

Date: 2026-10-10, Europe/Warsaw.
Status: Chat design approved. Written specification awaits user review.

## Intent and scope

Recreate the day cycle, headlights, and existing lamps from installed Steam build `11140890`.
Target source behavior and appearance within the current browser game.
Keep the fixed Lada, garage spawn, preparation controls, and procedural road.
Work on the separate `codex/day-night-lighting` branch.

Cover dawn, day, dusk, night, sky, sun and moon illumination, stars, clouds, ambient light, and fog.
Cover Lada headlights, cabin lighting, tail lamps, brake lamps, and reverse lamps.
Cover the five house switches and automatic roadside lamps.
Preserve pause, focus loss, recovery, restart, and floating origin behavior.

This specification supersedes the material specification's limit to startup lighting.
It also supersedes the UI specification's exclusion of headlight gameplay and its constant off headlight signal.
Preserve the other requirements of those specifications.
Share their lighting, material, interaction feedback, and instrument interfaces.

Portable flashlights, survival, sleep, weather events, audio, and vehicle lamp removal remain outside this scope.
Keep saves, multiplayer, other vehicles, and deployment outside the product scope.
Keep current engine heat and fluid rules. This change does not add temperature gameplay.

## Source authority

Use the [source findings](../../research/2026-10-10-day-night-lighting-reference.md) and installed scene component links.
Use `napszakvaltakozas` for cycle formulas, `headlightscript` for fades, and `flashlightscript` for house switches.
Use `carscript`, `rearlights`, and `electronicsscript` for vehicle light signals and power.
Use `kozlampa`, `roadscript`, and `roadGenScript` for roadside lamps.
Resolve the Lada's cabin controls through serialized usable events.
The obsolete `interiorLightScript` helper does not control the selected Lada's lamp.

Retain source object IDs, input hashes, and checked serialized values in export evidence.
Trace startup callers before accepting a serialized enable state, material, or intensity.
Use selected scene values and verify their relationship to shared prefabs.
Keep recovered code, assets, and captures outside version control.

Source behavior parity and rendered appearance require separate verification.
Claim pixel parity only after a matched live original-game comparison.

## Components and data flow

Retain Rust, Bevy, Rapier, Python export, and the existing browser bridge.
Use the pinned dependencies. Do not add a runtime package for this feature.

| Component | Responsibility |
| --- | --- |
| `tools/reference/lighting.py` | Recover source cycle, lights, controllers, materials, cookies, and pole placement records |
| `src/reference/lighting.rs` | Define and validate typed lighting records |
| `src/lighting.rs` | Own cycle time, lamp states, fades, and source output signals without Bevy dependencies |
| `src/interaction.rs` | Resolve aimed light controls and publish actions |
| `src/starting_area.rs` | Own lighting state beside the existing game state |
| `src/presentation/lighting.rs` | Bind lights and materials, apply output, and manage streamed lamp instances |
| `src/app.rs` | Order active time, input, simulation, imported transforms, and lighting updates |
| Browser input and feedback modules | Capture L and show valid source switch feedback |

Use the lighting modules named by the material plan. Keep one owner for each light and material binding.
Extend the shared lighting record inside the material rendering contract.
Keep controller state separate from light geometry and renderer handles.
Use stable node and source IDs. Use object names only as labels.

Load and bind records once. Publish numeric signals and borrowed state views to rendering and instruments.
Synchronize dynamic light poses after imported body poses and origin changes.
Reuse the existing instrument signal path for the Lada's high-beam indicator.
Export and animate the physical headlight and cabin switches where the selected source contains them.

## Export contract

Keep one export command, manifest, schema, and loading path.
Add required cycle and controller records to the shared source lighting contract.
Regenerate the installed export and native fixture. Reject older exports without compatibility paths.

Recover phase durations, startup offset, gradients, fog rules, sky parameters, stars, and cloud resources.
Recover clear-weather desert biome inputs from the source terrain rules.
Preserve the source one-second biome refresh where it affects visible output.

Each light record contains source identity, owner, local pose, type, color, range, cone, masks, and shadows.
Retain source intensity and the runtime rule that can overwrite it.
Include cookie textures, sampling rules, and their hashes.
Controller records link each state to its lights, on and off materials, button transforms, and aim colliders.
Recover authored shadow flags and material emission for all covered lamp surfaces.

Export the desert road pole hierarchy as a separate reusable scene asset.
Recover spacing, road-relative placement, light pose, and seeded pole rotation rules from the selected road prefab.
Export only the pole and lamp resources needed here. Preserve the current procedural road geometry.
Validate a pole's references without importing the original road generator.

Fail export on an unresolved required source link or rendering resource.
Fail loading on invalid values, missing hashes, unresolved nodes, materials, cookies, or shaders.
Name the failed source ID and field in the error.
Keep Start disabled until required lighting resources and bindings are ready.

## Cycle time and phase rules

Maintain an active elapsed clock separate from the capped physics accumulator.
Advance it with elapsed play time, including frames that drop physics catch-up steps.
Freeze it before Start, during pause, and while the game lacks focus.
Discard the resume frame's stale elapsed interval through the existing input-clear path.

Initialize the clock with the source offset of `1200 * 0.2 = 240` seconds.
Use seconds internally. The full source cycle lasts 1500 seconds.

| Phase | Source condition | Duration |
| --- | --- | --- |
| Dawn | `t <= 240` | 240 seconds |
| Day | `240 < t <= 1020` | 780 seconds |
| Dusk | `1020 < t <= 1200` | 180 seconds |
| Night | `1200 < t <= 1500` | 300 seconds |

Preserve the source strict comparisons, cycle reset order, and boundary output.
Do not add an extra dawn or dusk interval to the source day duration.
Preserve phase-specific interpolation. Do not replace the cycle with a sine curve or uniform color blend.
Use the original directional light for sun and moon illumination.

Apply source light colors, sun and moon rotations, ambient light, reflection strength, and sky exposure.
Day intensity is 1.125, noon intensity is 1.6, and night intensity is 0.3 in source units.
Day ambient is 0.2 and night ambient is 0.02.
The selected cycle's reflection intensity bounds are both zero. Preserve that source result.
Sky exposure spans zero through two. It is distinct from camera exposure.
Preserve source jumps between phases where the formulas produce them.

Apply the source fog model, density, color, and camera settings.
Use authored cloud colors, motion, and density rules for clear weather.
Use authored star resources, rotation, and visibility rules.
Recover source sun disk and flare rendering where those features affect the accepted phase views.
Keep sky geometry centered on the camera so origin shifts do not move it against the horizon.

## Headlights and vehicle lamps

Process L on its press edge while the player controls the driver seat.
Cycle off, low, high, then off. Consume one edge once during physics catch-up.
Route the physical headlight switch through the same source state change.
Do not bind the cabin lamp to L. Use its own aimed source control.

Retain requested state, current state, energy, and source controller enable state.
Drain energy before changing state. Restore energy after the state change.
Use `energyMove = 10` and preserve `headlightscript.Update` branch order.
Do not replace this sequence with a fixed transition timer.
Retain the source guard for requests made during an unfinished transition.
Initialize runtime controller values through source startup rules.

Multiply state intensity by energy and source electronics output.
Low beams use intensity 1.25, range 50 metres, and an 80-degree full cone.
High beams use intensity 1, range 250 metres, and an 80-degree full cone.
Use their shared source beam cookie and distinct source transforms and shadow settings.
Cabin lighting uses on-state intensity 0.8 and range 3 metres.
Preserve source on and off material assignments during transitions.

Headlight control does not require ignition. Preserve its source electronics behavior.
Rear lights require local ignition and their source attachment links.
Tail materials turn on for low or high beam state.
Brake lamps turn on above brake value 0.01.
Reverse lamps turn on for a negative selected gear ratio.
Derive these signals from the existing engine and transmission state.
Use source electronics availability for dashboard light signals. Add no battery drain or jamming gameplay.

## House and roadside lamps

Bind kitchen, garage, cellar, back, and entrance controls from the selected house scene.
Retain their source startup state: off, with six working lamp entries across five switches.
Use their source light objects, on and off materials, and button poses.
Preserve installed broken flags without adding new damage mechanics.

Resolve the aimed collider through its source switch ancestry and the existing reach and occlusion checks.
Toggle on Use press or allowed left-click press, following source action precedence.
Do not toggle twice when both inputs arrive together.
Keep door dragging, cap tuning, seat exit, carrying, and filling precedence consistent with the source UI design.
Show the source switch icon and action prompt only while the player can use that control.
Animate button poses through the recovered source transition rule.

Register each roadside lamp when its streamed pole enters the world.
Set its current state immediately from the cycle, including lamps first loaded at night.
Remove its renderer and light registration when its pole leaves the active window.
Use absolute distance identities so chunk seams cannot duplicate poles.
Preserve the source horizontal lamp orientation rule after road alignment.

Roadside lamps turn on during dusk when phase progress exceeds 0.5.
They turn off during dawn when progress reaches 0.5. They stay on at night and off during day.
Keep the source global enable flag and electronics threshold above 0.2.
Use the source warm color, 20-metre range, 120-degree full cone, and output intensity rule.
Keep placement on the existing road with source spacing and relative transforms.
Full original road shape and generator parity remain outside this change.

## Rendering and frame cost

Use Bevy light, visibility, render-layer, and shadow resources where they reproduce the required source behavior.
Convert Unity full cone degrees to Bevy half-cone radians once during loading.
Convert light units through an explicit documented mapping. Do not copy Unity intensity into lumens or lux.
Use the source project color space and the material specification's output transform.

Verify the pinned renderer's cookie support on WebGL2 before choosing the shader bindings.
Use its supported cookie path where it reproduces source projection and sampling.
Implement required source cookie and falloff behavior in the shared lighting shaders when the built-in path cannot reproduce it.
Ship one validated rendering path. Do not add device-dependent approximation modes.
Apply the same lighting behavior to imported materials and procedural terrain, roads, and station surfaces.

Preserve source shadow casting, receiving, bias, and light masks within the accepted WebGL2 implementation.
Verify beam occlusion by the car, walls, ground, and closed garage doors.
Retain a list of measurable renderer differences beside the visual comparisons.
Do not conceal missing shadows or cookies by increasing ambient brightness.

Allocate shared geometry, textures, materials, shader pipelines, and sky resources during loading.
Cache node, collider, light, and instrument bindings during setup.
Spawn and remove roadside instances only when the streamed world changes.
Per-frame updates change existing state, transforms, material selections, and uniforms.
Do not allocate collections, clone materials, resolve names, or rebuild shaders each frame.

## Pause recovery and restart

Pause and focus loss freeze cycle time, light fades, and switch motion.
Clear pending control edges so resume cannot toggle lights from an old press.
Recovery preserves cycle time and light states while moving attached lights with the car.
Floating origin changes preserve absolute pole positions, beam directions, and sky appearance.
Restart restores the 240-second offset, source lamp startup states, and original world registrations.
Remove old light instances before binding restarted instances.

## Acceptance and critical tests

Use the smallest critical tests that verify source behavior and integration.

- Verify cycle boundaries, source phase formulas, wrap order, startup offset, and elapsed time during dropped physics steps.
- Verify headlight request guards, fade order, rapid input, electronics scaling, and ignition independence.
- Verify tail, brake, reverse, cabin, and dashboard signals against the selected source components.
- Verify house reach, occlusion, input precedence, startup states, and on and off material links.
- Verify roadside state at dawn and dusk thresholds, late loading, chunk seams, and origin shifts.
- Verify pause, focus loss, resume, recovery, and restart without duplicate lights or retained input edges.
- Reject missing or invalid cycle records, resources, angles, masks, poses, state indices, and material bindings.

Run exporter tests, native tests, formatting, and the release WASM build.
Run the installed-export tests after generating assets inside the isolated worktree.
Check the browser at dawn, noon, dusk, and night with cockpit and walking views.
Check both beam states, the cabin lamp, each house switch, and night roadside streaming.
Confirm shadows, beam texture orientation, car movement, recovery, and a floating origin shift.
Measure steady frame cost with the covered lamps active. Inspect allocation and resource counts for growth.

Compare source and browser views with matching camera pose, field of view, time, lamp state, and material condition.
Record behavior evidence separately from image comparisons.
Report unverified live reference comparisons and measured rendering differences without claiming full-game parity.