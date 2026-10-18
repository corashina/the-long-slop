# Source performance results

Branch: `codex/source-performance`
Base: `7867d0e`
Implementation head: `84fe013`
The branch remains unmerged.

## Changes

| Commit | Change | Result |
|---|---|---|
| `f798e84` | Share immutable generated materials. | Isolate handles before color and LOD changes. |
| `6f4e16c` | Batch motion-vector uploads and draws. | Reuse a GPU array and group adjacent eligible draws. |
| `4427538` | Retain shadow bounds and view results. | Recompute after caster, view, light, or scene changes. |
| `28845d1` | Update query bounds for changed colliders. | Keep moves, shape edits, enablement, and lifecycle changes visible. |
| `84fe013` | Use the source fixed interval. | `FIXED_DT` supplies `0.01999999955` seconds to the clock and Rapier. |

Each task review and the combined review reported no issues. The combined review covered ownership, motion mapping, shadow recovery, query ordering, and interval use. Reviews are in the [evidence folder](../../../.cache/source-performance/evidence/).

## Verification

- Native suite: 67 passed, 0 failed, 35 ignored. Exit code 0. The ignored tests need installed source exports. See [native results](../../../.cache/source-performance/evidence/task-5-native.log).
- Web suite: 25 passed. This includes 21 core tests and 4 audio tests.
- Installed startup physics: 1 passed. It used a read-only frozen manifest. It saw four wheel contacts and peak angular speed of `0.4559755 rad/s`.
- `cargo fmt --all -- --check`: passed.
- Locked release WASM `--lib --tests` check: passed in 2m59s. It compiled the WASM-only motion history test, but did not execute it.
- Candidate Rust release build: passed in 9m28s. Wrapper and wasm-bindgen elapsed 574.298s (9m34.3s). The candidate WASM SHA256 is `2cedb358585ef532e17e1a3ac3536aca600fdb9309f7a3ef012ffc3e99e502da`.

## Runtime results

Both builds enabled New game. The baseline reached active physics and rendering. Its world still had 32 pending tasks and 2 jobs. The baseline walk moved `0.557 m` with 15 contacts. No baseline FPS sample passed acceptance.

The candidate reached renderer-ready state with no GPU or shader console errors. Controlled input advanced physics from 105 to 125 steps. The player moved `0.489 m` with 15 contacts. Main-frame count advanced from 3409 to 3413. Chromium pointer capture blocked normal Continue input; controlled input started gameplay. The candidate public save produced schema 6.

Public Load failed on both builds with `Missing source asset: sharedassets1.assets:735`. Driver entry was not completed in this check. Door opening, seated driving, streaming, and mutation parity remain unverified.

Baseline and candidate input draw receipts passed `scripts/check-batching.mjs`. A separate 30-frame candidate draw receipt also passed surface and shadow submission checks. These instrumented counts do not show a speedup. The receipts are [baseline input](../../../.cache/source-performance/reports/baseline-input-a1.json), [candidate input](../../../.cache/source-performance/reports/candidate-input-1.json), and [candidate draw check](../../../.cache/source-performance/reports/candidate-draws-1.json).

No garage FPS sample passed acceptance. The first candidate sample had 24 pending world tasks and 2 jobs. A later sample started after the counts reached zero, then failed because world work changed during sampling. No valid matched A/B/A result exists. Driving FPS remains unmeasured. This branch has no verified speedup.

Motion vectors retain the existing position-only coverage. This work adds no alpha or deformation parity claim.

## Frozen inputs and packages

| Input | Value |
|---|---|
| Toolchain image | `sha256:af7ab11c766a4a0a4637b8e772632368f5fa6be9aa76f749fbefd82bdb2cb677` |
| Baseline source archive SHA256 | `29294c187a243fd8bf7c14bf81fb7c90c1881fd4cf145b8c4a50d2d653683131` |
| Asset manifest SHA256 | `a041ac29e18acf889c3c5aa50a909ad103aa5ceb4b98e50402f4e7a714cef288` |
| Release profile | strip, thin LTO, one codegen unit, panic abort, optimization level 3 |
| Canvas | 1280×720, DPR 1 |
| Baseline build | 9m26s; WASM SHA256 `1e90d9f70967cde71cabf66ab546a12eebcc9db1cae144b237d84279a464eef4` |
| Candidate build | Rust 9m28s; wrapper and wasm-bindgen 574.298s (9m34.3s); WASM SHA256 `2cedb358585ef532e17e1a3ac3536aca600fdb9309f7a3ef012ffc3e99e502da` |

Full provenance is in [provenance.json](../../../.cache/source-performance/provenance.json). Runtime receipts and test logs are in [reports](../../../.cache/source-performance/reports/) and [evidence](../../../.cache/source-performance/evidence/).

## Reproduction

Run these commands in PowerShell:

```powershell
Set-Location 'C:\Users\Corashina\.codex\worktrees\source-performance\the-long-drive-wasm'
```

The host does not provide Cargo. Run Cargo checks inside the pinned Docker image. The native suite command is:

```powershell
docker run --rm -v C:/Users/Corashina/.codex/worktrees/source-performance/the-long-drive-wasm:/workspace -v tld-wasm-cargo:/cargo -v tld-wasm-target:/target -w /workspace -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target/source-performance --entrypoint sh sha256:af7ab11c766a4a0a4637b8e772632368f5fa6be9aa76f749fbefd82bdb2cb677 -c 'cargo test --locked --offline'
```

Run web checks from PowerShell with `node --test tests/web/audio.test.mjs tests/web/loading.test.mjs tests/web/input.test.mjs tests/web/performance.test.mjs tests/web/save-storage.test.mjs tests/web/save-session.test.mjs tests/web/save-bridge.test.mjs`. Run formatting with `cargo fmt --all -- --check` inside the same container. The release compile command was `cargo check --locked --offline --release --target wasm32-unknown-unknown --lib --tests` inside that container.

Serve the candidate with the worktree source and candidate package:

```powershell
node .cache/source-performance/serve.mjs --root . --pkg .cache/source-performance/packages/candidate --reference .cache/source-performance/assets/reference --runtime .cache/source-performance/assets/runtime --library .cache/source-performance/assets/library --fixture .cache/source-performance/reports/baseline-garage-save.json --port 4282 --reports .cache/source-performance/reports
```

Serve the baseline with its frozen source tree and baseline package:

```powershell
node .cache/source-performance/serve.mjs --root .cache/source-performance/baseline-source --pkg .cache/source-performance/packages/baseline --reference .cache/source-performance/assets/reference --runtime .cache/source-performance/assets/runtime --library .cache/source-performance/assets/library --fixture .cache/source-performance/reports/baseline-garage-save.json --port 4281 --reports .cache/source-performance/reports
```
