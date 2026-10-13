# Rendering completion ledger

Date: 2026-10-12. Branch: `codex/rendering-completion`.

This record preserves the execution ledger and source rulings from the isolated worktree.
Entries describe intermediate states. Read the latest status in [verification](../verification.md).
The implementation keeps ignored evidence and game assets in the retained worktree.
Evidence paths use `C:/Users/Corashina/.codex/worktrees/rendering-completion/the-long-drive-wasm` as their root.
No merge, push, deployment, or native pixel-equivalence claim follows from these checks.

## progress.md

## SDD ledger - plan: docs/superpowers/plans/2026-10-11-rendering-completion.md
Base: dccd516387dff48bf9388ad96ecd52eab3f29c00. Branch: codex/rendering-completion.
User approved full rendering resumption, parallel subagents, isolated worktree, and working layer gates on 2026-10-12.
Tasks: 1 texture color; 2 shadows; 3 probe; 4 motion; 5 postprocessing; 6 stained glass; 7 lifecycle and verification.
Preflight:
| Tasks | Shared boundary | Finding |
| 1 | Image bank and source storage flags | Preserve master seven-bank layout; merge source flags only. |
| 2 | View snapshots, caster dispatch, atlas targets | Root owns shared interfaces; shadow worker owns queues/atlas/collector. |
| 3 | Surface rendering, submission and capture targets | Root supplies scene encode callbacks; probe worker controls job/publication state. |
| 4 | View snapshots, caster dispatch and history | Root supplies draw route; effects worker supplies motion history and output. |
| 5 | Depth, motion, HDR, transparency, camera order | Root wires graph; effects worker owns camera chains. |
| 6 | Ordered transparent stream and color targets | Probe/glass worker owns captures; root wires pass stream. |
| 7 | All prior layers and readiness | Run after connected pipeline passes. |
| 2/3/4/5 | view_frame/render_targets/surface_graph | Single root owner prevents interface races. |
| 3/6 | probes/stained glass | One worker owns both; integration gates remain separate. |
| 4/5 | motion inputs/effect consumers | One worker owns both; producer gate precedes chain integration. |
Ruling: Use parallel disjoint module ownership as the user requested; override the skill's sequential implementer default. Risk: interface coordination requires explicit root ownership.
Ruling: Resume the existing approved source design and deferred completion plan. The user's current request restores full rendering scope while retaining master's day cycle. Risk: older startup-only clauses need current lighting inputs.
Ruling: The later native GrabPass evidence controls capture ordering; include earlier glass draws in later captures. Risk: incorrect source interpretation requires glass rework.
Recovery branch name is absent. Commit 1f18cc5 exists and provides the source snapshot.
Copied retained source reports and current master runtime assets/package into this worktree. Recovery originals remain unchanged.
Baseline: pending Docker startup and fresh checks. Workers may audit before the baseline passes.
Baseline native: 233 passed, zero failures. Copied current package Linux Chromium startup, pointer lock, simulation and pause passed. WASM baseline still compiling unchanged active modules. Workers released to disjoint, unregistered recovery modules.
Baseline WASM release finished successfully in 6m40s. Baseline browser passed startup/lock/simulation/pause.
Task1: source stored m_ColorSpace correction implemented in materials.py and rendering validator; Python17 passed. Exact source export generated and 1388 Python dependency/metadata checks passed. Native239 passed; required new render_lighting manifest validated; combined all-target WASM check2 passed with two private-interface warnings.
Task1 frozen snapshot: .cache/layer1-source built release+bindgen in 1m10s; browser gate running at .cache/layer1-browser. Snapshot starts from dccd516 plus new source schema/reference modules and exact corrected export, preserving original active renderer for the first gate.
Current ownership:
- shadows agent owns shadow_* and reference schemas/exporters, then materials.rs/renderers.rs/world.rs/materials/generated.rs/instruments.rs/world/lamps.rs + source_surface/source_mesh/source_caster/source_view_lighting shaders.
- effects agent owns effects/motion shaders+Rust and camera_depth.rs/motion_draw.rs. No shared builds.
- probes_glass agent owns probes/glass and shared source_lighting/lighting_assets/lighting_scene/surface_views/view_frame; now sole graph owner for app.rs/frame_graph.rs/surface_graph.rs/transparent.rs/source_sky.rs/clouds.rs. It must request runnable source-shadow gate before later integration.
- root owns builds/browser/reviews/module declarations/render_targets.rs/caster.rs/frame_copy.rs. No overlapping edits.
Root added persistent FrameCopy, lean caster group0, COPY_SRC target usage and SurfaceTargets.resize_stack preserving fixed capture targets.
Source reference lighting.rs recovered as render_lighting.rs; presentation lighting.rs recovered as source_lighting.rs. Existing master lighting remains active.
Ruling: Use a frozen first-layer fixture to validate corrected storage before later shader/graph changes. Risk: later assembled tree still requires independent final verification.
Effects agent had one transient model-capacity error; resumed same agent via followup_task.
No rendering completion, runtime shadow acceptance, probe publication, effects acceptance, glass pixels, or 1:1 parity claimed yet.
Task1 runtime gate passed: frozen texture layer startup 55136ms; trusted pointer lock; simulation 12+ steps; pause releases capture. No HTTP/page errors. One favicon console404 recorded. Screenshot source-ui-ready.png. Layer2 source-shadow integration may proceed.
Root inspected layer1 screenshot: garage/car visible. Promoted exact corrected runtime export to web/assets/reference after preserving previous export at .cache/pre-rendering-runtime. Master checkout untouched.
Effects implementer handoff complete; review package effects-review.diff created from dccd516 including untracked owned files. Root check4 did not find errors in newly registered motion draw (shared material/schema interfaces failed separately). Guard changes still require next compile and connected validation.
Effects scoped independent review passed static spec and quality with no confirmed defects; connected acceptance remains open. Required enable_instancing recovered from exact m_EnableInstancingVariants (137 true/155 records), Python18 passed; exact export revalidated and promoted to worktree runtime. Layer2 compile1 has one raw/wrapper output view error and five graph warnings; root fixes FrameCopy output raw view, graph owner fixes warnings. Source restart PressedRestart Single reload proved; report pending.
Layer2 source-shadow package check4/release/bindgen passed without warnings (release1m31s). On-demand source_render_audit export included.
Shadow/material static review fix1 passed spec+quality; both P1 findings addressed. Exact HDR replacement disabled blend+half4; additive stays raw. Collector now typed Depth and scalar sample.
Connected Linux browser first run failed harness-before-WASM-init, not a product error. Corrected harness waits Loading the garage before calling audit export. Run2 in progress, file .cache/layer2-browser/run2.log.
Source scene layer currently intentionally keeps Start closed until final probe/effect requirements. Root keeps passed texture-only working snapshot .cache/layer1-source.
Source restart proof saved restart-source-report.md: PressedRestart reloads active scene through LoadSceneMode.Single; effect/camera/probe state reconstruction required. Preserve storage allocation where valid; reset dither and publication validity on recreated layers.
Layer2 Linux run2 failed at startup: missing DrawFunctions<SourceShadow> before AddRenderCommand registration. Exact panic retained run2-failure.json. Graph owner initialized it before registration.
Ruling: Restore shared culling inverse(V*S) because caster bounds/corners use SOURCE world. Motion independently uses inverse(S*V). Root's earlier instruction conflated matrix roles; cost if wrong is misplaced culling/shadows or motion. New rotated/translated consistency test tests/source_view_domain.rs verifies inverse identity, reflected camera position, point mapping, and distinct motion inverse.
Source-domain native test, all-target check and release package build in progress: .cache/layer2-domain-build.log.
Source-domain test passed1; clean all-target check+release/bindgen passed. Layer2 run3 audit wait returned early because async predicate produced a Promise; sync handle predicate prepared, run4 pending. Rebase model history now uses raw TARGET origin shift; WASM-only property test added, ignored native-state fixture requested for execution.
Motion native-state fixture executed successfully:4 passed; source body hash/wiring equality, registration/stale-frame rules, pure rebase projection invariance and physical static-model rebase. Product algorithms copied verbatim with only GPU derive/import removed; .cache/motion-state-check/extraction.json preserves equality proof.
Layer2 run4 is a real loading gate timeout, not browser control failure: graph frame6659 advanced, but mask0/skyfalse, zero depth/atlas/surface completions; GPUvalidated0; stateLoading. No page/GPU errors. Source material and lamp caches loaded. Last logs 0 receiver/5184 local shadow draws. Root preserved run4-failure.json.
Core3d phase extraction and ViewKeyCache sources have no CameraRenderGraph filter, so the duplicate-phase hypothesis was rejected. Graph owner now adds fixed scalar gate telemetry; no behavior fix without boundary evidence.
Original probes_glass encountered repeated gpt6.1-sol capacity errors and was interrupted to prevent overlapping writes. New graph_integrator (gpt6-astra, high) now owns all prior graph/app/preparation/probe/glass files. Root builds/browser remains single owner. New gate_telemetry (gpt6-sol) implemented typed MaterialPrewarmAudit and removed old array API; graph uses canonical prewarm_audit(). Telemetry compile/GPU run pending.

Layer2 hardware diagnosis: SwiftShader lacks HDR8, while native hardware supports8 but panicked in glow multisample texture storage. HAL29 renderbuffer path requires attachment-only usage; unconditional COPY_SRC on source_msaa_hdr/depth selected unsupported texture allocation. Root removed copy usage only from multisample targets. Resolved single-sample targets retain copy/sampling. Exact8 unchanged; runtime verification pending. Master failure propagation added by graph owner.

