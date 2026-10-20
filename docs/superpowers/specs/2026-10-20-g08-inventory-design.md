# G08 inventory and equipment design

Date: 2026-10-20.
Status: Approved by the user on 2026-10-20. Implementation plan review remains pending.
Base: `cc7540b255d683722c67e09103a66bfa789c0fe7`.
Branch: `codex/g08-inventory`.

## Outcome and scope

G08 completes inventory and equipment behavior for the current garage experience.
The player can equip eligible tools, release equipment into carry, select slots, swap slots, and use equipped tools.
These actions preserve item identity, slot membership, fluid contents, tool state, and physical ownership.
Save and Load preserve each supported state.

The task boundary is G08 in [the roadmap](../../roadmap.md).
Steam build `11140890` and its decompiled code define behavior.
The current save schema defines the saved representation.
Clothing, survival, weapons, extra item actions, and general cargo expansion remain deferred.
This task adds no items or acquisition routes.

## Chosen approach

Extend the existing inventory coordinator in `src/starting_area/inventory.rs`.
That coordinator validates and applies transfers between carry, equipment, and inventory storage.
`InventoryState` retains number-key timing and slot membership.
`EquipmentState` retains the equipped item, physical setup, collider state, and hand pose.
Carry retains its existing joint, movement, rotation, drop, and throw behavior.

An alternative adds equip branches separately to carry, inventory, and tool code.
That approach duplicates ownership rules and makes partial transfers harder to prevent.
A general ownership framework would alter unrelated attachment and cargo behavior.
The chosen approach limits changes to existing garage paths.

## Source evidence

The source file is `.cache/reference-library/decompiled/managed/Assembly-CSharp.cs`.
Line numbers refer to the source available during this design review.
Ignored reference files remain in the primary checkout.

| Source | Required behavior |
| --- | --- |
| `InventoryStuff`, lines 14665-14709 | Released slots precede held slots. Lowest slot number wins each phase. |
| `InvSwitchTo`, lines 14813-14831 | Store the previous slot, retrieve the target, then change the selection. |
| `InvSwapBegin` and `InvSwapTo`, lines 14834-14865 | Accumulate progress, exchange entries, retrieve the selected entry, and suppress release selection. |
| `InvStore` and `InvUnStore`, lines 14866-14945 | Storage and active ownership differ from slot membership. |
| Recenter input, lines 15773-15787 | Record Tab press time. Apply the source hold and rearm gates. |
| Equip action, lines 16427-16445 | A short Tab release equips eligible carry. A held Tab recenters it. |
| `usePickedUpStuff`, lines 16974-17119 | Tool use distinguishes carry from equipment. Brushes require equipment. |
| `PickedUpForceAdd`, lines 17366-17371 | Tab or MMB releases equipment only when carry is empty. |
| `RightHandGrip`, lines 18346-18362 | Equip eligible carry into an empty right hand without clearing its slot. |
| `RightHandGripAnimation`, lines 18364-18377 | Interpolate toward the source hand pose, then finish at that pose. |
| `RightHandRelease`, lines 18380-18405 | Return equipment to carry and reset the equip and recenter gates. |
| `Pickup` and `Drop`, lines 18415-18462 | Register eligible carry in an empty selected slot. Drop clears its slot references. |
| `sprayscript.Fire`, lines 61319 onward | Spray behavior depends on right-hand ownership. |

`src/reference/inventory.rs` validates the imported four slots, swap gain, hold duration, and hand interpolation values.
Each item's existing `PickupReference` defines inventory and equipment eligibility.
Item names do not define eligibility.
The installed export has eight eligible garage tools, item IDs 126 through 133.
These are two flashlights, two brushes, two sponges, and two spray cans.
Oil can 120 permits inventory but cannot equip. Fuel can 122 and barrel 124 permit neither.
Existing part inventory flags remain authoritative. This task does not remove their current behavior.

## Input behavior

Tab continues to use the existing `RECENTER` input bit.
Digits 1 through 4 continue to use the existing inventory masks.
The browser bridge needs no new input field.

