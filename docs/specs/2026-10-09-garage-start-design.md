# Garage start and first-person control

Date: 2026-10-09, Europe/Warsaw.
Status: The user approved this written specification in chat on 2026-10-09.

## Intent and scope

The player starts inside the garage, prepares a Lada 2101, and drives onto the existing desert road.
The user selected installed Steam build `11140890` as the reference.
The user approved original home and Lada asset import, first-person control, doors, fluid preparation, wheels, and seating.
The user also authorized subagents where they reduce work time.

This stage extends the earlier driving-only scope. It retains Rust, WebAssembly, Bevy, Rapier, and localhost hosting.
It replaces the procedural starter car and road spawn. It does not retain a second startup path.
Two requested differences from the original remain explicit: garage spawn and a fixed Lada 2101.

Match the installed home's layout, object transforms, selected vehicle assets, and the preparation behaviors described below.
Use recovered code and measured asset values as evidence. Do not claim full-game or identical Unity physics parity.
Keep Steam files unchanged. Keep recovered C# and generated original assets outside version control.

This stage covers the start-to-road loop. It does not add survival, weapons, other vehicles, multiplayer, saves, or deployment.
Home decoration can appear as geometry. Only preparation objects receive new gameplay behavior.

## Reference and asset contract

The [research record](../research/2026-10-09-garage-reference.md) records sources, recovered code, and unresolved component reads.
The inspected home hierarchy is `level1` GameObject `32`, Transform `21400`, named `haz02`.
Use `sharedassets1.assets` GameObject `40132`, Transform `82556`, named `Car02Full`, for the assembled Lada.
It contains 697 transforms, the engine, radiator, and four wheel assemblies.
The bare `Car02` root, GameObject `5152`, has empty attachment slots.
Identify road wheels through their slots. Audit auxiliary colliders such as `PlusWheelToKeepAwake` separately.
Resolve the starting home and Lada references through their scene and prefab links before export.
Names alone do not establish the selected root. Do not use the inactive `TestHome` as the starting layout.

Use a repeatable local extraction command with an explicit Steam installation argument.
Validate app ID, build ID, required roots, and required component values before accepting its output.
Export standard glTF assets and one JSON manifest into an ignored browser asset directory.
Use Bevy's existing glTF loader. Enable its required features and image decoders instead of writing a runtime mesh loader.
Generated assets contain the home, Lada, preparation containers, and wheel parts required by this scope.
The manifest records source file IDs, object IDs, hierarchy paths, asset hashes, and recovered gameplay values.

Preserve local transforms, hierarchy, door pivots, vertex normals, UVs, material textures, and alpha modes.
Convert Unity handedness once during export. Apply the same conversion to geometry, rotations, colliders, and interaction anchors.
Reverse triangle winding where the conversion requires it.
Preserve nested scale, including door parents with scale `100` and reflected wheel nodes.
Convert original collider geometry separately from visual meshes. Trigger volumes must not become solid walls.
Bake static collision geometry offline. Use supported primitive or convex shapes for moving objects.
Preserve collision filters separately from trigger and solid classifications.
Movable doors use their imported hinge transforms and corresponding moving colliders.
Car doors, lids, wheels, caps, seats, and fluid openings have stable manifest IDs.

Recover capacities, initial fluids, engine constants, player dimensions, movement values, and seat limits from the selected assets.
Distinguish serialized prefab values from changes made during original startup.
Trace startup rules and validate instantiated fluid amounts, wheel geometry, and assembled mass before accepting these parameters.
Decode only the required component fields. Validate byte consumption and plausible data structure sizes.
If a required field cannot be recovered, report the exact field and stop that checkpoint before inventing its value.
Wiki values can identify conflicts. They do not replace unresolved installed-build values.

Start stays disabled until required assets, colliders, and interaction anchors load.
Missing or invalid data produces an error naming the failed asset or field. No procedural replacement loads after an error.
Adapt original materials to the browser renderer and record material differences during visual comparison.
Export and load assets once. Reuse meshes, materials, handles, and buffers during play.

## Area and world

Place the imported home, required exterior features, parked Lada, and preparation supplies at one fixed absolute location.
Recover their relative placement from the installed starting area.
Put the player at a clear standing position inside the garage, facing the Lada.
Close the garage doors at restart. Park the complete Lada with ignition off and handbrake engaged.
These restart choices provide a repeatable preparation flow. Record any differences from the installed default state.
Use the recovered initial fluid amounts and wheel conditions. Do not invent mandatory missing parts or empty tanks.

