# Source world evidence

The probe uses installed Steam build `11140890` and Unity `2021.3.18f1`.
The source archive and engine hashes gate capture. Proprietary helpers and samples stay under `.cache/world-source`.
No product generator supplies expected values.

Run from the isolated source-world worktree:

```powershell
./scripts/sample-source-world.ps1 -ReferenceRoot C:/Users/Corashina/Documents/Projects/the-long-drive-wasm -OutputRoot .cache/world-source
```

`ReferenceRoot` identifies the project with the accepted library and Python environment.
The checked local harness is required. Its manifest hash is pinned in `world_probe.py`.
The script builds hooks, prepares a scratch player, captures five seeds, repeats capture, and checks payload hashes.
Use `-Verify` to check an existing envelope.
Source bodies, captures, instruction maps, and expected values must remain outside Git.

## Source authority

| Input | Checked identity |
| --- | --- |
| UnityPlayer.dll | `553c1e7560e1b42bd4638c53d0168dcd1b4b9eb5b9df324b0a44f14b3f18bc86` |
| Assembly-CSharp.dll | `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e` |
| Whole managed decompilation | `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee` |
| Library manifest | `92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99` |

`managed-methods.json` records original method RVAs, body sizes, header sizes, and hashes.
The native range receipt covers the full two-dimensional Perlin function and quaternion boundary.
The cloud noise slice is not an authority for world noise.
`LibraryReader` checks selected typed records after `verify_library` checks the accepted library.

## Sample classes

| Sample | Execution and limits |
| --- | --- |
| Native noise | Original x64 instructions in an isolated process. Both noise coordinates vary. |
| Canonical placement candidates | Original managed source job inside Unity, with source random state and branch draws. |
| Standalone placement | Diagnostic source execution with container adapters and checked Mono optimization settings. |
| Standalone terrain | Executed source job with explicit empty road, building, and hole inputs. |
| Standalone roads | Diagnostic source execution with the checked native quaternion boundary. |
| Canonical roads and terrain | Original road methods and scheduled terrain jobs inside the scratch Unity player. |
| Bones, meshes, poles, holes | Assembled source objects and native Unity transforms and mesh baking. |
| Nested spawn decisions | Original source callers with explicit shared state and controlled coroutine scheduling. |

All five seeds use `42`, `0`, `-1705217151`, `-2147483648`, and `2147483647`.
Road samples cover segments `0, 1, 99, 100, 255, 256, 257, 999, 1000, 9999, 49999`.
Each source road run computes every preceding segment.
Placement capture contains 10,000 candidates, including rejected choices and large-range random draws.
The envelope includes initial random arrays, raw consumed values, range bounds, and results.
Candidate samples precede the object caller's biome, road, and building exclusions.

## Initialization and source frame

The menu passes the world seed to `mainscript`.
`TerrainGenerationSettings.FStart2` transfers that seed when `randomSeed` is true.
It starts terrain, then road generation, before moving starting objects.
`settingsscript.RefreshInMain` computes a clamped interpolation factor from the selected draw distance.
It interpolates generation, removal, camera, and item distances between the source minimum and maximum settings.
`UpdGenDists` sends these values to the active generators.
The canonical context uses the saved default settings profile, with draw distance 5000 metres.
Its interpolation factor is `(5000 - 500) / (25000 - 500)`, or `9/49`.
The source menu reset coroutine first yields one frame.
Capture waits for that reset, then invokes the original settings `Open` and `PBack` actions.
`PBack` saves defaults inside the isolated profile before the normal scene-load action.
Gameplay loads that profile through its original settings caller.
The envelope records menu settings, default settings, live settings, and all distance bounds.

An unsaved first launch is a distinct timing context.
Its menu reset reaches 5000, but gameplay initially reads the serialized value 2500.
Gameplay queues another reset after initialization when the settings file is absent.
The envelope does not call this unsaved path the canonical profile.
Earlier captures that loaded during menu startup remain timing diagnostics.

The first `StartObjs` entry computes one shared vertical offset.
Every `StartObjs` root receives that offset before child movement, foundation setup, and hole refresh.
Fixed roots compute separate terrain heights.
The garage and well holes register even though their roots start inactive.
The source later activates those roots through `EnableAfterStart`.
The checked runtime captures `haz02/Luk` and `kut/Luk (1)`.
The garage foundation has three helpers. The bus stop has one.

The road world and terrain world start with identity rotation and unit scale.
Road initialization changes the road world's height.
The road parent has a nonzero source translation. Mesh capture retains it.
Canonical generation keeps this fresh frame fixed for the complete road calculation.

Source rebasing changes road results when generation runs after a shift.
`CalcOneRoad` casts world coordinates to float before terrain origin subtraction.
Matched origin shifts therefore change rounding at distant segments.
The envelope retains translated-frame diagnostics at segments 1000, 9999, and 49999.
These diagnostics are not canonical road samples.
The replacement must keep generation coordinates separate from presentation and physics origin changes.
The samples do not establish parity for arbitrary original-game rebase schedules.

## Native and managed math

The assembled source enables `__use_fma3_lib=1` on this machine.
Mode zero is a separate diagnostic context.
Quaternion source ranges include the injected wrapper, Euler conversion, `sinf`, and `cosf`.
Guarded emulation proves the sampled mode-zero instruction and data accesses.
The installed emulator rejects a mode-one FMA instruction.
Mode-one native execution uses the checked finite-angle branch ranges on the host CPU.

