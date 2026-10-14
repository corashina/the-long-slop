# Cleaning and Painting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore source brush, sponge, and spray use on eligible surfaces in the current garage and both cars.

**Architecture:** Checked condition records feed one runtime condition store and retained tool state.
Held tools use the existing input, carry physics, collider ownership, audio, and particle systems.
Presentation updates retained material instances after changes; saves retain semantic state through stable source identities.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, Rapier 0.36.0, serde, Python, WASM, WebGL2, and the existing Docker toolchain.
**Spec:** [Approved cleaning and painting design](../specs/2026-10-14-cleaning-painting-design.md).
**Status:** Awaiting plan review and execution-method selection.
**Execution:** Recommend native execution because each layer uses the previous layer's runtime and presentation interfaces.
**Base:** Includes design commit 5218726. Record the actual implementation base when creating the worktree.

## Global Constraints

- Use Steam app 1017180, build 11140890, and Unity 2021.3.18f1.
- Read the accepted library through its existing receipt and hash checks.
- Keep Steam files, the accepted source archive, and recovered C# unchanged.
- Keep generated assets and recovery records outside Git.
- Use only source-enabled startup tools.
- This feature does not add inventory slots, new vehicles, loot generation, weapons, or a color selection interface.
- Do not create GPU assets, scene bindings, or collections during frame updates.
- Bump the save schema for the changed contract. Add no migration or old-schema reader.
- Preserve other chats' files and worktrees.
- Keep each layer usable before adding the next.
- Compare matched source actions and views before claiming source parity.
- Follow AGENTS.md and stop-slop for documentation and UI text. Add only critical tests.

## File Responsibilities

| Files | Responsibility |
| --- | --- |
| `tools/reference/conditions.py`, `src/reference/conditions.rs` | Checked source records, startup choices, and validation |
| `src/conditions.rs` | Mutable condition, repair progress, color, wash state, and propagation |
| `src/interaction/cleaning.rs` | Tool state, contact motion, rays, repair, spray, and feedback signals |
| `src/presentation/conditions.rs` | Retained materials, variants, tool poses, and particle presentation |
| Existing save modules | Stable snapshots, validation, restoration, and candidate preparation |
| Existing app, interaction, and audio modules | Dispatch and scheduling within current owners |

## Review Focus

1. Target movement, rebase, or load must not create rubbing. Task 3 tests contact resets and relative displacement.
2. A shared source material must not spread one part's paint. Task 2 checks owner isolation in the browser.
3. A held tool must reserve LMB through an ineligible aim. Task 3 tests precedence against doors, filling, and light switches.
4. Save preparation failure must preserve the current runtime and visuals. Task 2 extends the existing candidate failure test.
5. Frame catch-up must not repeat rubbing or spray work. Task 3 tests one tool call per accepted active frame.

---

### Task 1: Recover the source tools and complete condition contract

**Files:** Create `src/reference/conditions.rs`, `tests/conditions_reference.rs`, and `tools/reference/tests/test_conditions.py`.
Modify `tools/reference/{conditions,export,manifest,gltf,materials,audio,weather}.py`, `src/reference.rs`, and `tests/common/reference.json`.
Modify existing reference test literals only where the required contract changes.

**Interfaces:**
- Add `ConditionId(u32)` and `WashId(u32)` through the existing ID macro.
- Add required `ReferenceManifest.conditions: ConditionRecord`, without a serde default.
- `ConditionRecord` contains `surfaces`, `brushes`, `sprays`, `washes`, `collider_bindings`, `repair_tables`, `clean_tables`, and `repair_rate`.
- `ConditionStartup` contains `state: u8`, `state2: u32`, `state3: u32`, `repair_progress: f32`, and `color: [f32; 4]`.
- Surface records bind source controller, node, optional item owner, startup state, permission-table index, variants, renderers, and child flags.
- Brush and spray records bind source controller, item, body, pickup, tool parameters, model/nozzle nodes, voices, and particle records.
- Wash records bind source controller, node, colliders, linked objects, permission index, progress, activation, and destruction rules.
- Collider records bind direct, pickup, forwarded, installed-part, and wash owners. Keep their distinct source precedence.
- Reuse `SourceId`, `ItemId`, `NodeId`, `ColliderId`, `PartSlotId`, existing material keys, audio IDs, and `ParticleReference`.
- Python: `prepare_cleaning_roots(source, roots) -> None` and `recover_conditions(source, nodes, components, pointer_node, item_nodes, colliders, output) -> dict`.
- Rust: `ConditionRecord::validate(&self, reference: &ReferenceManifest) -> Result<(), ReferenceError>`.

