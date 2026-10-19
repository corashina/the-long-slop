# Project roadmap

Updated: 2026-10-19.
Source authority is Steam build `11140890`. Save schema is 5.

## Current state

Master contains the source world, two cars, parts, mechanisms, survival, sleep, and the source renderer.
It also contains source audio, flashlights, plates, cleaning, painting, driving controls, and inventory tasks 1 and 2.
Builds and reviews passed. Runtime acceptance of the merged game is still open.

The export used for scene-resolution checks stopped loading at an unexpected `exclusions` field in material data.
The installed export changed after those checks. Its native manifest validation now passes.
Fresh browser acceptance and performance measurements remain open.
An earlier package reached "The garage is ready." New game then failed with `Missing source asset: reference/food.gltf`.
That file exists. Session validation looks for its lazy prototype in the eager scene cache.

## Work order

Keep each layer working before you add the next.
Follow the check rules in [AGENTS.md](../AGENTS.md). The check script does not build release WASM.

1. **Fix New game asset validation.** Match eager checks to the eager scene owners.
   Keep strict checks for loaded scenes and for each lazy prototype's owner.
2. **Build and check one release package.** Use one source snapshot.
   Confirm garage readiness and "New game ready" on that exact WASM. Then install the matching assets and WASM.
3. **Verify active gameplay.** Get mouse capture. Check walking, preparation, both cars, cleaning, painting, Save/Load, recovery, and restart.
   Drive out of the yard. Check ground, collision seams, long drives, rebasing, revisits, memory, and audio.
4. **Measure source driving.** Collect original-game measurements before you change tire, rim, hub, suspension, or brake response.
   Then verify launch, both-car driving, wheel spin, and 30/60/120 FPS behavior.
5. **Resume inventory tasks 3 to 10** when the user asks.

Performance proposals below do not change this order.

## Inventory tasks 3 to 10

Stopped at the user's request. Branch: `codex/inventory-clothing`.

| Task | Remaining scope |
| --- | --- |
| 3 | Explicit equip/release, direct Tab behavior, and source item-action priority. |
| 4 | Station loot selection, source RNG order, item creation, stable identities, and saving. Keep empty and unsupported outcomes. |
| 5 | Source box contents and one-time release. Save the release state. |
| 6 | Shovel digging and buried-item recovery. Save burial and recovery state. |
| 7 | Supported wearables, cigarettes, and holiday startup. Keep disabled starter apparel disabled. |
| 8 | Both player skins, source pose and IK, wearable anchors, and camera visibility. |
| 9 | Appearance controls and remaining HUD behavior. Commit profiles atomically. |
| 10 | Compare the feature with the running source game. Record limits. |

## Performance improvements

These 15 improvements are ordered from easiest to hardest, not by execution order. No performance gain is verified.
Review each proposal against current source and profile evidence before implementation. Item 2 changes image quality and needs specific approval.
Items 1 to 5, 7 to 9, and 14 are merged. Items 6, 10 to 13, and 15 are proposed.

Use this prompt preamble for each item:

> Recheck the current code and source behavior before editing. Preserve dirty files. Use a separate `codex/` branch and worktree for approved implementation. Design first and get approval at each required design stage. Use Steam build `11140890` as the source authority, except for an explicitly approved image-quality change. Reuse project dependencies and remove obsolete paths. Avoid per-frame allocations and repeated setup. Measure release builds with matched A/B/A runs and exclusive benchmark windows. Record instrumented call counts separately. Report garage and driving average, p95, and p99 frame times. Measure startup separately. Check real input, active physics, submitted frames, streaming, and Save/Load. Report blockers and unverified cases. Do not merge.

Effort estimates include developer work, builds, critical checks, and browser verification. One day means eight hours. Blockers can extend these estimates.

