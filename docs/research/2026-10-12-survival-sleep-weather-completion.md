# Survival, sleep, and weather completion

Date: 2026-10-13, Europe/Warsaw.
Branch: `codex/survival-weather`.
Execution base: `8daacbd1047883ea5a9aa00c82fca2597225ccae`.
Final fix base: `5abfb6c19a88d6b99f1cacf3c5cacf32198917b1`.

Tasks 1–8 implement the approved feature. Task 9 inspected integration and retained the evidence.
The final branch review found two P2 contract defects. Scoped review approved both fixes; no findings remain open.
Reviewed product head: 03964b7da1a27c55302195d532bf7a5eb5d1d7c8.

The user said “Continue. Ignore tests”, after “Skip tests”.
We skipped further behavioral test edits, test runs, browser acceptance, and audible checks.
Earlier test counts describe earlier code. They do not certify the final fixes.
Compilation and source validation do not establish assembled behavior or matched Unity appearance.

## Source identities

We used Steam build `11140890`, Unity `2021.3.18f1`.
The accepted source library and primary assets stayed read-only.

| Source | Identity or SHA256 |
|---|---|
| Assembly-CSharp.dll | `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e` |
| Accepted managed output | `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee` |
| UnityPlayer.dll native particle/audio evidence | `553c1e7560e1b42bd4638c53d0168dcd1b4b9eb5b9df324b0a44f14b3f18bc86` |
| Active player / survival / main / cycle | `level1:80125` / `73239` / `64919` / `64920` |
| Mouth / urine / shared tank / blood | `level1:67363` / `70534` / `73403` / `66379` |
| Starter sausage / edible / shared edible | `level1:3542` / `67140` / `sharedassets1.assets:134643` |
| Scheduler / item database / storm | `level1:64755` / `66024` / `sharedassets1.assets:158276` |
| Large / small storm particles | `sharedassets1.assets:128154` / `128153` |
| Urine / dormant urine splash / splash object | `level1:58924` / `59006` / `12756` |
| World / inside storm AudioSources | `sharedassets1.assets:123416` / `level1:56215` |
| Sleep / death / burp / fart AudioSources | `level1:56133` / `56499` / `56321` / `56131` |
| Desert day / night AudioSources | `level1:56222` / `56789` |

Task 9 checked both mouth and urine records. Both Tank pointers resolve to `level1:73403`.
The mouth has `isDrinkCap=1`. The urine outlet links particle `58924` and splash `59006`.
`Tankolas` requires a different tank before drink effects. Urine therefore has no self-drinking effect route.
The shared tank starts with 0.25 alcohol and 0.5 water; its capacity is 2.
The decompilation receipt matches the managed output hash and reports zero diagnostics.
See the [source audit](2026-10-12-survival-sleep-weather-reference.md), [particle record](../reference/particles.md), and [audio native record](../reference/audio-doppler.md).

## Implemented behavior and owners

| Owner | Implemented behavior |
|---|---|
| `SurvivalState` | Source needs, ordered metabolism, starvation, healing, queues, shared bladder, toilet gates, and terminal death |
| Item and interaction owners | One bite per primary edge, six sausage states, exhausted-item removal, direct drinking, contact receipts, and turds |
| `WorldStream` | Stable station food identities through streaming, consumption, recovery, and origin changes |
| `SleepState` | Source hold, strict cooldown, scheduled wake, phase factors, and cancellation rules |
| `StepClock` and `GarageRuntime` | Accepted real-time budget, fixed physics slices, normal simulation time, and endpoint factor changes |
| `CycleState` | Cycle time, desert temperature, sleep clouds/stars, source weather fog, light, sky, clouds, and flare |
| Engine and fluid owners | Source simulation, sleep fluid factors, temperature input, and thermal-loss routing |
| `WeatherState` | Source scheduler, absolute storms, life/movement order, influence selection, retained wind scans, and storm particles |
| `FluidContact` per outlet | One physical trajectory; rendering reads that trajectory. Urine uses the same implementation. |
| Existing presentation owners | Source food parts, sleep camera, need UI, death fade, and retained particle batches |
| `AudioState` and browser `AudioContext` | Shared playback, source gates, selected loops, bounded resources, session reset, and error reporting |

Startup uses health 5, food 0.75, water 1.8, bowel 1, and bladder 0.75.
Food and water loss use source `foodLoss=0.0004`, including the nested temperature expressions.
The recovered update leaves running modifiers and `waterLoss` unused.
Queued effects use normal simulation delta. Requested drink effects remain distinct from clamped source debit.
ToiletOutput owns urine debit. Particles do not debit the bladder again.

