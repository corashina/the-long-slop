# Survival, Sleep, and Weather Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the source-supported survival loop, sleep clocks, and sandstorms from build 11140890 to the browser game.

**Architecture:** Rust owns needs, sleep, weather, and lifecycle state. Existing interaction, fluid, camera, sky, and lighting owners consume these states. Deliver survival, sleep, and weather as three working layers.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, Rapier 0.36.0, serde, Python reference export, wasm-bindgen, JavaScript, and Web Audio.

**Spec:** [Approved specification](../specs/2026-10-12-survival-sleep-weather-design.md).
**Evidence:** [Source audit](../../research/2026-10-12-survival-sleep-weather-reference.md).
**Branch:** `codex/survival-weather`, isolated checkout based on `d7995c8`.
**Execution:** The user selected Subagent-driven execution through the active thread goal. Implementation is in progress.
**User override:** The user later said 'Skip tests'. Skip further test runs, new behavioral tests, and browser acceptance. Continue implementation, source review, formatting, and build checks. Earlier test results do not certify later changes.

## Global Constraints

- Use installed Steam build `11140890`, Unity `2021.3.18f1`, as the source.
- Keep game state in Rust and presentation separate.
- Add no runtime dependency for this feature.
- Do not add compatibility paths, old manifest support, or migrations.
- Reuse storage during simulation and rendering; avoid per-frame setup and allocation.
- Do not require fatigue, a bed, a stopped vehicle, or full needs.
- Keep each physics step at 1/60 second. Never send a cycle-time jump to Rapier.
- Do not reduce the spawn distance to make storms easier to see.
- Do not add save persistence, a settings menu, control rebinding, or deployment.
- Keep recovered code, assets, and captures in ignored local storage.

Use the root AGENTS.md writing rules and the stop-slop skill for UI text and documentation.
Implement critical tests. Use literal source expectations, not the implementation's helper results.
Keep the current game playable during the first three tasks. Activate the complete survival layer in Task 4.
Do not introduce runtime feature flags for staging these tasks.

## Review Focus

1. One input press across twelve steps consumes one bite. Task 3 tests the input edge.
2. Streaming, recovery, and origin shifts must not restore consumed station food. Task 3 tests its stable identity.
3. Sleep entry or wake within a frame must not apply both clock scales to the same interval. Task 5 tests mixed intervals.
4. Removed or sheltered bodies must receive no wind impulse from stale scan bindings. Task 7 tests both cases.
5. Failed audio and stale session events must not change toilet eligibility or resumed sleep. Tasks 4 and 6 test those cases.

## Files and ownership

| Files | Responsibility |
| --- | --- |
| `tools/reference/survival.py`, `tools/reference/weather.py` | Recover and check the selected source behavior and assets |
| `src/reference/survival.rs`, `src/reference/weather.rs` | Strict required manifest contracts |
| `src/survival.rs` | Needs, queued effects, toilet state, health, and death |
| `src/sleep.rs` | Sleep state, deadlines, cooldown, and clock factors |
| `src/weather.rs` | Storms, scheduling, wind targets, and source influence |
| `src/interaction/consumption.rs`, existing item and target modules | Eating, drinking, removal, and valid action feedback |
| `src/starting_area/survival.rs`, `src/starting_area/weather.rs` | Runtime integration, lifecycle, and physical bindings |
| Existing `src/starting_area.rs`, `src/app.rs`, `src/simulation.rs` | Ownership, input, fixed-step routing, and presentation order |
| Existing engine, lighting, camera, and world files | Source sleep factors, weather composition, and station supply state |
| `src/presentation/consumables.rs`, `src/presentation/weather.rs` | Food instances and the selected particle subset |
| `src/audio.rs`, `web/audio.js` and their reference export | One shared playback owner for the required feedback |
| `web/survival.js`, existing browser bridge and menu | Need displays, sleep display, and death menu actions |
| Native, exporter, and browser tests below | Critical boundaries and complete layer checks |

Add focused integration files instead of expanding the large runtime and interaction files with whole new systems.
Do not move unrelated code.
The audio paths are the shared owner from the approved source-audio specification.
Inspect that owner before editing if another branch has landed it. Reconcile its contract, rather than create a second owner.

## Workspace and commands

Run implementation commands in:
`C:/Users/Corashina/.codex/worktrees/survival-weather/the-long-drive-wasm`.
Reuse this attached worktree. Do not create another checkout or switch the shared audio branch.
Use the using-git-worktrees skill's existing-isolation path at execution start.

The accepted source and library remain at the original workspace:
`C:/Users/Corashina/Documents/Projects/the-long-drive-wasm`.
Use its existing `tools/reference/.venv/Scripts/python.exe`; Python's current directory must be this worktree.
Do not copy or rebuild the complete library.

Create an ignored `.cache/derive-survival-reference.py` in this worktree for development export.
It calls existing APIs with explicit paths:

```python
base = Path('C:/Users/Corashina/Documents/Projects/the-long-drive-wasm')
snapshot = load_snapshot(base / '.cache/reference-library')
receipt = verify_library(base / 'web/assets/library', snapshot.root)
derive_reference(snapshot, receipt, Path('web/assets/reference'))
```

Import these functions from `tools.reference.library.archive`, `tools.reference.library.validation`, and `tools.reference.export`.
Use the worktree's modules and the shared accepted receipts. Write only this worktree's selected runtime output.
The regular export command remains the product interface.

Commands used below:

- Rust target: `pwsh -NoProfile -File scripts/check.ps1 -Test <target>`.
- Full checks: `pwsh -NoProfile -File scripts/check.ps1`.
- WASM package: `pwsh -NoProfile -File scripts/build.ps1`.
- Python source tests: `& 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/tools/reference/.venv/Scripts/python.exe' -m unittest tools.reference.tests.test_survival tools.reference.tests.test_weather`.
- Browser unit tests: run the explicit test-file list in `scripts/check.ps1`, including the new files added in Task 4.
- Preview: `node scripts/serve.mjs`; select a free port through its existing `PORT` variable.

Shell wildcards must use the shell's supported expansion or explicit file paths.
Docker checks can require sandbox escalation. Preserve diagnostics and use the normal approval tool when needed.