Native hardware passed exact8 allocation after attachment-only multisample usage. It then reported stars/flares alpha_blend_mesh_pipeline Depth32Float against D24S8 sky pass. Graph owner sets SkyMaterial specialization depth format to source D24S8. Native audit had all material warmups ready and all source pipelines compiled; no full shadow frame completed before validation failure. Separate skybox projection now uses source_sky_gpu_projection while clouds/stars keep ordinary sky-camera projection. Fresh coherent build log layer2-sky-depth-build.log pending. Ignored native evidence server at4225 saves full audit JSON through same-origin POST; original preview4224 remains available.

Latest sky/depth-specialization package check/release/bindgen passed. Hardware then failed collector WGSL depth texture mip level type, fixed integer0; GLES subsequently rejected generated noncomparison depth textureLod. Full failure audit retained native-gles-depth-failure.json. Naga29 also rejects textureLoad on typedDepth. Graph owner found supported wgpu-core29 Float{filterable:false} over DepthOnly D24 view (resource.rs3541..3556). Bounded fix uses matching WGSL float texture+unfiltered float binding and point/clamp texelLoad on actual D24 storage, with no mirror or shader quantization; comparison atlas remains Depth. Same raw transport needed by AO depth copy and motion camera. Patch pending. Native Cua CDP diagnostic POST must use nonawait evaluate (void import().then(...)); awaitPromise calls stalled for several minutes, nonawait saves immediately and server result is checked separately.

Raw-depth package all-target check/release/bindgen passed. Exact shipping collector, AO-copy and camera-motion WGSL Naga29 GLES300 fixture all3 passed; extraction hashes matched. Native hardware raw-depth frame retained native-raw-depth-frame.json: sky true, mask7, presented and deferred validated true, frame682/token678. Main cascade attempted=issued [102,84,34,35], cameraDepth119/119, base152/152, add148/148; far7/7, foreground0 valid empty. All9 queue masks511. Screenshot native-shadow-vertical-flip.jpg confirms final image vertically inverted, so layer2 orientation acceptance remains open. Root holds probes until final boundary fix. Graph owner freshly extracted installed shader66 DXBC2/6 into .cache/final-display-proof and recovered native UV flip/quad DLL slices; proof in progress. Original native cache unavailable; do not invent shader TexelY branch. Native screenshot saved through Cua screenshot bytes and nonawait CDP same-origin POST to local evidence server, then root writes JPEG. Tab1 was removed; fresh background tab2 at4225.

Layer2 orientation fixed only in display_copy; fresh installed shader66/native fullscreen proof maps (screenU,1-screenV). Hardware upright garage image inspected; full counter/package/image evidence native-shadow-pass.json and native-shadow-oriented.jpg. Three surface views, all4 main cascades, draw counts/tokens and deferred GPUvalidation passed. Exact shipping depth GLES fixture3 passed. Fresh native test suites passed 240 with0 failures. Root prepares local source-layer checkpoint before allowing probe integration. Primary remains clean but another task advanced its HEAD to36b08f047bca0f6e117241e5ec26ad8b11f8d640; rendering branch keeps approved base dccd516. No merge/rebase of others work performed.

Local checkpoint81821531bfedb104f459f1c86ef69dec74b27453 retains passed shadow layer and required recovered modules. Layer3 probe build check/release/bindgen passed (release6m45s). Scoped probe static review passed with no confirmed findings; root whitespace check succeeded. Hardware probe counter snapshot: full6 faces mask63, mips127, published generation87, published token696, nextjob8 token704 held during publication validation177. Each64x64 face has positive depth/surface issued draws and matching shadow counts; atlas128. No GPU/page console errors found. Main sky/mask7/GPUvalidated retained. Resize960x540 passed with capture64x64 unchanged and published generation1346; native-probe-resize-pass.json retained in .cache/layer3-browser. Temporary viewport reset. Legacy menu timed out at120s because later post/glass gates intentionally remain closed; render state stillLoading, notGPU failure. Final full readiness must pass before120s. Root prepares probe checkpoint before Task4 motion producer. Terminal material prewarm audit cache removes request/pipeline scans afterReady orFailed; state/seal/vector storage changes refresh it. Larger graph asset scans and draw setup guard remain later performance work.

Task1: complete - texture storage and source export validated; frozen working runtime retained.
Task2: complete - source shadow scene, upright display, issued draws and hardware GPU validation passed at81821531bfedb104f459f1c86ef69dec74b27453.
Task3: complete - six probe faces, required mips, repeated validated publication and resize passed at1b77a1a. Repeat with glass atTask6.
Task4: active - graph owner integrates motion producer only; expected changed files frame_graph/surface_graph/lighting_assets/source_lighting/motion_draw. No effect or glass connection until its gate passes.

Task4 build check/release/bindgen passed (release7m19s). Static motion review passed, no confirmed defect. Shipping state fixture refreshed and4 tests passed0; algorithm hash unchanged. Hardware static frame61/token57: camera/object/submitted57; motion validatedgeneration2;669registered,zero eligible rigid draws valid; mask7 and probe publication retained. Motion resize960x540 passed, count1 and validationgeneration124; full evidence native-motion-resize-pass.json. Viewport reset. Nonzero rigid draw binding path remains unexercised, so postprocessing stays held. Graph owner prepares ignored source overlay fixture with controlled real camera and Object-mode model transforms; shipping motion code/shaders byte-identical and hash receipt. Rebase fixture4 alreadypassed; global origin alone must not be changed without scene/probe/light shifts. No shipping debug control or Start bypass permitted. Root native evidence tab3 at4225; no final Start readiness or full rendering completion claimed.

Execution handoff: graph_integrator now owns remaining rendering implementation, new builds, evidence inspection, and verified local checkpoints.
Root retains native Cua actions because the subagent's Cua inventory is empty. Root also retains final branch review/report.
TASK4 moving fixture source overlay is prepared with 97 unchanged source files and hashed shipping motion code/shaders.
Its check and release build passed; release4m32s, one unused fixture-only tick warning.
Native moving gate remains pending. TASK5 product remains untouched pending that result.
The bounded TASK5 plan is in graph-integration-report.md. It corrects Uber's final destination to original HDR8 before source callbacks.
Fresh source input proof .cache/layer5-input-proof.json maps shutter speed to Player Rigidbody walking and vehicle root while seated.

Task4: complete - shipping static and resize GPU gates passed; moving overlay passed four27-frame stages with validations7/12/17/21.
Rigid attempts=successes289; restore287. Exact first/last camera/object/submitted tokens agree.
Evidence: .cache/layer4-browser/native-moving-motion-pass.json and moving-motion-receipt.json.
Overlay byte verification covered97 unchanged source files plus shipping motion math/shaders. No velocity pixel readback is claimed.
Prior shipping check/release/bindgen, four extracted state tests, and static review passed. Whitespace check passed before checkpoint.
Task5: active - implement bounded direct-HDR8 camera-local effects plan in graph-integration-report.md.
Task4 checkpoint538810e committed locally. No push or merge.
Task5 connected AO and three persistent camera effect chains. Source callbacks use one sequential HDR1 scratch.
Uber writes original shared HDR8 plus resolve; obsolete per-chain final output removed.
All-target WASM check passed. Release/bindgen running in .cache/layer5-browser/build.log.
Independent review requested through root using .cache/layer5-browser/task5-review.diff.
Task5 GPU/image/resize gate remains open. Start remains closed pending final layers.
Task5 package check, release2m58s, and bindgen passed with exit0. Whitespace check passed.
Package hashes saved .cache/layer5-browser/package-hashes.json. Root requested native shipping4225 image and resize acceptance.
Task5 complete: native-effects-pass.json, native-effects-resize-pass.json, and native-effects-steady-pass.json retained in .cache/layer5-browser.
Root inspected upright native-effects.jpg. All effect tokens, AO markers, post7, callback31, and deferred validation matched.
Resize960x540 retained shadow, motion, and published64x64 probes. Counters stayed fixed over1114 later frames.
Console warning/error arrays were empty. Independent effects-layer-review.md passed specification and quality without findings.
Connected effect storage:82338896 bytes at1280x720;46453560 at960x540. Driver memory remains outside this count.
Task6 active next: ordered unnamed glass grabs. Start remains closed.
Task6 corrected all-target check/release2m33s/bindgen passed. Package hashes saved .cache/layer6-browser/package-hashes.json.
Scoped re-review passed after moving DrawFunctions preparation before each camera chunk loop.
Shipping-extracted cache/comparator contract2 passed0; source hashes retained in .cache/layer6-contract/extraction.json.
These checks cover unchanged Grab reuse, intervening BASE invalidation, and earlier far glass before nearer capture.
Native Task6 image/capture/both-material/probe/resize acceptance requested. Start remains closed.
Shipping Task6 GPU/image/resize/steady gates passed. Root retained native-glass-draw-pass.json, native-glass.jpg,
native-glass-resize-pass.json, and native-glass-steady-pass.json in .cache/layer6-browser.
Main image shows room-door47 refraction. Aggregate accepted source material draw counts[2,2] include probe views.
Resize960x540 and1559 steady later frames preserve target bytes, allocation counts, and repeated64x64 probe publication.
Console error/warning arrays were empty. Visible48 and overlap fixture remain the Task6 gate.
Ignored .cache/layer6-glass-fixture overlays frame_graph only and verifies98 unchanged source files and critical shader hashes.
Its stage selector exists only in the verification package. It leaves Start closed and uses real imported glass meshes/materials.