A sleeping slice uses time scale 2, needs factor 23.4375, and fluid/cloud factor 31.25.
The cycle receives 31.25 extra seconds per normal simulation second.
Rapier retains 1/60-second steps. Engine RPM and heat gain receive normal simulation delta.
Instrument smoothing receives the accepted fixed-step sum. Storm clocks receive normal simulation time.
`StepClock` accepts at most 0.1 real seconds and permits six awake or twelve sleeping steps per frame.
Sleep factors change at fixed-step endpoints. The next slice samples the new phase once.

Storm intervals remain 1200–5400 normal simulation seconds. The spawn ring remains 15000 metres.
The source radius is 750; destroy distance is 25000. Prior distance controls life evaluation.
Weather adds no direct health, traction, vehicle-condition, or temperature penalty.
Normal play calls `advance_weather` once before physics, after fluid work.
`CycleState` remains the sole sky/light owner. The audio owner handles both storm source bindings.

## Lifecycle inspection

These findings come from source inspection. We ran no lifecycle acceptance tests.

| Event | Inspected path and behavior |
|---|---|
| Pause / focus loss | `web/menu.js` clears input on blur, hidden document, Escape, and capture loss. Rust resets StepClock and gestures. Entered sleep, needs, storms, and queues remain. |
| Resume | Rust clears carry time and skips the first elapsed frame. Hidden time does not advance simulation. Browser capture uses a generation token. |
| Recovery | `GarageRuntime::recover` rejects dead state, interrupts sleep, retains its wake/cooldown endpoint, and refreshes physical/weather bindings. It retains needs, queues, consumed items, engine state, and storms. |
| Death | `announce_death` runs once, stops sleep, clears actions, and starts source feedback. The world continues through the 14-second transition. At its end, Continue cannot resume. Recovery cannot revive. |
| Restart | `GarageRuntime::restart` constructs a new runtime and increments the audio session. Food visuals reject prior-generation scene callbacks. Browser audio rejects old sessions and callbacks. Particle output reads new owners. |
| Floating origin | Storm particles and sleep camera retain absolute coordinates. Physics, item trajectories, urine, aim, and shelter bounds use the new local origin. Audio retains absolute histories and converts positions once. |
| Storm destruction / item removal | Weather removes invalid wind bindings and old storm loops. Presentation empties reusable particle slots. Food removal clears bodies, colliders, action bindings, joints, and visual poses. |
| Playback failure | Required duration/pitch data keeps simulation gates valid. Playback errors disable the shared route and report a diagnostic. Gameplay does not depend on successful playback. |

An interrupted death transition may resume before its deadline. Continue never restores health or clears death.
Pause suspends entered sleep. Recovery ends it while retaining the source scheduled cooldown endpoint.

## Adapters and unsupported dependencies