### Task 0: Verify the execution baseline

**Files:** No product changes. Logs under `.cache/survival-weather/`.

- [x] Confirm `git status --short`, branch, HEAD, git-dir, common-dir, and absence of a superproject.
- [x] Verify source build and assembly hash against the approved audit.
- [x] Run full checks once and retain the output. Expect successful native tests, web tests, and release WASM compilation.
- [x] Verify the accepted shared library using `verify_library`; do not derive new assets yet.
- [x] Investigate any failure before feature work. Record unresolved baseline failures and seek the user's direction if they prevent progress.

## Layer 1: Survival

### Task 1: Add strict source records and consumable assets

**Files:**
- Create: `tools/reference/survival.py`, `tools/reference/weather.py`.
- Create: `src/reference/survival.rs`, `src/reference/weather.rs`.
- Modify: `tools/reference/export.py`, `tools/reference/manifest.py`, `tools/reference/parameters.py`, `tools/reference/ui.py`.
- Modify: `src/reference.rs`, `src/reference/ui.rs`, `src/presentation/reference.rs`, `tests/common/reference.json`.
- Test: `tools/reference/tests/test_survival.py`, `tools/reference/tests/test_weather.py`, `tests/survival_reference.rs`.
- Update affected strict path/count expectations in existing exporter and reference tests.

**Interfaces:**
- Python: `recover_survival(source: LibrarySource, roots: dict, output: Path) -> dict`.
- Python: `recover_weather(source: LibrarySource, output: Path) -> dict`.
- Python: `validate_survival_export(record: dict, manifest: dict) -> None` and `validate_weather_export(record: dict, manifest: dict) -> None`.
- Rust: required `ReferenceManifest.survival: SurvivalReference`, `.sleep: SleepReference`, and `.weather: WeatherReference`.
- Each reference exposes `validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError>`.
- `NeedValues { hp: f32, food: f32, water: f32, bowel: f32 }` describes initial and maximum needs.
- `ConsumptionEffect { food: f32, water: f32, bowel: f32, hp: f32 }` represents signed source effects and queues.
- Both value types derive Copy, Debug, Default, PartialEq, Serialize, and Deserialize.
- `SurvivalReference` contains `initial`, `maximum`, `initial_bladder: [f32; 6]`, `bladder_capacity_liters: f32`, and `foods: Vec<FoodReference>`.
- `SurvivalReference.starter_food: ItemId` identifies the one starter sausage. Add `ItemKind::Food` and validate this binding.
- `FoodKind` has `Sausage` and `Turd`. `FoodReference` owns `kind: FoodKind`, `effect: ConsumptionEffect`, and `bites: u8`.
- It also owns source part groups, pickup rules, and prototype geometry.
- Prototype geometry includes source mass, local collider geometry/masks, asset/root nodes, and ordered visible part groups.
- Other source constants use their snake_case source names. Sleep fields include `hold_gain`, `sleep_real_seconds`, `sleep_cycle_seconds`, and `cooldown_seconds`.
- Weather fields include `min_interval_seconds`, `max_interval_seconds`, `spawn_radius`, and `storm`.
- `WeatherReference.shelters: Vec<ShelterReference>` and `.particles: Vec<ParticleReference>` supply the shared shelter/particle contracts.
- `ShelterReference { source: SourceId, root: BodyId, geometry: ColliderGeometry, local_pose: Transform }` binds selected source interiors.
- `ParticleReference` owns source identity, checked rates, authored capacity, scalar/vector/color curves, shape/material settings, and trigger rules.
- Add `AssetUsage::FoodPrototype`. Prototype assets load once and create no initial physical instance.

- [x] **Step 1: Add critical source and validation tests.** Assert these literal values:

```rust
let r = common::reference();
assert_eq!((r.survival.initial.hp, r.survival.initial.food, r.survival.initial.bowel), (5., 0.75, 1.));
assert!((r.survival.initial.water - 1.8).abs() <= 1e-6);
assert_eq!(r.survival.maximum, NeedValues { hp: 5., food: 1., water: 3., bowel: 2. });
assert_eq!(r.survival.initial_bladder, [0., 0., 0.25, 0.5, 0., 0.]);
assert_eq!(r.survival.bladder_capacity_liters, 2.);
let sausage = r.survival.foods.iter().find(|f| f.kind == FoodKind::Sausage).unwrap();
assert_eq!(sausage.bites, 6);
assert_eq!(sausage.effect, ConsumptionEffect { food: 3., water: -1., bowel: 0., hp: 0.375 });
let turd = r.survival.foods.iter().find(|f| f.kind == FoodKind::Turd).unwrap();
assert_eq!(turd.bites, 3);
assert_eq!(turd.effect, ConsumptionEffect { food: 0.1, water: -1., bowel: 0.2, hp: -2. });
assert_eq!((r.sleep.hold_gain, r.sleep.sleep_real_seconds, r.sleep.sleep_cycle_seconds, r.sleep.cooldown_seconds), (0.5, 8., 500., 800.));
assert_eq!((r.weather.min_interval_seconds, r.weather.max_interval_seconds, r.weather.spawn_radius), (1200., 5400., 15000.));
```

Use tests named `recovers_active_player_and_source_values` and `rejects_missing_links_and_nonfinite_rates`.
Also test exhausted part indices, duplicate prototype kinds, invalid shelter roots, and invalid particle capacity.
Removing a required section from the fixture must make `ReferenceManifest::from_json` fail.

- [x] **Step 2: Run those tests and confirm failure.** Use the two Python modules and Rust target `survival_reference`.
Expect missing recovery functions or missing required reference sections.
- [x] **Step 3: Implement the interfaces and strict asset export.** Append starter sausage GameObject `level1:3542` to the selected items.
Export one `food.gltf` prototype package for the shared sausage and turd through their source pointers.
Recover startup transforms, source opening/drinking rules, toilet timings, input bindings, need UI records, and particle module flags.
Derive startup values with the source's float32 operation order; compare rounded report values with explicit float tolerance.
Resolve source data through `LibrarySource`; reuse accepted images, meshes, and WAV records.
Read source assertions from the managed receipt and audit. Do not select required clips or prototypes by names.
Extend current runtime path checks for these selected files. Replace fixed five-asset assumptions without accepting the old contract.
Keep unsupported particle modules in a source-identity limit ledger, including their visible effect.
- [x] **Step 4: Run the focused tests, derive the selected runtime, and validate it.** Expect passing tests and a valid current manifest.
Confirm food prototypes create no extra player, vehicle, or starter scene instance.
Build the WASM package and check that the existing garage still reaches readiness.
- [x] **Step 5: Commit only the exporter, reference, fixture, and test changes.** Message: `Add source survival sleep and weather records`.

