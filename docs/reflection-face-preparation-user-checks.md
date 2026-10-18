# Reflection preparation: user checks

The implementation is complete. Runtime and performance acceptance belong to the user.
No tests or benchmarks were run after that instruction.

## Packages

Run this command from the project folder to create a new matched pair:

```powershell
./scripts/build-reflection-comparison.ps1
```

The command builds two normal locked release packages. It does not run tests or open a browser.
Both packages use one source commit, one toolchain image, and the same assets.
Both contain the same opt-in counters, disabled by default.
The baseline prepares six faces for every job. The candidate prepares one face for capture and none for mips.
Both prepare all six faces during warmup.
The baseline changes only the face selector in its private source copy.
Game capture order, resolution, filters, and publication remain the same.

The command prints both server commands. Run each command in a separate terminal.
The candidate uses port 4198. The baseline uses port 4199.
`comparison.json` records the source, toolchain, asset hash, package paths, and WASM hashes.
Keep that file with any samples. Do not compare these packages with the older build-75 packages.

## Functional checks

Use the same browser, viewport, graphics settings, and saved scene for both packages.
Export the save file, then import that same file on the other port.
Keep the shipped reflection quality settings for both packages.
Close the other package's game tab before each sample. Close other games before measuring.

1. Start New game. Wait for readiness. Confirm the garage and vehicle are visible.
2. Inspect the roof, windows, and mirrors from several directions.
3. Wait through several full reflection cycles. Look for missing faces, stale objects, and seams.
4. Walk outside the garage. Drive outside the yard. Check ground, streamed objects, and reflections.
5. Move or introduce visible content. Confirm later reflection cycles include the change.
6. Save and load repeatedly. Confirm readiness returns and reflections update after each load.
7. If capture fails, record the renderer error and whether the last valid reflection remains visible.

Earlier build-75 packages hit a shadow-atlas validation error during driving.
They also hit a `food.gltf` save error. Later source changes affect that save validation.
Record current failures separately. Earlier results do not establish the status of these packages.

## Measurements

Use a settled scene. The sampler rejects startup, streaming changes, hidden tabs, paused gameplay, and stalled rendering.
Each enabled probe must finish a capture cycle during a sample.
If the sampler rejects a run, retain the error. Do not report that run as a performance result.

Open the browser console after readiness:

```javascript
const {sampleReflection} = await import('/reflection-benchmark.js');
```

Take three samples per package with profiling disabled:

```javascript
const fpsRun = {variant: 'candidate', scene: 'garage',
  ...await sampleReflection({seconds: 10, preparation: false})};
```

Take a separate preparation sample:

```javascript
const preparationRun = {variant: 'candidate', scene: 'garage',
  ...await sampleReflection({seconds: 10, preparation: true})};
```

Use `baseline` for baseline samples. Use `driving` only for comparable active driving samples.
Save samples as one JSON array. Summarize them from the project folder:

```powershell
node scripts/summarize-reflection-benchmark.mjs path/to/samples.json
```

Record FPS, frame-time percentiles, preparation time, and prepared face counts.
Preparation time covers the nine named source stages. It is not total CPU or GPU frame time.
Expected counts after warmup are six for the baseline, one for candidate capture, and zero for candidate mips.
Warmup remains six in both packages. Keep measured results separate from these expected counts.
