# Additional worker tasks baseline

Date: 2026-10-20.
Status: startup measurements saved; worker selection and implementation remain unresolved.
The user requested "Skip testing". Further tests, builds, and browser measurements stopped.
The branch remains unmerged. No road or grass worker was added.

The reviewed startup repair now reaches Ready and enables New game.
The reviewed timing repair permits profiled startup and preserves terrain result delivery.
These startup results do not establish active gameplay, input response, driving, or save/load acceptance.

## Current source and frozen package

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

Rust sources, crates, lockfile, and the build script are unchanged since the startup repair.
The official `./scripts/build.ps1` passed, including bindings and compression.
Cargo reused its release cache and finished in 5.30 seconds.
Compression reported 392 files, 186215917 source bytes, and 18462373 encoded bytes.
Four existing warnings remain in `src/presentation/materials.rs`.
All 5861 catalog files passed length and hash checks, totaling 3146447872 bytes.
All 39 served script and package hashes matched the saved server configuration.

Artifact paths below use `.cache/benchmark/more-worker-tasks/` as their prefix.
The current freeze is `baseline-timing-repair/`, with separate `web/`, `scripts/`, and `pkg/` roots.
The asset root remains this isolated worktree's `web/assets/` directory.
The full asset library was not copied.
The old `baseline/` and `baseline-startup-repair/` freezes remain intact.
Their diagnostics are historical evidence, not samples for the current package.

## Saved startup measurements

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

## CPU stage boundaries

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
No narrow `deform_road` or `GrassExpansion::prepare` span was added.
These startup totals cannot select a gameplay worker or establish its net saving.

Each Rust stage retains 4096 samples for quantiles.
Render preparation overwrote 340 cold and 666 second-run samples.
The second Grass stage overwrote 111 samples.
Other high-frequency streaming stages also overwrote samples; the analysis JSON lists each count.
Count, total, mean, and maximum cover all recorded calls.
The p95 and p99 values cover only the latest retained samples.
Browser input and worker record rings report zero overwrites.
Zero Physics, SavePrepare, SaveCommit, and LoadPrepare calls mean those scenarios were not exercised.

## Existing terrain worker reference

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

## Trace coverage and host limits

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

## Remaining work and user stop

The original goal remains pending. No worker is selected or implemented.
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
- Candidate selection, actual transfer costs, and matched release A/B/A acceptance.

New game succeeded, but Continue failed pointer capture through the available IAB routes.
The latest error says the root document is not valid for pointer lock.
The screenshot is `workflow-timing-repair/pointer-capture-failure.png`.
The user then requested that testing stop.
No additional test, build, browser acceptance, or measurement ran after that request.
Only saved artifact analysis and documentation continued.
Full checks were not requested or run.

## Evidence and self-run commands

The two receipts are in `results-timing-repair/`.
Their names are `baseline-timing-repair-startup-cold-cpu.json` and `baseline-timing-repair-startup-warm-cpu.json`.
The workflow directory contains source, package, asset, toolchain, server, and analysis identities.
The complete report is `.superpowers/sdd/2026-10-19-more-worker-tasks/task-2-report.md`.
The release log is `.superpowers/sdd/2026-10-19-more-worker-tasks/task-2-timing-repair-build.log`.
Earlier startup failures remain documented there and in their separate frozen workflow directories.

For a later user-authorized session, start the frozen package with:

```powershell
Set-Location 'C:/Users/Corashina/.codex/worktrees/more-worker-tasks/the-long-drive-wasm'
./.cache/benchmark/more-worker-tasks/workflow-timing-repair/run-server.ps1
```

Open `http://127.0.0.1:4199/?profileCpu=1` in a visible, focused browser.
The saved sampler commands are `workflow-timing-repair/parent-sampler-commands.md`.
Their warm label still requires actual cache evidence before use.
