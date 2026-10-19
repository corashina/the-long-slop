# Single world camera and roadside reset implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce frame work through one world camera, fixed debris, and expiry of abandoned roadside edits.

**Architecture:** Keep the existing render pipeline and world stream. Replace the runtime camera stack with one world view. Apply decoration and edit-retention policies at publication, retirement, and save reconstruction boundaries.

**Tech Stack:** Rust, Bevy 0.19.1, Rapier 0.36.0, WebGL2, WebAssembly, and the existing Docker toolchain.

**Spec:** [Approved design](../specs/2026-10-18-single-camera-roadside-design.md).

## Global constraints

- Use `codex/single-camera-roadside` in the attached isolated worktree.
- Keep the main checkout and its unrelated edits intact. Leave this branch unmerged.
- Deliver three separate code commits. Keep each commit runnable.
- Use GPT-6.1 Sol with high reasoning for implementation and review.
- Use GPT-6 Luna for simple Git tasks. Never use GPT-6 Astra.
- Use subagents, as the user requested. Serialize tasks that share world-stream files.
- Do not add packages, runtime switches, compatibility paths, or unrelated features.
- Preserve source export records, generation checkpoints, spawn boundaries, RNG, and the save schema.
- Preserve imported garage state, both cars, possessions, and required attachment dependencies.
- Avoid allocations and repeated setup in per-frame update and render paths.
- Use the stop-slop skill for documentation. Keep instructions within 20 words.
- Run focused critical tests. Report runtime failures with current evidence.

## Review focus

1. Held or mounted loot survives source-owner retirement without a duplicate source spawn. Task 3 covers this.
2. A consumed item stays absent near its source, even after its runtime item retires. Task 3 covers this.
3. A saved debris item cannot leave save readiness pending after becoming fixed decoration. Task 2 covers this.
4. A failed or partial camera submission cannot advance readiness or temporal history. Task 1 covers this.
5. Record compaction cannot leave stale replay indices after origin rebasing. Task 3 covers this.

## Execution preparation

- [ ] Verify branch `codex/single-camera-roadside` and a clean worktree before product edits.
- [ ] Reuse frozen exported assets from the main checkout through a worktree-local asset junction if needed.
- [ ] Verify both junction paths first. Keep asset contents unchanged and browser packages local to the worktree.
- [ ] Read `scripts/check.ps1`, `scripts/build.ps1`, and `scripts/cargo-target.ps1` before running builds.
- [ ] Build the unchanged baseline with `./scripts/build.ps1`.
- [ ] Save its browser package, code hash, package hashes, and asset hashes under `.cache/benchmark/master-baseline/`.
- [ ] Capture garage and driving baseline receipts before changing code.
- [ ] Use the existing benchmark server and measurement helper. Keep instrumented counts separate from release measurements.

For a focused Rust test, reuse the built image and the existing target namespace:

```powershell
$taskRoot = (Get-Location).Path
. ./scripts/cargo-target.ps1
$taskTarget = Get-CargoTargetDirectory $taskRoot
docker run --rm --mount "type=bind,source=$taskRoot,target=/workspace" `
    --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
    --mount 'type=volume,source=tld-wasm-target,target=/target' `
    --env "CARGO_TARGET_DIR=$taskTarget" my-summer-slop-toolchain `
    sh -c 'cargo test --locked --lib fixed_decoration -- --include-ignored'
