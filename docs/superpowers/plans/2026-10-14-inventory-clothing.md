# Inventory and Clothing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or superpowers:subagent-driven-development to implement this plan task-by-task.
> Track steps with checkbox syntax (`- [ ]`).

**Goal:** Complete source inventory, equipment, clothing, appearance, and their native acquisition routes on current master's world.

**Architecture:** Keep item state in `ItemStore`. Add small inventory, equipment, clothing, pose, and acquisition owners.
Connect them through `StartingArea` and `GarageRuntime`. Reuse current rendering, storage, interaction, and station owners.

**Tech Stack:** Rust 2024, Rapier 0.36.0, Bevy 0.19.1, serde, Python reference tools, and browser JavaScript.

**Spec:** [Approved specification](../specs/2026-10-14-inventory-clothing-design.md).

**Status:** The user approved the written specification on 2026-10-14. This plan awaits review and an execution choice.
**Branch:** `codex/inventory-clothing`, starting from design commit `a475d86`.
**Worktree:** `C:/Users/Corashina/.codex/worktrees/inventory-clothing/the-long-drive-wasm`.
**Recommendation:** Native execution. Most tasks share item, save, input, and renderer ownership.

## Global Constraints

- Use Steam build `11140890`, Unity `2021.3.18f1`, as the behavior and content authority.
- Keep each layer playable before adding the next layer.
- Keep the existing procedural road and station placement. Do not merge the deferred source-world branch.
- Default inventory remains empty. Disabled source inventory starter groups remain disabled.
- Add no item spawn menu, guaranteed apparel drops, automatic starter grants, or invented spawn probabilities.
- Use the existing pinned dependencies. Add no generic equipment framework, event bus, or configurable slot engine.
- Do not allocate strings, clone settings, rebuild materials, or search source records in frame paths.
- Reject previous game schemas without migration. Keep browser database version separate from game schema version.
- Exclude unrelated item families, combat damage, general weapon use, full loot coverage, trailers, multiplayer, and source-world integration.
- Do not merge or push this branch during implementation.
- Apply AGENTS.md and the stop-slop skill to product text and documentation.
- Use critical tests. Use current libraries and native browser controls before adding code or dependencies.

## Review Focus

1. Cancelled holds must not select, swap, equip, or wear after pause, menu entry, or focus loss. Tasks 2 and 3.
2. A carried item beside an equipped item must not gain duplicate ownership during switching or load. Tasks 2 and 7.
3. Taking or breaking station loot must survive unloading, rebasing, revisiting, and saving before geometry returns. Tasks 4 and 5.
4. Failed skin, material, or profile preparation must retain the active scene and last committed profile. Tasks 8 and 9.
5. Player-view clipping must retain mirror body geometry. Glasses removal must clear its effects. Tasks 7 and 8.

Each owning task includes the corresponding critical assertion below.

## File ownership and interfaces

Use these boundaries. Existing files retain their current responsibilities.

| Files | Responsibility |
| --- | --- |
| `tools/reference/inventory.py`, `clothing.py`, `acquisition.py` | Recover checked source contracts and selected assets |
| `src/reference/inventory.rs`, `clothing.rs`, `acquisition.rs` | Typed reference contracts and validation |
| `src/inventory.rs` | Four slots, input progress, and source switching guards |
| `src/interaction/equipment.rs` | Right-hand ownership, interpolation, release, and item use |
| `src/clothing.rs` | Wearable membership and appearance values |
| `src/player/pose.rs` | Retained source anchors, targets, and IK state |
| `src/acquisition.rs`, `src/acquisition/boxes.rs`, `digging.rs` | Loot outcomes, box release, and buried recovery |
| `src/starting_area/acquisition.rs` | Prototype instances and transfer into existing gameplay owners |
| `src/presentation/items.rs`, `player.rs` | Dynamic item templates and intact player skin hierarchies |
| Current save modules | Stable identities, capture, validation, and candidate restoration |
| `web/player.js`, current menu, input, feedback, and storage modules | Player controls, profile transactions, and inventory HUD |

Task 1 defines `PrototypeId`, `InventoryReference`, `ClothingReference`, `AcquisitionReference`, and `ItemPrototype` in the reference modules.
`ItemPrototype` binds a stable source key to asset, root, body, collider, pickup, variant, and source behavior records.
Extend `PickupReference` with source storage, icon, right-hand, and visibility links. Keep one runtime rule owner.
Add `ReferenceManifest.inventory`, `.clothing`, and `.acquisition`. Do not accept old exports through serde defaults.

