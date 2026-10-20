# Additional worker tasks

Date: 2026-10-20.
Road and grass workers are merged into `master` from `codex/more-worker-tasks`.
The current source is in `C:/Users/Corashina/Documents/Projects/the-long-drive-wasm`.
Merge commit: `50d51c6fb2e40614e5b63a06e11662ae2221ebdf`.
The branch commit identities below record the historical implementation source.
The latest user instruction is "Implement workers. Skip tests".
The merge preserves the current master renderer path and G05 save schema 7.
The worker implementation report's schema 6 describes its pre-merge tree only.
It does not describe current master save behavior.
The historical worker worktree path is no longer current. The source now lives in the primary checkout.
The historical replay asset tree lives at `.cache/benchmark/more-worker-tasks/assets`.

Source review and compile checks do not prove runtime behavior.
No worker performance gain, memory gain, bitwise equality, input latency, or save/load acceptance is established.
No new release worker package or frozen candidate has been produced.
The startup receipts below describe an older package with no road or grass worker.

## Current integration checks

These checks ran on the merged working tree. It included the pre-existing local MSAA change in `src/presentation/render_targets.rs`.
The committed merge tree retains the prior four-sample value there, so these checks do not prove that exact committed file state.

`cargo fmt --all -- --check` passed.
`cargo check --locked --workspace --lib` passed for the native target.
`cargo check --locked --target wasm32-unknown-unknown --workspace --lib` passed for the WebGL target.
`cargo check --locked --target wasm32-unknown-unknown --lib -p my-summer-slop --no-default-features --features webgpu` passed.
The Rust checks used the existing `my-summer-slop-toolchain` Docker image and Cargo volumes.
Node syntax checks passed for the changed `.mjs` and worker bridge files.
PowerShell parsed `scripts/build.ps1` without errors.
`git diff --check` and `git diff --cached --check` passed.

No tests ran, as requested. No release build, full check script, or browser run ran.
Compilation and syntax checks do not establish runtime behavior, worker parity, or performance.

## Implementation identities

| Change | Commit |
| --- | --- |
| Shared road calculation | `5774e0b2bb3ddb9a5e8245e951c2c6913dd3e223` |
| Road worker integration | `12e71ebd755693cf63e9ef9821fd10b64ad0c779` |
| Shared grass calculation | `37b468ca5ea2e5644bbbf10e35209ea0e302a2e0` |
| Grass worker integration | `ba57f58fb1e2e42407a8aa6d3381948a43dc8ddd` |
| Save/load ownership and capacity policy | `333f74952a131e4666ac5ace51d0d65dc34d461d` |
| Final implementation tree | `ba7289be6f9dc00d3e13f5410ad0966c7f8b38b7` |
| Behavior authority | Steam build `11140890` |

The implementation reports are `.superpowers/sdd/2026-10-19-more-worker-tasks/task-3-report.md` through `task-7-report.md`.
The final source review is `task-7-review.md` in that directory.
Task 8 makes documentation changes only.

## Shared calculation and ownership

`crates/road-mesh` owns road deformation, frame math, and checked prototype and pose codecs.
Main road generation and `crates/road-worker` use that calculation.
Extraction preserved source arithmetic types, expression order, influence order, skin order, triangle groups, and reflected coordinates.
Dependency review found matching resolved math features for the main and road worker WASM targets.
These source checks do not prove equal output bits.

Each `WorldStream` owns one road actor through `src/world/road_worker.rs` and `web/road.js`.
Active and prepared worlds own separate actors and unread results.
Each actor holds three immutable source slots and one computing or unread result.
Source revisions identify each slot. Releasing source ownership terminates that world's actor before returning its charges.
Actor restart preserves the Rust job sequence.

Road identity contains session, request revision, source owner, and job sequence in 28 fixed-width bytes.
The 156-byte job input contains the source slot, source revision, and pose.
JavaScript carries `u64` identities as checked decimal strings. It compares them without Number conversion.
Initialization must complete before packing or installation.
The browser has no inline road calculation fallback. The native adapter uses the shared calculation.

