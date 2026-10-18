# GLES sampler cache results

Date: 2026-10-17. Branch: `codex/gles-sampler-cache`. Keep unmerged.
Base: `75ebdffa9b1512deff9b4e6fb7bf690a1a182cd5`.
The `codex/opaque-stencil` experiment is excluded.

The cache removes 37.8% of sampler binds in the garage.
Garage FPS improves about 2%. Severe lag remains.
Driving and streaming acceptance are incomplete because both builds can fail shadow-atlas validation.

## Change and ownership

Bevy's `DrawState` already checks pipelines, bind groups with dynamic offsets, and buffer slices with ranges.
The existing wgpu-core checks also remain unchanged.
The GLES encoder still emitted sampler binds after those checks.
Texture and pipeline changes can resolve to a sampler already assigned to the same physical texture unit.
The stock GLES `rebind_sampler_states` emitted that bind again.

The patch records the final emitted sampler per texture unit in the GLES command encoder.
It compares handles after resolving the existing logical sampler map.
Unknown state differs from an explicit null binding.
The fixed array adds no heap allocation.
There is no JavaScript call suppression.

| State or event | Rule |
| --- | --- |
| Texture units and targets | Samplers belong to units. Texture targets and texture parameters keep their existing commands. |
| Sampler state | Compare the resolved immutable sampler handle. Preserve changed handles and explicit null binds. |
| Vertex arrays and attributes | Keep existing ownership and commands. No attribute cache is added. |
| Buffer targets, ranges, offsets | Keep existing commands. Native tests check changed dynamic uniform offsets. |
| Pass transitions | Mark all sampler entries unknown at each render and compute pass start. |
| Encoder restart | Existing `State::default()` clears the cache. |
| Uploads and deletion | Uploads do not bind samplers. Existing wgpu resource lifetime rules remain authoritative. No cache survives into another pass or encoding. |
| Presentation and context recovery | Presentation can unbind unit zero. No cache crosses encoding boundaries or a new context. |

`wgpu-hal 29.0.4` is vendored with its licenses and exact upstream provenance.
Only `src/gles/command.rs` has production changes.
See `vendor/wgpu-hal/PATCH.md` for dependency update and removal rules.

## Matched release method

Both builds use base `75ebdffa`, the same manifest and lockfile, and a local `wgpu-hal` path.
The baseline uses the unchanged published backend. The candidate uses this patch.
Both use the normal release profile: optimization 3, thin LTO, one codegen unit, abort, and strip.
No profiling feature or reflection preparation hook is enabled.
Private target directories prevent stale source reuse.
Both release builds passed with the same 11 existing warnings.

Toolchain: rustc 1.99.0, cargo 1.99.0, wasm-bindgen 0.2.129.
Browser: Chromium 155, Windows, ANGLE D3D11, NVIDIA RTX 4070 Ti.
Canvas: 1280 by 720, device pixel ratio 1. Source assets and quality settings are equal.
The asset manifest SHA-256 is `3B5C9A7BC370FDD9C0F71C5CFA4D773231D3DDDEABF90C0D19FB2F391C6BD51B`.

| Artifact | SHA-256 |
| --- | --- |
| Baseline WASM | `4FB885955E568A5A6218AD6213E99320A63673C25E586602A8B7E2F95E166A9F` |
| Candidate WASM | `1DBE5FA03C0A7A3BD75AD772D290744BB2B84074ACE95080ACB11CEBFAA2557C` |
| Shared Cargo.toml | `2656C197EC7434F9B5FF68853009FF5EDE0086F7436FD320AC7155505203D732` |
| Shared Cargo.lock | `BEC00D27E4425E6CB15650D50CD12E587C0EFE15D8AA804167BD3279790997EE` |

The benchmark window was coordinated with the startup, batching, shadow, and reflection chats.
No compiler or second game tab ran during FPS samples.
The sequence was baseline, candidate, then a fresh baseline recheck.
Each block contains three ten-second samples after at least ten seconds of active warmup.
All use the default garage camera, the same seed, 111 active surfaces, and 2,517 colliders.
Candidate and baseline recheck start near simulation steps 3,581, 9,006, and 10,165.
Matching differs by at most five steps.
Idle vehicle physics can still change the parked vehicle slightly.

