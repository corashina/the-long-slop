# Shadow preparation validation

Date: 2026-10-19.
Branch: `codex/shadow-preparation`. The branch remains unmerged.
Baseline: `738ce8b2b907491725c52b3a917c42f552ac9174`.
Candidate Rust library: `0347697`.

| Check | Result |
|---|---|
| Four native integration targets | 10 tests passed |
| Native `reference::shadow` unit tests | 4 tests passed |
| Async benchmark polling regression tests | 2 tests passed |
| WASM workspace compile check | Passed |
| Formatting and JavaScript syntax | Passed |
| Baseline and candidate release builds | Passed |
| Active-game CPU profiling | Blocked at startup |
| Moving shadows, light updates, streaming, and repeated Load | Unverified |
| FPS A/B/A | Deferred |
| Full test suite | Not run |

The WASM builds produced four existing material warnings. This change did not modify that material code.

## Runtime blocker

Both release packages stop with the same error:

```text
Source lighting pose: Selected vehicle reflection probe count is invalid
```

The error comes from `src/presentation/lighting_scene.rs`. New game remains unavailable.
Both diagnostics used manifest hash `f1a86a6c91448e2c1d07dd90dae273cc136546d0e39f3e1b5cef963b2f7c9a8d`.
The corrected runner confirmed that profiling was enabled.
Candidate caster selection, fitting, split preparation, and upload counts remained zero.
Scene and queue counters recorded readiness checks only. These results cannot establish a performance gain.

Receipts and screenshots are in `.cache/shadow-preparation/diagnostics-verified/` inside this worktree.
Package identities are in `.cache/shadow-preparation/baseline/build.json` and `.cache/shadow-preparation/candidate/build.json`.
Each receipt records the package hashes and Chrome version.

## Check commands

```text
cargo test --locked --test source_shadow_preparation --test shadow_cache --test opaque_shadow_batching --test shadow_uploads
cargo test --locked --lib reference::shadow
cargo check --locked --target wasm32-unknown-unknown --workspace --lib
cargo fmt --all -- --check
node --test tests/web/benchmark-poll.test.mjs
node --check scripts/profile-shadow-preparation.mjs
```

The release builds used optimization level 3, thin LTO, and one codegen unit.
[Profiling options](shadow-preparation-profiling.md) describe the repeatable browser runner.
