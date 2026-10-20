# More worker tasks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce measured main-thread CPU work without changing deterministic results, save/load, or input response.

**Architecture:** Profile before selecting a task. Use one dedicated worker per selected calculation type. Keep simulation, admission, and publication on the main thread.

**Tech Stack:** Rust 1.99, wasm-bindgen 0.2.129, existing Rapier and glam versions, module Web Workers, transferable buffers, Node tests.

**Spec:** [Approved design](../specs/2026-10-19-more-worker-tasks-design.md), commit `e8f11ec`. The user approved the written spec on 2026-10-19.

## Global Constraints

- Use an isolated worktree.
- Keep the branch unmerged unless the user authorizes integration.
- Steam build `11140890` remains the behavior authority.
- The current save schema remains the format authority.
- Keep the terrain worker independent.
- Start with one outstanding job and one unread result per added worker.
- Use existing world job limits to bound total work.
- Do not create a general worker pool or dynamic worker count.
- Do not share mutable WASM memory.
- Do not silently run failed worker calculations on the main thread.
- Saving does not wait for a visual job.
- Disabled instrumentation must not read clocks or allocate per frame.
- Release profiling and matched release A/B/A browser runs are approved milestones.
- Do not run full checks unless separately requested.

Work only in `C:/Users/Corashina/.codex/worktrees/more-worker-tasks/the-long-drive-wasm`.
Use branch `codex/more-worker-tasks`.
The frozen source base is `738ce8b2b907491725c52b3a917c42f552ac9174`.
The primary checkout contains unrelated work. Do not copy or alter it.

## Review Focus

1. Delayed worker initialization must not stall input. Pin this in Tasks 1 and 4 or 6.
2. A paused frame must not count as input consumption. Pin this in Task 1.
3. A prepared load must not replace the active world's worker ownership. Pin this in Task 7.
4. Continuous camera movement must not starve grass publication. Pin this in Task 6.
5. A failed or discarded job must retain its reservation until physical release. Pin this in Tasks 4, 6, and 7.

## File structure and execution order

Create only the files required by a selected task.
Keep every completed task usable before starting the next.

| Task | Files and responsibility |
| --- | --- |
| 1 | `src/app/cpu_profile.rs`, `web/cpu-profile.js`: opt-in CPU and input measurements |
| 1 | `src/app.rs`, `src/app/save.rs`, `src/browser.rs`, `web/main.js`, `web/input.js`, `web/world-assets.js`: measurement call sites |
| 1 | `src/world/stream_profile.rs`, `src/presentation/world/runtime.rs`: retain existing stage measurements |
| 1 | `scripts/measure-content-index.mjs`, `scripts/serve-content-index-benchmark.mjs`, `scripts/summarize-streaming-profile.mjs`: receipts, hashes, and comparisons |
| 2 | `docs/performance/more-worker-tasks.md`: baseline evidence and task selection |
| 3 | `crates/road-mesh/`, `crates/road-worker/`, `src/world/road.rs`: shared road calculations and worker entry |
| 4 | `src/world/road_worker.rs`, `src/world/stream.rs`, `src/world/metrics.rs`, `src/world.rs`: road lifecycle and reservations |
| 4 | `web/road.js`, `web/road-worker.js`, `src/browser.rs`, `web/main.js`: road transport |
| 5 | `crates/grass-mesh/`, `crates/grass-worker/`: shared grass calculations and worker entry |
| 5 | `src/presentation/materials/grass.rs`, `src/presentation/materials/grass/settings.rs`: source material conversion |
| 5 | Remove `src/presentation/materials/grass/domain.rs` and `expansion.rs` after moving their code |
| 6 | `src/presentation/world/grass_worker.rs`, `grass_jobs.rs`, `src/presentation/world.rs`: grass lifecycle and reservations |
| 6 | `web/grass.js`, `web/grass-worker.js`, `src/browser.rs`, `web/main.js`: grass transport |
| 3–6 | `Cargo.toml`, `Cargo.lock`, `scripts/build.ps1`: selected packages only |
| 7 | `src/app/save.rs`, `src/presentation/save.rs`, `src/world/worker_tests.rs`: save/load ownership checks |
| 8 | `docs/performance/more-worker-tasks.md`: acceptance evidence and final decision |

