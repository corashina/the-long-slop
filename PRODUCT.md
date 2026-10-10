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

Survival, weapons, other vehicles, multiplayer, saves, audio, and deployment remain outside this stage.
The build does not claim identical Unity physics or full-game parity.

## Evidence

The approved design and plan are in `docs/superpowers`.
The installed build uses Unity 2021.3.18f1 and Mono.
The exporter records input hashes, source IDs, geometry, and recovered parameters.
Generated original assets remain outside version control.
The browser stops loading when required reference data is missing or invalid.

## Principles

Show the loaded garage before Start. Keep controls and state readable.
Use one fluid state and one physics world. Build each working layer before extending it.
