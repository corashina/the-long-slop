# Verification record

The source UI section records the latest UI checks.
The garage section retains earlier physics and scene evidence.

## Garage verification

Reference: installed Steam build 11140890. Vehicle: fixed Lada 2101.
Verification date: 2026-10-10, Europe/Warsaw.
Verified implementation commit: `a8dd272`.
The approved design is in `docs/superpowers/specs/2026-10-09-garage-start-design.md`.

## Reference recovery

The exporter follows the selected scene links to the home and Car02Full hierarchy.
It converts handedness once. It preserves nested scale and reflected wheel transforms.
Source IDs and input hashes remain in the generated manifest.
The generated export stays outside version control.

The startup audit confirmed 0.5 L gasoline, 2.5 L oil, and 3.6 L water.
The assembled car has 955 kg dry mass and 6.6 kg initial fluid mass.
The selected home contains the preparation cans, barrel, faucet, and spare wheel.
Installed wheel slots follow their individual part links.

## Checks

| Check | Result |
| --- | --- |
| Formatting and native suite | 91 passed; six source gates skipped by default |
| Installed-source gates | Six passed in explicit runs |
| Exporter suite | Four passed |
| Release WASM and browser package | Passed |
| Independent review | Passed after the sleeping-ground removal fix |

Native checks cover fluids, ignition, heat, collision, exit clearance, doors, wheels, pause, restart, recovery, and rebasing.
Source checks use the actual export. Synthetic geometry stays in native fixtures.
The source interaction tests stage clear player positions. They do not prove a continuous browser walking route.

The actual source flow opens both garage leaves through aim and Use.
It enters the recovered driver seat and closes the driver door from the seat.
The car starts and reverses through the garage onto the road.
It reaches the generated road lane after 5.65 seconds. All four wheels land within four more steps.
After braking, it continues beyond the lane. This check proves the join, not parking within the lane.

The car remains still for 90 seconds with the source spare wheel present.
Sleeping linear and angular velocity stay zero. All four wheels retain support.
After a 95-second rest, door use, seating, steering, and reverse wake the car without a jump.
Removing a supporting terrain collider also wakes a sleeping car.
The original inspection pit and cellar remain clear.

Seat collider aliases follow the source `mainseat` links.
The driver follows `driverSeat0`, source component 69296.
Use rays apply mask 12747265. Exit overlap queries apply mask 10125313.
Exit clearance uses capsule endpoints at plus or minus 0.568 meters and radius 0.25 meters.
Each imported dynamic body uses its recovered linear and angular damping.

Logs are in the ignored `.cache` directory:
`garage-oct10-final-check.log`, `garage-oct10-final-source.log`,
`garage-to-road-source.log`, and `garage-oct10-final-build.log`.

## Browser evidence and limits

Earlier browser checks showed the imported home and car, camera look, walking, doors, seats, ignition, pause, and restart.
The embedded browser rejected pointer lock. A temporary input fixture supplied browser events during those checks.
The fixture did not change native gameplay state. The product contains no capture fallback or test fixture.
A blocked texture kept Start disabled and named the failed glTF dependency.

On 2026-10-10, Computer Use stopped before final UI input.
It could not verify the current browser URL with enough confidence to enforce policy.
The final continuous browser garage-to-road route remains unverified.
The user requested that this test be skipped on 2026-10-10.
Native source tests verify that route's vehicle and interaction steps.

The earlier 1280 by 720 measurement used an RTX 4070 Ti with ANGLE Direct3D11.
After warmup, 300 frame intervals averaged 7.82 milliseconds. Median was 8.0; 95th percentile was 8.2.
These are earlier stationary garage measurements, not a benchmark of the final parking fix.
The last measured reload reached scene readiness in about 3.0 seconds over localhost.
The final WASM is 50,518,962 bytes. Reference export files total about 76.4 MB, including recovery evidence.

## Material and physics limits

