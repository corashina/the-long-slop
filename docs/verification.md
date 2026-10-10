# Garage verification

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
It reaches the road lane after 6.65 seconds with all four wheel contacts.
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