Main retains source order, RNG progression, collision activation, `supports()`, and `required_ready()`.
Completion polling runs before descriptor and geometry gates.
Terrain retains local collision priority and the shared publication slot.
A road result can wait unread while required terrain prepares.
Publication checks the session, request revision, road owner, source revision, required status, and existing geometry.
Retargeting changes the main claim only after source checks. It preserves the submitted worker identity.

`crates/grass-mesh` owns grass equations, domains, planning, approval, emission, and checked codecs.
`crates/grass-worker` uses those types and calculations.
The old presentation domain and expansion modules were removed.
Source comparison found unchanged calculation tokens and blade order. Resolved glam features match main Bevy.
This comparison did not execute the calculation or prove bitwise equality.

Each `GrassJobs` owns one grass actor through `src/presentation/world/grass_worker.rs` and `web/grass.js`.
Each entity installs immutable absolute source-space patches and settings once.
Four domains are shared within that actor. A render-origin change does not reinstall the dataset.
Reuse rejects a changed patch Arc or changed settings.
Active and prepared presentation worlds own separate actors.

Grass identity contains session, owned view revision, and all 64 entity bits in 24 bytes.
JavaScript uses checked decimal strings and BigInt for these fields.
The view payload is 80 bytes and contains four clip planes.
The worker returns a plan without geometry. Main admits output and GPU storage before approving emission.
A generation slot remains held through planning, approval, emission, and unread output.

Camera changes update the latest view. Submitted work keeps its captured view through upload and publication.
Accepted geometry remains active until its replacement passes upload, pipeline, and publication gates.
Parent ordering remains. Selection uses `last_started` and index for fair actor access.
Writer readiness gates submission. Polling and retirement continue while writers are blocked.
The browser has no inline grass calculation fallback.

## Transfers, synchronization, and release

The bridges copy encoded main input, then transfer that copy's ArrayBuffer to the worker.
WASM bindings copy input into worker linear memory.
Output getters move Rust vectors into JavaScript typed arrays.
The worker frees its result wrapper, then transfers the arrays to main.
The bridge validates output and copies it into main WASM storage.
A transferable buffer therefore does not remove the WASM boundary copies or main validation work.

Road validation checks identity, origin, typed-array sizes, finite values, indices, and contiguous submesh ranges.
Grass validation checks identity, plan formulas, typed-array sizes, finite values, indices, and source patch indices.
A result cannot acquire ownership. Only its submitted identity can complete the current job.
Valid stale or duplicate replies cannot replace current ownership.
Malformed identities, malformed current output, initialization failure, and transport errors fail explicitly.
Reads consume one result. A second read fails.

A canceled busy road job keeps its slot and reservation until completion or actor termination.
Grass output denial sends discard and keeps accepted geometry.
Its plan and slot remain charged until the discard acknowledgement.
Dataset retirement waits for its own acknowledgement before returning dataset charges.
Retiring an unrelated grass dataset does not cancel the active entity.
The last grass instance terminates its actor before returning domains and retained capacity charges.

Owner disposal calls `worker.terminate()` before clearing transport references and returning worker charges.
Road and grass Drop paths terminate only their own actors before field destruction.
Renderer continuations and uploads retain their existing physical drain gates.
Source ordering establishes these calls. Termination latency and physical memory reclamation remain unobserved.

## Logical storage and WASM capacity

Logical reservations and linear-memory capacity have separate accounting.
Neither is process memory. JavaScript object overhead and driver memory are outside these figures.

Road operation storage reserves the following before encoding or dispatch:

```text
3*S + 16*G + 48 + F + sizeof(RoadMesh) + 16 + 3*156 + 3*I
```

`S` is main surface storage. `G` is submesh count. `F` is frame scratch storage.
`I` is encoded installation size, or zero for an installed source.
The formula includes main output, worker output, transferred output, packed ranges, headers, scratch, and input copies.
Installed road sources have a separate active CPU charge.
The converted main source cache remains in prototype CPU charges.
Installation transfers its retained cache charge without changing the total charge.
Unread and canceled road work retain job, pending, and CPU charges.
Consumption moves the main surface charge into `PreparedSurface`.

