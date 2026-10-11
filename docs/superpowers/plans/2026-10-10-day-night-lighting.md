# Day Cycle Headlights and Lamps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or superpowers:subagent-driven-development after the user's plan review and execution choice.
> Track steps with checkboxes. Read the specification before execution.

**Goal:** Reproduce the source day cycle, Lada lights, house switches, and automatic roadside lamps in WebGL2.

**Architecture:** Recover checked source records offline. Keep active time and lamp state in Rust, separate from physics catch-up. Bind shared Bevy lighting resources once and reuse source shaders for beam cookies and falloff.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, WebGL2, Rapier 0.36.0, wasm-bindgen 0.2.129, Python, UnityPy 1.25.4, pygltflib 1.16.5.

**Spec:** [Approved specification](../specs/2026-10-10-day-night-lighting-design.md).

## Global Constraints

- Recreate the day cycle, headlights, and existing lamps from installed Steam build `11140890`.
- Keep the fixed Lada, garage spawn, preparation controls, and procedural road.
- Work on the separate `codex/day-night-lighting` branch.
- Keep current engine heat and fluid rules. This change does not add temperature gameplay.
- Portable flashlights, survival, sleep, weather events, audio, and vehicle lamp removal remain outside this scope.
- Keep saves, multiplayer, other vehicles, and deployment outside the product scope.
- Keep one export command, manifest, schema, and loading path.
- Use the pinned dependencies. Do not add a runtime package for this feature.
- Keep recovered code, assets, and captures outside version control.
- Per-frame updates change existing state, transforms, material selections, and uniforms.
- Do not allocate collections, clone materials, resolve names, or rebuild shaders each frame.
- Follow `AGENTS.md`. Apply stop-slop to UI text and documentation. Keep instructions within 20 words.

## Review Focus

1. Rapid L presses during a fade must preserve the source request guard and consume each edge once. Task 4 tests this.
2. Missing cookies, failed shaders, or missing switch nodes must block Start with a source-specific error. Tasks 1 and 3 test this.
3. Dropped physics steps and resume delays must not change cycle duration or advance paused time. Tasks 2 and 3 test this.
4. Use and left click on one switch must toggle once and preserve seat, door, cap, and fill precedence. Task 5 tests this.
5. Late night loading, chunk seams, rebase, and restart must preserve lamp identities without duplicate instances. Tasks 6 and 7 test this.

## File map and shared interfaces

| Files | Responsibility |
| --- | --- |
| `tools/reference/lighting.py`, `tests/test_lighting.py` | Cycle, controller, sky, cookie, and road pole recovery |
| `tools/reference/gltf.py`, `manifest.py`, `export.py` | Pole asset, unused on/off materials, stable links, and hashes |
| `src/reference/lighting.rs`, `reference.rs`, `reference/rendering.rs` | Required lighting records and validation |
| `src/lighting.rs`, `lib.rs` | Native cycle, fades, vehicle signals, and controller states |
| `src/starting_area.rs`, `simulation.rs`, `app.rs` | Runtime ownership, active time, input, and lifecycle |
| `src/interaction.rs`, `interaction/feedback.rs` | Switch aim, action precedence, and feedback |
| `src/presentation/lighting.rs`, `mod.rs`, `reference.rs` | Shared light owner, cached bindings, and readiness |
| `src/presentation/materials.rs`, `world.rs`, `web/assets/shaders/source_lighting.wgsl` | Cookie, falloff, source light response, and procedural receivers |
| `src/presentation/sky.rs`, `web/assets/shaders/source_sky.wgsl` | Source sky, sun, clouds, stars, and flare presentation |
| `src/world.rs`, `world/lamps.rs` | Absolute pole positions and streamed ownership |
| `web/main.js`, `web/input.js`, `src/browser.rs`, `src/simulation/instruments.rs` | L input, switch prompt, and high-beam signal |
| `tests/lighting.rs`, `lighting_reference.rs`, `lighting_interaction.rs`, `world.rs`, `starting_area.rs` | Critical behavior and integration checks |
| `README.md`, `PRODUCT.md`, `DESIGN.md`, `docs/verification.md`, `tools/reference/README.md` | Controls, supported scope, and verified limits |