Run Tasks 1 and 2 first.
If road wins, run Tasks 3 and 4, then Tasks 7 and 8.
If grass wins, run Tasks 5 and 6, then Tasks 7 and 8.
Only consider the second task after the first passes its acceptance run.
If neither task qualifies, finish with the profiling tools and negative findings.
Do not implement another task type without revisiting the design.

## Task 1: Add complete CPU and input receipts

**Files:** Use the Task 1 map above. Also modify `src/presentation/frame_graph.rs` at its submitted-token boundary.
**Tests:** `tests/web/cpu-profile.test.mjs`, `tests/web/content-index-benchmark.test.mjs`, `tests/web/input.test.mjs`.

**Interfaces:**

- `createCpuProfile(now, timeOrigin)` returns `enable(enabled)`, `begin(stage)`, `end(stage)`, `input(sequence, eventAt, trusted)`, `consume(sequence, step)`, `submitFrame(token)`, and `snapshot()`.
- `snapshot()` returns stage measurements, input latencies, worker records, retained bytes, and dropped-sample counts.
- Use fixed-capacity storage of 4096 input samples and 4096 job records.
- `src/app/cpu_profile.rs` exports `cpu_profile(enabled: bool)` and `cpu_profile_audit() -> String` through wasm-bindgen.
- Its `Timer::new(stage: Stage)` records startup, asset, save/load, physics, and render preparation spans.
- `measureContentIndex` adds `profileCpu: boolean = false` and stores `cpuProfile`.
- Extend scenarios with `startup-cold`, `startup-warm`, `interaction`, and `save-load`.
- `summarizeWorkerComparison(baseline, candidate, recheck)` is exported from `scripts/summarize-streaming-profile.mjs`.
- `createContentIndexBenchmarkServer` accepts a frozen `scriptsRoot` alongside `webRoot`, `assetRoot`, and `packageRoot`.
- Its identity includes served web scripts, runner scripts, and package files.
- Keep one receipt system. Remove obsolete receipt validation rather than adding a legacy reader.

- [ ] Add `disabled_profile_has_no_clock_reads` with a clock that throws whenever called after initialization.
- [ ] Add `input_requires_a_fixed_step`. Record event 1, pause, and submit a frame. Assert no consumption latency exists.
- [ ] Consume event 1 at step 2, then submit token 3. Assert both latency intervals exist exactly once.
- [ ] Pin disabled clock behavior with `assert.equal(clockCallsAfterReset, 0)` after begin, end, consume, and submit.
- [ ] Add `receipt_rejects_mixed_packages_and_invalid_samples`. Assert a worker hash mismatch blocks comparison.
- [ ] Assert hidden, unfocused, paused, non-driving, and missing-frame samples remain invalid where applicable.
- [ ] Add `clock_handshake_keeps_context_origins_separate`. Use different context origins and assert nonnegative delivery intervals.
- [ ] Run `node --test tests/web/cpu-profile.test.mjs tests/web/content-index-benchmark.test.mjs tests/web/input.test.mjs`.
- [ ] Confirm the new tests fail because the new interfaces do not exist.
- [ ] Implement the profile storage and Rust timer. Read clocks only when the new profile flag is enabled.
- [ ] Mark input events without changing input bits. Associate sequence IDs with the fixed step that actually consumes them.
- [ ] Carry the last consumed sequence to the submitted frame token. Clear pending sequences on blur, pause, and session replacement.
- [ ] Reuse existing streaming spans. Keep nested stage totals separate.
- [ ] Add asset decode, save prepare/commit, app physics, and render preparation spans at their real owners.
- [ ] Record browser render-main-thread traces alongside stage receipts to cover uninstrumented Bevy preparation.
- [ ] Extend package identity with hashes for all served package JavaScript and WASM files.
- [ ] Store explicit cache state, scenario, package identity, sample validity, input trust, and instrumentation mode.
- [ ] Guard the summarizer CLI entry with `import.meta.url` so tests can import comparison functions without running the CLI.
- [ ] Extend the summarizer with mean, p95, p99, maximum, and per-metric baseline repeat variation.
- [ ] Compute variation as `abs(baselineMetric - recheckMetric)`. Reject missing or mismatched metrics.
- [ ] Re-run the targeted web command. Require zero failures.
- [ ] Run `./scripts/check.ps1` for formatting and the workspace WASM compile check.
- [ ] Run `./scripts/check.ps1 -Test world_frontiers` as the targeted native regression check for Rust instrumentation.
- [ ] Commit as `Add CPU and input profiling receipts`.