Grass installation reserves decoded patches, a 256-byte ownership allowance, three encoded copies, and Vec-to-Arc overlap.
It also holds a generation slot until installation acknowledgement or termination.
Shared domain charges include four structures, 304 points, 510 triangles, and an Arc allowance.
Before planning, main reserves `GrassExpansion::planning_bytes` and 1024 bytes for view and control copies.
Before emission, main reserves twice `output_bytes` plus `gpu_bytes`.
Output size is 40 bytes per vertex plus four bytes per index.
The GPU charge preserves the conservative full-geometry reservation.
After consumption, one main output remains charged through upload and accepted geometry ownership.
Existing 4096 CPU and eight GPU upload allowances remain.

Both actors reserve an explicit 2 MiB initial linear memory before creation.
Before allocating sends, each owner admits this capacity estimate:

```text
estimate = next_power_of_two(2 MiB + 2 * logical worker operation envelope)
retained capacity charge = max(previous charge, estimate)
```

The envelope includes retained worker cache, installation overlap, and current worker operation storage.
Grass upload storage is excluded from this worker estimate. Its main CPU, staging, and GPU charges remain.
The capacity charge stays until actor termination.
Logical storage and estimated capacity deliberately overlap. This can deny work conservatively.

`road_worker_wasm_bytes` and `grass_worker_wasm_bytes` report acknowledged `memory.buffer.byteLength`.
`*_worker_reserved_bytes` report retained capacity estimates. `*_worker_cache_bytes` report logical installed datasets and domains.
Active CPU and active/prepared overlap include the reservations.
Actual capacity can remain high after live buffers are released.

The estimate is not a hard allocator cap.
An operation can exceed it before the post-operation reply guard observes growth.
An oversized reply causes failure and actor termination.
No hard allocator bound, bounded transient overshoot, or allocator equivalence is proven.
The 2 MiB linker settings have source and parse evidence only. No worker release package was linked.

## Save/load source boundaries

In the historical worker tree, save schema was version 6.
Current master uses G05 schema 7. Worker transport data stays outside saved snapshots.
Capture checks readiness and pause state, clears input, captures authoritative runtime fields, validates, serializes, and returns.
It adds no wait for road or grass visual jobs.
Existing restore-pending and snapshot validation checks remain.
Explicit snapshots retain source progression and RNG state.
They exclude transports, worker identities, buffers, capacity metrics, and timings.

Prepared presentation creates and exchanges its own complete `GrassJobs` owner.
Candidate cancellation terminates only candidate grass ownership and stops its pipeline continuations.
Charges stay live until renderer drain permits release.
Canceled runtime destruction terminates its own road actor.
Commit releases the exchanged old presentation before replacing the runtime.
Old runtime destruction terminates its road actor. Only then does the active owner remove overlap charges.
Candidate actors move with their owners. A repeated Commit cannot take the same candidate again.

These are source-reviewed boundaries, not save/load runtime acceptance.
Equal snapshots under worker delay, RNG runtime parity, message order, and renderer drain remain unverified.

## Recorded checks and deferred work

Tasks 3 through 7 recorded native library compilation, workspace WASM library compilation, and formatting checks.
Task 7 checked the final implementation tree with:

```text
cargo fmt --all -- --check
cargo check --locked --workspace --lib
cargo check --locked --target wasm32-unknown-unknown --workspace --lib
```

Both library checks passed in the project Docker toolchain. They did not compile or run test targets.
Native compilation still warns about unused `startup_job_audit` in `src/world/stream.rs`.
WASM compilation still warns about `mut result`, `layout`, `device`, and `force` in `src/presentation/materials.rs`.
These warnings predate the worker changes.
Task 6 had an intermediate E0061 compile failure. The corrected final native and WASM checks passed.