Task 2 defines `InventoryContext` with carried/equipped IDs, carried storage eligibility, player-ancestor carry guard, seated-clutch guard, and loading flag.
Use `Option<ItemId>` for IDs and `bool` for each guard.
Task 4 defines `LootOutcome` with station ID, source spawner, ordinal, selected source item, absolute pose, variant, and publication state.
Use `i64`, `SourceId`, `u32`, `Option<u32>`, `AbsolutePose`, `u32`, and `bool`, respectively.
Record unsupported source selections as empty outcomes. Store their original selected IDs.
Task 8 defines `PlayerVisuals` as retained joint, primitive, material, and camera bindings.

New signatures below belong to the named task. Use them unchanged in later tasks.

## Workspace and checks

All product edits and builds run in the worktree above.
Read the primary checkout's accepted source library and evidence. Keep that shared data read-only.
Primary checkout: `C:/Users/Corashina/Documents/Projects/the-long-drive-wasm`.
Python: `C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/tools/reference/.venv/Scripts/python.exe`.

Use `LibraryReader`, `LibraryReceipt`, `load_snapshot`, and `derive_reference` for worktree runtime derivation.
Build the receipt from the primary `web/assets/library/manifest.json` and its checked hashes.
Use its snapshot under `.cache/reference-library` and output only to this worktree's `web/assets/reference`.
Verify the output with `verify_reference(output, receipt)` before promotion.
Do not invoke the default exporter CLI if it would create or update another shared library.
Keep the bootstrap script and source reports under ignored `.cache/inventory-clothing/` in the worktree.

Before implementation, run `docker info`, then `./scripts/check.ps1` against the base revision.
Derive or copy the accepted runtime export for that base before the baseline browser check.
Run `./scripts/build.ps1`, then `node scripts/serve.mjs` with an unused `PORT`.
Check Ready, walking, seat entry, save/load, both cars, and current mirrors. Record input limits.
Use the existing `scripts/cargo-target.ps1` namespace. Do not share compiled output with another worktree.

The native test commands below use `scripts/check.ps1 -Test TARGET`.
They run existing Docker tooling and the named Cargo integration target.
Run source tests with the Python path above from this worktree.
Use actual browser gameplay for layer checks. Ignored test controls may support evidence, but never become product UI.

After each task, check `git diff --check`, commit only its changed files, and record verification in the execution ledger.
Do not repeat broad suites after a passing result unless new changes or findings require them.

---

### Task 1: Checked source contracts and content

**Files:** Create the three Python and three Rust reference modules listed above.
Modify `tools/reference/actions.py`, `export.py`, `manifest.py`, `gltf.py`, and `src/reference.rs`.
Update `tests/common/reference.json` and the existing exporter safety and runtime acceptance checks.
Create `tools/reference/tests/test_inventory_clothing.py` and `tests/inventory_reference.rs`.

**Interfaces:**
- `recover_inventory(source, manifest) -> dict` in `tools/reference/inventory.py`.
- `recover_clothing(source, manifest, output) -> dict` in `tools/reference/clothing.py`.
- `recover_acquisition(source, manifest) -> dict` in `tools/reference/acquisition.py`.
- Each Rust reference type exposes `validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError>`.
- Produces source pickup flags, prototype records, pose records, shader requirements, controls, UI labels, and icons.

- [ ] Write `test_checked_inventory_clothing_contract` and `rejects_broken_skin_or_wearable_anchor`.
  Assert `slot_count == 4`, `swap_gain == 5.0`, `equip_hold_seconds == 0.25`, and wearable type count 5.
  Assert hand interpolation factor 20, duration 1 second, and both checked character identities.
  Assert `misc4` counts `[18, 6, 38]`, source pack ID 74, cigarette ID 230, and holiday pumpkin link `level1:82`.
  Assert broken joint, renderer, variant, profile, or anchor links fail acceptance.
  Critical assertions:

  ```python
  assert inventory['slot_count'] == 4
  assert inventory['swap_gain'] == 5.0
  assert checked_misc4_spawner_counts == [18, 6, 38]
  ```
- [ ] Run the new source tests and `./scripts/check.ps1 -Test inventory_reference`. Observe failure before implementation.
- [ ] Recover effective startup fields, source category constructors, selection graphs, and reversed underground offset calls.
  Export all six wearable families, pack, Box, shovel, required debris, clips, player meshes, targets, and source profiles.
  Preserve source skin joints, inverse bind matrices, transforms, variant selection, and material slots.
  Retain native labels and key bindings: Tab, X, 1–4, and CapsLock for smoking.
  Extend exact asset acceptance whitelists and fixture contracts. Keep templates outside active scene publication.
  Record required player `_Dist`, glasses profile, shovel animation, and box release operations for their implementing tasks.
