# Day cycle and lamp source findings

Date: 2026-10-10, Europe/Warsaw.
Status: Source analysis. Product implementation has not started.

## Reference and workspace

The user requests source parity for the day cycle, headlights, and existing lamps.
The user confirmed house switches, car lamps, and automatic roadside lamps.

Steam app 1017180, build 11140890, supplies the reference.
The installation uses Unity 2021.3.18f1 and Mono.
Assembly-CSharp.dll SHA-256:
`9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.

The isolated worktree starts at commit `743ef8b`.
Recovered C# and serialized records remain in its ignored `.cache/reference` directory.
Steam files remain unchanged.

## Day cycle

`level1:64920`, class `napszakvaltakozas`, controls the cycle.
`Start` converts minutes to seconds. A new game sets its offset to `dt * 0.2`.
Here, `dt` is 1200 seconds. The new-game offset is 240 seconds.
Dawn and dusk belong inside `daytime`; they do not extend the full cycle.

| Phase | Cycle interval | Duration |
| --- | --- | --- |
| Dawn | 0 through 240 seconds | 4 minutes |
| Day | Above 240 through 1020 seconds | 13 minutes |
| Dusk | Above 1020 through 1200 seconds | 3 minutes |
| Night | Above 1200 through 1500 seconds | 5 minutes |

`FixedUpdate` uses strict greater-than comparisons at these boundaries.
It resets the time origin after `t > dt + nt`; the next update starts the new cycle.
Its phase methods update sun pose, color, intensity, ambient color, sky exposure, and star color.
The same directional light supplies moon illumination during night.

| Property | Installed value |
| --- | --- |
| Day / noon / night intensity | 1.125 / 1.6 / 0.3 |
| Day / night ambient | 0.2 / 0.02 |
| Maximum / minimum sky exposure | 2 / 0 |
| Minimum / maximum reflection intensity | 0 / 0 |
| Day sun pitch | 40 through 140 degrees |
| Day sun yaw | 60 degrees |
| Night moon pitch | -50 through 250 degrees |
| Day / night fog density | 0.0000175 / 0.001 |

Sun colors use a serialized gradient: white, orange, then blue.
The phase methods use separate formulas for exposure, fog, cloud color, and star visibility.
A single sine curve would change source behavior.
`SetBiome` updates biome colors every second. The current procedural world requires an explicit desert biome mapping.
The source also handles sandstorms, sleep, and audio. These systems remain outside the approved product scope.

## Headlights and car lamps

`carscript.InteractSwitchLights` cycles `lampak` through 0, 1, and 2.
`settingsscript` assigns the original light control to L.
`headlightscript` uses separate requested state, current state, and energy.
On a state change, it drains energy, changes state, then restores energy.
The installed Lada uses `energyMove = 10`, or about 0.1 seconds per fade at full energy.
Frame boundaries add steps; the transition is not one fixed 0.2-second timer.

| Lada lamp | Shared prefab source | States |
| --- | --- | --- |
| Right headlight | `sharedassets1.assets:145180` | Off, low, high |
| Left headlight | `sharedassets1.assets:152454` | Off, low, high |
| Cabin lamp | `sharedassets1.assets:141594` | Off, on |
| Rear lamps | `sharedassets1.assets:145425` | Tail, brake, reverse |

The exporter uses the matching scene Lada, not these prefab defaults alone.
Scene headlight IDs are `level1:72934` and `level1:76810`.
Scene cabin and rear IDs are `level1:70896` and `level1:73055`.
Export must resolve those scene links and check prefab agreement.

Low beams use a 50-metre range and an 80-degree full cone.
High beams use a 250-metre range and the same full cone.
Both use source cookie texture `sharedassets1.assets:1278`.
State intensity is 1.25 for low and 1 for high, multiplied by energy and electronics output.
The serialized high light intensity is 1.25; `RefreshLights` overwrites it with the state intensity.
Cabin lighting uses a 3-metre range and on-state intensity 0.8.

Headlight state propagation does not require ignition.
Rear lamps require `localIgnition`.
Tail materials turn on for light states 1 and 2.
Brake lamps turn on above brake value 0.01. Reverse lamps use a negative selected gear ratio.
`interiorLightScript.RefreshInteriorLight` performs no changes in this build.
The Lada cabin uses `headlightscript`; implementing the obsolete helper would change behavior.

## House switches

The selected scene has five `flashlightscript` controls.
They start off, have no random start, and contain unbroken lamps.

| Control | Source | Lamp count |
| --- | --- | --- |
| Kitchen | `level1:67484` | 1 |
| Garage | `level1:68886` | 2 |
| Cellar | `level1:72019` | 1 |
| Back | `level1:75508` | 1 |
| Entrance | `level1:76065` | 1 |

`fpscontroller` resolves the aimed collider's parent switch.
Use or left click toggles it, subject to the source left-click guard.
`Refresh` changes light objects, lamp materials, and button pose.
The selected house lights use a 10-metre range and source shadow settings.
None of the selected house switches uses the SingleLight helper.

## Roadside lamps

The desert road prefab contains `kozlampa` at `sharedassets1.assets:144809`.
Its light is `sharedassets1.assets:123899`: a warm spotlight with range 20 metres and full cone 120 degrees.
`roadGenScript.PlaceOneRoad` registers each road segment's lamp through `FStart`.
`kozlampa.FStart` flattens its forward direction onto the horizontal plane.
The road generator also applies seeded pole rotation.

`mainscript` starts with roadside lighting enabled and its switched state off.
The cycle switches lamps on at dusk when phase progress exceeds 0.5.
It switches them off at dawn when progress reaches 0.5.
`kozlampa` also requires electronics output above 0.2.
Its electronics maximum is 1.5. `Update` overwrites light intensity with electronics output.

The current Rust road has no lamp entities.
Source pole geometry, road spacing, and placement transforms still need export recovery.
Keep the current road path. Matching the original road generator is outside this request.

## Renderer interfaces and verification

The current app supplies fixed sunlight, clear color, linear fog, and Reinhard tonemapping.
It disables glTF light loading. No current runtime component controls these lamps.
The approved material plan reserves `tools/reference/lighting.py` and `src/presentation/lighting.rs`.
The approved UI plan reserves dashboard lamp bindings and currently leaves the headlight signal off.
The new design must share these interfaces and supply the recovered headlight signal.

Bevy light power uses physical units. Unity intensity needs an explicit conversion.
Bevy spot angles use half-cone radians; copying Unity's full cone would be wrong.
Source beam cookies and light falloff need a WebGL2 capability check before choosing the rendering path.

Primary renderer documentation:
[Bevy 0.19.1 SpotLight](https://docs.rs/bevy/0.19.1/bevy/light/struct.SpotLight.html).

The isolated native baseline passed with `cargo test --locked`.
Six installed-export tests were skipped because the worktree has no reference export.
The analysis reader verified component headers and full byte consumption for selected custom records.
Three unrelated components have empty script links; the scan excludes those records.

Live original-game timing and matched browser views remain unverified.
Source values establish behavior targets. They do not establish identical rendered pixels.