Reports also record `node --check` for both bridges, both worker scripts, main, and changed profiling/compression scripts.
PowerShell parsed `scripts/build.ps1` without running it.
Task 7's final syntax checks cover road changes. Task 6's syntax checks cover the unchanged grass scripts.
Task 8 reads that evidence and runs only `git diff --check` for its documentation edit.

Worker source includes opt-in timing for initialization, packing, copies, planning, emission, extraction, validation, waits, release, and publication.
Disabled diagnostics avoid diagnostic clock reads.
No new worker timing records have been collected in a browser.
Packing, copies, polling, validation, approval round trips, and publication remain costs to measure.
Fewer main-thread calculations alone do not establish a performance gain.

Deferred checks include tests, linked worker bindings, worker initialization, reply order, cancellation, retirement, denial, and physical reclamation.
Runtime parity, bitwise mesh equality, active garage physics, driving, collision readiness, and real input remain unverified.
Pending-work save/load, canceled preparation, repeated Commit, and continuous camera motion also remain unverified.
Startup cost, input latency, retained memory, job latency, and frame times remain unmeasured for these workers.
No comparison tool, fixture, delay injection, benchmark, browser run, build, or check script was added or run in Task 8.

## Self-run commands for current source

These commands are documented only. Task 8 did not execute them.
For a later user-authorized release and browser session, use the current build script without extra switches:

```powershell
Set-Location -LiteralPath 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm'
./scripts/build.ps1
```

The script emits main, terrain, road, and grass JavaScript/WASM bindings under `web/pkg/`, then prepares compression.
Road outputs are `road_worker.js` and `road_worker_bg.wasm`.
Grass outputs are `grass_worker.js` and `grass_worker_bg.wasm`.
Those outputs have no recorded current release hashes.

After that build, start the current static server:

```powershell
$env:PORT = '4173'
node ./scripts/serve.mjs
```

Open `http://127.0.0.1:4173/` in a visible, focused browser.
The server uses the primary checkout's `web/`, `web/assets/`, `web/pkg/`, and `.cache/http-compression/`.
It supports the `PORT` environment variable. It has no package-root or asset-root CLI option.
Use `http://127.0.0.1:4173/?profileCpu=1` only for instrumented CPU collection.
These commands do not establish runtime acceptance.

## Historical pre-worker startup baseline

Date: 2026-10-20.
This section records startup evidence collected before road and grass worker implementation.
The first instruction, "Skip testing", stopped further measurement at that time.
The later instruction, "Implement workers. Skip tests", authorized the source changes documented above.
The measurements below still describe the pre-worker package.

The measured startup repair reached Ready and enabled New game.
The measured timing repair permitted profiled startup and preserved terrain result delivery.
These startup results do not establish active gameplay, input response, driving, or save/load acceptance.

### Historical source and frozen package

| Item | Identity |
| --- | --- |
| Branch | `codex/more-worker-tasks` |
| Original behavior base | `738ce8b2b907491725c52b3a917c42f552ac9174` |
| Startup repair | `992b31969c9f13a984fe7d711be63d7238d93ac3` |
| Reviewed profiling source | `f6d5ded77ca38de139e920e2c5d12a3cde19d806` |
| Behavior authority | Steam build `11140890` |
| Main WASM SHA256 | `f32028116123a6f4d0dd1c97939b982576aaee18945998d52b9809324f7f7a5a` |
| Terrain WASM SHA256 | `dbf7906503765cfc5e906c3b834d81c80e42dbe978360cd13613c99621141701` |
| Manifest SHA256 | `f1a86a6c91448e2c1d07dd90dae273cc136546d0e39f3e1b5cef963b2f7c9a8d` |
| Runtime directory | `ac355a3c0a94ef989a8f12a12ba48e0ef8e86c0a63021ca2f2ae0ab9a07287c1` |
| Profile | Optimized release |
| Compiler | `rustc 1.99.0 (b940084d7 2026-09-28)` |
| Bindings | `wasm-bindgen 0.2.129` |
| Docker image | `sha256:cd7332c2295b45f746364ff5d4470846bff9ce6a4de936e56e5ac35aec50c096` |