- [ ] Run the new source tests, `./scripts/check.ps1 -Test inventory_reference`, and `test_export_safety`.
  Derive the worktree export atomically, run `verify_reference`, build, and check normal garage startup.
  Expected: checked contracts, retained prior output on failed derivation, and existing gameplay ready.
- [ ] Commit: `feat: export checked inventory and clothing contracts`.

### Task 2: Inventory, minimal hand ownership, and saved transfers

**Files:** Create `src/inventory.rs`, `src/interaction/equipment.rs`, `src/player/pose.rs`, and `tests/inventory.rs`.
Modify `src/lib.rs`, `player.rs`, `interaction.rs`, `interaction/items.rs`, `interaction/carry.rs`, and `starting_area.rs`.
Modify current save schema, validation, capture, restore, and identity modules.
Modify `web/input.js`, `web/feedback.js`, `src/browser.rs`, and their current input tests.

**Interfaces:**
- `InventoryState::new(reference: &InventoryReference) -> Self`.
- `InventoryState::step(&mut self, input: FrameInput, dt: f32, context: InventoryContext) -> Option<InventoryCommand>`.
- `InventoryCommand::{Select(u8), Swap(u8)}`; state exposes `slots: [Option<ItemId>; 4]` and `selected: u8`.
- `StartingArea::apply_inventory_command(&mut self, command: InventoryCommand, reference: &ReferenceManifest, physics: &mut PhysicsWorld) -> Result<bool, ReferenceError>`.
- `EquipmentState` exposes `item(&self) -> Option<ItemId>` and owns minimal source right-hand retrieval.
- `PlayerPoseState::new(reference: &ClothingReference) -> Result<Self, ReferenceError>`.
- `PlayerPoseState::update(&mut self, player: &PlayerState, interaction: &InteractionState, items: &ItemStore, reference: &ClothingReference, dt: f32)`.
- `PlayerPoseState::anchor(&self, node: NodeId) -> Option<VehiclePose>` supplies retained hand and wearable frames.

- [ ] Write `release_selects_hold_swaps_once`, `rejected_transfer_preserves_bodies`, and `two_hands_roundtrip_without_duplicates`.
  Assert four slots, source key priority, no selection after a completed swap, and no action after input cancellation.
  Assert rejected transfers keep item location, joint, velocity, and membership unchanged.
  Assert distinct carried and equipped items survive capture/load without sharing an identity.
  Critical assertions:

  ```rust
  assert_eq!(slots_after_swap, [Some(b), Some(a), None, None]);
  assert_eq!(physical_state_after_rejection, physical_state_before);
  assert_eq!(loaded_item_ids, saved_item_ids);
  ```
- [ ] Run `./scripts/check.ps1 -Test inventory` and `node --test tests/web/input.test.mjs`. Observe failure.
- [ ] Add number-key bits 21–24 to the shared u32 input contract. Reuse current Tab and contextual X dispatch.
  Add source slot guards, atomic transfer validation, storage subtree rules, and existing source pickup restrictions.
  Prepare permanent pose anchors and minimal equipment retrieval before any slot can retrieve a hand-eligible item.
  Extend locations with `Stored(u8)`, `Equipped`, and source worn location support when Task 7 needs it.
  Keep active slot membership distinct from physical location. Remove every link on drop, throw, or deletion.
  Use schema 2 for inventory and equipment snapshots. Reject schema 1 and invalid ownership graphs.
  Capture stored food progress and removed state through item snapshots where the changed ownership path needs them.
  Reuse existing `FoodState` source identity and bite values; leave unrelated needs/sleep/weather save gaps outside this task.
  Cancel transient holds through current pause, blur, menu, and `clear_input` paths.
- [ ] Run the critical tests and `./scripts/check.ps1 -Test save_roundtrip`.
  Build and check four slots with actual eligible existing items, ordinary carry, storage, retrieval, drop, and save/load.
  Expected: a playable inventory layer and unchanged stored fluids, condition, food state, and attachments.
- [ ] Commit: `feat: add source inventory ownership and saved transfers`.

### Task 3: Explicit equipment and existing item use