For eligible carry and empty equipment, Tab release before 0.25 seconds requests equip.
Tab hold after 0.25 seconds permits recenter.
At exactly 0.25 seconds, neither strict threshold has passed.
For carry that cannot equip, preserve the source immediate recenter path.
Occupied equipment also prevents a second equip and preserves the source recenter path.

With equipment and empty carry, Tab press requests release while standing.
MMB requests release while standing or seated.
Tab does not release equipment while seated.
Tab and MMB never evict existing carry through this input path.
The release frame cannot immediately equip the item again.
Release clears the equip and recenter gates before carry processing.
Tab release rearms both gates at the source end-of-frame point.
Tab-up time beyond 0.25 seconds also rearms equip before interaction.
A successful explicit equip sets a guard for the current gameplay update.
That guard blocks release during the same update, including held MMB.
The guard resets at the start of the next gameplay update.
These ordered gates replace a generic cooldown rule.

A number release selects its slot unless a completed swap suppresses selection.
A held number advances swap progress using the imported gain.
A swap occurs in the held phase after progress has already reached one.
Selection and swap reset progress at the source transition points.
Idle frames clear the completed-swap latch, rather than inventing a progress reset.

Released-slot processing occurs once before held-slot processing.
The held phase receives `inventory_released = 0` so it cannot repeat the released-slot action.
When several slots have input, the lowest number wins within that phase.

Inventory and gear keys share source bindings.
Preserve the source seated-clutch exclusion when those bindings collide.
Pause, input loss, reset, and load clear unfinished gestures through the existing cancellation path.
Cancellation produces no delayed equip, release, selection, or swap.

The G08 phases follow the source update order:

1. Update Tab timing and reset the same-update equip guard.
2. Resolve explicit Tab equip or recenter.
3. Process released slots, then held slots.
4. Resolve equipped tool use before carry processing.
5. Resolve equipment release, then carried tool use and carry movement.
6. Apply the late Tab-release rearm gates.

The coordinator moves explicit equip before the existing inventory update.
Combined Tab, MMB, and slot input must use this order rather than independent handlers.

## Ownership and transfers

Slot membership is separate from physical ownership.
An item has one physical location at a time.
Carry and equipment can contain different items at the same time.
An equipped item need not occupy a slot.

Each transfer validates both its source and destination before mutation.
Validation includes eligibility, item life, attachment state, body availability, and destination occupancy.
A rejected transfer changes no slot, location, body mode, collider, pose, or carry joint.
Unexpected missing retained bindings remain internal errors rather than partial success.

Equip removes the carry joint and sets the item to `Equipped`.
Equipment owns the body and uses the existing equipment collider policy.
Release restores the body and collider state, creates carry ownership, and sets the item to `Held`.
Release preserves the current item pose as the carry grip starts.
Neither action drops the item into the world or clears its slot membership.

Storage releases the active owner's physical setup and disables the stored body.
Retrieval restores the body and chooses equipment when eligible and available.
Otherwise, retrieval uses carry if carry is available.
Stored items cannot remain collidable or retain a live carry joint.

Selection drops non-storable carry before retrieving a nonempty target, as the source does.
An empty target preserves that carry.
Selection and swap reject a nonempty target when both carry and equipment are occupied.
The existing ancestor-carry exclusion also remains.

Dropping, attaching, or removing an item clears obsolete slot membership.
An item cannot appear in two slots or in both carry and equipment.
Fluids, tool quantity, color, condition, and identity survive every accepted transfer.

## Hand pose and tool use

Use the imported right-hand anchor, item offsets, and local item pose.
Use the imported hand interpolation factor and duration for a new equip transition.
After interpolation ends, retain the source final hand pose.
Slot retrieval uses the source immediate final hand pose instead of the explicit-equip animation.
Stored equipment and canceled transitions retain no stale interpolation state.
This task does not add player skins, clothing, or a new body animation system.

