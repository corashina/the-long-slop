# Inventory and clothing acquisition

Date: 2026-10-14, Europe/Warsaw.
Status: Source acquisition research for the inventory and clothing specification.

## Fresh decompilation

I decompiled the installed game's `Assembly-CSharp.dll` with the repository's checked ILSpy tool.
The installed Steam manifest reports build `11140890`.
The installed assembly SHA-256 is `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
The new decompilation reports completion, 637 types, 3,124 methods, and no diagnostics.
Its SHA-256 is `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee`.
It matches the retained accepted decompilation.

Command:

```powershell
pwsh -NoProfile -File tools/reference/decompile.ps1 `
  -AssemblyPath 'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive/TheLongDrive_Data/Managed/Assembly-CSharp.dll' `
  -OutputPath '.cache/inventory-clothing/fresh-decompilation'
```

The ignored `.cache/inventory-clothing/` directory retains the new receipt, source text, scripts, and checked acquisition records.
The acquisition scan read 6,831 checked records. The station hierarchy scan read 3,885 checked records.
These scans cover selected routes. They do not prove full world population coverage.

## Ordinary building loot

`buildingscript.CheckDist` calls `SpawnStuff` when the player enters the source item spawn distance.
`SpawnStuff` collects child `newRandomStuffSpawnScript` components, including inactive components.
It applies the source building chance override and starts underground spawning before the ordinary spawn coroutine.
The coroutine visits ordinary spawners in source order and skips `dontSpawn` entries.

`newRandomStuffSpawnScript.Spawn` applies holiday and spawn-chance checks.
`InitSpawn` builds cumulative item and place thresholds.
`SelectSpawn` chooses the entry and its source transform.
`SpawnItem` selects a source item or category, applies rotation, and calls source item initialization.

The three station prefabs already imported by master contain apparel-capable `misc4` spawners:

| Station | Ordinary `misc4` spawners | Underground spawners |
| --- | --- | --- |
| `Benzinkut01`, prefab `sharedassets1.assets:26573` | 18 | 1 |
| `LegjobbBenzinkut`, prefab `sharedassets1.assets:27344` | 6 | 1 |
| `NagyBenzinkut`, prefab `sharedassets1.assets:41123` | 38 | 1 |

The 62 ordinary spawners have individual source chances near 0.20, 0.30, 0.40, or 0.65.
Do not replace these per-spawner values with one station-wide rate.

Representative checked records:

| Record | Path | Source chance |
| --- | --- | --- |
| `sharedassets1.assets:132525` | `Benzinkut01/Spawn/MiscSpawn (4)` | `0.20000000298023224` |
| `sharedassets1.assets:137208` | `LegjobbBenzinkut/Spawn/MiscSpawn1` | `0.6499999761581421` |
| `sharedassets1.assets:130805` | `NagyBenzinkut/Spawn/MiscSpawn (9)` | `0.4000000059604645` |

Category `misc4`, ID 35, selects categories `32, 33, 34, 49, 50, 51, 58, 68, 70, 78, 89`.
Their ordered weights are `20, 20, 20, 5, 35, 25, 30, 15, 50, 20, 3`.
Child category 32 contains item IDs `74, 172, 217, 218, 220, 245, 305, 306`.
This includes a cigarette pack, sunglasses, a hat, and a cone.
Item ID 230 is the individual cigarette. Item ID 74 is the pack.

For category 32, the constructor stores its supplied weights in `categorychances`, while its leaf `itemchances` remain null.
`selectId` assigns unit weights to leaf items when `itemchances` are null.
Thus this leaf selection uses eight equal entries, despite the supplied weight array.
Preserve the executable constructor and selector behavior. Do not use the apparent array as corrected leaf weights.

## Buried items and boxes

Category `wearables`, ID 84, contains item IDs `217, 218, 245, 221` with equal leaf weights.
The items are sunglasses, hat, cone, and cap.

Each station's underground table includes category `wearables` at weight 1.
The table also includes other categories. Do not normalize it to clothing alone.
Each checked station underground spawner has chance near 0.4 and count range 1–5.
The source uses distinct per-station center, radius, and vertical offset fields.

| Record | Station | Radius range |
| --- | --- | --- |
| `sharedassets1.assets:154992` | `Benzinkut01` | 3–9 metres |
| `sharedassets1.assets:132529` | `LegjobbBenzinkut` | 5–11 metres |
| `sharedassets1.assets:157204` | `NagyBenzinkut` | 13–20 metres |

The source stores vertical endpoints near `-0.15` and `-0.4` metres in that order.
Preserve source calls and endpoint behavior. Do not reorder them during export without checking native behavior.

`undergroundSpawnScript.Spawn` selects categories and places each item, then calls `FStart`, `RandomSpawned`, and `Bury`.
`pickupable.CanPickup` rejects items while `tosaveitemscript.IsBuried` remains true.
`meleeweaponscript` calls `UpdBuried` during shovel contact.
`UpdBuried` raises the item by shovel speed and elapsed time, then checks the source ground relation.
`UnBury` restores dynamic physics and starts a one-second recent-recovery interval.
A buried cap cannot become ordinary surface loot merely because the current product lacks digging.

The underground tables also select source item `box01`, ID 62.
Its checked prefab has `spawnInBoxScript` at `sharedassets1.assets:158204` and GameObject `sharedassets1.assets:9085`.
The prefab enables content spawning and sets `spawnBeforeOpen`.
Its content table includes category `wearables`, with source weight 1 among other entries.
The source box has `breakablescript` at `sharedassets1.assets:135113` and content locking through `mountStuff`.
The eight scene boxes beneath `StartSpawnItems/outside` set `disableSpawn` true.
Do not activate their content tables to provide invented starting loot.

Source shovel item ID 216 belongs to leaf category `melee`, ID 67.
The checked station spawners select category `lengthy`, ID 82, whose category graph can reach the shovel.
The implementation must preserve that graph when selecting the shovel.
Box release and shovel recovery are acquisition dependencies. General combat remains a separate feature.

## Cigarettes

The checked pack has `cigiscript` at `sharedassets1.assets:141307` on GameObject `sharedassets1.assets:32825`.
Its `prefab` points to GameObject `sharedassets1.assets:11985`, the individual cigarette.
The individual cigarette has wearable component `sharedassets1.assets:142834`, type `mask`.

`usePickedUpStuff` exposes Take one when the pack has remaining count.
Use reduces the count and creates the source cigarette prefab.
The source wears the new cigarette when its wearable type slot is empty.
It retains a separate world item when that slot is occupied.
The smoke action consumes the worn edible through the existing survival path.

Source startup groups also contain cigarette packs for selected vehicles.
`TerrainGenerationSettings.CheckCar` activates the selected startup group and removes the others.
The project must retain its accepted two-car scene policy when adding those checked startup supplies.

## Pumpkins and starter inventory

`TerrainGenerationSettings.StartDateItems` links Halloween to GameObjects `level1:82` and `level1:8094`.
They are `PumpKin` and `Hand`.
The pumpkin's wearable component is `level1:79857`.
`mainscript.CheckHoliday` enables Halloween from October 15 through November 15, with inclusive calendar comparisons.
New-game startup applies this date activation only outside load and multiplayer paths.
Keep the pumpkin disabled outside that source window. Save/load restores the saved instance state.
Adding the pumpkin does not require adding the unrelated Hand item.

The three inventory starter flags remain false in the selected player record.
These flags are separate from vehicle startup supplies and holiday world items.
Do not turn them on to solve acquisition.

## Design result

Use native acquisition: source station loot, source buried recovery, cigarette packs, and holiday pumpkin activation.
Remove the proposed project spawn panel.
Keep source category selection, empty outcomes, conditions, transforms, and draw order.
Do not reroll unsupported outcomes into wearables or replace buried placement with surface placement.
Keep existing station placement and road generation. Add only the acquisition behavior needed for this feature.
Record unavailable item families as existing population gaps.
Source-global random-stream and world-generation parity require the deferred source-world comparison.

The written specification must include the required pack, shovel recovery, box release, and their saved state.
It must retain general weapons, unrelated destruction, and full loot coverage outside this feature.