At the freeze, Rust sources, crates, lockfile, and the build script were unchanged since the startup repair.
The pre-worker `./scripts/build.ps1` passed, including bindings and compression.
Cargo reused its release cache and finished in 5.30 seconds.
Compression reported 392 files, 186215917 source bytes, and 18462373 encoded bytes.
Four existing warnings remain in `src/presentation/materials.rs`.
All 5861 catalog files passed length and hash checks, totaling 3146447872 bytes.
All 39 served script and package hashes matched the saved server configuration.

Artifact paths below use `.cache/benchmark/more-worker-tasks/` as their prefix.
The saved pre-worker freeze is `baseline-timing-repair/`, with separate `web/`, `scripts/`, and `pkg/` roots.
The original asset root was the worker worktree's `web/assets/` directory.
That historical asset tree now lives at `.cache/benchmark/more-worker-tasks/assets` in the primary checkout.
The full asset library was not copied.
The old `baseline/` and `baseline-startup-repair/` freezes remain intact.
All three freezes are historical evidence. None contains the new road or grass worker package.

### Saved startup measurements

Both receipts use seed 42, the default garage route, 1280x720, and DPR1.
Chrome 155 uses ANGLE/D3D11 on NVIDIA RTX 4070 Ti.
CPU profiling is enabled. Streaming-only and WebGL instrumentation are disabled.
The CPU profile includes its nested streaming stage counters.
These receipts are instrumented startup evidence, not uninstrumented frame performance samples.

| Observation | Cold receipt | Second receipt, labelled warm |
| --- | ---: | ---: |
| Receipt validator | Pass | Pass |
| Startup complete, input ready | Yes | Yes |
| Hidden or unfocused during startup | No | No |
| Cache hits / misses / unknown | 0 / 1571 / 0 | 0 / 1572 / 0 |
| Last startup phase end | 30743.0 ms | 26487.9 ms |
| Generation phase | 3027.4 ms | 2017.8 ms |
| Decode/upload phase | 12313.0 ms | 8302.5 ms |
| Pipeline phase | 29272.8 ms | 25288.8 ms |
| Post-ready sampling interval | 10000.9 ms | 10007.2 ms |
| Submitted frames during that interval | 497 | 522 |
| Started / physics steps | false / 0 | false / 0 |

Startup phases overlap. Do not sum them.
The CPU records span navigation through the post-ready sampling interval.
They do not describe only the ten-second interval.

The second receipt does not establish a populated browser cache.
Its saved trace reports all 1716 responses with `fromCache=false` and `fromServiceWorker=false`.
All 1572 reconciled startup fetches report misses and 389556680 received bytes.
Runtime assets have immutable cache headers, and fetch uses the default cache mode.
The saved data does not establish why the browser reused no cached response.
The schema validator accepts the declared cache label without checking these counters.
Preserve the original receipt, but exclude it from claims about warm browser startup.
The faster second run does not establish a warm-cache gain or repeat variation.

### CPU stage boundaries

These are inclusive stage timer totals. They are not an exclusive CPU breakdown.
Nested stages and elapsed waits must not be added as independent CPU costs.

| Stage | Cold calls | Cold total / maximum | Second calls | Second total / maximum |
| --- | ---: | ---: | ---: | ---: |
| Rust startup | 1 | 2825.1 / 2825.1 ms | 1 | 1951.6 / 1951.6 ms |
| Asset decode | 871 | 917.1 / 48.9 ms | 871 | 825.6 / 40.8 ms |
| Render preparation | 4436 | 6407.4 / 16.0 ms | 4762 | 6473.8 / 9.7 ms |
| Broad Road stage | 20 | 0.7 / 0.3 ms | 68 | 0.6 / 0.1 ms |
| Broad Grass stage | 3768 | 17.1 / 0.3 ms | 4207 | 13.0 / 0.3 ms |

