# G10 garage and car controls implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Complete source-faithful garage Lada controls and verify them through normal input.

**Architecture:** Extend existing input, mechanism, radio, and instrument modules. Export source fields into required reference records. Reuse material binding and warmup for the added visuals.

**Tech stack:** Rust 2024, Rapier 0.36.0, Bevy 0.19.1, Python 3.12, existing PowerShell checks, and existing browser audio.

**Spec:** [G10 garage and car controls design](../specs/2026-10-20-g10-garage-controls-design.md), approved 2026-10-20.

**Worktree:** `C:/Users/Corashina/.codex/worktrees/g10-garage-controls/the-long-drive-wasm`.

**Branch:** `codex/g10-garage-controls`. Planning base: spec commit `bbddf28`.

**Execution method:** Subagents, selected by the user. Plan review remains pending. Do not implement before that review.

## Global constraints

- Use the selected Lada, starting home, garage, yard, and nearby terrain.
- The behavior authority is Steam build `11140890` and its decompiled code.
- Keep existing drag rules, action priority, part ownership, and collider bindings.
- Use the existing radio playback, lighting, instrument, and physics systems.
- Do not add a shared control subsystem.
- Do not add defaults for fields missing from an old export.
- Regenerate the runtime package after schema changes.
- Do not allocate, search the manifest, or create body bindings per frame.
- G09 owns the direct seat-switching correction and player posture work.
- G10 owns the L-key eligibility correction for source secondary control seats.
- Keep the branch unmerged unless the user authorizes integration.
- Do not run full checks during this task unless separately requested.
- Use GPT-6.1 Sol with high reasoning for implementers and reviewers. Never use GPT-6 Astra.
- Apply AGENTS.md, short STE sentences, and the unslop skill to documentation and UI text.
- Add only critical tests. Add no packages, custom music, vehicles, control animations, or driving physics changes.

## Review focus

1. E and LMB arrive together. The handbrake changes once, and held-item action priority remains intact. Task 1.
2. One large drag crosses both endpoints. Preserve each configured event without repeated forces on later held frames. Task 2.
3. A part changes its root body. Apply its reaction to the current root and clear pending reactions on restore. Task 2.
4. The car moves, rotates, or scales. Radio needle placement remains correct in its current parent space, including power-off tuning. Task 4.
5. New material variants are pending, invalid, or restored. Readiness and load validation must include those variants. Tasks 3 through 5.

## Execution and shared files

Run the five tasks in order in this worktree.
Use one fresh implementer subagent per task and one fresh task reviewer after each task.
Do not run concurrent implementers. Tasks share schema, app, and material code.
Provide each subagent with this spec, its complete task text, and the preceding task interfaces.
Give the reviewer the task diff, commands, and actual results.
Review source compliance and code quality before the next task.
Finish with a whole-branch review.
Record deviations and blockers in the execution ledger required by the subagent-driven-development skill.

Each task ends with a usable feature or a recorded acceptance result.
Schema, exporter, runtime, and required package updates belong to the same feature task.
Do not leave a schema-only commit that prevents the current package from loading.
Ignored generated assets stay in this worktree and remain outside Git.

Use these local paths during execution:

```powershell
$g10Root = 'C:/Users/Corashina/.codex/worktrees/g10-garage-controls/the-long-drive-wasm'
$g10Primary = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm'
$g10Python = "$g10Primary/tools/reference/.venv/Scripts/python.exe"
$g10Library = "$g10Primary/web/assets/library"
Set-Location -LiteralPath $g10Root
```

The primary library is a read-only input. All derived output belongs to this worktree.
Use Python JSON parsing. The manifest contains source material keys that differ only by case.
Never edit the primary checkout, its runtime package, its accepted library, or another task's worktree.

After every schema change, use this local export cycle:

```powershell
& $g10Python -m tools.reference.export --stage runtime --library-root $g10Library --output web/assets/reference
& $g10Python -m tools.reference.export --stage runtime --library-root $g10Library --output web/assets/reference --verify
```

Expected: source build `11140890`, validated reference records, and a matching immutable package under this worktree's `web/assets/runtime`.
Check each exit code. Do not run verification after failed derivation.
Do not change accepted source records to bypass a failed export.