## Task 2: Measure the baseline and select the first calculation

**Files:** Create `docs/performance/more-worker-tasks.md`.
**Consumes:** Task 1 receipts and summarizer.
**Produces:** Frozen baseline packages, saved receipts, and a ranked candidate decision.

- [ ] Verify branch, source base, clean product diff, available assets, and Docker.
- [ ] Record the exact asset root and hashes. Validate matching source manifest and runtime package before measurement.
- [ ] Run `./scripts/build.ps1` in this worktree. Stop on any build failure.
- [ ] Freeze the built package, web files, and measurement scripts under `.cache/benchmark/more-worker-tasks/baseline/`.
- [ ] Keep the asset root separate. Freeze its identity and reuse it without copying the full asset library.
- [ ] Record the instrumented source commit separately from the original source base.
- [ ] Use `createContentIndexBenchmarkServer` with frozen roots and an explicit receipt directory.
- [ ] Run one browser sample at a time. Coordinate an exclusive benchmark window before using the GPU.
- [ ] Record cold startup, warm startup, settled garage, garage input, local driving, and pending-work save/load.
- [ ] Use `measureContentIndex({profileCpu:true, profileStreaming:false, instrument:false, scenario, label})`.
- [ ] Collect streaming stages in a separate run with `profileStreaming:true`.
- [ ] Record worker initialization, copies, dispatch, return, consumption, and memory costs for terrain as the existing reference path.
- [ ] Collect browser main-thread traces for startup, asset work, physics, and render preparation.
- [ ] Record road and grass call counts, total CPU, worst call, and main-thread output sizes.
- [ ] Rank road and grass by observed CPU and estimated unavoidable copies and publication cost.
- [ ] Select the larger plausible net saving. Record why the other candidate ranks lower.
- [ ] Mark selection provisional until the candidate has actual transfer and synchronization measurements.
- [ ] If both costs are below repeat variation, report no justified worker task and skip Tasks 3–7.
- [ ] If required scenarios are blocked, record the blocker and avoid a complete performance claim.
- [ ] Commit baseline findings as `Record worker task baseline and selection`.

## Road path, only when selected

## Task 3: Share the road calculation without changing its results

**Files:** Create `crates/road-mesh/Cargo.toml`, `src/lib.rs`, `src/codec.rs`, `src/tests.rs`.
Create `crates/road-worker/Cargo.toml` and `src/lib.rs`.
Modify `Cargo.toml`, `Cargo.lock`, `src/world/road.rs`, and `src/world/source_lights.rs`.

**Interfaces:**