Task6 complete: controlled real47, real48, overlap, and restore stages passed. Receipt native-glass-fixture-receipt.json retains eight hashes. Overlap draws[1,1], captures2, and actual separation0.159999847 passed native image inspection. Shipping GPU/resize/steady and scoped review passed. Task7 lifecycle implementation begins.
Task7 corrected package passed format, all-target check16.26s, release4m34s, and bindgen with exit0.
Scoped lifecycle review passed after shared effective frame delta fix.
Clock contract3 and large-delta transition1 passed. Web suite24 passed.
Native healthy/restart/resize/fault gates requested. Final selected export regeneration and broad native checks remain in progress.
Task7 command verification complete:240 ordinary native tests and42 installed-source tests passed.
The installed export hash gate first lacked primary library receipts. Read-only input mounts resolved that environment failure.
Exporter suite172 ran:171 passed, one installed UI test skipped. The same UI test then passed with input/output path redirection.
Fresh selected export passed1388 checks and the Rust contract. All1380 shipping files match; no promotion occurred.
Receipt: .cache/layer7-browser/final-checks-receipt.json; source hashes and log hashes retained.
Native initial readiness passed in35111ms. Scene1 first-token/jitter, cold histories, first camera registration, and all graph gates passed.
The in-app browser rejected pointer capture because its root document was invalid. Genuine continuous mouse control remains unverified.
Root exercises existing actions only after true renderer readiness for remaining lifecycle acceptance. Shipping sources remain unchanged.

Task7 native restart passed. The transition closed Start and cleared current-scene validation and probe publication.
Scene2 restored cold histories, monotonic tokens, new validated probe publication, and unchanged allocation counts.
Existing game actions were injected after true Ready because the browser refused pointer capture.
Distance and actual rebase acceptance use .cache/layer7-physics-fixture, an app-only overlay.
Its manifest proves98 unchanged Rust sources and27 unchanged shaders. Normal runtime physics triggers the rebase.
No fixture code changes source rendering math, histories, readiness, pause, or elapsed time.

Task7 actual context loss exposed a terminal-state gap: JS closed controls, but Rust still advanced rendering.
The corrected bridge latches the first shared Rust failure and stops Bevy render schedules during extraction.
Native RenderErrorHandler keeps StopRendering and preserves an earlier browser message.
JS gameFailure is display-only, so Rust failure display does not call back into Rust.
Two exact-handler bridge tests and24 web tests passed. Corrected all-target check passed14.35s.
The old physics overlay package is superseded. Rebuild it from corrected sources after review.

Context review found one P2: clearing render schedules alone leaves recurring extraction commands undrained.
The fix moves the terminal check to the existing extraction wrapper, before original extraction.
It also stops on faults raised during extraction or source phase setup. Bevy time delivery stays active.
Bounded re-review: .cache/layer7-context-fix/context-review-fix.diff. Final coherent build is running.

Context scoped re-review passed specification and quality after the extraction guard correction.
Final stock format/check4.92s/release1m51s/bindgen passed. Package hashes: .cache/layer7-context-fix/package-final-hashes.json.
Source/log receipt: .cache/layer7-context-fix/final-source-receipt.json.
Asset failure named required texture735 and closed Start/Continue/Restart. Invalid Uber binding15 triggered actual GPU pipeline failure and closed controls.
Root retains final-asset-failure-pass.json and final-pipeline-failure-pass.json.
Physics fixture v2 uses this exact reviewed source. Its isolated build is running; shipping package remains frozen.

Physics fixture v2 check9.92s/release2m10s/bindgen passed. Fresh verify.py passed98 unchanged Rust sources and27 shaders.
Stock package hashes remain unchanged after the fixture build. Parent owns native context and physics fixture acceptance.

Final corrected context gate passed on native hardware. The healthy package reached Ready with mask7.
Actual WebGL context loss changed shared state to Failed and closed all controls.
final-context-stopped-pass.json and final-context-frozen-pass.json contain exactly equal complete renderer audits.
Frame3677/token3673, histories, probe publication, and global validated generation remain frozen. Physics remains paused at922 steps.
Independent implementation-owner comparison confirmed exact audit equality.
Distance/rebase fixture and whole-branch review remain open. Task7 is not committed yet.


#### Final motion streaming correction

The real-body +80 m fixture exposed a P1 before its distance check: map deletion reduced insertion headroom.
Whole-branch review found this single issue. Its capacity was not the fixed scene bound.
The correction retains exactly the existing snapshot bound in GPU slots and reuses inactive slots.
It rebuilds the entity lookup only when membership changes. New entities get cold histories.
Retained entities keep submitted history; source rebase and motion equations are unchanged.
The audit adds allocated and reused slot counts. Logical uniform bytes include every allocated slot.
Three exact-source tests passed, including 4,096 membership changes, cold reuse, and retained completion/rebase history.
WASM all-target check passed10.24s; formatting/release1m34s/bindgen passed.
Final source/package/log receipt: .cache/final-motion-fix/final-source-receipt.json.
Stock WASM56772820 bytes, SHA256365a657aad3ca3586b8d40c2b14190df5566ad7191b9ebc1b8461fad12c11d60.
The isolated physics v3 overlay preserves98 shipping Rust sources and27 shaders; its package build is running.
Scoped correction review and real-body native stages remain open. Task7 is uncommitted.


Physics fixture v3 check10.55s/release1m59s/bindgen passed. Fresh source equality passed98 Rust sources and27 shaders.
Its separate package hashes are .cache/layer7-physics-fixture-v3/package-hashes.json.
All135 stock runtime file hashes and both stock package hashes remained unchanged after the overlay build.
Parent owns fresh stock readiness and physics stages0 through4. No product code changed after the stock receipt.


### Final Task 7 acceptance

Parent accepted the final corrected package and all five native physics stages.
Fresh stock Ready was observed by72,746ms within120s. Cold camera/effect histories and validated probe remained correct.
The app-only physics v3 overlay retained98 identical stock Rust files and27 shaders.
Real bodies crossed the50m probe limit through actual5-second scaled-time checks, then returned.
The normal runtime produced forward origin[2304,0,-128] and return origin[0,0,-128].
Area and stream origins agreed. Runtime and motion rebase counts advanced0 to1 to2.
All stages retained780 motion GPU slots; final reuse count910. No target allocation count increased.
All stages retained Ready, mask7, sky, presentation, GPU validation, motion tokens, and effect masks7/31.
The final returned probe was enabled and published. All three live dither indices matched(token-firstToken+1)%64.
Far bodies fell outside existing ground; distance includes actual vertical displacement. The fixture changed no renderer state or origin.
Root console error and warning arrays were empty.

Whole-branch review found one P1; its bounded motion correction passed scoped review with no new finding.
Three critical exact-source tests passed. The final full native suite rerun passed240, failed0, default ignored42.
Installed42, exporter171 plus separate skipped UI check, web24, clock3, large-delta1, and byte-identical1380-file regeneration remain accepted.
Final source/package receipt: .cache/final-motion-fix/final-source-receipt.json.
Independent final assertions: .cache/final-motion-fix/final-acceptance.json.
Native receipt: .cache/layer7-browser/physics3-native-receipt.json.
Final stock WASM56772820 bytes; SHA256365a657aad3ca3586b8d40c2b14190df5566ad7191b9ebc1b8461fad12c11d60.
Source and package hashes stayed unchanged through the final checks. No additional product mutation followed native acceptance.

All seven planned layers are accepted. This establishes the tested target behavior, not identical installed-build pixels.
Remaining limits: no matched native pixel frame, no continuous IAB pointer capture, no automated native OS minimize,
unknown native identities for transparent ties, nine primitives without authored tangents, and logical bytes excluding driver overhead.
The installed Single scene-reload trace resolved the earlier restart semantic uncertainty.
The branch and worktree are preserved. No push, merge, rebase, or evidence deletion is authorized or performed.

## graph-integration-report.md

## Graph integration

I assumed graph ownership after the previous worker stopped.
I read the current progress, plan, preparation, restart, material audit, effects, probe, and glass reports.

### Loading boundary

I checked the new CPU and GPU telemetry against its producer APIs.
MaterialPrewarmAudit has fixed-size Copy and Serialize fields.
MaterialLoading::prewarm_audit reads the same request, mesh, material, and pipeline state as prewarm_materials.
The graph copies counters during Cleanup. It serializes them only through source_render_audit.
The audit does not change rendering behavior or allocate a per-frame report.

render_sky waits for nine snapshots, a sky snapshot, sky/cloud/display pipelines, and MaterialLoading readiness.
prepare_views needs sealed capacity, source assets, live lighting, and nine valid extracted extents.
It does not wait for the final SourceLighting readiness state.
prepare_frame needs surface targets, lighting assets, live lighting, and the sky snapshot.
surface_ready also needs the sky pass to complete in the current frame.

These gates explain why one blocked material request can keep sky and surface completion at zero.
The next browser audit must identify the blocked gate before behavior changes.
I made no behavior change before that audit.

The material telemetry report is gate-telemetry-report.md, with a hyphen.

### Later interfaces

