# Content lookup cache validation

## Scope

This note covers the content lookup cache on branch `codex/content-lookup-cache`, based on `48e88e51a093ff61412b82e35865d97cce7f16b5`.

The cache tracks three mutation revisions: active, static, and lamp. It uses keys `(session, active, static)` and `(session, lamp)`. It sorts by hash, then original index. Each lookup reads live values.

The core lookup test ran without export. The targeted `SourceContents` membership test uses an actual typed `WorldReference`. It does not weaken full manifest validation.

## Checks

- `cargo test --locked`: 65 passed, 37 ignored.
- Targeted membership check: 1 passed.
- `node --test tests/web/content-index-benchmark.test.mjs`: 3 passed.
- The cache-disabled snapshot test failed its rebuild assertion. It reported left `[2,0]` and right `[1,0]`.
- The two cache tests passed again after restoring caching.
- Both standard release builds passed.

## Release builds

The standard baseline release build passed in 13m54s. It used `opt=3`, thin LTO, one codegen unit, panic abort, and strip.
The candidate release build passed in 13m21s with the same release settings.
These build durations are not startup measurements.

The shared Docker image tag changed during concurrent work.
The baseline image ID was not captured.
The candidate used image `sha256:4cd6c858e8363e08de584bdcbad9824cef33413a4ef0ae37375ae2b536a57053`.
That image disappeared before the baseline rebuild. The retained packages are not an accepted matched pair.

Baseline WASM SHA-256:

```text
79f22d41b7a1f03ec1d3e3a1c9cceee580baea00debd99016642722f25fea4cc
```

Candidate WASM SHA-256:

```text
12896771f30a51da347fc19ae5b6c74c443192fe21bc05ed2d935bdc40b23edc
```

Frozen manifest SHA-256:

```text
a041ac29e18acf889c3c5aa50a909ad103aa5ceb4b98e50402f4e7a714cef288
```

The full manifest check rejects extra `exclusions` in `ShaderProperties`.
The error is at line 1, column 7001259. This check did not change.

Evidence is in `.cache/benchmark/content-index/`:

- `comparison.json`
- `manifest-validation.log`
- `cache-disabled-regression.log`
- `candidate-build.log`
- `baseline-pinned-rebuild-failure.log`

Frozen assets are in the `assets` directory. Packages are in `baseline-source/web/pkg` and `candidate-source/web/pkg`.

## Run the candidate benchmark

These commands serve the retained packages. The current assets reproduce the manifest error.

Open PowerShell in this worktree:

```powershell
Set-Location 'C:\Users\Corashina\.codex\worktrees\content-lookup-cache\the-long-drive-wasm'
$env:BENCH_VARIANT = 'candidate'
$env:BENCH_ASSET_ROOT = 'C:\Users\Corashina\.codex\worktrees\content-lookup-cache\the-long-drive-wasm\.cache\benchmark\content-index\assets'
$env:BENCH_PACKAGE_ROOT = 'C:\Users\Corashina\.codex\worktrees\content-lookup-cache\the-long-drive-wasm\.cache\benchmark\content-index\candidate-source\web\pkg'
$env:PORT = '4213'
node scripts/serve-content-index-benchmark.mjs
```

For the baseline, set `BENCH_VARIANT` to `baseline`.
Point `BENCH_PACKAGE_ROOT` to `baseline-source/web/pkg`.
For the final baseline run, set `BENCH_VARIANT` to `baseline-recheck`.
Use the baseline package.

## Browser measurements

First provide a frozen export that passes both packages' unchanged manifest validator.
Set `BENCH_ASSET_ROOT` to that export's asset directory.
Rebuild both packages inside one container from one immutable image ID before comparing frame times.
Open `http://127.0.0.1:4213/` and start the game.
Open DevTools. Schedule the sample, then close DevTools and focus the game.
Focus loss invalidates the sample. Use a new label for each sample.

```javascript
setTimeout(async () => {
  const {measureContentIndex} = await import('/__measure.mjs');
  console.log(await measureContentIndex({
    scenario: 'garage',
    label: 'garage-fps-01',
    durationMs: 10000,
    instrument: false
  }));
}, 5000);
```

Use actual seated movement for the driving scenario. Use `instrument: true` with separate labels for call counts. Run instrumented samples outside FPS samples.

The user stopped Computer Use with Escape before browser navigation. Browser control stopped at that point.
Gameplay, FPS, startup, Save/Load, streaming, and real input remain unverified.
Garage and driving average, p95, and p99 frame times remain unmeasured.
No A/B/A samples were collected. This branch has no measured speed gain.
The supplied export fails the game's unchanged manifest validator.