- `RoadNode` contains `parent: Option<u32>`, translation `[f32;3]`, rotation `[f32;4]`, and scale `[f32;3]`.
- `RoadPrototype` contains ordered nodes, control indices, and ordered skins.
- `RoadSkin` contains the existing bones, bind poses, positions, normals, tangents, UVs, joints, weights, and triangle groups.
- Define `RoadJobId { session: u64, revision: u64, owner: u32, sequence: u64 }` in `src/world/road_worker.rs`.
- Use the exact primitive field shapes from `WorldSkinReference`. Exclude source names, materials, and collider metadata.
- `RoadPose` contains `origin: [f64;3]` and `rotations: [[f32;3];10]`.
- `RoadMesh` contains origin, vertices, normals, tangents, UVs, triangles, and submesh ranges using current `WorldSurface` field types.
- `deform(prototype: &RoadPrototype, pose: &RoadPose) -> RoadMesh` owns the existing deformation algorithm.
- `frames(nodes: impl ExactSizeIterator<Item=RoadNode>, controls: &[u32], rotations: &[[f32;3];10]) -> Vec<RoadFrame>` owns frame math.
- `RoadFrame` retains the current Rapier vector and rotation types.
- `half_angle(degrees: f32) -> (f32, f32)` owns the existing source polynomial calculation.
- `encode_prototype(input: &RoadPrototype, out: &mut Vec<u8>)` and `decode_prototype(input: &[u8]) -> Result<RoadPrototype, String>`.
- `encode_pose(input: &RoadPose, out: &mut Vec<u8>)` and `decode_pose(input: &[u8]) -> Result<RoadPose, String>`.
- Worker `install(input: &[u8]) -> Result<(), JsError>` stores one admitted dataset.
- Worker `evaluate(input: &[u8]) -> Result<RoadSurface, JsError>` returns move-out array getters and submesh ranges.

- [ ] Add `road_worker_preserves_source_order` using fixed multi-bone, multi-skin inputs and existing deformation fingerprints.
- [ ] Assert identical output bits, triangle groups, absent tangents, reflected coordinates, and source light frame poses.
- [ ] Use `assert_eq!(legacy_attribute_bits, shared_attribute_bits)` against frozen pre-extraction fixture output.
- [ ] Add `road_codec_rejects_truncation_and_invalid_indices`. Assert errors before large allocation.
- [ ] Run `cargo test --locked -p road-mesh` in the project toolchain. Confirm the missing crate causes failure.
- [ ] Extract the calculation and source polynomial without changing arithmetic or operation order.
- [ ] Use the existing `rapier3d = "=0.36.0"` dependency for math. Do not replace its types during extraction.
- [ ] Preserve main-package math features in the shared worker build. Verify feature resolution before comparing bits.
- [ ] Adapt `road_frames` callers through mapped `RoadNode` values. Do not allocate a prototype copy for source light updates.
- [ ] Keep road progression on the main thread. Make its `half_angle` call use the shared calculation.
- [ ] Encode primitive values as checked little-endian bytes, following the terrain codec pattern.
- [ ] Cache one converted prototype per admitted source revision. Include its retained bytes in admission.
- [ ] Implement the small worker entry with `wasm-bindgen = "=0.2.129"`.
- [ ] Run `cargo test --locked -p road-mesh` and `./scripts/check.ps1 -Test world_frontiers`.
- [ ] Require zero test failures and a successful workspace WASM compile check.
- [ ] Commit as `Share source road mesh calculation`.

## Task 4: Integrate the road worker through world admission

**Files:** Use the Task 4 map. Also modify `scripts/build.ps1` and the Task 1 receipt code.
**Tests:** Create `tests/web/road.test.mjs` and `src/world/road_worker_tests.rs`.

**Interfaces:**

- Browser `createRoad(worker)` exposes `install(identity, input)`, `submit(identity, input)`, `poll()`, `read(output)`, `discard()`, and `dispose()`.
- `identity` contains `session`, `revision`, `owner`, and a checked per-worker sequence. Kind is the literal `road`.
- Carry Rust `u64` identities as fixed-width bytes or decimal strings. Do not cast them to JavaScript numbers.
- Rust `RoadWorker` exposes `submit(&mut self, identity: RoadJobId, input: &[u8]) -> Result<(), ReferenceError>`.
- `poll(&mut self) -> Result<RoadPoll, ReferenceError>` returns `Idle`, `Busy`, or `Ready(RoadJobId)`.
- `read(&mut self, output: &mut WorldSurface) -> Result<(), ReferenceError>` consumes one ready result.
- `discard(&mut self)` invalidates delivery but retains the charge until completion or worker termination.
- `dispose(&mut self)` terminates browser work and clears retained buffers.
- Native `RoadWorker` runs the same shared calculation. It is never an error fallback.