ProbePublication owns capture completeness and publication validation.
SourceViewClock owns the eight-job schedule.
The graph must acknowledge jobs after submission and publish after deferred capture validation.
Final publication uses SourceSubmissionAcks::probe_publication_submitted.

SourceAoChain must run after camera depth and before transparency.
The graph must encode main motion before the post chain and commit histories after submission.
SourcePostChain::completed and MotionObjects::completed require the same complete main token.
MotionObjects transports model history during rebase. The graph transports camera history using raw target XYZ deltas.

SourceGlassView::capture reads accumulated resolved color after earlier transparent draws.
The graph must end render passes before each executed grab and clear validity at camera cleanup.
GrabPassCache must observe intervening ordinary passes.

I held later layer integration until the parent confirms the working shadow scene.
The parent owns builds, browser checks, module declarations, target storage, caster dispatch, and frame copies.

I used the stop-slop skill when writing this report.

### Run5 device failure

Root reproduced the blocked state in run5-gates.json.
The master GPU status was Failed. Cookie preparation stopped, leaving242 source materials unprepared.
The source status still showed Loading because it did not inspect its master lighting dependency.
The captured device error reports unsupported RGBA16F sample count8 on SwiftShader.

I traced the installed Bevy0.19.1 sources in .cache/bevy-source.
WgpuSettings::default already requests TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES in render/settings.rs:151.
The renderer unions that request into required_features in render/renderer/mod.rs:318.
The app uses DefaultPlugins without overriding RenderPlugin settings.
I made no redundant feature request or sample-count fallback.
Root will test the exact8 contract on the native hardware browser.

I changed sync_lighting_readiness to inspect both the master shared GPU status and master CPU state.
It now forwards the first master failure into SourceLighting.state and SourceLightingStatus.
The same transition handles material failures and existing graph failures.
After failure, subsequent updates return without cloning the error again.
Healthy frames borrow status values and allocate no error strings.
I did not run a build or browser test. Root owns those checks.

### Main sky projection correction

The source sky shader writes the supplied clip transform and never replaces clip Z.
The previous snapshot used the ordinary finite-far camera projection for procedural sky geometry scaled by far*10.
The skybox-geometry appendix, lines128..160, proves the separate sky depth projection and subsequent camera projection restore.
The skybox-projection-flip appendix proves the target Y flip for both selected paths.

I added SourceViewFrame.skybox_clip_from_world using reference::sky::source_sky_gpu_projection.
The helper applies near*0.01, epsilon1e-6, reversed depth, and the source Y flip.
prepare_frame now passes that matrix to SkyPass only.
The existing sky_clip_from_world remains the ordinary camera matrix for clouds, stars, and flares.
I changed view_frame.rs and frame_graph.rs. git diff --check passed.
Root must build and inspect the resulting sky pixels.

### Next probe patch plan

This plan remains gated on root's complete shadow frame. I added no probe execution code.

1. Add persistent probe graph state beside FrameResources: ProbeFilter, ProbePublication, encoded token, and validation generations.
   Construct the filter once from SurfaceResources.probe and LightingRenderAssets.shaders.probe.
   Add a named filter pipeline failure check. Pipeline waits must retain the current job.
2. Retain the existing nine extracted cameras. Main views use shared HDR8; capture views use shared64x64 HDR1 scratch.
   Each capture view keeps its own depth, collector, atlas, lighting uniform, and glass storage.
   Keep their sample1 camera keys and native face matrices. Select execution with frame.probe_job, not camera activation.
3. Allow only view3+face during its selected Face job, after current shadow queues are ready.
   Clear its color, then run camera depth, atlas, collector, opaque, procedural sky, and ordered transparency.
   Use SkyPass binding1+face and source_sky_gpu_projection(snapshot.implicit_projection, snapshot.near).
   Pass the live probe position and far25. The capture shader suppresses the sun disk.
   Copy completed scratch HDR1 to ProbeTargets.base mip0, array layer face. Only then call publication.encoded.
4. Encode FirstMip or RemainingMipsAndPublish after camera rendering and before RenderGraphSystems::Submit.
   Process job acknowledgements in Finish, which Bevy defines after Submit.
   Advance face/mip jobs only when publication.submitted(token) returns true.
   For job8, watch its submission generation and retain the job during deferred validation.
5. After validation, call publication.validated, then encode_publication in a later submitted frame.
   After that submission, call publication_submitted and acks.probe_publication_submitted with the same token.
   Watch the publication copy too before first_probe_ready. Preserve the previous cube throughout capture waits.
6. Expose face/mip masks, job token, publication generation, and validation generation through fixed audit fields.
   Bridge probe pipeline and first-publication readiness into main-world SourceLighting.
   Keep Start closed for the remaining motion/effect/glass gates.

Material bindings already use the correct resources.
Main SourceViewLighting group1 binds ProbeTargets.published_view at binding3 and the stable probe sampler at binding4.
Capture bindings use capture_sample_view, the black cube, with ordinary candidate selection.
Publication copies into the existing published allocation. No material or bind-group replacement is needed per publication.
Stack resize preserves capture targets, probe storage, filter bindings, and the previous valid publication.
Scene restart must clear publication validity while retaining allocations where safe.

The selected source probe is level1:59674, mask349345813, near0.8399999737739563, far25, shadow distance5.
The source scratch appendix proves one sample and no private-camera postprocessing.
The capture-selection appendix proves the black local-probe override and retained main publication.
The source distance activation check, every5 seconds with squared distance<2500, remains a lifecycle integration item.
Final probe acceptance must repeat after ordered glass joins capture draws; the current shadow layer excludes glass.

### Sky extras depth format

Hardware reached draw_extras, then rejected alpha_blend_mesh_pipeline with Depth32Float against the source D24S8 attachment.
The retained stars and flares use SkyMaterial through Bevy's ordinary transparent material draw function.
Bevy first specializes its mesh pipeline, then calls SkyMaterial::specialize.
The previous sky specialization disabled depth writes but retained Bevy's depth format.
The source_sky_extras render pass uses FrameResources.sky_depth, allocated with render_targets::DEPTH_FORMAT.

I set depth.format to that same source DEPTH_FORMAT inside SkyMaterial::specialize.
The pipeline retains its camera sample count, blending, and depth-write behavior.
The change uses the existing attachment. I added no target or fallback.
git diff --check passed. Root must build and rerun the connected hardware check.
The diagnostic steady-scan change remains deferred until this draw fault is resolved.

### Raw camera depth transport for GLES

Hardware compiled the collector into an invalid GLSL textureLod call for a depth comparison sampler.
The installed Naga 29.0.4 GLSL writer declares non-MS Depth images as shadow samplers (writer.rs:594-598).
Its explicit depth load path also rejects WGSL textureLoad (writer.rs:4071).
Changing only the sampling operation therefore cannot fix this backend.

wgpu-core 29.0.4 explicitly accepts unfilterable Float bindings for Depth texture views (device/resource.rs:3541-3556).
The collector, AO camera copy, and camera motion pass now use that supported binding.
Each WGSL camera input is texture_2d<f32>. Each matching Rust layout uses Float { filterable: false }.
The physical input remains the existing D24S8 DepthOnly view. No mirror texture or depth quantization was added.
The existing source AO R32Float copy remains unchanged.

Matching group0 layouts:
- Collector: 0 uniform; 1 raw camera depth Float; 2 atlas Depth; 3 comparison sampler.
- AO copy: 0 uniform; 2 raw camera depth Float.
- AO downsample/tile: 0 uniform; 1 AO input Float; 2 raw camera depth Float.
- Camera motion: 0 uniform; 1 raw camera depth Float.

Raw reads use textureLoad at mip0, selecting the red component.
The collector maps UV to clamp(floor(UV * extent), 0, extent - 1).
AO and motion map fragment centers to clamp(floor(position.xy), 0, extent - 1).
These coordinates preserve point sampling and edge clamp. Existing UV and projection orientation remain unchanged.
The source Point/Clamp contract is documented in taa-input-samplers-appendix.md:34-58.
The obsolete collector and AO point samplers and their bindings were removed.
The shadow atlas retains typed Depth, linear comparison, and GreaterEqual behavior.

Changed files: shadow_collector.rs, postprocessing/ao_depth.rs, motion_vectors.rs, and their three source shaders.
Static binding inspection and git diff --check passed. Most added rendering files remain untracked, outside Git diff coverage.
Root owns compilation and hardware validation. Probe execution and diagnostic scan changes remain deferred.

### Final display orientation proof

The hardware shadow frame passed all three surfaces and deferred GPU validation, but its display was vertically inverted.
Root retains that frame in layer2-browser/native-current.json, label raw-depth-hardware.
I made no product change for this investigation.

The original final-display raw cache is absent from the known worktrees.
I recovered the required shader and native slices directly from the installed build.
Fresh evidence is in .cache/final-display-proof; verify.py repeats the extraction and saves verification.json.
UnityPlayer.dll SHA256 is 553c1e7560e1b42bd4638c53d0168dcd1b4b9eb5b9df324b0a44f14b3f18bc86.

Installed Resources/unity_builtin_extra:66, Hidden/BlitCopy, empty-keyword vertex blob2:
- Position is transformed by object and view-projection matrices.
- TEXCOORD output is mad(inputUV, CB0[2].xy, CB0[2].zw).
- It has no texel-sign branch or shader-level UV negation.
- Fragment blob6 samples that UV and multiplies by _Color.

