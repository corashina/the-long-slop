# Performance

Goal: keep the game at about 60 FPS (see the frame rate rule in [ROADMAP.md](ROADMAP.md)).
Performance work must not change game behavior.
Remove an open item when it is done. Add it to "Done" with its PR.

## Measure

- F3 overlay: frame time, "Worker tick avg", "Worker physics / world CPU avg", "WASM main / worker", draw counts, GPU prop rows, ECS entity count.
- CPU profile (`web/cpu-profile.js`): stages Physics, RenderPreparation, Input and others.
- No GPU time per pass yet (open item 1).
- Measure with a release build (`scripts/build.ps1`), not `-Fast`.

## Architecture facts

- The simulation runs in a Web Worker (`crates/simulation-worker`). Render FPS depends on the main thread and the GPU. A slow worker makes the simulation slow, not the frame rate low.
- Physics steps at 50 Hz, at most 2 steps per frame (`STEP_BUDGET` in `crates/simulation/src/simulation.rs`). The renderer interpolates poses.
- WebGPU is the only renderer. MSAA is 4x.
- World props draw through the own GPU path: compute cull and indirect draws (`src/presentation/prop_draws.rs`).

## Open items

Size: S is up to a day. M is a few days. L is about a week.
Impact is an estimate from the code. No item has F3 numbers yet.

### No visible change

| # | Item | Size | Impact | Notes |
| --- | --- | --- | --- | --- |
| 1 | GPU time per pass in F3 | S-M | Enables ranking | WebGPU `timestamp-query`. Shows if the game is GPU or CPU bound. Do this first. |
| 2 | Render bundles for static draws | M-L | Medium-high if CPU bound | Buildings, roads, terrain and parked cars encode the same draws each frame. |
| 3 | GPU grass | L | Medium-high in grass areas | Grass blades are built on the CPU and uploaded each change. |
| 4 | Send only changed poses | M | Low-medium | `FramePoses::fill` sends `previous` and `current` rows for every handle each frame, also sleeping ones. With a change list, the renderer can also skip sleeping item visuals. |
| 5 | Cache static shadow casters per cascade | M | Medium if shadow bound | Redraw a cascade only when it moves or the sun turns past one texel. |
| 6 | Warm prop row pipelines before use | M | Removes hitches | A compiling row pipeline makes the whole view wait (`prop_draws.rs`). |
| 7 | Drop own QueryBounds BVH | M | Low on render FPS | Worker side (`crates/simulation/src/vehicle/query_bounds.rs`). Check why it exists first. |

### Optional quality settings

The default stays as now. Each item adds one menu option.

| # | Item | Size | Impact | Notes |
| --- | --- | --- | --- | --- |
| 8 | More scene scale steps or auto scale | S-M | High if GPU bound | Only 100% and 75% exist (`crates/simulation/src/render_scale.rs`). |
| 9 | Shadow atlas size option | S | Medium-high if GPU bound | 4096x4096 at 1080p and above (`source_atlas_size`). |
| 10 | MSAA off option | S | Medium on weak GPUs | The code supports 1 sample. |
| 11 | AO quality option | M | Medium on weak GPUs | Multi-scale AO at full resolution. |
| 12 | Grass density and distance option | S | Medium in grass areas | |

Rejected: async compute, bindless, Hi-Z occlusion, f16 and subgroups.

## Done

Not measured in a browser unless the row says so.

| Change | PR |
| --- | --- |
| Body descendant node lists are built once per prototype, not per moved body per frame | #273 |
| Release build uses fat LTO | #273 |
| Survival and sleep HUD write to the DOM only on change | #273 |
| Simulation in a Web Worker (tested on PC 2026-10-27) | #153-#211 |
| Own GPU prop path: compute cull, indirect draws | #122 |
| Flat prop entities | #120 |
| Bevy GPU preprocessing | #119 |
| LOD scheduler | #118 |
| Shadow cascades 2 and 3 every 2nd and 4th frame | #117 |
| Camera-depth pass removed, MSAA depth resolved | #116 |
| Idle world scans skipped | #135 |
| Heightfield terrain for regular tiles | #131 |
| Structure retirement uses cached structure | #49 |
| simd128 and wasm-opt -O3 | #48 |
| WebGPU only (WebGL removed) | #46 |
| Sun cascades sampled in the surface shader | #41 |
| Physics capped at 2 steps per frame | #40 |
| One draw per opaque surface | #38 |
| Terrain tiles in a Web Worker | #35 |
| TAA and motion blur removed | #27 |
