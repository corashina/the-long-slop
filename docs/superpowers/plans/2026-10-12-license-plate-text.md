# License Plate Text Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render source-generated text on the two Lada startup plates, with correct removal and mounting.

**Architecture:** Export a typed plate contract, source font, and SDF material inputs.
A native module owns strings and glyph layout. A presentation module owns shared GPU assets.
A separate attachment module connects the plates to existing carry, physics, and interaction systems.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, Rapier 0.36.0, serde, Python, WASM, WebGL2, and the existing Docker toolchain.

**Spec:** [Approved plate design](../specs/2026-10-12-license-plate-text-design.md).

**Status:** The user approved the spec and authorized this plan on 2026-10-12. Implementation is in progress.
**Execution:** Preserve the user's selected subagent-driven method after plan approval.
**Base:** Local `master`, `a623c23`. The mirror implementation is merged at `62393dc`.

## Global Constraints

- Reference: installed Steam build `11140890`, Unity `2021.3.18f1`.
- Restore only these source instances. Do not add spare plates, loot, purchase paths, or other vehicles.
- Complete plate verification before starting portable flashlights.
- Use the existing carry, interaction, physics, and lighting systems.
- Keep extracted font data, textures, shader blobs, and generated vehicle assets outside Git.
- Leave installed Steam files and the primary source library unchanged.
- Do not use a browser font, system font, or replacement atlas.
- Do not allocate font assets, materials, or geometry during frame rendering.
- Preserve the mirror capture feedback guard and bounded render targets.
- Keep free-surface gluing, trailers, multiplayer, save-format changes, and full deferred rendering completion outside this feature.
- Require a fresh export for the new contract. Do not add an old-manifest reader.
- Implement only critical tests and visible browser checks.
- Follow AGENTS.md and stop-slop when changing documentation or UI text.

## Review Focus

1. A broken startup link must reject export rather than create an unlinked plate. Task 1 checks this.
2. Reversed startup callback order must still produce matching labels. Task 2 checks both orders.
3. Invalid font bounds or missing generated glyphs must reject data before GPU use. Tasks 1 and 2 check this.
4. A failed removal or unavailable target must retain ownership and carry state. Task 4 checks both failures.
5. Restart during loading must reject stale text work and retain bounded resources. Task 5 checks this in the browser.

## Workspace and commands

Use `C:/Users/Corashina/.codex/worktrees/equipment/the-long-drive-wasm` for execution.
Use the existing isolated checkout. Preserve other branches and worktrees.
Resume `codex/equipment` from approved `master` before implementation, with a fast-forward merge.
Do not reset a dirty checkout. Inspect its status first.
Do not push or merge completed plate code without the user's integration instruction.

Use the primary checkout's Python environment from the equipment working directory:

```powershell
& 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/tools/reference/.venv/Scripts/python.exe' -m unittest tools.reference.tests.test_plates
```

Run native targets through `./scripts/check.ps1 -Test plate_reference`, `-Test plate_text`, and `-Test plate_interaction`.
Run `./scripts/check.ps1` and `./scripts/build.ps1` for final acceptance.
These scripts preserve worktree-specific Cargo targets.

Use an ignored recovery script for installed export.
Load the primary `.cache/reference-library` snapshot and its accepted library receipt.
Use `LibraryReader` and `LibrarySource` to inspect selected checked records.
Use `derive_reference(snapshot, receipt, equipment / 'web/assets/reference')` for staged generation.
Use `verify_reference(equipment / 'web/assets/reference', receipt)` after generation.
Reuse accepted provenance checks. Do not copy the complete library or replace its records.

Run installed native checks with:

```powershell
. ./scripts/cargo-target.ps1
$plateCargoTarget = Get-CargoTargetDirectory (Get-Location).Path
docker run --rm --mount "type=bind,source=$PWD,target=/workspace" `
  --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
  --mount 'type=volume,source=tld-wasm-target,target=/target' `
  --env "CARGO_TARGET_DIR=$plateCargoTarget" my-summer-slop-toolchain `
  cargo test --locked --test plate_reference -- --include-ignored
