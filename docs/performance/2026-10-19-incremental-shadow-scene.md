# Incremental shadow scene updates

Roadmap item 8 uses branch `codex/incremental-shadow-scene`.
The base is `8eca150d90e7d0fa216823276c01825f8b1189d1`.
The branch remains unmerged.

## Change

Shadow preparation retains caster rows and their entity index.
Changed component queries select rows that need bounds or eligibility checks.
The inputs cover transforms, bounds, inherited visibility, renderer state, layers, and all five material handles.
Material asset resource changes refresh eligibility in the same frame.
This avoids waiting for asset messages, which Bevy delivers in `Last`.

Component removal, warmup membership changes, and new caster membership rebuild the ordered collection.
Session changes, scene changes, origin shifts, and recovery also rebuild it.
Rebuilds retain allocated storage and preserve query order.
The bounds cache no longer scans its table to remove unseen rows each frame.

Caster revisions change after validated data changes or a rebuild.
Render extraction copies caster arrays only when their revision changes.
Each view has a separate revision for mask extraction.
Failed preparation cannot publish a frame or reuse the last mask revision during recovery.

Camera and sun inputs still control candidate selection and cascade fitting through `ShadowViewCache`.
Per-frame visibility updates still include shadow-only objects outside camera visibility.
The source selection and fitting arithmetic for Steam build `11140890` remains unchanged.

## Checks

Targeted checks passed on 2026-10-19:

- Rust formatting passed.
- All four `source_shadow_preparation` native tests passed.
- The locked WASM library compile check passed.
- `git diff --check` passed.

No check failed.
Existing warnings remain in `world/stream.rs` for native tests and `presentation/materials.rs` for WASM.

Run the targeted checks from the worktree:

```powershell
.\scripts\check.ps1 -Test source_shadow_preparation
```

This command checks Rust formatting, runs native tests, and checks the WASM library.
The native tests compare cached masks and cascade fits with fresh computation.
They cover unchanged input, caster edits, membership rebuilds, origin and scene invalidation, and failed preparation.
They also check extraction skips and retained array storage.

## Limits

No performance gain is measured.
Bevy still checks component change ticks to find changed rows.
Material asset changes refresh the caster query.
Membership changes rebuild the full ordered collection.
Changed caster revisions still copy complete caster arrays into the render world.
Per-frame visibility checks remain.

Release builds, full checks, browser acceptance, and matched A/B/A measurements are deferred.
Garage and driving average, p95, and p99 frame times remain unmeasured.
Moving objects, streaming, origin shifts, real input, active physics, submitted frames, and repeated Save/Load need runtime acceptance.