Preserve the existing streamed desert beyond the starting area.
Flatten only the ground required by the imported floor, exterior path, and driveway.
Join the driveway to the road without a vehicle teleport. Prevent generated rocks, stations, and terrain from intersecting the home.
Give the home an absolute transform independent of road chunk ownership.
Keep its gameplay state throughout the session, including moved supplies, doors, and detached parts.
Reuse that state if the player returns after driving away.

Use the current floating origin for the home, player, vehicle, carried objects, loose objects, and interaction targets.
Center streaming on the player on foot and the vehicle while seated.
Rebase every affected physics body, collider, render pose, and stored local target between fixed steps.
Keep absolute positions, velocities, fluid quantities, part attachments, and trip distance unchanged by rebasing.

## Player and camera

Use one first-person camera on foot and in the driver's seat.
Keep the existing chase view as a seated camera option. First-person view is the startup default.
Use mouse look with pointer lock after Start or Resume. Escape releases capture and pauses.
Focus loss and capture loss clear held input and pending mouse movement.
Resume requires a user gesture and discards elapsed background time.
Show a useful error if the browser rejects capture. Keep Resume available for another attempt.

On foot, yaw rotates the body and pitch rotates the view. Clamp pitch to minus 89 through plus 89 degrees.
In a seat, rotate the view relative to the imported seat pose and apply its recovered yaw limits.
Clamp seated pitch to minus 89 through plus 89 degrees.
Use recovered camera height, field of view, and movement values.
Walk, run, jump, and crouch through Rapier capsule movement and collision queries.
Use Rapier's character controller and its dynamic-body impulse support for movement and pushing.
Grounding, slopes, steps, and crouch clearance must respect imported colliders.
Prevent standing where the full capsule has no room. Do not permit walking through doors, walls, or the car.

Enter an unoccupied seat through the aim interaction. Only the driver's seat enables vehicle input.
Disable the walking collider while seated. Preserve vehicle-relative camera motion.
Check capsule clearance along the current view direction before exit, as the recovered code does.
If blocked, retain the seat and show a short prompt. Do not teleport through a wall.
Exit clears driver input and occupancy. It does not stop ignition or close doors.
The vehicle continues its simulation while the player walks.

## Preparation interactions

Use one physics aim query to select the nearest visible interaction within the recovered reach.
Solid surfaces block targets behind them. Show the target name, available action, and relevant fluid quantity or part condition.
Use contextual input. On foot, WASD moves, Shift runs, Control crouches, and Space jumps.
E uses the selected target. Hold E and move the mouse to adjust a door or lid.
F picks up or drops an object. Mouse controls adjust its distance and rotation.
Hold LMB to fill the selected open destination. Hold I to use the starter while seated.
Space toggles the persistent handbrake while seated. C changes the seated camera. Escape and P pause.
Confirm these bindings against recovered input settings when extracting the reference. Record intentional browser binding differences.

Garage doors and Lada doors, hood, and trunk preserve hinge state separately from target state.
Use recovered axes, limits, and movement speeds. Update their visuals and colliders together.
Caps toggle their valve state. Containers remain physical objects after pickup, drop, or storage inside the car.
Carry objects through a reusable constraint and collision queries. Do not let carried geometry pass through walls or the car.
Mounted and loose wheel parts use the same item identity. Attachment must not duplicate or delete an item.

Implement wheel removal, tire and rim separation, compatible assembly, and installation in the selected wheel slot.
Keep condition when moving a part. Reflect installed condition in wheel grip.
An absent wheel removes that wheel's support and thrust. Installation restores the correct wheel pose and contact state.
Keep chassis and loose-part collisions consistent during removal and installation.

## Fluids and vehicle

Represent fluid quantities by type. Transfer mixtures without changing their proportions.
Conserve volume during transfer. Clamp transfer by available source volume and remaining destination capacity.
Direct filling requires the destination cap to be open. The recovered direct-fill code does not require source tilt or an open source cap.
Stop transfer when LMB releases, aim leaves the opening, the source empties, or the destination fills.
Physical spilling requires the source valve to be open and the container tilted.
Apply the recovered opening and tilt rules to spill flow. Remove spilled volume from the source without creating another fluid item.
Expose gasoline, oil, and water supplies needed for the Lada. Preserve other recovered fluid types present in those containers.