docker run --rm --mount "type=bind,source=$PWD,target=/workspace" `
  --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
  --mount 'type=volume,source=tld-wasm-target,target=/target' `
  --env "CARGO_TARGET_DIR=$plateCargoTarget" my-summer-slop-toolchain `
  cargo test --locked --test plate_interaction -- --include-ignored
```

Keep each task working before starting the next task.
Run only relevant checks after each task. Run the full check at final acceptance.

## File responsibilities

| File | Responsibility |
| --- | --- |
| `tools/reference/plates.py` | Checked startup, plate, font, material, and attachment recovery |
| `tools/reference/export.py`, `manifest.py`, `gltf.py` | Staged export, asset receipts, and selected TMP replacement |
| `src/reference/plates.rs`, `src/reference.rs` | Required typed contract and validation |
| `src/plates.rs`, `src/plates/layout.rs`, `src/lib.rs` | String state, startup draws, and source glyph layout |
| `src/presentation/plates.rs`, `web/assets/shaders/source_plate_text.wgsl` | Lit SDF material, mesh assets, and text lifecycle |
| `src/presentation/materials.rs`, `mirrors.rs`, `mod.rs`, `src/app.rs` | Shared texture helpers, pipeline loading, capture integration, and readiness |
| `src/interaction/attachments.rs` | Plate target ownership and physical attachment transactions |
| `src/interaction.rs`, `interaction/targets.rs`, `interaction/items.rs`, `interaction/labels.rs` | Existing controls, aim ownership, item state, and feedback |
| `src/starting_area.rs`, `src/presentation/reference.rs` | Startup state, owner movement, origin changes, and item visuals |
| `tests/plate_reference.rs`, `plate_text.rs`, `plate_interaction.rs`, `tests/common/reference.json` | Critical contracts, layout, and interaction checks |
| `tools/reference/tests/test_plates.py` | Source recovery and rejected source data |
| `docs/verification.md`, `tools/reference/README.md`, rendering completion plan | Acceptance evidence and integration requirements |

## Shared interfaces

Add required `ReferenceManifest.plates: PlateRecord` with `deny_unknown_fields`.
Do not make this field optional or supply a legacy default.

`PlateRecord` contains `plates: Vec<PlateReference>`, `targets: Vec<PlateTargetReference>`, `font: PlateFontReference`, and `material: PlateMaterialReference`.
Use existing `SourceId`, `ItemId`, `NodeId`, `ColliderId`, `Transform`, `MaterialState`, and `TextureRecord` types.
Add `AttachmentTargetId(u32)` through the existing ID convention.

`PlateReference` records script, pickup, save, text, font, and material source IDs.
It also records `item`, `node`, `text_node`, `same_as: Option<ItemId>`, `initial_text`, `initial_same`, and `startup_target`.
Its attachment fields retain source flags, collider IDs, removal offset, mass, and damping.

`PlateTargetReference` records `id`, `source`, `node`, `owner: ItemId`, `colliders`, `types`, and target pose.
Recover these values through source pointers. Do not bind through current runtime item numbers.

`PlateFontReference` retains source face metrics, glyph metrics, rectangles, character mapping, pair adjustments, atlas receipt, and padding.
`PlateLayoutReference` retains source TMP bounds, size, style, weight, alignment, sizing flags, spacing, color, and text transform.
Each `PlateReference` has `layout: PlateLayoutReference`.

`PlateMaterialReference` retains shader provenance, active keywords, `MaterialState`, source float/color properties, and named texture records.
Use existing checked image export and sampler decoding.
Keep this font material separate from unrelated nine-slot Standard material bindings.

### Task 1: Export and validate selected plate data

