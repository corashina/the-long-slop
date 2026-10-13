# Deferred rendering completion plan

Date: 2026-10-11.
Status: Deferred at the user's request. Local master integrates the stable materials with accepted lighting and gameplay.

Source: installed Steam build `11140890`, Unity `2021.3.18f1`.
Scope: the imported home, fixed Lada, preparation items, and their rendering.
Keep gameplay, physics, controls, and the existing road unchanged.

## Resume safely

1. Start an isolated worktree from current master.
2. Recover required modules from `codex/material-parity`, commit `1f18cc5ada986cc32082fd51bada22b1959d02d5`.
3. Preserve current master lighting, library, menus, instruments, gameplay, and seven-bank WebGL2 materials.
4. Resolve recovery interfaces against current master before compiling unfinished rendering code.
5. Read the recovery manifest, task reports, and acceptance map.
6. Verify retained input hashes before reusing evidence.
7. Keep one owner for app registration, graph wiring, and shared builds.
8. Use separate file ownership when independent effect work can run in parallel.

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
- Recheck texture sharing and resource counts; record texture counts from the merged runtime before comparing the corrected set.

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

## Mirror camera integration

The equipment branch owns the three selected Lada mirror cameras, targets, and materials.
`MirrorViewRole::Normal` and `MirrorViewRole::Capture(index)` are distinct roles.
Do not map captures to the deferred graph's numeric source view indices 0 through 8.
Do not advance its main, probe, or publication completion tokens for a mirror capture.

The equipment runtime reads mirror transforms after physics, part updates, and origin transport.
It updates camera poses after transform propagation and before camera frustum updates.
It checks normal view visibility first. It submits eligible captures by source rank before the normal view.
Each capture uses its source mask, clear color, finite projection, and a fixed 256 by 256 linear RGBA8 target.
Each camera owns one D24S8 capture attachment. Bevy can add HDR and depth intermediates.
The capture has no source postprocessing, depth effect, or heat haze.
Sky time and world lighting stay shared. Flare projection and occlusion use the capture camera.
Normal exposure, jitter, motion history, and completion tokens stay separate.

The owner screen uses its disabled source material while that camera writes its target.
Other mirror screens keep their completed targets. The material audit checks every sampled bank.
Reset advances the mirror instance ID and releases old cameras, targets, materials, and depth storage.
Queued preparation and completion reject work from an old ID.
The deferred graph must preserve these roles and reset rules when it later integrates mirror views.
It must keep its shadow, probe, motion, postprocessing, and GrabPass work inside this plan.

Mirror loading now prepares current source layouts, material choices, owner substitutions, and normal/capture view keys before readiness.
Normal frames select cached IDs through three fixed view slots. Pipeline errors still reach readiness failure after startup.
Current pose validity stays separate from eligibility. Loading and started restarts retain source initial far 1500.
The final mirror lifecycle receipt holds 46 capture variants, 109 total pipeline IDs, and 56 GL programs from first readiness.
Whole-branch mirror review remains pending. Trusted traversal, sustained powered travel, and matched source appearance remain unproved.

## Outside this plan

Audio, survival, other vehicles, saves, multiplayer, and deployment remain excluded.
The separate equipment work adds source plate text through its forward material path.
It shares world lighting and the three existing mirror captures.
It does not complete the deferred TMP shader path.
Restart waits for current plate and mirror readiness before input or physics resumes.
See `docs/verification.md` for the plate evidence and limits.

## Portable flashlight ownership

The equipment branch binds both imported flashlight roots to their item poses.
The older source object starts in the garage. The newer scene object stays inactive in gameplay.
The existing source light path handles both spot beams, shadows, and source cookies.
Binding 103 uses one three-layer cookie array across normal, mirror, and plate materials.
The renderer retains 34 light slots, 100 shadow slots, and the 4000 pixel atlas.
Mirror ownership remains three cameras and three fixed 256 by 256 targets.

Restart must clear flashlight control state and rebuild item ownership before current readiness.
The loading gate includes five source storage frames and current mirror and plate preparation.
The deferred graph must preserve these ownership and readiness rules when it later integrates mirror views.
See `docs/verification.md` for the current lifecycle checks and appearance limits.