The native final display transport supplies the missing orientation:
1. Blit setup at0x2e6ebb calls the orthographic matrix builder0x47b160 with (0,1,0,1,-1,100).
   Its X/Y clip mapping is 2*position-1. The view matrix is identity atRVA0x1a6a4e0.
2. Binding the null destination selects the backbuffer. SetActive writes projectionFlip=false at0x3de410..0x3de415.
3. ImageFilters::Blit calls the source orientation predicate0x301c10 at0x2eadaf.
   Its ordinary non-XR result tests source texelY at+0x78 against zero, then returns that result at0x301e0c.
4. Generated source targets pass through GetTempBuffer. Its reuse path calls CorrectVerticalTexelSize(true) at0x3b77b5.
   Fresh texture storage writes positive 1/height at0x3d8344.
   CorrectVerticalTexelSize0x3d7680 forces positive texelY for true on the selected D3D11 coordinate convention.
   These targets therefore select false from the ordinary orientation predicate.
5. Blit passes that flag to the quad writer0x3d83a0 at0x2eaf16.
   It supplies UV rectangle(0,0,1,1). A true flag swaps the two vertical UV endpoints at0x3d83dd..0x3d83e7.
   With the selected false flag, local position y0 receives v0 and y1 receives v1.
6. Standard D3D11 backbuffer viewport mapping places clip y+1 at screen top.
   Thus display top samples source v1; display bottom samples source v0.
   The retained final-display appendix proves the selected texture scale1/offset0 and color1.

The required equivalent for the current top-origin fragment position is UV=(screenU,1-screenV).
Apply it only in display_copy. Internal copy and copy_kill_nan keep their current storage coordinates.
Do not change the source camera, sky, culling, or projection matrices.
Negative TexelY handling exists in the native quad setup; it is not a branch in Hidden/BlitCopy.
Grab textures follow their separate negative-sign contract and do not determine this display mapping.

Verification extracts blobs2/6 again, records hashes and exact native ranges, and checks the selected mapping at three heights.
This proves installed control flow for the selected source path. It is not a native frame capture.
Root owns the shader edit and the following hardware orientation check.

### TASK3 probe graph layer

Root accepted the upright shadow layer before this change.
The new layer changes frame_graph.rs, probes.rs, and source_lighting.rs only.
No effects, motion, glass dispatch, target allocation policy, or Start gate changed.

ProbeFrame retains one ProbeFilter, one ProbePublication, transport tokens, and validation generations.
The filter creates its fixed bindings once from SurfaceResources.probe.
Main resize preserves that resource and all capture/filter/publication allocations.
ProbeFilter.failure reports permanent shader/pipeline errors by name while permitting pending shader imports.

The existing nine source surface cameras remain extracted.
Only the face selected by SourceViewFrame.probe_job can render among views3..8.
It waits for the working main display, current source queues, camera bindings, and filter pipelines.
Its source targets are the existing shared64x64 HDR1 scratch and its existing private depth/collector/atlas.
The capture order is color clear, camera depth, four-cascade atlas, collector, opaque, procedural sky, and ordered transparency.
Capture sky uses its established face matrix, source_sky_gpu_projection, current probe position, and far25.
SkyPass selects its capture variant and suppresses the sun disk. The probe mask excludes layer16 extras.
After successful capture sky and surface dispatch, finish_surface copies scratch HDR1 to the matching base-cube face at mip0.
Glass remains excluded until its later layer; final probe acceptance must repeat with glass.

Filter work executes after RenderGraphSystems::Render and before Submit.
Job7 generates the first Gaussian mip. Job8 generates the remaining levels and remaps roughness.
The existing source filter implementation is unchanged.

Submission ordering is explicit:
1. Face or filter commands record the exact current probe token through ProbePublication.encoded.
2. RenderGraphSystems::Finish runs after Submit and acknowledges that recorded token.
3. Face jobs and job7 then advance the main-world ProbeSchedule.
4. Job8 retains its token and registers a SourceExecutionGate filter validation generation.
5. After completion and the later device-error poll, filter_probe approves the capture and encodes publication.
6. Finish registers a separate generation for that publication copy.
7. Only after publication completion and device validation does Finish mark it published and acknowledge job8.

The final acknowledgement is deliberately later than the earliest permitted submission acknowledgement.
It satisfies the exact token contract and prevents a new cycle from starting before publication validation.
The previous published cube remains sampled throughout face and filter jobs.
Publication writes all levels into the existing published allocation after capture validation.
Main SourceViewLighting bindings remain unchanged. Capture bindings retain the source black local-probe override.

The on-demand source_render_audit now includes graph.probe:
- token and job1..8; job0 means no selected work.
- face_mask and mip_mask for the current capture.
- pipelines_ready, filter_validation, and publication_validation.
- submitted_token, published_token, published, and generation.

Expected first cycle: face masks1,3,7,15,31,63; mip mask1 after face6,3 after job7,127 after job8 submission.
The two validation waits retain job8/token8. Successful publication gives generation1 and published_token8.
Publication resets current masks. Later cycles keep published=true and advance tokens through16,24,and onward.
Main graph.mask retains7. Existing views3..8 audit entries now record each completed capture face.
SourceLighting reads the fixed audit fields to set probe_pipelines_ready and first_probe_ready.
The readiness bridge reads graph progress before locking lighting status, avoiding reversed mutex order.

Source startup enables this probe immediately. The current single-probe garage layer does not require a distance transition.
The5-second distance/cap policy, scene restart invalidation, and later glass recapture remain lifecycle acceptance items.
The prepare_source_draws scan and larger diagnostic scans were not changed in this layer.

Checks: scoped rustfmt passed; scoped git diff --check passed; camera sort inspected as order then target.
Bevy RenderGraph set order and RenderContext deferred command flushing were inspected in cached0.19.1 source.
No Cargo/build/browser command ran in this agent. Root owns compilation and hardware gates.


### TASK4 motion producer handoff (2026-10-12)

The graph now installs MotionScene extraction, persistent object histories, and motion pipeline preparation.
Both motion shaders join startup asset loading. Camera-depth routing already supports the motion draw command.
The new Motion surface stage runs after the collector and before opaque color, on main view1 only.
Camera and object systems remain separate because object draws borrow the render World.

MotionFrame owns one persistent SourceMotionPass and retains the main target Arc.
The camera pass reads the existing D24S8 DepthOnly view through the established unfilterable Float binding.
It writes RG16F motion and copied D24S8 depth, using the current main token and snapshot.
Rigid object draws then use that copied depth and the same token.
No source motion shader, matrix, depth state, or target allocation policy changed.

RenderGraph Finish commits camera and object histories only when:
- The complete main stack presented the current frame and main token.
- Camera motion encoded that frame and token.
- Object motion encoded that token, and the object snapshot matches the frame.
- The token exceeds the last motion submission.

The first matching submission starts deferred GPU validation.
Resize recreates copied depth and bindings while SourceMotionPass retains camera history and completed token.
Resize starts a new validation generation after its first complete motion submission.
Raw target XYZ origin shifts rebase camera history before any resize.
Existing object preparation rebases model histories using the same raw target XYZ shift.

The fixed graph.motion audit contains pipelines_ready, objects_ready, frame, camera_token, object_token,
submitted_token, registered_objects, draws[epoch,attempts,successes], extent, validation, validated,
resize_count, and rebase_count. Audit collection does not scan objects or allocate JSON each frame.
Expected hardware gate: camera_token == object_token == submitted_token == graph.token;
motion.frame == graph.frame; both readiness fields and validated are true; attempts equal successes.
A static scene may have zero eligible rigid draws. Camera motion must still encode and submit.
A resize must update motion.extent while retaining probe capture extent and working mask7.

prepare_source_draws now returns when PreparedMaterial assets did not change.
Its draw IDs are fixed at installation. New or replaced prepared assets mark that resource changed.
No larger scan optimization was included.

Checks performed: scoped rustfmt and git diff --check passed.
No build or browser check ran in this agent. Root owns the compiled and hardware gate.
No postprocessing, AO, glass, restart behavior, or Start readiness gate changed.

### TASK5 bounded integration plan (read-only, 2026-10-12)

Prerequisite: root accepts the moving camera/rigid GPU fixture and checkpoints TASK4.
No effect product code or fixture file changed during this planning pass.

#### 1. Close the final effect destination contract

Current SourcePostChain always owns a sample1 output. UberPass uses default sample count1.
This differs from the proved event18 destination in main-final-target-appendix.md.
The final Uber draw must write the existing shared original HDR8 attachment for all three camera layers.
The camera-stack-grouping appendix separately requires the later callbacks and their HDR1 scratch boundaries.
Do not wire the present sample1 output directly into final display or substitute it for the HDR8 effect destination.

Bounded API change in postprocessing/chain.rs and uber.rs:
- Remove SourcePostChain's owned final output image and output() accessor.
- Give SourcePostChain::encode the caller's HDR8 attachment and HDR1 resolve view.
- Configure UberPass for exact8, and encode its final draw with that destination and resolve.
- Keep its source inputs separate: initial copy, TAA output, blur output, bloom, exposure, LUT, and noise.
- Retain source half stores. Keep final display orientation only in display_copy.

