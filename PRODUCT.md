# Product

<!-- impeccable:product-schema 1 -->

## Platform and stack

Local desktop browser. Rust, WebAssembly, Bevy, Rapier, and a static browser shell.
Keyboard and mouse controls. WebGL2 rendering.

## Purpose

Prepare a fixed Lada 2101 in the imported starting garage. Drive onto the source desert road.
A source Skoda waits in the yard as a second car.
Installed Steam build 11140890 owns the reference values.
The approved garage spawn takes precedence over the original bedroom spawn.

## Scope

The build includes walking, first person look, crouching, jumping, seating, carrying, and aimed use.
Preparation includes doors, fluid caps, filling, spilling, ignition, and wheel assembly.
It includes source road segments, supported structures and supplies, chase camera, pause, recovery, and restart.
The interface uses source menus, action icons, fluid percentages, and animated car instruments.
Recovery preserves state. Restart restores the garage and clears session changes.
The checked source road contains 50,000 segments. The off-road grid has no finite extent.
The normal seed is 42. Generation uses the saved 5,000-metre default source settings.
The runtime retains stable source identities, bounded source replay inputs, and uncapped actual change records.
Changes include supported physics motion, fluids, attachments, mechanisms, and lamps. Unloaded objects do not simulate.
A full source-history budget can prevent further first encounters. Accepted collision support remains available.
Recovery destinations use actual supported supplies. A water tank does not imply available fuel.

The source cycle changes sky, illumination, ambient light, and fog during active play.
Lights include headlights, cabin lights, rear lamps, five house switches, and automatic roadside lamps.
L and the physical headlight controls share one request per car.
House switches use E or eligible LMB. Cabin controls follow source modes and door links.
Pause freezes cycle time, fades, and switch movement. Recovery preserves these states.
Restart restores the garage and source startup state at 240 seconds.

Adjustable mirrors show live views. The renderer covers source shadows, sky, reflections, post effects, and stained glass.
Source audio covers ignition, engine, horn, interaction sounds, and the Lada radio.
One manual browser save slot stores both cars, items, mechanisms, pumps, and lamps.
Driving includes source manual gears, clutch, Automatic clutch, and Automatic gears.
The save stores selected gears and both assist choices. Automatic gears requires Automatic clutch.

The cleaning branch adds two brushes, two sponges, and two spray cans from the authored startup scene.
Use carried tools with LMB. Rub an eligible surface with a brush or sponge; hold LMB to spray paint.
Brushes and sponges wear during work. Spray cans use their exported color and lose paint during firing.
The save contract stores surface state, color, progress, tool wear, paint quantity, and wash state.
Load preserves these values. Recovery preserves preparation; New Game restores exported startup values.
Looked-at paint quantity uses a bare rounded count. Carried cans hide that count and the native Paint prompt.
Brush, Sponge, and Paint are port item labels. Wash and Scrub use checked native prompts.

The final particle fix uses frozen code commit `1b51c388`.
Its release build and export verification passed.
The controller checked final-package garage readiness, New Game, and fresh-export Save/Load.
Continue failed pointer capture and left the menu paused.
See [verification](docs/verification.md) for final package hashes and check limits.
Active tool use, both-car driving, retained effect counts, and matched source views remain unverified.
Spray particles apply source local velocity, inherited emitter motion, and masked surface collision through the shared particle pool.
Spray scene-color refraction and brush soft-particle depth fading remain bounded shared-renderer deferrals.
These known visual differences keep background refraction absent and brush intersections hard.
The roadmap keeps Cleaning and painting Partial until the required checks and shared-renderer work pass.

Survival includes health, food, water, bladder, bowel, sausage, toilet actions, healing, starvation, and death.
Sleep follows source eligibility, hold progress, wake, and clock rules.
Weather simulation and weather reference data are removed and outside current scope.
Normal day/night, clouds, distance fog, ambient sound, and shared cleaning, fluid, and cigarette particles remain.
Engine heat follows the source cycle's ambient temperature.

Portable flashlights and source license plate text are implemented.
Fatigue, beds, creatures, weapons, and further vehicles remain outside this stage.
Saved needs, autosave, general settings, control rebinding, multiplayer, and deployment remain outside this stage.
The two driving assists are the approved settings exception.
The build does not claim identical Unity physics or full-game parity.

## Development assets

The complete local library covers installed build 11140890.
It preserves source files and typed records, and supplies converted media and model files for development.
The game uses the selected garage, the Lada, the Skoda, and preparation items.
Library availability does not expand gameplay scope.
Engine-dependent records require separate browser adapters before use.

## Evidence

The user approved the [lighting specification](docs/superpowers/specs/2026-10-10-day-night-lighting-design.md) on 2026-10-10.
The earlier garage approval remains in `docs/superpowers`.
Read [verification](docs/verification.md) for source checks, browser evidence, and rendering limits.
The installed build uses Unity 2021.3.18f1 and Mono.
The exporter records input hashes, source IDs, geometry, and recovered parameters.
Generated original assets remain outside version control.
The browser stops loading when required reference data is missing or invalid.

## Principles

Show the loaded garage before Start. Keep controls and state readable.
Use one fluid state and one physics world. Build each working layer before extending it.