**Files:** Modify `src/interaction/equipment.rs`, `carry.rs`, `consumption.rs`, `storage.rs`, `interaction.rs`, and `starting_area.rs`.
Modify `src/presentation/reference.rs`, current item transforms, and interaction feedback.
Create `tests/equipment.rs`; extend existing consumption and input checks where the changed caller needs them.

**Interfaces:**
- `EquipmentState::equip(&mut self, item: ItemId, reference: &ReferenceManifest, pose: VehiclePose) -> Result<(), ReferenceError>`.
- `EquipmentState::step(&mut self, anchor: VehiclePose, dt: f32) -> Option<(ItemId, VehiclePose)>`.
- `StartingArea::release_equipment(&mut self, reference: &ReferenceManifest, physics: &mut PhysicsWorld) -> Result<bool, ReferenceError>`.
- Consume Task 2's hand anchor and minimal retrieval owner. Retain that owner for explicit equip actions.

- [ ] Write `tab_boundary_retains_recenter`, `equipment_uses_existing_actions`, and `seat_transfer_preserves_hand_frames`.
  Assert the source short/held Tab boundaries, interpolation factor 20, final snap after one second, and `disableRHFix` behavior.
  Assert food, fluid, and flashlight actions use the active source item once.
  Assert both seat transitions and paused holds retain valid ownership and clear transient input.
  Critical assertions:

  ```rust
  assert_eq!(tab_hold_boundary, 0.25);
  assert_eq!(final_hand_pose, checked_source_offset);
  assert_eq!(item_action_count, 1);
  ```
- [ ] Run `./scripts/check.ps1 -Test equipment` before implementation. Observe failure.
- [ ] Implement source grip and release, layer changes, hand collision rules, and retained interpolation state.
  Route existing item actions through one active-item resolver. Keep loose carry and door dragging functional.
  Restore hand state before scene participation. Reset interpolation after load.
- [ ] Run equipment tests and the affected existing consumption checks.
  Build and check sustained walking, seated item use, seat exit, food, fluids, both flashlight types, and save/load.
  Expected: source equipment actions with existing carry and storage behavior intact.
- [ ] Commit: `feat: complete source equipment actions`.

### Task 4: Native station selection, prototype items, and persistence

**Files:** Create `src/acquisition.rs`, `src/starting_area/acquisition.rs`, `src/presentation/items.rs`, and `tests/acquisition.rs`.
Modify `src/world.rs`, `world/stations.rs`, `starting_area.rs`, `interaction/items.rs`, and current registration paths.
Modify save identity, schema, validation, world capture, restore, and presentation preparation.
Reuse `src/presentation/world/prototype.rs` for static item nodes and current consumable binding behavior.

**Interfaces:**
- `AcquisitionState::new(reference: &AcquisitionReference, seed: u64) -> Result<Self, ReferenceError>`.
- `AcquisitionState::activate(&mut self, station: &Station, draws: &mut AcquisitionDraw) -> Result<(), ReferenceError>`.
- `AcquisitionState::outcomes(&self) -> impl Iterator<Item = &LootOutcome>`.
- `AcquisitionDraw::new(seed: u64, station: i64) -> Self`.
- `AcquisitionDraw::int(&mut self, min: i32, max_exclusive: i32) -> i32`.
- `AcquisitionDraw::float(&mut self, min: f32, max: f32) -> f32`.
- `spawn_source_item(reference: &ReferenceManifest, area: &mut StartingArea, physics: &mut PhysicsWorld, id: ItemId, prototype: PrototypeId, pose: VehiclePose) -> Result<(), ReferenceError>`.
- `ItemStore::insert_prototype(id: ItemId, prototype: &ItemPrototype, pose: VehiclePose, body: RigidBodyHandle) -> Result<(), ReferenceError>`.
- `InteractionState::register_prototype(item: ItemId, prototype: &ItemPrototype, handles: &[ColliderHandle])`.
- Inject recorded draws in tests. Production draws use current-world deterministic state and preserve source call bounds.

- [ ] Write `source_category_constructor_matches_leaf_selection`, `unsupported_outcome_is_not_rerolled`, and `taken_loot_survives_revisit_rebase_and_save`.
  Assert category 32 uses eight unit-weight leaves; category 84 uses four unit-weight leaves.
  Assert the same supplied draws select the same source item, marker, rotation, and variant.
  Assert unsupported selections retain their original IDs as empty outcomes and consume no replacement draw.
  Assert taken and consumed items do not respawn after unloading, rebase, restoration, or revisit.
  Critical assertions:

  ```rust
  assert_eq!(misc4_effective_leaf_weights, [1; 8]);
  assert_eq!(wearable_effective_leaf_weights, [1; 4]);
  assert_eq!(draws_after_unsupported_choice, source_draw_count);
  assert_eq!(revisited_taken_instance_count, 1);
  ```
