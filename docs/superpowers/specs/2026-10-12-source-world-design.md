# Source road and world generation

Date: 2026-10-12, Europe/Warsaw.
Status: The user approved the design in chat. Written specification review remains pending.
Branch: `codex/source-world`.
Base commit: `dccd516387dff48bf9388ad96ecd52eab3f29c00`.

## Intent and scope

Recreate road and world generation from installed Steam build `11140890`.
Match source initialization, random progression, road geometry, terrain, biomes, and placement rules.
Preserve the imported garage, continuous garage exit, current Lada, and supported interaction state.
Keep active geometry bounded during driving, walking, rebasing, and revisits.
Retain changed objects as compact records. Prepare those records for later storage.

The user selected complete change retention without a record limit.
Active geometry remains bounded. Record memory grows with changes.
Measure these two memory costs separately.

This specification replaces the earlier procedural-road limit in the lighting specification.
Preserve accepted lighting, materials, controls, garage geometry, and vehicle behavior.
Keep storage, save UI, offscreen simulation, other vehicles, survival, audio, multiplayer, and deployment outside this work.
Use the existing Rust, Rapier, Bevy, Serde, and Python export dependencies.

## Source authority and current evidence

Use installed build `11140890`, Unity `2021.3.18f1`, and its checked library records.
Keep recovered assets, decompiled code, source captures, and analysis tools outside Git.
Write original implementation code. Retain source identities and hashes in evidence.