The Road stage includes admission, bounds, and ownership work.
The Grass stage includes scheduling, upload, publication, and idle checks.
Their call counts are stage calls, not pure calculation calls.
Neither record contains actual road or grass output lengths.
Those pre-worker records contain no narrow `deform_road` or `GrassExpansion::prepare` span.
These startup totals cannot select a gameplay worker or establish its net saving.

Each Rust stage retains 4096 samples for quantiles.
Render preparation overwrote 340 cold and 666 second-run samples.
The second Grass stage overwrote 111 samples.
Other high-frequency streaming stages also overwrote samples; the analysis JSON lists each count.
Count, total, mean, and maximum cover all recorded calls.
The p95 and p99 values cover only the latest retained samples.
Browser input and worker record rings report zero overwrites.
Zero Physics, SavePrepare, SaveCommit, and LoadPrepare calls mean those scenarios were not exercised.

### Existing terrain worker reference

Each run records 101 completed terrain jobs, with no discarded result.
These costs describe the existing terrain path during startup.
They do not prove that road or grass should use a worker.

| Observation | Cold | Second |
| --- | ---: | ---: |
| Worker initialization, once | 31.8 ms | 13.3 ms |
| Input preparation, total | 1.4 ms | 1.1 ms |
| Main input copy, total | 1.6 ms | 2.1 ms |
| Message submission, total | 7.3 ms | 4.5 ms |
| Result allocation, total | 0.7 ms | 0.2 ms |
| Main result copy, total | 0.5 ms | 0.2 ms |
| Worker evaluate with input copy, mean | 0.1505 ms | 0.1386 ms |
| Worker extraction, mean | 0.0564 ms | 0.0446 ms |
| Return delivery, mean | 10.8771 ms | 8.7010 ms |
| Result wait before consumption, mean | 2.2109 ms | 1.7901 ms |
| Total job latency, mean / p95 / maximum | 13.3970 / 22.0 / 28.9 ms | 10.7307 / 15.3 / 22.0 ms |
| Input bytes, total | 113192 | 113172 |
| Output bytes, total | 752576 | 752576 |
| Peak outstanding transfer bytes | 35948 | 35948 |
| Initial / peak worker WASM bytes | 1114112 / 1179648 | 1114112 / 1179648 |

The worker evaluation field combines calculation and the input copy into worker WASM.
Result wait and delivery are latency, not main-thread CPU work.
Publication CPU is not isolated from the broad Terrain and Stream stages.
Recorded zero-duration operations remain limited by browser clock resolution.
The browser does not report its timer coarsening.

The clock handshake passes in both runs.
Two cold submit-delivery values are negative and excluded from that metric.
Thirteen second-run submit-delivery values are excluded, including nine within arithmetic uncertainty below zero.
The retained submit-delivery means are 0.0749 ms across 99 values and 0.0409 ms across 88 values.
All 101 jobs still delivered results in each run.
No excluded value contributes a gain claim.
The exclusions are recorded individually in `workflow-timing-repair/startup-receipt-analysis.json`.

### Trace coverage and host limits

The cold trace stream was lost during export. No complete cold trace exists.
The second trace is `traces-timing-repair/startup-warm-cpu.json`, despite the unsupported cache label.
It contains 1091310 events and 263446072 bytes.
Its SHA256 is `8f644dafa6e2a1e67b2c8969337f87874aaa1fea697bb21a020a4bb5dbeffdeb`.
The parent observed `dataLossOccurred=false` when tracing completed.

The parsed trace covers 49.684 seconds.
It includes game navigation, resource loading, main-thread function and animation tasks, layout, paint, and the receipt POST.
The receipt POST occurs 49.563 seconds after navigation.
The trace has no CPU stack samples.
It does not isolate road, grass, physics, or Bevy preparation functions.
Trace event durations are inclusive and must not be added across nested names.
The saved coverage analysis is `workflow-timing-repair/warm-trace-analysis.json`.
The frozen summarizer attached the trace path and hash to the second receipt.
Its generic `coverageVerified` field remains false; the separate analysis records the inspected coverage and its limits.
The 334-second download delay occurred after measurement and is excluded from the timings above.