The material and UI interfaces are planned, but their implementations are absent at this branch's starting commit.
Inspect completed shared work before editing these owners. Integrate applicable reviewed commits before their dependent task.
Otherwise, create the named shared owner here for this feature. Do not create a parallel lighting or feedback owner.
Keep unrelated material and UI features in their own work.

Use `RenderingRecord.lighting: LightingRecord`, as specified by the material plan.
Move the light record definitions into `src/reference/lighting.rs`; import them from the single rendering contract.
Retain the material plan's `MaterialRecord`, `TextureRecord`, `RenderingState`, `SourceMaterials`, and source output transform.
Tasks below extend those contracts. They must not introduce a second manifest or renderer selection path.

Use the UI plan's `ActionAvailability`, `InteractionFeedback`, `InstrumentSample`, and `InstrumentState` interfaces.
Add `FeedbackContext::LightSwitch = 7` and `InstrumentSample.high_beam: bool`.
Add light-control selection to the same availability record that execution and feedback consume.
The selected Lada uses `headlight2`; do not add an unsupported generic headlight instrument.

## Verification commands

Run commands from this worktree. The existing Python environment can execute its exporter modules from this directory.

```powershell
$lightingPython = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/tools/reference/.venv/Scripts/python.exe'
& $lightingPython -m unittest discover -s tools/reference/tests -v
& $lightingPython -m tools.reference.export --steam-root 'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive'
./scripts/check.ps1 -Test lighting
./scripts/check.ps1 -Test lighting_reference
./scripts/check.ps1 -Test lighting_interaction
./scripts/check.ps1
./scripts/build.ps1
node scripts/serve.mjs
```

Use this installed-export check after generating assets:

```powershell
docker run --rm --mount "type=bind,source=$PWD,target=/workspace" --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' --mount 'type=volume,source=tld-wasm-target,target=/target' my-summer-slop-toolchain cargo test --locked -- --ignored
```

Expected: exporter tests, native tests, installed gates, formatting, and the release WASM build pass.
Use CUA for browser checks. Keep capture tools and diagnostic time controls under ignored `.cache`.
Do not add a production debug HUD, time control, or second loading path.

## Task 1: Export checked lighting records

**Files:** Create `tools/reference/lighting.py`, `tools/reference/tests/test_lighting.py`, `src/reference/lighting.rs`, and `tests/lighting_reference.rs`.
Modify `tools/reference/gltf.py`, `tools/reference/export.py`, `tools/reference/manifest.py`, `src/reference.rs`, `src/reference/rendering.rs`, `src/presentation/reference.rs`, and `tests/common/reference.json`.

**Interfaces:**
- Produce `recover_lighting(source: Source, roots: list[ObjectReader], output: Path) -> dict` in `lighting.py`.
- Produce `recover_cycle(source: Source, reader: ObjectReader, output: Path) -> dict`.
- Produce `recover_controllers(source: Source, roots: list[ObjectReader], node_ids: dict, material_ids: dict) -> list[dict]`.
- Produce `recover_road_lamp(source: Source, output: Path) -> dict`.
- Add transparent IDs `LightId(u32)` and `LightControlId(u32)` using the existing ID pattern.
- Add required `AssetRecord.usage: AssetUsage`, with values `UniqueScene` and `RoadLampPrototype`.
- Add `ReferenceVisuals.expected_instances: usize`; count only unique scene instances. Cache prototype assets without initial instantiation.
- `LightingRecord` retains the material plan's startup fields and adds required `cycle`, `controllers`, and `road_lamp` fields.
- `CycleReference` stores source identity, second-based durations, startup offset, source gradients, angles, intensity, ambient, fog, and sky resources.
- `LightControllerReference` stores ID, source, kind, aim collider IDs, animated node poses, startup state, electronics output, and state records.
- `LampStateReference` stores light IDs, source intensity, renderer node IDs, on/off material links, and working/broken flags.
- `RoadLampReference` stores asset ID, source identity, spacing, road-relative pose, pole rotation parameters, controller, and light IDs.
- Use asset ID plus glTF material index for material links. Reuse typed texture records for cookies and sky images.
- Produce `LightingRecord::validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError>`.

