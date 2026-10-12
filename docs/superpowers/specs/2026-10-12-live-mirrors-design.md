# Live mirrors

Date: 2026-10-12, Europe/Warsaw.
Status: The user approved this written spec on 2026-10-12. Implementation plan review is pending.
Branch: `codex/equipment`.
Base commit: `dccd516387dff48bf9388ad96ecd52eab3f29c00`.
Reference: installed Steam build `11140890`, Unity `2021.3.18f1`.

## Intent and sequence

The player must see live captures on the existing adjustable Lada mirrors.
Implement mirrors first. Complete their interaction tests and browser checks before starting license plate text.
Complete license plate text before starting portable flashlights.
Each later feature needs its own source recovery, design, and acceptance record.

Keep the existing mirror adjustment, carry, removal, and mounting controls.
Use the current renderer and source materials. Keep components separate and reuse existing dependencies.
Preserve the working garage and driving route throughout implementation.
Keep full deferred rendering completion in its existing plan.

Later plate work must preserve the Lada's empty starting slots until source evidence permits acquisition or spawning.
Later flashlight work must use source pickup, carry, lighting, controls, and power rules.
Add flashlight depletion only if the source requires it.

## Source evidence

Read the recovered `mirrorscript`, `mainscript.mirror`, `mainscript.MirrorsUpdate`, and `camscript` control flow.
Use the accepted typed records and archived source hashes to verify these findings before implementation.
Keep recovered C#, native records, generated assets, and captures outside Git.
Leave the installed Steam files unchanged.

The selected Lada has three complete mirror records:

| Mirror | Component | Camera | Surface renderer | Plane transform | Camera transform |
| --- | --- | --- | --- | --- | --- |
| Right side | `level1:66131` | `level1:40260` | `level1:43665` | `level1:28414` | `level1:39254` |
| Left side | `level1:67237` | `level1:40324` | `level1:40735` | `level1:38060` | `level1:31061` |
| Interior | `level1:70638` | `level1:40222` | `level1:44371` | `level1:32190` | `level1:26965` |

Chrome and dark-metal children also contain `mirrorscript` records with null links.
Exclude these empty records from capture creation. Require complete links for the three selected captures.

Recover defaults through `level1:64919`:

| Property | Source value |
| --- | --- |
| Target | `sharedassets1.assets:4495`, `defMirroRT` |
| Target extent | 256 by 256 pixels |
| Samples | 1 |
| Target storage | Linear; color format 8; depth/stencil format 92 |
| Filtering and addressing | Bilinear; clamp |
| Target mip sampling | Disabled |
| Dynamic target scale | Disabled |
| Live material | `sharedassets1.assets:624`, `defMirrorM` |
| Disabled material | Linked through `noMirrorMaterial` |
| Near plane | 0.1 metres |
| Serialized far plane | 1500 metres |
| Capture mask | `73113395` |
| Update FOV range | 10 to 55 degrees |
| Distance range | 0 to 4 metres |
| Update count limit | Five nearest registered mirrors |
| Capture camera effects | Empty postprocessing list; null depth and heat-haze links |

Resolve format codes through Unity's installed format definitions and the target GPU capabilities.
Retain camera HDR, clear, occlusion, rendering-path, and MSAA fields as source evidence.
Map the target's single sample count without assuming the camera's MSAA flag creates a multisample target.
Recover mirror draw distance and enabled state through source settings initialization.

## Capture behavior

Keep the camera transform attached to its recovered mirror hierarchy.
Compute capture direction from the player view position and the adjusted world-space mirror plane.
Use the source reflected direction and up vector, including signed scale and the existing coordinate conversion.
Interpolate FOV with the recovered camera distance rule.
Select at most five nearest registered mirrors using retained storage.
Apply the source distance, facing, visibility, enabled-state, and pointer-lock conditions.

Preserve the source update order: adjust FOV and direction, issue capture, then assign the distance-scaled far plane.
Keep the prior far-plane value for the capture that precedes that assignment.
Trace settings and startup callbacks before defining the first capture's far plane.
Use the normal view's visibility for scheduling. Capture visibility must not cause another mirror update.

The active `MirrorsUpdate` path does not call the oblique `mainscript.mirror.MoveReflectionCam` helper.
Recover clipping from the executed camera path. Do not activate this unused helper.
The source mirror cameras set `updMirrors` to false and omit camera postprocessing.
Keep captures nonrecursive and keep their updates separate from the normal view callback.