The initial trace checked the installed and archived `Assembly-CSharp.dll` hashes.
Both match `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
The checked decompilation hash is `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee`.
Source files remain under `.cache/reference-library/` in the main checkout.

| Selected scene record | Proven role |
| --- | --- |
| `level1:64898` | `TerrainGenerationSettings`; selects one road generator and terrain settings |
| `level1:64771` | `roadGenScript`; selected road progression and biome prefab links |
| `level1:64750` | `roadBuildingScript`; sequential building groups, spacing, and side selection |
| `level1:64900` | `TerrainGenerator`; terrain samples, road alignment, and biome selection |
| `level1:64899` | `TerrainManager`; generation readiness and startup dependencies |
| `level1:65060` | Near props; `ObjectGenerationScript`, seed offset 20 |
| `level1:64958` | Mountains; `ObjectGenerationScript`, seed offset 10 |
| `level1:64720` | Towers; `ObjectGenerationScript`, seed offset 30 |
| `level1:65037` | Desert buildings; `ObjectGenerationScript`, seed offset 40 |

The selected road has 100-metre segments and ten 10-metre bones.
Its configured length is 5,000,000 metres. Preserve that finite source extent.
`CalcOneRoad` carries `doublePos`, `sumRot`, and the segment index into the next segment.
Road generation therefore depends on previous segments.
The selected road disables road and building alignment while sampling terrain for slopes.
It enables world-edge shaping and reduces first-segment horizontal turns with `straightModifier`.

`TerrainGenerationSettings.FStart2` starts terrain before starting the selected road generators.
With `randomSeed` enabled, it replaces the serialized seed with `mainscript.M.seed`.
The serialized seed alone does not establish the new-game seed.
Trace its caller before accepting initialization evidence.

Road curves and terrain use `Mathf.PerlinNoise`.
Buildings use sequential `System.Random` state and changing selection pools.
`GenerationJob` derives a position seed, selects by cumulative weights, then draws rotation and an object seed.
`roadscript.RandomPoleRot` uses a separate seed derived from the world seed and segment index.
The existing lamp module already contains a source seeded random implementation.
Validate and share that implementation instead of adding a second random generator.
Support the source large-range integer draw used by `GenerationJob`.

These findings establish control flow and selected fields. They do not establish numeric or runtime parity.
Native noise, rotation, float rounding, deformed meshes, and spawn callers require independent comparison.

## Required source recovery

Trace `mainscript`, `TerrainGenerationSettings`, `TerrainManager`, `TerrainGenerator`, `TerrainGenerationJob`, and selected road startup callers.
Trace `roadGenScript`, `roadscript`, `roadBuildingScript`, `ObjectGenerationScript`, and `GenerationJob` through generation and placement.
Trace selected prefab callers of `buildingscript`, `newRandomStuffSpawnScript`, `stuffspawn`, and `terrainRandomStuffSpawn`.
Trace the garage terrain alignment and hole rules.
Resolve active serialized links before using another terrain or spawn system.

Recover each random stream's seed, draw order, range bounds, branch conditions, and reset points.
Distinguish `System.Random`, Unity random state, coordinate noise, and deterministic transforms.
Keep visual update timing out of road and placement random streams.
For shared Unity random draws, include the relevant caller order and captured initial state in comparison inputs.
Do not invent independent seeds for callers that share a source stream.

Recover road prefab bones, bind poses, skin weights, triangles, normals, UVs, materials, and collider ownership.
Recover terrain dimensions, resolution, border samples, normals, colors, world-edge shaping, and hole rules.
Recover biome thresholds, override behavior, and road prefab selection.
Recover building groups, repeat rules, spacing ranges, side offsets, orientation, and terrain alignment helpers.
Recover prop weights, empty selections, biome masks, road exclusions, and building exclusions.
Recover source lamps from the selected road prefab for each supported biome.

Create independent samples using checked source behavior and native primitive outputs.
Do not use the replacement generator to produce its own expected values.
Bind samples to input hashes, source record identities, seed, initial state, and sampler version.
Keep uncertain source behavior explicit in the research report. Resolve it before accepting that layer.

## Architecture and data flow

Keep generation free of renderer and physics handles.
Use source generation units independently from streaming and asset loading.

| Component | Responsibility |
| --- | --- |
| `tools/reference/world.py` | Recover checked world inputs and required scene assets through the existing exporter |
| `src/reference/world.rs` | Validate typed world inputs and dependency links |
| `src/world/random.rs` | Own checked source random state and numeric helpers |
| `src/world/road.rs` | Advance road state, restore checkpoints, and produce segment geometry |
| `src/world/terrain.rs` | Generate terrain geometry, colors, normals, and garage holes |
| `src/world/placement.rs` | Produce structure, station, prop, and lamp placement records |
| `src/world/changes.rs` | Retain stable identities and supported object changes |
| `src/world.rs` | Own streaming windows, active geometry, generation jobs, and absolute coordinates |
| `src/presentation/world.rs` | Load shared source prototypes and manage visible instances |

Extend the existing manifest and export command. Keep one current schema and loading path.
Reject missing required records and obsolete exports.
Validate source hashes, finite values, array sizes, identities, bone bindings, weights, and asset dependencies.
Report a failed source identity and field. Keep Start disabled when required inputs cannot load.

The runtime consumes validated inputs and a signed source world seed.
Keep the current fixed new-game seed unless the checked startup contract requires a different input.
Add no seed settings UI.

Generate terrain noise inputs first, then advance road state and compute deterministic placement records.
Apply source road and building alignment when producing terrain geometry.
Instantiate mesh, collision, and interaction resources from the same generation records.
Apply retained object changes before exposing restored instances to interaction or physics.
Publish source biome output to the existing lighting path.

## Road progression and checkpoints

Preserve source arithmetic order, casts, range bounds, angle limits, and first-segment rules.
Keep source double accumulation separate from float rotations and step vectors.
Apply the existing coordinate conversion once: source positive Z becomes runtime negative Z.
Preserve the source start transform and height relationship with the imported garage.
Do not shift or reshape the imported garage to fit a substitute road.

Store road progression at fixed segment intervals within the finite source road extent.
A road checkpoint contains the next segment index, accumulated position, rotation, and required evaluator state.
A building checkpoint contains its random state, selection pools, group counts, previous type, and next distance.
Record checkpoint state only at the same source operation boundary.
Include each additional carried field found by the completed source trace.

Replay from the nearest preceding checkpoint for an unloaded region.
Discard replay geometry outside the requested region.
Bound pending work and checkpoint count from the source extent and fixed checkpoint interval.
The implementation plan must specify that interval and its worst replay cost before coding.
Do not retain complete road or building arrays for the source extent.
Store absolute bounds for completed checkpoint blocks. Use those bounds to select blocks for spatial replay.
Expand block bounds by source mesh extents and placement offsets before testing a streaming window.
Replay an incomplete block from its checkpoint within the bounded generation job.

Use segment identity for road samples and joins.
Find active segments by spatial position; remove the existing assumption that distance equals negative absolute Z.
Use source bone transforms to deform the selected road mesh.
Publish the same positions and triangles to rendering and Rapier collision.
Preserve source pole transforms and their separate rotation draws.

## Terrain, biomes, and placement layers

Generate terrain on the source X/Z tile grid.
Preserve local and distant terrain dimensions, sampling, and source placement offsets.
Compute border samples before normals so adjacent tiles share surface normals.
Preserve source road alignment, building alignment precedence, world-edge shaping, and garage holes.
Validate the garage floor, inspection pit, entrance, and terrain-to-road join.

Evaluate source color channels and biome thresholds at source coordinates.
Use the source biome to select road meshes, materials, and supported props.
Preserve forced-biome behavior when it forms part of checked inputs. Add no new biome controls.

Add layers to the working runtime in this order:

1. Source road and terrain collision with a continuous garage exit.
2. Terrain colors, biome selection, and distant terrain.
3. Roadside structures and stations with source meshes and collision.
4. Source prop placement and supported nested spawn callers.
5. Road poles and lamps using the existing lighting controllers.

Preserve source draw order when adding each layer.
Treat weighted selection, biome rejection, spacing, and exclusion checks as separate placement steps.
Preserve empty selections. Do not reroll rejected objects unless the source caller does so.
Validate exact selection identities as well as observed placement rates.

Reuse current interaction modules for supported items, pumps, and lamps.
Recover source supply state and rates. Remove fixed synthetic pump replenishment during streaming.
Do not generate other vehicles or add unsupported gameplay through nested prefab spawn callers.
Preserve relevant random draws when a source caller selects content outside the approved product scope.
Record those exclusions in evidence. Do not substitute another object or claim complete content parity.

## Bounded streaming and floating origin

Keep bounded spatial windows around the player and current vehicle.
Preserve parked vehicle ground support when the player walks away.
Use checked source view distances with bounded job counts and active resource budgets.
Specify concrete window limits and byte budgets in the implementation plan before coding.
Reuse work buffers. Allocate meshes and colliders during generation and loading, not ordinary frame updates.

Unload inactive render instances, mesh buffers, colliders, lamp instances, and interaction bindings.
Share prototype assets across instances. Release unreferenced assets without retaining unloaded instance state.
Keep unchanged object state reproducible from seed, checked inputs, and stable identity.
Cancel stale generation requests before publishing resources.
Publish road and terrain collision before the player or vehicle can enter a new region.
Keep the existing playable region while preparing its replacement.

Keep world coordinates and retained object positions in absolute double precision.
Keep renderer and physics coordinates local to the current origin.
Rebase between physics steps through one shared translation.
Shift active bodies, terrain, road, imported objects, player state, lamp poses, and interaction targets together.
Preserve velocities, joints, absolute positions, object identities, and generation inputs.
Origin changes must not alter a random stream, checkpoint, or placement decision.

## Changed objects and later persistence

Identify generated objects by layer, source generator identity, generation unit, and source spawn ordinal.
Use nested source identities and child ordinals for generated contents.
Keep identities independent of load order, renderer entities, collider handles, and origin shifts.
Keep imported object identities distinct from generated identities.

Retain only supported changes from the deterministic baseline:

- Absolute pose and rotation for moved objects.
- Removal records for destroyed or collected objects.
- Attachment relationships and existing supported condition, fluid, and lamp state.
- Dynamic velocities and supported physics state captured at unload.

Freeze unloaded objects at their captured state. Restore their captured state on revisit.
Add no offscreen simulation.
Update the spatial ownership of moved objects so they load at their destination.
A moved-object record suppresses its original generated instance.
Index that same record by its destination region. Keep one current record for its identity.
Keep carried or attached objects active with their current owner.
Restore relationships before enabling physics and interaction.

Mark changes when supported state changes. Do not scan the complete record set each frame.
Merge repeated edits into one current record per identity.
Remove a record when the object returns to its complete deterministic baseline.
Keep removed-object records for the session.

Define records with Serde using stable identities, numeric state, and absolute poses.
Exclude renderer handles, physics handles, pointers, and file paths.
Expose session record access for future storage. Add no storage backend, save format migration, or compatibility layer.
Restart creates a new session and clears its changes through the existing restart path.

## Acceptance and measurements

Accept each layer with critical tests and source comparisons before adding the next layer.
Keep acceptance samples fixed before comparing the replacement.
Use source seeds 42, 0, -1705217151, and the signed integer boundary values for random-state checks.

| Area | Required evidence |
| --- | --- |
| Initialization | Selected caller order, seed replacement, starting transforms, and garage height relationship |
| Random state | Exact seeded draws, small and large ranges, branch draws, resets, and checkpoint restoration |
| Road samples | Source segment positions, bone rotations, slopes, curves, and joins at startup and distant indices |
| Replay | Sequential and checkpoint replay produce identical outputs for the same complete inputs |
| Geometry | Source mesh deformation, normals, triangles, materials, and matching collision surfaces |
| Terrain | Independent height, normal, color, biome, alignment, and hole samples |
| Placement | Exact candidate identities, rejection results, transforms, and fixed-seed frequency comparisons |
| Seams | Drive both directions across garage, road, terrain, and streaming boundaries without collision discontinuity |
| Rebase | Repeat positive and negative origin shifts while driving, walking, carrying, and parking |
| Revisit | Return after unloading moved, removed, refilled, drained, attached, and switched supported objects |
| Lifecycle | Restart and canceled jobs release resources and prevent stale instances or duplicate bindings |
| Memory | Active resources plateau on an unchanged route; record bytes track distinct changed identities |

Require exact integer values, selected identities, draw counts, checkpoint state, and triangle topology.
One ULP is the interval between adjacent float values at the sampled magnitude.
Compare native float primitive samples within four ULPs.
Compare sampled bone rotations within 0.0001 degrees.
Compare positions within the larger of 0.001 metres or eight source float ULPs for that coordinate.
Report absolute errors and ULP errors. Report long-route accumulation separately.
Require identical shared seam positions from the replacement geometry, with no missing or duplicate collision faces.
Do not relax tolerances after observing failures without reviewing the source evidence and updating this specification.

Sample road startup, checkpoint boundaries, tight curves, steep slopes, biome transitions, and the configured road end.
Compare at least 10,000 placement candidates across the fixed seeds.
Match source candidates and accepted placements exactly where integer decisions control selection.
Treat rate comparisons as supporting evidence, not a replacement for exact selection checks.

Run an unchanged 100-kilometre route, then revisit it in reverse.
Run a separate 5,000-kilometre streaming traversal to the source extent.
Perform repeated rebases and checkpoint replays during both routes.
Use automated traversal for distance coverage and continuous driving for garage and collision checks.
Distinguish automated traversal from real driving in the evidence.

Record active and peak counts for road segments, terrain tiles, bodies, colliders, meshes, lamps, and interaction bindings.
Record checkpoint bytes, shared asset bytes, active geometry bytes, pending job bytes, and change-record bytes.
Measure retained WASM memory, process memory where available, and logical GPU bytes.
Report driver memory as unavailable when the tools cannot measure it.
Repeat the unchanged route and restart to detect retained resources.
Repeat object edits to verify record growth follows distinct changed identities.
Report generation time and longest frame during checkpoint replay and streaming.

Run native world, reference, starting-area, lighting, and interaction checks affected by the replacement.
Run installed export verification, the production WASM build, and browser acceptance.
Use the existing Docker toolchain and project scripts.
Do not claim source runtime parity from synthetic fixtures or formula reconstruction alone.

## Replacement and review gates

Keep the existing product playable while validating the source replacement in the isolated branch.
Do not add a runtime generator selector or an obsolete-export fallback.
Replace each validated runtime layer through one current interface.
Remove the sine road, fixed station spacing, simple prop generation, obsolete lamp placement, and unused helpers after acceptance.
Preserve unrelated startup randomness while removing world generator helpers.
Remove obsolete tests and documentation for the deleted generator.
Check callers for stale negative-Z distance assumptions and synthetic station identities.

Before implementation, review this written specification and approve the implementation plan and execution method.
The implementation plan must set checkpoint intervals, concrete streaming limits, memory budgets, and source sampling commands.
Use the working branch checkpoint for each accepted layer.
Keep unresolved numeric or browser evidence visible in verification notes.