### Task 2: Implement the native needs, toilet, and death model

**Files:** Create `src/survival.rs`; modify `src/lib.rs`; test `tests/survival.rs`.

**Interfaces:**
- Consumes: `SurvivalReference`, `NeedValues`, `ConsumptionEffect`, and existing `FluidContainer`/`FluidMixture`.
- `SurvivalState::new(reference: &SurvivalReference) -> Self`.
- `SurvivalState::queue(&mut self, effect: ConsumptionEffect)`.
- `SurvivalState::apply_drink(&mut self, reference: &SurvivalReference, fractions: [f32; 6], requested_liters: f32)`.
- `SurvivalState::advance(&mut self, reference: &SurvivalReference, dt: f32, temperature: f32, need_scale: f32) -> NeedsOutput`.
- `SurvivalState::damage_instant(&mut self, reference: &SurvivalReference, amount: f32)`.
- `SurvivalState::toilet_step(&mut self, reference: &SurvivalReference, input: ToiletInput, dt: f32, draws: [f32; 2], gate_seconds: f32) -> ToiletOutput`.
- `ToiletInput { urinate: bool, burp_held: bool, burp_pressed: bool }` and `ToiletOutput { burp: bool, fart: bool, defecated: bool, poured_liters: f32 }`.
- `NeedsOutput { needs: NeedValues, bladder_liters: f32, can_eat: bool, opacity: f32, dead: bool, death_seconds: f64, look_scale: f32, menu_due: bool }`.
- `show_stats(&mut self)` resets the source feedback duration. Held burp/urination and active drinking refresh it.
- `output(&self) -> NeedsOutput` and `pending(&self) -> ConsumptionEffect` provide read-only values.
- `SurvivalState` owns the player bladder container. World containers remain in `ItemStore`.

- [x] **Step 1: Write tests `source_rates_and_queue_order`, `toilet_fullness_and_gate`, and `death_does_not_heal`.**
Use initial values from Task 1. Define local test helper `close(actual: f32, expected: f32)` with tolerance `1e-6`.
Pin the rate and queue tests with these assertions:

```rust
let r = common::reference().survival;
for (temp, food, water) in [(20., 0.7496, 1.7996), (-5., 0.7488, 1.79988), (40., 0.7496, 1.7996)] {
    let mut state = SurvivalState::new(&r);
    let out = state.advance(&r, 1., temp, 1.);
    close(out.needs.food, food);
    close(out.needs.water, water);
}
let mut empty = r.clone();
empty.initial.food = 0.; empty.initial.water = 0.;
let mut state = SurvivalState::new(&empty);
close(state.advance(&empty, 1., 20., 1.).needs.hp, 4.99765);
let mut queued = SurvivalState::new(&empty);
queued.queue(ConsumptionEffect { food: 0.2, water: 0.4, bowel: 0., hp: 0. });
let out = queued.advance(&empty, 0.25, 20., 1.);
close(out.needs.food, 0.1); close(out.needs.water, 0.2);
close(queued.pending().food, 0.1); close(queued.pending().water, 0.2);
```

For healing tests, set `food_loss=0` in the cloned reference to isolate healing from same-update depletion.
Full needs and empty toilets then heal `0.0035` per second; full toilets heal `0.00175`.
Defecation removes `0.6` bowel; an active duration gate permits no second action.
Zero-health queued healing cannot revive. Death time `13.999` is before the menu; `14` reaches it.
Set initial values through a cloned reference to isolate each test.
Test health-warning equality at 25%, excess food, excess water, urination latch at `0.01`, and release.
Test all six drinking fractions, including immediate alcohol water conversion and signed queued effects.

- [x] **Step 2: Run Rust target `survival`.** Expect missing model interfaces.
- [x] **Step 3: Implement the model in source order.** Apply metabolism, empty-need damage, healing, then delayed effects.
Use source-path clamps and capacities. Retain the source's `foodLoss` water expression and signed food-water effects.
Gate toilet actions using source clip duration divided by selected playback pitch, not browser playback state.
Enter death once, preserve its timer, and suppress healing revival.
Reject invalid deltas without mutating state. Do not add an unused survival settings switch.
- [x] **Step 4: Run target `survival`.** Expect passing numeric and boundary tests.
Keep the model unconnected to browser metabolism until Task 4 finishes the survival layer.
- [x] **Step 5: Commit the model and tests.** Message: `Implement source player needs and death state`.

### Task 3: Connect food, drinking, toilet items, and removal

**Files:**
- Create: `src/interaction/consumption.rs`, `src/starting_area/survival.rs`, `src/presentation/consumables.rs`.
- Modify: `src/interaction.rs`, `src/interaction/items.rs`, `src/interaction/targets.rs`, `src/interaction/feedback.rs`.
- Modify: `src/starting_area.rs`, `src/world.rs`, `src/presentation/reference.rs`, `src/presentation/mod.rs`.
- Modify: `src/simulation.rs`, `web/input.js`, source control labels, and affected input/interaction tests.
- Test: `tests/consumption.rs`, `tests/world.rs`, `tests/items.rs`.

