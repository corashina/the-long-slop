# Single world camera and roadside reset

Date: 2026-10-18
Status: Written spec awaiting user review.
Baseline: `95fe026036f5b07014fe6f2d6bd0552b7fd6a740`.
Branch: `codex/single-camera-roadside`.

## Intent and scope

Reduce render and physics work during garage play and driving.
Limit retained roadside edits by resetting abandoned objects after departure.
The user approved these changes in chat and requested subagents.

Deliver three separate code commits on the isolated branch.
Keep each commit runnable. Leave the branch unmerged.
Preserve the main checkout and its unrelated edits.

These changes replace selected source behavior with the approved simpler behavior.
Do not add packages, runtime switches, compatibility paths, or unrelated features.

## One world camera

Replace the far, main, and foreground surface views with one world view.
Reuse the existing `SourceSurface` render pipeline.

The world view uses these values:

| Setting | Value |
| --- | --- |
| Surface index | 0 |
| Field of view | 65 degrees, from the existing reference |
| Near plane | 0.05 m |
| Far plane | 15,000 m |
| Visibility mask | `0x14DA8F33`, the union of the previous surface masks |
| Color format | `Rgba16Float` |
| Depth format | `Depth24PlusStencil8` |
| MSAA | 8 samples |

Keep `DriveCamera` on this view.
Walking, cockpit, chase, and sleep poses continue to use that camera.
Use the same camera parameters and base projection for rendering and save preparation.
Preserve jittered raster, nonjittered motion, culling selection, and shadow cascade projections.
Preserve existing LOD and audio listener behavior.
Adding source zoom or a new field-of-view control is outside scope.

Keep the separate sky view, reflection probe faces, and optional mirror captures.
Those views produce separate images.
Set the main surface count to one.
Derive probe and mirror indices from the existing view-count constants.
Remove old index holes and assumptions about three surface views.

Use one world-view depth set, shadow collector, motion producer, and main postprocessing chain.
Keep both 8-sample raster depth and single-sample sampled depth for existing render consumers.
Remove obsolete stack cameras, targets, copies, callbacks, and effect chains.
Readiness and submission checks require the single world view.
Keep existing GPU validation and failure reporting.
Do not mark rendering ready before the required submission succeeds.

Foreground objects now share scene depth and main postprocessing.
Hands and held items can intersect scene geometry.
Layer-specific distance limits disappear within the combined camera range.
The wider depth range can cause depth artifacts with D24.
Check these effects in the browser before acceptance.

The main code areas are:

- `src/presentation/frame_graph.rs` and `surface_views.rs`: view creation, indices, submission, and presentation.
- `src/presentation/render_targets.rs`: target allocation and resize.
- `src/presentation/view_frame.rs`: view snapshots, culling, and temporal state.
- `src/presentation/source_effects.rs`: the single main effect chain.
- `src/presentation/save.rs`: required views during save reconstruction.
- Motion, shadow, reflection, and mirror consumers of surface indices.

## Fixed roadside decorations

Rocks and checked signs already use bodyless fixed colliders.
Keep their collision path. There is no dynamic body cost to remove from those instances.

Treat these generated world decorations as fixed:

| Decoration | Source file | Prototype path ID |
| --- | --- | --- |
| Pallet, `Raklap` | `sharedassets1.assets` | 84448 |
| Long plank, `deszka hosszu` | `sharedassets1.assets` | 69558 |
| Plank, `deszka` | `sharedassets1.assets` | 66737 |
| Bin lid, `Kukafedel` | `sharedassets1.assets` | 79572 |

Match exact source IDs when publishing a generated decoration instance.
Include ordinary nested spawn placements.
Do not classify arbitrary embedded vehicle or building components by their names.
Keep imported objects and retained possessions on their existing item paths.

Preserve collision shapes, collision groups, source transforms, and stream ownership.
Use fixed rigid bodies for these four prototypes.
Keep source body indices valid for existing instance consumers.
Fixed bodies do not require dynamic integration. Collision detection still applies.

Skip pickup, push, attachment, part, and mechanism registration for fixed decorations.
This removes carry and mount use from decorative pallets.
It also removes the bin lid's Open/Close mechanism.
Do not let distance updates or save reconstruction turn fixed decorations into dynamic bodies.
Apply the same rule after streaming, origin rebasing, and save reload.

The main code areas are `src/world/instances.rs` and `src/world/content_bindings.rs`.
The separate box-debris assets are outside this fixed-decoration list.
Their active runtime publication has not been established.

