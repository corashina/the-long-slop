# Optional scene resolution

Roadmap item 2. Source build `11140890`. Branch `codex/scene-resolution`.
Base revision `1103368`. Master includes source commit `b08a292` through merge `fad65b2`.
The user authorized the merge and push on 2026-10-19.

## Behavior

Select **Scene resolution** in Controls. Choose 100% or 75%.
Each page starts at 100%. The choice lasts until reload and does not enter saves.
At 1280×720, 75% renders the scene at 960×540, or 56.25% of the pixels.
The scene looks softer. This ratio does not predict an FPS gain.

The renderer scales the sky, three scene views, depth, attenuation, and scene effects.
The existing linear fullscreen copy enlarges the result to the native canvas.
HTML menus, text, HUD, and the crosshair keep their native size.
Mouse deltas, source projection, culling, world selection, and gameplay keep their existing paths.
Shadow atlases use native dimensions. MSAA remains 8 samples. Probe faces remain 64×64.

Snapshots retain native dimensions for projection and shadow fitting.
Render preparation uses scaled dimensions for scene targets and screen uniforms.
Sizing uses integer arithmetic, rounds down, and retains at least one pixel for positive dimensions.
Target creation remains limited to startup, resize, and scale changes.
The readiness check uses the scaled scene extent and the native window target.

## Checks

- Native library checks: 31 passed, 14 ignored. The baseline had 30 passed, 14 ignored.
- Web checks: 27 passed, including the selector, input, audio, loading, timing panel, and save paths.
- WASM compile check passed with five existing material warnings.
- Release WASM build and wasm-bindgen packaging passed with the same five warnings.
- Renderer and benchmark review passed after correcting gameplay validity checks.
- Isolated Controls layout checks passed at 1280×720 and 390×720.
  The browser control returned 0.75 and 1 through the real selector.
  These checks displayed Controls through DevTools because startup could not complete.

The frozen export used for these checks blocked runtime checks before renderer creation.
The baseline browser, candidate browser, and native validator report:

```text
unknown field `exclusions`, expected one of `floats`, `colors`, `textures`, `defaults`
at line 1 column 7001259
```

The tested manifest has SHA-256
`a041ac29e18acf889c3c5aa50a909ad103aa5ceb4b98e50402f4e7a714cef288`.
Two retained exports also fail the current contract because file records lack `bytes`.
The implementation does not change these exports or weaken validation.

The installed export changed during merge finalization. Its SHA-256 is
`20c89ef5c9e7ce2ae922f2dc6fa30d88eb3528a470a4e8b446655f9af6bc6ecb`.
The current native manifest validator accepts that export. Fresh browser checks remain open.
The earlier browser failures describe the frozen export above.

Garage and driving FPS, average frame time, p95, and p99 remain unmeasured.
Image quality, target resize, fullscreen, pointer capture, real input, physics, submitted frames,
probe publication, streaming, and Save/Load remain unverified at runtime.
No performance gain or source runtime parity is established.

## Merge checks

Master includes the remote README and roadmap cleanup, startup Brotli, and scene resolution.
Merge `fad65b2` passed 63 native tests, with 36 ignored, and 47 web tests.
The merged WASM compile check passed with the same five material warnings.
The merged Rust source tree matches the verified release source at `b08a292`.
The source tree hash is `e70c3b7b39267eb6654fad3d338c9112b4d5a153`.
The temporary worktree is archived. The source branch remains available.
Build packages, receipts, and merge logs remain in the primary checkout's ignored evidence directory.

## Run

Use the primary checkout in PowerShell:

```powershell
Set-Location 'C:\Users\Corashina\Documents\Projects\the-long-drive-wasm'
pwsh -File .\scripts\build.ps1
$env:PORT='4289'
$env:BENCH_VARIANT='candidate'
node .\scripts\serve-shadow-cache-benchmark.mjs
```

Open `http://127.0.0.1:4289/`. A matching export must pass validation before gameplay can start.
The primary checkout retains the installed source assets.
Preserved packages and receipts are under `.cache/performance/2026-10-19-scene-resolution/`.
The candidate package is under `candidate/pkg/`. The baseline package is under `benchmark/master-baseline/pkg/`.
Merge checks are recorded under `.cache/merge-scene-resolution/`.

After New game, check real input, active physics, and submitted frames.
Close competing game tabs and wait for builds to finish before measuring.
Use the same viewport, DPR, assets, starting state, and route for each sample.
Run this in DevTools on the focused game tab:

```javascript
const {measureSceneResolution} = await import('/__scene-measure.mjs');
await measureSceneResolution({scale:1,label:'garage-a1'});
await measureSceneResolution({scale:.75,label:'garage-b'});
await measureSceneResolution({scale:1,label:'garage-a2'});
```

Repeat with the same driving route. Record startup separately.
The helper requires a started game and checks advancing physics, submitted frames, visibility, and settled world work.
Inspect each receipt's `valid` field before comparing results.
Receipts include average, p95, p99, canvas dimensions, DPR, GPU, state, and renderer audits.
The server saves them under `.cache/benchmark/results/`.
Audits include native and scene dimensions, projection, atlas size, and probe publication.
Check resize, fullscreen, centered aiming, door dragging, item movement, and Save/Load at both scales.
Collect call counts in a separate run with `instrument:true`.
