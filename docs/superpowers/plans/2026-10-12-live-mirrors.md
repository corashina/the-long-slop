# Live Mirrors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Track steps with checkboxes.

**Goal:** Show source-based live captures on the three existing adjustable Lada mirrors.

**Architecture:** Recover mirror records through the accepted library.
Compute view decisions in a native module and render captures with Bevy cameras.
Reuse source materials, retained targets, and existing interaction poses.

**Tech Stack:** Rust, Bevy 0.19.1, Rapier 0.36.0, WASM, WebGL2, Python, and the existing Docker toolchain.

**Spec:** [Approved mirror design](../specs/2026-10-12-live-mirrors-design.md).

Date: 2026-10-12, Europe/Warsaw.
Branch: `codex/equipment`; base implementation commit: `058bea6`.
Status: Written plan review and execution-method selection are pending.

## Global Constraints

- Reference: installed Steam build `11140890`, Unity `2021.3.18f1`.
- Implement mirrors first. Complete their interaction tests and browser checks before starting license plate text.
- Complete license plate text before starting portable flashlights.
- Preserve the working garage and driving route throughout implementation.
- Keep full deferred rendering completion in its existing plan.
- Keep recovered C#, native records, generated assets, and captures outside Git.
- Leave the installed Steam files unchanged.
- Require a fresh export for this contract. Remove the obsolete omission path without an old-manifest reader.
- Keep the source 256 by 256 extent during window resize.
- Use retained candidate and capture storage. Avoid per-frame allocations, hierarchy scans, and pipeline creation.
- Use critical interaction tests and visible browser checks. Avoid tests that repeat implementation details.
- Require matched source and target views before claiming visual parity.
- Apply AGENTS.md and the stop-slop skill to changed documentation and UI text.

## Review Focus

1. Invalid camera vectors must disable capture without a panic or nonfinite projection. Task 2 tests coincident positions.
2. Nearest-count selection must retain source order and cutoff semantics with invisible candidates. Task 2 tests six candidates.
3. Bound but undrawn textures must not alias the writable target. Task 3 checks complete material bindings.
4. Restart during capture preparation must reject stale instance work. Task 4 checks instance identity and GPU resource counts.
5. Camera-dependent sky must use the capture camera without advancing shared cycle state. Task 4 checks opposite view directions.

## Workspace and verification commands

Use `C:/Users/Corashina/.codex/worktrees/equipment/the-long-drive-wasm` for edits, commands, and output.
Read installed evidence from the primary checkout; do not edit its accepted library or source cache.
Use its existing `tools/reference/.venv/Scripts/python.exe` for Python commands from the equipment worktree.

Run native tests with `./scripts/check.ps1 -Test mirrors` or `./scripts/check.ps1 -Test mirror_reference`.
Run the full final check with `./scripts/check.ps1` and create the browser package with `./scripts/build.ps1`.
These scripts use a worktree-specific Cargo target path. Preserve that separation during direct Docker commands.
Run targeted checks after each changed input. Run the full check once at final acceptance.
Run the Python checks with:

```powershell
& 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/tools/reference/.venv/Scripts/python.exe' -m unittest tools.reference.tests.test_mirrors
```

Run installed mirror checks after export with:

```powershell
. ./scripts/cargo-target.ps1
$mirrorCargoTarget = Get-CargoTargetDirectory (Get-Location).Path
docker run --rm --mount "type=bind,source=$PWD,target=/workspace" `
  --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
  --mount 'type=volume,source=tld-wasm-target,target=/target' `
  --env "CARGO_TARGET_DIR=$mirrorCargoTarget" my-summer-slop-toolchain `
  cargo test --locked --test mirror_reference -- --include-ignored