Rapier raycast suspension replaces Unity WheelCollider physics. Identical handling is not established.
Recovered source parameters bind dimensions, spring values, damping, brakes, condition, and drivetrain ratios.
The adapter uses Rapier solver and sleeping rules. It does not reproduce Unity's complete vehicle solver.
The fixed garage spawn replaces the original bedroom spawn by user request.
Seeded wheel condition gives repeatable startup within the recovered source ranges.

The exporter maps Unity materials to glTF materials.
It selects one high detail LOD layer. It does not reproduce all Unity shader effects.
Mirror surface materials appear, but live RenderTexture reflections are omitted.
Stale editor text meshes are omitted. Runtime plate text generation is not reproduced.
Audio remains outside this stage.

The player wrapper adds no allocation over the measured Rapier static movement path.
Rapier allocated 720 blocks and 374400 bytes across 120 static movement steps.
Its dynamic impulse path allocated five blocks and 1840 bytes in the measured case.
These library allocations remain. Do not interpret them as zero allocation for the complete game.

## Research sources

The [Starting House](https://the-long-drive.fandom.com/wiki/Starting_House),
[Getting Started](https://the-long-drive.fandom.com/wiki/Getting_Started), and
[Lada 2101](https://the-long-drive.fandom.com/wiki/Lada_2101) wiki pages provided secondary context.
The site is a community wiki. Installed-build evidence controls conflicting values.
See `docs/research/2026-10-09-garage-reference.md` for the recovery record.


## Source UI verification

Scope: approved current features in docs/superpowers/specs/2026-10-10-source-ui-design.md.
Source: installed Steam build 11140890.
Worktree: source-ui. Base: 743ef8b.
Date: 2026-10-10, Europe/Warsaw.

The UI now uses recovered menu sprites, canvas layouts, action icons, and fluid presentation flags.
The old serif shell, persistent vehicle card, X assembly action, and P/R shortcuts are removed.
F press picks up, drops, or dismounts. F release mounts.
RMB moves a held item. MMB rotates it.
Esc opens the menu. Backspace toggles feedback.
Recover remains in the pause menu. Fullscreen remains in Controls.

### Final checks

| Check | Result |
| --- | --- |
| Rust formatting and native suite | 109 passed; six installed-source gates skipped by default |
| Explicit installed-source gates | Six passed |
| Browser module suite | 14 passed |
| Python exporter suite | 11 passed |
| Complete installed-build export | Passed |
| Release WASM and browser package | Passed |
| Independent source review | One Important finding fixed; no Critical findings |
| Final diff and source-asset exclusions | Checked before the integration commit |

Commands use the source-ui worktree and its separate Cargo target volume.
The ignored .cache/check.ps1 and .cache/build.ps1 pin that volume.
The tracked scripts contain the same checks with the normal project cache.

- Python: unittest discover -s tools/reference/tests -v.
- Browser modules: node --test tests/web/menu.test.mjs tests/web/input.test.mjs tests/web/feedback.test.mjs.
- Native and release: .cache/check.ps1.
- Installed gates: cargo test --locked -- --ignored in the toolchain container.
- Export: tools.reference.export with the installed Steam root.
- Package: .cache/build.ps1.

Final logs are retained under ignored .cache/source-ui-verification.
They include final-check.log, final-source.log, final-python.log, final-node.log, final-export.log, and final-package.log.

### Review corrections

The independent reviewer found emission on the source warning lamps' Off materials.
Unity's _EMISSION keyword now controls both emission factor and texture export.
A focused regression test failed before the fix and passed afterward.
Regenerated Off materials have zero emission for fuel, oil, battery, and headlight2.
Their On materials retain the recovered emission values.

Browser startup exposed a late glTF material load that reloaded the vehicle scene.
The renderer now retains raw glTF handles during initial loading.
A startup adapter creates and caches the used lamp materials.
The final browser reaches the ready menu with the vehicle present.

Static review found clipped carrying text.
Its bottom alignment now follows the recovered lower-right text rectangle.
The confirmation capture shows all three lines above the bottom margin.

### Browser evidence

The final browser starts at http://127.0.0.1:4174/.
Desktop review used 1280 by 720 pixels. Narrow review used 390 by 720.
Start and Controls were inspected over the actual loaded garage.
Pause, failure, and fluid feedback were inspected with an isolated UI fixture.
That fixture contains no WASM runtime, simulated input, or product capture bypass.
It was removed after review.

Captures are under ignored .impeccable/review:

- source-ui-start-final.png: actual ready menu and vehicle.
- source-ui-controls-desktop.png and source-ui-controls-narrow.png: actual Controls.
- source-ui-pause-fixture.png: source pause geometry and session distance.
- source-ui-error-fixture.png: disabled actions and a readable failure.
- source-ui-fluid-fixture.png: source fluid text, fill icon, and complete carrying help.

The Impeccable detector ran once over web.
It reported three Arial warnings and one unrecorded error color.
Arial is the recovered gameplay font. The error color is now recorded in the palette.
Two files had invalid ellipsis bytes. They were repaired and validated as UTF-8.
The detector could not inspect those files before repair. It was not rerun.

The embedded browser rejects pointer lock.
Live pause and contextual gameplay remain covered by state and native tests, plus static layout review.
The approved final continuous browser route waiver remains.
No live pixel-parity claim is made.

### Instruments and limits

| Instrument | Source range | Unity Y angle |
| --- | --- | --- |
| Speed | 0 to 160 km/h | -64 to 64 degrees |
| Temperature | 10 to 130 degrees Celsius | -30 to 30 degrees |
| Fuel | 0 to 39 L | -26 to 26 degrees |

Needles use source signal smoothing at 5 times dt and value smoothing at 8 times dt.
The five odometer digits use base 10, one fractional digit, and source carry rules.
They use source X rotations and angle smoothing at 20 times dt.
The display starts from the recovered value and adds session travel.
Recovery and coordinate rebasing preserve it. Restart resets it.
The source fuel, oil, and battery guards use local ignition and recovered thresholds.
The unsupported headlight indicator remains off. The Lada has no RPM needle.

Entity and material binding occurs during loading.
Instrument updates use fixed arrays and cached handles.
Feedback retains display values and changes DOM text only when needed.
Browser text uses Liberation Sans and Arial. It differs from Unity's SDF renderer.
Liberation Sans 2.1.5 provenance and license remain in web/fonts.
Generated original assets remain excluded from Git.

### Execution rulings

- PowerShell briefs replace unavailable Bash bookkeeping helpers.
- A separate Cargo target cache prevents stale metadata from concurrent work.
- Menu synchronization uses three scalars to avoid a new frame object.
- Recovered Gas, empty, and (full) labels override illustrative plan wording.
- Cached startup material conversion replaces late subasset loading. Bevy's converter is private.
- Design documentation ran inline under the approved native workflow.

The design record uses the actual source palette, menu and gameplay fonts, canvas rules, and source text shadows.
Its sidecar records color metadata, motion, breakpoints, and standalone text components.
Original sprite previews require the ignored local export and remain outside the sidecar.
No independent-review Minor findings remain deferred.

## World generation

Verification date: 2026-10-10, Europe/Warsaw.
Sources: `roadGenScript` 64771, `TerrainGenerationSettings` 64898, and `terrainHeightAlignToBuildingScript` 79970 in `level1`.
Draw distances come from `settingsscript` 16865 in `level0` at the default slider.

The `System.Random` port returns the published .NET values for seeds 0 and 42.
The Perlin port returns the engine lattice value 0.4652731.
The serialized seed is -1705217151. The source picks a new seed for each new game. This build keeps the serialized one.
The source lifts the home onto the terrain. This build keeps the home fixed and lowers the world by 578.897 meters.

| Check | Result |
| --- | --- |
| Formatting and native suite | 90 passed; six source gates skipped by default |
| Installed-source gates | Six passed in explicit runs |
| Exporter suite | Four passed |
| Release WASM | Passed |

Native checks cover tile seams, skinned road bone ends, streaming bounds, recovery, rebasing, and home flat zones.
The source terrain sits 0.1 meters below the road within 4.5 meters of it. A wheel can leave that lip for a moment.

A headless Chromium run with SwiftShader showed the road, poles, sand terrain, and home.
That run used a temporary roadside spawn in a copied manifest. It is not a performance measurement.

Distant terrain layers, the `TerrainBiome` blend shader, building holes, power lines, lamps, and grass are not reproduced.

## Part condition and crashes

Verification date: 2026-10-10, Europe/Warsaw.
Sources: `partconditionscript`, `partscript.WouldFallOff`, `carscript.DamageStuff`, and `gumiscript` in `Assembly-CSharp.dll`.
Crash values come from `mainscript` 64919 and the Lada `carscript` in `level1`.

Each part keeps the source condition state from 0, new, to 4, rusty.
The radiator and tire effects read that state. A rim without a tire uses the source rim stiffness.
A crash is a change of forward speed of 15 to 40 km/h in one step. A rusty car body scales both speeds by 0.2.
Each part needs its source fall-off power. A rusty part scales that power by 0.2.
The exporter copies the static `fallOffPowerNeeded` table. It is code, not serialized data.
The source checks once per rendered frame. This build checks once per 60 Hz step.

The exporter copies 20 `partslotscript` slots: doors, mirrors, hood, trunk, bumpers, lights, cabin parts, radiator, and engine.
A removed part takes the source `DT` pose when the slot has one. It keeps its pose when the slot has none.
Items attached to the part, such as a mirror or plate, go with it.
A part fits only into its own slot. The source also accepts other matching slots. Hubcaps stay on.
The car does not run without the engine. Without the radiator it has no radiator cooling or leak.

| Check | Result |
| --- | --- |
| Formatting and native suite | 92 passed; seven source gates skipped by default |
| Installed-source gates | Seven passed in explicit runs |
| Exporter suite | Four passed |
| Release WASM | Passed |

A headless Chromium run loaded the garage with the part export. The car parts render in place.
That run did not drive a crash or fit a part. Native and source checks cover those steps.

## Day cycle and headlights

Verification date: 2026-10-11, Europe/Warsaw.
Sources: `napszakvaltakozas`, `carscript.InteractSwitchLights`, and `headlightscript` in `Assembly-CSharp.dll`.
Cycle values come from the `napszakvaltakozas` component in `level1`.

A cycle is 20 minutes of day and 5 minutes of night. Dawn takes 4 minutes. Dusk takes 3 minutes.
The game starts 4 minutes into the cycle, as the source does.
The sun angle, color gradient, intensity, and flat ambient follow the four source phases.
At night the light follows the source moon angles.
The fog color mixes the source desert, grass, and winter colors by the terrain color at the player.
The terrain color is sampled once per second.

L cycles the source lamp states while the player drives: off, low beam, high beam.
Each fitted headlight fades out, changes state, and fades in at the source rate.
A removed headlight keeps its last state, as in the source.
The lens glows with the source emission color of each state.

Unity light units have no exact Bevy match. This build uses these approximations:

- One Unity intensity unit is 8000 lux of sun light.
- One Unity ambient unit is 400 nits. The source day ambient then equals the Bevy default.
- A spot light matches the Unity falloff at one fifth of its range. Bevy uses inverse square falloff, so near surfaces are brighter than in the source.
- The lens emission is the source emission color times 8000 / π nits.

WebGL2 has no Bevy atmosphere. The sky color is the source fog color. The fog keeps the linear 110 to 430 meter range.
Not ported: street lamps, the interior light, rear lamps, stars, clouds, the sun flare, heat haze, sandstorm changes, light cookies, shadows, and the debug time keys.

| Check | Result |
| --- | --- |
| Formatting and native suite | 93 passed; eight source gates skipped by default |
| Installed-source gates | Eight passed in explicit runs |
| Exporter suite | Four passed |
| Release WASM | Passed |

A headless Chromium run showed the garage in day light and at the end of dusk.
A test build with the high beam on lit the garage wall in front of the car.
