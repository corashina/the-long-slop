# Source World Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the procedural generator with checked source road, terrain, placement, streaming, and revisit behavior.

**Architecture:** Advance source road state in order and retain sparse checkpoints with spatial bounds.
Keep generation records separate from physics, rendering, and compact object changes.
Add each layer to the working game before extending it.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, Rapier 0.36.0, Serde, Python 3.12, checked Unity exports, WebGL2.

**Spec:** [Approved source-world specification](../specs/2026-10-12-source-world-design.md).

Status: Written specification approved in chat on 2026-10-12. Plan review and execution method remain pending.

## Global Constraints

- Use installed build `11140890`, Unity `2021.3.18f1`, and its checked library records.
- Keep recovered assets, decompiled code, source captures, and analysis tools outside Git.
- Write original implementation code. Retain source identities and hashes in evidence.
- Preserve accepted lighting, materials, controls, garage geometry, and vehicle behavior.
- Keep storage, save UI, offscreen simulation, other vehicles, survival, audio, multiplayer, and deployment outside this work.
- Use the existing Rust, Rapier, Bevy, Serde, and Python export dependencies.
- Active geometry remains bounded. Record memory grows with changes.
- Origin changes must not alter a random stream, checkpoint, or placement decision.
- Do not use the replacement generator to produce its own expected values.
- Do not claim source runtime parity from synthetic fixtures or formula reconstruction alone.
- Keep one schema and generator interface. Remove obsolete paths after acceptance.
- Use short documentation sentences and the stop-slop skill. Add only critical tests.
- Compare native float primitive samples within four ULPs.
- Compare sampled bone rotations within 0.0001 degrees.
- Compare positions within the larger of 0.001 metres or eight source float ULPs for that coordinate.

## Review Focus

- A new region must acquire collision before the player or vehicle enters it. Task 6 tests publication order.
- Stale work must not publish after restart or an origin shift. Task 6 tests request identity and rebase.
- A moved child must keep its identity and attachment across distant regions. Task 8 tests ownership and restore order.
- A rejected or excluded spawn must consume the source draws without a substitute selection. Task 7 tests draw progression.
- Missing inputs or exceeded budgets must preserve the last playable region and report the source failure.
  Tasks 2 and 6 test failure.

---

## Workspace and verification commands

Work only in `C:/Users/Corashina/.codex/worktrees/source-world/the-long-drive-wasm` on `codex/source-world`.
The base is `dccd516387dff48bf9388ad96ecd52eab3f29c00`; the specification commit is `8137943`.
Do not switch the main checkout's branch or modify another chat's worktree.

Use these PowerShell variables from the isolated worktree:

```powershell
$referenceRoot = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm'
$referencePython = Join-Path $referenceRoot 'tools/reference/.venv/Scripts/python.exe'
$env:PYTHONDONTWRITEBYTECODE = '1'
```

Use the accepted library and source cache through read-only paths.
Create probe output, selected runtime exports, build output, and logs inside this worktree.
Do not rebuild or promote the main checkout's library.

Run baseline checks before Task 1:

```powershell
./scripts/check.ps1
```

Expected: native tests, browser-shell tests, formatting, and WASM compilation pass.
Keep baseline output in this plan's ignored workspace. Investigate failures before attributing them to this change.
The new worktree has no ignored exports or Python environment; use the paths above.

Use `./scripts/check.ps1 -Test NAME` for a native target named by a task.
Use `./scripts/build.ps1` for the production WASM package.
For installed tests, use the existing Docker image and volumes:

```powershell
. ./scripts/cargo-target.ps1
$target = Get-CargoTargetDirectory (Get-Location).Path
$cargoRun = @('run', '--rm', '--pull=never')
$cargoRun += @('--mount', "type=bind,source=$PWD,target=/workspace")
$cargoRun += @('--mount', 'type=volume,source=tld-wasm-cargo,target=/cargo')
$cargoRun += @('--mount', 'type=volume,source=tld-wasm-target,target=/target')
$cargoRun += @('--env', "CARGO_TARGET_DIR=$target", 'my-summer-slop-toolchain')
```

Installed command after Task 2:

```powershell
& docker @cargoRun cargo test --locked --test world_source -- --include-ignored
```

Expected: installed cases execute and pass; missing source samples fail the installed cases.
A skipped installed case does not establish acceptance.
After each task, run `git diff --check` and commit that task's listed files.

## Fixed replay, streaming, and memory limits

Use world seed 42 in normal play. Use signed seeds in source comparisons.
Use source default `defS.FDrawDistance = 5000` from `settingsscript`, records `level0:16865` and `level1:65886`.
Use the checked default input profile. The serialized `mainscript.DDS` distances are zero before startup.
Recover the default-setting caller and execute its interpolation in Task 1.
Keep the existing UI; add no world-quality control.

Compute distances with source float operations:
`drawLerp = InverseLerp(500, 25000, 5000)`; interpolate each checked `DDSmin` and `DDSmax` field.
Use source generation and removal distances, tile scaling, and integer rounding.
Do not substitute rounded decimal distances in the runtime.