## File responsibilities

| Files | Responsibility |
| --- | --- |
| `src/interaction/mechanisms.rs`, `src/starting_area.rs` | Input eligibility and fixed-step reaction dispatch. |
| `src/interaction/mechanisms/reactions.rs` | Endpoint event detection and bounded pending impulses. New in Task 2. |
| `src/reference.rs`, `tools/reference/mechanisms.py` | Required source reaction records and validation. |
| `src/world/content_mechanisms.rs`, `src/reference/world.rs`, `tools/reference/world_mechanisms.py` | Preserve, validate, and remap generated mechanism reaction records. |
| `src/reference/instruments.rs`, `tools/reference/instruments.py` | Dashboard and needle material records. |
| `src/presentation/instruments.rs` | Existing material binding, preparation, switching, and asset validation. |
| `tools/reference/radio.py` | Recover radio visual links. New in Task 4. |
| `tools/reference/audio.py`, `src/reference/audio.rs`, `tools/reference/export.py` | Connect radio visual records to the existing audio reference. |
| `tools/reference/gltf.py` | Include radio material variants in the selected glTF. Gauge variants already have a collection path. |
| `src/presentation/radio.rs`, `src/presentation/radio/needle.rs` | Cached radio transform bindings and parent-space needle placement. New in Task 4. |
| `src/app.rs`, `src/presentation/mod.rs`, `src/presentation/save.rs`, `src/presentation/lighting.rs` | App scheduling, readiness, warmup, and load integration. |
| `tests/garage_controls.rs` | Critical input, physics, schema, and restored-state integration checks. New in Task 1. |
| `tools/reference/tests/test_mechanisms.py`, `test_instruments.py`, `test_radio.py` | Critical source export tests. New tests under `tools/reference/tests`. |
| `tests/web/audio.test.mjs`, `input.test.mjs`, `menu.test.mjs` | Existing browser lifecycle checks. Extend only where critical coverage is missing. |
| `docs/g10-controls.md`, `docs/roadmap.md` | Actual results, source evidence, runtime limits, and roadmap status. |

### Task 1: Match source handbrake and light input rules

**Files:** Modify `src/interaction/mechanisms.rs` and `src/starting_area.rs`. Create `tests/garage_controls.rs`.

**Interfaces:**

- Consume `MechanismState::primary_switch(item: ItemId) -> bool` and `MechanismState::step(item, input, view, dt)` unchanged.
- Consume `GarageRuntime::step(&mut self, reference: &ReferenceManifest, input: FrameInput)` unchanged.
- Produce the corrected E, LMB, and L behavior without a new public input API.
- Create private test helpers `load_garage() -> (ReferenceManifest, GarageRuntime)` and `press(run: &mut GarageRuntime, reference: &ReferenceManifest, held: u32) -> ()`.
- `press` sends one pressed frame through `GarageRuntime::step`. Tests send held and release frames explicitly.

- [ ] **Step 1: Prepare the local source package and verify the baseline.**

Copy the primary `web/assets/reference` directory into the empty worktree asset directory.
Read its `content_base` with Python and copy that exact package from the primary `web` directory.
Preserve relative paths. Do not copy the whole runtime cache or use writable directory links.
If destination assets already exist, verify their receipts instead of overwriting them.
Record the manifest SHA-256 and package ID. The test helper sets started and unpaused state explicitly. This setup proves no browser acceptance.
The reference must contain one Lada and source build `11140890`.

Run `./scripts/check.ps1 -Test fresh_session`.
Expected: non-ignored baseline native tests and both WASM compile checks pass.
Report ignored tests as not run. Missing source or tool prerequisites are blockers, not skipped passes.

- [ ] **Step 2: Add failing tests `g10_handbrake_press_priority` and `g10_light_shortcut_seat_roles`.**

Use the actual Lada handbrake and seat records from the installed export. Each input case starts from a fresh runtime.
Resolve the handbrake action through interaction availability and dispatch, not only the direct mechanism method.
Use a carried cleaning tool to exercise the existing PRIMARY reservation.
Configure a fresh player instance for each seat case. Do not depend on G09 direct seat switching.

