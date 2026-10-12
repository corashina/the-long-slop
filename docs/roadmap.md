# Rendering roadmap

Updated: 2026-10-11.

The user requested a stable checkpoint and deferred the remaining rendering work.
The original delivery branch is `codex/material-parity-checkpoint`.
Its material base is `7009f62b4a757069286115617bf38d538e3db5c0`.
Full source lighting and 1:1 visual parity are not complete.

## Delivered material work

| Area | Completed work |
| --- | --- |
| Startup visibility | Hide source helper meshes that cover concrete and the inspection pit. |
| Normal maps | Decode packed source normals once. Keep normal and color conversion identities separate. |
| Texture data | Preserve authored mip levels, sampler settings, texture transforms, and dependency hashes. |
| Material records | Recover enabled maps, channel rules, emission, parallax, and startup condition values. |
| Surface shader | Implement selected Standard BRDF1, detail layers, normal scaling, channels, and source alpha rules. |
| Loading | Share texture handles, release uploaded CPU mip data, and prepare material pipelines before Start. |
| Failure handling | Keep Start disabled when required assets or pipelines fail. Report the failed source dependency. |

Tasks 1–3 passed independent review and their required checks.
The normal-map, material-channel, reflected-transform, loading-fault, and restart fixtures passed.
Checkpoint and local master merge results are recorded in [verification](verification.md).

Garage gameplay, physics, controls, and the road retain their existing implementation.
The local master merge combines these materials with the accepted source day/night and lamp lighting.
It also retains the accepted texture-role storage policy.
Source storage reconciliation remains deferred: five metallic, three AO, and one parallax RGB slots differ.
Three alpha-only mask differences do not affect RGB.
The merge keeps the unfinished Task 4 rendering graph on its recovery branch.

## Preserved rendering progress

Recovery branch: `codex/material-parity`.
Recovery commit: `1f18cc5ada986cc32082fd51bada22b1959d02d5`.
This branch contains unfinished rendering code. It is not a release.

All 36 changed or new product files were preserved and verified against the recovery commit.
The local manifest is `.superpowers/sdd/2026-10-10-material-parity/recovery-product-manifest.json`.
Its SHA256 is `f82768b4ac0e0acc05f7578355b2a4bdd901b04698bf2a56333c98d9c63d5e47`.
Ignored exports, source evidence, diagnostics, and reports remain in the retained worktree:
C:/Users/Corashina/.codex/worktrees/material-parity/the-long-drive-wasm.
Relative evidence paths below use that directory.
These files are local. Git does not contain recovered game assets.

Fixture results apply to their audited revisions.
The recovery branch also contains later untested edits.
Copy/TAA stores, effect interface changes, and the Uber draft lack combined-tree validation.

| Area | Verified progress | Remaining work |
| --- | --- | --- |
| Texture storage color | Source sRGB/linear sampling audit and correction are preserved. | Verify the preserved correction during integration; nine RGB-affecting slots remain incorrect in the checkpoint. |
| Source records | Lighting, camera, sky, sun, probes, and postprocessing contracts recovered. | Final combined export and runtime integration. |
| Surface passes | Nine-view draw issuance, generated-material routing, and ordinary-transform raster checks passed. | Connect final lighting and verify readiness. |
| Camera depth | Seven raster views and known-clear controls passed 219 checks. | Connect final graph inputs. |
| Sky and clouds | Source geometry, direction checks, rebase transport, and isolated draw execution passed. | Verify the final scene and capture order. |
| Shadows | Candidate, plane, hull, split, bias, and projection tests passed. Live preparation compiles. | Connect atlas and collector; verify shadow pixels and offscreen casters. |
| Reflection probe | Filter and sampling checks passed. Capture selection and allocation rules recovered. | Complete six faces, eight-job scheduling, mip generation, and validated publication. |
| Ambient occlusion | Depth, phase, filtering, conversion, execution, and pipeline-reuse checks passed. | Connect source depth and final composition. |
| TAA | Blend, MRT, history resize, and execution checks passed. | Connect source depth, motion, frame timing, and histories. |
| Motion blur | Tile output matches installed DXBC on the tested adapter. Velocity and reconstruction passed 6,048 component checks. | Add the motion-vector producer and connect effect inputs. |
| Exposure | Histogram, exposure, copy, and isolated execution checks passed. | Connect source input and retain histories across resize. |
| Bloom | Constant and spatial filtering checks passed. Spatial comparison passed 2,088 components against installed DXBC. | Connect source exposure, pyramid, and dirt input. |
| Color grading | Neutral 33-cubed LUT bake, lookup, and execution checks passed. | Connect Uber, seeded dither, and final display conversion. |
| Stained glass | Active GrabPass, copy, sampler, pass-cache, and distortion contracts recovered. | Implement the ordered capture and glass passes. |
| Frame lifecycle | View snapshots, projection roles, and rebase arithmetic are prepared. | Connect submission acknowledgements, suspension, restart, and fault handling. |

These checks prove their stated fixtures. They do not prove an assembled source frame.

## Remaining plan

Follow [the deferred rendering plan](superpowers/plans/2026-10-11-rendering-completion.md) when rendering work resumes.
Start from current master. Preserve its lighting, library, UI, instruments, and gameplay during recovery integration.
Add exact source shadows and each later rendering layer to the working scene.
Reuse retained evidence and passed checks when their inputs remain unchanged.

## Known limits

- No matched native frame establishes identical pixels.
- Exact-distance transparent ties use target identities. Native runtime identities remain unknown.
- Nine exported primitives lack authored tangents. Native comparison remains limited.
- The original new-game reload versus history-reset chain remains unproved.
- First-load texture-fetch failures occurred during diagnostics. Their transport cause remains unresolved.
- Embedded-browser pointer lock limits continuous gameplay verification.
- Logical GPU byte counts exclude driver memory and overhead.

Source vehicle, interaction, and Lada radio audio are implemented. Assembled audible acceptance remains incomplete.
The user stopped further checks. See the [audio verification report](research/2026-10-12-audio-verification.md).
Master now includes save/load, the source fleet, and live mirrors.
Survival, multiplayer, and deployment remain outside scope.