| Retained state | Fixed limit |
| --- | --- |
| Road checkpoints | Every 256 segments, plus initial and terminal state; at most 197 |
| Building checkpoints | Every 32 selected buildings, plus initial and terminal state |
| Road replay | At most 256 segments after a complete checkpoint |
| Generation jobs | Two jobs; at most 32 pending region requests |
| Road instances | At most 512 distinct source segments across both focus windows |
| Terrain tiles | At most 512 near tiles and 256 tiles per distant tier across both windows |
| Registered light proxies | At most 32, including fixed lights; retain the existing 100 shadow slots |
| Generated object instances | At most 8192, including nested supported contents and road poles |
| Active geometry CPU bytes | 128 MiB, including collision input buffers and deformation output |
| Active geometry GPU bytes | 128 MiB of logical buffer storage |
| Shared world prototypes | 256 MiB retained CPU bytes and 512 MiB logical GPU bytes |
| Pending generation buffers | 32 MiB total across the two jobs |
| Checkpoints and spatial bounds | 16 MiB total; no complete road or building array |
| Change records | No fixed limit; report distinct identities and retained bytes |

These budgets apply to added world resources. Measure the existing imported scene as a separate baseline.
Count container capacities and shared resources once. Report driver overhead as unavailable when tools cannot measure it.
Process at most eight road segments or 4096 terrain vertices per job update.
Limit publication to one region per frame. Do no generation allocation during an idle frame.

The road contains 50,000 segments. Finish checkpoint preparation before Start, using bounded work buffers.
Compute building checkpoints through the source finite extent without retaining every building.
Build bounds while traversing each block. A spatial query must not require replay from the world start.
Store candidate counts and bounds for placement blocks alongside their checkpoints.

Use one window around the player and one around the current vehicle. Deduplicate overlap.
Select road blocks by absolute bounds and terrain by source tile coordinates.
Preserve source removal hysteresis. Never truncate a required region to meet a count limit.
On a limit failure, keep the accepted region, freeze attempted boundary crossing, and report the failed resource.
Task 9 must prove that the fixed profile meets these limits; change limits only through a recorded plan ruling.

Stream pole geometry within the source road window. Register light proxies through conservative visible receiver influence checks.
Include offscreen lamps whose source ranges reach visible surfaces, plus their required shadow caster regions.
Keep controller state independent of a proxy slot. Do not truncate the required light set to 32.
Report a capacity failure when required lights or shadows exceed the accepted renderer's limits.
Compare source lighting before accepting the fixed profile. Do not change lamp ranges, intensities, or visible contributions.

## Shared types and ownership

Define these types in the tasks below. Do not create compatibility aliases for old chunk or station types.

| Type | Fields or ownership |
| --- | --- |
| `WorldReference` | Seed input, settings, generators, source prototype links, native evidence hashes |
| `WorldPrototypeReference` | Checked source hierarchy, assets, skinning, bodies, collision, and supported action templates |
| `WorldLayer` | Road, terrain, structure, station, prop, lamp |
| `GeneratedId` | Layer, checked generator ID, unit `[i64; 2]`, source ordinal, child ordinal |
| `ObjectKey` | `Imported(ItemId)` or `Generated(GeneratedId)` |
| `AbsolutePose` | Position `[f64; 3]`, rotation `[f32; 4]` |
| `WorldBounds` | Absolute min and max `[f64; 3]` |
| `WorldSurface` | Absolute anchor, local positions, normals, colors, UVs, triangle indices, material identity |
| `RoadState` | Next segment, accumulated position and rotation, current distance, checked evaluator state |
| `RoadSegment` | Segment index, start state, bone rotations and coordinates, bounds |
| `TerrainContext` | Borrowed nearby road samples, building helpers, and garage hole inputs |
| `Placement` | Generated ID, source prototype ID, absolute pose, biome, source object seed |
| `WorldRequest` | Region identity and session generation number; origin remains a publication input |
| `WorldFrame` | Borrowed active surfaces, placements, and published generation number |
| `WorldMetrics` | Active and peak counts; retained capacities and logical CPU/GPU bytes |
| `ObjectSnapshot` | Absolute pose, velocities, owner relationship, supported condition, fluids, lamp state |
| `ObjectChange` | Removal or current snapshot, keyed by `ObjectKey` |

Bind generator IDs to their `SourceId` records once in `WorldReference`.
A record envelope includes the checked world-input hash; IDs never depend on load order.
Use ephemeral `ItemId` and collider handles only for active interaction bindings.
Keep their mapping to `ObjectKey` separate from retained change records.
The one current change record suppresses an object's baseline and indexes its destination.

### Task 1: Establish checked source evidence and independent samples

**Files:** Create `tools/reference/world_probe.py`, `scripts/sample-source-world.ps1`, and `tools/reference/tests/test_world_probe.py`.
Create `docs/research/2026-10-12-world-reference.md`.
Keep probe helpers, captures, instruction ranges, and expected outputs under `.cache/world-source/`.

**Interfaces:**
- Consume `verify_library(output: Path, source_root: Path) -> LibraryReceipt`, `LibraryReader`, and `native_engine_evidence`.
- Produce `world_probe.capture(reference_root: Path, output: Path, seeds: list[int]) -> dict`.
- Produce `world_probe.verify(output: Path) -> None` and `.cache/world-source/samples.json`.
- The sample envelope records hashes, sampler identity, caller inputs, random draws, and numeric outputs.

