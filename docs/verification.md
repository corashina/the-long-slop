# Garage verification

Reference: installed Steam build 11140890. Vehicle: fixed Lada 2101.
Garage verification date: 2026-10-10, Europe/Warsaw.
Garage implementation commit: `a8dd272`.
The lighting section records the 2026-10-11 branch checks.
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

## Earlier garage checks

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

## Day cycle and lamps: 2026-10-11

The user approved [the lighting specification](superpowers/specs/2026-10-10-day-night-lighting-design.md) on 2026-10-10.
Installed Steam build 11140890 remains the source authority.
The selected cycle component is `level1:64920`.
Assembly-CSharp SHA-256 is `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
The manifest retains fifteen source hashes, ten controllers, seventeen light records, and eleven lighting textures.
The checked source metadata run matches all fifteen installed input hashes.

Final command checks use the isolated lighting checkout and Cargo target `/target/0aeee55818e76061`.
Python passes 26 tests. Native Rust passes 147 tests; all 15 installed-export cases pass in the explicit ignored run.
Formatting, release WASM, and matching wasm-bindgen pass after observation removal.
The final browser package timestamp is 2026-10-11T13:08:25UTC. Its WASM size is 53065640 bytes.
The final production source contains no temporary Task 7 observation system.
The final production browser reaches Ready, enables Start, shows L, and reports no current GPU error.
Its final seated night-low case renders wall beams and horizontal occlusion at 55 steps, with ignition on and engine off.
Root captured `lighting-task7-final-night-low.jpg` and accepted the recorded limits.


Source durations are day 1200, night 300, dawn 240, and dusk 180 seconds.
Dawn and dusk lie within the day interval. Production startup begins at 240 seconds.
The native lifecycle test freezes cycle time, fades, and switch movement during pause.
It preserves those states through recovery and restores startup state through restart.
The installed acceptance test checks source IDs, ranges, cones, shadow flags, material links, and exported file hashes.
Native source tests cover both physical headlight aliases, cabin modes, five house switches, brake input, and reverse input.

One `SourceLighting` owner retains imported scene nodes, textures, meshes, prepared material choices, and GPU buffers.
Restart removes fixed light proxies and owned pole hierarchies before rebinding.
Road pole identities use absolute source segment indices. Each active interval owns each pole once.
Eleven 128-metre intervals permit at most fifteen poles with source 100-metre spacing.
Road geometry and road lamps share the existing lateral admission rule: bands minus 1 through 1.
Movement and rebasing update existing poses. Stream changes replace instances with shared handles.
The steady lighting and sky paths update persistent uniform buffers and prepared material selections.
The source audit found no resource creation in those covered steady paths. It does not measure complete-game heap allocation.

### Browser evidence

Root records the browser checks in ignored Task 3 through Task 7 reports and captures.
The baseline Task 7 restart retains 38 of 38 old light and pole entities. This proves the renderer reset defect.
The production-offset scenario preserves startup 240 and advances the existing elapsed cycle after Ready.
The GREEN reset removes all 38 original entity IDs and restores cycle 240 with elapsed 0.
Eighteen rebound snapshots retain 16 unique fixed IDs and 11 unique poles, with no original IDs.
Imported node links and eleven source texture handles retain their exact identities.
The existing restart input owns this transition.
This elapsed advance tests renderer lifecycle. Native cycle tests provide timing rule evidence.
Synthetic phase captures change the served startup offset. They provide image evidence, separate from production reset proof.

Root captured walking and cockpit images at dawn, noon, dusk, and night from the same browser package.
The walking poses match. The night cockpit differs by 2.85 mm after one extra physics step.
Local driver pose, camera settings, source materials, and low-beam selection match across cockpit views.
The four images show the parked car, closed garage, and source sky changes.
Cockpit wall patches show low beams and interruption by horizontal geometry.
The dark night image does not prove distinct parked-car or closed-door moon shadow edges.
The noon procedural apron still clips.

Earlier accepted views show wall beams, tail emission, white reverse lamps, house 1 through 4 output, cabin modes, and roadside patches.
House 5 accepts aimed input, but its upward view lacks clear lamp pixels. Installed tests prove its output state.
The two physical headlight records share one usable source action; the browser uses its canonical alias.
Source ranges 50 and 250 metres remain numerical source checks. Close wall images do not measure those distances.
The matched night road run crosses seams and rebases once. It retains 14 poles, 30 light registrations, and 68 shadow views.
The run beyond 100 million metres retains those counts and shows distant warm patches.
The synthetic dawn threshold view shows poles without warm lamp patches.

The actual upload payload has one active radiance slot in daytime and twelve at night with eleven road lamps.
The source enabled-proxy count matches those values. The parked fixture has 27 registrations.
Reset clears registrations and radiance; rebound restores one active daytime slot out of 27 registrations.
The loading marker count stays zero after Ready and after restart.
These counts read source payload RGB and SourceLight.enabled. They exclude Bevy's zero-intensity native light fields.
The temporary observer performs no driver GPU-buffer readback.

Resource observations count asset storage and current instance registrations.
Image mip payload bytes count retained CPU image data, including decoded mip chains.
They exclude GPU alignment, render targets, framebuffer storage, and driver overhead.
The custom 4000-square Depth32Float atlas holds 64000000 bytes; hard and soft bindings share that allocation.
The source uniform holds 14800 bytes. GPU limits report 16384-pixel textures and 65536-byte uniform bindings.
The garage observations retain 126 source materials, 126 standard materials, 97 images, 244 meshes, 11 targets, and 27 prepared choices.
Their image mip payload totals 309788668 bytes through recovery and restart.

| Settled phase at 1280 by 720 | Mean ms | Median ms | p95 ms |
| --- | --- | --- | --- |
| Night before recovery |31.5683|27.9|41.7|
| Night after recovery |19.6475|20.8|27.8|
| Day after restart |18.1421|20.8|27.7|

Frame measurements use 240 consecutive requestAnimationFrame intervals in each settled phase.
They measure browser frame pacing, including scheduling and application work. They do not isolate GPU time.
The night movement run completes 190 metres with one rebase and 14 poles, 30 light registrations, and 68 shadow views.
Its 240-frame rebase window spans originZ 0 to -2048, at steps 719 through 895.
That window has mean 12.2396 ms, median 13.9 ms, p95 14 ms, and maximum 21 ms.
Twelve movement batch means range 10.6196 through 16.8983 ms.
Batch 9 retains a single 1118.1 ms interval of unproved cause. These measurements do not isolate GPU work.


### Startup gates and expected warnings

Required cookie failures and shader failures keep Start disabled and name the failed resource.
Healthy production loads reach Ready.
The final cookie gate names its missing SMIP. The final shader gate reports a Composer error.
Both keep Start disabled. Root captured both failures, then accepted the healthy phase views.
I stopped both dedicated fault servers after their checks. Production files retain their healthy resources.
Temporary fault routes and observations stay outside production files.
The exporter filters only the exact pinned pygltflib provisional-validator notice.
It still runs the same schema validation and shows other warnings and validation errors.

Bevy warns that its native WebGL shadow path supports one directional cascade.
The custom source atlas consumes the four checked cascade transforms. Keep those four source bounds.
The imported geometry also produces an unused TEXCOORD_2 warning.
Covered source lighting and cloud shaders use UV0; the exporter retains extra source UV channels.
These two named warnings remain expected. GPU validation errors, panics, and other warnings require inspection.

### Source rendering limits

No matched running original-game screenshot set exists. Source pixel parity remains unverified.
The source uses a camera stack; this renderer uses one combined camera with source near and far bounds.
Its 398-square shadow cells differ from source resolution. Native bias units and cascade edge blending remain unverified.
The sky sphere uses 64 by 32 segments. Native sky geometry and star RNG remain unverified.
Flare UV inset, exact physics queries, flare fog, and final color quantization remain unverified.
The renderer uses zero extra flare inset and no extra flare fog term.
Source mip bytes retain their original rows; native GPU cookie upload orientation remains unverified.
The synthetic noon view clips the bright procedural apron. Full procedural surface parity remains outside this task.

Dashboard node 861 selects source on material 325, with checked emission, a loaded texture, and visible entity flags.
Nine checked seated-eye rays cross opaque Doboz before the back-facing icon. Its source texture crop contains the correct symbol.
Part 74816 on root 34046 retains localRotationAfterMount [0, 180, 0] and a null saved slot.
The checked Craft rule applies that angle, but the selected slot and executed mount path remain unresolved.
No guessed panel rotation, opacity, culling, winding, or color change repairs that gap.
The dashboard icon has no proved visible pixels in the current cockpit view.

Close beam images do not establish far cutoff or all terrain, wall, and closed-door occlusion cases.
Four-phase parked-car and closed-door shadow acceptance requires observable edges in each captured phase.
Keep any dark or hidden edges as an acceptance limit.
The embedded browser rejects ordinary pointer lock. Deterministic ignored input scenarios do not prove real mouse capture.
Engine heat, fluids, portable flashlights, survival, sleep, weather events, audio, saves, multiplayer, and deployment retain the approved scope.