**Interfaces:**
- `GarageRuntime` owns `survival: SurvivalState` and `simulation_seconds: f64`.
- `GarageRuntime::consume(&mut self, reference: &ReferenceManifest, item: ItemId) -> Result<bool, ReferenceError>` returns whether a bite succeeded.
- `GarageRuntime::drink(&mut self, reference: &ReferenceManifest, source: ItemId, requested_liters: f32) -> Result<DrinkResult, ReferenceError>`.
- `DrinkResult { requested_liters: f32, debited_liters: f32, effect: ConsumptionEffect }` preserves the requested/debited distinction.
- `GarageRuntime::remove_consumable(&mut self, item: ItemId) -> Result<(), ReferenceError>` removes physical, interaction, and visual bindings.
- `FoodState { prototype_index: usize, bite: u8 }` lives on `ItemState` as `food: Option<FoodState>`.
- `ItemState.removed: bool` retains consumed identity without keeping a physical body active.
- `ItemStore::insert_consumable(&mut self, id: ItemId, prototype_index: usize, reference: &FoodReference, pose: VehiclePose, body: RigidBodyHandle) -> Result<(), ReferenceError>`.
- Add `InteractionAction::Consume(ItemId)` and `InteractionAction::Drink { source: ItemId, requested_liters: f32 }`.
- Pass `NeedsOutput` into `InteractionState::step` and valid-action derivation. Update all callers; retain no old overload.
- Rename the current `keys::FILL` bit `512` to `keys::PRIMARY` across all callers. Mouse0 remains bit `512`.
- Reserve `URINATE = 1 << 18`, `BURP = 1 << 19`; map the recovered source keys and update control help.

- [ ] **Step 1: Write `one_primary_press_across_steps_takes_one_bite`, `final_bite_removes_all_bindings`, and `station_food_stays_consumed`.**
Merge one primary press into `FrameInput`; take twelve step inputs; assert exactly one successful bite and five remaining bites.
Six separate presses must queue total food `3`, water `-1`, and health `0.375`, then remove the item.
Pin final removal before advancing queues:

```rust
let r = common::reference();
let mut runtime = GarageRuntime::new(&r).unwrap();
runtime.started = true;
let id = r.survival.starter_food;
let body = runtime.area.items.get(id).unwrap().body.unwrap();
for _ in 0..6 { assert!(runtime.consume(&r, id).unwrap()); }
assert!(runtime.area.items.get(id).unwrap().removed);
assert!(runtime.physics.rapier().bodies.get(body).is_none());
assert_eq!(runtime.survival.pending().food, 3.);
assert_eq!(runtime.survival.pending().hp, 0.375);
```

Eating at bowel plus queued bowel equal to `2` must change neither item nor queues.
Test a near-empty drink source: requested effects follow the source request, while actual debit cannot exceed stored liquid.
Capture mixture fractions before debit. Test closed/no-drink openings and input release.
Consume station food, unload/reload its chunk, recover, and rebase. Assert no replacement item or body.
Assert defecation creates one source turd with three part groups and its harmful eating effects.

- [ ] **Step 2: Run targets `consumption`, `world`, and `items`.** Confirm the new tests fail for absent consumption/removal behavior.
- [ ] **Step 3: Implement successful-action handlers and prototype instances.** Revalidate eligibility at mutation time.
Apply source requested effects before the source transfer/debit path; keep direct and particle-contact drinking paths distinct.
Reuse source mouth/outlet records and existing fluid operations, extending their source-specific filtering where required.
Remove exhausted bodies, colliders, joints, targets, carry state, source-body caches, and visuals through the runtime owner.
Keep a consumed tombstone for stable station identity. Allocate dynamic IDs from a retained per-session registry, with no hash collisions.
Add one source sausage per current station recipe. Keep this placement adapter separate from source starter placement.
Render the selected part group, including turd groups. Keep pick/drop/throw/origin behavior intact.
- [ ] **Step 4: Run focused tests and affected interaction/input tests.** Expect one bite per input edge and no stale physical handle.
Build the WASM package. Check starter food rendering and the existing garage interactions.
- [ ] **Step 5: Commit the consumption integration.** Message: `Add source food drinking and toilet interactions`.

### Task 4: Activate the complete survival layer and required feedback

**Files:**
- Create or extend shared owners: `tools/reference/audio.py`, `src/reference/audio.rs`, `src/audio.rs`, `web/audio.js`.
- Create: `web/survival.js`.
- Modify: `src/lib.rs`, `src/reference.rs`, `tests/common/reference.json`, and selected runtime asset validation for required audio records.
- Modify: `src/starting_area/survival.rs`, `src/starting_area.rs`, `src/player.rs`, `src/app.rs`, `src/browser.rs`.
- Modify: `web/main.js`, `web/menu.js`, `web/index.html`, `web/style.css`, `tools/reference/ui.py` and manifest integration.
- Test: `tests/survival_lifecycle.rs`, `tests/web/survival.test.mjs`, `tests/web/audio.test.mjs`, `tests/web/menu.test.mjs`.

**Interfaces:**
- `GarageRuntime::advance_needs(&mut self, reference: &ReferenceManifest, dt: f32, need_scale: f32)` uses cycle temperature.
- `GarageRuntime::needs_output(&self) -> NeedsOutput` supplies presentation and action gating.
- `PlayerState` exposes existing `grounded()`, `body_handle()`, and `view_pose()` for source death protection and speed sampling.
- `AudioReference` contains required source clip/binding IDs, WAV metadata, gains, pitch ranges, durations, spatial settings, and priority.
- Require `ReferenceManifest.audio: AudioReference` with `validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError>`.
- `GarageRuntime` owns `audio: AudioState` and a monotonic session ID. Add `pub mod audio` in `src/lib.rs`.
- `AudioKind` includes Eat, Burp, Fart, Urinate, Sleep, Death, Storm, AmbientDay, and AmbientNight.
- `AudioAction` has Start, Update, and Stop, covering one-shots and retained loops.
- `AudioEvent { session: u32, emitter: u32, kind: AudioKind, action: AudioAction, clip: u32, gain: f32, pitch: f32, position: [f64; 3] }`.
- Event positions are absolute. Convert them through the current origin when publishing float fields, after all fixed steps.
- `AudioState::new(reference: &AudioReference) -> Self`, `emit(&mut self, event: AudioEvent)`, `events(&self) -> &[AudioEvent]`, and `clear_events(&mut self)`.
- Reuse the audio specification's bounded priority queue and retained voices. Export `recover_audio(source: LibrarySource, output: Path) -> dict` for this selected subset.
- `gameSurvival(hp, food, water, bowel, bladder, opacity, dead, lookScale, menuDue)` receives scalar Rust output.
- `gameAudio(ids: Uint32Array, values: Float32Array)` uses separate integer and float fields, preserving dynamic item IDs.
- Integer tuples are `(session, emitter, kind, action, clip)`; float tuples are `(gain, pitch, x, y, z)` in corresponding order.
- `gameAudioListener(session, x, y, z, fx, fy, fz, ux, uy, uz, masterGain, originX, originY, originZ)` publishes scalar listener state.
- `createAudio({bindings, onError})` returns `resumeFromGesture()`, `publish(ids, values)`, `pause()`, `reset(session)`, and `dispose()`.
- It also exposes `setListener` with the scalar arguments of `gameAudioListener`, using retained listener and origin fields.
- Publish listener/origin state before events. Shift retained emitters on origin changes and apply source death master-gain fading.
- `createSurvival(ui)` returns `update(output)` and `setVisible(visible)`. It performs no game calculation.
- Its display object has the flat fields of `gameSurvival`; Rust computes opacity and eligibility.