The assertions must include:

```rust
assert_eq!(after_e_press, !initial_handbrake);
assert_eq!(after_lmb_press, !initial_handbrake);
assert_eq!(after_combined_press, !initial_handbrake);
assert_eq!(after_held_frame, after_lmb_press);
assert_eq!(after_reserved_lmb, initial_handbrake);
assert_eq!(driver_headlights, 1);
assert_eq!(secondary_control_headlights, 1);
assert_eq!(ordinary_passenger_headlights, 0);
```

Run `./scripts/check.ps1 -Test garage_controls`.
Expected: native failures for handbrake LMB and the secondary control seat's L shortcut.

- [ ] **Step 3: Correct the two input predicates.**

Include `TurnAction::ToggleHandbrake` in existing primary-switch eligibility.
Keep the single pressed-bit dispatch and the existing action reservations.
Allow the L shortcut when `driving()` or `driving2()` is true.
Do not change other vehicle input permissions or source seat records.

- [ ] **Step 4: Run the targeted checks.**

Run `./scripts/check.ps1 -Test garage_controls`.
Expected: all added tests pass, formatting passes, and WebGL2 and WebGPU compile checks pass.

- [ ] **Step 5: Commit the input slice.**

Stage only the Task 1 source and test files.
Commit with `fix: match source garage control inputs`.

### Task 2: Export and apply endpoint reactions

**Files:** Modify `src/reference.rs`, `src/reference/world.rs`, `tools/reference/mechanisms.py`, `tools/reference/world_mechanisms.py`, `src/world/content_mechanisms.rs`, `src/interaction/mechanisms.rs`, and `src/starting_area.rs`.
Create `src/interaction/mechanisms/reactions.rs` and `tools/reference/tests/test_mechanisms.py`.
Extend `tests/garage_controls.rs`.

**Interfaces:**

- Add required `MechanismReference.reaction: EndpointReactionReference`.
- Define `EndpointReactionReference` as a tagged enum with `None` and `Rotation` variants.
- `Rotation` contains `node: NodeId`, `axes: [bool; 3]`, `inverted: bool`, `force: f32`, and `events: [bool; 4]`.
- The event order is enter minimum, enter maximum, leave minimum, and leave maximum.
- Serialize `None` explicitly. Missing `reaction` must fail deserialization.
- Add `MechanismState::apply_reactions(&mut self, physics: &mut PhysicsWorld) -> ()`.
- Keep `MechanismState::step` unchanged at its call boundary.
- Keep `recover_mechanisms` and generated registration entry points unchanged at their call boundaries.

- [ ] **Step 1: Add failing source and physics tests.**

Python `test_endpoint_fields_and_clip_gates` checks axis flags, inversion, force, and all four clip-presence events.
Python `test_no_reaction_is_explicit` checks a non-force control and rejects a missing required source pointer.
Native `g10_endpoint_reactions` exercises both axes, each endpoint event, and a full-range crossing.
Native `g10_endpoint_root_and_restore` checks the root body, torque, drain-once behavior, and restored state.
Native `g10_reaction_schema` removes `reaction` from a valid record and tests invalid node and non-finite force rejection. Test non-finite force through a typed record mutation, not invalid JSON syntax.
Assert the reaction through body momentum and angular momentum, not a mocked callback count alone. Check event identities separately because opposite events can cancel.

Use these numerical cases:

```text
source force = 10000
input = 0.5, dt = 0.02 -> intensity = 0.25, force magnitude on one axis = 2500
input = 2, dt = 0.02 -> intensity = 1
leaving an endpoint reverses the corresponding entering force
impulse = force * FIXED_DT
unchanged endpoint, dt <= 0, or non-finite input -> no new impulse
```

Check detached-part root selection and a source root without a rigid body.
A restore or rebind after an emitted event must leave no pending impulse.
Check that changed collider poses still match their mechanism poses.

Run `& $g10Python -m unittest tools.reference.tests.test_mechanisms -v`.
Run `./scripts/check.ps1 -Test garage_controls`.
Expected: failures for absent reaction records and absent physics effects.

