# Reflection face preparation

Status: runtime acceptance is incomplete. No accepted FPS result exists yet.
The user requested a local merge on 2026-10-18.

## Scope

Branch: `codex/reflection-face-preparation`.

Base: `75ebdffa9b1512deff9b4e6fb7bf690a1a182cd5`.

The base includes the approved startup changes. It excludes `codex/opaque-stencil`.
The initial `7eb1774` base could render but could not finish startup.
The provisional `703d8e6` comparison was replaced after master advanced.

`snapshot()` previously produced six capture snapshots each frame.
Shadow fitting, shadow queueing, and related view preparation consumed all six.
The render graph selected one face job.

The change retains the six face identities and all camera entities.
After renderer warmup, it produces only the scheduled face snapshot.
Mip and publication jobs produce no capture snapshots.
Scene restart resets the warmup latch.
Streaming does not reset that latch.

The complete probe frontier remains in streaming admission.
That frontier is required preparation, separate from capture preparation.
Capture order, acknowledgements, resolution, filtering, and publication are unchanged.
`probes.rs` and `render_targets.rs` are unchanged.

## Verification

Seven native checks passed on the current base before the final guard correction:

- Two reflection preparation checks.
- Two source view-domain checks.
- Three loading-dispatch checks.

The reflection check failed when runtime selection was changed back to six faces.
It passed after selection was restored.

The candidate and timing-only baseline compiled with the same locked release profile.
Both contain the same opt-in preparation counters.
Disabled counters perform no clock reads or allocations.
They are not unmodified release binaries.

| Artifact | WASM SHA-256 |
|---|---|
| Candidate before final guard correction | `7B7B3F357875F542452A7F17E0B3B222744EB94815C7330544403C05B935CA8D` |
| Timing-only baseline | `B9B1D102C97D9B5D122CCD9C81086D9E29ACD84E3217048B6C3A598023DE3F96` |

Both use manifest `3B5C9A7BC370FDD9C0F71C5CFA4D773231D3DDDEABF90C0D19FB2F391C6BD51B`.
The manifest identifies source build `11140890`.
The runtime package is `1590268be14053b8ddc55aa5ab5f724e78ebc89df786c3bc11fc1a486a7399f3`.
The viewport was 1280 by 720, with DPR 1.

The saved runtime receipt is `.cache/reflection-benchmark/candidate-functional.json`.
It applies to the candidate hash above, not the later source correction.

## Runtime observations

The candidate completed startup and enabled New game.
The public New game action started a session.

| Preparation state | Observed capture faces per call |
|---|---:|
| Warmup snapshot | 6 in all 80 recorded calls |
| Normal face job | 1 |
| First mip job | 0 |
| Remaining mips and publication | 0 |

The one-face and zero-face results held across all nine measured stages.
These stages cover snapshots, shadow fitting, extraction, queues, cascades, collectors, caster uniforms, surface uniforms, and transparent order.
Some early warmup stages did no work while resources loaded.
No warmup stage prepared a partial set of faces.

Chromium rejected pointer capture.
The test used the public browser input bridge instead.
This does not prove native pointer capture.

The test opened the driver door, entered the seat, and started the engine.
The engine reached about 800 RPM.
A short reverse action moved the vehicle about 0.43 metres.
This was not a sustained road test.

The renderer then failed with this validation error:

```text
Attempted to use Texture with 'source shadow atlas' label
with conflicting usages. Current usage TextureUses(RESOURCE)
and new usage TextureUses(DEPTH_STENCIL_WRITE).
```

Both probes still reported `published=true`, at generation 441.
This confirms retained publication metadata after failure.
It does not prove the published texture pixels by readback.

The error label contains spaces.
`lighting.rs` allocates that legacy local-light atlas.
`shadows.rs` writes it.
The separate per-view atlas uses the label `source_shadow_atlas`.
The sampler chat independently reproduced this error on stock master `75ebdffa`.
That standard release used stock `wgpu-hal`, without reflection hooks or the sampler cache.
Driver entry, ignition, handbrake release, and reverse triggered the same error.
Its driving sample had 395 callbacks but only one submitted frame. It was rejected.
This peer evidence confirms the failure exists in the baseline.
The root cause remains open. Sustained driving acceptance remains blocked.

Public load preparation failed with `Missing source asset: reference/food.gltf`.
The file exists on disk. The retained-asset check rejects its renderer state.
Successful repeated Load acceptance remains open.
Master later gained a fixed-owner save validation correction at `6a557e0`.
The `food.gltf` failure above belongs to the tested `75ebdffa` package.
Repeated Load must be checked against the merged source before assigning its current status.

The sampler chat also tested both standard75 releases.
Both failed save preparation for `food.gltf` and Chromium pointer capture.
Both recovered to Ready after context loss and reload, then accepted controlled walking.
These are peer baseline observations, not final-source reflection acceptance.

## Pending work

The final source correction restricts new skip guards to probe indices 3 through 8.
Stack and mirror preparation retain their original behavior.
That correction has not been rebuilt or tested yet.
The benchmark now checks submitted frames and capture-cycle progress.
Those added sample checks also await runtime validation.

Rebuild the corrected candidate in a coordinated resource window.
Run native checks again on the merged source.
Retain the sampler chat's stock-baseline atlas reproduction with the final evidence.
Collect accepted FPS, frame-time, and preparation-time samples.
Complete visual comparison for missing faces, stale content, and seams.
Keep driving and repeated Load limitations explicit.

The previous browser export call stalled for 1593 seconds.
The tab was then closed and the resource window was released to the WebGL chat.
No timing sample was active during that stall.

## Benchmark tools

`web/reflection-benchmark.js` exports `sampleReflection()` and `createReflectionTestInput()`.
The sampler rejects incomplete startup, hidden tabs, changing world work, and paused gameplay.
It also rejects callback counts that differ from submitted frames by more than two.
Each enabled probe must complete a capture cycle during the sample.
FPS samples disable preparation profiling.
Separate samples enable preparation timing and face counts.

`scripts/summarize-reflection-benchmark.mjs` summarizes saved samples.
It rejects different viewport, DPR, or source identities.

The input helper overrides public host calls only in the test page.
Call `restore()` or reload after use.
Keep `autoResume` off when testing pause behavior.

The verified interaction sequence was:

1. Start New game and install the input helper.
2. Enable `autoResume` and wait until `gameState.paused` is false.
3. Aim at Car door.
4. Hold input bit 512 and drag with mouse deltas -90 and 90.
5. Release after 500 milliseconds. Confirm the door opens.
6. Aim at Driver seat. Confirm action mask 256.
7. Tap input bit 32. Confirm `gameState.seated`.
8. Hold ignition bit 1024 for two seconds, then release.
9. Confirm `running=true` and about 800 RPM.
10. Hold reverse input bit 2 and press bit 16 to release the handbrake.
11. After three seconds, release reverse and press bit 16 to brake.

Aim from the visible scene. Do not replay unverified coordinates.
The source seat record is `level1:69296`, in vehicle `level1:77839`.
The saved scene fixtures from the shadow-cache chat failed load preparation.
They are not accepted load fixtures.