- [ ] **Step 1: Write `recovery_retains_needs_and_restart_rebuilds_startup`, `speed_change_death_uses_source_threshold`, and browser lifecycle tests.**
At a speed change of `13.5`, the player lives; above it, an unprotected player dies.
Recovery and rebase must not count as speed changes. Grounding ends startup/recovery protection.
Recovery retains health, queued effects, and consumed food. Restart restores Task 1 initial values and the starter item.
At death time `14`, show the browser death menu. Continue and Recover cannot revive it; Restart can create a new session.
Pin the native death/restart boundary:

```rust
runtime.survival.damage_instant(&r.survival, 5.);
runtime.recover(&r);
assert!(runtime.needs_output().dead);
runtime.restart(&r).unwrap();
assert!(!runtime.needs_output().dead);
assert_eq!(runtime.needs_output().needs, r.survival.initial);
```

Test need display at five seconds and the one-second fade interval.
Reject stale audio sessions. Fail clip decoding and playback; assert unchanged needs and toilet gate duration.
Across repeated output frames, assert one AudioContext and retained loop voices, rather than new contexts.

- [ ] **Step 2: Run target `survival_lifecycle` and the affected web test files.** Confirm failure for missing feedback and lifecycle wiring.
- [ ] **Step 3: Activate needs in the fixed runtime update and connect feedback.** Initially use awake factors only.
Create source input and feedback events at successful mutation points; publish them once after fixed steps.
Use source sliders/icons and recovered five-second display timing. Keep ordinary actions hidden when dead.
Wire the source speed-change death check, startup/recovery protection, 14-second timer, look scale, and browser menu return.
Keep world and vehicle simulation active through the death transition; pause at its end.
Initialize the one shared AudioContext inside Start/Continue's gesture, before awaiting pointer lock.
Load the selected source eating, toilet, and death clips. Gameplay continues if browser playback fails.
Update `scripts/check.ps1` to include the new critical web test files; retain existing checks.
- [ ] **Step 4: Run focused tests and the release build.** Exercise eating, drinking, toilet actions, depletion, healing, death, recovery, and restart in the browser.
Verify source mixture effects with the native fixture and valid container interactions with the installed export.
Capture this first working layer under `.cache/survival-weather/layer-1/`; record shader errors and audible feedback.
- [ ] **Step 5: Commit the survival layer.** Message: `Activate survival feedback and death lifecycle`.

## Layer 2: Sleep

### Task 5: Implement sleep state and separate clock routing

**Files:** Create `src/sleep.rs`; modify `src/lib.rs`, `src/simulation.rs`, `src/lighting.rs`, `src/app.rs`.
- Modify: `src/starting_area.rs`, `src/starting_area/survival.rs`, `src/simulation/engine.rs`, `src/fluids.rs`.
- Modify: engine reference fields/export, common fixture, and all existing cycle/clock callers.
- Test: `tests/sleep.rs`, `tests/input.rs`, `tests/engine.rs`, `tests/lighting.rs`.

**Interfaces:**
- `SleepPhase` has Idle, Charging, and Sleeping.
- `SleepInput { held: bool, active: bool, camera_allowed: bool, dead: bool }`.
- `SleepOutput { phase: SleepPhase, eligible: bool, charge: f32, wake_time: f64, cooldown_remaining: f64, progress: f32 }`.
- `ClockFactors { time_scale: f32, need_scale: f32, fluid_scale: f32, cycle_extra_per_sim_second: f32, cloud_scale: f32 }`.
- `SleepState::new(reference: &SleepReference) -> Self`, `step(&mut self, reference: &SleepReference, input: SleepInput, now: f64, dt: f32) -> SleepOutput`.
- `clock_factors(&self, reference: &SleepReference) -> ClockFactors`, `output(&self) -> SleepOutput`, `cancel_charge(&mut self)`, and `interrupt(&mut self)`.
- `StepClock::accept_frame(&mut self, elapsed_seconds: f64)`, `take_step(&mut self, time_scale: f32) -> bool`, `alpha(&self, time_scale: f32) -> f32`, and `reset(&mut self)` replace `advance`.
- `CycleAdvance { simulation_delta: f32, cycle_extra: f32, cloud_scale: f32, sleep_progress: Option<f32> }`.
- `CycleState::advance(&mut self, reference: &CycleReference, advance: CycleAdvance)` replaces the old scalar API.
- Rename `CycleOutput.visual_temperature` to `ambient_temperature`; update all callers and its source comment.
- Add `EngineInput.sleep_modifier: f32`, default `1`. Remove obsolete `EngineGlobals.awake_heat_modifier`.
- Export `EngineGlobals.outside_temperature_modifier` from source `outsideTempM = 10`.
- Add `GarageRuntime::clock_factors(&self, reference: &ReferenceManifest) -> ClockFactors`.
- `GarageRuntime` owns `sleep: SleepState`. `SleepOutput` and `ClockFactors` derive Copy, Debug, and PartialEq.

- [ ] **Step 1: Write `sleep_hold_cooldown_and_wake_boundaries`, `sleep_clocks_are_separate`, and `mixed_frame_scales_do_not_double_count`.**
Use direct deadline cases and source literals. Add these clock-factor assertions after entering sleep:

```rust
let factors = state.clock_factors(&r.sleep);
assert_eq!(factors.time_scale, 2.);
assert_eq!(factors.need_scale, 23.4375);
assert_eq!(factors.fluid_scale, 31.25);
assert_eq!(factors.cycle_extra_per_sim_second, 31.25);
assert_eq!(factors.cloud_scale, 31.25);
assert_eq!(8. * factors.time_scale, 16.);
assert_eq!(16. * factors.cycle_extra_per_sim_second, 500.);
assert_eq!(16. * factors.need_scale, 375.);
```

Assert ineligibility at new-game time zero, and eligibility after a positive time advance.
Charge gain is `0.5`; charge reaches one after two simulation seconds, followed by the source entry update.
Release before entry clears charge. Equality at wake time wakes; equality at wake plus `800` remains ineligible.
At 20 degrees, sleep removes about `0.15` food and water over the uninterrupted interval.
Test pause/death during charging and entered sleep, and release after entry without early wake.
Test negative, NaN, infinite, and ten-second frame deltas. A valid stalled frame advances at most `0.1` accepted real seconds.
Assert no more than six awake or twelve sleeping fixed steps; each step retains `FIXED_DT`.
Pin engine RPM/unscaled heat to simulation time, and fuel/oil/coolant/thermal loss to the source sleep factor.

- [ ] **Step 2: Run targets `sleep`, `input`, `engine`, and `lighting`.** Expect missing sleep state and changed clock APIs.
- [ ] **Step 3: Implement state and replace obsolete clock routing.** Clamp incoming real delta before scaling.
`take_step` spends `FIXED_DT / time_scale` real seconds. A frame touching sleep has budget twelve; an awake frame has budget six.
Keep only fractional carry after the budget. Reject invalid delta without changing the accumulator.
Sample factors before a step; apply entry/wake/death at its endpoint, then sample factors for the next step.
Use normal simulation time for deadlines and storms. Send only the additional cycle span to cycle advancement.
Apply source queue fractions without the metabolism multiplier.
Apply source fluid and thermal factors once. Keep engine RPM, start timers, torque, and unscaled heat gain on fixed simulation delta.
Compute outside heat from current source temperature and the exported outside-temperature modifier.
Retain the source previous-step star extra and the `Felho` cloud-speed curve. Restore normal cloud speed on wake.
Remove the separate raw real-frame `advance_lighting` path. Use one authoritative accepted interval route.
Route lamp fades and shader time through normal simulation time, including sleep's twofold rate.
Remove the raw real-frame `advance_light_fades` caller; advance the existing fade owner from accepted simulation slices.
- [ ] **Step 4: Run focused targets and release WASM build.** Expect numeric boundary tests and existing lighting/engine checks to pass.
Use fixed-step tolerance for endpoint frames; do not widen tolerance to conceal a wrong multiplier.
- [ ] **Step 5: Commit clock routing.** Message: `Add source sleep clocks without large physics steps`.

### Task 6: Connect sleep input, camera, feedback, and lifecycle

**Files:** Modify `src/starting_area/survival.rs`, `src/starting_area.rs`, `src/app.rs`, `src/browser.rs`.
- Modify: `src/presentation/camera.rs`, `web/input.js`, `web/survival.js`, `web/main.js`, `web/menu.js`, and control help.
- Extend shared audio bindings for sleep. Test `tests/sleep_lifecycle.rs`, `tests/web/survival.test.mjs`, `tests/web/menu.test.mjs`.

**Interfaces:**
- `keys::SLEEP = 1 << 17`; use its recovered source key.
- `SleepCamera { absolute_position: [f64; 3], rotation: [f32; 4] }` is logical Rust state in `src/sleep.rs`.
- `GarageRuntime::sleep_camera(&self) -> Option<SleepCamera>`.
- `gameSleep(phase, eligible, charge, cooldownRemaining, progress)` publishes Rust outputs.
- `createSurvival(ui).updateSleep(output)` displays recovered hold and cooldown UI.
- Existing `clear_input`, `recover`, `restart`, and camera update use the sleep state; add no parallel lifecycle owner.

- [ ] **Step 1: Write `pause_suspends_sleep_and_recovery_retains_cooldown`, `sleep_camera_rebases_without_jump`, and `moving_vehicle_sleep_keeps_ignition`.**
Pause halfway through sleep; resume after an arbitrary hidden interval. Assert unchanged remaining time and needs while hidden.
Recovery ends sleep and retains scheduled wake endpoint, cooldown, needs, queues, consumed items, and engine ignition.
Releasing sleep input after entry does not wake. Death ends sleep once.
Test camera ring bounds `[5, 50]`, height `[0.1, 10]`, source ray mask, and the missed-ray 50-metre placement.
Test no pointer-lock capture after a focus-loss race, and no stale sleep voice after restart.
With a moving seated vehicle, assert cleared drive commands and continuing finite physical motion.
Pin suspension after the fixture has entered sleep:

```rust
let before = runtime.sleep.output();
let needs = runtime.needs_output().needs;
runtime.paused = true;
for _ in 0..600 { runtime.step(&r, FrameInput::default()); }
assert_eq!(runtime.sleep.output(), before);
assert_eq!(runtime.needs_output().needs, needs);
```

- [ ] **Step 2: Run target `sleep_lifecycle` and affected web tests.** Expect missing input/camera/lifecycle integration.
- [ ] **Step 3: Implement source input and camera behavior.** Use existing physics queries for the source ground ray.
Store camera position in absolute coordinates; convert through `WorldOrigin` for rendering and audio listener placement.
Restore the active gameplay camera on wake, death, recovery, and restart.
Suppress player interactions and drive commands during sleep; retain seat, ignition, gear, latched handbrake, and automatic simulation.
Pause cancels charge but retains entered sleep. Recovery calls `interrupt` and retains the scheduled cooldown endpoint.
Connect source sleep audio through the shared owner. Report radio station advancement as unsupported in this scope.
- [ ] **Step 4: Run focused tests and release build.** Check hold, cooldown, sleep, pause/focus loss, moving-vehicle sleep, recovery, death, and restart in the browser.
Capture the second working layer and clock traces under `.cache/survival-weather/layer-2/`.
Confirm physics and camera remain finite after a stall and an origin shift.
- [ ] **Step 5: Commit the sleep layer.** Message: `Connect sleep camera feedback and lifecycle`.