**Files:** Create `tools/reference/plates.py`, `src/reference/plates.rs`, `tests/plate_reference.rs`, and `tools/reference/tests/test_plates.py`.
Modify exporter, manifest, glTF integration, `src/reference.rs`, and the native fixture.

**Interfaces:**
- Consume: `LibrarySource`, manifest node/component maps, `pointer_node`, and existing checked asset export.
- Produce: `recover_plates(source, nodes, components, pointer_node, item_nodes, colliders, output) -> dict` in Python.
- Produce: `PlateRecord::validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError>`.
- Produce: the shared types above, checked image files, and dependency receipts.

- [x] **Step 1: Add critical recovery and validation tests.**

`test_lada_startup_plate_bindings` must assert these exact source values:

```python
self.assertEqual([p['source']['path_id'] for p in result['plates']], [78750, 73955])
self.assertEqual([p['startup_target'] for p in result['plates']], [66346, 66601])
self.assertEqual(result['font']['source']['path_id'], 1366)
self.assertEqual(result['font']['atlas']['source']['path_id'], 366)
self.assertEqual(result['material']['source']['path_id'], 18)
```

Use the existing installed-library test setup. Add `test_broken_startup_link_rejects_export` with a cleared target pointer.
Require a diagnostic naming the plate source ID and missing field.
Add `plate_contract_rejects_invalid_font_data` in Rust.
Mutate missing `A`, out-of-atlas glyph bounds, duplicate item ownership, and invalid `same_as` references.
Assert `manifest.validate().is_err()` for each mutation.

- [x] **Step 2: Run the new tests and record their expected initial failure.**

Run the Python command and `./scripts/check.ps1 -Test plate_reference`.
Expected: missing recovery function or required contract types.

- [x] **Step 3: Implement checked source recovery and contract validation.**

Recover the exact two scripts, their activation chain, pickup links, and attachment targets from the spec.
Recover `attachablescript.Start`, `Detach`, target eligibility, and pickup body settings.
Source `attachTargetscript.CanAttach` checks type matches and has no occupancy limit.
Recover `attachs` as multiple membership. Do not add a one-plate target limit.
Keep the player target path consistent with `onlyToPoint=false` and `ignorePoints=false` for these selected plates.
Export all generated glyphs, source pair adjustments, exact alignment definitions, and active shader data.
Require finite metrics, valid rectangles, unique plate owners, acyclic sharing, and complete startup links.

Keep the source text nodes, but remove their obsolete editor mesh binding.
Replace the selected TMP omission report with a runtime-glyph binding report.
Retain unrelated TMP omission rules.
Export font material assets independently of unsupported glTF Standard material conversion.
Preserve staged generation, dependency verification, and atomic promotion.
Update the required fixture contract and installed manifest together.

- [x] **Step 4: Run recovery, contract, and staging checks.**

Run Python plate checks and `tools.reference.tests.test_export_staging`.
Run `./scripts/check.ps1 -Test plate_reference`, fresh installed export, and installed plate reference checks.
Expected: two bound source plates; checked font assets; zero failures; previous accepted output retained after failed export.

- [x] **Step 5: Commit the contract and exporter changes.**

Commit message: `feat: recover source Lada plate contracts`.

### Task 2: Generate strings and source glyph geometry

**Files:** Create `src/plates.rs`, `src/plates/layout.rs`, and `tests/plate_text.rs`.
Modify `src/lib.rs` and `src/starting_area.rs`.

