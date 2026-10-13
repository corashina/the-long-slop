# Portable Flashlights Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore source flashlight switching, moving beams, and vehicle storage locking without battery depletion.

**Architecture:** Checked source records feed portable item bindings and the existing lamp switch state.
A focused storage module manages body and collider ownership. Existing imported nodes drive beam poses and shared rendering resources.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, Rapier 0.36.0, serde, Python, WASM, WebGL2, and the existing Docker toolchain.
**Spec:** [Approved flashlight design](../specs/2026-10-13-portable-flashlights-design.md).
**Status:** Complete on 2026-10-13. All task reviews and the final scoped review passed. Storage correction: d85cf4f.
**Execution:** Preserve the user's selected subagent-driven method. Complete each task and review before starting the next.
**Base:** codex/equipment at c7ddede. Plate implementation is complete at 542be78.

## Global Constraints

- Use installed Steam build 11140890 and Unity 2021.3.18f1.
- Keep installed files and the primary source library unchanged.
- Keep source records, models, textures, cookies, and browser probes outside Git.
- Instantiate the older source-enabled garage flashlight at its authored pose.
- Keep the inactive newer scene object inactive. Do not add a second garage flashlight.
- Recover newer prefab and inventory links as provenance. Do not invent an acquisition route.
- This feature does not add inventory, loot, purchase, or cheat interfaces.
- Add no battery, charge, consumption, or recharge behavior.
- Use the existing switch feedback and controls. Add no new control key.
- Use the existing light uniform, cookie atlas, source shadow atlas, and spotlight rendering path.
- Require a fresh export. Add no old-manifest reader or compatibility default.
- Restrict new locking integration to eligible flashlight items. Other cargo behavior remains outside this feature.
- Preserve all three mirror captures, their feedback guard, and their bounded targets.
- Full deferred rendering replacement remains outside this feature.
- Do not add or run automated tests unless the user changes the current instruction.
- Use release builds, source review, and visible browser checks. State their limits.
- Do not claim matched Unity appearance, exact native motion, or unimplemented acquisition behavior.
- Follow AGENTS.md and stop-slop for documentation and UI text.

## Review Focus

1. Inactive newer models must remain unavailable through normal gameplay. Task1 records activation; Task4 checks visible startup ownership.
2. Held LMB must toggle once and suppress competing world-target LMB use. Task2 owns dispatch; Task4 checks switch precedence.
3. Rejected pickup from storage must retain collider, body, vehicle mass, and lamp state. Task2 owns transactions; Task4 observes rejection.
4. Missing slots or source usable values above0.1 must release storage ownership without changing the switch state. Task2 owns eligibility.
5. Pause, restart, and origin changes must retain coherent ownership and bounded resources. Tasks3 and4 cover the assembled lifecycle.

These checks use code review and labelled browser observations. No automated assertions or test suites are required under the user override.

---

### Task 1: Recover models, startup, controls, and storage records

**Files:** Create tools/reference/flashlights.py and src/reference/flashlights.rs.
Modify tools/reference/{export,manifest,lighting,gltf}.py and src/reference.rs and src/reference/lighting.rs.
Apply the required switch-kind rename in src/lighting.rs and existing schema consumers that use the obsolete symbol.
Update existing fixtures only when required by the new contract or symbol rename. Add no test cases and run no tests.

**Interfaces:**
- Add required ReferenceManifest.flashlights: FlashlightRecord, with deny_unknown_fields and no serde default.
- FlashlightRecord contains models: Vec<FlashlightModelReference>, instances: Vec<FlashlightInstanceReference>, and storage: StorageLockRecord.
- FlashlightModelReference records source prefab root, source controller, checked scene root node, geometry/material provenance, and source light links.
- FlashlightInstanceReference contains model: SourceId, item: ItemId, node: NodeId, pickup: SourceId, save: SourceId, controller: LightControlId, startup_active: bool, and colliders: Vec<ColliderId>.
- StorageLockRecord contains settings_source: SourceId, default_setting: u8, mount_mask: u32, drop_delay_seconds: f32, scan_interval_seconds: f32, and volumes: Vec<StorageVolumeReference>.
- StorageVolumeReference contains source: SourceId, node: NodeId, owner: ItemId, collider: ColliderId, storage_type: u8, only_manual: bool, no_physics_lock: bool, owner_pickup: Option<SourceId>, dont_lock_if_owner_picked_up: bool, slots: Vec<PartSlotId>, and usables: Vec<SourceId>.
- Retain source collider geometry and transforms through existing NodeRecord and ColliderRecord. Do not duplicate world bounds in this contract.
- Rename LightControllerKind::House to Switch and serialized kind house to switch. Rename private house-only switch helper names accordingly.
- Both stationary switches and handheld controllers use Switch. Portable ownership remains in FlashlightInstanceReference, rather than another controller implementation.
- recover_flashlights(source, roots, pointer_node, item_nodes, controllers, colliders, output, part_slots) -> dict produces the manifest record and checked evidence.
- prepare_flashlight_roots(source, roots) -> None adds the two existing startup scene roots to the items asset before lighting recovery.
- Reuse existing SourceId, ItemId, NodeId, LightControlId, ColliderId, PartSlotId, and resource receipts.

