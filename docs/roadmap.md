# Project roadmap

Updated: 2026-10-14.

## Current status

Master is at `4de050b`. Source-world implementation is at `44c45ad`.
We completed nine source-world implementation layers, their reviews, and the final correction review.
We are merging source-world with current master in `codex/source-world-merge`.
The merge is open. Master does not yet contain source-world.

### Changes in master

| Area | Implemented changes |
| --- | --- |
| Vehicles and interaction | Two imported cars, vehicle selection, carry, parts, mechanisms, and source recovery |
| Rendering | Shared lighting, shadows, probes, camera effects, transparent glass, live mirrors, and source plates |
| Audio | Source vehicle, interaction, weather, and Lada radio audio |
| Survival and weather | Existing consumables, survival state, sleep, weather, storm lighting, and particles |
| Portable flashlight | Source bindings, carried controls, storage restrictions, and shared light rendering |
| Save/load | Browser storage, fleet state, fluids, item state, and scene replacement |
| Performance | Reduced camera light clustering and depth/shadow material bindings |

The local door-control changes add left-button dragging and suppress view movement during a drag.
These changes remain uncommitted. We will preserve them during the merge.

### Source-world changes

| Layer | Implemented changes |
| --- | --- |
| Source inputs | Checked build `11140890` records, generation rules, assets, and source receipts |
| Roads | Sequential random state, checkpoints, source positions, slopes, curves, and segment joins |
| Geometry | Road meshes, collision geometry, imported garage, and continuous garage exit |
| Terrain | Source height layers, road/building alignment, biomes, and distant terrain |
| Placement | Separate structures, stations, props, and lamp layers with source selection rules |
| Rendering | Source materials, terrain, grass, impostors, lamps, shadows, and selected prototype ownership |
| Streaming | Bounded jobs, requests, geometry, collision windows, and floating origin |
| Revisits | Stable identities, absolute poses, compact change records, attachment state, and source replay |
| Final corrections | Loader headroom, double-precision local transforms, and lazy painting assets |

We retain each changed object as a compact record. Active geometry has separate limits.
Changed-object records have no fixed count limit. Source checkpoints and first-call history share a 16 MiB limit.
Active CPU/GPU limits remain 128 MiB each. Prototype limits remain 256 MiB CPU and 512 MiB GPU.
Generation and presentation share 32 MiB of pending storage.

See the [source-world design](superpowers/specs/2026-10-12-source-world-design.md),
[implementation plan](superpowers/plans/2026-10-12-source-world.md), and
[source research](research/2026-10-12-world-reference.md).

### Merge changes

We replaced station save identities with canonical generated identities, change records, and source spawn history.
We added save schema 2 and reject incompatible schema 1 saves. Browser database version remains separate.
We preserve source integer identities as decimal strings at the browser save boundary.
We apply the saved seed before generation and restore supported object state before physics participation.
We preserve imported fleet state, dormant absolute poses, recovery, and cross-owner attachments.
Vehicle selection updates future source spawn context. Stored first-call boundaries retain their original context.
We connect existing sausage and turd item definitions to generated state and source-owned geometry.

Native integration compiled. We are completing hidden renderer preparation before scene replacement.
This includes candidate failure handling, shared asset ownership, GPU readiness, and combined memory limits.
Generated meshes now require checked renderer bounds and flags. Final input publication remains pending.
We must retain master's latest depth/shadow change before promotion.

Remaining merge work:

1. Finish renderer preparation and compile the current native/WASM product.
2. Publish merged source inputs and resolve the remaining conflicts.
3. Include master `4de050b`, then build and review the final integration.
4. Update both branches and restore local changes.

### Verification limits

The source-world branch passed native and release WASM builds, packaging, and static review.
These results establish compilation and reviewed structure. They do not establish source runtime parity.
The user skipped tests and further runtime checks. Earlier rendering checks below describe earlier revisions.
Fixed source samples, collision seams, long drives, rebasing, revisits, and memory plateau remain unverified.
We have not measured retained objects or runtime memory for the merged world.
We removed obsolete generator paths from the integration. Acceptance of the replacement remains open.
We added no new storage engine, save migration, multiplayer, or deployment work.