- [x] Add `test_cycle_uses_scene_values_and_nested_transition_durations` with 1200 day, 300 night, 240 dawn, and 180 dusk seconds.
  Assert the startup offset equals 240 and the total cycle equals 1500, not 1920 seconds.
- [x] Add `test_runtime_intensity_overrides_serialized_light_intensity`.
  Assert the selected high-beam state intensity is 1, although its serialized Light intensity is 1.25.
- [x] Add `test_selected_scene_lamps_have_checked_links`.
  Assert five house controls, six lamp entries, two headlight controllers, one cabin controller, and a rear-light controller.
- [x] Add `rejects_missing_cookie_and_invalid_controller_links` in `lighting_reference.rs`.
  Remove a required cookie, alter a node ID, duplicate a control ID, and inject a nonfinite angle. Assert rejection.
- [x] Add `prototype_loading_does_not_spawn_a_pole_at_the_origin`. Verify readiness counts only unique scene instances.
- [x] Run the new Python cases and `./scripts/check.ps1 -Test lighting_reference`. Expect failures before implementation.
- [x] Recover source `level1:64920`, selected scene Lada links, house switch links, and desert road pole links.
  Trace startup overrides, usable events, electronics output, on/off materials, sky shaders, stars, source fog mode, and color space.
- [x] Export on/off materials even when no mesh uses them at startup. Retain stable material indices and condition keys.
- [x] Export the pole scene separately. Keep its transforms reusable; do not bake its original prefab world placement.
- [x] Keep prototype nodes outside unique scene binding and body/item construction. Validate their links within their reusable asset.
- [x] Implement required records and validation. Check full custom-component byte consumption through `Source.read`.
- [x] Regenerate the fixture and installed export. Write the accepted manifest after resource and link validation succeeds.
- [x] Run the Python suite, lighting reference tests, and existing reference tests. Expect no regressions.
- [x] Build and load the current daylight game with the new export before replacing its renderer.
- [x] Commit as `Export source cycle and lamp controllers`.

## Task 2: Implement the active cycle clock

**Files:** Create `src/lighting.rs` and `tests/lighting.rs`. Modify `src/lib.rs`.

**Interfaces:**
- Consume `CycleReference` from Task 1.
- Produce `CyclePhase { Dawn, Day, Dusk, Night }`.
- Produce `CycleState::new(reference: &CycleReference) -> Self`.
- Produce `CycleState::advance(&mut self, reference: &CycleReference, elapsed_seconds: f64)`.
- Produce `CycleState::reset(&mut self, reference: &CycleReference)` and `CycleState::elapsed_seconds(&self) -> f64`.
- Produce `CycleState::output(&self) -> &CycleOutput`.
- `CycleOutput` contains phase, progress, directional pose/color/intensity, ambient, reflection intensity, sky exposure, fog, and roadside enable state.
- Retain numeric cloud, star, and flare outputs in `CycleOutput`; use fixed arrays for the recovered clear-weather layers.
- Evaluate source gradients with exported key times and modes. Keep source values separate from Bevy units.

- [x] Add `cycle_boundaries_match_source` with these assertions after source fixture evaluation:

```rust
assert_eq!(phase(240.0), CyclePhase::Dawn);
assert_eq!(phase(240.001), CyclePhase::Day);
assert_eq!(phase(1020.0), CyclePhase::Day);
assert_eq!(phase(1020.001), CyclePhase::Dusk);
assert_eq!(phase(1200.0), CyclePhase::Dusk);
assert_eq!(phase(1200.001), CyclePhase::Night);
```

  Define `phase(t)` as a test-local helper evaluating a zero-offset fixture at `t` seconds.
- [x] Add `source_cycle_formulas_match`.
  Assert dawn midpoint intensity 0.5625, ambient 0.11, and sky exposure 1 within `1e-6`.
  Assert noon intensity 1.6, night intensity 0.3, night exposure 2, and reflection intensity 0.
- [x] Add `roadside_thresholds_match_source`.
  Assert on before dawn progress 0.5, off at 0.5, off at dusk 0.5, and on above 0.5.