- [ ] Add `road_single_flight_and_stale_reply`. Assert a second submit fails, stale output never publishes, and read is single-use.
- [ ] Add `road_initialization_failure_reaches_poll`. Delay initialization, fail it, and assert explicit failure without blocking input.
- [ ] Add `road_discard_keeps_reservation_until_release`. Assert charge persists during computation and returns exactly once afterward.
- [ ] Add `road_worker_preserves_required_collision`. Delay replies and assert no ready-world claim with missing required collision.
- [ ] Run `node --test tests/web/road.test.mjs`. Confirm failure before bridge implementation.
- [ ] Implement the state machine and browser imports using the terrain pattern.
- [ ] Reserve input, cache, worker scratch, output, and copy overlap before dispatch.
- [ ] Track worker WASM capacity separately from logical world reservations.
- [ ] Poll road replies before dispatch decisions, including frames when terrain has priority.
- [ ] Recheck session, revision, owner, required status, and existing surface before publication.
- [ ] Keep the existing terrain collision priority and main-thread publication limits.
- [ ] Bind transport storage to its owning world. Account for the fixed active-world and prepared-world overlap separately.
- [ ] A prepared world may share immutable source data but must never overwrite the active world's pending result.
- [ ] Drain or terminate only the replaced world's transport on commit. Do not terminate the active transport on canceled preparation.
- [ ] Replace the old browser mesh calculation with worker submission. Retain shared native calculation only.
- [ ] Wire worker installation, build output, serving, and all package hashes.
- [ ] Record handshake, packing, both WASM copy boundaries, computation, delivery, consume, publication, and physical release.
- [ ] Run targeted road web tests and `./scripts/check.ps1 -Filter road_worker`.
- [ ] Require native lifecycle tests and the workspace WASM compile check to pass.
- [ ] Commit as `Run admitted road mesh jobs in a worker`.

## Grass path, only when selected

## Task 5: Share grass calculations with the same math version

**Files:** Create `crates/grass-mesh/Cargo.toml`, `src/lib.rs`, `src/domain.rs`, `src/expansion.rs`, `src/codec.rs`, and `src/tests.rs`.
Create `crates/grass-worker/Cargo.toml` and `src/lib.rs`.
Use the Task 5 map for existing callers. Modify `Cargo.toml` and `Cargo.lock`.

**Interfaces:**

- Move `GrassPatch`, `GrassView`, `GrassSettings`, `GrassAnchor`, `GrassDomain`, and `GrassMeshData` into `grass_mesh`.
- Preserve their fields and primitive types.
- Use `glam = "=0.32.1"`, the current Bevy math version. Terrain uses another version and is not the grass authority.
- Move `GrassExpansion` into the shared crate. Split planning, admission, and emission into distinct states.
- `plan(&mut self) -> Result<Option<GrassAllocation>, String>` advances at most 4096 work units and returns exact output sizes.
- `approve(&mut self) -> Result<(), String>` allocates output only after main-thread admission.
- `prepare(&mut self) -> Result<bool, String>` emits at most 4096 work units after approval.
- Keep `finish(self) -> (GrassView, GrassMeshData, usize)` for completed output consumption.
- Define `GrassAllocation` in the shared crate. Presentation converts its errors at the boundary.
- `settings_from_source(record: &WorldSourceMaterial) -> Result<grass_mesh::GrassSettings, ReferenceError>` stays in presentation.
- `encode_dataset(patches: &[GrassPatch], settings: GrassSettings, out: &mut Vec<u8>)` serializes immutable inputs.
- `encode_view(view: GrassView, out: &mut Vec<u8>)` serializes only the current view.
- Worker `install(input: &[u8]) -> Result<(), JsError>` owns patches, settings, and the four domains.
- Worker `plan(input: &[u8]) -> Result<GrassAllocation, JsError>` determines exact output counts without allocating geometry.
- Worker `evaluate() -> Result<GrassSurface, JsError>` emits the admitted plan after a main-thread acknowledgement.