## Earlier rendering completion

The user approved source rendering completion in an isolated worktree based on `dccd516`.
On 2026-10-12, the user authorized merging rendering commit `67798ec` into local master `a5b9778`.
Keep the accepted gameplay, road, library, menus, instruments, and day cycle.
The merged package passed verification on 2026-10-13. Retain the rendering worktree and its local evidence.
Do not push without a separate request.

## Accepted layers

| Layer | Evidence | Checkpoint |
| --- | --- | --- |
| Texture storage | Source storage audit, corrected exports, startup and pause checks | Included in shadow checkpoint |
| Shadows and sky | Three camera views, four main cascades, issued draws, GPU validation, upright image | `8182153` |
| Reflection probe | Six 64x64 faces, two mip jobs, validated publication, resize | `1b77a1a` |
| Motion producer | Camera and rigid motion fixture, matching submission tokens, resize | `538810e` |
| Camera effects | AO, TAA, blur, exposure, bloom, grading, dither, callbacks, resize and steady storage | `6148e66` |
| Stained glass | Both real door materials, two ordered overlap captures, probe views, resize and steady storage | `6f84766` |
| Lifecycle and streaming | Cold restart, readiness, terminal faults, probe distance, two actual rebases, fixed motion slots | Final branch commit |

The implementation uses exact 8-sample HDR rendering on the accepted hardware.
The WebGL backend uses attachment-only multisample renderbuffers and explicit resolves.
Shader consumers read the original D24 depth storage through a supported unfiltered float view.

## Final acceptance

Task 7 connects scene restart, view suspension, current-scene readiness, probe distance activation, and failure handling.
Healthy startup, pause, resize, restart, missing assets, invalid pipelines, and context loss passed native checks.
Restart resets scene histories and retains compatible storage. Terminal failure stops extraction, rendering, and history updates.
Whole-branch review found one motion streaming fault. The fixed-slot correction passed three critical tests.
Its scoped review and all five real-body native stages passed. Motion retained 780 GPU slots through streaming and both rebases.
The rendering branch passed 240 native tests before this merge.
The merged checks passed 318 native tests, 58 installed-source tests, 87 web tests, and 182 exporter tests.
Seven final export checks passed. The final default release package built and passed browser startup and Load.
Controlled lifecycle checks passed six stages. Both vehicle records survived a manual save and full page reload.
Both probes published. Three main views completed, and three mirror submissions passed GPU validation.
The merged motion pool retained 953 slots through the checked scene replacements.
See the [implementation plan](superpowers/plans/2026-10-11-rendering-completion.md) and [verification record](verification.md).

Local evidence remains in the rendering worktree under `.superpowers/sdd/2026-10-11-rendering-completion/` and `.cache/layer*-browser/`.
Installed source reports remain under `.superpowers/evidence/material-parity/reports/`.
The old `codex/material-parity` branch is not required for this implementation.

## Limits

- No matched native frame establishes identical pixels.
- Exact native blend rounding remains unverified.
- Equal-distance transparent ties use target identities. Native runtime identities remain unknown.
- Nine exported primitives lack authored tangents.
- Logical GPU byte counts exclude driver memory and overhead.
- Native operating-system minimize cannot be automated with the available browser tools.
- The in-app browser refused pointer capture. Continuous mouse control remains unverified for the final package.

The installed restart caller reloads the active scene with `LoadSceneMode.Single`.
That source proof resolves the former reload-versus-reset uncertainty.

Source vehicle, interaction, and Lada radio audio are implemented. Assembled audible acceptance remains incomplete.
The user stopped further checks. See the [audio verification report](research/2026-10-12-audio-verification.md).
Master now includes save/load, the source fleet, and live mirrors.
Survival, sleep, and weather are now implemented in master. Multiplayer and deployment remain outside scope.
