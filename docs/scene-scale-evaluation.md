# Scene scale evaluation

Date: 2026-10-19. Branch: `codex/scene-scale-75`. Baseline: `738ce8b`.

## Scope

Compare the existing 100% and 75% settings in matched garage and driving scenes.
Keep 100% as the default. Integration into `master` was approved on 2026-10-19.
The baseline uses 4x MSAA. Other renderer changes are excluded.
Assets are copied into `.cache/benchmark/scene-scale/assets` before browser testing.
The snapshot contains the manifest, its content package, and this branch's shaders.

## Current result

The user stopped further checks, builds, and browser benchmarks on 2026-10-19.
No accepted garage or driving comparison was collected. A GPU bottleneck remains unverified.
The existing scale implementation is unchanged. This branch improves the measurement tools only.

| Check | Result |
| --- | --- |
| Targeted web tests | Six passed before the stop instruction. |
| Script syntax and diff whitespace | Passed before the stop instruction. |
| Release build | Stopped before completion. No release package was tested. |
| Matched garage and driving timing | Not run. |
| Image quality and HUD sharpness | Not verified in the game. |
| Input alignment and resize behavior | Not verified in the game. |
| Full checks | Not run. |

The standalone mouse capture probe failed with `WrongDocumentError` in the in-app browser.
Its message was `The root document of this element is not valid for pointer lock.`
The failure occurred with the browser hidden and visible, without game code.
Other builds and GPU activity also prevented an exclusive measurement window.
This chat did not contact or control other chats.

The first server test attempt hit sandbox error `EACCES` for localhost.
The rerun with localhost access passed all six tests.
The frozen manifest SHA-256 is `f1a86a6c91448e2c1d07dd90dae273cc136546d0e39f3e1b5cef963b2f7c9a8d`.

At 1280x720, 75% renders the scene at 960x540.
This removes 43.75% of scene pixels. It does not remove the same fraction of total GPU work.
Probe captures and shadow atlases keep their existing sizes.
The camera projection uses native dimensions. The final copy uses linear filtering.
The canvas and HTML HUD keep native display dimensions.
These are code findings. Browser checks must establish visible quality and input behavior.

## Measurement tool

The scene probe uses the existing content-index benchmark server and measurement code.
Receipts include package hashes, manifest hash, viewport, DPR, renderer state, physics steps, and submitted frames.
The probe records median, p95, and p99 frame times.
It rejects changed scale, resize, wrong target extent, paused frames, and hidden or unfocused samples.
Driving samples require seated movement. The probe does not force resume or inject input.
Real input remains a separate acceptance check.

## Repeat the comparison

Build the release package from the project root:

```powershell
.\scripts\build.ps1
```

Start the benchmark server from the project root:

```powershell
$env:PORT = '4185'
$env:BENCH_VARIANT = 'candidate'
$env:BENCH_ASSET_ROOT = Join-Path $PWD '.cache\benchmark\scene-scale\assets'
$env:BENCH_PACKAGE_ROOT = Join-Path $PWD 'web\pkg'
$env:BENCH_RESULTS_ROOT = Join-Path $PWD '.cache\benchmark\scene-scale\results'
node scripts/serve-content-index-benchmark.mjs
```

Open `http://127.0.0.1:4185/` in a browser with working mouse capture.
Set the viewport to 1280x720 at DPR 1. Close competing game tabs and wait for other builds.
Start New game. Verify walking, mouse look, and targeting through normal input.
Save one garage state and one driving state. Restore each state before its next sample.
Use the same camera, route, inputs, and warmup for each scale.
Compare 100%, 75%, then 100%. Keep screenshots and resize checks outside timing windows.

Run each sample through DevTools:

```javascript
const {measureSceneResolution} = await import('/__scene-measure.mjs');
await measureSceneResolution({scale:1,scenario:'garage',durationMs:30000,label:'garage-100-a'});
// Restore the same garage state and repeat the same warmup.
await measureSceneResolution({scale:.75,scenario:'garage',durationMs:30000,label:'garage-75'});
// Restore again before the final sample.
await measureSceneResolution({scale:1,scenario:'garage',durationMs:30000,label:'garage-100-b'});
```

Repeat with `scenario:'driving'` and distinct labels. Drive the same route after each restore.
Confirm matching receipt hashes, viewport, DPR, scene state, and workload before comparing results.
Check that the manifest hash remains unchanged after all runs.
Use fresh receipt labels for reruns. The server refuses to overwrite receipts.

## Interpretation

Stable improvement at 75% indicates sensitivity to GPU work that depends on scene pixels.
It does not establish that every stall comes from the GPU.
Little improvement can reflect CPU work, draw submission, or GPU passes that keep their original sizes.
Do not infer a CPU bottleneck from scale results alone.