- [ ] Write `test_probe_rejects_changed_engine_and_missing_native_results` using changed-hash and missing-row output directories.
  Assertions: changed hashes and missing native rows fail validation.

  ```python
  with self.assertRaises(ValueError):
      world_probe.verify(changed_hash_path)
  with self.assertRaises(ValueError):
      world_probe.verify(missing_native_rows_path)
  ```

- [ ] Run `& $referencePython -m unittest tools.reference.tests.test_world_probe`.
  Expected: failure because the probe module or required validation is absent.
- [ ] Trace the spec's callers using checked managed IL, typed records, and active prefab links.
  Include seed transfer, default view-distance interpolation, garage height, holes, nested spawn order, and excluded content.
- [ ] Implement `capture` and `verify`; execute checked native primitive instructions through an isolated Windows probe.
  Confirm the native ABI and complete called instruction/data ranges before executing the probe.
  Hash `UnityPlayer.dll` against `native_engine_evidence`; keep source files and the Steam installation unchanged.
  Execute checked source managed methods with the native math boundary in the development probe.
  Keep source-derived probe method bodies outside Git. Record their managed method hashes and IL ranges.
  Stop this task if a required native primitive or managed caller cannot produce independent output.
  Do not substitute Python noise, the cloud slice, or replacement runtime calculations.
- [ ] Capture seeds `42, 0, -1705217151, -2147483648, 2147483647` through the script's `-ReferenceRoot` and `-OutputRoot` arguments.
  Command: `./scripts/sample-source-world.ps1 -ReferenceRoot $referenceRoot -OutputRoot .cache/world-source`.
  Cover road segments `0, 1, 99, 100, 255, 256, 257, 999, 1000, 9999, 49999`.
  Capture 10,000 placement candidates across the five seeds, RNG branch draws, and positive/negative two-dimensional noise coordinates.
  Capture source geometry seams, terrain border normals, biome thresholds, garage holes, and pole transforms.
  Classify each result as native primitive, executed managed source, reconstruction, or assembled source runtime.
- [ ] Repeat the probe and compare output hashes. Run probe unit tests.
  Expected: identical sample payloads, checked native hashes, complete required rows, and passing tests.
- [ ] Commit the four listed files as `docs: recover source world generation and sampling contract`.

### Task 2: Export and validate the world contract and prototypes

**Files:** Create `tools/reference/world.py`, `src/reference/world.rs`, `src/world/types.rs`, and `tests/world_source.rs`.
Create `tests/common/world.rs` and `tools/reference/tests/test_world.py`.
Modify `tools/reference/export.py`, `manifest.py`, `gltf.py`, `src/reference.rs`, and `src/world.rs`.
Update `tests/common/reference.json`, `tests/web/menu.test.mjs`, installed export validation, and `tools/reference/README.md`.

**Interfaces:**
- Consume Task 1 evidence and existing `LibraryReader` selected-record, mesh, texture, and checked-file methods.
- Produce `recover_world(source: LibrarySource, reader: LibraryReader, output: Path) -> dict` in `world.py`.
- Produce `WorldReference::validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError>`.
- Add required `ReferenceManifest.world: WorldReference` and `AssetUsage::WorldPrototype`.
- Define `WorldReference` and `WorldPrototypeReference` in `src/reference/world.rs`.
- Define `WorldLayer`, `GeneratedId`, `ObjectKey`, `AbsolutePose`, `WorldBounds`, `WorldSurface`, and `Placement` in `src/world/types.rs`.
- Re-export these runtime value types through `src/world.rs`; use Serde for stable identities and absolute poses.
- Produce `synthetic_world() -> WorldReference` and source-sample loading helpers in `tests/common/world.rs`.

- [ ] Write `rejects_missing_world_broken_weights_and_wrong_source_hashes` and `test_world_selection_requires_resolved_links`.
  Assertions: missing `world`, invalid skin weights, stale hashes, and unresolved source links produce errors.
  Verify loading keeps Start disabled after a required world dependency fails.

  ```rust
  assert!(manifest.world.validate(&manifest).is_err());
  assert!(ReferenceManifest::from_json(&world_less_json).is_err());
  ```

  Use the existing `MenuState` readiness fixture for the failure assertion:

  ```javascript
  const model = new MenuState();
  model.fail('Missing required source world dependency');
  model.ready();
  assert.equal(model.readyFlag, false);
  assert.equal(model.state, 'error');
  ```

  Verify the production loader reports a missing prototype dependency through this same failure path.
- [ ] Run `./scripts/check.ps1 -Test world_source` and `& $referencePython -m unittest tools.reference.tests.test_world`.
  Expected: failures because the world contract and required validation are absent.
- [ ] Implement `recover_world` and `WorldReference`; export selected road variants and supported structure/prop prototypes.
  Retain bone links, bind poses, weights, collision ownership, local materials, terrain inputs, spawn rules, and source identities.
  Validate one required source record before reading its dependent files; include identity and field in errors.
- [ ] Extend the existing export CLI with `--library-root`; verify its receipt and read its source cache without writes.
  Stage runtime output under this worktree's `.cache/`, then promote only this worktree's selected runtime directory.
  Replace the fixed five-asset whitelist with the checked selected-scene and world-prototype set.
  Preserve unique scene counts and per-instance prototype node ownership. Reject obsolete world-less manifests.