- [ ] **Step 2: Export and validate the reaction contract.**

Read `Xforce`, `Yforce`, `Zforce`, `invForce`, `force`, and the four endpoint clip pointers.
Map clips from `startPosEnd`, `endPosEnd`, `startPosStart`, and `endPosStart` into the event order above.
Use the actual control node. Preserve axis sums without normalization.
Do not use source `startForce` or `endForce` as event gates.
Validate finite nonnegative force and valid node references.
Preserve explicit `None` for controls without this source effect.
Validate generated reaction nodes in prototype-local space in `src/reference/world.rs`. Update generated recipe remapping for the new control node.
Generated controls retain their actual source records, never fabricated Lada values.

- [ ] **Step 3: Detect transitions and apply bounded impulses.**

Keep the endpoint algorithm in `src/interaction/mechanisms/reactions.rs`.
Retain the previous angles and the transformed input before clamping.
Use the intensity formula in the spec and the source event conditions on each axis.
Store at most four event contributions per control update in inline fixed storage.
The storage belongs to each control, including generated controls.
Resolve the current root handle from cached frame bindings after the mechanism transform update.
Capture the current source control point and world force axes. Use frame rotation for directions. Do not scale force axes with node scale.
Apply the single-step impulse with the equivalent torque at that point.
Use the existing PhysicsWorld body mutation path so query bounds remain current.
Drain reactions after interaction updates and before `PhysicsWorld::step_physics`.
Clear pending reactions on restore, mount, rebind, unbind, and cancellation paths.
Do not emit from those lifecycle operations.

- [ ] **Step 4: Regenerate the package and run targeted checks.**

Run the local export cycle defined above.
Run `& $g10Python -m unittest tools.reference.tests.test_mechanisms tools.reference.tests.test_world -v`.
Run `./scripts/check.ps1 -Test garage_controls`.
Expected: tests pass and both WASM compile checks pass.
Record any export prerequisite failure without modifying primary assets.

- [ ] **Step 5: Commit the complete endpoint slice.**

Stage Task 2 source and tests. Do not stage recovered assets.
Commit with `feat: apply source mechanism endpoint reactions`.

### Task 3: Complete dashboard and needle lighting

**Files:** Modify `tools/reference/instruments.py`, `src/reference/instruments.rs`, `src/presentation/instruments.rs`, `src/app.rs`, `src/presentation/lighting.rs`, and `src/presentation/save.rs`.
Create `tools/reference/tests/test_instruments.py`. Extend `tests/garage_controls.rs`.

**Interfaces:**

- Add `MaterialSwitchReference`, a tagged enum with `Absent` and `Renderer { node: NodeId, on: MaterialLink, off: MaterialLink }`.
- Add required `InstrumentReference.backlight: MaterialSwitchReference` and `NeedleReference.backlight: MaterialSwitchReference`.
- `Absent` represents a source-null renderer, never an omitted field or missing asset.
- Preserve `InstrumentState::update`, `InstrumentVisuals::bind`, `InstrumentVisuals::update`, and `InstrumentVisuals::validate_assets` signatures.
- Existing `InstrumentState.digits_lit` supplies the dashboard, needle, and digit lighting condition.
- Expose the existing `MaterialVisual` record as `pub(super)` for Task 4. Add `MaterialVisual::select(&self, on: bool, materials: &mut Query<&mut MeshMaterial3d<SourceMaterial>>) -> ()`. Migrate existing `select_material` callers and remove that function.

- [ ] **Step 1: Add failing export and lighting tests.**

Python `test_dashboard_and_needle_material_links` checks source renderer identities separately from moving needle nodes.
Python `test_source_null_and_missing_material` distinguishes a legitimate source-null dashboard renderer from a broken material pointer.
Native `g10_instrument_backlight_contract` rejects omitted backlight fields and broken node, asset, or material indices.
Native `g10_instrument_lighting_matrix` checks ignition off and on against headlight states 0, 1, and 2.
Check both sides of the electronics cutoff and each warning threshold from the source reference.

The lighting assertion is:

