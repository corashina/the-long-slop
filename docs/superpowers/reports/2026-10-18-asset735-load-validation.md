# Asset 735 load validation

## Cause and change

Save validation treated each lighting image as an `AssetServer` load. Fixed lighting images come from decoded payloads and `Assets<Image>::add`. These handles have no `AssetServer` dependency record. This check rejected the valid image `sharedassets1.assets:735`.

Source inspection identifies asset 735 as `densmap2`, a 512 × 512 `Texture2D` with ten mips. The verification receipt confirms all 24 exported payload files match their expected sizes and hashes. See [payload verification](../../../.cache/asset735-fix/source-asset-verification.json).

The validator now checks fixed payload readiness and confirms each lighting image remains in `Assets<Image>`. File-loaded glTF assets still require `AssetServer` dependency completion. See [save validation](../../../src/presentation/save.rs) and [fixed image assembly](../../../src/presentation/fixed_assets.rs).

## Build and review

The release WASM build passed in 5m43s with exit code 0. The output SHA256 is `3984dfc3e94b20829bb0f1292efb01a202700500fcd61325190aa17e1d939e46`. The verified replacement image was `sha256:73d988714da759d6834ab5a333e82b7920f730aff5355276af59de68557bf8cb`. It has separate provenance from the original pinned image. Independent review found no issues.

The earlier locked library check passed with exit code 0 and 11 existing warnings. It reported no warning in changed files. See [WASM check log](../../../.cache/asset735-fix/wasm-check-replacement.log) and [release build log](../../../.cache/asset735-fix/release-build.log).

## Runtime result

The retest began from a fresh retry. It advanced 14 physics steps with 15 contacts, then called `probe.load` with the unchanged fixture. The browser remained visible and focused. Animation frames and timers remained live.

The public bridge dispatched `Prepare`, then stopped responding. After 120 seconds, the probe aborted with `Save operation canceled`. `gameUpdate`, `gameHasSaveRequest`, and `gameMeasureRequested` all stopped at 311. `gameSaveRequest` was 1; `gameSaveResult` remained 0. No committed replacement, readback, or post-load physics check occurred. The [runtime receipt](../../../.cache/asset735-fix/reports/asset735-load-runtime-blocker.json) records the counters and final state. The [first attempt](../../../.cache/asset735-fix/reports/asset735-first-attempt-stalled.json) also stalled during loading.

The console capture contains no post-`Prepare` exception. The cause remains unknown. The source-level validator correction compiled and passed review, but runtime evidence does not prove the Asset 735 fix. Full Load remains unverified. This result makes no GPU or FPS claim.

## Provenance

Source commit: `ca2ab30ba0dc1191001f606d7f12c7e3d4710ab5`. Branch: `codex/source-performance`, unmerged. See [toolchain and build provenance](../../../.cache/asset735-fix/provenance.json).