- [ ] **Step 1: Create the isolated execution workspace and check its baseline.**

Use using-git-worktrees. Inspect attached worktrees and reuse a suitable cleaning worktree when available.
Otherwise use the native worktree tool with the current approved-document commit as `ref`.
Use branch `codex/cleaning-painting`; do not copy unrelated uncommitted files.
Copy the current accepted runtime assets into the worktree's ignored asset directory before baseline checks.
Verify their recorded hashes. Do not link runtime output back to the primary checkout.
Run `docker info`, `./scripts/check.ps1`, and `./scripts/build.ps1` from the worktree.
Require exit 0. Record failures before attributing them to this feature.
Use the primary accepted library as read-only input through checked `LibraryReader` and `LibrarySource`.
Keep runtime output and browser package in the isolated worktree. Do not replace primary generated assets.
Load the shared snapshot with `load_snapshot(primary_cache)` and its library with `verify_library(primary_library, snapshot.root)`.
Call `derive_reference(snapshot, receipt, worktree_output)` and `verify_reference(worktree_output, receipt)` through the existing production Python API.
The isolated output must remain outside the protected accepted library and source storage.

- [ ] **Step 2: Pin checked source identities and startup rules.**

Verify the managed SHA-256 from the specification and each selected record through its receipt.
Terrain settings `level1:64898` enable GameObject `level1:117` through `EnableAfterStart`.
That group is serialized inactive; its six selected tool children are serialized active.

| Tool | Controller | GameObject | Linked state |
| --- | --- | --- | --- |
| Brush 1 | level1:66615 | level1:10842 | slide 68227; condition 77707 |
| Brush 2 | level1:66617 | level1:10846 | slide 68224; condition 77709 |
| Sponge 1 | level1:66616 | level1:10838 | slide 68226; condition 71298 |
| Sponge 2 | level1:66619 | level1:10849 | slide 68225; condition 71299 |
| Spray 1 | level1:73834 | level1:10841 | ammo 78225; color 73209 |
| Spray 2 | level1:73835 | level1:10840 | ammo 78223; color 73210 |

Include these six source-enabled roots before condition, material, lighting, audio, and geometry recovery.
Keep their authored transforms. Resolve their pickup, body, mass, renderer, and save links through source pointers.
Exclude the player `PointAtObject` controllers and the duster from this approved brush/sponge/spray scope.
Brush cleaning type is 1; sponge cleaning type is 2. Both have `canRuin=true` and `repairModifier=1`.
Recover masks, pose settings, wash search radius, particles, source text, and initial condition from each checked controller.
The selected brush/sponge slides use distance 2 and mask 8552449.
Force uses source float values 0.009999999776482582 and 0.10000000149011612 for minimum and maximum displacement.
The minimum repair force is 0.10000000149011612; shared repair rate is 0.5.
Sprays use distance 3, mask 8552449, colorLerp 0.25, paintLoss 4.5, valveSpeed 10, and rate 250.
Both spray ammo controllers use floats, `fammo=100`, and `max=100`.
`tosaveitemscript` excludes sprays from its ammo-randomization branch. Do not start them with a random 20-to-100 quantity.
Use the existing deterministic startup policy for unset can colors while retaining source branches and palettes.
Record the selected colors. Do not claim native Unity random-sequence parity.

- [ ] **Step 3: Write and run critical failing reference checks.**

Python tests: `test_startup_tools_follow_enable_after_start`, `test_spray_keeps_authored_float_ammo`, and `test_variants_close_material_dependencies`.
Assert exactly two brushes, two sponges, and two sprays; preserve their authored activation and pointers.
Assert both spray quantities are 100 and all reachable condition materials/meshes have checked resources.
Use these Python assertion values after resolving tool records from the checked startup group:

```python
self.assertEqual(len(record['brushes']), 4)  # Two brushes and two sponges.
self.assertEqual(sorted(t['cleaning_type'] for t in record['brushes']), [1, 1, 2, 2])
self.assertEqual([t['startup_quantity'] for t in record['sprays']], [100.0, 100.0])
```

