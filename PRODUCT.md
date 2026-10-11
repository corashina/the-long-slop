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
The road and terrain follow the source generator: seed, noise, road path, biomes, and power poles.
Placeholder stations sit beside the road. The build retains the chase camera, pause, recovery, and restart.
The interface uses source menus, action icons, fluid percentages, and animated Lada instruments.
Recovery preserves state. Restart restores the garage.
Part condition follows the source states. Hard crashes knock worn wheels and parts off.
The player can remove a part and fit it again into its own slot.
The source day cycle drives the sun, ambient light, and fog color. The car headlights have low and high beams.
Sound follows the source rules with the original clips. It covers the engine, car, steps, doors, caps, wind, ambience, and menu buttons.

Survival, weapons, other vehicles, multiplayer, saves, and deployment remain outside this stage.
Unity shader effects and live mirror reflections are not reproduced.
Terrain uses the source sand texture only. Biome blending and distant terrain are not reproduced.
Sound has no radio, horn, door squeak, pour sound, or Doppler shift.
The build does not claim identical Unity physics or full-game parity.

## Evidence

The approved designs are in `docs/specs`. Open plans are in `docs/plans`.
The installed build uses Unity 2021.3.18f1 and Mono.
The exporter records input hashes, source IDs, geometry, and recovered parameters.
Generated original assets remain outside version control.
The browser stops loading when required reference data is missing or invalid.

## Principles

Show the loaded garage before Start. Keep controls and state readable.
Use one fluid state and one physics world. Build each working layer before extending it.