- [x] Add `cycle_wrap_retains_source_update_order` and `invalid_elapsed_does_not_corrupt_state`.
  Preserve the terminal night output on the source reset update. Verify the following update begins dawn.
  Assert negative, NaN, and infinite elapsed input leave state unchanged.
- [x] Run `./scripts/check.ps1 -Test lighting`. Expect new cases to fail.
- [x] Implement the active clock and source `FixedUpdate` phase formulas, including source interpolation clamps and exposure jumps.
  Retain one-second desert biome refresh and clear-weather visual temperature inputs. Do not change engine temperature.
- [x] Implement clear-weather cloud density and motion from the recovered shader and cycle rules.
  Verify Unity's native Perlin behavior through reference samples. Record any measured mismatch; do not substitute undocumented noise.
- [x] Run the lighting tests and native suite. Expect source numeric outputs to pass.
- [x] Commit as `Implement source day cycle state`.

## Task 3: Render the cycle and validate WebGL2 lighting

**Files:** Create or extend `src/presentation/lighting.rs`, `src/presentation/sky.rs`, and `web/assets/shaders/source_sky.wgsl`.
Modify `src/presentation/mod.rs`, `src/presentation/materials.rs`, `src/presentation/world.rs`, `src/presentation/reference.rs`, `web/assets/shaders/source_lighting.wgsl`, `src/app.rs`, and `src/starting_area.rs`.
Extend `tests/starting_area.rs` and `tests/lighting_reference.rs`.

**Interfaces:**
- Produce `SourceLightingPlugin: Plugin` and the single `SourceLighting` renderer resource.
- Retain `load_lighting(commands: &mut Commands, server: &AssetServer, lighting: &LightingRecord)` from the material plan.
- Produce `bind_lighting` and `sync_lighting` Bevy systems using cached imported node and material maps.
- Produce `lighting_readiness` setting `SourceLighting.state: RenderingState` after assets, bindings, and shaders succeed.
- Produce `SourceSkyPlugin: Plugin` and `sync_sky`, consuming `CycleOutput` and the active camera pose.
- Add `GarageRuntime.lighting: LightingState`; `LightingState` owns `cycle: CycleState` and preallocated controller state.
- Produce `LightingState::new(reference: &LightingRecord) -> Self` and `LightingState::reset(&mut self, reference: &LightingRecord)`.
- Produce `GarageRuntime::advance_lighting(&mut self, reference: &ReferenceManifest, elapsed_seconds: f64)` for cycle advancement only.
- Produce `LightOutput { id: LightId, enabled: bool, intensity: f32 }`.
- Produce `LampMaterialOutput { renderer: NodeId, asset: AssetId, material_index: u32 }`.
- Produce `ControlPoseOutput { node: NodeId, local_pose: crate::reference::Transform }`.
- Produce `LightingState::light_outputs(&self) -> &[LightOutput]` and `LightingState::material_outputs(&self) -> &[LampMaterialOutput]`.
- Produce `LightingState::control_poses(&self) -> &[ControlPoseOutput]`. Allocate these buffers once in `LightingState::new`.

- [x] Add `active_cycle_is_independent_of_physics_cap`.
  Advance an active runtime by 1 second. Assert cycle time advances 1 second while physics advances at most six steps.
- [x] Add `pause_focus_and_restart_preserve_clock_rules`.
  Assert pre-Start and paused calls do not advance time. Assert restart returns the cycle to the 240-second offset.
- [x] Add `required_sky_dependency_is_validated` by removing a required sky texture from the fixture.
- [x] Run starting-area and lighting reference tests. Expect new cases to fail.
- [x] Wire active elapsed time once per rendered frame, respecting `started`, `paused`, and `skip_elapsed`.
  Advance lighting before physics catch-up. Keep physics and source fade inputs separate.
- [x] Replace fixed sun, clear color, ambient, linear fog, and tonemapping assumptions with the shared source lighting output.
- [x] Bind source sky, cloud layers, stars, sun disk, and flare resources once. Update transforms and uniforms without rebuilding geometry.
- [x] Implement source cookie and falloff projection in the shared lighting shader using ordinary textures and WebGL2 uniform buffers.
  Bevy's cached `decal/clustered.rs` requires bindless textures; `render/mesh.rs` enables `LIGHT_TEXTURES` only when those bindings work.
  Do not enable `pbr_light_textures` as the browser solution. Keep one source shader path for covered lights.