- [ ] Regenerate the synthetic fixture and selected installed export.
  Command: `& $referencePython -m tools.reference.export --library-root "$referenceRoot/web/assets/library"`.
  Command: `& $referencePython -m tools.reference.export --library-root "$referenceRoot/web/assets/library" --verify`.
  Run Python `test_world`, native `reference`, `validate_reference`, and `world_source` targets.
  Run `node --test tests/web/menu.test.mjs` for readiness failure handling.
  Expected: source fields and dependencies validate; old manifests and corrupted bindings fail their rejection cases.
- [ ] Commit the listed files as `feat: export checked source world inputs and prototypes`.

### Task 3: Implement source random and terrain numeric primitives

**Files:** Create `src/world/random.rs` and `src/world/noise.rs`; modify `src/world/lamps.rs` and `src/world.rs`.
Modify `tests/world_source.rs` and `tests/lighting.rs` only where shared random exports require changes.

**Interfaces:**
- Consume `WorldReference` and Task 1 source samples.
- Produce `SourceRandom::new(seed: i32) -> Self` and `next_range(&mut self, min: i32, max: i32) -> i32`.
- Produce `sample(&mut self) -> f64`, `snapshot(&self) -> RandomState`, and `restore(state: RandomState) -> Self`.
- `RandomState` contains the checked subtractive state array and the two source indices.
- Produce `source_perlin(x: f32, y: f32) -> f32` and `TerrainEvaluator::new(reference: &WorldReference, seed: i32) -> Self`.
- Produce `TerrainEvaluator::base_height(&self, xz: [f32; 2], edge: bool) -> f32`.
- Produce `TerrainEvaluator::color(&self, xz: [f32; 2], height: f32) -> [f32; 4]`.

- [ ] Write `source_random_restores_small_large_and_branch_draws` and installed `native_noise_and_base_height_match_source`.
  Assertions: source draws and restored states are exact; native primitive errors are at most four ULPs.
  Cover equal bounds, signed overflow seed sums, negative coordinates, octave order, and world-edge fade.

  ```rust
  assert_eq!(observed_draws, expected_draws);
  assert_eq!(restored.snapshot(), random.snapshot());
  assert!(native_ulp_error <= 4);
  ```

- [ ] Run `world_source`, including installed cases through the Docker command above.
  Expected: source comparisons fail because general noise and large-range random draws are absent.
- [ ] Move the checked subtractive RNG into `random.rs`; implement source small/large range rules and state snapshots.
  Keep source cast and draw order. Route existing lamp rotation through this shared implementation.
- [ ] Implement two-dimensional native noise from Task 1 evidence, preserving source float operations and lookup data.
  Cache seed offsets and octave inputs during evaluator construction. Keep source evaluation order.
  Implement source base height and color channels without per-sample allocations.
  Retain the existing cloud slice and its independent bit checks; it does not prove the new noise domain.
- [ ] Run `world_source` installed comparisons and `lighting` tests.
  Expected: exact RNG state and draws, numeric tolerances within the spec, and unchanged accepted lamp rotations.
- [ ] Commit the product modules and affected tests as `feat: implement checked source world numeric primitives`.

### Task 4: Implement sequential road replay and spatial checkpoints

**Files:** Create `src/world/road.rs`; modify `src/world.rs`, `tests/world_source.rs`, and `tests/common/world.rs`.

**Interfaces:**
- Consume `WorldReference`, `SourceRandom`, and `TerrainEvaluator`.
- Define `RoadState`, `RoadSegment`, and `RoadCheckpoint` in `src/world/road.rs`.
- Produce `RoadSequence::new(reference: &WorldReference, seed: i32) -> Result<Self, ReferenceError>`.
- Produce `advance(&mut self, terrain: &TerrainEvaluator) -> Option<RoadSegment>` and `checkpoint(&self) -> RoadCheckpoint`.
- Produce `restore(&mut self, checkpoint: &RoadCheckpoint)`.
- Produce `RoadIndex::new(sequence: RoadSequence, terrain: TerrainEvaluator) -> Self` and `prepare(&mut self, max_segments: usize) -> bool`.
- Produce `segments_in(&self, bounds: WorldBounds, output: &mut Vec<u32>)` and `segment(&mut self, index: u32) -> RoadSegment`.
- `RoadIndex` owns sparse states, block bounds, and reusable replay buffers; never all 50,000 segment records.

- [ ] Write `road_checkpoint_replay_matches_source_and_sequential_generation` and `road_end_and_first_segment_follow_source`.
  Assertions: sequential and restored outputs match bit for bit; source rotations and positions meet the spec tolerances.
  Assert 197 maximum checkpoints, 256 maximum replay segments, and no segment after index 49,999.
  Query curved-road bounds where negative Z alone selects the wrong segment.

  ```rust
  assert_eq!(replayed_segment, sequential_segment);
  assert!(position_error <= position_limit);
  assert!(checkpoint_count <= 197);
  assert_eq!(sequence.advance(&terrain), None);
  ```

- [ ] Run native and installed `world_source` cases.
  Expected: failure because source progression and checkpoints are absent.