FPS uses passive animation-frame timestamps with no graphics or frame callback wrappers.
Submission tokens confirm continued rendering in every accepted sample.
These are browser frame intervals, not GPU timer or monitor presentation measurements.
The input bridge hides overlays and prevents focus pauses. Normal mouse capture fails in this browser.
That input limit applies to both builds.

## FPS and frame intervals

Values are the median of three samples in each block.

| Build | FPS | Mean ms | Median ms | p95 ms | p99 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 9.55 | 104.75 | 104.40 | 119.90 | 126.70 |
| Candidate | 9.74 | 102.66 | 101.80 | 118.70 | 120.90 |
| Baseline recheck | 9.53 | 104.98 | 103.40 | 121.40 | 142.60 |

Candidate FPS is 2.0% above the first baseline and 2.3% above the recheck.
Sample ranges overlap. This short test does not establish a general game speedup.

## Separate WebGL call samples

The counting probe forwards every call and restores each method afterward.
Its frame times are excluded from FPS results.
Both count samples start near step 11,295 and run for ten seconds.
Values below are calls per submitted frame. Baseline submits 94 frames; candidate submits 96.

| Command | Baseline | Candidate |
| --- | ---: | ---: |
| bindSampler | 42,596.8 | 26,512.4 |
| Repeated bindSampler | 16,287.1 | 290.5 |
| bindTexture | 20,035.1 | 20,019.7 |
| texParameteri | 40,065.9 | 40,035.0 |
| bindBuffer | 30,666.4 | 30,650.7 |
| bindBufferRange | 11,801.7 | 11,795.6 |
| vertexAttribPointer | 29,161.0 | 29,134.8 |
| useProgram | 3,943.1 | 3,940.6 |
| Draw calls | 5,365.5 | 5,363.9 |

Sampler binds fall 37.8%. Repeated sampler binds fall 98.2%.
Sampler binds per draw fall from 7.939 to 4.943.
Draw count differs by 0.03%.
Other state categories retain their commands.
Some repeated sampler calls remain because each pass starts unknown.

## Correctness checks and limits

- Four native tests pass. They cover unit ownership, null binds, target changes, sampler mapping, dynamic offsets, and pass resets.
- All four tests fail their assertions on the stock backend. All pass again after restoring the patch.
- Both builds render the garage with visible materials, glass, mirrors, and shadows.
- Both probes complete repeated reflection generations during twenty-second observations.
- Baseline probe generations advance from 173/172 to 180/180. Candidate generations advance from 188/188 to 196/196.
- All four main shadow cascades issue draws in both observations. Reflection and mirror views also issue shadow draws.
- The stock driving attempt fails validation on the legacy `source shadow atlas`. Only one render frame submits during 395 callbacks. The FPS sample is rejected.
- Candidate driver entry, ignition, and a short reverse attempt submit frames. Public Recover then reaches the road and triggers the same atlas error.
- No matched driving FPS or driving call reduction is claimed. Long driving, streaming unload/revisit, and moving shadow acceptance remain open.
- Save preparation fails in both builds with `Missing source asset: reference/food.gltf`. Saved-scene reload acceptance remains open.
- Forced context loss from Ready produces the same reload instruction in both builds. Page reload creates fresh graphics resources.
- Both context-loss reloads reach Ready, present frames, and accept controlled walking input.
- Normal Continue fails Chromium mouse capture in both builds. Normal mouse input remains unverified.

The branch does not change the atlas, save admission, input capture, or rendering quality.
It remains an isolated sampler experiment with a small measured garage benefit.

## Evidence and rerun

Raw reports, screenshots, package hashes, and build logs remain in `.cache/sampler-benchmark/` in this worktree.
`reports/summary.json` contains accepted values, rejected samples, and missing evidence.
`reports/environment.json` records the adapter and toolchain.
`native-green.log`, `native-red.log`, and `native-green-restored.log` record the backend test checks.
See the adjacent `README.md` for the server, input probe, and summary commands.
Run `scripts/check-gles-samplers.ps1` for the four native backend checks.