**Interfaces:**
- Consume: Task 1's `PlateRecord` and `PlateLayoutReference`.
- Produce: `PlateState::new(record: &PlateRecord, draw: &mut dyn FnMut(i32, i32) -> i32) -> Result<Self, ReferenceError>`.
- Produce: `PlateState::text(&self, item: ItemId) -> Option<&str>` and `PlateState::revision(&self, item: ItemId) -> Option<u64>`.
- Produce: `PlateState::set(&mut self, item: ItemId, text: &str) -> Result<(), ReferenceError>`.
- Produce: `layout_text(font: &PlateFontReference, layout: &PlateLayoutReference, material: &PlateMaterialReference, text: &str) -> Result<GlyphMesh, ReferenceError>`.
- Produce: `GlyphMesh { vertices: Vec<GlyphVertex>, indices: Vec<u32> }`.
- Produce: `GlyphVertex { position: [f32; 3], normal: [f32; 3], uv0: [f32; 2], uv1: [f32; 2], color: [f32; 4] }`.
Use quad vertex order bottom-left, bottom-right, top-right, top-left. Set index winding from the converted source normal.
GlyphMesh vertices are text-local, with local Z zero. Do not bake `layout.transform` into glyph vertices.

- [x] **Step 1: Add generation, callback-order, and layout tests.**

`plate_generation_uses_source_ranges` supplies lower and upper boundary draws.
Assert `AAA-000` and `ZZZ-999`; assert requested bounds are `(65,91)` and `(48,58)`.
`plate_start_order_preserves_shared_text` tests front-first and rear-first script activation.
Reverse the `record.plates` activation order while retaining item IDs and `same_as` links.
Assert both labels match after activation. Assert a script marked `same` does not draw again.

```rust
assert_eq!(state.text(front), state.text(rear));
assert_eq!(lower_boundary.text(rear), Some("AAA-000"));
assert_eq!(upper_boundary.text(rear), Some("ZZZ-999"));
```
`plate_layout_uses_source_metrics` checks bearings, baseline, advances, kerning, bounds, and signed bold scale in UV1.
Use a small font fixture with point size `10`, font size `0.8`, and glyph `A` advance `10`.
Set its bearing to `(1,8)`, width to `6`, height to `8`, and first `AA` advance adjustment to `-2`.
Set all other pair adjustments to zero; set character and glyph scales to one.
Use source bold style `0.75`, bold spacing `7`, zero normal spacing, and the source material parameters.
Use the source `orthographic=false` flag and TMP world scale factor `0.1`.
Recover `ShaderUtilities.GetPadding` and bold style expansion before positioning quads.
Source material padding is `4.85`; bold style expansion is `1.6875` in font units.
For `AA`, assert quad width `0.1526` and left-edge distance `0.0696`, within `0.00001`.
Keep the source alignment codes and validate their common translation independently.

```rust
assert!((mesh.vertices[1].position[0] - mesh.vertices[0].position[0] - 0.1526).abs() < 0.00001);
assert!((mesh.vertices[4].position[0] - mesh.vertices[0].position[0] - 0.0696).abs() < 0.00001);
```
Assert missing glyphs and nonfinite layout values return errors.

- [x] **Step 2: Run `./scripts/check.ps1 -Test plate_text` and confirm the initial failure.**

Expected: missing state and layout functions.

- [x] **Step 3: Implement native string state and glyph layout.**

Retain strings and revisions by existing item identity.
Preserve `Set`, `Same`, propagation, and conditional startup generation.
Reject invalid draw results before indexing glyph data.
Keep callback activation order explicit in the startup adapter and acceptance evidence.

Seed retained startup draw state from the existing game seed.
Reuse `world::hash` with a retained draw counter and unbiased integer range mapping.
Keep this adapter separate from the lamp's `System.Random` implementation.
Record that this adapter does not establish Unity global random-sequence parity.
Do not expose new random settings or regenerate strings during item transitions.

Own `PlateState` in `StartingArea` as `pub plates: PlateState`.
Use source TMP layout definitions, material-dependent padding, and bold spacing.
Retain `layout.transform` as binding data. The imported text node applies its converted transform once.
Read the checked TMP decompile and receipt named in Task 1 report.
Expand glyph UVs with the same recovered padding and style values as geometry.
Build glyph winding, normals, UV0, UV1, and color for the source SDF surface.
Do not allocate or rebuild glyph geometry during unchanged frame updates.