- [ ] Implement source startup, recursive offset initialization, terrain lookahead, angle clamping, and float step accumulation.
  Preserve `CalcOneRoad` order, `startStraight`, and the carried double position and source rotation.
  Initialize road height from the checked garage/start relationship, not an arbitrary translation.
- [ ] Prepare sparse checkpoints and conservative block bounds through the source extent using bounded work buffers.
  Implement segment replay and spatial queries. Return records without per-bone heap allocations.
  Freeze checked inputs after preparation; generation and origin shifts cannot mutate them.
- [ ] Run the task's cases, including startup and distant source indices.
  Expected: matching samples, exact replay, source terminal behavior, and retained state below 16 MiB.
- [ ] Commit as `feat: generate source road with bounded checkpoint replay`.

### Task 5: Generate source building layout and region checkpoints

**Files:** Create `src/world/placement.rs`; modify `src/world.rs`, `tests/world_source.rs`, and `tests/common/world.rs`.

**Interfaces:**
- Consume `WorldReference`, `SourceRandom`, and `TerrainEvaluator` with the checked road curve inputs.
- Produce `BuildingSequence::new(reference: &WorldReference, seed: i32) -> Self` and `next(&mut self) -> Option<Placement>`.
- Produce `checkpoint(&self) -> BuildingCheckpoint` and `restore(&mut self, checkpoint: &BuildingCheckpoint)`.
- Define `BuildingCheckpoint` with source RNG, group pools/counts, previous type, and next distance.
- Produce `PlacementIndex::new(reference: &WorldReference, seed: i32) -> Self` and `prepare(&mut self, max_buildings: usize) -> bool`.
- Produce `buildings_in(&mut self, bounds: WorldBounds, output: &mut Vec<Placement>)`.
- Produce `nearest_supply(&mut self, point: [f64; 3]) -> Option<Placement>` from source station and supported supply records.
- Index source foundation helpers and station recovery points beside placement blocks; do not retain a complete building array.

- [ ] Write `building_checkpoint_replay_and_station_selection_match_source`.
  Assert source types, draw counts, repeat-pool state, spacing, sides, and checkpoint restoration match.
  Assert nearest supply uses source coordinates and a supported source supply, including both sides of a curved road.

  ```rust
  assert_eq!(replayed_ids, sequential_ids);
  assert_eq!(observed_draws, expected_draws);
  assert_eq!(selected_supply.id, expected_supply.id);
  ```

- [ ] Run native and installed `world_source` cases.
  Expected: building layout, region replay, or station comparisons fail because this sequence is absent.
- [ ] Implement sequential building layout through the finite source extent; checkpoint every 32 selected buildings.
  Preserve group rotation, duplicate-type extra draws, source float spacing quantization, and side orientation.
  Include source special-stop behavior only where active callers use it. Do not activate unused serialized rules.
- [ ] Build placement block bounds, foundation helper queries, and station recovery selection from checked prototypes.
  Use source curve samples and terrain heights with the source origin reset/restore sequence.
  Keep station identities independent of the current region, camera, and generated item handles.
- [ ] Run the task's source comparisons.
  Expected: exact selection and replay, correct source placement transforms, and checkpoint storage within the shared 16 MiB budget.
- [ ] Commit as `feat: generate source building and station layout with checkpoints`.

### Task 6: Connect source road, terrain, stations, and garage streaming

**Files:** Modify `src/world.rs`, `src/world/terrain.rs`, `src/presentation/world.rs`, `src/starting_area.rs`, and `src/app.rs`.
Modify `src/browser.rs`, `web/main.js`, and `tests/hud.mjs` for changed world metrics and removed chunk-band fields.
Modify `src/presentation/world/lamps.rs`, `src/presentation/lighting.rs`, and `src/presentation/materials.rs` for source world rendering.
Modify `src/interaction.rs`, `src/interaction/items.rs`, and required collider lifecycle methods in `src/vehicle.rs`.
Update `tests/world.rs`, `tests/starting_area.rs`, `tests/world_source.rs`, and `tests/lighting_interaction.rs`.

**Interfaces:**
- Consume `RoadIndex`, `PlacementIndex`, `TerrainEvaluator`, `WorldReference`, and current garage collision records.
- Define `TerrainContext`, `WorldRequest`, `WorldFrame`, and `WorldMetrics` in their owning world modules.
- Produce `deform_road(reference: &WorldReference, segment: &RoadSegment, output: &mut WorldSurface)`.
- Produce `terrain_surface(reference: &WorldReference, unit: [i64; 2], context: TerrainContext, output: &mut WorldSurface)`.
- Produce `biome(color: [f32; 4], override_index: Option<u8>) -> u8`.
- Replace `WorldStream::new` with `new(reference: &ReferenceManifest, seed: i32) -> Result<Self, ReferenceError>`.
- Produce `request_at(&mut self, player: [f64; 3], vehicle: [f64; 3])` and `prepare(&mut self)`.
- Produce `publish(&mut self, physics: &mut PhysicsWorld) -> Result<(), ReferenceError>` and `frame(&self) -> WorldFrame<'_>`.
- Produce `supports(&self, swept: WorldBounds) -> bool` for movement into a published collision region.
- Produce `recovery_target(&mut self, point: [f64; 3], prefer_supply: bool) -> Result<AbsolutePose, ReferenceError>`.
- Produce `ItemStore::insert_generated(key: GeneratedId, prototype: &WorldPrototypeReference, pose: AbsolutePose) -> Result<ItemId, ReferenceError>`.
- Produce `remove_generated(id: ItemId)` and `InteractionState::unbind_generated(id: ItemId, colliders: &[ColliderHandle])`.
- Produce `required_world_lights(frame: &WorldFrame<'_>, receiver_bounds: &[WorldBounds], output: &mut Vec<GeneratedId>)`.
- Retain `WorldOrigin` and the between-step rebase contract; remove chunk-distance assumptions from callers.

