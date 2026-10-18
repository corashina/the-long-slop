# Measure the GLES sampler patch

Use an exclusive window for builds and game tests.
Close other game tabs. Keep the browser, adapter, source assets, and compiler equal.
Use a 1280 by 720 backing canvas with device pixel ratio 1.
Keep baseline and candidate packages in separate directories.

Start the server from this worktree:

```powershell
node tools/benchmarks/samplers/serve.mjs <source-web-root> <package-directory> <shared-web-assets-root> <report-directory> 4186
```

The asset root contains `assets/reference` and the matching `assets/runtime` package.
Use the comparison revision's unchanged web files for both builds.
Keep only one running game tab during each sample.

After the garage reaches Ready, load the probe through the browser console:

```javascript
await import('/__probe.js');
await samplerBench.ready();
```

Capture a paused scene with `await samplerBench.capture('garage-fixture')`.
Verify preparation and loading before using a saved fixture for both packages.
Put fixtures in the report directory. Read them with `await samplerBench.fixture('garage-fixture')`.

The probe controls input for repeatable samples. It does not capture the mouse.
It hides the menu and performance overlays and resumes after focus-related pauses during controlled samples.
Check pointer input through the normal game interface in a separate correctness test.
Use `hold(bits, milliseconds)`, `tap(bits)`, and `look(x, y)` for controlled input through the public input bridge.
Record any use of that bridge and any failure of normal mouse capture.

Run three baseline and three candidate samples for each working scene.
Use a baseline, candidate, baseline sequence. Start from equal scene states.
Warmup lasts ten seconds. Measurement lasts ten seconds.

```javascript
await samplerBench.load(saved);
await samplerBench.sample('baseline-garage-1');
await samplerBench.load(drivingSave);
await samplerBench.sample('baseline-driving-1', {keys: 1});
```

FPS samples use passive animation-frame timestamps. They do not wrap graphics or frame callbacks.
Reports include frame intervals, render submission tokens, state, and render audits.
Reports also record menu visibility and mouse capture.
Samples keep the session running. Call `await samplerBench.pause()` after the sample set.
Use the tokens to confirm that the renderer submits frames throughout the sample.
Animation-frame intervals do not prove monitor presentation timing.

Count calls in separate samples:

```javascript
await samplerBench.sample('baseline-garage-counts', {instrumented: true});
```

The counting probe forwards each call and restores each wrapped method after measurement.
Do not use instrumented frame times for performance conclusions.

Saved-scene preparation can fail with `Missing source asset: reference/food.gltf`.
Record any failure. Do not treat captured saves as validated fixtures when preparation fails.
For fresh-session measurements, start through the public New Game action after each page reload.
Record the camera, position, active world, route, and warmup for both builds.
Measure the settled garage view before using input to drive the vehicle.
Check equal scene states before comparing results. Report any unmatched scene as a limitation.

Check reflections, all shadow cascades, streaming, repeated loads, and context loss.
Use `await samplerBench.observe('candidate-reflections', 20)` to retain reflection audits.
The baseline handles context loss by requiring a reload. Verify the same behavior for the candidate.
Reload after `await samplerBench.contextLoss()` and confirm Ready and submitted frames again.

Before normal input tests, call `samplerBench.cleanup()` or reload the tab.
Record package hashes, source revision, source asset hashes, browser version, and adapter.
Report FPS, frame intervals, and call counts separately. A lower call count does not prove a speed gain.

Create the summary after the FPS and count samples:

```powershell
node tools/benchmarks/samplers/summarize.mjs <report-directory>
```

The summary rejects invalid samples and reports missing evidence.
It checks canvas sizes, gameplay state, vehicle movement, and render submissions.
Retain rejected reports. Do not use callback rates after render failure as game FPS.