- [x] **Step 4: Run `./scripts/check.ps1 -Test plate_text` and confirm all critical assertions pass.**

Also run `./scripts/check.ps1 -Test plate_reference` after fixture changes.
Expected: both startup orders pass; all generated glyphs resolve; no nonfinite vertices.

- [x] **Step 5: Commit native generation and layout.**

Commit message: `feat: generate source plate labels and glyph meshes`.

### Task 3: Render mounted text with shared source assets

**Files:** Create `src/presentation/plates.rs` and `web/assets/shaders/source_plate_text.wgsl`.
Modify presentation registration, texture helpers, mirror warmup collection, and `src/app.rs`.

**Interfaces:**
- Consume: Task 2's `PlateState`, `layout_text`, and `GlyphMesh`; `ReferenceVisuals.nodes` from imported scenes.
- Produce: `PlateTextPlugin`, `bind_plate_text(world: &mut World)`, and `reset_plate_text(world: &mut World)`.
- Produce: `PlateTextStatus::ready(&self) -> bool` with loading, ready, or failed state and an instance epoch.
- Produce: `PlateTextMaterial`, a Bevy material using the source SDF shader and shared source texture handles.
- Produce: `plate_warmup_pairs(world: &World, pairs: &mut Vec<(AssetId<Mesh>, UntypedAssetId)>)`.
Use Bevy asset IDs in this signature, not reference asset IDs.

- [ ] **Step 1: Add critical material and mesh integration assertions.**

Extend `plate_text.rs` with `plate_text_uniforms_keep_source_surface_values`.
Extract pure uniform preparation into `src/plates/layout.rs` as `sdf_parameters(material: &PlateMaterialReference, layout: &PlateLayoutReference) -> SdfParameters`.
Define `SdfParameters` with `face_color: [f32;4]`, `outline_color: [f32;4]`, and these `f32` fields:
`face_dilate`, `outline_width`, `outline_softness`, `gradient_scale`, `perspective_filter`, `scale_ratio_a`, and `weight`.

```rust
assert_eq!(parameters.face_color, [16.0 / 255.0, 16.0 / 255.0, 16.0 / 255.0, 1.0]);
assert_eq!(parameters.face_dilate, 0.1);
assert_eq!(parameters.gradient_scale, 10.0);
assert_eq!(parameters.weight, 0.75);
```
Assert face RGB `16/255`, dilation `0.1`, outline width `0.1`, softness `0.2`, gradient scale `10`, and bold weight `0.75`.
Define an ignored browser probe for actual material sharing, pipeline readiness, and rendered text.
Keep this probe outside product code and Git.

- [ ] **Step 2: Run native parameter checks and the browser probe before rendering integration.**

Expected: missing parameter function; browser probe reports absent labels.

- [ ] **Step 3: Implement the lit SDF material and imported-node binding.**

Use the existing Bevy material API and current lighting uniforms.
Translate active source TMP shader behavior, including SDF sampling, bold scale, perspective, alpha, depth, and lit face/outline treatment.
Use the source white TMP vertex color and dark material face color.
Retain shadow behavior as supported by the current lighting path; record any missing source pass behavior.
Share one font atlas and one authored text material across both labels.
Reuse existing checked texture sampler preparation through a small shared helper if needed.

Bind generated text children to recovered text nodes and source render layers.
Keep text-local glyph geometry distinct from item transforms. Bind it at the recovered text node.
Its imported transform applies `layout.transform` once. Do not apply the text offset to vertices again.
Create meshes only at initial binding or a changed text revision.
Use identical vertex layouts for all generated strings.

Register the new material type with existing main-view loading and mirror capture preparation.
Add plate pairs to cold warmup before sealing the pipeline request set.
Use existing erased material capture dispatch; do not add a second capture renderer.
Gate game readiness on `PlateTextStatus`, source textures, and compiled main/capture variants.
Surface failures through the existing loading diagnostic.