Graph-owned callback scratch has one full-size HDR1 allocation shared by the sequential far/main callbacks.
This is the required native scratch lifetime reused across cameras; neither callback has temporal scratch history.
Create one stack-resolved-to-scratch FrameCopy (samples1) and one scratch-to-stack FrameCopy (samples8).
The existing FrameCopy API already supports both. Root owns that file; no implementation change is required.
No callback scratch is needed for foreground. No extra effect output image remains.

Exact surface sequence:
1. Far surface resolves its current scene. Dither-only chain: initial copy, Uber into shared HDR8 and resolve.
2. Far PP callback copies that resolve into HDR1 scratch. Native copyback copies scratch into HDR8 and resolves.
3. Main loads the shared color, renders opaque, applies AO, then renders transparent and resolves current color.
4. Main chain: NaN initial copy, TAA, blur, histogram/exposure, bloom, LUT, Uber into shared HDR8 and resolve.
5. Main PP callback copies resolve into HDR1 scratch. Enabled zero-weight heat callback copies scratch into HDR8 and resolves.
6. Foreground loads shared color and clears its own depth, then draws and runs its own NaN-copy/dither-only chain into HDR8.
7. Foreground's callback uses the existing display_copy from the final resolve to the sRGB display view.

The heat weight remains the validated source startup value0. Its callback remains an explicit counted copy.
Nonzero heat behavior is outside this layer's selected profile; no guessed heat shader will be added.

#### 2. Load the existing effect inputs

LightingShaders currently loads copy, taa, histogram, and exposure already.
Add motion_blur, bloom, lut, uber, ao_depth, ao_render, ao_upsample, and ao_composite (eight handles).
The exact files are web/assets/shaders/source_<name>.wgsl. Include them in startup shader load checks.
Construct the existing PostShaders from these handles only at preparation, not during each draw.

LightingRenderAssets already owns record, texture SourceId map, and loaded source image handles.
Resolve record.postprocessing.blue_noise in its recorded order to 64 GpuImage texture views.
Resolve record.postprocessing.bloom.dirt to its GpuImage view and physical dimensions.
All these images already belong to record.textures and the startup loading path. Do not reload or duplicate images.
Missing GPU images keep preparation pending. Invalid dimensions or failed loads set SourceLightingStatus::Failed.
PostTextures borrows these views while constructors create stable bind groups.

#### 3. Allocate persistent AO and camera chains

Add one graph resource, prepared after SurfaceResources and MotionObjects preparation.
It owns SourceAoChain for main, [SourcePostChain;3] for far/main/foreground, callback scratch, and two callback copies.
Use record.cameras[index+1].postprocessing.scope and stop_nan for each layer.
Only main uses CameraEffectScope::MainStack. Far and foreground use DitherOnly with independent dither state.
No post chain or AO is attached to sky or any probe face.

Create AO first with the existing main raw D24 sampled view and physical extent.
SourceAoChain::raw_depth returns its source-required R32 copy for TAA and blur.
This copy is an AO/effect resource; motion still consumes original D24 through its established Float binding.
Main SourcePostChain receives AO raw depth and main RG16F motion.
Dither-only chains do not bind or consume depth or motion despite the current common constructor parameters.

Detect physical resize through the stack target Arc identity. Move old chains into constructors with previous=Some(old).
AO recreates only extent-dependent storage and bindings. It retains pipeline IDs.
Post resize preserves completed tokens, dither, exposure images, LUT image/bake state, and TAA history resampling.
Rebind the two callback copies and resize their one scratch allocation.
Do not reconstruct during a normal frame, update, probe cycle, or publication.

#### 4. Place dispatch and input snapshots

AO belongs in SourceSurfaceStage::AfterOpaque, main view1 only, before transparency.
It must see current-frame camera depth and successful opaque drawing.
Pass main near/far, implicit projection x_axis.x, and SourceSceneLighting fog_color_density.w.
Its composite multiplies original HDR8 samples and resolves into the shared single-sample color.
The existing AoCompositePass already declares sample8 and preserves alpha through its source blend factors.

Add an explicit post stage after Transparent and before finish_surface.
It runs the selected camera chain and required callback copies before that camera earns its mask bit.
Main post requires camera and object motion encoded with the current main token, plus this frame's AO marker.
Far/main/foreground all use frame.main_token, each in its own chain. Probe tokens never enter these chains.
All effect and callback pipelines must be ready before any stack effect work starts.
A ready encode returning false is not completion; it must not permit display acknowledgement or history advance.

Add one extracted, fixed-size effect input snapshot, stamped alongside SourceViewFrame.
PostFrame receives exact frame.main_token, frame.jitter.uv, selected view near/far, shutter, and scaled delta_seconds.
The runtime has no current shutter state or post input resource. Add only that scalar state and frame snapshot.
Installed postprocesscontrol.Update lines49692..49703 specifies:
 target = lerp(minShutter,maxShutter,inverseLerp(minSpeed,maxSpeed,rootRigidbody.velocity.length*3.6));
 target = minShutter when that root has no Rigidbody;
 shutter = lerp(previousShutter,target,clamp01(shutterLerp*Time.deltaTime)).
The serialized stationary initial value is5, range5..100, speed20..80, rate8 (motion-blur-appendix.md).
Use frame delta, not fixed-step count, for the controller and exposure. Paused source Time.deltaTime is0.
Before wiring live speed, verify which target body corresponds to the selected source head root when walking/seated.
Available target APIs are player.body_handle(), player.seat(), physics.chassis_handle(), and body.linvel().
Do not substitute vehicle speed for an unverified source hierarchy. Stationary loading input is the proved shutter5.
Keep startup/pause input independent of the closed Start gate so loading frames can validate the effect chain.

#### 5. Commit only the complete submitted stack

Each chain already rejects zero/repeated completed tokens and stages history/dither until completed(token).
Record per-camera encoded frame/token and callback mask after successful dispatch.
finish_surface requires this camera's post/callback markers before setting its mask bit.
RenderGraph Finish commits all three chains only after the complete displayed mask7 frame matches those markers.
This advances TAA ping-pong, exposure, blur reset, LUT bake state, and each independent dither sequence together.
Motion camera/model histories keep their existing matching full-stack submission rule.
AO has no temporal history to commit. Its frame marker only prevents stale consumer input.
Start a new deferred SourceExecutionGate validation after first complete effects submission and after resize.
Propagate constructor, pipeline, encode, and token failures into source status.

Lifecycle APIs already exist:
- SourcePostChain::reset_history resets TAA/exposure/blur but preserves dither and transport tokens.
- SourcePostChain::rebase preserves image histories; camera/model motion rebase remains the existing TASK4 path.
- A recreated source scene constructs chains with previous=None, restarting per-layer dither.
- SourceViewClock::restart resets source sequences while keeping transport tokens monotonic.
Do not confuse ordinary camera cleanup with scene recreation. Wire full restart/recovery acceptance in the lifecycle layer.
The existing readiness struct has no separate glass/lifecycle bit.
Do not set its final postprocessing_pipelines_ready gate true merely because this bounded effect audit passes.
Keep Start closed until root connects the final glass/lifecycle gates.

#### 6. Fixed audit and resource accounting

Add graph.effects with fixed fields only:
- frame, token, ao_frame, ao_token, post_mask, callback_mask, and submitted_token.
- pipelines_ready, inputs_ready, validation, and validated.
- encoded_tokens[3], completed_tokens[3], extent[2], chain_bytes[3], ao_bytes, callback_bytes, total_bytes.
- startup_counts[3], resize_counts[3], AO startup/resize counts, callback allocation count.
- input shutter and delta_seconds for exact frame evidence.
Add small read-only chain audit accessors if needed for TAA/exposure/dither state; do not expose writable history controls.
All render updates copy fixed scalars. Serialization stays inside explicit source_render_audit calls.

After removing owned final outputs, each dither-only chain owns one HDR1 initial image: area*8 bytes.
Main owns initial plus TAA output, two TAA histories, and its existing blur/histogram/exposure/bloom/LUT storage.
Main bytes = area*16 + TAA(area*16) + blur.layout.logical_bytes + histogram.layout.logical_bytes
             + exposure8 + bloom.layout.logical_bytes + LUT287496 + curve2048.
Shared callback scratch adds area*8 once. Existing stack colors and RG16F motion are not counted again.
AO sums depth, render, and upsample logical_bytes. Its full rawR32+linearR16 terms equal area*6.
AO levels retain existing ceil sizing, four slice groups, R8 visibility, and float32 halo accounting.
Existing motion copiedD24 adds area*4 separately; object uniforms add registered_objects*144.
Publish total connected bytes without claiming backend heap size.

Acceptance after implementation: all three encoded/completed tokens equal graph.token; post mask7; all expected callbacks counted;
AO marker equals main frame; deferred validation succeeds; shadow/probe/motion gates remain intact.
Inspect the assembled image after effects. Repeat physical resize and confirm target/counter stability after resizing stops.
Root owns builds, browser checks, source review, and the layer checkpoint.

#### TASK5 input proof follow-up

Fresh installed level1 controller80667 is204 bytes; its h pointer is local transform37616.
The unchanged MonoBehaviour header consumes32 bytes, followed by LayerMask4, then that Transform PPtr at offset36.
Fresh UnityPy transform reads follow37616→39820→22249→39811→39851→39843.
The root GameObject20758 is Player and contains Rigidbody. Evidence: .cache/layer5-input-proof.json, including raw controller SHA256.
Thus walking shutter reads the player's body speed, not unrelated vehicle speed.
Installed sitting code lines19107..19140 moves Tb to sitPos, parents Tb under sitPos, then destroys its own Rigidbody.
This selects the vehicle root Rigidbody while seated; use target chassis velocity for that mode.
Installed mainscript.Update lines31231..31264 sets timeScale1 when running,0 in the ordinary menu/loading pause,2 while sleeping.
The currently supported target runtime exposes started/paused. Use zero effect delta before Start or while paused; otherwise frame delta.
No motion or effect product code changed for this proof.