- [x] Reuse Bevy shadow maps and light transforms. Replace covered direct-light contributions so the source contribution is not counted twice.
  Apply cookies to direct light, including diffuse and specular response. Keep emission separate from illumination.
- [x] Apply the shared path to procedural ground, road, station, and prop materials as well as imported receivers.
- [x] Document angle, intensity, fog, and output conversions. Validate shadow masks and WebGL2 binding limits before Start.
- [x] Run native tests and release WASM checks. Load the browser and capture dawn, noon, dusk, and night.
- [x] Break one required cookie path and one shader in temporary diagnostics. Confirm named loading errors and Start remaining disabled.
  Restore the files before proceeding.
- [ ] Verify a parked car and closed garage door cast shadows through all four phases. Record renderer differences.
  Task 7 owns the final night shadow check. Night edges remain unverified in the current dark capture.
- [x] Commit as `Render source day cycle and shared lighting`.

## Task 4: Implement Lada light state and controls

Source behavior and its fix pass review. Task 7 retains the remaining visual checks.
Numerical ranges and source cookie projection pass. Native GPU orientation and far cutoff remain unproved.
Source car switch pose arrays are empty. No animation is required.

**Files:** Modify `src/lighting.rs`, `src/starting_area.rs`, `src/simulation.rs`, `src/app.rs`, `src/browser.rs`, and the single browser input module.
Extend `src/presentation/lighting.rs`, `src/simulation/instruments.rs`, and `tests/lighting.rs`, `tests/input.rs`, `tests/instruments.rs`.

**Interfaces:**
- Add `keys::LIGHTS: u32 = 1 << 14`; map browser `KeyL` to this bit in the single input owner.
- Produce `HeadlightState::new(reference: &LightControllerReference) -> Self`.
- Produce `HeadlightState::request(&mut self, state: u8)` and `HeadlightState::update(&mut self, dt: f32)`.
- Produce `HeadlightState::current_state(&self) -> u8` and `HeadlightState::energy(&self) -> f32`.
- Produce `LightingState::cycle_headlights(&mut self, reference: &LightingRecord)`.
- Produce `VehicleLightSample { local_ignition: bool, brake: f32, gear_ratio: f32, electronics_output: f32 }`.
- Produce `LightingState::update_vehicle(&mut self, reference: &LightingRecord, sample: VehicleLightSample)`.
- Produce `LightingState::high_beam(&self) -> bool`; update the borrowed output buffers defined in Task 3.
- Produce `LightingState::advance_fades(&mut self, reference: &LightingRecord, dt: f32)` for controller and switch motion.
- Add `InstrumentSample.high_beam: bool`; map it to the selected `headlight2` lamp through the existing instrument update.

- [x] Add `headlight_fade_uses_source_branch_order`.
  From energy 1 in low state, request high. After `dt = 0.1`, assert energy 0 and current state still low.
  After the next update, assert state high and energy 0. After another `dt = 0.1`, assert energy 1.
- [x] Add `rapid_headlight_requests_keep_source_guard`.
  Request high during low fade, then request current low. Assert the current-state request does not cancel the pending high state.
- [x] Add `headlights_do_not_need_ignition_and_rear_lights_do`.
  Assert headlight intensity remains available with ignition off; tail, brake, and reverse outputs remain off.
- [x] Add `brake_reverse_and_high_beam_signals_match_source`.
  Assert brake 0.01 is off and 0.011 is on with ignition. Assert a negative gear ratio enables reverse.
  Assert requested high state enables `headlight2` when source electronics are available.
- [x] Extend catch-up input tests: one L press changes state once across six physics steps; hold and release do not repeat it.
- [x] Run lighting, input, and instrument targets. Confirm new cases fail.
- [x] Implement source state request, fade, intensity, electronics, material, and dashboard rules.
  Read signed selected ratio and actual brake input from the existing drivetrain decision, not absolute torque or vehicle motion.
