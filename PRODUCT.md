# Product

<!-- impeccable:product-schema 1 -->

## Platform and stack

Local desktop browser. Rust, WebAssembly, Bevy, Rapier, and a static browser shell.
Keyboard and mouse controls. WebGL2 rendering.

## Purpose

Prepare a fixed Lada 2101 in the imported starting garage. Drive onto the endless desert road.
Installed Steam build 11140890 owns the reference values.
The approved garage spawn takes precedence over the original bedroom spawn.

## Scope

The build includes walking, first person look, crouching, jumping, seating, carrying, and aimed use.
Preparation includes doors, fluid caps, filling, spilling, ignition, and wheel assembly.
It retains the road, stations, chase camera, pause, recovery, and restart.
The interface uses source menus, action icons, fluid percentages, and animated Lada instruments.
Recovery preserves state. Restart restores the garage.

The source cycle changes sky, illumination, ambient light, and fog during active play.
Lights include headlights, cabin lights, rear lamps, five house switches, and automatic roadside lamps.
L and the two physical headlight controls share one car request.
House switches use E or eligible LMB. Cabin controls follow source modes and door links.
Pause freezes cycle time, fades, and switch movement. Recovery preserves these states.
Restart restores the garage and source startup state at 240 seconds.

Portable flashlights, survival, sleep, weather events, weapons, other vehicles, multiplayer, saves, audio, and deployment remain outside this stage.
Engine heat and fluid rules retain their current behavior.
The build does not claim identical Unity physics or full-game parity.

## Development assets

The complete local library covers installed build 11140890.
It preserves source files and typed records, and supplies converted media and model files for development.
The game uses the selected garage, fixed Lada, and preparation items.
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