- [ ] **Step 4: Verify a working mounted-text product.**

Run native plate checks, `./scripts/build.ps1`, and the visible browser probe.
Expected: two matching labels; correct front and rear reading direction; lit text; shared atlas and material.
Expected: no sampler error, capture feedback, unknown pipeline, or new render target.
Record a screenshot and code/build hashes before continuing.

- [ ] **Step 5: Commit the complete mounted-text rendering layer.**

Commit message: `feat: render source SDF license plate text`.

### Task 4: Integrate source plate attachment transactions

**Files:** Create `src/interaction/attachments.rs` and `tests/plate_interaction.rs`.
Modify interaction dispatch, aim targets, item ownership, labels, `StartingArea`, and imported item synchronization.

**Interfaces:**
- Consume: Tasks 1 and 2 contracts and state; existing `ItemStore`, `CarryState`, `PhysicsWorld`, and `PlayerState`.
- Produce: `PlateAttachments::new(reference: &ReferenceManifest) -> Result<Self, ReferenceError>`.
- Produce: `PlateAttachments::bind(&mut self, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<(), ReferenceError>`.
- Produce: `PlateAttachments::can_mount(&self, item: ItemId, target: AttachmentTargetId, items: &ItemStore) -> Result<(), InteractionBlock>`.
- Produce: `PlateAttachments::mount(&mut self, item: ItemId, target: AttachmentTargetId, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<(), ReferenceError>`.
- Produce: `PlateAttachments::remove(&mut self, item: ItemId, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<ItemId, ReferenceError>`.
- Produce: `PlateAttachments::sync(&mut self, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<(), ReferenceError>` and `rebase(&mut self, shift: [f32; 3])`.
- Own this state as `InteractionState.attachments`. Add `SlotTarget::Attachment(AttachmentTargetId)`.

- [ ] **Step 1: Add critical transaction tests with source-backed plate fixtures.**

`plate_remove_carry_drop_remount_keeps_identity_and_text` checks one item across the full sequence.
Assert the string and revision stay fixed, while pose, body, and target ownership change correctly.
`failed_plate_acquisition_keeps_attachment` forces carry rejection before removal.
Assert target membership, collider parent, body state, and text remain unchanged.
`invalid_plate_mount_keeps_carry` attempts a mount against `AttachmentTargetId(u32::MAX)`.
Require `InteractionBlock::Unavailable` before releasing carry.
Assert the same carry constraint survives and no collider ownership changes.

```rust
assert_eq!(interaction.held(), Some(plate));
assert_eq!(items.get(plate).unwrap().body, body_before);
assert_eq!(plate_state.text(plate), Some(text_before.as_str()));
assert_eq!(plate_state.revision(plate), Some(revision_before));
```
`plate_target_allows_multiple_source_members` mounts the removed rear plate on the occupied front target.
Assert both items remain separate target members, with unchanged strings and unique collider ownership.
`plate_follows_removed_bumper_and_rebase` removes the front bumper with its plate mounted.
Assert plate and text follow the bumper; plate-only removal uses the converted source offset.
Assert world pose continuity after origin rebasing.

- [ ] **Step 2: Run `./scripts/check.ps1 -Test plate_interaction` and confirm the initial failure.**

Expected: missing attachment state and dispatch.

- [ ] **Step 3: Implement plate attachment and existing control dispatch.**

Retain each source plate as one existing item. Do not create `PartReference` or `PartSlotReference` entries for plates.
Use `ItemLocation::Attached(owner)` for mounted ownership and retain target membership in `PlateAttachments`.
Recover collider ownership and target scan geometry from the contract.
Retain target membership for both selected items. Do not use a single occupied-slot field.
Prepare loose bodies once during binding, disabled while mounted.
Use source mass, damping, collider state, and locked target pose.