Use `cleaning_type` and `startup_quantity` as the corresponding exported tool field names.
Rust tests: `missing_condition_owner_is_rejected`, `child_cycle_is_rejected`, and `invalid_variant_is_rejected`.
Use malformed copies of `common::reference()`; require `ReferenceError` with the source identity and field.
Run `tools/reference/.venv/Scripts/python.exe -m unittest tools.reference.tests.test_conditions` and `./scripts/check.ps1 -Test conditions_reference`.
Require a failure caused by the missing contract before implementation.

- [ ] **Step 4: Implement the contract, variant export, and validation.**

Extend existing condition recovery instead of adding a second startup selector.
Export selected startup values and every reachable runtime material, mesh, object variant, and child flag.
Add unused runtime variants to the selected asset's checked geometry/material resources and rendering records.
Keep source material flags and color rules; do not use a generic PBR replacement for condition variants.
Bind condition/wash targets even when their visible node has no separate pickup item.
Add `conditions-evidence.json` to selected export verification and receipts.
Include condition/tool texture, audio, and particle resources in existing dependency collection and path validation.
Validate unique identities, required links, finite parameters, table indices, source state bounds, and acyclic child ownership.
Keep tool-disabled state 5 valid only for its source tool controller.
Update the synthetic fixture with explicit condition records and source-derived permission tables.
Derive a fresh runtime export through the existing atomic promotion path and verify it.

- [ ] **Step 5: Verify unchanged startup behavior and commit.**

Run the Step 3 commands, `./scripts/check.ps1 -Test reference`, and `./scripts/build.ps1`; require exit 0.
Open the worktree browser build. Require loading readiness, the six authored tools, and working walking/driving interactions.
Record export/package hashes and source identities in `docs/research/2026-10-14-cleaning-painting-reference.md`.
Commit only Task 1 files: `feat: recover source cleaning and painting records`.

### Task 2: Add live conditions, retained variants, and save restoration

**Files:** Create `src/conditions.rs`, `src/presentation/conditions.rs`, and `tests/conditions.rs`.
Create the startup-state portion of `src/interaction/cleaning.rs`.
Modify `src/{lib,starting_area,interaction,app}.rs`, `src/presentation/{mod,materials,reference,save}.rs`,
`src/save/{schema,validate}.rs`, `src/starting_area/save.rs`, `src/app/save.rs`, and existing save fixtures/tests.

**Interfaces:**
- `ConditionState` uses the five semantic fields in `ConditionStartup`; it has no renderer handles.
- `ConditionStore::new(reference: &ReferenceManifest) -> Result<Self, ReferenceError>`; owned by `StartingArea.conditions`.
- `ConditionStore::get(&self, id: ConditionId) -> &ConditionState` and `quality(&self, id: ConditionId) -> f32`.
- `ConditionStore::repair(&mut self, id: ConditionId, tool: u8, gain: f32) -> RepairResult`.
- `RepairResult` is `None`, `Improved`, or `Worsened`. Completed steps reset progress to zero.
- `ConditionStore::paint(&mut self, id: ConditionId, color: [f32; 4]) -> bool` and `wear(&mut self, id: ConditionId) -> bool`.
- `paint` reports an allowed color change; `wear` reports whether the source tool is now disabled.
- `ConditionStore::take_changed(&mut self) -> Option<ConditionId>` drains a bounded, preallocated dirty queue.
- `CleaningTools::new(reference: &ReferenceManifest) -> Result<Self, ReferenceError>`; owned by `InteractionState.cleaning`.
- `ToolState` retains item identity, can quantity/color, and source-disabled state; brush wear stays in its condition owner.
- `ConditionSnapshot` contains condition, wash, and tool vectors keyed by existing `SourceKey`.
- `ConditionStore::capture_save(&self, reference: &ReferenceManifest) -> Result<(Vec<SurfaceSnapshot>, Vec<WashSnapshot>), SaveError>`.
- `ConditionStore::restore_save(&mut self, reference: &ReferenceManifest, saved: &ConditionSnapshot) -> Result<(), SaveError>`.
- `CleaningTools::capture_save(&self, reference: &ReferenceManifest) -> Result<Vec<ToolSnapshot>, SaveError>`.
- `CleaningTools::restore_save(&mut self, reference: &ReferenceManifest, saved: &[ToolSnapshot]) -> Result<(), SaveError>`.
- The save owner combines these vectors into `ConditionSnapshot { surfaces, washes, tools }`.
- `ConditionVisuals` retains per-owner variant handles and bindings; register `ConditionPlugin` in the existing presentation schedule.