1. **Compress startup downloads.**
   Status: Implemented and merged into local master at `499b685`. Merge notes are in `b46c862`.
   The worktree is archived. The `codex/startup-brotli` branch remains.
   Owners: [startup server](../scripts/serve.mjs), [world assets](../web/world-assets.js).
   Build-time Brotli covers WASM and current package JSON. Receipt checks remain intact. Compressed images remain unchanged.
   Checks: 46 web checks passed. Package receipt verification passed. Tested HTTP transfers used 90.17% fewer body bytes.
   Conditional requests returned 304 without a body. No startup speed gain is verified.
   Results: [Brotli delivery results](performance/2026-10-19-startup-brotli.md).
   Local evidence: `.cache/performance/2026-10-19-startup-brotli/` retains measurements and the compression index.
   Earlier blocker: The tested WASM and export mismatched at field `exclusions` under identity and Brotli delivery.
   The replaced export passes native manifest validation. Fresh browser checks remain open.
   Remaining: Build matching release WASM and export. Compare cold and cached playable startup in exclusive matched A/B/A runs.
   Check real input, active physics, submitted frames, streaming, and Save/Load. Garage and driving frame times remain unmeasured.

2. **Lower internal rendering resolution.**
   Status: Implemented and merged into master at `fad65b2`. Source commit: `b08a292`.
   Owner: [scene sizing](../src/app.rs). Controls offers 100% and 75%; each page starts at 100%.
   Scene targets and effects scale together. UI, mouse deltas, projection, shadow resolution, and 64×64 probes keep their existing paths.
   Checks: Release WASM build and packaging passed. Merged checks passed: 63 native tests and 47 web tests. WASM check passed.
   Results: [Scene resolution checks](performance/2026-10-19-scene-resolution.md).
   Local evidence: `.cache/performance/2026-10-19-scene-resolution/` retains packages, receipts, and merge checks.
   Earlier blocker: Both releases rejected the frozen export's unexpected `exclusions` field before renderer creation.
   The replaced export passes native manifest validation. Fresh browser checks remain open.
   Remaining: Check image quality, resize, fullscreen, aiming, probes, streaming, and Save/Load at both scales.
   Measure garage and driving average, p95, and p99 frame times in exclusive matched release A/B/A runs.
   Runtime acceptance and performance gains remain unverified.

3. **Reuse unchanged content lookup tables.**
   Status: Implemented and merged into master on 2026-10-19. Runtime performance remains unverified.
   Owners: [content index](../src/world/content_index.rs), [instance presentation](../src/presentation/world/instances.rs).
   The cache keeps membership and order. Lookups read live pose and appearance.
   See the [validation note](content-lookup-cache.md) for merged-tree checks and historical benchmark limits.

4. **Remove redundant fullscreen image copies.**
   Status: Implemented and merged into master at `2c80503`. Source commit: `2e3d483`.
   Owner: [source effects](../src/presentation/source_effects.rs).
   Uber writes one value to all MSAA samples and the resolve. Far and Main no longer copy it to a scratch image and back.
   The scratch image and its callback audit counts are removed. The display copy keeps only its sRGB path.
   Finite values, half-float rounding, pixel sampling, and MSAA are unchanged.
   One case differs: on some GPUs, the bilinear copy could spread Inf or NaN from a neighbor pixel. That no longer occurs.
   Checks: Native and WASM Clippy show no new warnings. WASM check passed.
   Remaining: Check the image in a browser with a fresh export. Measure garage and driving average, p95, and p99 frame times.

5. **Reduce MSAA8 to 4.**
   Status: Merged, not measured. Approved by the user. `MSAA_SAMPLES` sets 4 samples for all three cameras, targets, and pipelines. The source export still records 8. Difficulty: Moderate. Estimate: 2–3 days. Owner: [render targets](../src/presentation/render_targets.rs).
   Effect: Halve the multisample color and depth memory and resolve cost. Edges, glass, and particles are not compared in a browser.
   Prompt:
   ```text
   After specific approval, inspect all sample-count checks and render targets. Add one fixed 4-sample profile and keep camera, target, pipeline, effect, and device counts consistent. Preserve layers, shadows, and effects. Check edge detail, glass, particles, and resize. Compare image quality and GPU cost with the current 8-sample profile.
   ```

6. **Split expensive streaming steps.**
   Status: Implemented and merged into master on 2026-10-19. Source commit: `760af3d`. Worker integration: `f13afc7`.
   Terrain work resumes at deadline checks in the shared terrain crate.
   Native worker polling uses the deadline. Browser terrain runs in the worker from item 14.
   Native terrain measurements selected the stage. Native tests and the WASM compile check passed.
   Browser testing is deferred by request. Driving frame times and runtime acceptance remain unverified.
   Earlier profiling hit mouse capture and Load failures.
   Checks and profiling commands: [Streaming stage measurements](performance/2026-10-19-streaming-steps.md).
   Difficulty: Hard. Estimate: 3–5 days. Owner: [world runtime](../src/presentation/world/runtime.rs).
   Effect: Bound measured long driving stages as resumable work within the current frame budget.
   Prompt:
   ```text
   Measure each driving stage first; runtime.rs checks time only after a step. Split only the measured costly stage into resumable bounded units within the existing budget. Preserve deterministic output, ownership, cancellation, collision priority, publication order, and source road timing. Compare matched routes and report p95 and p99 frame times.
   ```