Store engine ignition, starter progress, RPM, running state, temperature, oil, coolant, and fuel suitability separately.
A short ignition press toggles ignition on release. Holding past the recovered starter delay engages the starter.
Use recovered engine start chance and constants. Tests use a fixed random seed.
Require ignition and the recovered RPM, fuel suitability, oil penalty, and heat penalty conditions for engine running.
Preserve the selected Lada's electronics gate on effective ignition. Electronics repair remains outside this stage.
Fuel suitability must exceed `0.3`. Oil and heat penalties must remain below `0.99`.
For the four-stroke engine, compute the oil penalty from oil volume against 30 percent of sump capacity, then cube it.
Apply fuel suitability to supplied fuel and target RPM. Apply oil and heat penalties to output.
Consume fuel while running and oil under the recovered operating conditions.
Coolant affects cooling and temperature. Do not add a coolant-only start lock.
At engine shutdown, retain coasting, steering, and brakes.
Use recovered wheel anchors, suspension dimensions, engine values, and runtime mass with the Rapier vehicle adapter.
The serialized root mass is 717 kilograms. Recover attachment and fluid mass rules before treating it as total vehicle mass.
Document measured handling differences from Unity WheelCollider behavior.

Replace the old seated Hold E refuel shortcut with aimed filling through the car's fuel opening.
At existing roadside stops, let the player refill a carried container at the pump, then fill the car.
Station stock remains unlimited, as in the existing road slice. Fluid transfer still respects container capacity.
Keep recovery as an explicit game action. Recovery preserves fluids, attachments, and trip distance, and chooses a clear road pose.
With exhausted fuel, retain station recovery so the existing fuel loop remains playable.
Recovery does not restore missing parts. Restart restores the complete garage state, supplies, origin, seed, and trip distance.

## Module boundaries and schedule

| Module | Responsibility |
| --- | --- |
| `reference` | Typed manifest, validated asset IDs, and recovered parameters |
| `starting_area` | Home placement, preparation objects, restart state, and area colliders |
| `player` | Movement, look, seat occupancy, carry state, and exit clearance |
| `interaction` | Aim selection, contextual actions, doors, caps, and part attachment |
| `fluids` | Mixtures, capacity, transfer, spills, engine oil, and coolant |
| `vehicle` | Rapier bodies, wheels, installed parts, vehicle forces, and collision queries |
| `simulation` | Fixed clock, ignition and engine state, pause, travel, and recovery |
| `presentation` | Imported visuals, interpolation, camera, and moving-part transforms |
| `world` | Road streaming, starting-area exclusion, terrain join, and origin |
| `browser` and `app` | Input bridge, loading, UI, and ordered integration |

Share one Rapier world. Keep simulation modules independent of Bevy and the DOM where practical.
Pass stable item and part IDs across module boundaries. Do not expose render entities as gameplay identity.
Keep one authoritative Rust gameplay state. The browser reads HUD state and sends input.
Retain cached DOM writes. Avoid allocating input objects, query results, meshes, or materials during frame updates.

Each fixed step processes interactions, player movement, engine state, vehicle forces, physics, travel, streaming, and rebasing in order.
Rendering interpolates committed poses and updates the camera and HUD. Rendering does not change fluid or part state.
Pause freezes gameplay, physics, doors, spills, ignition, and consumption.
Restart clears carry constraints and input before rebuilding the initial state.
Remove the old bodies, colliders, and constraints. Reset interpolation poses and stored local targets with the origin.
Update product documentation and controls after implementation. Remove obsolete startup and refuel instructions.

## Acceptance

Build each layer on a working product. Verify asset loading and the garage-to-road join before adding preparation mechanics.
Then verify walking and seating, fluids and engine state, and wheel replacement.
Run the repository formatting, native tests, WASM build, and browser checks before delivery.
Use subagents only for independent work after shared interfaces and file ownership have been defined in the plan.

Critical native tests cover collision and exit clearance, volume conservation, start and shutdown, wheel attachment, pause, restart, and rebasing.
Check blocked targets, missing wheels, bad fluid mixtures, full containers, and held input during focus loss.
Retain meaningful road streaming and vehicle regression coverage. Replace tests for removed startup and refuel behavior.

Browser acceptance follows one complete flow: start in the garage, walk, inspect, fill, replace a wheel, enter, start, and drive out.
Check closing and opening garage doors against collision geometry.
Check crouch clearance, blocked exit, seated look, carried objects, pouring, spilling, overheating, pause, and restart.
Check a short ignition tap, a held starter, and filling with a closed destination cap.
Repeat restart after carrying an object, detaching a wheel, and crossing the rebase threshold.
Drive across chunk and rebase thresholds, exit the car, and return to the starting area.
Record screenshots and interaction evidence against the installed assets and recovered rules.
Record material and physics differences. Do not label unverified comparisons as 1:1.
Measure browser frame pacing at the existing 1280 by 720 target and compare it with the current recorded baseline.
Record asset size, loading failures, frame measurements, and incomplete checks in the verification record.