- [ ] **Step 1: Write the failing condition and save assertions.**

In `tests/conditions.rs`, build from `common::reference()` and select fixture surfaces by exported condition IDs.
`repair_threshold_resets_without_overflow`: at state 2 and progress 0, gains 0.75 then 0.5 produce state 1 and progress 0.
`damage_precedes_repair`: enable source permission column 3; type 1 and gain 1 produce one worse state.
`tool_wear_disables_after_state_four`: wear at state 4 produces state 5 and a disabled tool.
`children_follow_only_source_flags`: permitted children inherit; independent children retain their condition/color.
For `repair_threshold_resets_without_overflow`, set the fixture surface to state 2 and permit cleaning type 2:

```rust
let mut r = common::reference();
let surface = &mut r.conditions.surfaces[0];
surface.startup.state = 2;
surface.startup.repair_progress = 0.;
let id = surface.id;
let mut store = ConditionStore::new(&r).unwrap();
assert_eq!(store.repair(id, 2, 0.75), RepairResult::None);
assert_eq!(store.repair(id, 2, 0.5), RepairResult::Improved);
assert_eq!(store.get(id).state, 1);
assert_eq!(store.get(id).repair_progress, 0.);
```

The fixture's first surface must use the checked repair-property table permitting sponge repair from state 2.
Extend save round-trip tests to preserve different surface colors, progress -0.5, tool wear, quantity 37.5, and cleaned wash state.
Reject duplicate/missing owners, NaN values, invalid variants, and out-of-range quantities before runtime replacement.
Extend existing save candidate-failure coverage: missing variant assets must preserve the live runtime and prior visuals.
Run `./scripts/check.ps1 -Test conditions` and `./scripts/check.ps1 -Test save_roundtrip`; require feature-specific failure.

- [ ] **Step 2: Implement source condition rules and semantic snapshots.**

Apply source repair tables and the type-1 damage branch before normal repair.
Propagate state/color/variants through the checked child flags; use retained traversal storage and reject duplicate dirty entries.
Store wash activation, progress, and destruction independently from surface condition.
Expose quality as a derived value only where a source condition controls physics or the existing condition display.
Remove duplicate mutable scalar item/save condition where the live owner replaces it; adapt existing consumers and fixtures.
Bump `SAVE_SCHEMA_VERSION` from 1 to 2 and update matching web/save fixtures. Do not add defaults or old readers.
Restore semantic state into the existing candidate runtime before presentation preparation or input activation.

- [ ] **Step 3: Implement retained presentation and candidate preparation.**

Prepare all selected variants and per-owner color instances during loading, using existing `SourceMaterials` and renderer bindings.
Keep the material color conversion in the existing source-color path.
Update existing uniforms/handles for dirty owners; retain caster, reflection, mirror, mesh bounds, and motion bindings.
Apply source mesh/object variants and wash visibility without deleting required stable identities.
Extend existing pipeline warmup and readiness checks for condition variants and rebuilt load/restart instances.
Include prepared condition bindings in `PreparedPresentation`; commit them only when the whole save candidate is ready.

- [ ] **Step 4: Verify this working layer and commit.**

Run `./scripts/check.ps1 -Test conditions`, `./scripts/check.ps1 -Test save_roundtrip`, and `./scripts/build.ps1`; require exit 0.
Use a labelled browser diagnostic to change one eligible part and save/load it. Remove that diagnostic before final acceptance.
Require another part sharing the source material to retain its own color in the main view and mirrors.
Check repeated load/restart readiness and missing-variant candidate rejection. Record visible evidence and limits.
Commit Task 2: `feat: retain live source conditions and save state`.

### Task 3: Add brush and sponge work with source motion and feedback

**Files:** Modify `src/interaction/{cleaning,targets,feedback,labels,carry}.rs`, `src/{interaction,starting_area,app,audio}.rs`,
`src/presentation/conditions.rs`, and `src/weather/particles.rs` only for necessary shared particle controls.
Create `tests/cleaning.rs`; extend existing interaction tests only for LMB conflicts.