Pause disables the source mirror display and capture through pointer-lock loss and the pause callback.
Restore the source enabled display and update rules on resume.
Track removal and mounting through existing node and body bindings, including removal of the containing door.
Keep loose and carried mirrors eligible under the same source capture rules.

## Modules and data

Add source recovery in `tools/reference/mirrors.py` and extend the current manifest contract in `src/reference.rs`.
Store source identities, node bindings, camera fields, target fields, material links, and update parameters.
Validate required links, finite values, positive target extents, masks, and the three selected capture owners.
Require a fresh export for this contract. Remove the obsolete omission path without an old-manifest reader.

Keep view selection and projection calculations in `src/mirrors.rs` for native interaction checks.
Keep camera entities, target ownership, material binding, and render scheduling in `src/presentation/mirrors.rs`.
Resolve node, mechanism, body, and material handles during loading.
Reuse existing source material and lighting paths for captured surfaces and scene geometry.
Bind the capture to both source albedo and emission inputs with the recovered material factors.
Avoid an unlit mirror material that discards those source factors.

## Resource ownership and feedback

Allocate one retained capture target and live material per selected mirror instance during binding.
Retain both across adjustment, movement, removal, carrying, mounting, pause, and window resize.
Keep the source 256 by 256 extent during window resize.
Release mirror cameras, targets, and materials on instance destruction and game restart.
Use retained candidate and capture storage. Avoid per-frame allocations, hierarchy scans, and pipeline creation.

The selected mirror screens use source layer 0, which their capture masks include.
Apply the source mask and add an explicit guard against sampling the writable capture attachment.
Use capture-specific bindings with the source disabled material for the screen that owns the current writable target.
Keep the remaining scene and other completed mirror captures available to that capture.
Do not remove all mirrors or unrelated glass from capture visibility.
Ensure no material texture bank binds the writable target, even for a surface outside the current frustum.
Validate submission order before exposing capture contents to the normal view.
Keep target failure explicit. Do not show an invalid capture as a successful live mirror.

## Deferred rendering boundary

Coordinate against [the rendering completion plan](../plans/2026-10-11-rendering-completion.md).
The equipment branch owns mirror source records, capture cameras, capture targets, and mirror material bindings.
The rendering work owns shared camera snapshots, graph registration, shadows, probes, motion, and postprocessing.
Keep one integration owner for changes to `src/app.rs` and shared render interfaces.
Do not import unfinished rendering modules to deliver mirrors.

Define mirror capture as an explicit view role with its source projection, mask, target, and effect policy.
Synchronize camera and mirror transforms after physics, mechanism updates, and floating-origin transport.
Prepare the capture view before its visibility and render work. Issue it before the consuming normal view.
Use capture-specific view data for sky, lighting, and view-dependent materials.
Remove main-camera singleton assumptions where the capture requires another view.
Keep exposure, jitter, motion history, and normal-view completion tokens separate from mirror captures.
Document these interfaces for later deferred rendering integration without adding a compatibility layer.

## Critical checks

Use critical interaction tests and visible browser checks. Avoid tests that repeat implementation details.

| Check | Required result |
| --- | --- |
| Source export | Three complete Lada captures; verified identities, settings, targets, masks, and material links |
| Movement and adjustment | Player movement, car movement, and mirror rotation change the view in the correct direction |
| Removal and mounting | Capture follows the mirror and containing door; reinstallation preserves the adjusted pose |
| Carry and drop | The loose mirror capture uses its current pose without a stale body anchor |
| Floating origin | Capture direction and clipping stay correct through a rebase while mounted and loose |
| Pause and resume | Display and capture match source pause rules; resume restores capture updates |
| Resize | Capture orientation stays correct; source target extent and resource counts stay fixed |
| Restart | Old resources release; the new Lada has three capture cameras, targets, and live materials |
| Feedback | GPU validation reports no attachment sampling or sampler-layout conflict |
| Allocation | Repeated adjustment, mounting, and resize do not increase retained capture resources |
| Regression | Start, preparation, seating, and driving remain usable with working lights and materials |

Save visible production browser captures and GPU diagnostics with the matching package and source identities.
Use real browser input for interaction checks where pointer lock permits it.
Label deterministic diagnostic scenarios and their limits where browser input cannot prove an interaction.
Do not claim source appearance parity from unmatched screenshots or isolated fixtures.
Require matched source and target views before claiming visual parity.

## Review and completion

Review this written spec before writing the implementation plan.
Review the plan and select its execution method before product implementation.
Complete mirrors only after the required source, interaction, lifecycle, and browser checks pass.
Record unresolved limits with the exact failed check and evidence.
Start license plate recovery after mirror acceptance.
