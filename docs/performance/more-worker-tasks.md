# Additional worker tasks baseline

Status: BLOCKED by baseline startup. No worker candidate is selected.

This baseline uses Steam build `11140890` as the behavior authority.
The existing save schema remains the format authority.
No road worker or grass worker is implemented.

## Source and package identity

| Item | Identity |
| --- | --- |
| Behavior base | `738ce8b2b907491725c52b3a917c42f552ac9174` |
| Instrumentation source | `b059711ae2054061e533379bd430d49b9fbc96d7` |
| Instrumentation commits | `4c7a7ecdd2a7437a35c015fd6945a65325687f64`, `b059711ae2054061e533379bd430d49b9fbc96d7` |
| Profile | release, optimized |
| Main WASM SHA256 | `b161b4c21795f8780b194e39ec1b15d6e22faf0ddaf269ab8c25fad7ee8ee774` |
| Terrain WASM SHA256 | `dbf7906503765cfc5e906c3b834d81c80e42dbe978360cd13613c99621141701` |
| Manifest SHA256 | `f1a86a6c91448e2c1d07dd90dae273cc136546d0e39f3e1b5cef963b2f7c9a8d` |
| Runtime content directory | `ac355a3c0a94ef989a8f12a12ba48e0ef8e86c0a63021ca2f2ae0ab9a07287c1` |
| Docker image | `sha256:afbd45e6aa83421104670a07274832f267be933d21d6d6b1163d5cf17295b98d` |
| Compiler | `rustc 1.99.0 (b940084d7 2026-09-28)` |
| WASM bindings | `wasm-bindgen 0.2.129` |

The release build passed in 10 minutes 45 seconds.
It reported four existing unused-variable warnings in `src/presentation/materials.rs`.
All 5,861 catalog files matched their declared length and SHA256.
Their total length is 3,146,447,872 bytes.
The asset library uses immutable hardlinks. This task did not change their contents.

The frozen roots are below `.cache/benchmark/more-worker-tasks/baseline/`.
Git archive froze tracked `web/` and `scripts/` files from the instrumentation commit.
The successful build supplied the separate `pkg/` directory.
The separate asset root is this worktree's `web/assets/` directory.
`workflow/server-configuration.json` records every served script and package hash.
`workflow/asset-identity.json` records every verified asset hash.

## Measurement boundaries

CPU stage totals can overlap. Do not sum nested stages as exclusive CPU time.

| Work | Current observation boundary | Limit |
| --- | --- | --- |
| Startup | Stored startup phases, fetches, environment history, browser trace | A completed menu is not gameplay acceptance |
| Asset validation | Browser validation span after the digest | Digest wait is outside this span |
| Asset decode | Rust PNG decode, image assembly, mip validation | Nested CPU span, not total asset work |
| Physics | Accepted frame's simulation loop and related update work | Includes more than the physics solver |
| Input | Browser event, accepted fixed step, submitted frame token | Scripted input overrides do not establish real input |
| World render preparation | `WorldPresentation::prepare` | Does not cover every Bevy renderer system |
| Road | `WorldStream::prepare_road` | Includes idle checks, admission, deformation, bounds, result ownership |
| Grass | Grass preparation and grass requests in world presentation | Includes scheduling, expansion, upload, and publication work |
| Save and load | Capture, prepare, commit CPU spans | Each successful operation needs separate runtime evidence |
| Terrain | Rust preparation/allocation, bridge copies, delivery, worker evaluation, consume | Worker evaluation also includes its input copy and decode |

Road deformation is in `src/world/road.rs::deform_road`.
Grass expansion is in `src/presentation/materials/grass/expansion.rs::GrassExpansion::prepare`.
Grass preparation processes at most 4,096 prospective or emitted vertices per update.
The current broad Road and Grass spans cannot alone isolate those pure calculations.
Narrow calculation spans require a valid runtime baseline before further instrumentation.

Length bytes and retained capacity measure different costs.
Road transfer length must count vertices, normals, tangents, UVs, colors, triangles, submeshes, and origin metadata.
Grass output length is `40 * vertex_count + 4 * index_count` bytes before transport metadata.
Neither formula measures transfer time or proves a worker saving.
Publication, mesh upload, and GPU work stay on their current owners.

## Runtime evidence

The focused cold startup attempt failed before New game became available.
The exact error was `Source lighting pose: Selected vehicle reflection probe count is invalid`.
The stored startup history reported no hidden or unfocused interval.
The initial viewport was 1280 by 720 pixels at DPR 1.
CPU profiling started through `?profileCpu=1` before initialization.
CDP cache bypass was enabled before navigation. This was not a warm-cache sample.
External tracing was requested. No usable trace file was retained.

The elapsed startup phases below describe one failed attempt.
They overlap and do not measure exclusive CPU time.

| Phase | Elapsed duration |
| --- | ---: |
| WebGL | 11.3 ms |
| Manifest | 979.9 ms |
| Interface | 644.1 ms |
| Generation | 4,461.1 ms |
| Decode and upload | 17,586.3 ms |
| Pipeline, incomplete | 17,614.4 ms |

