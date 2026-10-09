# Garage reference findings

Checked on 2026-10-09. This record covers research. It does not record an implemented garage.

## Selected reference

The user selected installed Steam build `11140890` and a fixed Lada 2101 starter car.
The user requested a garage spawn, first-person control, and car preparation before road travel.
These requests extend the earlier driving-only scope.

Installation: `C:\Program Files (x86)\Steam\steamapps\common\The Long Drive`.
The Steam manifest confirms build `11140890`.
The installation contains Unity Mono assemblies and serialized scene assets.

## Current game

The Rust build starts the player inside a procedural car on the road.
It has chase and cockpit cameras. Neither camera supports mouse look.
It has no walking, seat entry, garage doors, loose objects, or fluid containers.
The simulation uses one fuel amount and enables thrust when fuel exceeds zero.
It consumes fuel during active play, without a separate ignition state.
The existing road uses streaming and a floating origin.

## Recovered code

I inspected assembly metadata and recovered C# with ILSpy `11.1.0.9782`.
I also disassembled selected method bodies with dncil `1.0.2`.
The assembly contains 637 type definitions, including nested and compiler-generated types.

I recovered these classes into the ignored `.cache/reference` folder:

- `fpscontroller`: walking, looking, object pickup, seat entry, and exit clearance.
- `carscript`: driving, ignition, handbrake, transmission, and attached component references.
- `enginescript`: starting, fuel quality, oil, heat, torque, and consumption.
- `door_rot`: hinge limits, current position, target position, and movement speed.
- `seatscript`: seat occupancy and driver control.
- `tankcapscript` and `tankscript`: caps, filling, pouring, capacity, and mixtures.
- `mainscript`: shared fluid types and the nested `fluidcontainer` class.
- `homesickscript` and `itemdatabase`: home references and item registration.

The original camera clamps vertical look to minus 89 through plus 89 degrees.
On foot, horizontal look rotates the player body. In a seat, it rotates the head within seat limits.
The exit code checks available space before moving the player outside.

The engine requires ignition, sufficient RPM, usable fuel, sufficient oil, and acceptable temperature.
It checks fuel quality above `0.3` and oil and heat penalties below `0.99`.
It also applies an engine-specific start chance.
Low coolant affects cooling and temperature. A mandatory coolant check before starting would change this behavior.
The engine scales output with oil and temperature penalties.
The fluid code computes suitability from the mixture required by each engine.
Caps and doors have separate state. Holding a container does not imply an open cap.
Direct filling checks the destination cap. It does not check source cap opening or source tilt.
Physical spilling checks the source valve and tilt. These are separate fluid paths.

These are code findings. I have not compared their timing with live original-game play.

## Recovered scene references

I inspected `level1`, `resources.assets`, and `sharedassets1.assets` with UnityPy `1.25.4`.
The scene contains car start markers, player objects, doors, meshes, and materials.
The garage door colliders belong to the `haz02` home hierarchy.
I recovered 434 transforms in that hierarchy, including local position, rotation, scale, and parent links.
It contains garage, cellar, and kitchen interior volumes, tables, a sink, hinged doors, and roof access.

I saved the hierarchy to `.cache/reference/hierarchy-186.json`.
I saved asset names and assembly metadata beside it.
The scene also contains `TestHome`. Its inactive state means its name alone cannot establish the runtime starting home.

Some custom component reads failed with generated serialization schemas.
I stopped the broad component scan. I have not established all original vehicle or player defaults.
Original meshes, textures, and colliders still need a tested conversion for Bevy and Rapier.

Targeted reads later recovered the fuel, sump, coolant, engine, and player components.
Generated schemas had a base-header alignment error. I replaced their base-header nodes with UnityPy's standard MonoBehaviour nodes.
I compared source class links with the standard header and required complete byte consumption.
The corrected reads confirmed fuel capacity `39 L`, sump capacity `5 L`, and coolant capacity `5 L`.
Serialized amounts are `0.5 L` gasoline, `2.5 L` oil, and about `3.6 L` water.
These amounts remain serialized evidence until the selected startup caller has been traced.
`FStart()` does not randomize fluids. `RandomFluid()` replaces them when a caller invokes it.
The engine records starter `700 RPM`, idle `800 RPM`, maximum `6600 RPM`, shutdown `550 RPM`, and maximum torque `87 Nm`.
The player records base speed `1.75`, run multiplier `2.5`, and aim reach `1.85`.
I saved the corrected reads to `.cache/reference/targeted-parameters.json`.

### Lada standard component checks

The complete Lada prefab is `sharedassets1.assets` GameObject `40132`, Transform `82556`, named `Car02Full`.
I verified its 697 transforms, engine and radiator references, and four wheel assemblies.
The bare `Car02`, GameObject `5152`, has empty attachment slots.
The selected scene home is GameObject `32`, Transform `21400`, named `haz02`.

The Lada root Rigidbody records 717 kilograms. This value does not establish runtime total mass.
Attachment and fluid mass rules still need a trace.
The four road wheel mounts have X coordinates of plus or minus `0.6582` and Y `0.024`.
Front wheel Z is `1.494`. Rear wheel Z is `-0.970`.
Each road WheelCollider has radius `0.300`, center Y `-0.100`, and suspension distance `0.300`.
An additional helper WheelCollider is named `PlusWheelToKeepAwake`. It is not a fifth wheel assembly.
I saved these standard component reads to `.cache/reference/lada-standard.json`.

The original Lada front points along positive Z. The current Rust car uses negative Z.
Some imported nodes use scale `100`. Wheel hierarchies also contain reflected transforms.
Export must convert geometry, winding, normals, colliders, and pivots together.

## Wiki evidence

The [Starting House](https://the-long-drive.fandom.com/wiki/Starting_House) page describes the house and its preparation supplies.
The [Getting Started](https://the-long-drive.fandom.com/wiki/Getting_Started) guide describes pickup, pouring, door use, seating, and ignition.
It describes a bedroom spawn. The requested garage spawn takes precedence.
The [Lada 2101](https://the-long-drive.fandom.com/wiki/Lada_2101) page describes gasoline and water cooling.
It lists a 39-liter fuel tank and a 5-liter cooling system.
It contains conflicting mass values. I will not select a mass from that page without installation evidence.
The [Developer menu](https://the-long-drive.fandom.com/wiki/Developer_menu) page maps `Car02` to the Lada 2101.

Direct page opens failed. Search-index results supplied the retrieved page content.
The wiki identifies itself as a community site. I have not verified developer endorsement.
Wiki facts remain secondary to recovered installed-build data.

## Verification limits

The game still uses Unity WheelCollider physics in the reference installation.
The Rust build uses Rapier raycast vehicle physics.
Asset recovery and selected code recovery do not establish identical physics or full-game parity.
Each implemented behavior will need an explicit comparison and a browser check.

Docker Desktop was not running during this research.
I did not run product tests or change product code.
Steam files remain unchanged. Recovered source and analysis packages remain in the ignored project cache.

## Analysis tool sources

- [ILSpy](https://github.com/icsharpcode/ILSpy).
- [UnityPy](https://github.com/K0lb3/UnityPy).
- [dnfile](https://github.com/malwarefrank/dnfile).
- [dncil](https://github.com/mandiant/dncil).