## Layer 3: Weather

### Task 7: Implement source storm scheduling, movement, and wind

**Files:** Create `src/weather.rs`, `src/starting_area/weather.rs`; modify `src/lib.rs`, `src/starting_area.rs`, `src/world.rs`.
- Test: `tests/weather.rs`, source-unit wind tests inside `src/weather.rs`, and `tests/weather_lifecycle.rs`.

**Interfaces:**
- `StormId(pub u32)` identifies a runtime storm. `Storm` stores absolute position, size, life, direction, speed, previous distance, and wind state.
- `StormInfluence { id: Option<StormId>, strength: f32 }`; no storm has `id=None`, `strength=0`.
- `WeatherState::new(reference: &WeatherReference, seed: u64) -> Self`.
- `advance(&mut self, reference: &WeatherReference, now: f64, dt: f32, player_absolute: [f64; 3], world_seed: u64)`.
- `spawn_at(&mut self, reference: &WeatherReference, position: [f64; 3]) -> StormId` follows the recovered source method.
- `storms(&self) -> &[Storm]`, `influence(&self) -> StormInfluence`, and `next_spawn_time(&self) -> f64`.
- `ShelterVolume { root: RigidBodyHandle, bounds: Bounds }` is the retained current-world AABB view of source shelter records.
- `scan_wind(&mut self, physics: &PhysicsWorld, roots: &[(RigidBodyHandle, RigidBodyHandle)], reference: &WeatherReference, now: f64)`.
- `apply_wind(&mut self, physics: &mut PhysicsWorld, shelters: &[ShelterVolume], dt: f32)`.
- `GarageRuntime` owns `weather: WeatherState`, shelter bindings, and retained body-root maps.
- Extract `terrain_height(seed: u64, absolute: [f64; 3]) -> f32` from the current terrain generator's existing height rule.

- [ ] **Step 1: Write `source_schedule_and_influence_boundaries`, `storm_life_uses_previous_distance`, and `wind_skips_shelter_and_removed_bodies`.**
For deterministic native seeds, assert interval `[1200, 5400]`, spawn ring `15000`, size `[0.5, 1.5]`, and speed `[3, 12]`.
At the exact scheduler deadline, spawn once and resample before creation.
With size/life one, assert strength `0` at distance `750`, `1` at `375`, and `0.5` at `562.5`.
Test strict strongest-storm selection, initial life zero, gain `0.05`, loss `0.1`, and destroy boundary `25000`.
Use internal wind fixtures with a unit-mass body, zero gravity, vector `(1,0,0)`, extent one, strength one, and delta `1/60`.
Expected horizontal impulse and velocity change are `1.3888889`, within floating-point tolerance.
Test extent clamp `30`, shelter exclusion, same-root exemption, and a body removed after the scan.
Rebase and recover without changing absolute storm position or schedule; restart reconstructs them.
Pin exact scheduler equality:

```rust
let mut weather = WeatherState::new(&r.weather, 42);
let due = weather.next_spawn_time();
weather.advance(&r.weather, due - 0.001, 0.001, [0.; 3], 42);
assert!(weather.storms().is_empty());
weather.advance(&r.weather, due, 0.001, [0.; 3], 42);
assert_eq!(weather.storms().len(), 1);
assert!(weather.next_spawn_time() > due);
```

- [ ] **Step 2: Run targets `weather` and `weather_lifecycle`.** Confirm missing weather behavior fails the tests.
- [ ] **Step 3: Implement the source scheduler and storm state.** Use a separate deterministic storm random stream with source ranges and draw order.
Use normal simulation time, current procedural desert height, and source biome zero.
Preserve previous-distance life evaluation, movement order, influence, destruction, and strongest selection.
Apply each storm's own strength to its wind targets. Strongest selection affects lighting only.
Preserve source storm order so equal strengths keep the first qualifying storm.
Scan source-mask colliders once per simulation second and retain grouped body extents.
Sample wind direction every source interval `[0.1, 1.5]`; preserve smoothing and the explicit force delta.
Apply the additional Rapier integration delta when converting the source force call to impulse.
Refresh root bindings on creation, removal, attachment, carry/drop, and origin change.
Update retained shelter bounds from current body poses before wind application, without waiting for the next scan.
Allocate scan capacity during lifecycle events.
Remove destroyed storm bindings and skip invalid body handles before applying force.
Do not add direct health, traction, vehicle-condition, or temperature effects.
- [ ] **Step 4: Run focused tests and existing world/vehicle tests.** Expect finite forces and no change to clear-weather driving.
Build the release WASM package with normal source scheduling intact.
Keep weather advancement disconnected from normal play until Task 8 provides its visual and audio signals.
- [ ] **Step 5: Commit the gameplay weather model.** Message: `Implement source sandstorm scheduling and wind`.

### Task 8: Compose source weather lighting and render selected particles

**Files:** Create `src/presentation/weather.rs`, `web/assets/shaders/source_particles.wgsl`.
- Modify: `src/lighting.rs`, `src/presentation/lighting.rs`, `src/presentation/sky.rs`, `src/presentation/mod.rs`, `src/app.rs`.
- Extend selected source particle/material export and shared audio storm bindings.
- Test: `tests/weather_lighting.rs`, `tests/particles.rs`, particle/export checks in `tools/reference/tests/test_weather.py`.

**Interfaces:**
- `CycleState::compose_weather(&mut self, reference: &WeatherReference, influence: StormInfluence)` modifies existing cycle outputs.
- Retain a clear-cycle output so repeated composition is idempotent; do not multiply an already-modified intensity.
- `ParticleState::new(reference: &ParticleReference, seed: u64) -> Self` and `advance(&mut self, dt: f32, emitter_absolute: [f64; 3], strength: f32, shelters: &[ShelterVolume])` live in `src/weather.rs`.
- `particles(&self) -> &[ParticleOutput]` provides active logical particles; `ParticleOutput` holds absolute pose, source size, color, and age.
- `SourceWeatherPlugin` owns retained render buffers and draw resources in `src/presentation/weather.rs`.
- `sync_weather` reads runtime particles and cycle output; it does not advance storm gameplay.
- Storm loop events and emitter poses use Task 4's shared audio owner and stable `StormId`-derived emitter identities.
- Select the source desert day/night ambient bindings for weather's ambient-volume reduction, using the same shared owner.