- [ ] Write `garage_exit_and_source_collision_seams_remain_continuous` and `stream_publication_preserves_support_and_rejects_stale_work`.
  Assert matching visual/collision positions and indices, shared seams, preserved garage holes, and four-wheel support.
  Restart during generation, then rebase before publication; assert no stale session output and correct local collision coordinates.
  Exceed a budget; assert the previous region remains valid and unsupported movement stops.
  Write `source_recovery_preserves_cargo_and_uses_real_supplies` for empty fuel, curved roads, and destination publication order.
  Write `terrain_biome_borders_and_visible_light_admission_match_source` for normals, biome thresholds, and offscreen light influence.

  ```rust
  assert_eq!(surface.positions, collider_vertices);
  assert!(!stream.supports(unpublished_bounds));
  assert_eq!(biome([0.5, 0.0, 0.25, 0.0], None), 2);
  assert_eq!(biome([0.5, 0.0, 0.2501, 0.0], None), 1);
  assert_eq!(restored_cargo_state, previous_cargo_state);
  ```

- [ ] Run `world`, `starting_area`, `lighting_interaction`, and installed `world_source` cases.
  Expected: source geometry, station recovery, terrain, or publication assertions fail.
- [ ] Implement source skin deformation and source near terrain using shared rendering/collision geometry.
  Preserve source bone order, weights, bind poses, normals, UVs, biome road variants, and single coordinate conversion.
  Implement source road/building alignment precedence, garage floor and pit holes, and border normals.
  Generate terrain colors and both distant tiers with source scaling, integer resolution clamps, and placement offsets.
  Bind generated surfaces through the existing source material cache.
  Preserve checked terrain material parameters, vertex channel rules, authored samplers, and prototype material identities.
  Supply borrowed terrain colors to the existing biome lighting refresh.
- [ ] Publish source stations and supported supplies from Task 5's layout through checked prototype recipes.
  Initialize source rates and contents once. Remove per-sync synthetic pump replenishment.
  Schedule recovery to a source road or supply point; publish its ground and supply before transporting the vehicle and cargo.
  Preserve current recovery clearance, held-object handling, cargo transport, and supported state.
- [ ] Connect two bounded generation jobs and session request identities to runtime movement and renderer publication.
  Keep the last playable region on generation failure. Rebase absolute output against the current publication origin.
  Gate unsupported movement through the existing runtime failure path; retain supported poses and report the failed region.
  Replace road/terrain consumers through `WorldFrame`; bind source prototypes before showing instances.
  Separate streamed pole geometry from required light proxies using conservative receiver influence bounds.
  Preserve current lamp controller output. Task 7 extends biome lamp recipes and all supported structures.
- [ ] Verify normal-input garage driving and a controlled continuous-drive fixture.
  Run `world`, `starting_area`, `vehicle`, `lighting_interaction`, installed `world_source`, and `./scripts/build.ps1`.
  Expected: continuous source road, source terrain and stations, correct recovery, accepted lighting, and bounded resource use.
- [ ] Commit as `feat: connect source world terrain stations and garage streaming`.

### Task 7: Add structures, stations, props, and source road lamps

**Files:** Modify `src/world/placement.rs`, `src/world/lamps.rs`, and `src/presentation/world.rs`.
Modify `src/presentation/world/lamps.rs`, `src/presentation/lighting.rs`, `src/interaction.rs`, and `src/interaction/items.rs`.
Modify `tests/world_source.rs`, `tests/world.rs`, and `tests/lighting_interaction.rs`.

**Interfaces:**
- Consume `PlacementIndex`, `WorldReference` prototype recipes, Task 3 numeric primitives, and existing lighting controllers.
- Define `SourceSpawnState` with the traced source random streams and caller progression.
- Produce `generate_props(reference: &WorldReference, seed: i32, unit: [i64; 2], output: &mut Vec<Placement>)`.
- Produce `spawn_contents(reference: &WorldReference, parent: &Placement, state: &mut SourceSpawnState, output: &mut Vec<Placement>)`.
- Produce `place_road_lamp(reference: &WorldReference, segment: &RoadSegment) -> Placement`.
- Consume Task 6 generated-item insertion/removal, interaction unbinding, and light admission interfaces.
- Active bindings map ephemeral item IDs to stable generated IDs; unload removes those bindings.

