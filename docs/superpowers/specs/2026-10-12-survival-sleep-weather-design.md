# Survival, sleep, and weather

Date: 2026-10-12, Europe/Warsaw.
Branch: `codex/survival-weather`.
Status: The user approved this written specification in chat on 2026-10-12.

## Intent and scope

Add a complete survival loop to the current garage-to-road game.
Use installed Steam build `11140890`, Unity `2021.3.18f1`, as the source.
Keep game state in Rust and presentation separate.
Retain the selected Lada, existing world, fixed physics steps, sky, and lighting owners.

Include health, food, water, bladder, bowel, consumption, natural healing, starvation damage, and death.
Add sausage as the required food type. Include the turd created by the required toilet action.
Use existing containers and fluid quantities for drinking.
Add source sleep eligibility, hold progress, interruption, wake, camera, and clock behavior.
Add source-scheduled sandstorms with movement, transitions, fog, light, wind, and selected particles.
Provide source-supported feedback and handle pause, focus loss, recovery, restart, and floating origin.

This scope replaces previous exclusions for survival, sleep, weather, and their required feedback.
It also replaces the lighting specification's requirement to retain fixed engine temperature inputs.
Keep unrelated product exclusions and existing gameplay requirements.
Do not add fatigue, beds, rain, active snow, enemies, weapons, wounds, other vehicles, or multiplayer.
Do not add save persistence, a settings menu, control rebinding, or deployment.
Do not add compatibility paths, old manifest support, or migrations.

## Source authority