- [ ] **Step 1: Write `weather_composition_preserves_phase_rules` and `particle_capacity_and_origin_are_stable`.**
At the native fixture's noon, full storm influence gives intensity `0.72`, ambient `0.15`, and source storm fog density `0.001`.
At night it gives intensity `0.135` and ambient `0.015`; night-specific ground/cloud/atmosphere rules remain source-defined.
Test dawn/dusk boundary expressions and daytime atmosphere's cubic influence.
Compose twice from the same clear output and assert identical results.
At zero influence, assert the original clear-weather output.
Pin full daytime influence and repeated composition after sampling the fixture's noon:

```rust
let influence = StormInfluence { id: Some(StormId(1)), strength: 1. };
cycle.compose_weather(&r.weather, influence);
close(cycle.output().directional_intensity, 0.72);
close(cycle.output().ambient, 0.15);
close(cycle.output().fog_density, 0.001);
let first = cycle.output().clone();
cycle.compose_weather(&r.weather, influence);
assert_eq!(cycle.output(), &first);
```
Particle tests assert source capacity, lifetime/gradient endpoints, zero paused emission, interior trigger behavior, and stable absolute positions after rebase.
Restart and storm destruction must leave no active particle or stale audio emitter from the previous owner.
For base ambient gain `0.8`, source decrease `1`, and strength `0.4`, assert gain `0.4`; full strength clamps gain to zero.

- [ ] **Step 2: Run targets `weather_lighting` and `particles`, and the weather exporter tests.** Confirm missing composition/render data fails.
- [ ] **Step 3: Implement phase-aware composition and the selected particle subset.** Reuse current fog uniforms, sky materials, lights, clouds, and flare ownership.
Activate weather advancement and wind in the normal runtime at this layer's complete connection point.
Preserve the one-second source sky-tint sampling and each phase's separate formulas.
Build particle buffers from authored capacity at loading or creation; reuse buffers and mesh/material handles.
Implement active source lifetime, shape, velocity, size, gradient, material, and trigger settings for the selected storm and outlet effects.
Keep unsupported active modules in the explicit source limit ledger; add no invented replacement effect.
Use the existing Bevy render pattern and source texture samplers. Avoid duplicate glTF root loads and mismatched image samplers.
Connect source storm loops and ambient gain reduction through the shared audio owner.
- [ ] **Step 4: Run focused tests and release build.** Inspect clear weather and controlled storm entry/exit in the browser.
Check shader/console text, particle shelter, source sound, lamps, and camera fog at all cycle phases.
Capture the third working layer under `.cache/survival-weather/layer-3/`.
- [ ] **Step 5: Commit weather presentation.** Message: `Compose source storm lighting particles and feedback`.

### Task 9: Verify the assembled feature and document limits

**Files:** Test `tests/survival_weather.rs`; update affected critical web lifecycle tests and `scripts/check.ps1` if needed.
- Update the source audit and this plan's completed checkboxes.
- Keep browser fixture inputs, screenshots, console captures, clock traces, and logs under `.cache/survival-weather/`.

**Interfaces:** Uses the completed runtime, existing `run(reference_json)`, and layer interfaces above. Add no production diagnostic API.

- [ ] **Step 1: Write `assembled_pause_recovery_restart_and_origin` against the native runtime.**
Consume station food, begin sleep in a moving vehicle, enter a controlled storm, pause, resume, recover, rebase, and restart.
Assert single consumption, source clock factors, retained logical state during pause/recovery, stable storm coordinates, and source reset values.
Assert dead Continue/Recover cannot revive and stale prior-session feedback cannot affect the new session.
- [ ] **Step 2: Run target `survival_weather` and confirm any uncovered integration failure.** Use systematic debugging for failures.
- [ ] **Step 3: Fix only integration failures within the approved scope.** Remove obsolete call paths and any duplicate ownership introduced during integration.
Create browser acceptance fixtures by copying the manifest in memory and changing scheduler interval/ring values for controlled storm entry.
Use the existing `run(reference_json)` entry. Mark fixture captures as controlled tests; retain production manifest bytes and source schedule.
Test the real production startup and controls separately from the controlled fixture.
- [ ] **Step 4: Run final checks.** Expect full checks, exporter tests, current reference validation, and release packaging to pass.
Run only installed-export tests relevant to the changed source records and visual bindings.
Check the browser's food, drinking, toilet, sleep, storm, death, pause, focus loss, recovery, restart, and origin behavior.
Verify audible required feedback, finite physics, source item removal, no shader errors, and unchanged production manifest hash during fixtures.
Run `git diff --check`; inspect the final branch diff and source limit ledger.
- [ ] **Step 5: Commit the integration checks and evidence report.** Message: `Verify survival sleep and weather lifecycle`.
Report implementation, tests, browser evidence, source adapters, unsupported dependencies, and appearance limits.
Keep radio sleep advancement, ragdoll/postprocessing, hostile damage, wounds, multiplayer, and expanded saves outside this completion claim.
Use verification-before-completion before claiming success, then finishing-a-development-branch to choose integration with the user.

## Self-review and execution handoff

The tasks cover source audit/export, survival, consumption, toilet actions, damage, death, sleep, vehicle clocks, weather, feedback, and lifecycle.
Each working layer ends with browser verification before the next layer starts.
The Review Focus cases have named tests in Tasks 3, 4, 5, 6, and 7.
Interfaces use one shared effect type, one normal simulation clock, one cycle owner, and one audio owner.
No required function relies on an undefined future task or an old compatibility interface.

Read the approved specification and audit before execution.
Choose Native or Subagent-driven after reviewing this plan.
Native is recommended because these tasks share runtime, manifest, input, and lifecycle interfaces.
If Native is chosen, use executing-plans and one fresh whole-branch review on the most capable model at completion.
If Subagent-driven is chosen, use subagent-driven-development's task and reviewer gates.
Implementation begins only after the user reviews this plan and selects the execution method.