```

Change only the test filter for Task 3. Confirm that junction assets are visible inside Docker before source-export tests.
If Docker cannot follow the junction, use a worktree-local frozen asset copy. Keep the source assets intact.

The benchmark commands run from this worktree:

```powershell
$env:PORT = '4187'
$env:BENCH_VARIANT = 'baseline'
node scripts/serve-shadow-cache-benchmark.mjs
```

Serve each candidate with `BENCH_VARIANT` cleared and the same port after stopping the owned baseline server.
The existing `/__measure.mjs` endpoint exposes `measureShadowCache`.
Use `instrument: false` for frame-time receipts.
Use actual browser input for gameplay acceptance. Synthetic resume does not establish input acceptance.

Match viewport, DPR, assets, scene state, route, and build profile.
Serialize builds, gameplay sessions, and GPU measurements.
Keep previous and candidate packages for each commit's A/B/A comparison.
A failed baseline scenario remains open. Do not replace it with a readiness-only measurement.

### Task 1: Replace the runtime camera stack

**Files:**

- Modify `src/presentation/camera.rs`, `surface_views.rs`, `frame_graph.rs`, `render_targets.rs`, and `view_frame.rs`.
- Modify `src/presentation/source_effects.rs`, `save.rs`, `motion_draw.rs`, `shadow_collector.rs`, and `surface_graph.rs`.
- Modify index consumers in `src/app.rs` and other render modules found by symbol search.
- Modify `scripts/batching-probe.js` and `scripts/check-batching.mjs` where they select the main audit view.
- Exercise existing `tests/source_view_domain.rs`, `reflection_preparation.rs`, `world_frontiers.rs`, and `loading_dispatch.rs`.

**Interfaces:**

- Add `pub struct WorldCameraSettings { pub near: f32, pub far: f32, pub fov_degrees: f32, pub mask: u32 }`.
- Add `pub fn world_camera_settings(reference: &ReferenceManifest) -> WorldCameraSettings` in `presentation/camera.rs`.
- Return near `0.05`, far `15000.0`, mask `0x14DA8F33`, and the existing player reference FOV of 65 degrees.
- Keep the existing `spawn_cameras` and `camera_snapshot` signatures.
- Replace `sync_stack_cameras` with `sync_sky_camera` and update its app registration.
- Replace `STACK_VIEWS` with `WORLD_VIEWS = 1`. Derive `PROBE_START`, `MIRROR_START`, and `VIEW_COUNT` from it.
- Rename stack target storage and `resize_stack` to world target storage and `resize_world`.
- Leave no obsolete surface-index aliases or compatibility wrappers.

- [ ] **Step 1: Record the failing browser acceptance condition on the unchanged baseline.**

Inspect `source_render_audit()` and submission state through the existing browser audit.
The baseline must show three world surface submissions, violating the required one-view result.
Also record existing temporal-history and GPU validation state before edits.
Use browser evidence for WASM-only modules. Do not add static string tests for camera constants.

- [ ] **Step 2: Implement the shared world camera settings and camera publication.**

Spawn world index 0 with `DriveCamera`.
Keep sky, six probe faces, and optional mirrors as separate outputs.
Use shared settings in spawn, snapshots, required views, and save preparation.
Keep existing jittered raster and nonjittered motion math, culling selection, and shadow cascade projections.
Preserve LOD and audio listener behavior.

- [ ] **Step 3: Replace stack targets and graph sequencing.**

Allocate one world-view target set and one main effect chain.
Keep both multisample raster depth and sampled single-sample depth.
Remove obsolete callback scratch, copyback, secondary effect chains, stack masks, and final-index assumptions.
Present from index 0. Update texture accounting, submission acknowledgements, motion ownership, and readiness.
Require successful world submission before advancing temporal history.
Update consumers by role. Preserve unrelated three-mirror loops and pipeline variant numbers.

- [ ] **Step 4: Run critical checks and record actual one-view acceptance.**

Run `./scripts/check.ps1`, then `./scripts/build.ps1` sequentially.
Expected: formatting, native tests, web tests, and release WASM build succeed.
In the browser, assert one submitted world view, advancing history, and no obsolete stack allocations.
Check held items, glass, distant roads, resize, mirrors, chase view, sleep, and save reload.
Exercise a submission failure or pending pipeline and confirm readiness and history do not advance.

- [ ] **Step 5: Measure and commit the camera change.**

Capture garage and driving A/B/A receipts against the retained previous package.
Record unverified scenarios and current errors separately.
Commit only Task 1 files with `Replace stacked world views with one camera`.
Run a fresh task review before Task 2.

### Task 2: Publish the selected debris as fixed decoration

**Files:**

- Modify `src/world/instances.rs`, `src/world/stream.rs`, and `src/world/content_bindings.rs`.
- Add critical tests in `src/world/stream_tests.rs` using its existing runtime and replay fixtures.

**Interfaces:**

- Add `pub(crate) fn fixed_decoration_source(source: &SourceId) -> bool` in `instances.rs`.
- Add `pub(crate) fixed_decoration: bool` to `StructureInstance`.
- Change `StructureRecipe::publish` to `publish(&self, placement: Placement, origin: WorldOrigin, fixed_decoration: bool, physics: &mut PhysicsWorld) -> Result<StructureInstance, ReferenceError>`.
- Add `WorldStream::fixed_decoration(&self, placement: &Placement) -> bool` for instance-level possession exclusions.
- Add `fixed_decoration: bool` to `binding_reservation`. Preserve its `Result<(usize, usize), ReferenceError>` return type.
- Update all existing callers. Do not preserve the old signatures.

The source predicate matches only `sharedassets1.assets` IDs `84448`, `69558`, `66737`, and `79572`.
The instance policy excludes retained possessions and required attachment dependencies.
It includes ordinary nested placements without matching arbitrary embedded bodies by name.

- [ ] **Step 1: Add failing fixed-decoration tests.**

Add `fixed_decorations_preserve_collision_without_item_bindings`.
Cover all four sources, cached and scaled publication, body types, collider groups, and body indices.
Assert no pickup, part, attachment, or mechanism registration.
Include ordinary dynamic loot and a matching path ID from the wrong file as controls.

Add `fixed_decoration_replay_rebases_retires_and_republishes`.
Assert a nested decoration stays fixed through physics steps, rebasing, retirement, and republication.
Assert no itemless replay remains pending.

Add `fixed_decoration_possessions_restore_on_item_path`.
Cover held and attached objects, their owner dependencies, save reload, and an unrelated vehicle part.

- [ ] **Step 2: Run the focused tests before implementation.**

Use the existing Docker toolchain and worktree-specific target directory from `scripts/cargo-target.ps1`.
Run `cargo test --locked --lib fixed_decoration -- --include-ignored` inside that toolchain.
The `--include-ignored` option must execute the source-export tests as well as pure tests.
Expected before implementation: fixed-body or binding assertions fail.

- [ ] **Step 3: Implement fixed publication and binding exclusions.**

Keep collision recipes unchanged. Set selected instance bodies to `Fixed` before insertion.
Preserve source body indices, shapes, groups, transforms, and stream ownership.
Apply the policy to both cached and scaled publication paths.
Return `Ok((0, 0))` from binding reservation for fixed decoration.
Skip fixed instances in `sync_static_controls` and before item and mechanism registration in `sync_contents`.
Set itemless fixed content `restored = true` so replay retirement can complete.

Before save reconstruction, exclude ordinary mutable debris records selected by the new fixed-decoration policy.
Keep protected possession and dependency records on the item path.
Do not convert obsolete mutable decoration records into another save representation.
Assert that selected ordinary debris cannot block `saved_bindings_ready` after reload.
This applies the new authoritative decoration policy without a compatibility layer or migration.

- [ ] **Step 4: Verify fixed behavior and retained possessions.**

Run the new focused tests and relevant existing world-stream tests.
Expected: fixed bodies retain collision, omit interaction bindings, and retire without pending replay.
Build the release browser package.
Verify walking and both-car collisions, origin rebasing, stream return, and save reload.
Confirm dynamic body counts change only when these decorations are present.

- [ ] **Step 5: Measure and commit fixed decoration.**

Capture garage and driving A/B/A receipts against the Task 1 package.
Commit only Task 2 files with `Keep generated roadside debris fixed`.
Run a fresh task review before Task 3.

### Task 3: Expire abandoned roadside edits and preserve possessions

**Files:**

- Modify `src/world/changes.rs`, `src/world/content_bindings.rs`, `src/world/stream.rs`, and `src/world/content_lamps.rs`.
- Add critical tests in `src/world/stream_tests.rs`.
- Update `PRODUCT.md` and `docs/roadmap.md` to replace the uncapped-retention promise.

**Interfaces:**

- Expose `pub(crate) fn forget(&mut self, key: &ObjectKey) -> bool` on `ChangeRecords`.
- Add `pub(crate) fn retain(&mut self, keep: impl FnMut(&ObjectKey, &ObjectChange) -> bool) -> bool`.
- Add `pub(crate) fn roadside_change_expired(change: &ObjectChange, foci: [[f64; 3]; 2], radius_squared: f32) -> bool` in `changes.rs`.
- Add `WorldStream::invalidate_changed_selection(&mut self)` and `WorldStream::prune_unloaded_changes(&mut self, items: &ItemStore)`.
- Keep public capture, save, restore, and retirement entry points intact.
- Task 3 consumes the Task 2 instance policy. It must preserve protected debris possessions on their item path.

The expiry predicate requires a source caller anchor and distant source and destination positions.
Use horizontal absolute distances and the exported `ItemRemoveDist`, about 188.57 m.
For removal records, check only the source anchor.
Keep records with unresolved required reconstruction dependencies.
Keep imported records, live owner records, possessions, and required attachment dependencies outside abandoned-record deletion.

- [ ] **Step 1: Add failing retention and replay tests.**

Add `roadside_expiry_requires_both_sites_outside_both_foci`.
Assert that nearby source, nearby destination, player focus, and vehicle focus each retain an edit.
Assert expiry only when all four distance checks reach or exceed the removal boundary.
Use the existing horizontal-distance arithmetic and cover the exact boundary.

Add `roadside_consumed_tombstone_survives_until_source_departure`.
Assert that immediate item retirement does not respawn consumed loot nearby.

Add `roadside_possession_survives_owner_retirement_without_duplicate_spawn`.
Cover held, stored, mounted, and attached loot with dependencies and source suppression.
Cover dropping transported loot and leaving both sites.

Add `roadside_expired_edits_do_not_return_after_capture_or_save_reload`.
Assert absence after dirty capture, save capture, reload, and deterministic revisit.

Add `roadside_owner_controls_keep_state_until_owner_retirement`.
Cover fluid supply, control, and lamp state before and after the existing owner boundary.

Add `roadside_pruning_compacts_indices_without_stale_replay_access`.
Delete an indexed record before another selected record.
Assert valid metrics, replay selection, accounting, and revision after compaction and rebasing.

- [ ] **Step 2: Run the new tests before implementation.**

Run `cargo test --locked --lib roadside_ -- --include-ignored` through the existing Docker toolchain.
Confirm that both pure tests and source-export runtime tests execute.
Expected before implementation: distant edits remain retained or fail the new expiry assertions.
Do not treat ignored tests as executed.

- [ ] **Step 3: Implement record deletion and expiry.**

Reuse existing record deletion to maintain identity and region indices, retained bytes, and revision.
Clear replay indices, query indices, stale roots, and bounds when selection becomes invalid.
Delete expired active records only after successful retirement removes their runtime bindings.
Keep nearby consumed removal records until their source leaves both focus windows.
Delete retiring controls, supplies, and lamps by exact object identity.
Keep possessions and dependencies that share a retired caller.
Preserve the existing 5,000 m building retirement boundary.

Gate unloaded expiry sweeps on the existing one-metre structure refresh and successful retirement.
Do not add another history scan on each movement frame.
Reuse retained scratch buffers when needed.

- [ ] **Step 4: Apply the same policy to save capture and reconstruction.**

Capture live state and compute required attachment dependencies before filtering the saved copy.
Use the same expiry predicate and protection rules as runtime retirement.
Preserve pending restoration dependencies until reconstruction completes.
Keep generation history and save schema unchanged.
Update product and roadmap text to state nearby retention, protected possessions, and distant reset.

- [ ] **Step 5: Verify, measure, and commit retention.**

Run the focused tests, then `./scripts/check.ps1` and `./scripts/build.ps1` sequentially.
Expected: all relevant tests pass and the release browser package builds.
Exercise loot, fluids, lamps, both focus windows, save reload, origin rebasing, and source revisits.
Record changed-object counts and retained bytes before and after departure.
Capture garage and driving A/B/A receipts against the Task 2 package.
Commit only Task 3 files with `Forget abandoned roadside edits after retirement`.
Run a fresh task review, then one whole-branch review.

## Final acceptance and handoff

- [ ] Fix review findings through the assigned implementer and verify the fix diff.
- [ ] Run `git diff --check` and confirm a clean branch after all commits.
- [ ] Verify enabled New Game, active physics, actual input, submitted frames, driving, and save reload.
- [ ] Run a matched original-baseline/final-candidate/original-baseline comparison with frozen assets and profile.
- [ ] Record package hashes, FPS, frame-time percentiles, route, viewport, DPR, and current failures.
- [ ] Report verified behavior and unresolved browser scenarios separately.
- [ ] Provide exact worktree build and launch commands. Leave the branch unmerged.

No product code may change before the user reviews this written plan.
The user already chose subagent execution. Preserve that method after plan approval.