7. **Batch opaque shadows across surface materials.**
   Status: Implemented and merged into master on 2026-10-19 at `1989b3a`. Source commit: `3b17ccc`.
   Runtime performance remains unverified.
   Owner: [shadow phase](../src/presentation/shadow_phase.rs).
   Scope and checks: [opaque shadow batching](performance/2026-10-19-opaque-shadow-material-batching.md).
   Effect: Reduce draw splits by material identity for proven equivalent opaque casters.
   Prompt:
   ```text
   Inspect material-based splits in shadow_phase.rs. Batch only casters with equivalent opaque behavior, geometry, and pipeline. Exclude cutouts, transparency, grass, impostors, and LOD fades. Preserve transforms, cascade masks, offscreen casters, and exact instance selection. Verify each cascade and report draw counts separately from FPS.
   ```

8. **Update shadow scenes incrementally.**
   Status: Implemented and merged into master on 2026-10-19 at `5004b7e`. Source commit: `194e5ec`.
   Runtime performance remains unverified.
   Owner: [shadow scene](../src/presentation/shadow_scene.rs).
   Caster component changes update cached rows. Membership, scene, origin, and recovery changes rebuild the ordered collection.
   Revisions skip unchanged caster and mask copies. Camera-dependent selection, fitting, and per-frame visibility remain.
   See the [implementation note](performance/2026-10-19-incremental-shadow-scene.md) for checks and limits.

9. **Select local lights per object.**
   Status: Implemented and merged into master on 2026-10-19. Source commit: `f7b7e53`.
   Runtime acceptance and performance remain unverified.
   Owner: [surface shader](../web/assets/shaders/source_surface.wgsl).
   The additive vertex pass builds a conservative 64-bit mask from full renderer bounds.
   Selected lights keep source slot order. Deformed meshes retain all eligible local lights.
   See the [implementation and check note](performance/2026-10-19-object-local-lights.md).
   Effect: Skip only local lights that cannot affect an object's full bounds.
   Prompt:
   ```text
   Inspect the lamp loop in source_surface.wgsl. Exclude a light only when conservative full-object bounds prove it cannot contribute. Preserve light order, cookies, attenuation limits, sunlight, and moving-light and moving-object behavior. Recheck origin shifts and streaming. Compare image output and CPU selection cost with GPU time saved.
   ```

10. **Track physics query mutations.**
    Status: Proposed. Difficulty: Hard. Estimate: 5–8 days. Owner: [vehicle query bounds](../src/vehicle/query_bounds.rs).
    Effect: Replace repeated query-bound scans with complete mutation tracking.
    Prompt:
    ```text
    Audit every collider write, including rapier_mut, position and shape changes, attachments, enabled state, add, remove, and generation. Track all query-bound mutations. Preserve physics notifications and any intervening refresh mechanism. Compare existing query results for wheels, doors, parts, contacts, and Save/Load.
    ```

11. **Use native BC streamed texture arrays.**
    Status: Proposed. Difficulty: Very hard. Estimate: 5–9 days. Owner: [world textures](../src/presentation/materials/world_textures.rs).
    Effect: Replace eligible RGBA8 world texture arrays with native BC1 or BC3 arrays. Preserve source blocks and mip levels.
    Prompt:
    ```text
    Reuse exporter eligibility and the primary texture loader. Preserve source BC blocks and mip levels; do not generically recompress. Add compatible arrays, block uploads, receipts, byte accounting, cancellation, and acknowledgement. Verify normal, alpha, color, and terrain LOD paths, including failed or cancelled uploads.
    ```

