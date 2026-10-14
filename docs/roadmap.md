# Project roadmap

Updated: 2026-10-14.

## Current status

Master retains the accepted game features and local door-control changes.
We keep the complete source-world integration on `codex/world-integration-deferred`, based on `05bdc507`.
We completed nine source-world implementation layers and two integration merges in that separate branch.
The user deferred integration into master. Master does not contain the source generator or Bevy ownership patch.
Source-world changes shared startup, collision, save identities, and renderer paths. It requires the later integration work.

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
| Roadside stations | Source fuel station models, static collision, and ground alignment on the current procedural road |

We added left-button door dragging and suppress view movement during a drag.
The station models come from the concurrent remote master update. The complete source road generator remains separate.

### Deferred source-world changes

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

The separate branch contains the source-world design, implementation plan, source research, and checked input provenance.
The new worktree also retains exported runtime files and implementation reports.

### Deferred integration changes

We replaced station save identities with canonical generated identities, change records, and source spawn history.
We added save schema 2 and reject incompatible schema 1 saves. Browser database version remains separate.
We preserve source integer identities as decimal strings at the browser save boundary.
We apply the saved seed before generation and restore supported object state before physics participation.
We preserve imported fleet state, dormant absolute poses, recovery, and cross-owner attachments.
Vehicle selection updates future source spawn context. Stored first-call boundaries retain their original context.
We connect existing sausage and turd item definitions to generated state and source-owned geometry.

We completed hidden renderer preparation before scene replacement.
We preserve active scene owners until candidate uploads, pipelines, collision, and restored bindings are ready.
We added scoped GPU errors, completion receipts, separate mesh buffers, and combined memory charges.
We patched the existing Bevy 0.19.1 pipeline cache to own candidate preparation, cancellation, and retries.
We retain the original crate licenses and source provenance. The patch changes one upstream file.
Generated meshes require checked renderer bounds and flags. Selected ShadowsOnly meshes skip visible and motion passes.
We published 365 asset entries and recorded hashes for 7,890 runtime files, including the WASM package.
We retained master's latest depth/shadow bindings and connected candidate caster ownership.
The final native/WASM builds, packaging, 22 shader variants, JavaScript syntax, and scoped whitespace checks passed.
We preserved existing WASM warnings and unchanged upstream whitespace.

Remaining deferred work:

1. Resolve the reported hidden grass write and generated attachment owner restore issues.
2. Complete the interrupted integration review and repeat builds after required fixes.
3. Perform runtime acceptance when authorized. Merge into master requires a later user instruction.

### Verification limits

The source-world branch passed native and release WASM builds, packaging, and static review.
These results establish compilation and reviewed structure. They do not establish source runtime parity.
The user skipped tests and further runtime checks. Earlier rendering checks below describe earlier revisions.
Fixed source samples, collision seams, long drives, rebasing, revisits, and memory plateau remain unverified.
We have not measured retained objects or runtime memory for the merged world.
We removed obsolete generator paths from the integration. Acceptance of the replacement remains open.
We added no new storage engine, save migration, multiplayer, or deployment work.

## Full-game feature gaps

Reference: installed Steam build `11140890`. This backlog records gaps beyond the implemented layers above.
We keep source-world integration separate. Its roads, terrain, biomes, and placement need acceptance before a later merge.
Missing means no gameplay implementation. Partial means the current implementation covers selected behavior or content.
The backlog does not expand approved implementation scope. Each new feature needs a source-based specification.

| Feature | Status | Remaining work |
| --- | --- | --- |
| Source world generation | Deferred branch | Resolve integration findings; verify source samples and runtime behavior before merging. |
| Buildings and exploration | Deferred branch | Check source buildings, interiors, landmarks, placement, content coverage, and collision before merging. |
| World population | Partial | Integrate source loot placement; add excluded abandoned vehicles, items, and encounters. |
| Full vehicle range | Partial | Add vehicles beyond the Lada and Skoda, including diesel, two-stroke, automatic, bus, and bike mechanics. |
| Driving physics | Partial | Add manual gears and clutch; match throttle ramps, tire friction, bare-rim behavior, suspension, and combined braking. |
| Inventory and clothing | Missing | Add inventory slots, equipped items, wearables, and player customization. |
| Cleaning and painting | Missing | Add brushes, sponges, spray paint, and source condition changes. |
| Cargo and towing | Partial | Add general cargo locking, attachment objects, towing hooks, and trailers. Current storage locking covers flashlights. |
| Full item range | Partial | Add remaining source food, tools, containers, and their actions. |
| Animals and enemies | Missing | Add rabbits, human enemies, movement, attacks, and spawn rules. |
| Weapons | Missing | Add firearms, ammunition, reload actions, melee weapons, and projectiles. |
| Damage and destruction | Partial | Add wounds, blood loss, combat damage, breakable objects, and death ragdolls. |
| Complete save state | Partial | Preserve needs, food progress, sleep, storms, toilet state, and radio state; add save slots and autosave. |
| Settings and game setup | Partial | Add seed entry, source game options, editable controls, graphics, audio, and player settings. |
| Custom radio music | Missing | Add user music playback through the existing audio owner. Source radio playback already exists. |
| Multiplayer | Missing | Add the reference build's multiplayer behavior and shared state. |
| Remaining visual effects | Partial | Add soft storm particles, fluid refraction, and source death effects. |

Generated identities and change records belong to the current merge. Complete save state requires the remaining gameplay owners too.
Source placement and imported geometry do not provide excluded item actions, vehicle mechanics, or enemy behavior.

Suggested order: finish world integration, complete saves, correct driving, extend items and cargo, expand the fleet, then add combat.
Add inventory with item use. Add damage with combat. Connect settings, custom music, and effects to their existing owners.
Keep each layer working before adding the next. Plan multiplayer after the local gameplay and save state are stable.

Evidence: [vehicle coverage](vehicles.md), [driving differences](research/2026-10-11-rea-driving-audit.md),
[save and survival limits](research/2026-10-12-survival-sleep-weather-completion.md), and
[flashlight storage scope](superpowers/specs/2026-10-13-portable-flashlights-design.md).
The selected source has no supported rain scheduler or active snow scheduler. Add neither without source evidence.

## Remaining game comparison

Implemented features still need comparison with matched running source views and inputs.
Check continuous walking, preparation, seat entry, sustained driving, ordinary aiming, and lifecycle behavior after the merge.
Check assembled audio on a listening device. Compare rendered frames, particles, and physics under matched source conditions.
Retain the world sample, seam, rebase, revisit, object-count, and memory checks listed above.
Existing build, review, and historical test results do not establish full-game parity.

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
