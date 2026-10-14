# Inventory and clothing

Date: 2026-10-14, Europe/Warsaw.
Status: The user approved this written specification on 2026-10-14. Implementation awaits plan review and an execution choice.
Base: local master `36ba94b05ff6679af733aa2f36bd014bc23b92f6`.
Worktree: `C:/Users/Corashina/.codex/worktrees/inventory-clothing/the-long-drive-wasm`.
Branch: `codex/inventory-clothing`.

## Goal and scope

Complete the `Inventory and clothing` row in [the roadmap](../../roadmap.md).
Use Steam build `11140890`, Unity `2021.3.18f1`, as the behavior and content authority.
Include four inventory slots, right-hand equipment, source wearables, both player models, and appearance controls.
Keep each layer playable before adding the next layer.
Keep item use, movement, seat entry, fleet state, lighting, mirrors, audio, and scene replacement connected.
Use the existing physics, source asset library, source material renderer, browser menus, and save owners.

Use native acquisition from source station loot, buried loot, cigarette packs, and holiday pumpkin activation.
Include source box release and shovel recovery because native acquisition needs them.
Limit those additions to acquisition. General combat and other tool actions remain separate features.
Default inventory remains empty. Disabled source inventory starter groups remain disabled.
Keep the existing procedural road and station placement. Do not merge the deferred source-world branch.

Exclude unrelated item families, combat damage, general weapon use, full loot coverage, trailers, multiplayer, and source-world integration.
Exclude game save slots, autosave, control rebinding, VR, and unrelated settings.
Do not merge or push this branch during implementation.

## Source authority

The [source audit](../../research/2026-10-14-inventory-clothing-audit.md) records the initial findings.
The [acquisition audit](../../research/2026-10-14-inventory-clothing-acquisition.md) records fresh decompilation and native acquisition routes.
The retained typed records and accepted library receipts bind source identities to checked files.
Recheck selected hashes when exporting runtime assets.