- [ ] Write `source_placement_weights_rejections_and_exclusions_preserve_draws` and `world_prototypes_bind_per_instance_and_unload`.
  Assert exact identities and rotations for the 10,000 source candidates and exact draws after empty or excluded selections.
  Assert road/building distance exclusions, biome masks, distinct instance ownership, source pump rate/state, and complete binding removal.
  Assert visible receivers retain required lamp contributions while distant pole geometry remains streamed without unused light proxies.

  ```rust
  assert_eq!(observed_ids, expected_ids);
  assert_eq!(observed_draws, expected_draws);
  assert!(items.get(unloaded_item).is_none());
  assert_eq!(required_light_ids, expected_light_ids);
  ```

- [ ] Run `world_source`, `world`, and `lighting_interaction`.
  Expected: selection, source structure geometry, station state, or instance teardown cases fail.
- [ ] Instantiate source structures and stations using checked local geometry, collision, and existing supported interaction rules.
  Preserve source selection, empty results, rejection order, terrain helpers, and nested spawn state.
  Consume source draws for excluded product content without instantiating or rerolling it.
- [ ] Add near props, mountains, towers, and desert-building generators as separate placement layers.
  Cache source tables and cumulative weights once. Use source coordinate seeds and large-range draws.
  Replace pole placement from the old road sample with checked segment bones and prefab local transforms.
  Recover supported structure light recipes; reuse current light/material caches and controller state.
  Release per-instance bindings and light slots on unload. Keep light state independent of proxy registration.
- [ ] Remove per-sync synthetic pump replenishment; initialize source contents once per deterministic baseline.
  Run native and installed source comparisons, lighting checks, and the production build.
  Expected: exact accepted candidate identities, source meshes, active light continuity, and bounded instance counts.
- [ ] Commit as `feat: stream source structures stations props and lamps`.

### Task 8: Preserve changed objects through unloading and revisits

**Files:** Create `src/world/changes.rs`; modify `src/world.rs`, `src/starting_area.rs`, and supported interaction mutation paths.
Modify `src/interaction/items.rs`, `src/interaction/carry.rs`, `src/interaction/parts.rs`, and `src/presentation/world.rs`.
Modify `tests/world.rs`, `tests/items.rs`, and `tests/starting_area.rs`.

**Interfaces:**
- Consume `ObjectKey`, active generated-item mappings, existing item/fluid/light state, and deterministic baseline recipes.
- Define Serde `ObjectSnapshot`, `ObjectChange`, and `ChangeRecords` without runtime handles.
- Produce `ChangeRecords::record(key: ObjectKey, current: ObjectSnapshot, baseline: &ObjectSnapshot)`.
- Produce `remove(key: ObjectKey)`, `get(key: ObjectKey) -> Option<&ObjectChange>`, and `iter()` over borrowed records.
- Produce `changes_in(&self, bounds: WorldBounds, output: &mut Vec<ObjectKey>)` and `retained_bytes(&self) -> usize`.
- `WorldStream` owns session changes and its region index. Restart clears them; recovery preserves them.

- [ ] Write `changed_objects_survive_revisit_without_duplicates` and `attachments_restore_before_physics_after_rebase`.
  Assert moved-object destination ownership, one record per identity, removal suppression, and unchanged-baseline regeneration.
  Assert condition, fluid quantities, switch state, captured velocities, and imported/generated attachment ownership survive unload.
  Assert repeated edits keep one record; complete baseline restoration removes it; restart clears records.
  Round-trip a record envelope with Serde; assert stable IDs and numeric state survive without runtime handles.

  ```rust
  assert_eq!(records.get(key), Some(&expected_change));
  assert_eq!(records.iter().count(), 1);
  assert_eq!(restored_snapshot, captured_snapshot);
  ```

- [ ] Run `world`, `items`, and `starting_area`.
  Expected: state resets, duplicate baseline objects, or missing destination objects expose absent change retention.
- [ ] Implement event-driven dirty marking at supported mutation sites and absolute snapshot capture before unloading.
  Use one record for baseline suppression and destination indexing; never key it by ephemeral handles.
  Keep imported identities distinct and preserve existing fixed garage/Lada ownership.
  Keep carried and attached objects active with their owner; freeze unloaded dynamic objects at captured state.
  Store owner key, stable slot identity, and local attachment pose alongside the supported attachment state.
- [ ] Apply records before instance publication; restore parents before children and bind interaction after physics ownership is complete.
  Restore captured state and velocities. Avoid simulating unloaded time.
  Expose borrowed session records and measure container capacity; add no disk or browser storage.
- [ ] Run the task's tests and a distant browser revisit with moved, drained, removed, attached, and switched objects.
  Verify recovery, restart, and repeated origin shifts through the same record path.
  Expected: correct state, no duplicates, bounded active instances, and record growth tied to distinct changed identities.
- [ ] Commit as `feat: retain world object changes across streamed revisits`.

### Task 9: Prove long-route acceptance and remove obsolete generation

**Files:** Create `src/world/metrics.rs`, `scripts/world-acceptance.ps1`, and `tests/world_acceptance.rs`.
Modify `src/world.rs`, `src/app.rs`, `src/browser.rs`, `web/main.js`, and `tests/hud.mjs`.
Modify `tests/world.rs` and `tests/world_source.rs`.
Update `PRODUCT.md`, `DESIGN.md`, `README.md`, `docs/verification.md`, and `tools/reference/README.md`.