The parent waited for an observed quiet window without stopping unrelated work.
At window start, no Docker container was active; GPU usage was 1% at 14.49 W.
Two CPU readings were 17.8% and 22.4%.
At cold-window end, CPU readings were 16.6% and 12.0%.
The passive Docker event log alone cannot prove host exclusivity.
Other user applications were not controlled. Full host exclusivity remains unproven.
Gate and window snapshots are saved in `workflow-timing-repair/`.

### Historical stop and evidence gaps

At the measurement stop, neither new worker was implemented. Tasks 3 through 7 later implemented both workers.
The missing gameplay evidence is not a negative result for either candidate.
No net saving, repeat variation, or worker performance gain is established.

Deferred scenarios and checks are:

- A startup run with observed populated browser cache and matched repeat runs.
- A complete cold startup trace with verified coverage.
- Settled garage with active physics and submitted frames.
- Accepted pointer and keyboard input, with input-to-step and step-to-frame latency.
- Local driving with collision, terrain work, and road work.
- Save/load while derived work is pending.
- A separate streaming-only run.
- Pure road and grass calculation costs and actual output lengths, if valid gameplay requires narrower spans.
- Actual worker transfer costs and matched release A/B/A measurements.

New game succeeded, but Continue failed pointer capture through the available IAB routes.
The latest error says the root document is not valid for pointer lock.
The screenshot is `workflow-timing-repair/pointer-capture-failure.png`.
The user then requested that testing stop.
No additional runtime measurement followed. Later implementation used compile and syntax checks under the updated instruction.
Task 8 uses the saved artifacts and source reports. It runs no runtime measurement.
Full checks were not requested or run.

### Historical evidence and self-run commands

The two receipts are in `results-timing-repair/`.
Their names are `baseline-timing-repair-startup-cold-cpu.json` and `baseline-timing-repair-startup-warm-cpu.json`.
The workflow directory contains source, package, asset, toolchain, server, and analysis identities.
The complete report is `.superpowers/sdd/2026-10-19-more-worker-tasks/task-2-report.md`.
The release log is `.superpowers/sdd/2026-10-19-more-worker-tasks/task-2-timing-repair-build.log`.
Earlier startup failures remain documented there and in their separate frozen workflow directories.

For a later user-authorized replay of the historical pre-worker package, run:

```powershell
Set-Location 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm'
./.cache/benchmark/more-worker-tasks/workflow-timing-repair/run-server.ps1
```

The old worktree path is historical. The wrapper and frozen package now live in the primary checkout.
The wrapper sets these exact roots:

| Server variable | Root or identity |
| --- | --- |
| `BENCH_WEB_ROOT` | `.cache/benchmark/more-worker-tasks/baseline-timing-repair/web` |
| `BENCH_SCRIPTS_ROOT` | `.cache/benchmark/more-worker-tasks/baseline-timing-repair/scripts` |
| `BENCH_ASSET_ROOT` | `.cache/benchmark/more-worker-tasks/assets` |
| `BENCH_PACKAGE_ROOT` | `.cache/benchmark/more-worker-tasks/baseline-timing-repair/pkg` |
| `BENCH_RESULTS_ROOT` | `.cache/benchmark/more-worker-tasks/results-timing-repair` |
| `BENCH_COMMIT` | `f6d5ded77ca38de139e920e2c5d12a3cde19d806` |
| `BENCH_VARIANT` | `baseline` |
| `PORT` | `4199` |

It invokes the frozen `baseline-timing-repair/scripts/serve-content-index-benchmark.mjs`.
It serves the historical main and terrain packages, not the newly implemented workers.
Task 8 inspected the wrapper and server options but did not execute them.

Open `http://127.0.0.1:4199/?profileCpu=1` in a visible, focused browser.
The saved sampler commands are `workflow-timing-repair/parent-sampler-commands.md`.
Their warm label still requires actual cache evidence before use.
