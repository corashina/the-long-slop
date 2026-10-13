# Survival, sleep, and weather source audit

Date: 2026-10-13, Europe/Warsaw.
Status: Tasks 1–8 implemented and reviewed. Task 9 completed source inspection and documentation.
Final branch review and scoped fix review are complete. Both implementation defects are addressed.
See the [completion report](2026-10-12-survival-sleep-weather-completion.md) for current evidence and limits.

## Evidence

I checked the installed Steam manifest and assembly hash during this audit.
Steam app `1017180` uses build `11140890` and Unity `2021.3.18f1`.
The installed `Assembly-CSharp.dll` SHA-256 is:
`9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
The accepted decompilation receipt records this hash and no decompiler diagnostics.

Local source evidence:

During this audit, the source cache lives in `C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/.cache`.

- `.cache/reference-library/records/managed/Assembly-CSharp.cs`: decompiled control flow.
- `.cache/reference-library/records/managed/decompilation.json`: decompilation receipt.
- `.cache/reference-library/catalog.json`: object identities and resolved dependencies.
- `.cache/reference-library/records/<container>/<path_id>.json`: serialized fields.
- `.cache/reference/survival-weather-records.json`: 196 selected records from this audit.

The selected records include 173 interior components and no `snowfallscript` component.
Recovered source remains in ignored local storage.
This report records findings. It does not establish browser or installed-game parity.

## Systems present in this build

| System | Source behavior |
| --- | --- |
| Health | Blood quantity, damage, queued effects, healing, and death |
| Food and water | Depletion, temperature modifiers, queued consumption, and empty-state damage |
| Bowel | Metabolic conversion, excess food, eating restriction, and reduced healing |
| Bladder | Water conversion, excess drink, alcohol effects, and urination |
| Sleep | Hold input, cooldown, detached camera, accelerated clocks, and timed wake |
| Weather | Scheduled moving sandstorms, fog, sky, light, cloud color, particles, audio, and wind forces |
| Fatigue | No fatigue meter in the recovered player or sleep control flow |
| Rain | No rain scheduler or rain behavior in the recovered game assembly |
| Snow | `snowfallscript` exists; the catalog has no attached component or proven active scheduler |

`gamemodesettingsscript.defS.survival` is `2`, the enabled survival mode.
The serialized working settings contain `0`; startup loads settings before use.
`SetSurvival` selects `1` for disabled and `2` for enabled.
Only mode `2` depletes food and water and applies empty-state damage.
Other modes restore food and water to their maxima during updates.
Health, toilet state, and item effects have separate paths.

## Player needs

The active player is `level1:80125`.
Its survival pointer selects `level1:73239`, rather than the second survival component `73238`.
`survivalscript.Restart` supplies initial values. Serialized zero values are not the initial game state.

| Quantity | Maximum | Initial value |
| --- | ---: | ---: |
| Health | 5 | 5 |
| Food | 1 | 0.75 |
| Water | 3 | 1.8 |
| Bowel | 2 | 1 |
| Bladder | 2 | 0.75 total: 0.25 alcohol and 0.5 water |

The bladder record is `level1:73403`; its outlet is `level1:70534`.
The blood record is `level1:66379`, with five units of blood.
`Restart` clears queued food, water, bowel, and health changes and destroys wounds.
It does not reset bladder contents in that method.
Full scene startup supplies the serialized bladder contents.

| Field | Recovered value | Active use |
| --- | ---: | --- |
| `foodLoss` | 0.0004 per second | Food loss and water loss |
| `waterLoss` | 0.0024 per second | Declared; unused by the recovered update |
| Cold / reference / hot temperature | -5 / 20 / 40 | Temperature interpolation |
| Cold / hot food modifier | 3 / 0.3 | Nested interpolation below |
| Cold / hot water modifier | 0.3 / 3 | Nested interpolation below |
| Run food / water modifier | 3 / 3 | Declared; unused by the recovered update |
| Food to bowel | 0.75 | Each unit of metabolic food loss |
| Water to bladder | 0.85 | Each unit of metabolic water loss |
| Empty food damage | 0.0006 per second | Adds to empty water damage |
| Empty water damage | 0.00175 per second | Adds to empty food damage |
| Maximum healing | 0.0035 per second | Scales with needs and toilet fullness |
| Full toilet healing multiplier | 0.5 | Bowel or bladder fullness |
| Need multiplier during sleep | 0.75 | Multiplies the global sleep modifier |
| Queued effect speed | 2 | Separate delayed-effect update |

The update requires cursor lock and uses `dt = min(Time.deltaTime, 1)`.
Its global multiplier is `mainscript.sleepModifier`.
During sleep, it multiplies that value by `survivalscript.sleepModifier`.

The temperature expressions preserve the source's nested multiplication:

```text
cold = inverse_lerp(20, -5, temperature)
hot = inverse_lerp(20, 40, temperature)
food_rate = 0.0004 * sleep * lerp(1, 3, cold * lerp(1, 0.3, hot))
water_rate = 0.0004 * sleep * lerp(1, 0.3, cold * lerp(1, 3, hot))
```

At temperatures at or above 20, both rates use `0.0004 * sleep`.
Below -5, food uses `0.0012 * sleep`; water uses `0.00012 * sleep`.
The hot fields do not produce a hot-weather penalty through this expression.
The implementation must preserve these expressions unless the user approves a behavior change.

Healing uses the smaller food and water fractions.
It also uses the fuller bowel or bladder to interpolate between half and full healing speed.
The update applies starvation damage before healing.
Health below 25% triggers feedback; health above 25% clears that warning.
Exact equality preserves the prior warning state.

Queued effects use `fraction = min(Time.deltaTime * 2, 1)`.
Each update removes that fraction from each queue and applies it to the current state.
The global sleep multiplier does not multiply this queue fraction.
Excess food increases bowel state; excess drink changes bladder fluid.
Some source fluid paths bypass capacity clamps. Do not replace them with one uniform clamp without evidence.

Final review source checks confirm retained bladder order `[alcohol, water]` in tank `level1:73403`.
`ChangeOne` appends a new fluid type; later gasoline therefore follows water.
`fluidcontainer.ChangeWithoutMix`, managed lines 28368–28389, mutates that retained list.
`GetPercent`, lines 28304–28314, divides each amount by the current total after the prior mutation.
The fluid owner now keeps this order for urine debit and queued excess water.
The ordered path also sums the retained entries in source order.
`tankcapscript`, lines 63456–63458, calls `Tank.Upd()` after the urine debit.
`tankscript.Upd`, lines 63615–63624, removes the first nonpositive entry and stops.
The urine boundary now calls the fluid owner's one-entry cleanup. Queued excess water adds no capacity clamp or cleanup.
Source `GetPercent` has no zero-total guard. Exact intermediate zero totals can produce non-finite fractions.
These fixes preserve that source division; we ran no malformed-input or behavioral harness under the user waiver.

The final particle validator checks role, enabled-module, outlet, collision, fluid-color, and material map fields before construction.
It checks consumer bounds for layers, sizes, radii, lifetimes, rates, masks, and module enums.
Errors include particle source, role, and field. Validation accepts finite signed force, gravity, rotation, sorting fudge, and HDR colors.
Current production and fixture schema checks pass; this result does not establish malformed-input execution or runtime parity.

## Consumption and toilet actions

`fpscontroller` permits eating when `bowel + queued_bowel < 2`.
It consumes discrete food on the primary input press, rather than once per held-input step.
`ediblescript` advances its part index and divides each total effect by its part count.
It selects the next part group and destroys exhausted items with `destroyOnEnd`.

| Food | Parts | Total food | Total water | Total health | Total bowel |
| --- | ---: | ---: | ---: | ---: | ---: |
| Sausage (`Kolbasz`) | 6 | 3 | -1 | 0.375 | 0 |
| Bread roll (`Zsemle`) | 3 | 0.2 | -0.1 | 0.05 | 0 |
| Crescent roll (`Kiflike`) | 4 | 0.2 | -0.1 | 0.05 | 0 |
| Bacon (`Szalonna`) | 8 | 7 | -4 | 0.5 | 0 |
| Biscuit (`Linzer`) | 2 | 0.05 | -0.1 | 0.25 | 0 |
| Chocolate (`Csoki`) | 4 | 0.25 | -0.15 | 0.25 | 0 |
| Turd | 3 part groups | 0.1 | -1 | -2 | 0.2 |

A sausage bite queues `0.5` food, `-1/6` water, and `0.0625` health.
The static starter sausage uses edible record `level1:67140` and GameObject `level1:3542`.
The starter food belongs to `StartSpawnItems`, which `TerrainGenerationSettings.StartObjs` moves at startup.
The final selected runtime includes the starter sausage and its source part states.
The shared sausage edible record is `sharedassets1.assets:134643`.
The runtime uses sausage with six source part states and the required turd subset.
Smoking has separate continuous behavior and is not required for this loop.

Drinking affects each fluid fraction:

- Water queues water gain.
- Blood queues health gain with ratio `1`.
- Alcohol queues health gain with ratio `1` and converts water to bladder fluid with ratio `2`.
- Oil, gasoline, and diesel queue health loss with ratio `1`.

`fpscontroller.DrinkFill` uses source pour speed `2`, valve, outlet area, frame delta, and drink modifier `0.1`.
The fluid transfer adds its global sleep multiplier through `fluidcontainer.pour`.
The player drinking tank and particle-hit drinking path have different transfer rules.
Source filters depend on the current water and health deficits.
The source queues effects before its transfer clamps actual source debit.
Tests must distinguish requested effects from accepted fluid transfer.

Urination opens its valve toward one using interpolation speed `3`.
It latches empty at bladder amount `<= 0.01`; releasing the input clears that latch.
The outlet area is `pi * radius^2 * 0.025`, with radius `1` and no angle restriction.
Defecation removes `0.6` bowel units and spawns the source turd.
The burp/fart action uses random branches: `0.5` for fart/defecation, then defecation if bowel is at least `0.6`.
The second branch uses another random value greater than `0.5`.
The source blocks this action while its sound source plays.
These toilet actions are needed because fullness restricts eating and healing.

## Damage, recovery, and death

`DamageInstant` updates health from blood, subtracts damage, and clamps health to `[0, 5]`.
Zero health requests player death. Healing does not revive a dead player.
`fpscontroller.Death` checks god mode, flight mode, and startup protection.
`FallDamage` requests death when the absolute change in speed magnitude exceeds `13.5` metres per second.
This is a speed-change check, not a fall-height damage curve.
The player update skips this check during sleep.

Other source damage paths include hostile AI, explosions, weapon hits, and wounds with leaking blood.
Those paths require gameplay systems absent from this runtime.
Wound lifetime uses global `woundHealTime = 60`, multiplied by a random value in `[0.9, 1.1]`.
The audit does not authorize adding enemies, weapons, or explosion simulation.

`mainscript.Died` stops sleep, hides normal feedback, and starts a 14-second death transition.
Single-player source then loads `SceneMenu`; multiplayer source respawns the player.
The current browser runtime has no Unity menu scene, ragdoll, or multiplayer respawn system.
The current menu adapts the source death flow with New game and Load.
`GarageRuntime.recover` rejects dead players. It repositions bodies and retains needs, queues, consumption, cooldown, and storms.
New game and Load use the current candidate/commit lifecycle and advance the audio session.
The browser adapts SceneMenu with New game, Load, and a black death fade.

## Sleep rules and clocks

The main serialized parameters come from `mainscript` record `level1:64919`.
The earlier `.cache/reference/globals-debug.json` extraction contains the same sleep values.

| Parameter | Value |
| --- | ---: |
| Hold gain | 0.5 per simulation second |
| Sleep real time | 8 seconds |
| Sleep time added to the day cycle | 500 seconds |
| Cooldown after the scheduled wake | 800 simulation seconds |
| Global sleep modifier | `500 / 8 * 0.5 = 31.25` |
| Unity time scale during sleep | 2 |
| Need multiplier during sleep | `31.25 * 0.75 = 23.4375` |

Eligibility requires `Time.time > sleepEndTime + 800`, cursor lock, and no alternate camera view.
A new game sets `sleepEndTime = Time.time - 800`.
The strict comparison permits sleep after the first positive time advance.
Eligibility does not require a bed, stopped vehicle, fatigue, or full needs.
Releasing the input before entry clears hold progress.
The source starts sleep on the update after progress has reached one.
Sleep sets its deadline to `Time.time + 16` and wakes at or after that deadline.
Releasing the input after entry does not wake the player.
Pause sets normal time scale to zero and suspends the deadline.
Death ends sleep. The recovered controller has no movement or damage-only wake rule.

For eight uninterrupted real seconds of sleep:

| Clock or effect | Advance |
| --- | ---: |
| Normal simulation, storm movement, storm scheduler, and sleep deadline | 16 seconds |
| Day cycle | 16 normal seconds plus 500 added seconds |
| Need depletion, starvation damage, and natural healing | 375 baseline seconds, with temperature and fullness factors |
| Cloud noise coordinate | 500 baseline seconds |
| Fluid consumption and thermal-loss paths using the global sleep modifier | 500 baseline seconds |
| Engine heat-gain paths without that modifier | 16 baseline seconds |

The source has separate clock paths. One global time multiplier would change behavior.
`carscript` resets drive commands and skips `GetPlayerInput` while the player sleeps.
It retains seated driving state and automatic shifting; it does not force the vehicle to stop.
The day-cycle star rotation also adds the prior fixed update's `tekerve` to its normal rotation.
Sleep adjusts low and high cloud speeds through `Felho` and restores their normal speed on wake.
Exact totals depend on source update order and endpoint frames.
Fixed-step boundary tests must cover that order.

The source sleep camera samples a horizontal ring from 5 to 50 metres.
It casts down through mask `10125313` and chooses height from 0.1 to 10 metres.
A missed ground ray selects a camera 50 metres above the player.
The declared sleep angle-range fields have no active use in this method.
The camera location must rebase with the world and restore on wake.

## Weather scheduling and movement

Scheduler: `level1:64755`, `sandStormSpawnScript`.
Storm prefab: `sharedassets1.assets:158276`, reached through item database `level1:66024`.
The database has one storm prefab at index zero.

| Parameter | Value |
| --- | ---: |
| Spawn interval | Random 1,200 to 5,400 simulation seconds |
| Spawn ring around player | 15,000 metres |
| Storm size | Random 0.5 to 1.5 |
| Storm speed | Random 3 to 12 metres per simulation second |
| Influence radius | 750 metres |
| Destroy distance | 25,000 metres |
| Supported storm biome | 0 |
| Life gain in biome and range | 0.05 per simulation second |
| Life loss outside biome or range | 0.1 per simulation second |
| Initial life | 0 |

The scheduler resamples its deadline before spawning.
It spawns when `Time.time >= lastTime + nextTime`.
It samples spawn angle and storm orientation, then the storm samples size, direction, and speed.
It does not use the accelerated day-cycle clock.
Storm movement follows source terrain height.
Life decreases on a biome mismatch or distance at least 25,000 metres; zero life destroys the storm.
The source checks the previous update's distance during movement.

The visual strength is `size * clamp(life, 0, 1)`.
Inside 750 metres, local strength is `inverse_lerp(750, 375, distance) * visual_strength`.
The radius does not scale with size in this distance test.
`SandStormSet` selects the storm with greatest local strength.

## Weather effects

| Effect | Full source value |
| --- | --- |
| Fog density | 0.001 |
| Fog color RGBA | `(0.71698117, 0.48307487, 0.31452474, 1)` |
| Sky and ground RGBA | `(0.76415098, 0.63926613, 0.42893380, 1)` |
| Cloud RGBA | `(0.79245281, 0.53710723, 0.38501248, 1)` |
| Atmosphere thickness | 5 |
| Directional intensity multiplier | 0.45 |
| Ambient multiplier | 0.75 |
| Storm sound volume | 1 |
| Ambient sound decrease | 1 |
| Large / small particle rates | 1,000 / 1,000 per simulation second |
| Small particle height offset | -20 metres |

The day-cycle controller owns weather composition with phase-specific expressions.
Night fog, ground color, cloud color, and atmosphere handling differ from daytime handling.
Daytime atmosphere uses the cube of local storm strength.
The storm also reduces sun-flare brightness during daylight and transitions.
Sky tint updates in the one-second biome coroutine.
The implementation must extend current cycle outputs and reuse the current sky and lighting renderer.

Storm particle systems are `sharedassets1.assets:128154` and `128153`.
They use custom simulation space and interior trigger colliders.
Their source color uses gradients, particle-system time, random color mixing, and the day/night phase.
The particle rates above do not establish particle-rendering parity.

Wind scans the influence sphere once per simulation second through mask `75501569`.
It groups colliders by rigid body and sums their bounds dimensions.
It resamples a vector inside the unit sphere after a random 0.1 to 1.5 simulation seconds.
It interpolates the current vector toward that vector with factor `Time.deltaTime`.
The source force call supplies:

```text
current_vector * 5000 * Time.deltaTime * clamp(bounds_dimension_sum, 1, 30) * local_storm_strength
```

Unity applies this as a force, with another integration step.
A Rapier adapter must preserve force/impulse units and source update cadence.
Interior bounds suppress wind when they contain the body and belong to another root.
Storm particles also use interiors for suppression.
The source storm has no direct health damage or vehicle-condition damage call.
The empty `windscript` class does not establish an additional global wind system.

## Current runtime integration points

- `src/starting_area.rs`: `GarageRuntime` state, fixed simulation, reset, recovery, and floating origin.
- `src/app.rs`: frame input, pause, active elapsed time, fixed steps, presentation, and feedback.
- `src/simulation.rs`: fixed step at 1/60 second and bounded frame accumulation.
- `src/lighting.rs`: existing cycle state and clear-weather output.
- `src/presentation/lighting.rs`: existing fog, directional light, sky, and flare rendering ownership.
- `src/interaction/`: aimed actions, held items, openings, quantities, and feedback.
- `tools/reference/`: strict source export and validated manifest generation.
- `web/menu.js` and `web/input.js`: focus-loss pause, pointer lock, and input clearing.

The app accepts real frame time through one StepClock, with a 0.1-second frame limit.
GarageRuntime samples SleepState factors once per fixed slice.
CycleState receives normal simulation delta and the separate cycle extra span.
Needs use the combined sleep factor. Fluid and thermal-loss paths use the fluid factor once.
The cycle supplies desert temperature to needs and engine heat. Engine heat gain uses normal simulation delta.
Physics retains 1/60-second steps. Weather uses normal simulation time before physics.
Presentation reads outputs and advances no gameplay clock.

## Source dependencies and limits

| Dependency | Evidence and required treatment |
| --- | --- |
| World biome and terrain | Current world uses a procedural desert. Map it to source biome zero and report this map. |
| Food geometry and bite states | Final export includes starter sausage, six part states, and the required turd subset. |
| Food replenishment | Starter placement exists. A current-world station recipe requires a stated placement adapter, not a parity claim. |
| Interior shelter | The combined export supplies source shelters for home and both fleet vehicles, with roots and trigger links. Runtime AABBs adapt native overlap. |
| Particles | The combined export supplies 36 selected systems, including both vehicles. Retained rendering uses the existing FluidContact trajectories. See active depth/distortion limits. |
| Audio | One shared AudioState and AudioContext retain the full master schema, with ten added survival roles. Clips share source identities. |
| Death | Source ragdoll, Unity volume, and menu scene need adapters or explicit limits. |
| Wounds and hostile damage | Require weapons, enemies, blood leaks, or explosions. Exclude these absent gameplay systems. |
| Radio during sleep | The existing radio owner advances FM stations once at SleepCamSource gain > 0.5. Each loop condition redraws Random.Range(1,6). |
| Saves and multiplayer | Source saves needs, queues, sleep cooldown, and storms. Core saves project item fluids to finite, nonnegative quantities within capacity. Load restores that projection and resets new survival/weather states, food progress, generated food, and turds. Multiplayer respawn is absent. |
| Weather variety | Rain scheduling and active snow lack source support. Exclude them. |
| Random draws | Source ranges and draw order exist. Unity's native random sequence remains unrecovered; report that limit. |

## Verification status at delivery

The original plan required critical native, exporter, browser, lifecycle, and assembled acceptance checks.
Those checks cover needs, consumption, toilet paths, sleep boundaries, separate clocks, weather, wind, origin, and resets.
Task 3 retained earlier test logs and a garage-ready browser capture. Pointer lock blocked its gameplay acceptance.
The user later said “Skip tests” and “Continue. Ignore tests”. We skipped further behavioral checks and browser acceptance.
Earlier pass counts do not certify the final code. Behavioral assertions remain unchanged under the waiver.
Final source/schema, format, native/release/WASM, package, syntax, and shader-build receipts exited zero.
Task 9 verified unchanged final Task 8B artifact hashes and inspected lifecycle ownership.
Compilation and source evidence do not prove assembled behavior, audible output, or matched Unity appearance.
The [completion report](2026-10-12-survival-sleep-weather-completion.md) records receipts, adapters, unsupported dependencies, rulings, and test debt.

## Implemented clock fields

`SleepReference.need_modifier` owns `survivalscript.sleepModifier` from `level1:73239`: 0.75.
`SurvivalReference` no longer stores that field. Needs receive the sampled combined factor, 23.4375 during sleep.
`EngineGlobals.outside_temperature_modifier` owns `mainscript.outsideTempM` from `level1:64919`: 10.
The engine uses `10 * inverse_lerp(tempNights[0], tempDays[0] + tempAddNoons[0], current_temperature)` for outside heat.
The cycle output supplies current temperature to needs and engine heat.
Fuel, oil, coolant, transfers, emission quantities, and thermal loss use the sampled fluid factor once.
Engine RPM, starter deadlines, torque, and heat gain use fixed simulation seconds.
The cycle retains the previous step's extra span for star rotation. Cloud speed uses the source `Felho` curve.
The runtime accepts at most 0.1 real seconds per frame and discards excess stall time.

Instrument smoothing uses accepted normal simulation slices, including the twofold sleep rate.
`carscript` smooths fuel and temperature at source lines 8823 and 8828 with `Time.deltaTime * 5`.
`mutatoscript` uses `Time.deltaTime * 8 * lerpSpeedMultiplier` at lines 35619 and 35631.
The odometer uses `Time.deltaTime * 20` at line 13307. Cycle acceleration does not enter these paths.
The fixed-step frame sum quantizes smoothing at the existing physics interval.