- [x] **Step 1: Recover checked selected source links and geometry.**

Use .cache/equipment-source/flashlights/{source,details,startup,mounts,settings}.json as researched identities, then re-read selected checked records.
Older scene: GameObject 12118, Transform 32123, controller 74834, pickup 69340, body 50144, save 74314, mass 73773, Light 56881.
Newer inactive scene: GameObject 12119, Transform 32124, controller 74835, pickup 69341, body 50143, save 74315, mass 73774, Light 56882.
Both live under StartSpawnItems117. Terrain64898 enables117; preserve each child's original activation.
Prefab roots are sharedassets1.assets:28750 older and28749 newer. Their controllers are148780 and148779.
Recover source masks, cookies, renderer links, source materials, and pickup flags. Exclude PumpKin and player inventory prototypes from startup instances.

- [x] **Step 2: Add the typed contract and portable switch binding.**

Require exactly one source-enabled older startup item and retain the newer source object inactive.
Bind both source model records without forcing inactive actors visible, collidable, or pickable.
Keep body 1 kg, drag0, angular drag0.05, gravity=true, and kinematic=false from their authored records.
Both controllers start OFF, with randomStart=false, null electronics, null SingleLight, and empty break triggers.
Recover old beam range 12/intensity 0.8500000238418579/cones 90 and8/warm RGB from the spec.
Recover new beam range 15/intensity 1.5/cones 80 and62.188602447509766/white RGB from the spec.
Validate links, source startup evidence, atlas capacities, required cookies, and same-item pickup/save/controller identity.
Scope managed expression checks to their named source class and method. Do not repeat assembly-wide semantic checks.

- [x] **Step 3: Recover source storage bounds, policy, and scheduler.**

Volumes are65838,65854,67560,79410,77806,78464,79301,69971 in level1, linked to parentTosave73405.
Resolve their owners through source pointers. Do not hardcode runtime item numbers.
Settings65886 and dropdown70023 establish fresh-default policy 2 through useDefInt=true and defInt=2.
Policy2 enables glove storage, trunk, and engine bay item storage, and disables passenger-room item storage.
Mount mask 8388609; drop delay1 second; non-basket update group1 uses strictly greater than1.5 seconds between scans.
All eight selected volumes are non-basket. Preserve the source two-scan membership rule and source usable threshold0.1.
Recover slot, usable, owner-pickup, active, onlyManual, and noPhysicsLock rules.
The first six volumes have no slot or usable restrictions. Trunk uses source slot 77859; engine bay uses77842.
All eight susable arrays are empty. Resolve the occupied hood/trunk item and reuse MechanismState::value(item, false).
Recover the initialization sequence: two yielded frames, followed by three QuickFreeze passes; only the first pass prescans.

- [x] **Step 4: Derive a fresh export and build a matching package.**

Use the accepted LibraryReader and existing atomic promotion path. Do not repeat whole-library owner audits.
Run the necessary production export and ./scripts/build.ps1. Run no test suites or test commands.
Confirm the manifest contains required records and one eligible startup light. Record code, manifest, model, cookie, and package hashes.
Inspect source records and loading diagnostics. Fix source-ID failures before committing.

- [x] **Step 5: Commit recovered source bindings.**

Commit: feat: recover source portable flashlight bindings.

### Task 2: Integrate held use and source storage ownership

**Files:** Create src/interaction/storage.rs.
Modify src/interaction.rs, src/interaction/{carry,feedback,items,labels,parts,wheels}.rs, src/starting_area.rs, and src/lighting.rs where required.
Keep source data in Task1 contracts and presentation in Task3.