Assembly SHA-256: `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
Decompiled text SHA-256: `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee`.
Fresh decompilation of the installed assembly confirmed both hashes and reported no diagnostics.
The extended content audit read 1,880 checked records before the separate home hierarchy scan.

| Contract | Authority |
| --- | --- |
| Player and startup inventory | `level1:80125`, `fpscontroller` |
| Inventory operations | `InventoryStuff`, `InvSwitchTo`, `InvSwapBegin`, `InvSwapTo`, `InvStore`, `InvUnStore`, `Pickup`, `Drop` |
| Storage and hand eligibility | `pickupable.canInventory`, `canHoldRightHand`, `Stored`, `UnStored` |
| Equip motion | `RightHandGrip`, `RightHandGripAnimation`, `RightHandRelease` |
| Wearable operations | `wearableType`, `wearable.Wear`, `fpscontroller.Wear`, `TakeOff` |
| Glasses effects | `szbglasscript`, its selected `PP` profile, `disableLensFlare` |
| Player appearance | `level1:70544`, `characterscript`, `playermodeloutfitscript`, `settingsplayercustomscript` |
| Source model segments | `level1:71215`, `level1:67763` |
| Player pose | Source player targets, `MoveNearestHand`, `MoveNearestLeg`, `MoveLegsWalk`, and `IKscript` |
| Camera visibility | `setsubshaderplayercamscript`, `makeinvisible`, `SetDistance`, `ReSetDistance` |
| Local source home | `level1:21400`, building `level1:70166` |
| Station acquisition | `buildingscript`, `newRandomStuffSpawnScript`, `undergroundSpawnScript`, and checked station child records |
| Box contents and release | `spawnInBoxScript`, source Box, its `mountStuff`, `breakablescript`, and release event links |
| Buried recovery | `pickupable.CanPickup`, `tosaveitemscript.Bury`, `IsBuried`, `UpdBuried`, `UnBury`, and source shovel handling |
| Packs and holidays | `cigiscript`, `usePickedUpStuff`, `TerrainGenerationSettings.StartDateItems`, and `mainscript.CheckHoliday` |

The home building has no `newRandomStuff` entries and no discovered wearable spawner beneath its root.
The source player has four empty slots, selected slot zero, and inventory swap gain 5.
All three serialized starter-group flags are false.
Source starter prototype links do not authorize default spawning.

## Content contract

Export the checked player hierarchy, both character meshes, skeletons, targets, segment materials, and wearable anchors.
Export item eligibility flags, inventory icons, storage visibility links, right-hand offsets, and hand grip links.
Preserve source node transforms, signed scale, mesh skinning, renderer masks, and material properties.
Use the current coordinate conversion once.

Include these six wearable prefab families:

| Source item | Database item ID | Wearable component | Type |
| --- | --- | --- | --- |
| Sunglasses | 217 | `sharedassets1.assets:146739` | Glasses |
| Hat | 218 | `sharedassets1.assets:147668` | Hat |
| Cap, `buntetoSapka` | 221 | `sharedassets1.assets:147667` | Cap |
| Cone, `bolya` | 245 | `sharedassets1.assets:131348` | Hat |
| Pumpkin, `PumpKin` | 92 | `sharedassets1.assets:148817` | Helmet |
| Cigarette, `Cigi` | 230 | `sharedassets1.assets:142834` | Mask |

Treat enum names as type identifiers. Do not invent a mask or helmet asset from their English names.
The cone and cigarette cannot enter inventory slots. They can use their source wearable actions.
The sunglasses, hat, cap, and pumpkin can enter inventory slots.
These selected wearable pickups do not support right-hand equipment.
Equipment uses existing eligible items, including checked flashlight records.

Preserve source sunglasses model and lens variants through their checked variant selection records.
Include only their referenced postprocessing profiles and parameters.
The cigarette requires its own source consume, removal, and exhausted-item behavior through the existing survival owner.
Include its source cigarette pack, item ID 74, as its native acquisition route.
Limit new consumable content to this pack and wearable. Do not add unrelated food.
Include source Box, item ID 62, and shovel, item ID 216, for required acquisition actions.

Apply checked source startup assignments before validating optional serialized links.
Export five wearable anchors from `level1:80125` in source type order:
`26477`, `28766`, `35477`, `36615`, and `39162`.
Export the right-hand anchor `level1:23118` and player character root `level1:29438`.
Resolve these identities once during loading.

Missing selected records, unsupported required shader operations, or broken skin bindings stop asset acceptance.
Do not replace required source content with generic models, tint overlays, or static body poses.

## State and ownership

Keep `ItemStore` as the owner of item state and identities.
Add `InventoryState` with four optional item IDs, selected slot, and source swap progress.
Add one optional right-hand item ID beside the existing carry state.
Add `WearableState` with five optional item IDs in source type order.
Add `AppearanceState` with selected model and segment settings for both models.
Keep these modules separate from movement and rendering.
`StartingArea` connects their commands to existing physics, interactions, and scene state.

Slot membership and physical item location are separate facts.
An item can remain listed in a slot while the player carries or equips it.
Represent stored, equipped, and worn locations through the current item location contract.
Keep one item record and one physical location for each identity.

An item cannot occupy two inventory slots or two wearable slots.
A worn item cannot remain in inventory, carry, an attachment, a wheel slot, or a part slot.
A stored item cannot participate in world physics or ordinary aim queries.
A hand item cannot remain a free dynamic body.
Source rules permit a distinct carried item beside the right-hand item. Preserve those rules and switch restrictions.

Validate a command before changing any owner.
Apply each accepted transfer once. A rejected action retains the previous ownership and physics state.
Use current attachment and part operations when transferring items out of those owners.
Consumption or deletion removes every reference to that identity.

## Inventory and equipment behavior

Number keys 1–4 use source held and released transitions.
Release selects a slot when the hold did not complete a swap.
Hold advances `inventorySwapValue` at source gain 5 and uses the source threshold check order.
After a swap, suppress selection on release and reset the swap latch after the source release condition.
Preserve source priority when several slot inputs arrive in one frame.
Use simulation elapsed time. Pause, lost focus, and menu entry clear held input and transient swap progress.

Pickup adds eligible items to an empty selected slot according to `Pickup`.
Store the previous selected item before retrieving the requested item.
Use the source restrictions when both hands or an occupied slot prevent switching.
Drop a non-inventory carry item only under the source `InvSwitchTo` condition.
Do not silently replace occupied inventory entries.

Stored items hide only their source `disableThisWhenStored` subtree.
Preserve source `dontDisableWhenStored` links where selected content has them.
Keep item state, fluids, condition, food progress, and attachments intact while stored.
Stored attached contents retain their owner and disappear or remain visible through the source subtree rules.
Do not store an assembly if its source rules or current owner prevent storage.

Retrieval chooses right-hand equipment when `canHoldRightHand` permits it and the hand is free.
Otherwise, retrieval uses the existing source carry behavior.
Dropping or throwing clears inventory membership for the dropped identity.
Existing food, container, flashlight, and interaction actions use the active source hand or carry item.
Do not duplicate those action implementations in inventory code.

A short Tab action equips an eligible carried item under the source eligibility checks.
A held Tab action retains existing recenter behavior.
Preserve the source 0.25-second hold boundary and the distinct comparison used by each caller.
Equip interpolation uses source factor 20 for up to one second, then snaps to the checked hand offset.
Respect `disableRHFix`, `RHP`, and `RHR`.
Release returns the item to source carry behavior and restores its source collision layers.
Seat entry and exit retain valid item ownership and update the hand frame.

Keep the number-key versus seated-clutch guard in the shared input dispatch.
The concurrent driving feature also changes input dispatch. Review that intersection before later integration.
Do not change driving physics in this branch.

## Wearable behavior

Use short Tab to wear an eligible carried wearable when its type slot is empty.
Keep source action priority with equipment and recenter commands.
Disable the wearable's checked collider set and remove dynamic physics participation.
Apply the source type anchor, `useCenterObj`, and `CenterObj` transform sequence.
Clear carry and inventory links before publishing worn state.

Use the source removal action on X release when the source carry guard permits it.
Check edible wearables first, then the first occupied type slot.
Restore checked colliders, mass, and dynamic physics before pickup.
Keep X's existing assembly action and dispatch the eligible action once through context and source priority.
Do not run both assembly and wearable removal from one input event.

The type slots are independent. Do not invent exclusive hat, cap, and helmet rules.
The source rejects wear into an occupied type slot. Do not replace or drop the current item automatically.
Use source renderer visibility rules for first-person views, external views, mirrors, and shadows.
Do not disable worn geometry globally to hide it from the player camera.

Glasses assign their checked postprocessing profile and source flare-disable flag.
Apply the profile only to the source-supported player view and respect its enable setting.
Removal clears both effects. Other cameras retain their source view behavior.
Use the existing postprocessing and flare owners. Compose the selected profile in source order.

## Player models and appearance

Load both source models. Publish only the selected model's checked renderers.
Keep a persistent pose owner for player targets, hand equipment, and wearable anchors.
Drive those targets from current walking, crouch, view, carry, and seat state.
Preserve source leg paths, hand targets, and checked IK chain parameters for the selected player hierarchy.
Reuse Bevy skinning and existing imported glTF joints.
Port the required source `IKscript` chain operation rather than adding an unrelated animation framework.
Retain chain storage and resolved joints. Update their values without rebuilding lists each frame.

Preserve the source player material `_Dist` behavior where the selected camera applies it.
Use view-specific bindings so mirrors and external cameras can render the body without corrupting the main view.
Include player and worn geometry in existing shadow, mirror, probe, and motion ownership as source rules permit.
Release all candidate pose and material resources when scene preparation fails.

The first model exposes Hair, Eyebrows, Eyes, Mustache, Skin, Shirt, Pants, and Shoes.
The second exposes Hair, Eyebrows, Eyelashes, Eyes, Skin, Shirt, Pants, and Shoes.
Expose visibility only when `disableAble` permits it.
Use source `defEnabled` and `defColor` values for reset.
Non-optional segments remain visible even when their serialized enabled field is false, matching source `Refresh`.
Keep settings for both models when the player changes selection.

Store hue, brightness, and saturation as finite values in the source range 0–1.
Convert to RGB through the source HSV ordering: hue, saturation, brightness.
Apply color to checked segment material slots. Do not recolor shared world materials.
Create material instances during model preparation, then update them only after an appearance change.
Do not treat shirts, pants, or shoes as pickup items.

## Native acquisition

Export all source selection entries and category graphs needed to evaluate the selected acquisition paths.
Export source station spawn markers and controls from the three existing station prefabs.
Their ordinary `misc4` spawner counts are 18, 6, and 38 for the checked station types.
Each station also has one checked underground spawner.
Retain source hierarchy order, item spawn distance, building overrides, `dontSpawn`, and holiday guards.
Retain source thresholds, place selection, rotation, item initialization, and random variant selection.
Port effective category weights from the executable constructor and selector, including the leaf-weight behavior noted in the acquisition audit.

Select from the complete source tables before deciding whether the current product supports an outcome.
Publish checked clothing and its required acquisition objects through the existing item owner.
Leave unsupported item outcomes empty and recorded. Do not reroll them or narrow the source category tables.
This preserves selected content probabilities while retaining the roadmap's broader population gaps.
Use repeatable current-world draws and persist first outcomes for each station.
Preserve source per-call draw bounds and order. Do not claim source-global random-stream parity on the current procedural world.
Verify selection with matched source draw sequences. Keep global world comparison as a recorded limitation.

Bind each station outcome to its seed, station index, source spawner, and source output ordinal.
Publish each first spawn once when its source activation conditions pass.
Store inactive outcomes and item changes through current station persistence and item ownership.
Revisiting or loading must not repeat a consumed, taken, broken, or rejected outcome.
Apply station transforms to source-local marker poses. Do not place loot beside the player.
Bound active station geometry and physics through current streaming windows.
Retain picked-up, stored, worn, and changed objects through existing persistent owners.

For buried outcomes, preserve source count, radius, offset, category selection, and rotation rules.
Set source buried state and reject pickup while the source ground check still reports burial.
Include the source shovel's equip pose, attack motion, contact mask, and shovel-speed recovery action.
Limit shovel outcomes to source recovery, contact feedback, and required motion. Add no combat damage.
Use current collision queries and retained hit storage to preserve source recovery without per-frame allocation.
Restore dynamic physics on `UnBury` and preserve the one-second recent-recovery rule.
Keep buried items in their source positions. Do not raise them to provide easier pickup.

For source boxes, select contents through the checked table and respect `spawnBeforeOpen` and `disableSpawn`.
Preserve source content locking, impact break thresholds, release links, debris, and selected source sounds.
Limit destruction to this acquisition object. Add no general damage framework or unrelated breakables.
Release contents once. Save the box, selected contents, locking, opened state, and destruction state together.
The source outside starter boxes have disabled spawning. Keep their contents disabled.

For cigarette packs, preserve source count and Take one behavior.
Create the checked cigarette prefab once after decrementing count.
Wear it when the mask slot is empty; otherwise retain the source world-item result.
Import checked packs from the accepted vehicle startup groups under the current two-car scene policy.
Do not activate a different vehicle group merely to provide a pack.

Activate the checked starter pumpkin through the source Halloween date rule.
The source window runs from October 15 through November 15 with inclusive calendar comparisons.
Use the browser's local calendar date, matching native local-date behavior.
Apply holiday activation only during new-game preparation. Load restores saved instance state.
Keep unrelated Hand and winter starter items outside this feature.

## Menus and feedback

Add a Player screen to the current start and pause menus.
Include model selection, segment selection, permitted visibility, three color controls, and Reset.
Expose the source Sunglasses effects toggle through the feature controls and current view settings owner.
Show the source player preview with the same appearance state and material rules.
Keep gameplay input, audio, and pointer capture under current menu ownership.
Do not accept gameplay commands while the player edits a menu.
Use current menu layout, fonts, and source labels. Add no unrelated navigation or style system.

Show four source inventory icons with the selected slot and source swap progress in the HUD.
Refresh the HUD after inventory changes. Reuse retained nodes and buffers.
Connect source prompts for Equip, Put on, and removal to existing interaction feedback.

Include source feedback for pack count, Take one, source box release, and buried recovery.
Add no item spawn menu, guaranteed apparel drops, automatic starter grants, or invented spawn probabilities.

## Save and profile contract

Add inventory membership, selected slot, right-hand state, wearable state, and appearance state to the game snapshot.
Retain stable item identities and existing absolute world poses.
Add a station loot identity with seed, station index, source spawner, and source output ordinal.
Give cigarettes created from packs a parent-instance identity and persistent creation counter.
Use initial source identities for checked startup packs and the holiday pumpkin.
Preserve prototype, selected variant, consumed state, buried state, and current location for each such instance.
Restore those instances during candidate creation before binding saved ownership.
Do not use runtime array indices or physics handles as saved identities.

Advance the game save schema for the changed snapshot. Reject previous game schemas without migration.
Keep browser database version separate from game schema version.
Validate distinct identities, membership, locations, eligibility, wearable types, variants, models, and segment counts before restore.
Reject unknown links and non-finite values. Do not clamp corrupt saved state into a different valid state.
Reset transient held inputs, swap progress, and equip interpolation after load.

Persist the appearance profile under a separate checked key in the existing browser object store.
Use current storage checksums and transactions. Adding this key needs no database schema upgrade.
New game uses the saved profile or source defaults when no profile exists.
Load uses the appearance recorded in that game snapshot and leaves the profile unchanged.
Applying appearance changes writes the profile and changes the current player.
Reset changes the selected model's profile to its checked source defaults.
Report profile storage failures through the Player screen and retain the last committed profile.

Restore item instances and links before physics participation, rendering visibility, and item use.
Prepare both required player and wearable assets before scene replacement.
A failed candidate leaves the active scene and saved record intact.
Do not mark Loading complete before skinning, materials, camera bindings, and ownership are ready.

## Modules and integration

| Owner | Responsibility |
| --- | --- |
| `tools/reference/inventory.py` | Checked slot inputs, item flags, icons, hand anchors, and storage links |
| `tools/reference/clothing.py` | Checked wearables, variants, profiles, player hierarchy, segments, and pose records |
| `src/reference/inventory.rs`, `clothing.rs` | Typed source contracts and validation |
| `src/inventory.rs` | Slot membership, selection, swapping, and source action guards |
| `src/clothing.rs` | Wearable membership and appearance settings |
| `src/interaction` and `src/starting_area` | Transfers, equipment, item use, physics changes, and spawned instances |
| `src/world/stations` and a focused acquisition module | Source selection, station loot outcomes, bounded publication, boxes, and buried recovery |
| `src/presentation/player.rs` | Retained pose bindings, skinning, segment materials, and view visibility |
| Current camera effects owners | Source glasses profile and flare state |
| Current save owners | Snapshot capture, validation, candidate restore, and profile persistence |
| Current web menu and feedback owners | Player controls, inventory HUD, acquisition feedback, and source prompts |

Use the existing pinned dependencies. Add no generic equipment framework, event bus, or configurable slot engine.
Resolve source IDs, hierarchy roots, and material slots during startup or candidate preparation.
Do not allocate strings, clone settings, rebuild materials, or search source records in frame paths.
Use bounded slots and retained pose storage. Work grows only with source-selected bones and live item instances.
Existing world streaming retains its current rules.

## Stages and acceptance

1. Export and validate source contracts, content, player poses, and required shader operations.
2. Complete item ownership, slot retrieval through carry and right-hand owners, four-slot inventory, HUD, and save restoration.
3. Complete explicit equipment actions, source interpolation, seat transitions, current item use, and save restoration.
4. Complete native acquisition, required pack, box, shovel actions, and save restoration.
5. Complete wearables, glasses effects, cigarette use, and save restoration.
6. Complete both animated player models, appearance controls, profile storage, and final scene integration.

Prepare permanent player pose anchors before stage 2. Add full model rendering in stage 6.
Each stage must run end to end before the next stage begins.
Do not merge partial layers into master.

Use critical tests for source slot selection and swaps, rejected transfers, duplicate ownership, wear restrictions, and save restoration.
Test source short-versus-held Tab actions, X dispatch, simultaneous hand state, and item removal after consumption.
Test category constructor behavior, source draw sequences, empty unsupported outcomes, source burial, box release, and revisit persistence.
Test pack decrement and cigarette creation once, plus new-game holiday boundaries and saved pumpkin restoration.
Test a failed candidate restore and a failed profile write without replacing the last working state.
Test source HSV conversion and reset on one representative segment for each model.
Test broken selected asset links and incompatible snapshot versions.

Build native and release WASM targets, package the export, and check affected JavaScript and shader variants.
Run browser scenarios for each gameplay layer using real imported items.
Check walking, crouch, seat entry, sustained driving, food use, fluids, flashlight use, and door dragging.
Save and reload stored, carried, equipped, and worn states, including simultaneous distinct hand items.
Check both models, optional face segments, clothing colors, reset, profile reuse, and scene replacement.
Check player and wearable geometry in main views, mirrors, shadows, and source glasses effect views.
Measure repeated equip, wear, model changes, and load operations for stable retained render resources.

Matched source inputs and views are required before claims of behavior or appearance parity.
Fixture tests, exports, compilation, and browser readiness do not establish parity.
Record limits when trusted pointer input or matched source runtime evidence is unavailable.

## Completion record

Update the roadmap with implemented behavior, source content coverage, verification results, and remaining acquisition limits.
Record the scoped station loot coverage and unimplemented source item outcomes.
Keep complete source population and global random-stream comparison under source-world work.
Keep the branch and worktree for later integration with driving and world work.
Review shared input, source contracts, save schemas, and player ownership before that integration.