#### TASK5 implementation handoff

TASK4 is committed as538810e after the moving hardware gate passed.
TASK5 adds source_effects.rs and six integration edits. The review patch is .cache/layer5-browser/task5-review.diff.
The all-target WASM check passed. Release and bindgen are running; log .cache/layer5-browser/build.log.
No effect equation or effect shader changed. Uber now writes the original shared HDR8 attachment with its HDR1 resolve.
The obsolete per-chain final HDR1 output is removed. One persistent HDR1 scratch serves both sequential callback pairs.
AO runs after main opaque. Each camera post chain runs after its transparency and before its completion marker.
Far and foreground retain separate dither chains. Main connects TAA, blur, exposure, bloom, grading, and dither.
The enabled zero-weight heat callback remains an explicit second main copy.
The complete displayed stack commits all three pending effect tokens together. Incomplete frames cannot advance effect histories.
First effects submission and each physical resize start separate deferred GPU validation.
Resize reuses pipeline IDs and preserves the existing source history rules through previous-chain constructors.
No Start gate changed. Probe faces do not run AO or effects.

The on-demand graph.effects audit exposes frame, token, AO markers, post mask, and callback mask.
A successful submitted frame requires post_mask7, callback_mask31, and all encoded/completed tokens equal graph.token.
AO frame/token must match the graph. Submitted token must match graph.token. Pipelines and inputs must be ready.
Effects validated must become true after the asynchronous GPU completion check.
The audit also reports physical extent, logical bytes, constructor counts, shutter, and scaled delta.
Loading shutter is5 and delta0. These are the proved source paused input values.
Resize must increment each resize counter once, preserve startup counters1, and settle allocation counters afterward.
Prior sky, surface mask7, shadow counts, motion tokens, and probe publication must remain valid.
Native acceptance must inspect the image, then resize and verify these fields again.
The logical byte count excludes stack, motion, probe, and backend driver storage.
Native matched source pixels remain unavailable. This gate cannot establish1:1 visual parity.

TASK5 accepted: root inspected native-effects.jpg and retained full normal, resize, and steady audits in .cache/layer5-browser.
Normal frame647/token643 and resize frame1703/token1699 met all current-token effect and existing graph gates.
The960x540 target held stable across1114 later frames without constructor count changes.
Independent effects-layer-review.md passed both review stages. No native matched-pixel parity is claimed.

#### TASK6 ordered glass integration

TASK5 checkpoint6148e66 retains the accepted camera effects layer.
TASK6 scope: transparent.rs, surface_graph.rs, stained_glass.rs, materials.rs, and frame_graph.rs.
The review patch is .cache/layer6-browser/task6-review.diff. Build log is .cache/layer6-browser/build.log.
Glass capacity includes separate Grab and BASE records. Both records use the established native pass-level comparator.
Grab keeps the first-pass bit and grab marker. BASE uses pass1. No per-object adjacency override is added.
The pass cache consumes every sorted record, including ordinary surface passes.
An executed Grab ends earlier draws, resolves current HDR8 when necessary, then copies into the existing HDR1 grab texture.
Probe views copy their current HDR1 directly. The following BASE uses DrawSourceGlass and the retained source shader.
The draw samples a separate capture image, never its current color attachment.
Camera cleanup clears image validity. View completion requires the current glass frame marker.
Capture storage remains owned per view and is replaced only with its existing target resize lifecycle.

Fixed per-view glass audit: frame/token, captures, skipped_grabs, material_draws[47,48], total_captures, extent, and logical_bytes.
Positive material draws are counted from nonempty prepared batches. Existing issued-draw checks still reject skipped commands.
Root mapped source47 to home.gltf material11/node101, and source48 to material8/node79.
These doors must both be covered by native acceptance; the default garage view may omit one.
All shadow/effect/motion gates and repeated probe publication must remain valid with glass connected.
Resize must preserve full physical grab extent and fixed64x64 probe grabs.

Fresh installed native cache slices are .cache/layer6-proof/native-3393e0.txt and native-339900.txt.
The normalized flag branch is preserved as evidence without inventing names for its bits.
Target cache identity uses full runtime material identity plus prepared pipeline/pass identity.
The latter includes actual target raster state. It is not claimed as an observed native ShaderLab pointer.
Selected glass has no instancing variant. The selected target has no lightmaps, custom property blocks, or static batches.
Other normalized native runtime values and exact equal-distance tie identities remain unobserved.
Unequal-distance objects retain complete far-to-near pass groups, and intervening BASE invalidates the next Grab.
No native tie-state equivalence or matched glass pixels is claimed by this target integration.

#### TASK7 bounded lifecycle plan

Read lifecycle-acceptance-audit.md and steady-frame-audit.md before implementation.
Retain all accepted layer gates. Do not change source motion, effect, shadow, or glass equations.

Use one main-world scene generation. Increment it only for the proved scene restart path.
Carry that generation in SourceViewFrame. Reject old graph readiness until its generation matches.
Restart SourceViewClock before the new snapshot. Keep transport tokens monotonic and clear old acknowledgements.
Check all nine cameras and sky presence, activity, and physical extent before consuming any acknowledgement.
A suspended view must publish no renderable snapshot and consume no jitter/probe step.

At render extraction of a new scene generation, reset camera-local motion and effect state once.
Recreate effect chains with previous=None so seeded dither and temporal histories match a cold scene.
Reset retained object histories from the current models; reuse their uniform storage where safe.
Keep static shader pipelines, source assets, fixed probe storage, and ordinary stack targets where safe.
Reset probe publication state, pending tokens, and validation references. Clear the old published cube before new scene sampling.
Old GPU callbacks may finish, but their generations cannot validate the new scene's explicit submission.
Clear glass validity at the existing camera boundary and on scene reset.
A resize preserves temporal source resize rules and publication, but resets main target validation.
A world rebase preserves TAA/exposure/dither and uses the accepted raw target XYZ motion transport.

Wire final readiness to the current scene's complete displayed stack, current motion/effect validation, and validated probe publication.
Use explicit saved validation generations. Global latest-request equality must not make readiness flap during later probe cycles.
Remove the CPU two-update delay as a substitute for render completion.
Restart recloses browser readiness and pauses gameplay until the new renderer is valid.
The browser must show loading while rebuilding, then restore a user action to resume mouse capture.
Faulted states remain terminal until a healthy page load. No shipping fault injection controls.

Implement the retained source probe activation boundary: every5 scaled seconds, strict squared distance<2500, selected cap10.
Fresh managed proof: Assembly-CSharp.cs29244..29252 and30545..30564; reflectionsscript.Start51200..51206 enables from source settings.
Selected profile contains one probe, so the cap cannot exclude it after the distance check.
Pause disables elapsed source time. Disabled capture keeps the last published cube valid.
Prove how a partially submitted probe job resumes before changing its acknowledgement rules.

Cache only diagnostic asset scans: prepared source material counts and source GPU image counts.
Refresh on relevant Bevy resource changes or removal. Keep frame tokens and live counts current.
Add only fixed lifecycle/history audit fields needed by native and extracted contract checks.
Expose first-scene completion state if normal audit timing could miss it.

Verification: shipping-state suspension/restart contract, existing rebase fixture, actual native restart and physical resize.
Root handles isolated missing asset, pipeline layout failure, and WebGL context loss through external server overlays.
Native OS minimize is unavailable; report that limit and test missing/inactive/zero-size contract plus browser restore.
Update docs/roadmap.md, docs/verification.md, and the plan header to the approved branch and retained evidence.
Preserve the ledger and rulings in a tracked local completion record. Do not delete ignored evidence or the worktree.
Remaining limits: matched native pixels, exact transparent runtime ties, missing authored tangents, and driver heap bytes.

#### TASK7 review handoff

Diff: .cache/layer7-browser/task7-review.diff. Build: .cache/layer7-browser/build.log.
Scene generation changes only on the proved restart caller. Commands reset clock and close shared readiness before snapshot.
All nine surface cameras and sky must exist, remain active, and have positive physical extents before clock consumption.
Suspension clears renderable snapshots and probe job. It preserves unconsumed acknowledgements and temporal state.
Main readiness requires matching scene, presented mask7, current motion/effect tokens, their explicit GPU validations, and published probe.
Continuous probe validation requests no longer invalidate a previously validated main submission.
Main resize invalidates its validation generation; effect and motion targets retain their prior dedicated resize rules.
Restart resets retained motion models, cold camera registration, all three dither sequences, TAA, exposure, blur, and LUT validity.
Camera cold registration uses existing raster VP, then normal commits use nonjittered VP. No zero pixel-velocity claim follows.
Probe restart clears the retained published texture's seven HDR mip levels and clears all publication/token/validation state.
Static pipeline IDs, compatible targets, bindings, and buffers remain allocated.
Browser loading rejects late mouse capture, closes controls, and pauses gameplay. Fresh readiness exposes Continue after restart.
Source probe activation checks strict squared distance below2500 every5 scaled seconds. One selected probe cannot exceed cap10.
Disabled capture holds its job and prior cube. In-flight validation may finish; held acknowledgements are consumed once on activation.
Diagnostic asset counts refresh only when relevant Bevy resource ticks change; frame masks and tokens remain live.
Fixed audits include scene, first token/jitter, first motion registration/draws, first and live effect histories, and scan refresh count.
Effect history rows are noise index, TAA next index/initialized, exposure output index/initialized.
Cold and recreated initial rows are zeros. First completed main row is[1,1,1,0,1]; far/foreground are[1,0,0,0,0].
Build check passed before final audit fields. Seven menu tests passed. Focused clock/suspension/restart fixture is in preparation.
Native healthy/restart/resize, distance activation, and three isolated faults remain open. No new parity claim.