**Interfaces:**
- StorageLocks::new(reference: &ReferenceManifest) -> Result<Self, ReferenceError>.
- StorageLocks::bind(&mut self, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<(), ReferenceError>.
- StorageLocks::advance(&mut self, items: &mut ItemStore, physics: &mut PhysicsWorld, parts: &PartAttachments, mechanisms: &MechanismState, dt: f32) -> Result<bool, ReferenceError> returns whether ownership or mass changed.
- StorageLocks::initialize_frame(&mut self, items: &mut ItemStore, physics: &mut PhysicsWorld, parts: &PartAttachments, mechanisms: &MechanismState, dt: f32) -> Result<bool, ReferenceError> returns ownership changes during initialization. StorageLocks::initialization_ready() -> bool gates readiness after the source frame sequence.
- StorageLocks::unlock(&mut self, item: ItemId, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<(), ReferenceError>.
- StorageLocks::owns_collider(&self, collider: ColliderId) -> bool; contains(&self, item: ItemId) -> bool; loose_body(&self, item: ItemId) -> Option<RigidBodyHandle>; rebase(&mut self, shift: [f32;3]).
- InteractionState.storage owns StorageLocks and a retained item-to-LightControlId lookup.
- InteractionState.held_light_control(&self) -> Option<LightControlId> reads that lookup.
- InteractionState.pickup_stored_item(item: ItemId, point: [f32;3], player: &PlayerState, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Result<ItemId, ReferenceError> preflights carry before unlock.
- Reuse LightingState::use_control(&LightingRecord, LightControlId) and InteractionAction::LightControl.

- [x] **Step 1: Bind retained bodies and unique collider ownership.**

Use existing item bodies. Keep one retained independent body per flashlight, disabled while stored or source-inactive.
Mark storage-owned shapes so StartingArea.sync_compound cannot overwrite their live parents.
Store source-relative poses and retained shape offsets once. Prepare overlap and previous-membership storage once; allocate nothing in unchanged frame paths.
Use ItemLocation::Attached(owner) for storage ownership. Keep storage membership separate from plate membership.

- [x] **Step 2: Route source world and held switch presses.**

World use or permitted LMB presses call the existing switch action.
Held flashlight LMB calls its pickup.lampa controller and consumes competing world LMB use.
Preserve E/use routing, reach, layers, seated rules, and existing carry controls.
No new mount key or flashlight socket is needed. Do not release carry on a switch press.
Keep inactive instances unavailable to normal aim and pickup.

- [x] **Step 3: Implement source overlap, delay, restrictions, and locking.**

Use existing Rapier oriented-shape queries with source bounds and mount mask.
Advance source elapsed time only in unpaused active runtime updates. Use strict group1 scan interval>1.5 seconds and elapsed drop time>=1 second.
Track preceding and current overlap membership. Exclude held, locked, source-inactive, restricted, and same-root candidates.
Apply policy 2 and source slot/usable restrictions. Missing required parts or GetValue()>0.1 makes storage unavailable.
Lock at the current pose. Disable the loose body, transfer shapes to the owner, and retain the current switch state.
Unstore when source eligibility becomes false. Preserve pose and inherited point/angular velocity.
Preflight stored-item acquisition against its retained dynamic body before changing ownership or carry.
A rejected acquisition retains membership, shape parent, body state, mass, and switch state.

- [x] **Step 4: Integrate mass, timing, and origin transport.**

Add portable items once to the existing current-body ownership mass accounting.
Their source startup mass is outside the car. Do not subtract it from the car's initial base mass.
Subtract only source-initial car-owned members when deriving that base. Add1 kg when the active flashlight becomes car-owned; remove it on unlock.
Apply mass changes before carry acquisition. Do not add full mass through both collider and body.
Use existing update phases for initialization and storage scans. Keep rendering readiness separate from simulated elapsed time.
Rebase retained bodies and world bounds once; keep local storage offsets unchanged.
Run source-reviewed visible old-model pickup, switch, drop, storage lock, unlock, rejection, and origin checks. Run no automated tests.

- [x] **Step 5: Build, self-review, and commit native integration.**

Run ./scripts/build.ps1 and record its result. Record ownership and switch observations in an ignored labelled probe.
Commit: feat: support carried flashlight controls and source storage locking.

### Task 3: Bind moving beams and shared renderer preparation

**Files:** Modify src/presentation/{lighting,reference,materials}.rs and src/app.rs only where required. Complete the existing loading hook in src/interaction/storage.rs and src/starting_area.rs. Correct per-node identity in tools/reference/gltf.py and validate actual exported flashlight geometry.
Use existing shaders and source shadow implementation. Modify shader code only for a demonstrated integration defect.

**Interfaces:** Consume Task1 instance/controller/light records and Task2 live item ownership.
Imported ItemVisual roots and their light children remain the pose authority.
Current SourceLighting, SourceMaterials, LightingGpu, and mirror warmup collection remain the renderer interfaces.
No separate flashlight renderer or camera-owned beam state is introduced.

- [x] **Step 1: Connect authored light nodes and material choices.**

Bind active and source-inactive source nodes without changing startup eligibility.
Read the item's live imported hierarchy after physics, storage, and origin updates.
Use source spot range, full/inner cone, color, cookie, mask, hard-shadow parameters, and on/off materials.
Do not apply a second source-axis conversion or tie an independent light to the player camera.
Reuse the fixed source atlas and capacity checks: MAX_LIGHTS 34, SHADOW_SLOTS 100, atlas 4000, tile 400.

- [x] **Step 2: Warm all required layouts and material choices during loading.**

Prepare both models' existing source materials, cookies, spot shadow variants, and normal/mirror material variants before readiness.
Preserve authored material binding order and late request sealing from the plate implementation.
Source-inactive model assets can warm without making that instance available in gameplay.
Switch, carry, drop, lock, unlock, and rebase must select retained assets and warmed keys.
Keep the existing three capture targets and owner feedback guard.

- [x] **Step 3: Connect initialization, pause, and restart.**

Call the storage initialization frame hook in an existing loading/frame phase after physical bindings exist.
Reset native portable ownership and OFF state through the existing game restart.
Wait for current source lighting, material, plate, and mirror readiness before input or physics resumes.
Use existing mirror epochs. Add no unused asynchronous flashlight layer.
Paused frames retain switch state and beam pose; source storage elapsed time stops.

- [x] **Step 4: Build and perform labelled source beam checks.**

Use the actual imported older model and a clearly labelled inactive newer-model diagnostic.
Observe beam color, cone, cookie, shadows, pose, material switching, and movement across owner changes.
Observe eligible mirror lighting and zero browser/GL errors. State any clipping or trusted-input limit.
Restore paired production source and package after any ignored probe build.
Commit: feat: render source portable flashlight beams through shared lighting.

### Task 4: Complete visible lifecycle acceptance and review

**Files:** Update docs/verification.md, tools/reference/README.md, docs/superpowers/plans/2026-10-11-rendering-completion.md, and this plan.
Fix product lifecycle code only for a demonstrated defect. Keep harnesses, source extracts, receipts, and screenshots outside Git.

**Interfaces:** Consume all prior tasks, the game loading lifecycle, and current resource observers.
Use held UI stages and read-only DOM observations through parent CUA. Subagents provide routes; the parent performs visible captures.

- [x] **Step 1: Build one matching final production revision.**

Run ./scripts/build.ps1. Run no automated tests or assertions.
Record code, WASM, JS, manifest, library, model, cookie, and source shader receipts together.
Keep production restore checks exact after every diagnostic build.

- [x] **Step 2: Observe critical interaction and source states.**

Check source-enabled older startup OFF and inactive newer startup.
Check world use, held LMB, one-toggle precedence, carry rotation, drop, source storage delay, allowed/disallowed volumes, lock, and unlock.
Check rejected acquisition with prior owner/collider/body/mass retained.
Check a source storage restriction change, origin shift, pause, held resume, and restart to source startup state.
Check newer source beam behavior only in a labelled diagnostic; do not claim its gameplay acquisition.
Use separate non-overlapping held-stage buttons. Label any paused/frozen physics or scripted placement.

- [x] **Step 3: Compare matching lifecycle resource states.**

Observe three transition/restart cycles and a restored render size.
Compare model, material, image, light, mesh, buffer, shadow storage, capture target, and pipeline counts after current readiness.
Require bounded retained resources and no unknown playing-state pipeline keys.
Keep exactly three owned mirror capture targets. Record requested storage separately from driver memory.
Check the real paused flag for pause/resume evidence. Do not use missed interval samples as positive evidence.

- [x] **Step 4: Restore production and complete documentation.**

Visible production must reach readiness with existing controls enabled and zero browser/GL errors.
Record screenshots, complete receipt hashes, source settings, resource counts, skipped automated tests, and explicit evidence limits.
Complete task reviews and one whole-branch review under the preserved execution method.
Commit: docs: verify source portable flashlight lifecycle and rendering.
Report flashlight completion only after the accepted current revision is concrete and reviewable.
Keep codex/equipment and its attached worktree. Do not push or merge without the user's integration instruction.