- [ ] Add `grass_shared_calculation_preserves_bits` with culled patches, smoothing, factors 2, 4, 8, and 16.
- [ ] Assert identical ordered output bits, patch IDs, triangle winding, and maximum segment counts.
- [ ] Add `grass_count_before_output_allocation`. Assert no geometry buffer exists before the allocation acknowledgement.
- [ ] Pin the plan boundary with `assert_eq!(task.output.retained_bytes(), 0)` before approval.
- [ ] Add `grass_codec_rejects_invalid_counts`. Assert truncated data, overflow, and invalid domain factors fail.
- [ ] Run `cargo test --locked -p grass-mesh`. Confirm failure before creating the crate.
- [ ] Move calculation code. Keep source material parsing and GPU writers in presentation.
- [ ] Preserve Bevy's resolved glam math features in the main and worker packages. Verify the resolved feature tree.
- [ ] Update imports in material, instances, instance materials, runtime, save preparation, and grass jobs.
- [ ] Delete the old domain and expansion files. Do not add forwarding modules.
- [ ] Keep domains cached per worker dataset. Account for patch, domain, plan, and output overlap.
- [ ] Let the worker return exact planned counts before output allocation.
- [ ] Let the main thread reserve output and copy overlap before acknowledging emission.
- [ ] Keep the generation slot and plan reservation owned during that extra round trip.
- [ ] Measure the planning round trip as synchronization cost.
- [ ] Run `cargo test --locked -p grass-mesh` and `./scripts/check.ps1`.
- [ ] Require source parity tests and the workspace WASM compile check to pass.
- [ ] Commit as `Share source grass mesh calculation`.

## Task 6: Integrate grass results without view starvation

**Files:** Use the Task 6 map. Also modify `scripts/build.ps1`, Task 1 receipts, and `src/world/metrics.rs`.
**Tests:** Create `tests/web/grass.test.mjs` and `src/presentation/world/grass_worker_tests.rs`.
Modify `src/lib.rs` to load the pure grass worker client in native tests with a `cfg(test)` path module.

**Interfaces:**

- `createGrass(worker)` exposes `install`, `submit`, `poll`, `read`, `discard`, `dispose`, and `approve(identity, outputBytes)`.
- Task 6 needs no road crate or bridge. Its operations follow the same ownership contract independently.
- It is task-specific. Do not introduce a shared generic transport framework.
- `GrassWorker` exposes submit, poll, approve, consume, discard, and dispose with the same ownership rules as road.
- `GrassPoll` has `Idle`, `Busy`, `Planned(GrassJobId, GrassAllocation)`, and `Ready(GrassJobId)`.
- `grass_mesh::GrassAllocation` contains checked vertex, index, scratch, output, and GPU reservation sizes.
- `GrassJobId` carries world session, owned view revision, and entity bits without lossy numeric conversion.
- `GrassJobs` retains the existing queued, upload, accepted-view, per-parent, and retirement ownership.

- [ ] Add `grass_camera_changes_do_not_starve_owned_view`. Change the latest view each poll and assert the owned result publishes.
- [ ] Assert the next job receives the latest view after that publication.
- [ ] Add `grass_wrong_session_and_duplicate_plan_are_rejected`. Assert no double output reservation or publication.
- [ ] Add `grass_denied_allocation_never_emits`. Assert the accepted mesh stays active and the plan releases physically.
- [ ] Add `grass_failure_and_retirement_release_once`. Assert plan, output, cache, and continuation charges return correctly.
- [ ] Run `node --test tests/web/grass.test.mjs`. Confirm failure before bridge implementation.
- [ ] Implement the bridge, worker plan acknowledgement, and explicit lifecycle states.
- [ ] Replace inline expansion in `GrassJobs` with nonblocking worker polling.
- [ ] Keep the accepted mesh until upload and publication succeed.
- [ ] Preserve per-parent order and the existing latest-view policy.
- [ ] Charge pending worker work to existing job admission. Release the slot only after the owner physically releases its job.
- [ ] Terminate or drain retired datasets before releasing their retained memory.
- [ ] Keep active-world and prepared-world transport owners distinct. Charge their fixed save/load overlap before initialization.
- [ ] Add selected build output, serving, hashes, timing records, and explicit worker errors.
- [ ] Register the client tests under `grass_worker` in native builds. Presentation itself is WASM-only.
- [ ] Run the grass web tests and `./scripts/check.ps1 -Filter grass_worker`.
- [ ] Require the native command to execute the named lifecycle tests. A zero-test result is not verification.
- [ ] Run `cargo test --locked -p grass-mesh` in the project toolchain.
- [ ] Require zero failures and a successful workspace WASM compile check.
- [ ] Commit as `Run admitted grass expansion in a worker`.