- [ ] Run `./scripts/check.ps1 -Test acquisition`. Observe failure.
- [ ] Implement complete source selection tables before checking supported content.
  Preserve hierarchy order, chance overrides, holiday guards, source markers, reversed offset calls, and selected initialization behavior.
  Extend existing instance IDs with `StationLoot` and parent-created child identities. Save source keys as decimal strings.
  Use schema 3 for generated instances and outcomes. Reconstruct prototypes before ownership and physics restoration.
  Extend current registration and item capture to cover dynamic items, not only `IdentityMap::new` startup items.
  Include prototype, variant, consumed, buried, and child-creation state in generated item snapshots.
  Keep static template materials shared where source behavior permits. Preserve stable per-instance body, item, and visual links.
  Bound active loot through current streaming windows. Retain changed objects and inventory owners in absolute coordinates.
  Prepare dynamic candidate visuals before commit; discard them on failure without changing the active scene.
- [ ] Run acquisition tests and current save world/roundtrip tests.
  Build and check actual station loot selection, pickup, slot storage, streamed removal, rebase, and reload.
  Expected: native apparel selection on current station placement, stable identities, and recorded broader population gaps.
- [ ] Commit: `feat: add source station apparel acquisition`.

### Task 5: Source boxes and one-time content release

**Files:** Create `src/acquisition/boxes.rs`.
Modify acquisition integration, collision/contact handling, attachments, dynamic visuals, audio registration, and save records.
Extend `tests/acquisition.rs` and `tests/save_roundtrip.rs`.

**Interfaces:**
- `BoxState::new(item: ItemId, prototype: PrototypeId, reference: &AcquisitionReference) -> Result<Self, ReferenceError>`.
- `BoxState::impact(&mut self, source_force: f32) -> bool` reports the source break transition once.
- `StartingArea::release_box(&mut self, item: ItemId, reference: &ReferenceManifest, physics: &mut PhysicsWorld) -> Result<bool, ReferenceError>`.
- Consumes Task 4's prototype instantiation and identities. Child outcomes use the parent instance and source ordinal.

- [ ] Write `box_contents_unlock_once_after_source_break`.
  Assert source `spawnBeforeOpen`, disabled starter-box behavior, checked impact threshold, and one release transition.
  Assert repeated contact and failed loading cannot duplicate contents, debris, audio, or saved child identities.
  Assert box and content state survives unload, revisit, and capture/load together.
  Critical assertions:

  ```rust
  assert_eq!(release_count, 1);
  assert_eq!(disabled_starter_content_count, 0);
  assert_eq!(loaded_child_ids, saved_child_ids);
  ```
- [ ] Run `./scripts/check.ps1 -Test acquisition` before implementation. Observe the new failure.
- [ ] Implement source content selection, locking, collision break, release event links, debris, and checked sounds.
  Use current attachment operations for source content locking. Add no general cargo or damage framework.
  Use schema 4 for box-specific acquisition state. Existing prototype snapshots already carry child items.
- [ ] Run acquisition and save roundtrip checks; build and break a real source box through an ordinary physical interaction.
  Expected: one source release, valid pickup of released content, and preserved state after save/load.
- [ ] Commit: `feat: add source box acquisition and release`.

### Task 6: Source shovel recovery

**Files:** Create `src/acquisition/digging.rs`.
Modify equipment motion, acquisition integration, source pose sampling, aim restrictions, physics queries, and acquisition audio/particles.
Extend `tests/acquisition.rs` and `tests/equipment.rs`.

**Interfaces:**
- `DiggingState::new(reference: &AcquisitionReference) -> Result<Self, ReferenceError>`.
- `DiggingState::step(&mut self, input: FrameInput, pose: VehiclePose, dt: f32, items: &mut ItemStore, physics: &mut PhysicsWorld)`.
- `StartingArea::is_buried(&self, item: ItemId, physics: &PhysicsWorld) -> Result<bool, ReferenceError>`.
- Consumes source shovel prototype, hand frame, attack motion, contact mask, and existing source collision geometry.

