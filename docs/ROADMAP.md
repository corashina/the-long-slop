# Roadmap

Goal: recreate The Long Drive in the browser.
Reference: installed Steam build 11140890 and its recovered code.
Rows marked out of scope need a `PRODUCT.md` change before work starts.

| # | Feature | Original classes | State |
| --- | --- | --- | --- |
| 1 | Source UI: menus, crosshair icons, prompts, Lada gauges | `mainscript`, `menuhandler`, `meterscript` | Done |
| 2 | Material parity in the garage | `conditionmaterial`, `materialChangeScript` | Planned |
| 3 | Original road, terrain, and biome generation | `roadGenScript`, `TerrainGenerator`, `TerrainGenerationJob` | Done for near terrain |
| 4 | Roadside buildings, villages, and loot | `buildingscript`, `falugenscript`, `roadBuildingScript`, `newRandomStuffSpawnScript` | Next after 3 |
| 5 | Day and night, headlights, and lamps | `napszakvaltakozas`, `headlightscript`, `interiorLightScript` | Day cycle and headlights done; lamps open |
| 6 | Part wear, condition, and repair | `partconditionscript`, `partscript`, `carscript.DamageStuff` | Crashes and fitting done; repair open |
| 7 | Audio: engine, steps, ambience, and radio | `soundMainScript`, `stepSoundScript`, `radioscript` | Done except radio |
| 8 | Survival: hunger, thirst, food, and death | `survivalscript`, `ediblescript`, `wound` | Out of scope |
| 9 | Save and load | `savedatascript`, `save_*` | Out of scope |
| 10 | Weather: sandstorms, snow, and tumbleweeds | `sandstormscript`, `snowfallscript`, `thumbleweedScript` | Open |
| 11 | Creatures and enemies | `newAiScript`, `vultureAI`, `Sandworm` | Out of scope |
| 12 | Weapons | `weaponscript`, `revolverScript`, `meleeweaponscript` | Out of scope |
| 13 | Other vehicles, bus, and trailers | `CarType`, `busdoorscripts`, `utanfutoscript` | Out of scope |
| 14 | Tools and wearables | `flashlightscript`, `compassscript`, `metalDetectorScript`, `digholescript` | Open |
| 15 | Settings, key rebinding, and game modes | `settingsscript`, `inputSettingScript`, `gamemodesettings` | Out of scope |
| 16 | Complete asset library: models, textures, audio, UI, and scenes | — | Spec and plan written; not started |

Feature 1 does not show part condition or the headlight key in its prompts.
Feature 3 leaves distant terrain layers and the `TerrainBiome` blend shader open.
Feature 6 knocks wheels and car parts off in crashes. Each part fits again only into its own slot.
The source also lets some parts fit other slots, such as hood and trunk. Hubcaps stay on.
Repair needs source tools such as the wire brush. They come with feature 4 loot.
The source engine and battery have no condition.
Feature 7 leaves the radio open. The sandstorm sound comes with feature 10.
Feature 5 leaves street lamps, the interior light, rear lamps, stars, and clouds open.
Multiplayer and VR stay out. They do not fit a single-player browser build.
