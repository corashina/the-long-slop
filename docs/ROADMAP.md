# Roadmap

Goal: recreate The Long Drive in the browser.
Reference: installed Steam build 11140890 and its recovered code.
Rows marked out of scope need a `PRODUCT.md` change before work starts.

| # | Feature | Original classes | State |
| --- | --- | --- | --- |
| 1 | Source UI: menus, crosshair icons, prompts, Lada gauges | `mainscript`, `menuhandler`, `meterscript` | Planned |
| 2 | Material parity in the garage | `conditionmaterial`, `materialChangeScript` | Planned |
| 3 | Original road, terrain, and biome generation | `roadGenScript`, `TerrainGenerator`, `TerrainGenerationJob` | Done for near terrain |
| 4 | Roadside buildings, villages, and loot | `buildingscript`, `falugenscript`, `roadBuildingScript`, `newRandomStuffSpawnScript` | Next after 3 |
| 5 | Day and night, headlights, and lamps | `napszakvaltakozas`, `headlightscript`, `interiorLightScript` | Open |
| 6 | Part wear, condition, and repair | `partconditionscript`, `repair`, `electronicsscript` | Open |
| 7 | Audio: engine, steps, ambience, and radio | `soundMainScript`, `stepSoundScript`, `radioscript` | Out of scope |
| 8 | Survival: hunger, thirst, food, and death | `survivalscript`, `ediblescript`, `wound` | Out of scope |
| 9 | Save and load | `savedatascript`, `save_*` | Out of scope |
| 10 | Weather: sandstorms, snow, and tumbleweeds | `sandstormscript`, `snowfallscript`, `thumbleweedScript` | Open |
| 11 | Creatures and enemies | `newAiScript`, `vultureAI`, `Sandworm` | Out of scope |
| 12 | Weapons | `weaponscript`, `revolverScript`, `meleeweaponscript` | Out of scope |
| 13 | Other vehicles, bus, and trailers | `CarType`, `busdoorscripts`, `utanfutoscript` | Out of scope |
| 14 | Tools and wearables | `flashlightscript`, `compassscript`, `metalDetectorScript`, `digholescript` | Open |
| 15 | Settings, key rebinding, and game modes | `settingsscript`, `inputSettingScript`, `gamemodesettings` | Out of scope |

Feature 3 leaves distant terrain layers and the `TerrainBiome` blend shader open.
Multiplayer and VR stay out. They do not fit a single-player browser build.