**Interfaces:**
- Consume `WorldFrame`, active physics/render counts, `ChangeRecords`, source samples, and fixed budgets above.
- Move `WorldMetrics` from its initial Task 6 owner into `metrics.rs`; re-export it without an obsolete alias.
- Produce `WorldStream::metrics(&self) -> WorldMetrics` and development measurement snapshots without technical player-facing UI.
- Produce `world-acceptance.ps1 -ReferenceRoot <path> -OutputRoot <path>` using the existing exporter and Docker toolchain.
- Acceptance output separates native traversal, continuous physics driving, source captures, browser results, and memory classes.

- [ ] Write `source_extent_and_repeated_routes_keep_world_resources_bounded` and `restart_releases_pending_and_instance_resources`.
  Assert 100-kilometre forward/reverse traversal, finite 50,000-segment traversal, checkpoint bounds, and fixed resource budgets.
  Assert unchanged-route repeats retain no new identities; repeated edits merge records; restart releases changed/active/pending state.
  Track positive/negative rebases, off-road walking, separated player/vehicle windows, and parked support.

  ```rust
  assert!(metrics.active_cpu_bytes <= 128 * 1024 * 1024);
  assert!(metrics.pending_bytes <= 32 * 1024 * 1024);
  assert_eq!(second_run.active_counts, first_run.active_counts);
  assert_eq!(records_after_repeat, records_after_first_route);
  ```

  Repeat the same measurement positions after warmup. Separate retained capacities from process high-water memory.
  Count actual physics changes as edits; do not label settling or vehicle collisions as an unchanged baseline.
- [ ] Run `./scripts/check.ps1 -Test world_acceptance`.
  Expected: missing metrics or retained obsolete resources fail the acceptance assertions.
- [ ] Implement capacity-based CPU metrics and unique-handle logical GPU accounting.
  Record imported assets separately, process/WASM memory, checkpoint bytes, replay time, peak frame time, and initial Start readiness time.
  Run the full source extent through automated region traversal; never label teleports as continuous driving.
- [ ] Run `./scripts/world-acceptance.ps1 -ReferenceRoot $referenceRoot -OutputRoot .cache/world-acceptance`.
  Run sampling verification, installed export verification, native acceptance, `./scripts/check.ps1`, and `./scripts/build.ps1`.
  Execute the 100-kilometre forward/reverse route, full extent traversal, repeated edits, and restart cycles.
  Run browser loading, source geometry, garage traversal, seam-driving, biome, lamp, rebase, and revisit checks.
  Use controlled input where pointer lock prevents ordinary input; label that limitation in evidence.
  Compare matched installed-source views and collision samples before claiming assembled source behavior parity.
  Expected: required checks execute, numeric tolerances hold, unchanged resource use plateaus, and no budget exceeds its limit.
- [ ] After acceptance passes, delete sine-road generation, fixed stations, simple props, old chunk bands, and obsolete lamp sampling.
  Remove stale negative-Z routing, synthetic station identities, unused generator helpers, and obsolete tests.
  Preserve startup randomness used outside world generation; move its helper to the owning startup module if needed.
  Update product/documentation limits to the finite source extent and retained session changes.
  Keep one world loader and one runtime generation path.
- [ ] Repeat affected checks after deletion. Search callers for obsolete generator symbols and verify memory plateau again.
  Command: `rg -n 'generate_chunk|generate_chunk_in_band|nearest_station|STATION_SPACING|LATERAL_BAND_WIDTH|lateral_band|road_sample' src tests`.
  Expected: no obsolete product generator references; source-test terminology must have an explicit current purpose.
  Commit as `feat: verify source world acceptance and remove obsolete generator`.

## Plan self-review

| Specification requirement | Owning tasks |
| --- | --- |
| Source callers, seed progression, and independent samples | 1, 2, 3 |
| Road progression, geometry, joins, and checkpoints | 4, 6 |
| Terrain, biomes, garage continuity, and station recovery | 5, 6 |
| Structure, prop, lamp selection and placement | 7 |
| Bounded streaming, parked support, and floating origin | 6, 8, 9 |
| Changed-object identity, ownership, and future record access | 8 |
| Long routes, collision seams, retained memory, and obsolete generator removal | 9 |

Keep source evidence capture as the first acceptance gate. Do not manufacture expected values to continue past it.
Review interfaces after each task; later tasks must consume the signatures above or a recorded ruling.

## Completion and handoff

Each task ends with its required checks, a commit, and a ledger entry.
Keep the accepted product checkpoint available through Git, without a runtime fallback path.
Do not mark Task 9 complete while installed comparisons, collision traversal, or memory measurements remain unavailable.

Review the full branch against the approved spec, source evidence, Review Focus, and ledger rulings.
Report native-source limits, browser limits, measured budgets, and change-record growth with their evidence paths.
Use the finishing-a-development-branch skill after implementation and required verification.
Do not merge, push to a shared branch, or delete another chat's worktree as part of this plan.

The user must review this plan and select Native or Subagent-driven execution before implementation.
Native execution uses one implementer in this chat and one fresh reviewer for the completed branch.
Subagent-driven execution uses implementer and reviewer agents for each task, then a whole-branch review.
Recommend Native here because the nine tasks share source state, manifest types, and runtime ownership.