## Roadside edits and possessions

Keep nearby changes to generated objects.
Keep imported garage changes and supported vehicle state.

For loose generated objects, use the existing `ItemRemoveDist` value of about 188.57 m.
Use the existing horizontal distance calculation in absolute world coordinates.
An abandoned edit expires when both its source site and current position are outside both focus windows.
The two focus positions are the player and the active vehicle.
Keep the edit if either focus remains near either position.

Use `ObjectChange.replay.pose` as the source-site anchor after unload.
That pose identifies the caller site, not the item's exact initial position.
Keep the generated identity and deterministic replay inputs needed to recreate the baseline.
Do not reseed generation or regenerate unrelated sites.

Preserve held, stored, equipped, mounted, and externally attached possessions.
Protect slots within possession assemblies. Internal parts can retire with an abandoned source owner.
Preserve their attachment dependencies and source-spawn suppression.
This prevents lost possessions and duplicate loot during a source-site revisit.
A dropped possession becomes an ordinary abandoned world object.
Do not delete records for a possession merely because its original building retires.

Consumed objects can retire while the player remains nearby.
Keep their removal records until their source site leaves both focus windows.
Removal records have no destination snapshot, so source-site distance decides their expiry.

Delete expired object records after successful retirement removes the runtime bindings.
This prevents dirty capture and save capture from recreating expired edits.
Delete records by object identity, not by every object sharing one source caller.

Static controls, supplies, and lamps expire when their unprotected owners retire.
Preserve their existing owner distances and retirement guards.
Buildings currently retire at the 5,000 m generation boundary.
Changing that boundary is outside scope.
Do not reset a still-active structure merely because it exceeds the loose-item distance.

Sweep unloaded abandoned records and removal records when focus windows change.
Do not scan the whole edit history on every frame.
Reuse existing selection state and buffers where practical.
Forget records through actual deletion. `ChangeRecords::remove` creates a retained removal record.
Update record accounting and revision after deletion.
Invalidate cached replay indices because record compaction changes their positions.

Apply the same retention policy to saved world changes.
Capture live possessions and required dependencies before excluding expired abandoned edits.
Save reload must preserve nearby edits and possessions without restoring expired history.
Recovery preserves retained state. New Game clears session edits through the existing reset path.

The main code areas are:

- `src/world/changes.rs`: deletion, indexing, accounting, and revision.
- `src/world/content_bindings.rs`: loose-object retirement and item protection.
- `src/world/stream.rs`: owner retirement, replay selection, save capture, and restore.
- `src/world/content_lamps.rs`: generated lamp records and owner lifetime.

Update `PRODUCT.md` and `docs/roadmap.md` during implementation.
Replace the promise of uncapped retained roadside edits with this retention policy.

## Critical verification

Use focused tests for behavior that can lose state or block rendering.
Do not add tests that repeat constants without exercising the relevant behavior.

Check these cases:

- Exactly one world surface view submits, with no obsolete stack allocations or completion bits.
- Garage hands, held items, glass, distant roads, chase view, sleep, mirrors, and resize render correctly.
- Fixed decorations collide with the player and both cars and have no dynamic or interactive bindings.
- Fixed decorations stay fixed after streaming, origin rebasing, and save reload.
- Move, consume, drain, or switch a roadside object. Leave, return, and check the expected baseline reset.
- Keep either focus nearby and confirm its edits remain.
- Carry, store, mount, and attach loot. Travel, save, reload, and revisit without loss or duplicate loot.
- Drop transported loot, leave its source and destination, and confirm the reset.
- Repeat retirement after origin rebasing. Check record counts, retained bytes, and replay index validity.

Run relevant native checks and a release WASM build.
Then verify enabled New Game, active physics, submitted frames, actual input, driving, and save reload in the browser.
Build success alone does not establish runtime acceptance.

Measure garage and driving frame times in matched release A/B/A runs.
Keep assets, route, viewport, DPR, and build settings fixed.
Record baseline and candidate hashes, FPS, and frame times.
Serialize builds and GPU measurements. Keep instrumented call counts separate.
Report each commit's result so the gain can be attributed.
Do not claim a frame-time gain from fewer views or fewer dynamic bodies alone.

Record any runtime failure observed during these checks.
Past shadow, pointer-lock, or asset failures require current reproduction before attribution.
Do not treat an unverified driving scenario as accepted.