```rust
assert_eq!(state.digits_lit, local_ignition && matches!(headlights, 1 | 2));
```

Initialize from a sample, update, and restore to the same sample.
Assert equal needle angles, lamp bits, odometer digits, and lighting state.
These state tests do not prove that the rendered materials change.

Run `& $g10Python -m unittest tools.reference.tests.test_instruments -v`.
Run `./scripts/check.ps1 -Test garage_controls`.
Expected: new link-contract tests fail before implementation.

- [ ] **Step 2: Extend source records and manifest validation.**

Recover meter `R`, `OnM`, and `OffM` and each needle's corresponding fields.
Use existing glTF material indices. The exporter already collects meter and needle material variants.
Reject required renderer or material pointers that cannot be resolved.
Accept `Absent` only where the actual source renderer pointer is null.
Require the new fields during deserialization.

- [ ] **Step 3: Extend the existing presentation path.**

Include dashboard and needle material records in the same bind, prepare, switch, and validation iteration as digit materials.
Use the existing material cache, source variants, sort metadata, receiver masks, and warmup entities.
Switch all backlight materials when `digits_lit` changes.
Keep warning-lamp switching separate.
Include all added choices in `validate_assets` and load validation.
Gate session readiness on required instrument preparation.
Do not mark a partially bound material set as ready.

- [ ] **Step 4: Regenerate the package and verify the slice.**

Run the local export cycle.
Run `& $g10Python -m unittest tools.reference.tests.test_instruments -v`.
Run `./scripts/check.ps1 -Test garage_controls`.
Expected: native state and contract tests pass, and both WASM compile checks pass.
Rendered material selection remains subject to Task 5 browser acceptance.

- [ ] **Step 5: Commit the lighting slice.**

Stage Task 3 source and tests.
Commit with `feat: complete source dashboard backlights`.

### Task 4: Bind the built-in radio display and tuning needle

**Files:** Create `tools/reference/radio.py`, `tools/reference/tests/test_radio.py`, `src/presentation/radio.rs`, and `src/presentation/radio/needle.rs`.
Modify `tools/reference/audio.py`, `tools/reference/export.py`, `tools/reference/gltf.py`, `tools/reference/tests/test_audio.py`, and `src/reference/audio.rs`.
Modify `src/presentation/instruments.rs`, `src/presentation/mod.rs`, and `src/app.rs`. Extend `tests/garage_controls.rs`.

**Interfaces:**

- Add required `RadioAudioReference.visuals: RadioVisualReference`.
- Define `RadioVisualReference { screen: MaterialSwitchReference, indicator: MaterialSwitchReference, needle: NodeId, start: NodeId, end: NodeId }`.
- Require renderer variants for both radio material switches. Source `TMutato`, `ST1`, and `ST2` supply the three transform IDs.
- Add `recover_radio_visuals(source, radio, manifest, reports) -> dict` in `tools/reference/radio.py`.
- Change `recover_audio(source, roots, manifest, library, output, reports) -> dict` to require the existing glTF reports.
- Update all callers in the exporter and tests. Keep no old signature or fallback.
- Add `pub(super) struct RadioVisuals` in `src/presentation/radio.rs`.
- `RadioVisuals::bind(reference: &ReferenceManifest, radio: &RadioVisualReference, nodes: &HashMap<NodeId, Entity>, materials: [MaterialVisual; 2]) -> Result<Self, ReferenceError>` caches bindings.
- `RadioVisuals::update(&mut self, power: bool, tuning: f32, transforms: &mut Query<&mut Transform, Without<DriveCamera>>, materials: &mut Query<&mut MeshMaterial3d<SourceMaterial>>) -> ()` updates the display.
- `update_needle(transform: &mut Transform, start_world: Affine3A, end_world: Affine3A, parent_world: Affine3A, tuning: f32) -> ()` lives in `radio/needle.rs`. It changes translation only.
- Store the radio binding within `InstrumentVisuals` and reuse its preparation and asset-validation lifecycle. Expose its two material records within the presentation module for those existing iterators.
- Add `InstrumentVisuals::update_radio(&mut self, power: bool, tuning: f32, transforms: &mut Query<&mut Transform, Without<DriveCamera>>, materials: &mut Query<&mut MeshMaterial3d<SourceMaterial>>) -> ()`.

