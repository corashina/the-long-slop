# Task 6 implementation layer

Status: in progress. Source publication is staged behind material admission. The existing runtime remains active.

## Source geometry

Road deformation uses checked bones, weights, bind poses, normals, tangents, UVs, and material slots.
The road assets have no color channel. The implementation does not invent one.
Source Z is reflected once. Absolute geometry is rebased only at publication.
Road biome selection uses the segment origin and strict source thresholds.
The added biome field occupies existing RoadSegment padding. Heap replay capacity remains 256 segments.

Near terrain applies source building and road alignment order. Normals include the source border.
Hole carving follows normal calculation. Terrain colors use grid coordinates and final height.
Both distant tiers retain source scaling, resolution clamps, material scale, and vertical offsets.
Direct startup height queries filter road origins at squared distance 22500.
Tile jobs use the separate RoadCoordInit2 squared distance 1000000.

StartupTerrain owns startup transforms, helper registration, shared Map height, fixed-root height, and holes.
Startup road queries use a separate bounded replay. Moving requests cannot change startup inputs.
The runtime still needs to apply these offsets to imported garage, player, and starter ownership.

## Narrow producer changes

WorldSkinReference now retains authored tangent streams. No mesh export ran.
Typed startup holes retain callers 68463/77620, filters 46406/49294, and Mesh2393's first four source vertices.
Each hole retains its source mesh receipt and hash.
PhysicsManager globalgamemanagers:10 supplies all 32 collision rows.
Its record hash is d4c5771ca0240fb4e14d193370459013911564395a2c53165f9fdba7746decc7.
Terrain prefab owner GameObjects are sharedassets1:33057,33059,33058. Their layers are 17,19,19.
Validation requires the exact owner links, receipts, matrix rows, and valid layer values.

The enriched runtime manifest is 103224436 bytes, with 54657 records and 61101 receipts.
It remains compact JSON. The first edit script used excessive indentation; self-review corrected that format.
Checked assets, source bodies, disassembly, and edit scripts remain ignored.

## Publication and lifecycle

The staged SourceWorldStream prepares the complete finite indices before world queries.
Road index construction and replay each advance at most eight segments per update.
Terrain sampling advances at most 4096 vertices per update.
Session and request identities reject stale pending output.
A publication ticket permits one publication per prepare call. The application must call prepare once per frame.
Mesh insertion is fallible. A failure leaves accepted collision intact.
Collision and rendering consume the same WorldSurface arrays.
Source layer rows apply to generated surfaces and structure recipes.

Structure recipes share collider shapes and retain bodies, gravity, damping, constraints, materials, and source identities.
Prototype admission must finish before structure recipes can publish.
Structure retirement primitives exist. Stream retirement and changed-object retention still need integration.
Supplies publish from PlacementIndex. Supply readiness requires terrain support and completed item binding.
Recovery targets stage a new request. Vehicle movement must wait for readiness.
The accepted cargo, held-object, and clearance code remains unchanged and is not yet connected to this gate.

Generated item identities are distinct from imported identities.
Stock and opening state initialize once per admitted instance. Repeated sync cannot refill stock or reset valves.
Direct source supplies contain water. No gasoline stock is inferred from a structure name.
Source fluid flow uses radius squared, PI, 0.025, pourSpeed, angle, and valve.
Checked cap fields retain noAngle, noPour, tube, defPourAngle, canToggle, canTune, and tuneSpeed.
Checked tank fields retain noFillThis, noFillFrom, noDrink, and slowOil.
SupplyDefinition emission_min_rate/emission_max_rate retain particle rates. They are not fluid rates.
The old per-sync fake pump refill was removed. Old generator removal remains Task 9 work.

## Material evidence

Checked TerrainBiome shader sharedassets0:722 was statically disassembled through system D3DDisassemble.
Retained forward vertex program 24 and fragment program 112 were inspected.
The vertex program consumes tangent, normal, UV, color, and distance-based vertical displacement.
The fragment program uses seven normal slots and five albedo slots, plus source lighting resources.
The inspected shader uses subshader 0, pass 0, keyword index 8, vertex blob 24, and fragment blob 112.
Normal texture/sampler pairs are SandNormal t0/s2, GrassNormal t1/s3, DirtNormal t2/s4, and SnowNormal t3/s5.
The remaining normal pairs are CliffNormal t4/s6, NormalMap t5/s7, and NormalMap2 t6/s8.
Albedo pairs are Dirt t7/s9, Grass t8/s10, Sand t9/s11, Snow t10/s12, and Cliff t11/s13.
Reflection cubes use t12/t13 with s0. The source SH volume uses t14/s1.
The missing input is the native value supplied to TANGENT0 when the terrain Mesh has no tangent stream.
Normal maps use DXT5 alpha/green unpacking and sequential normalized blending.
Albedo blends dirt, grass, sand, snow, then cliff. Alpha selects authored color pairs.
Eleven distinct near-terrain images plus existing lighting exceed the minimum sixteen sampler limit.
Texture arrays require matching dimensions, color space, mip count, and sampler state.
The controller is tracing the missing terrain tangent stream's native default.
No guessed tangent, dropped normal map, or Standard shader replacement was added.
TerrainBiome and other selected deferred shader families still need visible binding implementation.

## Resources and open work

WorldReference is Arc-shared. Cloning the manifest no longer duplicates its immutable world graph.
Source publication requires explicit shared graph accounting and prototype reservations.
Count guards cover 512 roads, 512 near tiles, 256 tiles per distant tier, and 8192 generated objects.
No geometry is truncated to satisfy a guard. Existing collision remains accepted after rejection.
The current geometry byte counters are estimates. They do not establish the active memory budget.
Complete accounting must include renderer copies, Rapier allocations, recipes, jobs, bounds, and source graphs.
Prototype decoded image and mesh reservations still need the actual material cache integration.
No memory measurements were run.

Remaining work includes material bindings, source lamp placement, light proxy admission, and conservative caster bounds.
The 32 light proxies and 100 shadow slots remain unchanged in the active runtime.
WorldFrame consumers, browser metrics, unsupported movement gating, and recovery transport remain unconnected.
The final WorldStream name/API replacement must happen after visible bindings are ready.
The staged source stream is implementation work, not a permanent fallback.

## Builds and checks

Native command: Docker toolchain, cargo fmt --all, cargo build --locked --lib.
Final native build passed in 5.25s, without warnings.
WASM command: cargo build --locked --release --target wasm32-unknown-unknown --lib.
Final WASM release compilation passed in 1m43s, without warnings.
Build evidence: .cache/world-source/task-6-final-build.log.
The initial WASM build took 8m07s after the Serde shared-ownership feature changed dependencies.

No tests, browser checks, captures, samplers, exports, or whole-library verification ran in this agent.
Only necessary checked records and existing accepted mesh channels were read.
Numeric, runtime, driving, seam, recovery, lighting, and memory checks were skipped as instructed.
Compilation does not establish source parity. Source acceptance remains open before obsolete generator removal.

## Commits

Implementation layer commit: pending hash. Task 6 remains in progress.