| Source dependency | Adapter or limit |
|---|---|
| Source world placement | Current procedural desert maps to biome 0 and current terrain height. Station sausage offset is `[1,0.5,-2]`. Station frequency has no source parity claim. |
| Unity wall time and update phases | Bounded real-frame time, fixed-step deadlines, coroutine scans, camera transitions, audio transitions, and instrument smoothing quantize source timing. Excess stall time is dropped. |
| Unity random engine | Deterministic scalar streams retain selected ranges and draw order. Native seed sequence and SIMD particle draws remain unrecovered. |
| Sleep recovery and source menu scene | Recovery ends sleep with retained cooldown. Browser Restart adapts SceneMenu. A black fade adapts death feedback. |
| Source ragdoll and postprocessing | The current runtime does not reproduce ragdoll or exact Unity death volumes. |
| Wind root parenting | Ordinary pickup retains item roots. Unsupported right-hand grip parenting stays outside current controls. |
| Other storm biomes | Current desert supports biome 0. Other-biome deferred-destruction wind behavior remains unsupported. |
| Shelter trigger geometry | Retained shelter AABBs replace native overlap with source radiusScale 0.75. Curved or rotated edges can differ. |
| Native fluid particle collision cache | Rapier swept spheres replace native collision timing and bounce paths. Generic world-tank filling has no selected contact route. |
| Smooth player head transforms | Existing capsule and view ray adapt urine emitter motion. Fine posture motion can differ. |
| Storm soft particles, material `sharedassets1.assets:50` | Ultra enables softParticles and `_InvFade=1`. Current materials lack sampled scene depth. Dust retains hard scene intersections. |
| Fluid distortion, material `sharedassets1.assets:481` | Active EFFECT_BUMP / _NORMALMAP uses strength 50, scaled strength 5, blend 0.1. Current materials lack scene color. Fluid sprites do not refract the scene. |
| Fluid soft particles and splashes | Fluid `_SoftParticlesEnabled=0` is dormant. Urine splash object 12756 and eleven selected container splashes are inactive. No supported activation or Emit link exists. |
| Native particle integration and rendering | Scalar fixed-step integration and 128-particle camera-facing batches can change spawn interpolation and cross-batch transparency order. |
| Source sky exit sampling | Zero influence returns clear output immediately. Unity can retain storm tint until its next one-second SetBiome tick. |
| Native audio management | Shared Web Audio routes, 512 pending events, and 128 retained voices adapt Unity resources. Source Doppler inputs and gates remain explicit. Audible output remains unverified. |
| Radio sleep advancement | Current runtime has no radio simulation. It does not advance source stations during sleep. |
| Wounds, hostile AI, projectiles, and explosions | Their damage and blood-leak systems remain outside this feature. |
| Multiplayer and saves | No multiplayer respawn or expanded save persistence exists. Current saves cannot claim preservation of needs, sleep, or storms. |
| Rain and active snow | No supported rain scheduler or attached active snow scheduler exists in the selected source. |

No other selected active particle module is omitted. We added no substitute blur, opacity change, distortion, or splash.
We added no runtime dependency, compatibility loader, migration, settings menu, rebinding, or deployment.

## Current build and source evidence

The final fixes use retained fluid order for bladder changes and remove one nonpositive entry after each urine debit.
Queued excess water still uses the uncapped source path.
Particle validation checks fields before fluid, storm, splash, and render owners can index their maps.
Errors identify the particle source, role, and field. Signed force, gravity, rotation, sorting fudge, and HDR colors remain valid.

| Current check | Evidence and result |
|---|---|
| Changed Rust format, full format check, native check/release, WASM release, bindings | `final-fix-build-2.log`; each command and Docker exited 0 |
| Managed order, fractions, cleanup, and initial tank | `final-fix-source.log`; accepted managed hash and tank receipt matched |
| Strict selected particle recovery, production and fixture schemas, material/texture hashes | `final-fix-validation.log`; each check exited 0 |
| JavaScript/Python syntax and diff whitespace | `final-fix-static.log`; each check exited 0 |
| Current artifact and frozen source hashes | `final-fix-hashes.log` and `final-fix-current-hashes.json` |

The first build stopped during native release after Docker lost its container connection, exit 125.
`final-fix-build.log` retains that failure. Docker reported no running container before the retry.
The retry completed the full build against frozen code. We added no runtime check or test harness.
The export, particle shader, emitted GLSL, and particle source document match the Task 8B hashes.
We reuse their static shader and source-export receipts below.

### Historical Task 8B and Task 9 receipts

Task 9 verified artifact sizes and hashes against the Task 8B receipts at that earlier head.
Documentation is the only Task 9 change. We reused those builds and did not rebuild unchanged code.

| Check | Evidence and result |
|---|---|
| Rust format, native check, native release, WASM release, binding generation | `task8b-final-verified-lf.log`; each command and Docker exited 0 |
| Shader build | Same log; Naga validated WGSL and generated WebGL2 GLSL ES 300. Vertex has zero samplers; fragment has one. |
| Strict selected source recovery and runtime schema | `task8b-validation.log`; final particle rederivation and runtime schema exited 0 |
| Corrected current fixture and final source/material receipts | `task8b-validation-complete.log`; schema, materials, textures, and selection exited 0 |
| Generated JavaScript syntax and Python compilation | `task8b-final-static.log`; each exited 0 |
| Task 9 source receipts and current hashes | `task9-source-hashes.log`; selected records, managed receipt, and eight artifact hashes matched, exit 0 |
| Task 9 final documentation checks | `task9-static.log`; diff whitespace, artifact unchanged checks, document contract, and evidence retention checks exited 0 |

Task 8B logs retain intermediate failures and their repairs. Their final receipts certify that earlier build.
Git emitted five CRLF conversion notices in Task 8B. We make no warning-free claim.