- [ ] **Step 1: Add failing source, schema, and transform tests.**

Python `test_radio_visual_source_links` checks `RS`, `SOn`, `SOff`, `RMutato`, `MutatoOn`, `MutatoOff`, `TMutato`, `ST1`, and `ST2`.
Python `test_radio_material_variants_are_exported` verifies those on and off materials appear in the selected glTF report.
Native `g10_radio_visual_contract` rejects omitted visuals and invalid renderer, transform, asset, or material links.
Native `g10_radio_power_and_tuning` checks exported electronics cutoff, volume zero, power-off tuning, and both band endpoints.
Native `g10_radio_needle_parent_space` imports the small `radio/needle.rs` module directly through a test `#[path]` attribute.
That module uses Bevy math and Transform, which the existing native dev dependency provides. Import it with `#[path = "../src/presentation/radio/needle.rs"]`.

Use these assertions for the transform test:

```text
translation at tuning 0 = parent_inverse * start_world.translation
translation at tuning 1 = parent_inverse * end_world.translation
translation at tuning 0.5 = parent_inverse * midpoint
translation at tuning -1 or 2 = the corresponding clamped endpoint
```

Use a moved, rotated, non-uniformly scaled parent with different endpoint ancestor chains.
Assert the source needle rotation and scale remain unchanged.
Run `& $g10Python -m unittest tools.reference.tests.test_radio -v`.
Run `./scripts/check.ps1 -Test garage_controls`.
Expected: radio visual and material-export tests fail before implementation.

- [ ] **Step 2: Extend radio export and validation.**

Add `radioscript` material collection to the existing extra-material pass in `tools/reference/gltf.py`.
Recover visual links using the selected asset report and source node mapping.
Call `recover_radio_visuals` from `recover_audio` and update the required `reports` argument at each caller.
Validate radio material links against the game's assets and the transform IDs against its node graph.
Do not add serde defaults or substitute missing materials.

- [ ] **Step 3: Bind and update visuals through the existing lifecycle.**

Build the two `MaterialVisual` values through the existing instrument material binding code.
Include radio materials in instrument preparation, warmup, readiness, and `validate_assets` iteration.
Cache root-to-node entity paths for the needle parent and both endpoints during `RadioVisuals::bind`.
At update, compose current local transforms along those paths without allocation.
Call `update_needle` with those current world transforms. It interpolates the source world point and converts that point into needle parent space.
Read transforms before mutating the needle transform.
This avoids stale GlobalTransform values before Bevy propagation.

Call `update_radio` from the existing app visual update after runtime state advances and before transform propagation.
Use `audio.radio_frame.power` and `audio.radio.tuning` from the existing `AudioState`.
Do not create a second radio simulation or infer tuning from rounded frequency.
Update the needle while radio power is off. Cache the last power value and switch materials only when that value changes.
On restore, initialize materials and position from the restored runtime state before a visible frame.
Keep audio playback independent of material binding.

- [ ] **Step 4: Regenerate and verify the radio slice.**

Run the local export cycle.
Run `& $g10Python -m unittest tools.reference.tests.test_radio tools.reference.tests.test_audio -v`.
Run `./scripts/check.ps1 -Test garage_controls`.
Expected: source contract and transform tests pass, and both WASM compile checks pass.
Audible playback and visible display behavior remain subject to Task 5.

- [ ] **Step 5: Commit the radio slice.**

Stage Task 4 source and tests.
Commit with `feat: render source radio controls`.

### Task 5: Verify lifecycle and accept the normal-input controls

**Files:** Extend `tests/garage_controls.rs` and existing web tests only where critical lifecycle coverage is missing.
Create `docs/g10-controls.md`. Update `docs/roadmap.md`.
Repair a reproduced G10 defect in its owning module before marking that scenario complete.

**Interfaces:**

- Consume the preceding task interfaces and the unchanged save schema.
- Produce a results document with source build, commit, manifest hash, package ID, commands, browser, inputs, and outcomes.
- Produce a roadmap entry that distinguishes implemented code, passing checks, failures, dependencies, and open runtime acceptance.