## Task 7: Prove deterministic state and save/load ownership

**Files:** Create `src/world/worker_tests.rs` and register it from `src/world.rs`.
Modify `src/app/save.rs`, `src/presentation/save.rs`, and selected worker owners only where lifecycle wiring is required.
**Consumes:** The first selected worker, its native adapter, and browser bridge.
**Produces:** Critical lifecycle evidence before measuring performance.

- [ ] Add `worker_delay_preserves_save_state`. Replay identical fixed inputs with immediate, delayed, and reordered replies.
- [ ] Compare authoritative world state and captured save values at identical fixed steps.
- [ ] Use `assert_eq!(immediate_save.world, delayed_save.world)` and compare every remaining authoritative snapshot field.
- [ ] Exclude only explicit capture timestamps. Do not normalize worker-induced differences.
- [ ] Add `worker_candidate_load_is_isolated`. Prepare and cancel another world while the active world has pending work.
- [ ] Assert current-world output remains owned, canceled output cannot publish, and commit replaces ownership once.
- [ ] Add `worker_save_does_not_wait_for_visual_output`. Assert snapshot completion while the worker remains busy.
- [ ] Add `worker_session_replacement_releases_physical_storage`. Assert termination precedes reservation release.
- [ ] Add native delay injection only under `cfg(test)`. Do not add production benchmark mode switches.
- [ ] Implement required drain or terminate behavior at existing save prepare, discard, and commit boundaries.
- [ ] Preserve save schema version 6 and all existing save validation.
- [ ] Run `./scripts/check.ps1 -Filter worker_`.
- [ ] Run `./scripts/check.ps1 -Test world_frontiers` and `./scripts/check.ps1 -Test fresh_session`.
- [ ] Run `node --test tests/web/save-session.test.mjs tests/web/save-bridge.test.mjs tests/web/input.test.mjs tests/web/menu.test.mjs`.
- [ ] Require zero failures and a successful workspace WASM compile check.
- [ ] Commit as `Preserve worker ownership across save and load`.

## Task 8: Measure actual costs and keep only justified workers

**Files:** Update `docs/performance/more-worker-tasks.md`.
Create `scripts/verify-worker-output.mjs` for same-build WASM comparison.
**Tests:** `tests/web/worker-output.test.mjs` checks comparison of typed-array bits and output metadata.

**Interfaces:**

- `verifyWorkerOutput({inline, worker, fixtures})` returns per-fixture bit mismatches and failures.
- The inline function runs in a separate verification context using the same worker WASM package.
- The production application has no inline fallback or mode toggle.
- `summarizeWorkerComparison` consumes the Task 1 receipt schema and refuses invalid sample sets.