The earlier Task 8B WASM had 54,681,405 bytes and SHA256
`dbf8a05b30f6633b64beefc0ed6607892a98cee0e6dc72bf6d7a541e522477bf`.
The following table records the current final-fix artifacts.

| Current artifact | Bytes | SHA256 |
|---|---:|---|
| WASM | 54689967 | `3adbee6be9632ad21147713f3075b3a542f9fae86a2c293d566af24da487141f` |
| Generated JavaScript | 104793 | `57ecb910a598e15d730344dcef3ac42fa42f15fec3c9ddefd4d41ff09d1531c0` |
| Production manifest | 6726486 | `783ccf07f57c44a46a938f7ed1298981177ce83be7c42fb4e932e4b69d39ff3e` |
| Current schema fixture | 1046113 | `76b70ec8c2ecc762dc04b6644ee27d4ff5c523c956cfc67600343f72c8217e51` |
| Particle WGSL | 1993 | `7d3f2b20215db3075e57e4d9316f3c7d8ac0eaa23628ac890c22573c409c76bf` |

## Retained evidence

Keep `.cache/survival-weather/` in the owned worktree through integration.
`execution-evidence/` preserves SDD briefs, reports, reviews, raw logs, diffs, override, and the complete ledger.
`particles-evidence/` preserves selected native shape, activity, pose, module, and material receipts.
`task9-source-evidence/` preserves mouth 67363, urine 70534, tank 73403, decompilation receipt, and numbered managed excerpts.
`task9-current-hashes.json` preserves eight historical artifact hashes, including emitted GLSL stages and the particle reference.
`final-fix-current-hashes.json` records current artifacts and the three frozen source files.
`final-fix-report.md` records the fix commit, commands, source causes, self-review, and limits.
The cache retains final build scripts and full command output. The tracked reports retain source identities and limits.
Recovered source, selected assets, and local evidence remain ignored.

Task 3 captured an earlier garage-ready canvas. Its pointer-lock failure blocked gameplay acceptance.
That capture and earlier test counts do not certify this final build.
We created no controlled browser fixture, screenshot, console capture, clock trace, or production diagnostic API for Task 9.

## Skipped acceptance and stale test debt

We skipped new assembled tests, full checks, exporter unit tests, and browser lifecycle tests after the user waiver.
We skipped food/drink/toilet/sleep/storm/death acceptance, focus/recovery/restart/origin acceptance, audible output, and runtime shader inspection.
We did not verify finite runtime physics, source item removal in a browser, or matched Unity appearance.
Static source inspection, schema validation, and shader builds have different limits from runtime acceptance.

Known debt comes from static inspection and retained reviews. We did not run these targets.

| Path and location | Stale caller or assertion |
|---|---|
| `tests/consumption.rs:150` | `near_full_bladder_filters_water_and_retains_other_fluid` keeps old debit 0.54444444 and blood retention assertions. |
| `tests/consumption.rs:264` | `open_particle_contact_emits_once_and_reaches_bladder` expects an immediate proximity action, rather than collision-driven travel. |
| `tools/reference/tests/test_survival.py:58` | `test_recovers_active_player_and_source_values` reads `self.assertEqual(len(r['audio']),7)`. Audio now belongs to `source.audio['bindings']`, with ten final bindings. |
| `tools/reference/tests/test_survival.py:61` | The same test expects four particle capacities. Current strict recovery exports 26, including eleven outlet/splash pairs. |
| `tests/input.rs` | Removed `advance_lighting` / private `advance_light_fades` at 229, 231, 242, 253, 263, 310, 314, 347, 348, 415, 457. |
| `tests/lighting_interaction.rs:607`; `tests/parts.rs:44` | Obsolete external light-fade calls. |
| `tests/starting_area.rs` | Removed raw-frame lighting/fade/clock calls at 2188–2189, 2206–2208, 2212–2213, 2225. |
| `tests/vehicle.rs:336–338` | Removed `clock.advance` and zero-argument `clock.alpha` calls. |
| `tests/lighting.rs:85,93,94,100,107,108` | Exact f64 expectations remain after mechanical f32 CycleAdvance input conversion. |

The listed Rust API callers cannot compile until an authorized test-maintenance pass removes their obsolete assumptions.
We restored no product compatibility path to satisfy these tests.