- [x] Process L only in the driver seat. Call `advance_fades` once after control requests during each active rendered frame.
- [x] Bind low and high lights to their separate source nodes. Animate the source physical headlight control.
- [x] Run native and WASM checks. Verify 50-metre low and 250-metre high ranges, source cookie orientation, and lamp materials.
- [ ] Check walls, terrain, and garage doors occlude both beams. Check cockpit and chase views while steering.
- [x] Commit as `Implement source Lada lights and beam control`.

## Task 5: Implement aimed house and cabin switches

**Files:** Modify `src/interaction.rs`, `src/interaction/feedback.rs`, `src/starting_area.rs`, `src/reference.rs`, `src/browser.rs`, and `src/lighting.rs`.
Extend `src/presentation/lighting.rs`, UI sprite binding, `tests/lighting_interaction.rs`, and existing interaction tests.

**Interfaces:**
- Extend the single `AimTarget` with `node: NodeId`, `collider: ColliderId`, and `item: Option<ItemId>`.
- Keep hit distance, point, and wheel slot. Update existing item paths to consume the optional item explicitly.
- Resolve non-item switch colliders through `ColliderId -> LightControlId`, built once during setup.
- Add `InteractionAction::LightControl(LightControlId)`.
- Add `ActionAvailability.light_control: Option<LightControlId>` and `FeedbackContext::LightSwitch = 7`.
- Produce `LightingState::use_control(&mut self, reference: &LightingRecord, control: LightControlId)`.
- Use `InteractionState::available_actions` for both execution and feedback, retaining the source UI plan's signature.

- [x] Add `switch_aim_obeys_reach_and_occlusion`: a wall blocks use and prompts; a reachable unobstructed collider permits both.
- [x] Add `use_and_click_toggle_once`: simultaneous Use and allowed left-click press produce one controller action.
- [x] Add `cabin_switch_does_not_exit_seat`: aimed cabin Use toggles its controller while preserving seat occupancy.
- [x] Add `switch_input_preserves_latched_door_cap_and_fill_actions`.
  Assert the shared availability result follows source precedence, including the original left-click guard.
- [x] Add `switch_feedback_matches_execution`: both paths select the same control and exclude blocked targets.
- [x] Run lighting interaction, existing interaction, feedback, wheel, and player targets. Confirm new cases fail before implementation.
- [x] Replace item-only aim decoding with cached collider/node resolution. Preserve sensor, layer mask, held-body, and player exclusions.
- [x] Bind source usable events for physical car controls and parent switch ancestry for house controls.
- [x] Implement house on/off state, working/broken visibility, materials, and source button pose interpolation through `advance_fades`.
  Implement the cabin controller's two-state headlight fade. Keep L reserved for driving headlights.
- [x] Publish the source switch icon and valid action through the shared feedback bridge. Remove stale prompt text.
- [x] Run native and WASM checks. Use each of the five house controls and the cabin control in the browser.
- [x] Verify six house lamp entries start off, turn on through their controls, and preserve source shadows and materials.
- [x] Commit as `Implement source house and cabin light switches`.

## Task 6: Stream source roadside lamps

**Files:** Create `src/world/lamps.rs`. Modify `src/world.rs`, `src/presentation/lighting.rs`, `src/presentation/world.rs`, and `src/presentation/reference.rs`.
Extend `tests/world.rs` and `tests/lighting.rs`.

**Interfaces:**
- Produce `RoadLampId(pub i64)` from absolute source-spacing index.
- Produce `RoadLampPlacement { id: RoadLampId, absolute_point: [f64; 3], rotation: [f32; 4] }`.
- Convert absolute translation to local `f32` only through `WorldOrigin::local` when placing the instance.
- Produce `road_lamp_placement(seed: u64, id: RoadLampId, reference: &RoadLampReference) -> RoadLampPlacement`.
- Produce `road_lamp_range(start_m: f64, end_m: f64, spacing_m: f64) -> std::ops::Range<i64>` using half-open intervals.
- Clamp road segment IDs below zero. Preserve the original road start without poles behind its first segment.
- Extend `SourceLighting` with preallocated `RoadLampId -> instance` ownership and cached active keys `[ChunkKey; 11]`.
- Compare the cached keys and lateral band with current chunks before changing instances. Origin changes update existing local poses.
- Produce `sync_road_lamps` using source assets, current cycle output, `WorldStream`, and origin.