Tool frames distinguish carry from equipment.
Brushes require equipment for rubbing and repair.
Sprays preserve the source equipped aiming and carried emission paths.
Flashlight controls continue to resolve the active equipment or carry owner.
Process equipped tool use before carried tool use.
Equipped flashlights reserve LMB before carried flashlights.
Sprays do not share that reservation. Two owned sprays can both fire during the same update.
Storage, release, pause, and input loss clear obsolete tool contact and use state.
Existing condition, fluid, depletion, sound, and particle systems remain responsible for their effects.
G06 visual effects remain outside G08.

Context hints and the controls panel describe equip, release, recenter, selection, and swap accurately.
Hints reflect eligibility, occupancy, and seat restrictions.

## Save and Load

The existing schema already stores slots, selection, carry, and independent equipment.
This design requires no schema version change, compatibility path, or migration.

A nonempty selected slot refers to the active carried or equipped item.
Each nonselected member has `Stored(slot)` ownership.
Each member is unique and eligible for inventory.
Equipment is live, eligible for the right hand, unattached, and distinct from carry.
Direct equipment without slot membership remains valid.

Imported and existing supported generated items use the same ownership rules.
Remove the generated-equipment requirement for selected-slot membership.
Retain generated identity, eligibility, attachment, and carry checks.
This repair adds no general cargo or world population behavior.

Restore waits for existing item and body bindings before equipment setup.
Stored bodies remain disabled after restore.
Carry joints and equipment collider state are reconstructed once.
Gesture progress resets on load.
Tool quantities and contents use their existing saved records.

## Module boundaries

| Existing module | G08 responsibility |
| --- | --- |
| `src/inventory.rs` | Slot gesture phases, progress, selection, and membership. |
| `src/starting_area/inventory.rs` | Validated transfers and runtime input order. |
| `src/interaction/equipment.rs` | Equipment body state, collider state, and source hand interpolation. |
| `src/interaction/carry.rs` | Carry acquisition, release, and recenter after gesture resolution. |
| `src/interaction.rs` | Input consumption, cancellation, and owner-aware action priority. |
| `src/interaction/cleaning.rs` and `src/app.rs` | Distinct carry and equipment data for garage tool use. |
| `src/save/validate.rs` and `src/starting_area/save.rs` | Independent ownership validation and reconstruction. |
| `src/interaction/targets.rs` and `web/index.html` | Context hints and controls text. |

No generic command registry or configuration layer is required.
Per-frame gesture and pose updates add no allocations or repeated collider setup.
Reusable physical state remains in the existing owner components.

## Verification and acceptance

Only critical tests are required.
Gesture tests cover short Tab, held Tab, the exact threshold, release rearm, occupied carry, and seated release.
Combined-input tests cover short Tab release with held MMB, and Tab equip with slot selection.
Slot tests cover both input phases, simultaneous keys, completed swaps, cancellation, and the clutch conflict.
Ownership tests cover repeated equip, release, store, retrieve, drop, and rejected transfer cycles.
Checks inspect identity, contents, joints, body mode, body activation, colliders, and unique slot membership.
Tool tests cover equipped brush use and the distinct spray ownership paths.
Save tests cover stored items, equipment without a slot, both active owners, and invalid duplicate ownership.
Existing generated-equipment save coverage applies only to already supported item paths.

Use targeted native tests and both existing WASM compile checks for Rust changes.
Use targeted input web tests if input handling changes.
The installed-export inventory test is ignored by default and needs an explicit ignored-test invocation.
A skipped test does not count as executed coverage.

Full checks, release builds, and browser acceptance remain deferred unless separately requested or agreed.
Browser acceptance must later demonstrate normal Tab, number-key, tool-use, and save input on the tested package.
Native tests and compilation cannot establish that runtime behavior.
The implementation report lists actual checks, failures, deferred checks, and unverified behavior.
G08 can become `Verify` only when code coverage is complete, with browser acceptance still stated separately.

## Review and execution boundary

This specification records the user-approved design.
Product code remains unchanged at this stage.
Written-spec approval permits the implementation plan.
The user reviews that plan before implementation starts.
The requested execution method uses subagents in this isolated worktree.
The branch remains unmerged unless the user authorizes integration.