Task7 command gates passed:240 ordinary native,42 installed-source,24 web,3 clock, and1 large-delta transition checks.
The fresh exporter suite passed171 with one missing-library skip. Its exact skipped UI test passed with path-only fixture substitution.
Fresh selected export matches all1380 shipping files after1388 dependency/metadata checks and the Rust runtime validator.
The independent lifecycle reviewer passed the three-file effective-delta correction.
Native first readiness passed after35111ms, with all current-scene graph and cold-history audit fields.
The in-app browser refused pointer capture. Root uses existing action injection after true readiness for lifecycle checks.
This checks renderer restart and pause behavior; it does not establish genuine mouse-control acceptance.

#### TASK7 assembled physics fixture

The native restart transition closed Start and reset scene2 validation, publication, and effect histories.
The accepted new scene restores the initial jitter/history rows and cold camera registration.
Its first main token22425 exceeds the prior token22423. First rigid-object draws remain zero.
Compatible allocation counts remain unchanged.

The ignored .cache/layer7-physics-fixture changes only an isolated app.rs copy.
Its manifest verifies98 unchanged Rust source files and27 shipping shaders, including the complete renderer.
After stock Ready and the existing Start action, each selected stage replaces the actual player body through public recovery APIs.
The path removes the previous body, constructs PlayerState, configures seats, binds interaction, and refreshes physics queries.
Stages hold the original position, move80m along absoluteX, return, move2304m along absoluteX, and return.
No fixture writes origin, rebase counters, rendering clocks, validation, started, paused, or readiness.
The stock GarageRuntime::step calls WorldStream::rebase when the real focus exceeds2048m.
The exported fixture audit records scaled seconds, positions, fixed steps, actual origins, and runtime rebase counters.
Native acceptance must also inspect the shipping graph's probe state and motion rebase count.
Fixture check passed37.47s. Release and native fixture acceptance remain pending.

#### TASK7 context-loss correction

Actual WEBGL_lose_context testing stopped gameplay but exposed continued renderer tokens after JS failure.
The old main.js fault handler did not notify shared SourceLightingStatus.
The source_render_failure export now latches Failed only if no earlier fault exists.
The existing shared diagnostic handle supplies the renderer status; no separate fault state is allocated.
ExtractSchedule clears Bevy's public RenderScheduleOrder labels on terminal failure.
Bevy run_render_schedule then skips First and Render while its normal time delivery remains active.
This stops deferred validation, GPU work, probe publication, motion commits, and effect history commits.
Source snapshot already returns before consuming acknowledgements when shared status is Failed.
Native RenderState is private. Native device failures retain their existing StopRendering handler.
That handler now preserves a prior browser fault message.
JS failure latches once; Rust gameFailure only displays the failure and cannot echo into the Rust bridge.
The exact shipping JS handler fixture passed two critical cases. Full web suite24 passed again.
Scoped review diff: .cache/layer7-context-fix/context-fix-review.diff.

#### Context review: deferred extraction correction

The review found that the earlier empty render schedule still allowed repeated extraction batches.
Bevy applies ExtractSchedule deferred commands only inside Render, so those batches would accumulate after failure.
The final check lives in the existing surface_graph extraction wrapper, before calling Bevy extraction.
On terminal failure it clears render labels and returns without extraction.
After a healthy extraction it checks again for device or camera faults, before normal phase setup.
It also checks after the one-time phase setup. No new extraction batch is queued on later failed updates.
Bevy's outer RenderRecovery still sends time independently of extraction and the empty render schedule list.
The prior stop_failed_rendering extraction system is removed.
The pending batch from a fault raised inside extraction remains bounded; no later producer runs.
Scoped correction changes source_lighting.rs and surface_graph.rs only.

The scoped context correction passed specification and quality re-review.
Final stock build passed check4.92s/release1m51s/bindgen; WASM56773593 bytes, SHA256fbce4a3fe1e2a807676941446dd6d3686c15dc8341ffb25aa5b208c0e1d68718.
The physics fixture was regenerated as .cache/layer7-physics-fixture-v2 from those exact sources.
Its check9.92s/release2m10s/bindgen passed. Hash verification confirms98 unchanged Rust files and27 unchanged shaders.
The fixture owns only an app.rs overlay. Stock package hashes remain unchanged.
Native final context loss and physics fixture acceptance remain open.

#### Final corrected context acceptance

The native hardware package reached Ready before the test. Existing Start action produced active physics.
Actual WebGL context loss then set shared Failed: The graphics context was lost.
All controls closed and physics paused at922 steps.
final-context-stopped-pass.json and final-context-frozen-pass.json have exactly equal complete renderer audits.
The implementation owner independently asserted that equality.
Graph frame3677/token3673, effect and motion histories, probe publication, and validated generation remain fixed.
This accepts the terminal extraction/render stop without a synthetic failure control or renderer-state override.
The only remaining runtime gate is the real-body distance/rebase fixture. Whole-branch review and Task7 checkpoint remain pending.


#### Final motion streaming correction

The real-body +80 m fixture exposed a P1 before its distance check: map deletion reduced insertion headroom.
Whole-branch review found this single issue. Its capacity was not the fixed scene bound.
The correction retains exactly the existing snapshot bound in GPU slots and reuses inactive slots.
It rebuilds the entity lookup only when membership changes. New entities get cold histories.
Retained entities keep submitted history; source rebase and motion equations are unchanged.
The audit adds allocated and reused slot counts. Logical uniform bytes include every allocated slot.
Three exact-source tests passed, including 4,096 membership changes, cold reuse, and retained completion/rebase history.
WASM all-target check passed10.24s; formatting/release1m34s/bindgen passed.
Final source/package/log receipt: .cache/final-motion-fix/final-source-receipt.json.
Stock WASM56772820 bytes, SHA256365a657aad3ca3586b8d40c2b14190df5566ad7191b9ebc1b8461fad12c11d60.
The isolated physics v3 overlay preserves98 shipping Rust sources and27 shaders; its package build is running.
Scoped correction review and real-body native stages remain open. Task7 is uncommitted.


Physics fixture v3 check10.55s/release1m59s/bindgen passed. Fresh source equality passed98 Rust sources and27 shaders.
Its separate package hashes are .cache/layer7-physics-fixture-v3/package-hashes.json.
All135 stock runtime file hashes and both stock package hashes remained unchanged after the overlay build.
Parent owns fresh stock readiness and physics stages0 through4. No product code changed after the stock receipt.


### Final Task 7 acceptance

Parent accepted the final corrected package and all five native physics stages.
Fresh stock Ready was observed by72,746ms within120s. Cold camera/effect histories and validated probe remained correct.
The app-only physics v3 overlay retained98 identical stock Rust files and27 shaders.
Real bodies crossed the50m probe limit through actual5-second scaled-time checks, then returned.
The normal runtime produced forward origin[2304,0,-128] and return origin[0,0,-128].
Area and stream origins agreed. Runtime and motion rebase counts advanced0 to1 to2.
All stages retained780 motion GPU slots; final reuse count910. No target allocation count increased.
All stages retained Ready, mask7, sky, presentation, GPU validation, motion tokens, and effect masks7/31.
The final returned probe was enabled and published. All three live dither indices matched(token-firstToken+1)%64.
Far bodies fell outside existing ground; distance includes actual vertical displacement. The fixture changed no renderer state or origin.
Root console error and warning arrays were empty.

Whole-branch review found one P1; its bounded motion correction passed scoped review with no new finding.
Three critical exact-source tests passed. The final full native suite rerun passed240, failed0, default ignored42.
Installed42, exporter171 plus separate skipped UI check, web24, clock3, large-delta1, and byte-identical1380-file regeneration remain accepted.
Final source/package receipt: .cache/final-motion-fix/final-source-receipt.json.
Independent final assertions: .cache/final-motion-fix/final-acceptance.json.
Native receipt: .cache/layer7-browser/physics3-native-receipt.json.
Final stock WASM56772820 bytes; SHA256365a657aad3ca3586b8d40c2b14190df5566ad7191b9ebc1b8461fad12c11d60.
Source and package hashes stayed unchanged through the final checks. No additional product mutation followed native acceptance.

All seven planned layers are accepted. This establishes the tested target behavior, not identical installed-build pixels.
Remaining limits: no matched native pixel frame, no continuous IAB pointer capture, no automated native OS minimize,
unknown native identities for transparent ties, nine primitives without authored tangents, and logical bytes excluding driver overhead.
The installed Single scene-reload trace resolved the earlier restart semantic uncertainty.
The branch and worktree are preserved. No push, merge, rebase, or evidence deletion is authorized or performed.