Preflight carry acquisition, target eligibility, and ownership before committing changes.
Preflight mount work before releasing the carry constraint.
Route existing removal and mount inputs through the source attachment rules.
Retain source reach, layers, pickup restrictions, and release-edge behavior.
Reuse existing feedback text where it fits. Do not add new controls.

Apply the source removal offset in plate coordinates before changing the parent.
Preserve physical velocity and pose when ownership changes.
Include plate colliders in containing-bumper transitions without duplicate collider ownership.
Update target poses and retained loose bodies during rebase.
Keep text state independent of attachment state.

- [ ] **Step 4: Run native interaction checks and visible carry/drop/remount checks.**

Run `plate_interaction`, existing `items`, `parts`, and `interaction` test targets.
Run installed plate interaction checks after fresh export.
Build and verify actual imported front and rear plate movement in the browser.
Expected: all critical transactions pass; labels keep their text and orientation; existing part behavior remains working.

- [ ] **Step 5: Commit attachment and interaction integration.**

Commit message: `feat: support source plate removal and mounting`.

### Task 5: Verify lifecycle, resources, and the assembled product

**Files:** Modify plate presentation lifecycle, `src/app.rs`, and related native tests if acceptance finds a defect.
Update `docs/verification.md`, `tools/reference/README.md`, and `docs/superpowers/plans/2026-10-11-rendering-completion.md`.
Keep browser harnesses, captures, generated assets, and source extracts outside Git.

**Interfaces:**
- Consume: all prior tasks and the existing game instance/readiness lifecycle.
- Produce: tested reset, pause, resize, restart, stale-instance rejection, and final acceptance evidence.
- Use `reset_plate_text(world)` before disposing old game-owned text assets.
- Preserve shared checked source assets according to existing asset ownership.

- [ ] **Step 1: Add the missing lifecycle assertions to the ignored browser harness.**

`plate_restart_during_loading_rejects_stale_epoch` restarts before text readiness.
Assert only the current instance can bind or report ready.
`plate_resources_stay_bounded` repeats detach, mount, resize, pause, resume, and restart for three cycles.
Compare post-readiness material, image, mesh, render-target, and pipeline counts at matching lifecycle states.
Require stable ownership counts and zero unknown pipeline keys after readiness.
Require exactly three mirror capture targets, unchanged from the merged mirror implementation.

- [ ] **Step 2: Run these checks against the current build and record any failure.**

Use the actual imported Lada and labelled browser diagnostics.
A passing check requires no lifecycle code change solely to manufacture a failing test.

- [ ] **Step 3: Complete lifecycle integration where the checks show a gap.**

Reject stale binding work with the current instance epoch.
Release replaced glyph meshes and per-game text entities.
Keep shared atlas and material reuse explicit.
Keep strings fixed during pause, resize, mount, and origin changes.
Run cold pipeline preparation during each game loading phase before readiness.
Do not specialize new pipelines during playing-state item transitions.

- [ ] **Step 4: Run full acceptance on one implementation revision.**

Run Python plate and export staging checks, all installed plate checks, `./scripts/check.ps1`, and `./scripts/build.ps1`.
Verify readable front/rear text, bumper removal, individual removal, carry, drop, remount, origin shift, resize, pause, and restart.
Check eligible mirror text and main-view lighting on that same build.
Record code, WASM, manifest, source-library, font, and atlas hashes.
Record visible screenshots, errors, resource counts, and pipeline counts.
Label scripted input, trusted-input limits, random-sequence differences, and unsupported source shader behavior.
Do not claim matched Unity appearance without matched source runtime views.

- [ ] **Step 5: Commit the acceptance record and any verified lifecycle fixes.**

Commit message: `test: verify source plate lifecycle and browser rendering`.
Mark completed checkboxes only after their evidence exists.
Complete task reviews and whole-branch review under the preserved execution method.
Report plate completion before starting flashlight source recovery.
