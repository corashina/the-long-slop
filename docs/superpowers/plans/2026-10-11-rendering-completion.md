# Deferred rendering completion plan

Date: 2026-10-11.
Status: Deferred at the user's request. The current delivery ends at the stable material checkpoint.

Source: installed Steam build `11140890`, Unity `2021.3.18f1`.
Scope: the imported home, fixed Lada, preparation items, and their rendering.
Keep gameplay, physics, controls, and the existing road unchanged.

## Resume safely

1. Start an isolated worktree from `codex/material-parity`, commit `1f18cc5ada986cc32082fd51bada22b1959d02d5`.
2. Apply the stable checkpoint's build fix before compiling unfinished rendering code.
3. Read the recovery manifest, task reports, and acceptance map.
4. Verify retained input hashes before reusing evidence.
5. Keep one owner for app registration, graph wiring, and shared builds.
6. Use separate file ownership when independent effect work can run in parallel.

Ignored evidence stays in the retained worktree:
`C:/Users/Corashina/.codex/worktrees/material-parity/the-long-drive-wasm`.
Read relative evidence paths from that directory.
Git does not copy these files into a new worktree.

Local records are in `.superpowers/sdd/2026-10-10-material-parity/`.
Source evidence and fixtures are in `.cache/lighting-research/`.
The current renderer inventory and unfinished interfaces are preserved on the recovery branch.

## 1. Finish texture storage reconciliation

- Verify the source sRGB/linear storage correction already present on the recovery branch.
- Preserve separate decoded-normal and color identities.
- Verify five metallic, three AO, and one parallax RGB slot.
- Retain source alpha handling for the three alpha-only masks.
- Recheck texture sharing and resource counts; the corrected material set uses 104 textures.

Acceptance: source storage flags, sampled RGB values, alpha preservation, and loading readiness.
Keep this correction separate from lighting graph registration.

## 2. Connect a working shadow scene

Use `view_frame`, `shadow_scene`, `shadow_queue`, `shadow_phase`, `shadow_atlas`, and `shadow_collector` modules.

- Register the live view snapshot after camera and transform updates.
- Preserve separate raster, nonjittered, implicit, and culling projections.
- Extract live transformed bounds and source caster eligibility.
- Keep offscreen shadow visibility separate from camera-visible lists.
- Connect persistent queues, four cascades, atlas draws, and collector bindings.
- Treat valid empty fits as empty shadows. Reject malformed fits.
- Keep frame stamps consistent from candidate selection through collection.

Acceptance: a runnable imported scene, visible shadows, correct offscreen membership, and deferred GPU validation.
Verify alpha and dither casters, zero camera bias, target orientation, and resize.
Use the existing numeric and issued-draw checks without repeating unchanged cases.

## 3. Publish the local reflection probe

- Connect source camera selection and the eight-job capture schedule.
- Advance jobs only after the corresponding submission acknowledgement.
- Keep capture sampling and published sampling separate.
- Complete six scene faces and required mip jobs before publication.
- Publish only after deferred GPU validation succeeds.
- Preserve the previous valid probe during waits and failed captures.

Acceptance: face completeness, mip completeness, stable publication, retained allocation, and correct rebased position.

## 4. Produce motion vectors

Use the recovered camera and rigid-object producer contracts.

- Write the source single-sample RG16F motion target.
- Use current nonjittered velocity projection and current jittered reconstruction rays.
- Retain previous completed camera and model transforms.
- Apply source first-frame and ForceNoMotion rules.
- Preserve the proved zero motion depth bias.
- Transport retained matrices during rebasing.

Acceptance: static, camera, rigid-object, first-frame, resize, and rebase motion checks.
Keep producer checks separate from the already verified motion-blur effect.

## 5. Connect camera-local postprocessing

The effect worker interface is recorded in `task-4-effects-interface.md`.
The shared graph owner registers modules and owns camera-stack transitions.

Connect the source order:

1. Camera depth and opaque surfaces.
2. Ambient occlusion composition.
3. Ordered transparent passes.
4. TAA.
5. Motion blur.
6. Histogram and exposure.
7. Bloom.
8. Neutral grading and seeded dither.

Preserve source formats, sampling, half stores, and physical extents.
Reuse pipeline IDs during resize. Retain exposure history when only inputs change.
Advance jitter and histories from completed render tokens, not preparation updates.

Preserve the far-camera copyback and the main camera's second zero-weight callback copy.
Load the shared HDR target for the foreground camera.
Resolve HDR8 to HDR1 before the final source copy to the single-sample sRGB display.

Acceptance: connected depth and motion, frame order, color boundaries, final output, and deferred validation.
Report target memory, steady-frame allocations, and resize ownership.

## 6. Implement stained-glass GrabPass

Follow `stained-glass-grab-appendix.md` and the native pass-level transparent ordering.

- Insert source Grab records into the ordered stream.
- Apply the recovered pass-cache skip and invalidation rules.
- Copy current accumulated color before each executed unnamed grab.
- Include earlier glass draws in later captures.
- Prevent sampling the current writable color attachment.
- Resolve main HDR8 when the grab requires a single-sample copy.
- Use full physical extent, RGBA16F1, bilinear filtering, and clamp addressing.
- Apply exact projected UV, negative texel Y, depth weight, distortion, tint, and alpha replacement.
- Retain BackCull, LEqual, ZWrite1, and disabled hardware blending.
- Reuse allocations while clearing image validity at source render cleanup boundaries.

Acceptance: two visible glass materials, earlier-glass inclusion, cache reuse, resize, and no attachment feedback.
Keep unknown native exact-distance tie identities as an explicit limit.
Do not exclude all glass from every capture.

## 7. Verify lifecycle and finish

- Trace the original new-game camera and layer lifecycle before mapping restart resets.
- Suspend missing or minimized views without advancing render tokens.
- Preserve projection and model histories during rebasing.
- Verify resize, minimize, restart, resource failure, and pipeline failure.
- Keep Start gated until the connected scene and first complete capture are valid.
- Regenerate the final export and matching browser package.
- Run native, exporter, installed-source, formatting, and release checks.
- Run independent review after the connected implementation passes.
- Save matched target views and update the roadmap and verification record.

Fixtures must save full results through their local server before exiting.
This avoids slow browser-control result capture.
Run only critical new checks. Repeat passed checks when inputs change or a fault requires it.

Full 1:1 appearance requires matched native and target views with the same camera and rendering inputs.
Unmatched screenshots and isolated fixtures do not establish that result.

## Outside this plan

Dynamic mirrors, plate text, audio, survival, other vehicles, saves, multiplayer, and deployment remain excluded.