- [ ] **Step 1: Verify lifecycle boundaries with targeted tests.**

Add `g10_control_restore_has_no_side_effects` to the native suite.
Use `GarageRuntime::capture_save`, `GarageRuntime::from_save`, and `advance_saved_restore` for changed mechanism and radio state. Construct instrument samples from the restored control values without duplicating app presentation code.
Assert unchanged saved control values, zero new endpoint impulse, zero replayed endpoint event, and derived visual state matching the restored sample.
Inspect existing audio, input, and menu tests for held horn release and duplicate radio voice coverage.
If missing, add `capture_loss_clears_horn` and `radio_resume_reuses_playback` to those existing suites.
Observe the real InputState and VoiceBank outputs. Do not mock the assertion target.
Run each added test before a fix and record the actual failure, if any.
A passing regression case requires no invented product change.

- [ ] **Step 2: Run the final targeted checks.**

```powershell
& $g10Python -m unittest tools.reference.tests.test_mechanisms tools.reference.tests.test_instruments tools.reference.tests.test_radio tools.reference.tests.test_audio tools.reference.tests.test_world -v
./scripts/check.ps1 -Test garage_controls
./scripts/check.ps1 -WebTest audio,input,menu
```

Expected: all targeted tests pass, formatting passes, and both WASM compile checks pass.
Do not run `scripts/check.ps1 -Full`.

- [ ] **Step 3: Build the final matching package at the approved milestone.**

Run the local export verification command once for the final schema.
Run `./scripts/build.ps1` once after targeted checks pass.
Expected: the existing release build produces matching WASM and browser modules without errors.
Record build output and package receipts.
Repeat only if a subsequent fix or a failed check requires another build.

- [ ] **Step 4: Run all 12 browser scenarios from the spec through normal input.**

Launch `node scripts/serve.mjs` from this worktree with an unused local `PORT`.
Record the exact launch command and URL in `docs/g10-controls.md`.
Use mouse capture, keyboard presses, mouse movement, and clicks on the visible package.
Require active physics, submitted frames, and visible control results.
Record each spec scenario as pass, fail, or blocked with evidence.
Include audible horn and radio output, gauge readability, reaction behavior, all five seats, and all five home switches. Audio flags alone do not prove audible output. Record audio evidence or leave that result unverified.
Record G09 seat dependencies instead of duplicating G09 code.
Capture or asset failures leave dependent scenarios open. Report independent scenarios already completed.
Do not close G10 using HTTP status, menu readiness, synthetic events, or compile success.

- [ ] **Step 5: Record results and complete branch review.**

Write `docs/g10-controls.md` with actual evidence and deferred checks.
Update the G10 note in `docs/roadmap.md` and link the results document.
Keep G10 at Verify while any required normal-input scenario remains open.
Run `git diff --check` and review the entire implementation against the approved spec.
Keep the whole-branch review after the Task 5 commit and task review.

- [ ] **Step 6: Commit the verification record and report the outcome.**

Commit with `docs: record G10 control verification`.
Report changes, checks, failures, deferred checks, and unverified runtime behavior.
Report the worktree path and branch. Keep the branch unmerged.

After the Task 5 review, dispatch the final whole-branch reviewer.
Resolve actionable G10 findings and rerun only the affected checks.
Commit review fixes before reporting completion.

## Plan self-review

- Coverage: Tasks 1 through 4 cover all five approved source differences.
- Existing controls, restored state, source dependencies, and all 12 browser scenarios are covered by Task 5.
- Schema additions are required. Explicit absence describes source-null data and provides no old-export compatibility.
- Each feature task includes export, runtime, targeted checks, and a usable package.
- The shared interfaces above define each cross-task dependency once.
- The five review-focus conditions have named tests or final acceptance scenarios in their owning tasks.
- Material preparation and save validation include the new variants.
- No task changes G09 seat switching, G11 driving physics, G13 save scope, or the deferred custom radio scope.
- The plan uses the selected subagent method. Implementation waits for the user's plan review.