- [ ] Write `buried_pickup_requires_source_recovery` and `repeated_shovel_contacts_advance_once`.
  Assert pickup rejection before recovery, source vertical progress, restored dynamics after ground clearance, and one-second recent-recovery behavior.
  Assert several contacts in one update do not multiply progress or damage another item.
  Assert saved partial recovery retains its absolute pose and buried state after load.
  Critical assertions:

  ```rust
  assert!(!pickup_allowed_before_ground_clearance);
  assert_eq!(progress_after_repeated_contacts, one_source_update);
  assert_eq!(recent_recovery_seconds, 1.0);
  ```
- [ ] Run acquisition and equipment checks before implementation. Observe the new failures.
- [ ] Implement the source shovel motion and contact recovery branch with retained query storage.
  Reuse checked normalized motion tracks and existing Bevy facilities where they cover the required source motion.
  Recover missing required bindings from original records; do not invent a swing or replace it with a static pose.
  Preserve source ground tests and collider exclusions. Limit outcomes to recovery, motion, particles, and source contact sounds.
- [ ] Run acquisition/equipment tests; build, find the source shovel through the checked category graph, and recover buried apparel.
  Expected: source burial restrictions, usable recovery, unchanged general damage behavior, and working save/load.
- [ ] Commit: `feat: add source buried wearable recovery`.

### Task 7: Wearables, cigarettes, glasses, and holiday startup

**Files:** Create `src/clothing.rs` and `tests/clothing.rs`.
Modify interaction dispatch, consumption, acquisition integration, startup preparation, item visibility, audio, and current camera effects owners.
Modify save contracts and `web/input.js`; update current input and source-reference checks.

**Interfaces:**
- `WearableState::new() -> Self`; `WearableState::items(&self) -> &[Option<ItemId>; 5]`.
- `StartingArea::wear_item(&mut self, item: ItemId, reference: &ReferenceManifest, physics: &mut PhysicsWorld) -> Result<bool, ReferenceError>`.
- `StartingArea::remove_wearable(&mut self, reference: &ReferenceManifest, physics: &mut PhysicsWorld) -> Result<bool, ReferenceError>`.
- `StartingArea::take_cigarette(&mut self, pack: ItemId, reference: &ReferenceManifest, physics: &mut PhysicsWorld) -> Result<bool, ReferenceError>`.
- `ClothingReference::halloween_active(month: u8, day: u8) -> bool` implements the checked inclusive source dates.
- Consumes pose anchors, source prototypes, parent-child IDs, and existing survival consumption.

- [ ] Write `wear_rejects_occupied_type_without_transfer`, `x_dispatches_one_context_action`, and `pack_creates_one_cigarette`.
  Assert distinct hat/cap/helmet slots, source center-object transforms, disabled worn colliders, and restored removal physics.
  Assert edible-first removal, no duplicate assembly/removal action, and no invalid inventory or hand references after wearing.
  Assert pack count decrements once and an occupied mask slot leaves the source cigarette world result.
  Assert CapsLock consumption removes exhausted references once.
  Assert Halloween boundaries: October 14 false, October 15 true, November 15 true, November 16 false.
  Assert loading preserves a saved pumpkin instead of applying a new date decision.
  Critical assertions:

  ```rust
  assert_eq!(occupied_type_after_rejected_wear, occupied_type_before);
  assert_eq!(x_action_count, 1);
  assert_eq!(new_cigarette_count, 1);
  assert!(!ClothingReference::halloween_active(10, 14));
  assert!(ClothingReference::halloween_active(10, 15));
  ```
- [ ] Run `./scripts/check.ps1 -Test clothing` and current web input checks before implementation. Observe failure.
- [ ] Implement source Wear/TakeOff transitions and action priority. Add smoking at input bit 25; retain contextual X.
  Apply source glasses profiles, view guards, enable setting, and flare flag through current owners.
  Register checked startup packs under the accepted two-car policy. Add only the source holiday pumpkin.
  Route cigarette consumption through survival, audio, and retained source particles.
  Use schema 5 for wearable snapshots. Preserve existing pack snapshots and restore instances before ownership and participation.
- [ ] Run clothing, acquisition, and save roundtrip checks.
  Build and check all six wearable families, removal, occupied slots, source variants, cigarette use, glasses effects, and save/load.
  Expected: no input duplication, valid two-hand states, and cleared main-view effects after glasses removal.
- [ ] Commit: `feat: add source wearable actions and effects`.

### Task 8: Intact player skins, source pose, and camera visibility

**Files:** Create `src/presentation/player.rs` and `tests/player_pose.rs`.
Extend `src/player/pose.rs`, `src/presentation/items.rs`, source materials, caster/motion paths, cameras, and save preparation.
Modify `web/assets/shaders/source_mesh.wgsl`, `source_surface.wgsl`, `source_caster.wgsl`, `source_caster_math.wgsl`, and `source_motion_objects.wgsl` as required.
Use existing source shader and render graph owners. Add no second renderer.

