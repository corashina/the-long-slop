# Content lookup cache validation

## Scope

The content lookup cache was implemented on `codex/content-lookup-cache` at commit `a435a7f73ab23f9cab3900f9a89d8adf94835a4f` and merged into master on 2026-10-19.

The cache tracks active, static, and lamp mutation revisions. It keys active and static entries by session and revision. It keys lamp entries by session and revision. It sorts by hash and original index. Each lookup reads live values.

## Feature branch checks before merge

These checks ran on the feature branch. They do not verify the merged tree.

- `cargo test --locked`: 65 passed, 37 ignored.
- Targeted source-world membership check: 1 passed.
- `node --test tests/web/content-index-benchmark.test.mjs`: 3 passed.
- The cache-disabled snapshot check failed its rebuild assertion, as expected. It reported left `[2,0]` and right `[1,0]`.
- Both cache tests passed after caching was restored.
- Both standard release builds passed.

## Merged-tree checks

Rust checks passed on merge commit `1c104988fa18853e8dbf447d5fe7ef8c95a0c559` in one container. They passed again after the remote merge at `8eca150d90e7d0fa216823276c01825f8b1189d1`. Both runs used image `sha256:b3b065a8a465210384242521959d701e952aad17ddcfdd4102d42dc68f21d96c`; the final container exited 0.

- `cargo fmt --all -- --check` passed.
- `cargo test --locked --lib cache`: 2 passed.
- `cargo test --locked --lib hash_matches_keep_original_order`: 1 passed.
- `cargo test --locked --lib membership_revisions_cover_push_remove_and_retain -- --include-ignored`: 1 passed.
- `cargo check --locked --target wasm32-unknown-unknown --lib` passed.

The web suite ran on Windows outside the container at `1c10498` and passed 3 tests. Its files did not change through `8eca150`. The first attempt could not access localhost in the sandbox (`EACCES`); the elevated retry passed.

The initial and final Rust logs are `.cache/merge-content-lookup/merged-checks.log` and `.cache/merge-content-lookup/final-checks.log`. Rust reported existing warnings in unrelated renderer and material files. Full tests, release builds, and browser acceptance were not rerun. Garage and driving frame times remain unmeasured. No A/B/A samples were collected, so no performance gain is verified.

## Historical release evidence

The feature branch baseline release build passed in 13m54s. The candidate passed in 13m21s with the same release settings. These build times are not startup measurements.

The shared Docker tag changed during the benchmark. The baseline image ID was not captured. The candidate used image `sha256:4cd6c858e8363e08de584bdcbad9824cef33413a4ef0ae37375ae2b536a57053`. That image was unavailable for the baseline rebuild. The retained packages are not an accepted matched pair.

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

At that time, the frozen manifest check rejected an extra `exclusions` field in `ShaderProperties`. This was a historical export blocker. The main branch later recorded a replacement export that passes native manifest validation. Recheck current assets before treating this as a current blocker.

## Retained evidence

The preserved feature evidence is in `.cache/content-lookup-cache-a435a7f/`:

- `comparison.json`
- `manifest-validation.log`
- `cache-disabled-regression.log`
- `candidate-build.log`
- `baseline-pinned-rebuild-failure.log`
- `assets/`
- `baseline-source/web/pkg/`
- `candidate-source/web/pkg/`

## Run the retained benchmark packages

These commands serve the retained packages from the main checkout. They use preserved feature-branch assets and packages. The historical manifest error may no longer apply to current assets.

```powershell
Set-Location 'C:\Users\Corashina\Documents\Projects\the-long-drive-wasm'
$env:BENCH_VARIANT = 'candidate'
$env:BENCH_ASSET_ROOT = 'C:\Users\Corashina\Documents\Projects\the-long-drive-wasm\.cache\content-lookup-cache-a435a7f\assets'
$env:BENCH_PACKAGE_ROOT = 'C:\Users\Corashina\Documents\Projects\the-long-drive-wasm\.cache\content-lookup-cache-a435a7f\candidate-source\web\pkg'
$env:PORT = '4213'
node scripts/serve-content-index-benchmark.mjs
```

For the baseline, set `BENCH_VARIANT` to `baseline` and point `BENCH_PACKAGE_ROOT` to `baseline-source/web/pkg`. For the final baseline run, set `BENCH_VARIANT` to `baseline-recheck` and use the baseline package.

Open `http://127.0.0.1:4213/` and start the game. Use actual seated movement for the driving scenario. Run instrumented samples outside FPS samples. Focus loss invalidates a sample.

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

No accepted A/B/A samples exist. The report does not establish a speed gain. Gameplay, FPS, startup, Save/Load, streaming, and real input remain unverified.