Automatic approval review rejected two earlier behavioral assertion edits after “Skip tests”.
Task 3 could not update the near-full bladder assertion or its old contact/filter assertions.
Task 4 could not update `tools/reference/tests/test_survival.py:58`.
Its rejection stated: “The action edits an existing behavioral test despite the explicit user override forbidding further test edits; request a version that omits that file change.”
The agents left those assertions unchanged and completed unaffected product work. Task 9 had no approval rejection.

## Ledger rulings

These eleven Ruling entries come from the execution ledger. We retain their reasoning and revision costs verbatim.

Ruling: Use station sausage offset [1, 0.5, -2] from station absolute_point as the current-world placement adapter — source has no procedural station food offset, and proposed X=4 lies on the existing solid building face (center +6, half-width 2) — if wrong, adjust this adapter placement after geometry/ground-contact checks; source behavior and identity are unchanged. Task 3 implementer instructed to test overlap/access and record placement as an adapter, not source parity.

Ruling: Add bounded six-entry insertion order to FluidContainer for required source drinking paths — source fluidcontainer iterates a retained list and ChangeOne appends new types; six amounts alone lose that behavior — if wrong, revise this state representation and its callers; no runtime allocation, compatibility fallback, or second tank is authorized.

Ruling: Render outlet particles from Task3 FluidContact through a read-only view; reserve Task8 ParticleState for storm particles — required gameplay contact already owns source trajectories, and a second outlet trajectory would duplicate state and diverge contact from visuals — if wrong, revise the read-only particle view and renderer interface; preserve one gameplay trajectory and source effects. Task8 context carries this ownership clarification.

Ruling: Move the recovered 0.75 needs sleep modifier from SurvivalReference.sleep_modifier to SleepReference.need_modifier — SleepState::clock_factors receives SleepReference and must derive factors from one current owner, not duplicate or hardcode the source value — if wrong, revise field ownership and current callers/export/fixture; source value and source survivalscript identity remain unchanged. Original field removed, no compatibility schema.

Ruling: Route instrument smoothing through the accepted normal simulation clock in Task5 — remaining raw Time<Real> needle smoothing bypasses pause/stall limits and sleep scaling; reuse the same clock owner and exclude extra cycle span — if wrong, revise instrument smoothing cadence after source comparison; no second clock or unrelated instrument feature is added. Added presentation file must be reported and reviewed.

Ruling: Preserve gear control state and auto_gear memory during sleep, while normal source AutoShift may select neutral after drive commands clear — recovered carscript8387–8470 changes active ratio from cleared inputs; freezing AutoShift would stop required automatic simulation — if wrong, revise sleep gear policy after source comparison; add no direct gear reset and preserve ignition, seat and handbrake. Task6 also cancels unfinished ignition gestures without a synthetic release/tap.

Ruling: Remove expired storms at the end of advance after their Wind state/random updates in current desert biome0 — source Move uses prior distance>=25000 for loss, whose matching prior distanceLerp is zero, so the final supported wind impulse is zero — if wrong, retain final wind impulses through application before removal. Other-biome deferred-destruction effects remain unsupported; no unnecessary destruction hook is added. Source inspected52230–52350.

Ruling: Keep initial storm scan at pre-Move position; later scans use post-Move position after cached wind application — Unity2021.3 WaitForSeconds resumes after Update, while source Start begins its first scan immediately — if wrong, revise scan center/order after matched native capture. One bridge retains fixed-step deadline quantization, no second clock or pre/post lifecycle hook. Primary source https://docs.unity3d.com/2021.3/Documentation/Manual/ExecutionOrder.html#Coroutines.

Ruling: Split Task8 into8A composition/audio and8B particles/final activation — missing source fields, urine owner and retained GPU path make one change broad; sequential buildable stages reduce review risk — if wrong, reconcile interfaces during8B and finalreview; no feature requirement is removed and weather remains inactive until8B.

Ruling: Return original clear output immediately at zero storm influence, while sampling storm tint each normal second — Task8 brief explicitly requires zero influence to preserve clear output; native sky can retain old storm tint until next SetBiome tick — if wrong, revise this exit presentation rule after an explicit spec change; source phase formulas remain literal and the native stale-tint interval is reported.

Ruling: Preserve the source fog modifier and correct the Task8 literal expectation — source36193 multiplies the0.001storm parameter by DDS.fogModifier; fixture modifier3.2959184646606445 yields final density about0.0032959186 — if wrong, revise the output contract after source evidence; do not change correct source code or set modifier1. Plan/brief example now multiplies parameter by r.cycle.fog_modifier. No test file edited/run.