12. **Split startup metadata and world chunks.**
    Status: Proposed. Difficulty: Very hard. Estimate: 7–12 days. Owners: [reference loader](../src/reference.rs), [web manifest](../web/main.js).
    Effect: Replace the roughly 139 MB monolithic manifest reviewed at `2dadfc0` with hashed chunks and a small garage dependency closure.
    Prompt:
    ```text
    Get architectural approval for chunk ownership and validation before editing. Define the smallest garage dependency closure, then add hashed chunks with existing receipts and budgets. Preserve identities, validation, generations, save references, distant loads, and cancellation. Remove the monolithic path. Measure transferred bytes, parse time, peak memory, and playable readiness.
    ```

13. **Combine opaque base and additive lighting.**
    Status: Proposed. Difficulty: Very hard. Estimate: 1–3 weeks. Owner: [source lighting](../src/presentation/source_lighting.rs).
    Effect: Reuse geometry and material samples to combine opaque standard-material draws.
    Prompt:
    ```text
    Inspect the two source lighting draws. Combine opaque standard materials only, reusing geometry and material samples. Preserve transparent order, blend rounding, fog, emission, depth, sunlight, lamps, and shadows. Capture matched source images first. Report pixel inequivalence before changing fidelity.
    ```

14. **Move terrain work to a Web Worker.**
    Status: Merged, not measured. Owners: [terrain crate](../crates/terrain/src/lib.rs), [worker](../web/terrain-worker.js), [client](../src/world/terrain.rs).
    Tile meshes come from one Web Worker with a 42 KB WASM. Results match the old job bit for bit in native checks.
    Effect: Evaluate pure terrain data in one worker with its own WASM; keep engine ownership on the main thread.
    Prompt:
    ```text
    Design a worker boundary for pure terrain evaluation with its own WASM and bounded transferable inputs and outputs. Keep Bevy, Rapier, admission, and publication on the main thread. Preserve session and job generations, stale-result cancellation, load and world replacement, origin rules, exact geometry, and budgets. Measure transfer copies, duplicated RAM, and stalls. Do not move the whole engine to workers.
    ```

15. **Add a WebGPU renderer.**
    Status: Proposed. Difficulty: Architectural. Estimate: 2–4 weeks. Owner: [Cargo features](../Cargo.toml).
    Effect: Add a WebGPU backend after equal-quality behavior works across the current renderer paths.
    Prompt:
    ```text
    Check current Bevy and wgpu support, then audit WebGL shaders, depth, formats, MSAA, and custom phases. Reuse existing dependencies where possible. Match image quality before comparing backends; add GPU-driven features only after that comparison. Verify garage, driving, shadows, glass, streaming, resize, and Save/Load. Remove obsolete WebGL paths after acceptance. Do not claim a performance gain without matched measurements.
    ```

## Feature gaps in scope

| Feature | Status | Remaining work |
| --- | --- | --- |
| Source world and buildings | Implemented; runtime unverified | Check streaming, revisits, interiors, placement, collision, and memory in the release package. |
| Driving physics | Partial | Source measurements first. Then wheel, rim, hub, suspension, and combined-brake work. |
| Inventory and clothing | Partial | Tasks 3 to 10 above. |
| Cleaning and painting | Partial | Verify active tools. Add spray refraction and brush soft-particle depth fading. |
| World population | Partial | Source loot placement (tasks 4 to 6). |

Not in current scope (update PRODUCT.md first): further vehicles, cargo and towing, remaining items, creatures, weapons, combat damage,
saved needs, save slots, autosave, general settings, rebinding, custom radio music, multiplayer, deployment,
weather, live mirrors, lamp shadows, motion blur, and TAA.

## Known limits

- Earlier reports: New game took 43 s with cached assets and 79 s on a new profile. Shader compilation took about 40 s on a new browser.
  Four audio decode failures were reported. Reproduce each with matching assets before you treat it as current.
- A loose flashlight fell below the floor during a long browser run. The cause is unknown.
- The in-app browser refuses pointer capture. Continuous mouse control and active tool use are unverified.
- No matched native frame proves identical pixels or blend rounding. Nine exported primitives lack authored tangents.
- At zero wheel RPM, the port can stay in neutral. Source curve extrapolation is unverified.

Implementation owners: [vehicles](vehicles.md), [save contract](../src/save/schema.rs), [survival](../src/survival.rs),
[cleaning](../src/interaction/cleaning.rs), [world](../src/world.rs), [audio](../src/audio.rs).
Git history keeps the earlier verification records, hashes, and integration notes.