- [ ] Add `worker_bit_comparison_checks_all_attributes`. Change one float bit and one index, then assert both mismatches appear.
- [ ] Assert `result.failures.length === 2` for those two altered fixture attributes.
- [ ] Implement the comparison script and run `node --test tests/web/worker-output.test.mjs`.
- [ ] Build the selected candidate with `./scripts/build.ps1`. Freeze all web and WASM package files.
- [ ] Serve the same frozen measurement scripts for baseline, candidate, and recheck.
- [ ] Run the same worker WASM inline and in its worker against fixed source fixtures.
- [ ] Require identical output bits, counts, submeshes, and patch IDs.
- [ ] Run release A/B/A samples with baseline, candidate, and baseline-recheck labels.
- [ ] Use uninstrumented samples for frame intervals. Use separate CPU samples for task cost and input latency.
- [ ] Check mean, p95, p99, maximum, input latency, startup cost, retained memory, and job latency.
- [ ] Verify active physics, submitted frames, real pointer and keyboard input, valid collision, local driving, and pending-work save/load.
- [ ] Record screenshots or traces that show visible correctness for road or grass changes.
- [ ] Compare total removed main-thread calculation against packing, copy, validation, polling, and publication costs.
- [ ] Require net saving greater than the same metric's baseline repeat variation.
- [ ] Reject p95, p99, or input latency regressions beyond their own repeat variation.
- [ ] Repeat inconclusive samples. Keep invalid or blocked samples as evidence, never as valid gains.
- [ ] If the worker fails acceptance, remove its browser route and unused worker packages in a normal commit.
- [ ] Preserve shared calculation changes only when they remain useful and pass source parity checks.
- [ ] Re-run targeted checks for any removal. Keep the profiler and negative findings.
- [ ] If the first worker passes, assess the second candidate from fresh profiles before entering its conditional tasks.
- [ ] Run Tasks 7 and 8 again for the combined candidate if a second worker is added.
- [ ] Record exact commands, package identities, receipt paths, costs, failures, and unverified scenarios.
- [ ] Provide exact self-run commands with the frozen asset and package roots.
- [ ] Commit as `Record measured worker costs and acceptance`.

## Toolchain commands

Run native crate commands in the same Docker toolchain used by `scripts/check.ps1`.
Build that image with `docker build --tag my-summer-slop-toolchain --file tools/Dockerfile .`.
Use `Get-CargoTargetDirectory` from `scripts/cargo-target.ps1` for this worktree's target namespace.
Mount the worktree at `/workspace`, `tld-wasm-cargo` at `/cargo`, and `tld-wasm-target` at `/target`.
Do not install another Rust toolchain or change dependency versions.
Update the lockfile only for selected local crates and already-locked dependencies.

`scripts/check.ps1` always includes a workspace WASM compile check.
Its targeted native form is `./scripts/check.ps1 -Filter <filter>`.
Its integration form is `./scripts/check.ps1 -Test <name>`.
Do not use `-Full` for this plan.

## Plan review

The plan covers profiling, selection, transfers, synchronization, determinism, memory, errors, save/load, input, and runtime acceptance.
Conditional tasks prevent unmeasured workers from entering the product.
All five Review Focus conditions have named checks.
The plan creates no production fallback and changes no save schema.

Implementation waits for plan review and an execution choice.
Native execution is recommended because task selection and measurement are sequential.
Use GPT-6.1 Sol with high reasoning for implementation and any delegated review.
Never use GPT-6 Astra.

## Approved startup prerequisite

The user approved this scope extension on 2026-10-19.
Before Task 2 resumes, repair authored reflection probe slots in the existing two-slot renderer.
Keep absent slots disabled across startup, scene reset, distance checks, and prepared load.
Preserve source registration, distance cadence, GPU capacity, shader layout, and manifest contents.
Reject zero probes and counts above capacity at the data boundary.
Verify critical native regressions and the routine WASM check before review.
Release and browser acceptance remain the next parent-owned Task 2 milestone.

## User execution override, 2026-10-20

The user instructed: "Implement workers. Skip tests."
Resume Tasks 3 through 7 for both road and grass, in that order.
Task 2 profiling is retained as partial evidence, not an implementation gate.
Skip all test creation and execution steps, browser runs, and benchmark steps.
Keep production correctness and lifecycle requirements.
Use compile-only checks and source review. Do not run scripts/check.ps1 or scripts/build.ps1.
Task 8 records implementation and unverified costs, without gain or acceptance claims.
