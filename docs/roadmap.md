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

These improvements are ordered from easiest to hardest, not by execution order. Finished items are removed; numbers stay.
Review each proposal against current source and profile evidence before implementation.
Items 11 to 13 and 15 are proposed.

Use this prompt preamble for each item:

> Recheck the current code and source behavior before editing. Preserve dirty files. Use a separate `codex/` branch and worktree for approved implementation. Design first and get approval at each required design stage. Use Steam build `11140890` as the source authority, except for an explicitly approved image-quality change. Reuse project dependencies and remove obsolete paths. Avoid per-frame allocations and repeated setup. Measure release builds with matched A/B/A runs and exclusive benchmark windows. Record instrumented call counts separately. Report garage and driving average, p95, and p99 frame times. Measure startup separately. Check real input, active physics, submitted frames, streaming, and Save/Load. Report blockers and unverified cases. Do not merge.

Effort estimates include developer work, builds, critical checks, and browser verification. One day means eight hours. Blockers can extend these estimates.

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
