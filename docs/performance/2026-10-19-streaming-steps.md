# Streaming stage measurements

## Scope

Roadmap item 6 requires driving measurements before a streaming stage is split.
The branch starts at `8eca150d90e7d0fa216823276c01825f8b1189d1`.
Steam build `11140890` remains the source authority.

The branch adds an opt-in profiler and deadline checks inside terrain construction.
The user approved implementation with browser testing deferred.
Native measurements selected terrain. No accepted driving profile selected this stage.
No browser performance gain is verified.

Driving passes a zero budget to `WorldPresentation::prepare`.
This value permits one complete presentation step. It does not impose a full-frame time limit.
Terrain now uses a 1 ms slice within that step.

## Terrain split

`TerrainSurfaceJob::advance` accepts a yield callback. Every caller uses the new signature.
All eight phases retain their cursors when the callback requests a yield.
A checkpoint follows 32 to 38 weighted work units.
An expired deadline still permits one small slice, so work cannot stall.
The existing 4096-work cap remains.

`WorldStream::prepare` includes descriptor time in the terrain deadline.
A positive loading budget keeps its supplied duration. Zero selects the 1 ms driving slice.
Terrain setup and final bounds calculation remain synchronous.
Other presentation stages also remain synchronous. This is not a strict whole-frame deadline.
A checkpoint can exceed its deadline by one slice.

Road deformation measured below 0.2 ms in the native probe and was left unchanged.
The near-terrain probe used the installed source export, seed 42, and 110 nearby road points.
It included one foundation and two holes.
Before the split, its longest advance took 2.281 ms in the native debug build.
After the split, a diagnostic run measured 2.096 ms without a deadline and 1.119 ms with 1 ms slices.
Those runs used five and six calls respectively.
These are native diagnostics, not release frame times or driving benchmarks.

Reservations still occur before job construction.
The split adds no allocation to job advancement. Capacity checks cover every output vector and both border vectors.
Stale requests discard their job and pending bytes.
The existing collision priority, completed-surface publication, and source road generation paths are unchanged.

## Terrain split checks

- Native library suite passed: 34 tests. Export-dependent tests remain ignored by default.
- Two explicit export-dependent terrain tests passed against the frozen source export.
  They cover admission at capacity, continuation at capacity, stale-request cancellation, and pending-byte release.
- Terrain fingerprints match values captured before the split for all three source tiers.
  Checks force yields after each checkpoint and after every third checkpoint.
  The near-terrain check resumes all eight phases, including both holes.
  It also exercises a live 1 ms deadline and verifies that advancement does not grow vector capacities.
- WASM compile check passed with five warnings in unchanged material code.
- Formatting and diff checks passed. No current check failed.
  The earlier export-dependent publication failure below was not retested.
- Browser testing is deferred at the user's request.
  Driving throughput, collision readiness, visual output, recovery, and Save/Load remain unverified for this change.
  Garage and driving average, p95, and p99 frame times still require matched release A/B/A runs.
- No new release package has been built for the terrain split. Full checks remain deferred.

The native check log is `.cache/streaming-step-budget/check.log` in this worktree.
Repeat the export-dependent checks with the frozen assets mounted at `web/assets`:

```text
cargo test --locked --lib
cargo test --locked --lib terrain_ -- --ignored --nocapture
cargo check --locked --target wasm32-unknown-unknown --lib
```

## Profiler

`streaming_profile(true)` clears and enables 27 fixed stage counters.
`streaming_profile_audit()` returns call counts, total milliseconds, maximum milliseconds, and calls above 2 ms.
`streaming_profile(false)` disables and clears the counters.
Disabled timers read no clock and allocate no heap memory.

`StreamDescriptors`, `StreamGeometry`, `Terrain`, and `Road` are nested measurements.
Do not add their times to their parent times.
The `Stream` stage also contains required-content selection.

The existing benchmark tool accepts `profileStreaming: true`.
It stores the stage measurements in the receipt.
It marks profiled samples with `performanceSample: false`.
WebGL call counts and streaming measurements require separate samples.

## Earlier profiler checks and limits