**Interfaces:**
- `ToolFrame` contains `held: Option<ItemId>`, `lmb: bool`, `active: bool`, `dt: f32`, `view: VehiclePose`, and `origin: [f64; 3]`.
- `ToolSignals` exposes retained per-tool contact pose, force, work state, voice gain, emission, and color to presentation/audio.
- `CleaningTools::advance(&mut self, frame: ToolFrame, reference: &ReferenceManifest, conditions: &mut ConditionStore,
  items: &ItemStore, physics: &PhysicsWorld, parts: &PartAttachments) -> bool` returns whether it reserves held LMB.
- `CleaningTools::reset_contact(&mut self)` clears transient motion/firing without changing semantic state.
- `CleaningTools::signals(&self) -> &[ToolSignals]` provides borrowed outputs without frame allocation.
- `GarageRuntime::advance_tools(&mut self, reference: &ReferenceManifest, frame: ToolFrame)` owns one active-frame call.

- [ ] **Step 1: Pin rubbing, ownership, and scheduling failures.**

`stationary_contact_does_not_repair`: holding LMB without relative motion leaves progress unchanged.
`moving_target_does_not_count_as_rubbing`: equal tool/target translation produces zero displacement.
`contact_reset_does_not_wear_tool`: target change, rebase, recovery, or load initializes the sample without progress.
`blocked_ray_cannot_repair_far_surface`: nearest solid hit blocks an eligible condition behind it.
`held_tool_reserves_lmb_on_invalid_target`: no door, fill, or flashlight action runs during the same held-tool use.
`one_active_frame_has_one_tool_update`: simulate several physics catch-up steps; repair applies once from that frame's sample.
Test wash progress at exactly 1 and above 1; only the latter completes and applies permitted tool wear.
Run `./scripts/check.ps1 -Test cleaning`; require feature-specific failures.

- [ ] **Step 2: Implement brush and sponge contact and repair.**

Resolve source tool-specific rays through Rapier with checked masks and nearest-hit blocking.
Build collider-to-condition links during loading; resolve current slot occupancy through existing `PartAttachments`.
Apply wash, installed-part, direct-parent, and forward-link precedence, plus checked nearby wash-trigger search.
Retain contact samples and source model interpolation. Use target-relative displacement and source strict force threshold.
Call source repair/wash rules and apply wear only on source completion events.
Disable worn tools through their source model, text, sound, and particle rules.

- [ ] **Step 3: Integrate one active-frame call and tool feedback.**

Reserve held-tool LMB in existing action dispatch before competing world actions.
Retain held-tool frame input separately from fixed-step `take_step`; do not add a new key or browser input protocol.
Schedule tool work once after the current physics batch and imported pose update, before condition sync and HUD publication.
Use the accepted active frame delta, not repeated `FIXED_DT` or accumulated `render_delta_seconds` as a movement sample.
Respect pause, startup readiness, death, pointer capture, and the existing skip-elapsed gate after lifecycle changes.
Reset contact on release, drop, load, recovery, rebase, and loss of pointer capture.
Use recovered source names/prompts; show tool wear through the existing HUD condition field.
Drive retained source voices and particles from `ToolSignals`; use existing `AudioState::voice` and `ParticleState`.
Add focused particle pose/color setters only when existing visibility prevents tool control. Do not create a second particle engine.

- [ ] **Step 4: Verify cleaning end to end and commit.**

Run `./scripts/check.ps1 -Test cleaning`, `./scripts/check.ps1 -Test interaction`, and `./scripts/build.ps1`; require exit 0.
Use both brush and sponge instances on eligible surfaces on both cars. Check unsupported and covered targets.
Wear one tool to disabled state; save/load it and check the model, prompt, sound, and particles.
Check wash-target completion when the selected scene includes an eligible wash target.
Check moving targets, stationary contact, pause/resume, recovery, and origin shifts for false progress.
Commit Task 3: `feat: restore source brush and sponge cleaning`.

### Task 4: Add spray painting, depletion, and release behavior

**Files:** Modify `src/interaction/cleaning.rs`, `src/presentation/conditions.rs`, `src/{audio,app}.rs`, and `tests/cleaning.rs`.
Modify `src/interaction/feedback.rs` and the existing browser feedback adapter for spray quantity when required.

**Interfaces:** Reuse Task 3's frame dispatch and signals; add spray state within each retained tool instance.
Map source `inRightHand` and carried-nozzle behavior to the existing supported carry mode through checked player handling.
Keep the unused inventory mode outside gameplay. Do not force a camera ray when the supported source mode uses the nozzle.
Reuse the existing amount/capacity HUD fields for can quantity, with the source quantity label and percentage policy.
Keep `FluidFeedback` reserved for real fluids; obtain tool quantity through `CleaningTools` when publishing the HUD.