**Interfaces:**
- `PlayerVisuals::prepare(reference: &ReferenceManifest, world: &mut World) -> Result<Self, SaveError>`.
- `PlayerVisuals::apply_pose(&mut self, pose: &PlayerPoseState, world: &mut World)`.
- `PlayerVisuals::apply_appearance(&mut self, state: &AppearanceState, world: &mut World)`; Task 9 supplies user commands.
- `AppearanceState::new(reference: &ClothingReference) -> Self` creates source defaults required for first model publication.
- Extend Task 2's pose owner with checked leg, hand, pelvis, and IK updates without changing its existing anchor API.

- [ ] Write `source_pose_preserves_grip_and_ik_chain` and `non_optional_segments_ignore_disabled_visibility_field`.
  Assert source pole/stretch/force-end behavior, retained joint ordering, and hand/wearable anchors across walking and seats.
  Assert non-optional segments remain visible when their serialized enabled flag is false.
  Define browser checks for main-view clipping, animated mirrors/shadows, and failed skin preparation retaining the active scene.
  Critical assertions:

  ```rust
  assert_eq!(source_joint_order, bound_joint_order);
  assert!(non_optional_segment_visible);
  assert_eq!(live_scene_after_failed_prepare, live_scene_before);
  ```
- [ ] Run `./scripts/check.ps1 -Test player_pose` before implementation. Observe failure.
- [ ] Preserve full glTF skin entities and inverse bind matrices; do not flatten player meshes through the static `Recipe`.
  Use existing Bevy skinning and source material/caster bindings for every selected joint and primitive.
  `source_mesh.wgsl` already imports Bevy skinning. Retain that path and check specialization attributes and definitions before changing it.
  Port required source IK and `_Dist` behavior with per-view parameters.
  Support skinning in visible, depth, shadow, mirror, and motion paths. Use correct skinned bounds and source renderer flags.
  Keep worn geometry visible in source-supported external views and casters.
  Include player resources in candidate preparation, readiness, cancellation, disposal, and restored pose publication.
- [ ] Run pose and existing player/mirror checks; build and exercise both models with actual GPU submissions.
  Expected: correct joints, checked shader variants, no main/mirror visibility leak, and intact active scene after preparation failure.
- [ ] Commit: `feat: render source player models and poses`.

### Task 9: Appearance controls, inventory HUD, and atomic profiles

**Files:** Create `web/player.js` and `tests/web/player.test.mjs`.
Modify current menu, input, feedback, layout, storage, save-session, HTML, and CSS owners.
Modify `src/clothing.rs`, `browser.rs`, `app.rs`, and current snapshot/view handling.
Extend `tests/clothing.rs`, `tests/save_roundtrip.rs`, and `tests/web/save-storage.test.mjs`.

**Interfaces:**
- `AppearanceState::set_segment(&mut self, model: u8, segment: usize, hsv: [f32; 3], enabled: bool, reference: &ClothingReference) -> Result<(), ReferenceError>`.
- `AppearanceState::reset_model(&mut self, model: u8, reference: &ClothingReference) -> Result<(), ReferenceError>`.
- `createPlayerScreen({menu, profileStorage, requestPlayer, clearInput})` in `web/player.js`.
- Extend existing storage with `readProfile()` and `writeProfile(payload, {signal} = {})`; reuse its database, transactions, and checksum.
- `requestPlayer(command)` queues checked feature commands through the current app boundary; it never mutates simulation from the DOM.

- [ ] Write `profile_write_failure_keeps_committed_state` and `model_settings_survive_switch_reset_and_load`.
  Assert source HSV order with `HSV(0, 1, 1) == RGB(1, 0, 0)` and source default reset for each model.
  Assert optional-only visibility edits, invalid values rejected, separate model values retained, and no shared world-material recoloring.
  Assert failed/aborted profile transactions retain the previous committed value and active scene.
  Assert New uses the profile, Load uses snapshot appearance, and Load does not overwrite the profile.
  Assert menu input cannot trigger held gameplay actions and four HUD slots update without DOM reconstruction.
  Critical assertions:

  ```javascript
  assert.deepEqual(committedProfileAfterFailure, committedProfileBefore);
  assert.deepEqual(profileAfterLoad, profileBeforeLoad);
  assert.equal(hudSlotNodeCount, 4);
  ```
