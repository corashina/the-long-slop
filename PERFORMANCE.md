# Performance

Goal: keep the game at about 60 FPS (see the frame rate rule in [ROADMAP.md](ROADMAP.md)).
Performance work must not change game behavior.
Add only items that increase render FPS or remove hitches. Remove an item when it is done.

## Measure

- F3 overlay: frame time, "Worker tick avg", "Worker physics / world CPU avg", "WASM main / worker", draw counts, GPU prop rows, ECS entity count.
- F3 "GPU ms": GPU time of the world view per stage, average of 30 frames. Needs the browser `timestamp-query` feature. Probe faces, sky and prop culling are not in the stages.
- CPU profile (`web/cpu-profile.js`): stages Physics, RenderPreparation, Input and others.
- Measure with a release build (`scripts/build.ps1`), not `-Fast`.

## Architecture facts

- The simulation runs in a Web Worker (`crates/simulation-worker`). Render FPS depends on the main thread and the GPU. Worker-only work does not change render FPS.
- The worker sends only changed pose rows. The renderer interpolates poses.
- Physics steps at 50 Hz, at most 2 steps per frame (`STEP_BUDGET` in `crates/simulation/src/simulation.rs`).
- Terrain tiles and grass meshes are built in their own workers.
- WebGPU is the only renderer. MSAA is 4x. The "Antialiasing" setting turns it off after a reload (`src/presentation/antialiasing.rs`). No TAA, motion blur or camera-depth pass.
- World props draw through the own GPU path: compute cull and indirect draws (`src/presentation/prop_draws.rs`).
- Shadow cascades 0 and 1 render each frame, 2 and 3 every 2nd and 4th frame (`crates/simulation/src/reference/shadow_cadence.rs`).
- Release build: fat LTO, simd128, wasm-opt -O3.

## Open items

Size: S is up to a day. M is a few days. L is about a week.
Impact is an estimate from the code. No item has F3 numbers yet.

### No visible change

| # | Item | Size | Impact | Notes |
| --- | --- | --- | --- | --- |
| 1 | Render bundles for static draws | M-L | Medium-high if CPU bound | Buildings, roads, terrain and parked cars encode the same draws each frame. |
| 2 | Skip unchanged item visuals | S | Low | The renderer still does a pose lookup and interpolation for every item visual each frame (`src/presentation/reference.rs`, `spawned.rs`). Use the changed pose rows to skip the rest. |
| 3 | Reuse static shadow cascades | M | Medium if shadow bound | Skip a cascade when its fit and its casters did not change. Start after item 1. |
| 4 | Warm prop row pipelines before use | M | Removes hitches | A compiling row pipeline makes the whole view wait (`prop_draws.rs`). |

### Optional quality settings

The default stays as now. Each item adds one menu option.

| # | Item | Size | Impact | Notes |
| --- | --- | --- | --- | --- |
| 5 | More scene scale steps or auto scale | S-M | High if GPU bound | Only 100% and 75% exist (`crates/simulation/src/render_scale.rs`). |
| 6 | Shadow atlas size option | S | Medium-high if GPU bound | 4096x4096 at 1080p and above (`source_atlas_size`). |
| 8 | AO quality option | M | Medium on weak GPUs | Multi-scale AO at full resolution. |
| 9 | Grass density and distance option | S | Medium in grass areas | |

Rejected: async compute, bindless, Hi-Z occlusion, f16, subgroups, GPU grass, own QueryBounds BVH removal.

## Final test pass

Do on Tomasz's PC after the open items are done: F3 frame time and "GPU ms" before and after each item, in a town and in a grass area.