- Native library tests passed: 34. Another 16 tests were ignored.
- Benchmark-tool tests passed: 3.
- The existing export-dependent publication test failed at `src/world/stream_tests.rs:270`.
  It requested 33 pending requests against a limit of 32.
- The installed package reached garage readiness and New game readiness.
  The in-app browser then refused mouse capture.
- A diagnostic Save/Load route setup crashed in the installed package.
  The error was `OwnedData has no binding resource` at `vendor/bevy_render/src/render_resource/bind_group.rs:679`.
  The installed package's source commit was not established. This is not a matched baseline result.
- The standard release WASM build passed. It completed in 14 minutes, 40 seconds.
  Its SHA-256 is `8bd1994505386ebbfbd53ba10cb50b84470c27f9e546b85308fd73ae7f3d1d83`.
- This exact release reached garage readiness and New game readiness.
  Mouse capture failed again. Chrome automation was unavailable.
- Synthetic input advanced physics and operated car controls. It did not produce a seated driving route.
- The profiler receipt check recorded 27 submitted frames and 53 physics steps.
  The sampler rejected the sample because the world was not settled. It recorded no real input.
  `profileStreaming` was true and `performanceSample` was false.
- Normal Save completed on this exact release. Normal Load then produced the same `OwnedData` panic.
  The Load failure is saved as `profile-baseline/results/baseline-load-failure.json` in the evidence directory.
- Accepted garage and driving average, p95, and p99 frame times remain unmeasured.
  No matched A/B/A comparison ran. Driving, streaming, recovery, and active Load acceptance remain open.
- Full checks were deferred. The user approved release builds and profiling, but did not request `scripts/check.ps1 -Full`.

The profiler path was exercised at 1280 by 720 pixels, DPR 1, on RTX 4070 Ti through ANGLE/D3D11.
The frozen shaders match this worktree. Later shader changes in the main checkout were excluded.
No costly driving stage can be selected from the invalid browser samples.
The later terrain split uses the native evidence above. Browser acceptance remains pending.

The frozen manifest SHA-256 is `20c89ef5c9e7ce2ae922f2dc6fa30d88eb3528a470a4e8b446655f9af6bc6ecb`.
Local evidence is in `.cache/performance/2026-10-19-streaming-step-budget/` in the main checkout.

## Collect a baseline driving profile

The preserved profiling release contains the profiler without the terrain split.
It remains the baseline package. Rebuild a candidate release before comparing this change.
Start the benchmark server, or use the existing server on port 4226:

```powershell
Set-Location 'C:/Users/Corashina/.codex/worktrees/streaming-step-budget/the-long-drive-wasm'
$env:BENCH_VARIANT = 'baseline'
$env:BENCH_ASSET_ROOT = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/.cache/performance/2026-10-19-streaming-step-budget/assets'
$env:BENCH_PACKAGE_ROOT = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/.cache/performance/2026-10-19-streaming-step-budget/profile-baseline/pkg'
$env:BENCH_RESULTS_ROOT = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/.cache/performance/2026-10-19-streaming-step-budget/results'
$env:PORT = '4226'
node scripts/serve-content-index-benchmark.mjs
```

Open `http://127.0.0.1:4226/` in a browser that permits mouse capture.
Start New game. Enter a car and prepare the route.
Schedule this sample in DevTools. Then focus the game and drive:

```javascript
const {measureContentIndex} = await import('/__measure.mjs');
setTimeout(async () => console.log(await measureContentIndex({
  scenario: 'driving', profileStreaming: true,
  label: 'driving-profile-01', durationMs: 10000
})), 5000);
```

Summarize the saved stage measurements:

```powershell
node scripts/summarize-streaming-profile.mjs 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/.cache/performance/2026-10-19-streaming-step-budget/results/baseline-driving-profile-01.json'
```

Run frame-time samples with `profileStreaming: false` and `instrument: false`.
Use matched release A/B/A runs to verify the terrain split.
Preserve the route, assets, viewport, resolution scale, and device pixel ratio.
Keep other builds and GPU work outside the measurement window.
