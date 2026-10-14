# Inventory and clothing source audit

Date: 2026-10-14.
Status: The user approved the staged design. Fresh acquisition research extends this initial audit. Implementation has not started.
Target: The `Inventory and clothing` row in [the roadmap](../roadmap.md).
Base: Local master at `c189df3`.

## Source evidence

I read the retained build `11140890` records and decompiled methods.
I checked the decompiled file hash against `managed/decompilation.json`.
The SHA-256 matches: `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee`.

Source paths below refer to `.cache/reference-library/records/`.
These retained files are ignored development evidence.

| Behavior | Source evidence | Finding |
| --- | --- | --- |
| Inventory capacity | `level1/80125.json`, `inventory` | Four empty slots in the serialized player record. |
| Initial selection | Same record, `selectedInv` | Slot zero. |
| Slot controls | `managed/Assembly-CSharp.cs`, `InventoryStuff`, default inputs 31–34 | Number keys 1–4. Release selects; hold starts a slot swap. |
| Swap timing | `InvSwapBegin`, `inventorySwapGain` | Gain is 5 per second. Preserve the source threshold and update order. |
| Pickup ownership | `Pickup`, `InvSwitchTo` | Eligible items can enter an empty selected slot while the player carries them. |
| Item restrictions | `pickupable.canInventory`, `canHoldRightHand` | Storage eligibility and right-hand eligibility are separate flags. |
| Store and retrieve | `InvStore`, `InvUnStore` | Stored items hide their selected object. Retrieval uses right-hand or carry behavior. |
| Drop | `DropFromInv`, `Drop` | Remove slot ownership when the player drops the item. |
| Wearable types | `wearableType` | Glasses, hat, cap, helmet, and mask. |
| Wearable anchors | `level1/80125.json`, `TWearablePos` | Five anchors: `26477`, `28766`, `35477`, `36615`, `39162`. |
| Wear action | `Wear`, `wearable.Wear` | Bind the item to its type anchor. Disable wearable colliders and remove dynamic physics participation. |
| Occupied wearable type | Wear callers in `usePickedUpStuff` and interaction paths | Callers require an empty type slot. Do not invent automatic replacement. |
| Remove action | `TakeOff`, removal input branch | Restore physics and pick up the item. Removal checks edible wearables first, then type order. |
| Glasses effect | `Wear`, `TakeOff`, `wearable.sunglass` | Connect the source view effect and its enable setting. |
| Player models | `level1/70544.json`, `characters` | Two character records: `level1:71215` and `level1:67763`. |
| Appearance | `characterscript`, `settingsplayercustomscript`, `playermodeloutfitscript` | Select a model. Set segment color, brightness, saturation, and permitted visibility. |
| Clothing segments | Both character records | Shirt, pants, and shoes belong to model segments. They are distinct from wearable pickup items. |
| Startup inventory | `level1/80125.json`, `startWithItems1/2/3` | All three serialized flags are false. Prototype links do not prove default acquisition. |

The first character exposes Hair, Eyebrows, Eyes, Mustache, Skin, Shirt, Pants, and Shoes.
The second exposes Hair, Eyebrows, Eyelashes, Eyes, Skin, Shirt, Pants, and Shoes.
The records permit visibility changes for selected hair and face segments.
The records do not permit shirt, pants, or shoes visibility changes through that flag.

## Current runtime

`src/interaction/items.rs` owns item state and existing item locations.
It has world, carried, attached, wheel, and part locations. It has no inventory or wearable state.
`src/player.rs` owns movement, view, and seat state. It does not own appearance state.
`web/input.js` has no number-key inventory bindings.
`src/save/schema.rs` uses schema 1 and captures the existing carry state.
The published library includes player assets. Export success does not establish animated player rendering.

## Proposed design

Complete the roadmap row in a separate worktree based on current master.
Use build `11140890` methods and checked records as the behavior authority.
Keep the deferred source-world branch separate.

Use one item owner. Slots reference existing item identities and do not create duplicate item instances.
Keep slot membership separate from the active carry or right-hand state, as the source does.
Keep wearable state and appearance settings in separate modules.
Reuse current interaction, physics, source materials, camera effects, and save preparation.
Resolve model, material, and wearable anchors during loading. Retain those bindings during frame updates.

1. Export checked inventory flags, anchors, controls, and selected content records.
2. Add four slots, select and swap actions, storage, retrieval, drop, and existing item use.
3. Add right-hand equipment and source hand transforms without replacing existing carry behavior.
4. Add native acquisition, including required pack, box release, and buried recovery actions.
5. Add source wearable items, type restrictions, anchors, visibility, and glasses effects.
6. Add both source player models and appearance controls for their checked segments.

Each gameplay layer includes save capture, validation, restoration, and runtime checks before the next layer.
Reject duplicate ownership, invalid identities, invalid segment settings, and invalid wearable type links.
Use a new save schema for the changed contract. Do not add a save migration or obsolete schema fallback.
Restore item ownership before physics and rendering participation during candidate scene preparation.

Use critical tests for ownership transitions, failed actions, source restrictions, and save restoration.
Check existing item use, walking, seat entry, driving, mirrors, and scene replacement after each relevant layer.
Check player and wearable visibility in the source-supported camera views.
Require matched source inputs and views before claiming behavior or appearance parity.

## Approach choices

| Approach | Result |
| --- | --- |
| Full row in stages, recommended | Covers slots, equipment, wearables, and appearance. Each stage retains a working game. |
| Inventory first as a separate feature | Delivers slots sooner. Requires another design and implementation cycle for clothing. |
| One combined implementation | Changes ownership, rendering, controls, and saves before an end-to-end checkpoint. Conflicts with the layer requirement. |

## Acquisition follow-up

The user requested native acquisition research after the initial design approval.
I decompiled the installed assembly again and traced its building, buried, pack, and holiday acquisition paths.
The [acquisition audit](2026-10-14-inventory-clothing-acquisition.md) records checked identities, selection rules, and tool dependencies.
The written specification uses those paths and includes no project item spawn panel.

## Implementation source checks

Bind the checked wearable pickup records, models, materials, and acquisition routes during export.
Trace player model roots, skeletons, camera visibility rules, and pose updates.
Recover exact equip and removal bindings, hand transforms, item store callbacks, and glasses parameters.
Check source appearance defaults, color conversion, and profile persistence against the serialized settings records.
Do not infer content coverage from the five-value wearable enum.
Do not add starter items from disabled startup groups.

This feature adds scoped acquisition actions. General combat, full loot coverage, and source-world integration remain separate.
The written specification defines native acquisition for current station placement and checked starter content.