Use the [source audit](../../research/2026-10-12-survival-sleep-weather-reference.md).
It records initial values, active formulas, item effects, clock behavior, weather parameters, and source identities.
The installed assembly SHA-256 is:
`9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.

Use typed records and managed control flow together.
Use `survivalscript`, `fpscontroller`, `ediblescript`, `mainscript`, and `tankcapscript` for survival and sleep.
Use `sandStormSpawnScript`, `sandstormscript`, and `napszakvaltakozas` for weather and clock composition.
Resolve the active player's survival pointer to `level1:73239`.
Resolve the main controller to `level1:64919` and the storm scheduler to `level1:64755`.
Resolve the storm prefab through item database `level1:66024`, rather than by a name search.

Preserve the source's use of `foodLoss` for water depletion.
Preserve its nested temperature expressions and unused running modifiers.
Do not infer behavior from unused fields or class names.
Retain identities, hashes, startup callers, and formula evidence in the export.
Keep recovered code, assets, and captures in ignored local storage.

## Ownership and data flow

Use pinned Rust, Bevy, Rapier, Python export, and the current browser bridge.
Add no runtime dependency for this feature.

| Owner | Responsibility |
| --- | --- |
| `tools/reference/survival.py` | Recover needs, items, fluid effects, toilet rules, sleep rules, and their bindings |
| `tools/reference/weather.py` | Recover scheduling, storm settings, shelter bounds, and selected particle settings |
| `src/reference/survival.rs` and `src/reference/weather.rs` | Define and validate required typed records |
| `src/survival.rs` | Own needs, queues, toilet state, item effects, health, and death state |
| `src/sleep.rs` | Own hold progress, eligibility, deadline, cooldown, and sleep clock factors |
| `src/weather.rs` | Own storm scheduling, movement, life, selection, and wind state |
| `src/starting_area.rs` | Own these states and connect them to actions, engine state, physics, and lifecycle |
| `src/lighting.rs` | Compose sleep and weather inputs into the existing cycle output |
| `src/presentation/weather.rs` | Render the selected source particle subset using retained buffers |
| Existing camera and lighting presentation | Apply sleep camera, sky, fog, light, clouds, and flare outputs |
| Existing interaction and item owners | Consume food, debit fluids, remove exhausted items, and emit successful-action events |
| Existing browser feedback and menu | Display Rust output and expose valid actions |

Rust modules must support native tests without browser or Bevy objects.
Presentation reads outputs. It must not advance needs, decide eligibility, or schedule storms.
Keep one owner for each item instance, storm, camera mode, light, and playback resource.
Build reference maps and caches during loading.
Reuse storage during simulation and rendering; avoid per-frame setup and allocation.

## Export contract

Extend the current manifest and the existing export command.
Require the new records in the current schema and replace obsolete fixtures.
Reject missing required records, unresolved non-null pointers, invalid bounds, and non-finite values before publishing.
Leave the accepted export intact if derivation fails.

Export source maxima, startup values, queues, loss formulas, healing factors, damage rates, and fluid effects.
Export input bindings through the existing source input contract.
Export sausage geometry, colliders, pickup rules, six part states, and exhaustion behavior.
Export the source turd subset needed by defecation.
Export sleep timing, camera placement, and cloud-speed settings.
Export scheduler ranges, storm movement and life settings, colors, gradients, particle settings, and wind rules.
Export shelter bounds with root identities and relevant particle-trigger relationships.
Keep source field names and method evidence beside derived runtime values.

Select required assets from the accepted library. Do not rebuild the complete library.
Validate current selected runtime assets with the existing reference validator.
Do not accept silent defaults for required source parameters.

## Survival loop

Start with survival enabled, the source default mode `2`.
Initial health is `5`, food `0.75`, water `1.8`, and bowel `1`.
Initialize the bladder from its source fluids: `0.25` alcohol and `0.5` water, capacity `2`.
Health, food, water, and bowel maxima are `5`, `1`, `3`, and `2`.

Advance metabolism, starvation damage, and natural healing in source order.
Use the cycle's source desert temperature, including night and phase transitions.
Keep temperature formulas from the source audit; do not add a storm temperature penalty.
Use `0.0004` for the base food and water loss expressions.
Convert metabolic food loss to bowel at `0.75`, and water loss to bladder water at `0.85`.
Apply empty food damage `0.0006` and empty water damage `0.00175` per source second.
Add both damage rates when both needs are empty.
Natural healing uses source maximum `0.0035`, need fractions, and toilet-fullness multiplier `0.5`.

Retain delayed food, water, bowel, and health queues.
Apply their source fraction `min(delta * 2, 1)` after metabolism and natural healing.
Apply source clamps at the individual source paths. Do not impose a new uniform fluid clamp.
Preserve the source distinction between requested drinking effects and clamped transfer debit.
Test both quantities and their effects at near-empty source and near-full player boundaries.

Permit eating only while bowel plus queued bowel is below its maximum.
Use one primary input press for each sausage bite.
Each bite advances one part state and queues `0.5` food, `-1/6` water, and `0.0625` health.
Remove the sixth exhausted item from simulation, interaction, physics, and presentation.
Keep ordinary pickup, carry, drop, throw, and origin behavior for remaining food.

Drinking must use source openings, valves, flow areas, mixture fractions, and eligibility.
Water restores water. Blood and alcohol restore health at source ratio `1`.
Alcohol also converts water to bladder fluid at ratio `2`.
Oil, gasoline, and diesel damage health at source ratio `1`.
Keep source filtering and capacity behavior in the existing fluid owner.
Do not duplicate fluid state in the survival module.

Include source urination and burp/fart/defecation controls.
Preserve the urination valve interpolation, empty latch, flow area, and release rule.
Preserve defecation eligibility, random branches, `0.6` bowel debit, sound-duration gate, and spawned turd.
The spawned turd uses its source discrete consumption effects if the player consumes it.
Use simulation time, recovered clip duration, and playback pitch for the action gate, independent of playback success.

Place the starter sausage from its source starter hierarchy and startup transform rules.
Add one sausage to the current procedural station supply recipe for continued consumption.
Use stable station and item identities; consumed station food must not return after streaming or recovery.
Station placement is a current-world adapter. Do not claim source placement or spawn-frequency parity.

## Damage and death

Health at zero enters death once and stops sleep.
Reject further player actions and disable healing as a revival path.
Retain source blood-to-health behavior through one authoritative health quantity.

Include the existing player's source speed-change death rule above `13.5` metres per second.
Preserve strict comparison and source protection through startup and recovery until grounding.
Measure consecutive active source speed samples; never treat a teleport or origin shift as physical acceleration.
Skip the source speed-change check during sleep, as the player controller does.
Do not convert this rule into a fall-height damage curve.
Exclude hostile AI, projectile, explosion, and wound damage because those gameplay systems are absent.

Use the source 14-second death transition and its decreasing look sensitivity.
Keep world and vehicle simulation running during the active transition.
At its end, pause through the browser menu and permit Restart.
Continue and Recover must not revive the player.
This menu transition adapts the source's single-player return to `SceneMenu`.
The current scope excludes ragdoll animation and exact Unity postprocessing volume parity.
Use existing presentation facilities for the required death feedback and report those appearance limits.

## Sleep and clocks

Use explicit idle, charging, and sleeping states.
Require cursor lock, an eligible camera view, and simulation time strictly past the source cooldown deadline.
Do not require fatigue, a bed, a stopped vehicle, or full needs.
Charge at `0.5` per simulation second and clear charge on input release before entry.
Enter sleep on the source update after the charge reaches one.
Set the wake deadline to current simulation time plus `16`.
Wake at or after that deadline and retain cooldown until `sleepEndTime + 800`.
Releasing the input after entry does not wake the player.

| Target | Awake | Sleeping |
| --- | --- | --- |
| Normal simulation time per accepted real second | 1 | 2 |
| Rapier step length | 1/60 second | 1/60 second, with twice the normal step count |
| Sleep deadline, cooldown, storm scheduler, and storm movement | Normal simulation time | Normal simulation time |
| Additional day-cycle time per simulation second | 0 | 31.25 |
| Needs, starvation damage, and natural healing | Factor 1 | Factor 23.4375 |
| Queued item effects | Source queue fraction | Source queue fraction using normal simulation delta |
| Cloud noise, sleep-scaled fluids, and thermal-loss paths | Factor 1 | Factor 31.25 |
| Engine RPM and unscaled heat-gain paths | Normal simulation delta | Normal simulation delta |

Eight uninterrupted accepted real seconds produce sixteen normal simulation seconds and about 516 day-cycle seconds.
They produce 375 baseline need seconds before temperature and fullness factors.
Apply the source star-rotation and `Felho` cloud-speed rules through the existing cycle owner.
Split time intervals at sleep entry, wake, and death so each interval receives one scale.
Suppress ordinary player movement and interaction during sleep while vehicle and world simulation continue.
Reset drive commands during sleep through the source `carscript.ResetInput` rule.
Retain engine ignition, selected gear, latched handbrake state, automatic shifting, and physical vehicle motion.
Do not turn off the engine or stop the vehicle because sleep starts.

Bound incoming active frame time before scaling, using the current 0.1-second frame limit.
Permit at most six awake or twelve sleeping physics steps per accepted frame interval.
Keep each physics step at 1/60 second. Never send a cycle-time jump to Rapier.
Reject negative or non-finite intervals and reset the accumulator on pause and resume.
Dropped stall time is a current-runtime safety adapter; report this difference from Unity wall-time behavior.
Do not run an unbounded catch-up loop after a suspended tab or delayed frame.

Pause and focus loss suspend entered sleep and its remaining time.
They cancel unfinished hold gestures and clear held controls.
Resume entered sleep without advancing the hidden interval.
Death ends sleep. Ordinary movement input and nonfatal damage do not add a wake rule.

Choose the sleep camera through the source ring, ground ray mask, height range, and missed-ray rule.
Store its world position in absolute coordinates and convert it through the current origin.
Restore the active gameplay camera on wake, death, recovery, and restart.

## Weather

Store storm positions in absolute coordinates.
Use source biome `0` for the current desert and current procedural terrain for ground height.
Report this biome and terrain map as a current-world adapter.

Schedule a storm after a random `1,200` to `5,400` normal simulation seconds.
Resample the schedule before spawning at a ring radius of `15,000` metres around the player.
Retain source random draw order within storm creation, ranges, initial life, size, speed, and travel direction.
Do not reduce the spawn distance to make storms easier to see.

Use radius `750`, destroy distance `25,000`, life gain `0.05`, and life loss `0.1`.
Preserve the source update order, including previous-distance use during life evaluation.
Use source distance and size/life expressions to determine influence.
Select the storm with greatest local influence for cycle composition.
Release storm resources after source destruction and discard its retained physics bindings.
Separate the storm random stream from item and toilet draws for stable native tests.
Report the unrecovered Unity native random sequence; matching ranges does not establish seed-sequence parity.

Compose source fog, directional intensity, ambient light, sky tint, atmosphere, clouds, and flare through `CycleState`.
Preserve phase-specific day, night, dawn, and dusk expressions.
Use source colors and gradients rather than a separate weather light or sky material.
Keep roadside and vehicle lamps under their current owners.

Scan wind bodies once per normal simulation second through the source layer mask.
Group colliders by body and retain the summed bounds dimensions.
Preserve source vector sampling, smoothing, influence, and interior-root exclusion.
Convert the source `AddForce` call to Rapier force/impulse units with the fixed integration interval.
Test the source's explicit delta factor; do not remove it as an apparent duplication.
Use retained scan storage. Remove invalid bindings after item or storm destruction.
Rebase retained bounds and particle space with the physics origin.
Add no direct storm health damage, vehicle damage, traction penalty, or temperature penalty.

## Presentation and audio

Use source need sliders, icons, thresholds, hold progress, cooldown feedback, and action labels where the library supplies them.
Keep values, eligibility, and visibility decisions in Rust outputs.
Publish compact changed state through the existing browser bridge.
Hide ordinary interactions while sleeping or dead.
Keep input help accurate for eating, drinking, sleep, and toilet actions.
Use source feedback timing: five-second need display with the recovered one-second fade interval.

Render the selected storm and fluid particle subset from recovered records.
Use source rates, lifetimes, shapes, velocities, gradients, materials, trigger suppression, and authored particle capacity.
Allocate particle buffers at loading or storm creation, using source bounds.
Reuse them during updates and rendering.
Limit export and renderer support to modules active in the selected effects.
List any unsupported enabled source particle module and its visible effect in the delivery report.
Do not claim full particle appearance parity without matched original-game evidence.

Use one shared audio owner, following the [source audio specification](2026-10-12-source-audio-design.md).
This scope authorizes eating, toilet, sleep, death, and storm bindings through `src/audio.rs` and `web/audio.js`.
It does not authorize the remaining audio project's features or a second AudioContext.
Initialize playback through the existing Start or Continue gesture.
Derive events from successful actions; playback failure must not alter gameplay or action gates.
Keep listener and storm emitter coordinates consistent during origin shifts.
Stop stale voices and particles on restart and storm destruction.
Pause playback and particle emission on focus loss without advancing logical state.
Report the absent radio simulation's source sleep-station advancement as an unsupported dependency.

## Lifecycle and errors

| Event | Required behavior |
| --- | --- |
| Pause or focus loss | Stop clocks and simulation; clear gestures; retain entered sleep and logical state |
| Resume | Clear stale input and elapsed time; resume retained state |
| Recovery | End sleep, restore camera, retain cooldown, health, needs, queues, item consumption, and storms |
| Restart | Reconstruct source startup state, food, bladder, queues, cooldown, storm schedule, and camera |
| Floating origin | Preserve logical absolute positions; move local physics, camera, particles, and emitters together |
| Death | End sleep once; block actions and healing; enter the source-timed browser death transition |
| Invalid required reference | Reject before starting gameplay; preserve the accepted export |
| Playback failure | Preserve simulation and action gates; report the audio error through the shared owner |

Recovery cancellation of sleep is an explicit current-runtime adapter.
It retains the scheduled wake endpoint for the source cooldown calculation.
Recovery must not revive death, refill needs, restore eaten food, or reset storms.
Restart uses source startup values rather than the source's limited `survivalscript.Restart` method alone.
It must invalidate prior-session visual and audio events.

Save persistence remains outside this feature.
The separate save/load feature will need to include these owners before it can preserve this expanded game state.
Do not ship a claim that current save/load preserves survival, active sleep, or weather without that integration.

## Acceptance and tests

Implement critical native tests for behavior and integration boundaries.
Use reviewed source fixtures and numeric expectations from the audit.
Keep tests independent of the implementation's own helper calculations.

| Area | Required checks |
| --- | --- |
| Rates | Temperature endpoints, empty needs, simultaneous damage, healing fullness, and delayed-effect order |
| Consumption | One press/one bite, six-bite exhaustion, eating restriction, mixture effects, requested effects, and actual debit |
| Toilet | Empty latch, release, source capacity paths, defecation eligibility, debit, random branches, and duration gate |
| Death | Zero health, no healing revival, speed-change threshold, startup protection, teleport protection, and menu deadline |
| Sleep | Hold release, strict cooldown equality, entry update, deadline equality, separate clocks, pause, focus, and death |
| Vehicle | Moving-vehicle sleep, engine fluids, thermal loss, stable fixed steps, and delayed-frame bound |
| Weather | Spawn deadline, radius edges, life edges, overlap selection, random ranges, shelter, and wind units |
| Presentation | Phase composition, source particle settings, camera restore, and stale event rejection |
| Lifecycle | Recovery retention, restart reconstruction, storm destruction, and floating origin |

Run source exporter tests, native Rust tests, existing browser lifecycle tests, and the release WASM build.
Run reference validation against the selected installed export.
Check format and whitespace before delivery.
Use browser acceptance to exercise eating, drinking, toilet actions, sleep, storm entry/exit, death, pause, recovery, and restart.
Use controlled storm placement only in acceptance fixtures; retain source scheduling in normal play.
Check shader errors, audible required feedback, finite physics state, and persistent consumed-item removal.
Match original-game captures before claiming appearance parity.

## Known limits and delivery contract

Report source-supported behavior separately from current-world adapters and unsupported dependencies.
The current-world adapters cover procedural food placement, desert mapping, frame-time bounds, recovery, and browser death return.
Current appearance limits include ragdoll, Unity death postprocessing, and unverified particle rendering.
Other unsupported dependencies include radio sleep advancement, hostile damage, wounds, multiplayer, and expanded save persistence.
Rain scheduling, active snow, fatigue, and general `windscript` behavior lack proven active source support for this scope.

A delivery must identify implemented behavior, tested boundaries, browser evidence, and remaining limits.
Do not mark this feature complete while its required survival loop, sleep lifecycle, or storm gameplay checks fail.
Write the implementation plan only after the user approves this written specification.