- [x] Add `road_lamp_intervals_have_one_owner_at_seams` using a synthetic spacing of 64 metres.
  Assert intervals `[0, 128)` and `[128, 256)` own IDs `[0, 1]` and `[2, 3]` without overlap.
  Assert `[-128, 0)` owns no IDs and `[-64, 64)` owns only ID 0.
- [x] Add `road_lamp_placement_is_absolute_after_rebase`.
  Assert two origin offsets change local translation but preserve absolute position, rotation, and lamp ID.
  Include an absolute position beyond 100 million metres; assert rebase round-trip error remains below `1e-6`.
- [x] Add `night_late_loading_uses_current_enable_state`.
  Assert a newly registered lamp inherits night on state, dawn threshold off state, and source electronics output above 0.2.
- [x] Run world and lighting tests. Expect new cases to fail.
- [x] Implement recovered spacing, road-relative transforms, horizontal light orientation, and seeded pole rotation.
  Use the existing `road_sample` path. Do not change the procedural road generator or invent pole spacing.
- [x] Spawn source pole instances only when their ownership interval enters the active window.
  Bind repeated scene nodes within their lamp instance. Do not insert repeated source IDs into the unique home/car node map.
- [x] Remove distant pole hierarchies and light registrations together. Respect source shadow flags and light masks.
- [x] Run world, lighting, and WASM checks. Drive across seams at night and through a floating origin shift.
- [x] Check warm spotlights, 20-metre range, 120-degree full cone, pole orientation, and bounded active light counts.
- [x] Commit as `Stream source roadside lamps with cycle control`.

## Task 7: Verify lifecycle and deliver the complete lighting change

**Files:** Extend `tests/starting_area.rs` and `tests/lighting_reference.rs`.
Modify `README.md`, `PRODUCT.md`, `DESIGN.md`, `docs/verification.md`, and `tools/reference/README.md`.
Preserve the recorded specification approval.

**Interfaces:**
- Reuse `LightingState::reset`, `GarageRuntime::advance_lighting`, `SourceLighting`, and the existing restart/recovery entry points.
- Produce `SourceLighting::reset_instances(&mut self, commands: &mut Commands)`; remove old light entities and retain loaded shared assets.
- Add no new save, debug, renderer-selection, or deployment interface.

- [x] Add `lighting_lifecycle_preserves_and_resets_owned_state`.
  Assert pause freezes time, fades, and switch movement; recovery preserves them; restart restores source startup state.
- [x] Add `restart_replaces_road_and_attached_light_instances` through the existing diagnostic browser harness.
  Assert old entities disappear and each active identity has one instance after restart.
- [x] Add ignored installed test `production_lighting_records_match_selected_scene`.
  Verify phase durations, source IDs, controller counts, material/cookie hashes, ranges, cones, and source shadow flags.
- [x] Wire `reset_instances` into restart before rebuilding bindings. Keep recovery on existing light instances.
- [x] Run the targeted lifecycle and installed tests. Fix integration faults before the complete suite.
- [x] Run Python tests, all native tests, installed-export tests, formatting, and the release WASM build.
- [x] Capture walking and cockpit views at dawn, noon, dusk, and night with matched camera and material conditions.
- [x] Check L, both physical car controls, every house switch, rear lamps, and roadside late loading.
- [x] Check missing-resource and shader failures leave Start disabled. Restore all temporary faults before delivery.
- [x] Compare source views where a live reference is available. Record timing evidence separately from image evidence.
- [x] Measure warm frame time, active light count, material count, and texture bytes during movement, rebase, recovery, and restart.
  Verify counts remain bounded and covered steady update paths do not allocate resources.
- [x] Document controls, source authority, rendering differences, and any unverified live comparisons.
- [x] Commit as `Verify source day cycle and lamp integration`.

## Plan review

Tasks 1 and 2 establish checked records and pure source behavior.
Task 3 delivers a working visual cycle before vehicle and switch interactions extend it.
Tasks 4 through 6 add the requested lamp families through shared state and rendering owners.
Task 7 verifies the combined lifecycle and records measured limits.

The five Review Focus cases have named tests in their owning tasks.
The plan covers the approved scope without adding excluded gameplay or duplicating shared interfaces.