- [ ] **Step 1: Write the failing spray assertions.**

`spray_consumes_paint_on_miss`: quantity 100, valve 0, dt 0.1 gives valve 1 and quantity 99.55 within float tolerance.
`spray_blends_by_source_distance`: distance 2, dt 0.1, colorLerp 0.25 gives clamped color factor 0.0125.
`empty_spray_stops_color_sound_and_particles`: quantity 0 changes no surface and emits no sound/particles.
`unsupported_surface_still_consumes_paint`: reject painting through permission column 0 while decreasing quantity.
`spray_release_closes_valve`: with valve 1, release and dt 0.05 produce valve 0.5 without paint loss.
`saved_spray_keeps_color_and_quantity`: save/load quantity 37.5 and a selected palette color; the label and output agree.
Run `./scripts/check.ps1 -Test cleaning`; require the new assertions to fail.

- [ ] **Step 2: Implement checked spray rules and presentation.**

Apply source valve interpolation, quantity clamp, direct/pickup condition resolution, and distance-weighted color interpolation.
Consume paint before target eligibility, as `sprayscript.Fire` does; retain source call order at the depletion boundary.
Run source release interpolation only when not firing. Derive `hasPaint` from quantity.
Update retained can material, source particles, and source audio volume from the same can color and valve state.
Keep can quantity and color under its stable item identity through carry/drop and save restoration.
Use existing quantity feedback where suitable; any required adapter must keep source labels and control conventions.

- [ ] **Step 3: Verify painting end to end and commit.**

Run `./scripts/check.ps1 -Test cleaning`, `./scripts/check.ps1 -Test save_roundtrip`, and `./scripts/build.ps1`; require exit 0.
Use both cans on each car. Check distance falloff, blocked targets, misses, unsupported surfaces, release, and an empty can.
Check painted removable parts after removal/reinstallation, save/load, pause, recovery, and repeated restart.
Require stable color in the main view, shadows, mirrors, and reflections without material changes on unrelated parts.
Commit Task 4: `feat: restore source spray painting and depletion`.

### Task 5: Complete source and browser acceptance

**Files:** Modify `docs/{verification,roadmap}.md`, `PRODUCT.md`, and the feature research record.
Remove temporary diagnostics and obsolete condition state paths in their owning modules.

- [ ] **Step 1: Run the integrated required checks.**

Run the new Python condition tests and production runtime-export verification through the existing exporter.
Run `./scripts/check.ps1` and `./scripts/build.ps1`; require exit 0 and record code/export/package hashes.
If unrelated baseline checks fail, report them without hiding feature failures or claiming full acceptance.

- [ ] **Step 2: Complete the visible preparation and driving route.**

Walk to the six authored tools, clean and paint eligible surfaces on both cars, and remove/reinstall a changed part.
Drive each car after preparation. Verify wheels, doors, fluids, lights, plates, storage, and existing save behavior.
Check pointer capture, pause, death gating, recovery, save/load, repeated restart, and origin changes.
Compare source actions and matched views. Record controls, surfaces, tool state, camera, and unresolved parity limits.
Inspect retained material, particle, and binding counts after repeated work and load/restart; require no repeated setup growth.

- [ ] **Step 3: Review, remove obsolete paths, and update documents.**

Review the whole feature branch against the specification, including frame allocations and required failure cases.
Use the selected execution method's independent review gate. Resolve feature findings and rerun affected checks.
Remove obsolete startup-only selection and duplicate scalar state paths after their live replacements pass.
Retain the startup initializer and its checked source inputs; remove obsolete runtime ownership, not necessary source derivation.
Remove labelled diagnostics and their routes before claiming gameplay acceptance.
Update `PRODUCT.md` and `docs/verification.md` with accepted behavior and evidence.
Mark Cleaning and painting complete only when all required rules and checks pass; otherwise list the missing evidence.
Commit Task 5: `docs: record cleaning and painting acceptance`.

- [ ] **Step 4: Finish the isolated development branch.**

Use finishing-a-development-branch. Preserve shared source assets and unrelated worktrees.
Report changed behavior, checks, source/browser limits, branch state, and integration options.
