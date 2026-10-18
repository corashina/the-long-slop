# Opaque and shadow batching

The user approved this bounded design on 2026-10-17.
The branch is `codex/opaque-shadow-batching`.
On 2026-10-18, the user requested a local merge of the implemented Stage 1.

The source baseline is `703d8e6eb3feb6e139d386e87551647db8ad2635`.
The baseline excludes `codex/opaque-stencil`.
The clean master commit `7eb1774` has a reported startup blocker, so it cannot establish gameplay performance.

## Scope

Replace the custom sorted shadow phase with Bevy CPU batching.
Preserve per-view caster selection, exact cascade masks, and offscreen casters.
Check submitted instance coverage separately from issued draws.
Reset retained bins when a mirror or main view changes.
Group fully opaque source casters only when the checked caster shader ignores their differing surface values.
Share eligible generated opaque and cutout primary materials within one scene owner.
Share each eligible native caster with its primary material.
Keep special depth states, blend states, and mirror materials private.
Give each object private material state before an independent change.
Keep LOD materials private because startup pipeline tickets can retain their handles.
Keep transparent sorting, motion draws, gameplay entities, and physics unchanged.

## Stages

1. Retained shadow bins and draw/instance counters.
2. Checked opaque-caster grouping.
3. Shared immutable generated materials.

Verify each stage before applying the next stage.
The initial first-stage check passed `cargo check --locked --release --target wasm32-unknown-unknown --lib`.
The retained-view reset passed the full release build on 2026-10-17.
The build took 8 minutes and 37 seconds.
Existing shadow-cache and view-domain tests passed: three tests, zero failures.
The check reported 11 warnings already present in the baseline.
Runtime checks and gameplay measurements remain pending.
Later-stage patches remain unapplied until the first stage passes runtime checks.

## Matched package

Use the same toolchain image, release profile, source assets, browser, and GPU for both builds.
Both measured packages must use image `sha256:7c31ae7cbfd74737a7e0533d07ec898c642d956751ac115c35a9f1be67c64aea`.
Use the standard release profile: optimization 3, thin LTO, one codegen unit, and stripped output.
The Stage 1 WASM has SHA-256 `149187447F9C9A28A53EAE57F7CF6562FB252928BA3FBD4B611E2E5DA1CFA30F`.
The exact-source baseline rebuild was interrupted before completion.
Do not accept the earlier cached baseline package for matched measurements.
Record the completed baseline package hash before testing.
The reference manifest has SHA-256 `3B5C9A7BC370FDD9C0F71C5CFA4D773231D3DDDEABF90C0D19FB2F391C6BD51B`.
The matching runtime package is `1590268be14053b8ddc55aa5ab5f724e78ebc89df786c3bc11fc1a486a7399f3`.

Use a 1280 by 720 canvas and DPR 1.
Use one game tab and an exclusive benchmark window.
Stop builds and tests before FPS sampling.
Warm both builds and restore the same camera, scene, simulation state, and driving input.
Take at least three 10-second samples for each scene and build.
Check baseline, candidate, and baseline again for drift.
Measure WebGL draw calls in separate samples.

## Reproduction

Run `scripts/benchmark-batching.mjs` with explicit package and asset paths.
The server reads the frozen assets and writes receipts under `.cache/batching/reports`.
Import `/__batching/probe.js` in the game tab.
Use `sample` for game frame timings and `draws` for separate WebGL counts.
Use `scripts/check-batching.mjs` to check the saved receipts.
A ready renderer or fewer draws alone does not establish success.

## Required runtime checks

Check garage and driving views, individual transforms, paint, dirt, and lights.
Check each cascade, offscreen casters, mirrors, streaming, and origin changes.
Check ordinary Save/Load and repeated Load against the baseline.
Record any baseline failure separately from a candidate regression.
Preserve screenshots and report FPS, frame times, draw counts, instance coverage, and remaining limits.

## Integration and remaining work

On 2026-10-18, the user requested merging the implemented Stage 1 into local master.
This request replaces the earlier instruction to keep the branch unmerged.
Stages 2 and 3 are not part of this merge.
The merge includes the benchmark tools, but no benchmark result is available.

Remaining implementation:

- Group compatible opaque source casters after checking the prepared caster classification.
- Share eligible generated primary materials and their native caster materials.
- Create private material state before each independent object change.

Remaining verification:

- Finish the exact-source matched baseline build and verify playable gameplay.
- Check Stage 1 at runtime before implementing Stage 2.
- Check Stage 2 at runtime before implementing Stage 3.
- Measure garage and driving FPS, frame times, and draw counts with matched release builds.
- Check cascade membership, offscreen casters, transforms, paint, dirt, lighting, mirrors, streaming, and origin changes.
- Check Save/Load and repeated Load against the same baseline.
- Record visual checks, measured gains, regressions, and remaining limits.

Each test window must exclude other builds and game tabs.
No FPS, frame-time, or draw-count gain is verified.
Native tests do not exercise the WASM render path.

Known reports from other chats include a legacy shadow-atlas validation error during driving.
They also include a Save/Load validator failure for a present source asset.
Reproduce each failure on both measured builds before classifying it.
Unique material handles still limit batching until Stage 3.
Transparent sorting, per-object motion draws, and native WebGL uniform-buffer limits remain unchanged.