Matching native quaternion components do not establish matching managed vector multiplication.
The standalone host's `float32` optimization changed one sampled vector coordinate by one float step.
It also changed position-derived placement seeds.
The corrected diagnostic uses `--optimize=all,-float32` to preserve the observed wider intermediate arithmetic.
Canonical acceptance still requires assembled original-method road and placement rows.
The envelope records quaternion-vector boundary samples for the later rotation implementation.
The original vector overload is at CoreModule RVA `0x1f514`, with a 301-byte body.
Its body hash is `5ee0975e8f75d416a4c16dabda8206b2764ff3d5a1bbaffa875c844d75e35cea`.
All 550 sampled road bone rows match the corrected diagnostic after float parsing.
All 10,000 candidate seeds, choices, rotations, object seeds, and initial random arrays also match.
No tolerance was widened to accept the standalone road diagnostic.

## Geometry and terrain

The three selected road variants share mesh `sharedassets1.assets:1601`.
The source mesh has 88 vertices and one submesh.
The road script controls 10 bones. Skinning uses 11 bones and bind poses.
Capture retains both sets, source vertex attributes, weights, baked geometry, and world matrices.
Adjacent selected segments supply seam evidence.
Pole capture includes the source transform and its seeded random branch draws.

Terrain samples schedule the original source job for four adjacent tiles and wait for completion.
Each tile also runs directly with the same input arrays; capture retains both outputs.
Verification requires exact equality for vertices, normals, colors, and topology.
The captured player reports Burst disabled.
A `BurstDiscard` observer counts one managed `Execute` entry for each scheduled tile.
This proves the observed jobs used Mono; the source attribute alone does not prove Burst execution.
Samples include source road coordinates, building ranges, hole coordinates, vertices, normals, colors, and triangles.
Biome capture samples both sides of the strict blue and red thresholds.
Blue above 0.25 selects winter before the red test.
Otherwise, red above 0.5 selects desert. The remaining case selects grass.
Biome overrides remain source caller inputs.

## Random streams and nested calls

Road initialization, building selection, placement candidates, and poles use distinct source `System.Random` streams.
Road building selection retains its sequential stream and changing pool order.
Pole initialization uses the world seed plus segment index.
Placement uses the selected generator's seed offset and coordinate-derived seed.
Its rejected branch does not consume accepted-branch rotation and object-seed draws.
The placement job receives the live source cumulative `chances` array and `maxChance`.
`ObjectGenerationScript.FStart` already accumulates the serialized weights.
Capture preserves raw weights, live type weights, live job weights, and the empty-selection range.
It never accumulates the live weights a second time.

Roadside buildings use the world-seeded stream through their full sequential `GenAll` pass.
Their checkpoints must retain group counters, changing type pools, the previous type, and the next distance.
The source repeats a type selection once when it equals the previous building.
It then draws a side, lateral offset, yaw offset, and distance to the next building.
Its float range helper draws integer hundredths before converting back to float.

Object placement tests biome masks after candidate selection.
It then applies the selected type's road-distance bounds before terrain alignment and prefab creation.
Rejected candidates are removed from the pending arrays without another selection draw.
The selected settings reference `ObjectGenerationScript` for these candidates.
The alternate `newMapRandomGenerator` and `terrainRandomStuffSpawn` path does not supply this probe's expected values.
`stuffspawn.randomItem()` uses ordered chance tests; its supplied-number overload consumes no random draw.
Those callers must not be replaced with a common weighted-choice rule.

Nested spawns share Unity random state with other source callers.
The harness supplies `Random.InitState(123456789)` at terrain initialization.
It captures the resulting four-word state and each later source state.
This is a declared input. It is not an untouched game's initial state or a world-seed derivation.
The source clock input is fixed at local `2026-10-12T09:00:00`.

`buildingscript.SpawnStuff` finds child spawners with inactive children included.
It applies the building chance, runs underground spawns, and starts its source coroutine.
The harness drains that coroutine synchronously at this boundary.
It records explicit garage, well, and ordered scene-spawner calls.
The well's checked `SpawnFridge` choice creates `Fridge` and invokes seven nested child spawners.
Capture includes rejected chances and holiday exclusions without introducing caller-specific seeds.
Active hierarchy, prior instances, and caller order can change draw counts.
The trace proves replay only for the declared source context and schedule.
Excluded product content must still preserve relevant source selection and draw order.

## Isolation and acceptance

The installed game and accepted archive stay unchanged.
The scratch player redirects `pathscript.path` to a fresh profile inside each capture directory.
Every unchanged scratch input is checked against the accepted archive before launch.
Public PlayerPrefs calls use an isolated in-memory adapter. Steam initialization is suppressed.
Hook changes are recorded. They are part of the sampler identity.
Headless capture provides numeric geometry evidence, not a rendered-image comparison.

Verification rejects changed engine or sampler hashes, missing seeds, missing geometry, and incomplete random-state evidence.
It rejects payload changes and unequal replay hashes.
It checks both source logs and rejects sampling exceptions or unknown shutdown exceptions.
The known camera cleanup exception is allowed only after the successful capture marker.
Headless shader errors, optional USB setup, and inactive passenger coroutine messages remain in the source logs.
They do not establish rendering or vehicle-startup parity.
Canonical comparison limits remain four float ULPs for primitives and 0.0001 degrees for bone rotations.
Position tolerance remains the larger of 0.001 metres or eight source float ULPs.
Later product checks must use matched source inputs, math mode, previous road state, and the fixed generation frame.