Failure occurred 19,253.1 ms after the startup trace began.
The UI kept New game, Load, and Controls disabled.
The runtime reported `started=false`, zero physics steps, and zero travel.
The failure snapshot contained 994 fetch records.
Submitted gameplay frames and accepted input were not established.
No CPU audit, road count, grass count, transfer measurement, or valid sampler receipt was retained.
`workflow/startup-cold-observed.json` transcribes the actual tool output and marks these limits.
It is diagnostic evidence, not a performance receipt.

## Startup blocker

`src/presentation/lighting_scene.rs:16` fixes `PROBE_COUNT` at two.
`scene_value` rejects any different count at line 170.
`SharedLightingRecord` reads the supplied manifest's `render_lighting` record directly.
That record contains one selected-vehicle probe.

| Source item | Verified value |
| --- | --- |
| Vehicle component | `level1:77839` |
| Spawn car ID | `15` |
| Vehicle root | node `435`, `Car02Full`, source `level1:37191` |
| Probe component | `level1:59674` |
| Probe node | `694`, source `level1:33315` |
| Probe hierarchy | `Car02Full/CarStuff/ReflectionProbe` |
| Probe resolution | `64` |
| Expected / supplied probe count | `2 / 1` |

The validator source blob is `dcde458bcbac2c0f02e4aac3862219923faa4383` at both declared source commits.
The profiling changes did not introduce this count check.
All supplied asset hashes match. The served manifest hash matches the declared manifest hash.
This identifies an existing runtime assumption that conflicts with the supplied selected-vehicle record.
It does not establish which source-faithful renderer change should resolve the conflict.
This task changed no renderer behavior or manifest content.
`workflow/probe-diagnosis.json` contains the compact evidence.
`workflow/diagnose-probes.mjs` repeats the source and manifest check.

## Scenario and candidate status

| Required evidence | Status |
| --- | --- |
| Cold startup | Failed attempt only. No valid receipt |
| Warm startup | Not measured |
| Settled garage | Blocked before New game |
| Garage input | Blocked. No real or scripted gameplay input was used |
| Local driving | Blocked. No seated movement |
| Pending-work save and load | Blocked. No test save was written |
| Separate streaming profile | Not measured |
| Terrain initialization, copies, dispatch, return, consume, memory | Not measured |
| Road calculation calls, CPU, worst call, output length | Not measured |
| Grass calculation calls, CPU, worst call, output length | Not measured |
| Browser main-thread trace coverage | Unavailable after trace transport failure |
| Baseline repeat variation | Not measured |

Road and grass remain unranked.
The evidence does not justify either worker or a negative finding against either calculation.
No comparison with repeat variation or estimate of net saving is possible.
Do not start candidate implementation from these measurements.
A later candidate selection must remain provisional until actual transfers and synchronization are measured.

The browser tool returned no `Tracing.dataCollected` events.
A broader event read timed out and closed the transport.
One reset recovered the tab, but diagnostic export then stalled.
The parent stopped browser recovery. No trace, CPU audit export, or screenshot file was verified on disk.
The evidence directories contain no invented receipts or trace files.

## Reproduction and artifacts

All ignored artifacts use `.cache/benchmark/more-worker-tasks/` in the isolated worktree.
The asset root is `C:/Users/Corashina/.codex/worktrees/more-worker-tasks/the-long-drive-wasm/web/assets`.
The runtime package remains separate from the frozen web, runner, and WASM directories.
The server uses origin `http://127.0.0.1:4197` and writes only to `results/`.
The default test slot would be origin-scoped database `tld-browser-save`, store `saves`, key `manual`.
No save operation ran.

From this worktree, start the preserved package with:

```powershell
./.cache/benchmark/more-worker-tasks/workflow/run-server.ps1
```

Open `http://127.0.0.1:4197/?profileCpu=1` in a focused browser.
The unchanged package reproduces the reported probe-count error.
Repeat the asset and validator checks with:

```powershell
node .cache/benchmark/more-worker-tasks/workflow/verify-assets.mjs
node .cache/benchmark/more-worker-tasks/workflow/diagnose-probes.mjs
```

The release command was `./scripts/build.ps1`.
Its full log is `.superpowers/sdd/2026-10-19-more-worker-tasks/task-2-build.log`.
The full task report is `.superpowers/sdd/2026-10-19-more-worker-tasks/task-2-report.md`.

The host has an Intel i7-11700K and NVIDIA RTX 4070 Ti, driver `32.0.16.1742`.
External Docker checks overlapped compilation, then finished before browser startup.
The parent reserved the browser window and ran no other build or browser sample.
No competing in-app game tab or build container was observed at startup.
An unrelated benchmark server on port 4191 was preserved.
Its listener alone does not prove an active measurement.
Other user applications were not controlled, so complete host exclusivity is not established.

## Checks and next milestone

The approved release build and all asset hash checks passed.
Document whitespace, workflow syntax, package hashes, and validator identities were checked.
No product code changed in Task 2. No Rust or web test was needed for this findings-only commit.
Full checks were not run.
Gameplay acceptance and all blocked measurements remain open.

The user later approved a separate startup prerequisite.
That prerequisite must pass review before a new release package is frozen.
Resume this baseline task from that new declared instrumentation commit.
Do not combine the failed package with later sample receipts.