- [ ] Run clothing/save roundtrip checks and `node --test tests/web/player.test.mjs tests/web/save-storage.test.mjs`. Observe failure.
- [ ] Add Player entry, checked model/segment controls, permitted toggles, native range inputs, Reset, and source preview.
  Add the source glasses enable control through view settings and source labels.
  Retain four HUD nodes, source icons, selection, swap progress, Equip/Put on prompts, and acquisition feedback.
  Use schema 6 for appearance and glasses setting snapshots. Keep the existing browser database version.
  Add a distinct profile key and checked commit operation. Do not add migrations or a new database.
  Apply appearance only through prepared material bindings; build or change instances only after source selection changes.
- [ ] Run critical profile/UI checks and save roundtrip checks; build and test both models, all segments, reset, reload, and profile failure.
  Expected: native appearance values, usable current menu flow, persistent profiles, and unchanged committed state on failure.
- [ ] Commit: `feat: add player appearance controls and saved profiles`.

### Task 10: Whole-feature verification and completion evidence

**Files:** Update `docs/roadmap.md`, `docs/verification.md`, this plan, and the source audit records.
Keep browser captures, draw traces, GPU receipts, and memory measurements under ignored `.cache/inventory-clothing/`.
Change product files only to repair findings from these checks.

**Interfaces:** Consume the completed owners and current lifecycle/status hooks. Add no product debug controls.

- [ ] Check all spec requirements against task results and ledger rulings. Run `./scripts/check.ps1` and `./scripts/build.ps1`.
  Run affected Python source tests and worktree `verify_reference` against the accepted receipt.
  Run `node --check` on each changed JavaScript module and check `git diff --check`.
  Expected: native/web checks, release WASM package, source export validation, syntax, and whitespace pass.
- [ ] Run assembled gameplay through walking, crouch, seat entry, sustained driving, inventory, fluids, food, flashlights, and door dragging.
  Check native acquisition, box release, digging, all wearables, both models, appearance, and source item use.
  Save/reload stored, carried, equipped, worn, buried, consumed, broken, and dormant station states.
  Check pause, blur, resize, New, Load, unavailable assets, and candidate failure without destroying the active scene.
- [ ] Check main, external, mirror, shadow, and motion submissions for both skinned models and glasses variants.
  Check source audio on a listening device; label unavailable audible evidence.
  Measure retained meshes, materials, joints, memory, and item identities through repeated equip, wear, model changes, and scene replacements.
  Repeat station unload/revisit and origin shifts. Check stable active resource counts and preserved changed objects.
- [ ] Compare supplied source draw sequences and matched running source inputs/views for the feature.
  Report unsupported population outcomes and current-world random-stream limitations.
  Record trusted-input and source-comparison limits. Build success and Ready do not establish parity.
- [ ] Request the execution method's fresh whole-branch review. Repair material findings and repeat only affected checks.
  Keep the worktree, evidence, and branch. Update the roadmap with verified behavior and explicit limits.
  Expected: no unresolved required work within the approved scope and no unsupported completion or parity claim.
- [ ] Commit: `docs: record inventory and clothing verification`.

## Layer checkpoints and schema changes

| Checkpoint | Tasks | Playable result | Save schema |
| --- | --- | --- | --- |
| Source | 1 | Existing game runs with checked new templates | 1 |
| Inventory | 2 | Four slots, source retrieval, ownership, and reload | 2 |
| Equipment | 3 | Explicit equip, release, item use, and seat transitions | 2 |
| Acquisition | 4–6 | Native station selection, boxes, and buried recovery | 3, then 4 |
| Wearables | 7 | Wear/remove, source effects, smoking, and holiday startup | 5 |
| Player | 8–9 | Both skinned models, appearance controls, profiles, and reload | 5, then 6 |
| Acceptance | 10 | Assembled evidence and whole-branch review | 6 |

Increment the game schema whenever a task changes serialized shape beyond the listed contract.
Reject superseded layer schemas. Do not preserve compatibility with intermediate branch saves.
Do not increase the browser database version for profile keys or game payload changes.

## Plan self-review

Source contracts map to Task 1. Inventory and equipment map to Tasks 2–3.
Native acquisition maps to Tasks 4–7. Player pose and rendering map to Task 8.
Appearance, UI, and profile storage map to Task 9. Assembled acceptance maps to Task 10.
Save, source ownership, and candidate readiness travel with each gameplay task.
Review Focus assertions appear in their owning tasks.
All new cross-task names and return types above have one named owner.
Execution remains pending the user's plan review and method choice.