```

For installed export, use the existing offline APIs from the equipment checkout in an ignored recovery script.
Load the primary source snapshot with `load_snapshot(primary / '.cache/reference-library')`.
Verify its library with `stage_library(None, primary / 'web/assets/reference', verify=True)`.
Call `derive_reference(snapshot, receipt, equipment / 'web/assets/reference')`.
Call `verify_reference(equipment / 'web/assets/reference', receipt)` after generation.
Supply the checked library receipt to these APIs; keep their existing hash and provenance checks.
Do not copy the multi-gigabyte library or add new exporter configuration for this setup.

## File responsibilities

| File | Responsibility |
| --- | --- |
| `tools/reference/mirrors.py` | Source mirror recovery, required links, settings, and evidence |
| `tools/reference/export.py`, `manifest.py`, `materials.py`, `gltf.py` | Export integration and replacement of live-capture omission |
| `src/reference/mirrors.rs`, `src/reference.rs` | Typed mirror contract and validation |
| `src/mirrors.rs`, `src/lib.rs` | Native view selection, projection decisions, and retained scheduling |
| `src/presentation/mirrors.rs`, `src/presentation/mod.rs` | Capture cameras, targets, material selection, lifecycle, and readiness |
| `src/presentation/materials.rs` | Source material construction and capture-specific prepared-material selection |
| `src/presentation/sky.rs`, `lighting.rs` | Per-view sky data and shared lighting integration |
| `src/app.rs` | Registration, view scheduling, input state, and restart integration |
| `tests/mirror_reference.rs`, `tests/mirrors.rs`, `tests/common/reference.json` | Critical source and interaction checks |
| `tools/reference/tests/test_mirrors.py` | Critical recovery and malformed-link checks |
| `docs/verification.md`, `tools/reference/README.md`, rendering completion plan | Acceptance evidence and camera integration contract |

### Task 1: Recover and validate the mirror contract

**Files:** Create `tools/reference/mirrors.py`, `src/reference/mirrors.rs`, `tests/mirror_reference.rs`, and `tools/reference/tests/test_mirrors.py`.
Modify export, manifest, material recovery, glTF integration, `src/reference.rs`, and the native fixture.

**Interfaces:**
- Produce `recover_mirrors(source, nodes, components, pointer_node, item_nodes, reports) -> dict` in the Python recovery module.
- Produce `ReferenceManifest.mirrors: MirrorRecord`; declare `MirrorRecord { rules: MirrorRulesReference, target: MirrorTargetReference, mirrors: Vec<MirrorReference> }`.
- `MirrorRulesReference` holds source identity, enabled state, draw distance, FOV bounds, distance bounds, and count limit.
- `MirrorTargetReference` holds source identity, extent, sample count, format codes, linear storage, filtering, addressing, and mip policy.
- `MirrorReference` holds source identity, owner, surface, plane, and camera nodes, plus camera source, fields, and material links.
- Use existing `ItemId`, `NodeId`, `SourceId`, and `MaterialBindingReference` types for those links.
- Expose `MirrorRecord::validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError>` and call it from manifest validation.

- [ ] **Step 1: Add the critical failing recovery and validation tests.**
  `test_selected_lada_mirrors` must recover components `66131`, `67237`, and `70638` with the spec's exact linked IDs.
  Assert three captures, extent `[256, 256]`, samples `1`, mask `73113395`, near `0.1`, and FOV bounds `[10, 55]`.
  Assert distance bounds `[0, 4]`, limit `5`, null-child exclusion, and the live albedo/emission material links.
  `test_required_link_failure` must fail a partly populated selected record and name its source identity and field.
  `mirror_contract_rejects_bad_inputs` must reject duplicate capture owners, missing nodes, zero extents, and nonfinite projection values.
- [ ] **Step 2: Run the new Python tests and `./scripts/check.ps1 -Test mirror_reference`; confirm the missing contract fails.**
- [ ] **Step 3: Implement recovery, typed records, and validation with the interfaces above.**
  Verify source container, record, and managed-code hashes through existing accepted receipts.
  Trace `mirrorscript.FStart`, `camscript.OnPreRender`, `MirrorsUpdate`, settings initialization, pause, and render-path changes.
  Resolve target format codes `8` and `92` through the installed Unity definitions; record the supported GPU mapping.
  Recover first-capture far-plane initialization, enabled state, disabled material, and mirror draw distance from executed source callbacks.
  Export the disabled material through existing material records so later capture bindings can use its checked handle.
  Replace the `live_mirror` omission evidence with explicit runtime bindings; preserve the source live material factors.
  Update the fixture to the current required contract. Remove old-contract acceptance.
- [ ] **Step 4: Run the targeted tests and derive the installed export through the offline APIs above.**
  Require hash checks, three complete bindings, native validation, and unchanged primary evidence files.
- [ ] **Step 5: Commit this contract and its passing checks.**

### Task 2: Compute source view decisions without frame allocations

**Files:** Create `src/mirrors.rs` and `tests/mirrors.rs`. Modify `src/lib.rs`.

**Interfaces:**
- Consume Task 1's `MirrorRecord`.
- Produce `MirrorState::new(reference: &MirrorRecord) -> Self` and `MirrorState::reset(&mut self, reference: &MirrorRecord)`.
- Produce `MirrorState::update(&mut self, rules: &MirrorRulesReference, frame: MirrorFrame, inputs: &[MirrorInput])`.
- Expose `MirrorState::outputs(&self) -> &[MirrorOutput]`.
- `MirrorFrame` holds player position, view-head position, source enabled state, and pointer-lock state.
- `MirrorInput` holds world plane position, source normal and up vectors, camera position, and normal-view visibility.
- `MirrorOutput` holds capture eligibility, `VehiclePose`, FOV degrees, capture near/far planes, and next far plane.
- Use array fields and existing Rapier math. Allocate input, candidate, and output storage once for the registered mirror count.

- [ ] **Step 1: Add critical failing projection and scheduling tests.**
  `source_projection_and_far_order`: use camera distance `2`, source FOV `[10, 55]`, and range `[0, 4]`; assert FOV `32.5`.
  With initial far `1500`, draw distance `50`, and player distance `2`, assert capture far `1500`, then next far `25`.
  On the following update, assert capture far `25`.
  `source_rank_cutoff_and_pause`: give six candidates; assert the sixth stays disabled even when a nearer candidate is invisible.
  Assert stable equal-distance ordering, hidden/back-facing/out-of-range rejection, and no capture after pointer-lock loss.
  `coincident_view_is_safe`: assert disabled output and finite retained projection data for a zero reflected vector.
  `translated_scene_preserves_capture`: translate player, plane, and camera by the same large vector; assert unchanged direction and projection.
- [ ] **Step 2: Run `./scripts/check.ps1 -Test mirrors`; confirm the missing module fails.**
- [ ] **Step 3: Implement source reflection, FOV, distance, facing, rank cutoff, and prior-far-plane behavior.**
  Use stable insertion into retained candidate storage. Preserve prior order for equal distances as the source stable sort does.
  Apply the existing Unity-to-runtime coordinate conversion once. Keep the unused oblique helper out of the active path.
  Preserve signed hierarchy scale when forming the source plane vectors.
- [ ] **Step 4: Run the native tests; confirm finite output, source order, and unchanged retained storage capacity.**
- [ ] **Step 5: Commit the native view calculation.**

### Task 3: Render working captures with safe source materials

**Files:** Create `src/presentation/mirrors.rs`. Modify presentation registration, materials, and `src/app.rs`.
Add the critical GPU scenario under ignored `.cache/equipment-checks/`; keep it outside the product UI.

**Interfaces:**
- Consume the checked `MirrorRecord`, native outputs, `ReferenceVisuals.nodes`, and existing `SourceMaterials`.
- Produce `SourceMirrorPlugin`, `SourceMirrors`, and `MirrorViewRole::{Normal, Capture(usize)}`.
- Expose `bind_mirrors(world: &mut World)` and `sync_mirror_views(world: &mut World)` in the presentation module.
- `SourceMirrors` retains three camera entities, three target handles, live/disabled material handles, cached node bindings, and native storage.
- Expose `make_mirror_material(record: &MaterialRecord, source: &SourceMaterials, target: Handle<Image>, reflected: bool, mask: u32) -> SourceMaterial`.
- Expose `mirror_material_for_view(role: MirrorViewRole, owner: usize, live: AssetId<SourceMaterial>, disabled: AssetId<SourceMaterial>) -> AssetId<SourceMaterial>`.
- Select the disabled material for the matching capture owner; select live material for the normal view and other captures.

- [ ] **Step 1: Add `capture_bindings_never_read_output` to the ignored GPU scenario.**
  Draw the mirror that owns the writable target and place a second mirror in that capture's view.
  Inspect complete prepared bindings; assert the owning target appears in no sampled bank of that capture.
  Assert the other mirror's completed target remains available. Require zero GPU attachment and sampler validation errors.
- [ ] **Step 2: Run the current-renderer scenario; confirm missing captures or unsafe bindings fail.**
- [ ] **Step 3: Bind source nodes once and create the retained cameras, targets, and materials.**
  Use the mapped target format, single sample count, bilinear clamp sampler, and fixed `[256, 256]` extent.
  Use source camera clear/HDR policy and exact source mask. Reuse the existing source mesh and lighting pipelines.
  Route `_MainTex` and `_EmissionMap` to the same retained bank and sampler; preserve UV transforms and material factors.
  Add capture-specific prepared-material substitution before queueing mirror surfaces in each view.
  Use the pinned Bevy material queue implementation for that substitution; retain its pipeline and bind-group ownership.
  Keep substitution in the render world. Avoid changing shared main-world material handles between camera renders.
  Include the source disabled surface for the capture owner; preserve other mirrors and unrelated glass.
  Warm capture pipeline variants during loading, including the inactive surface variant.
- [ ] **Step 4: Register camera order and update scheduling to consume the native outputs.**
  Read bound plane and camera nodes after imported transforms propagate, then synchronize capture Transform and GlobalTransform together.
  Compute normal-view visibility before capture selection. Keep capture views out of recursive mirror scheduling.
  Render capture cameras before the normal camera; honor source nearest order between captures.
  Retain the current target while inactive. Select the disabled display material under source pause/visibility rules.
  Publish readiness after capture pipelines and submitted target contents pass GPU validation.
  Require captures on the first eligible playing view; hidden or paused mirrors must not deadlock Start.
- [ ] **Step 5: Run `./scripts/build.ps1` and the GPU scenario in a visible browser.**
  Require three usable capture cameras, visible changing mirror contents, correct orientation, and passing complete-binding checks.
- [ ] **Step 6: Commit the working capture path.**

### Task 4: Complete interaction, sky, and resource lifecycle

**Files:** Modify `src/presentation/mirrors.rs`, `src/presentation/sky.rs`, `src/presentation/lighting.rs`, and `src/app.rs`.
Extend `tests/mirrors.rs` and the ignored GPU scenario.

**Interfaces:**
- Consume Task 3's retained cameras, capture roles, and source material selection.
- Expose `reset_mirrors(world: &mut World)`; release cameras and owned target/material assets before binding the restarted instances.
- Store a monotonically increasing instance ID in `SourceMirrors`; stamp capture work with it and reject stale work.
- Keep shared sky time/color state in the existing lighting resource; retain camera-dependent flare data per capture view.
- Keep the normal view's exposure, jitter, motion history, and completion state independent of mirror captures.

- [ ] **Step 1: Add `mirror_follows_part_transactions` and `mirror_rebase_keeps_projection` to the native tests.**
  Adjust a real fixture mirror, remove its containing door, detach the mirror, carry/drop it, and install it again.
  Feed current bound poses into the native calculation; assert correct direction and retained adjustment after each transaction.
  Rebase mounted and loose states; assert no stale camera anchor or changed relative projection.
  Reuse the existing part/interaction test helpers; do not duplicate their transaction implementation.
- [ ] **Step 2: Add critical GPU lifecycle assertions.**
  `capture_sky_uses_own_view`: use opposite normal/capture directions; assert camera-correct flares and one shared cycle update.
  `restart_discards_pending_capture`: restart between preparation and submission; assert old instance work cannot publish readiness.
  `resources_stay_bounded`: repeat resize, adjustment, mounting, pause/resume, and restart; assert three current capture targets and cameras.
  Count the fixed source material variants, and assert repeated actions do not add variants or pipeline IDs.
  Record backend target memory and retained intermediate targets; assert stable counts through repeated actions.
- [ ] **Step 3: Run the targeted native checks and lifecycle scenario; confirm missing lifecycle behavior fails.**
- [ ] **Step 4: Implement reset, per-view sky data, and capture lifecycle with the interfaces above.**
  Integrate reset with `GarageRuntime::restart` and imported-instance reset in `src/app.rs`.
  Share sky color/time and world lighting; compute flare projection and occlusion using each capture's camera and mask.
  Suspend minimized, missing, or paused views without creating resources or advancing capture completion.
  Restore source pause display and capture eligibility on resume. Report target or pipeline failure through existing readiness errors.
- [ ] **Step 5: Run the interaction and GPU lifecycle checks, then perform visible movement and adjustment checks.**
  Require passing part transactions, rebase checks, bounded resources, and camera-correct sky without normal-view regression.
- [ ] **Step 6: Commit the accepted mirror lifecycle.**

### Task 5: Verify the complete mirror feature and record integration

**Files:** Modify `docs/verification.md`, `tools/reference/README.md`, and `docs/superpowers/plans/2026-10-11-rendering-completion.md`.
Keep test scenarios, captures, packages, and logs ignored.

**Interfaces:**
- Consume the complete mirror path from Tasks 1 through 4.
- Record the `MirrorViewRole` contract, transform timing, capture order, effect policy, target ownership, and reset boundary.
- Add a camera-integration section to the rendering plan; keep its deferred effects outside equipment implementation.

- [ ] **Step 1: Regenerate and verify the installed export, then run `./scripts/check.ps1` and `./scripts/build.ps1`.**
  Require Python recovery checks, installed mirror hashes, native interaction checks, formatting, web checks, and release WASM compilation.
- [ ] **Step 2: Run the matching production package in a visible browser with an isolated server port.**
  Launch `scripts/serve.mjs` from the equipment checkout; select an unused `PORT` and retain the server process identity.
  Use trusted browser input to start, walk, adjust mirrors, remove/mount mirrors, remove/mount their doors, and drive.
  Check carry/drop, pause/resume, restart, window resize, and mounted/loose floating-origin scenarios.
  Use asymmetric scene markers to verify texture orientation and source clipping.
  Record diagnostic rebase/resource scenarios apart from trusted interaction evidence where browser controls impose a limit.
  Require zero GPU validation errors and stable resource counts. Save full diagnostics, not error-level console entries alone.
- [ ] **Step 3: Write the acceptance record and rendering integration section.**
  Link the matching package, source IDs, captures, commands, resource counts, and any exact unproved interaction limits.
  Replace the live-mirror omission statement in the exporter README.
  Keep source appearance parity unclaimed until matched native and target views support it.
- [ ] **Step 4: Review the whole branch after its required checks pass.**
  Use the execution method selected by the user. Fix findings and rerun checks for changed inputs.
  Check source control, lifecycle, allocation, feedback, signed transforms, and normal-camera regressions against the approved spec.
- [ ] **Step 5: Commit the acceptance record and report mirror completion.**
  Advance to license plate source recovery after mirror acceptance. Keep its empty starting slots intact during that recovery.
