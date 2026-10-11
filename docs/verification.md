# Asset library verification

Target: Steam app `1017180`, build `11140890`, Unity `2021.3.18f1`.
Date: 2026-10-11, Europe/Warsaw. Runtime implementation base: `63dff0a`.
The [asset audit](research/2026-10-10-full-asset-audit.md) contains complete counts, hashes, indexes, and conversion limits.

## Final command checks

Use the reference Python environment and run commands from the repository root.
Runtime checks require Docker, `my-summer-slop-toolchain`, `tld-wasm-cargo`, and `tld-wasm-target`.

```powershell
python -m tools.reference.export --stage runtime --verify
python -m unittest discover -s tools/reference/tests -v
node --test tests/web/server.test.mjs
./scripts/check.ps1
./scripts/build.ps1
docker run --rm --pull=never --mount "type=bind,source=$PWD,target=/workspace,readonly" --mount type=volume,source=tld-wasm-cargo,target=/cargo --mount type=volume,source=tld-wasm-target,target=/target my-summer-slop-toolchain cargo test --locked --test reference -- --ignored
```

The runtime verification command calls complete `verify_library`, checks installed counts, then runs the native runtime contract validator.
Use `python -m tools.reference.export --stage library --verify` for library-only verification without Docker.
The final proof records unchanged accepted source, record, library, and runtime paths, sizes, and modification times.
It compares all selected runtime hashes with Task 9 and checks the unchanged accepted library manifest hash.
The complete verifier validates accepted file content against source, typed-record, and library receipts.

| Check | Result |
| --- | --- |
| Complete library and native runtime verification | Passed; 262,784 owners; 642.802 seconds |
| Accepted-file protection | 277,800 paths, lengths, and modification times unchanged |
| Exporter suite | 109 passed; no warning output |
| Node server suite | One passed |
| Project checks | Formatting, 94 native tests, and release WASM compilation passed; six source gates skipped by default |
| Browser package build | Passed |
| Installed reference gate | One passed |

Command exits were zero. Logs use `task-10-final-*.log` under `.cache/reference-library/work/`.
`task-10-final-checks.json` records commands and durations; `task-10-final-proof.json` records hashes, counts, and protection.
`task-10-validator-revision.json` records the exporter source hashes used by the final verifier.

## Isolated browser fixtures

Temporary source, build output, and pages live under ignored `.cache/reference-library/browser-check/`.
A separate server uses loopback port 4201 and an explicit asset allowlist.
The fixture checks library file hashes before serving the selected files.
It leaves `web/index.html`, the production entry, and accepted asset bytes unchanged.
The fixture uses Bevy 0.19.1 and the exact product Bevy feature list, including glTF, PNG, and WebGL2.
It does not enable animation playback.

The capture uses Playwright 1.62.1 and isolated headless Chrome 155.0.8059.40 with `chromiumSandbox: true`.
It uses no user profile or security-bypass flags. Each capture has a 55-second external deadline.
Captures retain all console levels, request outcomes, completed response-body lengths, and screenshots.

Bevy loaded the static `pohar` hierarchy (`sharedassets1.assets:21505`): three nodes, one mesh, and one scene.
It loaded skinned `RandomBat` (`sharedassets1.assets:32394`): 25 nodes, one mesh, one skin, and one scene.
Both passed `AssetServer::is_loaded_with_dependencies` and exposed their loaded glTF assets.
The loader capture completed all 21 HTTP 200 response bodies with no fatal loader or page error.
Bevy logged expected WebGL compute and storage capability limits.
A separate render capture displayed the static cup with the production camera's Reinhard tonemapping and Msaa::Off.
That capture completed all 21 HTTP 200 bodies without console errors or page errors.
I inspected the screenshot. This establishes representative static rendering, not Unity material parity.
Skinned loading passed; skinned rendering and animation playback remain unverified.

The media capture completed 13 response bodies with HTTP 200 and no console or page errors.
Chrome loaded embedded Liberation Sans and used it for the font sample.
The WAV `sharedassets1.assets:4164` decoded to 1,868 mono frames at 8 kHz and reached its muted playback end.
Its duration was 0.2335 seconds. Audible output and game audio integration remain unverified.
Six labeled 128 by 128 cubemap PNG faces decoded and appeared in the screenshot.
Those previews clip to 8-bit color. Native BC6H DDS/HDR rendering remains unverified.

The rotated sprite uses six known pixels through UnityPy's sprite rotation and flip operations.
Chrome's canvas pixels match the expected 2 by 3 image.
This fixture is synthetic because the installed index has zero sprites with nonzero packing rotation.
The installed source also contains zero morph channels.
Synthetic morph tests cover deltas and glTF channels; complete source checks validate all 42 clips and their normalized samples.
These checks do not establish animation playback or humanoid retargeting.

Evidence lives under `.cache/reference-library/work/`:
`task-10-media.json/png`, `task-10-loader.json/png`, and `task-10-render.json/png`.
The fixture source and allowlist remain in `browser-check/` for local reproduction.

## Garage startup limit

Task 9 captured 92 startup requests with HTTP 200, including the exact 86 selected asset URLs.
It found no library, catalog, unused-model, or audio preload.
The bounded readiness check remained at the loading screen; Start stayed disabled.
A corrected prior-payload comparison also remained loading, with all 94 response bodies completed.
It retained the current validated manifest, so obsolete schema differences did not affect that comparison.

Chrome used WebGL2 through ANGLE/D3D11 on NVIDIA GeForce RTX 4070 Ti.
A 40.256-second CPU profile attributed 38.832 seconds, about 96.5%, to synchronous WebGL program-link status checks.
The caller chain runs through wgpu's GLES pipeline creation and Bevy's pipeline cache.
No fatal loader or page error occurred. The evidence does not justify an exporter correction.
This task retains those measured results and does not repeat the long stalled capture.
Garage readiness remains unverified on this path.
The user's continuous garage-to-road browser route waiver remains in effect.

Evidence: `task-9-startup-network.json`, `task-9-prior-payloads.json`, `task-9-startup.cpuprofile`, and `task-9-profile-summary.json`.
These files live under `.cache/reference-library/work/`.

## Deferred verification fixes

The typed-schema cache fix from Task 8 remains covered by the complete suite.
The type-tree test now restores its prior boost state through test cleanup.
A long WAV fixture covers binary32 duration error above one frame and rejects error outside that bound.
The glTF helper filters only pygltflib's exact provisional-validator notice. Unrelated warnings and validation exceptions remain visible.
The normal-map helper uses Pillow's supported `get_flattened_data` API.
The runtime audio exclusion test now passes the scene gate before adding a WAV dependency.

The historical garage evidence below predates the complete library and does not establish current browser readiness.

# Earlier garage verification

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
