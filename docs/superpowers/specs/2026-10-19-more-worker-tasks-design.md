# More worker tasks

Date: 2026-10-19.
Status: design approved in chat. Written spec awaits review.
Base: `738ce8b2b907491725c52b3a917c42f552ac9174`.
Branch: `codex/more-worker-tasks`.

## Purpose and scope

Reduce main-thread CPU work in the current garage and local driving experience.
Profile before selecting worker tasks.
Count transfer and synchronization costs when judging each task.
Preserve deterministic results, save/load behavior, and input response.

The user approved this design and its measurement milestones in chat.
Use an isolated worktree.
Keep the branch unmerged unless the user authorizes integration.

Steam build `11140890` remains the behavior authority.
The current save schema remains the format authority.
This work does not expand the garage milestone.

## Current behavior

The inspected base has one terrain worker with separate WASM memory.
[web/main.js](../../../web/main.js) starts that worker during startup.
[web/terrain.js](../../../web/terrain.js) permits one outstanding job and one unread result.
Submission copies the input from main WASM memory before transfer.
Result consumption copies returned arrays into main WASM memory.
Transferable buffers therefore do not remove all copies.

[TerrainWorker](../../../src/world/terrain.rs) provides browser and native clients.
The native client advances the same terrain task during polling.
[WorldStream](../../../src/world/stream.rs) checks the owning request before accepting terrain output.
Stale terrain output is discarded.

Road mesh deformation currently runs within `WorldStream::prepare_road`.
It calls `deform_road` after job and memory admission.
[deform_road](../../../src/world/road.rs) reads an admitted prototype and a completed road segment.
Its bone, influence, vertex, and submesh order carry source behavior.
Road progression and its random state are separate from this mesh calculation.

[GrassExpansion](../../../src/presentation/materials/grass/expansion.rs) plans and emits mesh data.
Each `prepare` call bounds work to 4096 prospective or emitted vertices.
[GrassJobs](../../../src/presentation/world/grass_jobs.rs) owns admission, request identity, upload, publication, and retirement.
An accepted grass mesh can remain resident while another result is pending.

[stream_profile.rs](../../../src/world/stream_profile.rs) already measures 27 streaming stages.
Four stages are nested and must not be added to their parent totals.
The existing benchmark scripts record package identity, submitted frames, physics steps, focus, and local driving.
They do not yet measure the complete proposed worker costs.

These findings identify candidates.
They do not establish current bottlenecks or performance gains.

## Selected approach

Use a dedicated worker for each task type that passes measurement.
Keep the terrain worker independent.
Start with one outstanding job and one unread result per added worker.
Use existing world job limits to bound total work.
Do not create a general worker pool or dynamic worker count.

The initial candidate set is road mesh deformation and grass expansion.
Select the first task by measured main-thread cost and achievable net saving.
Implement and verify that task end to end before adding the second.
Add the second only if its own measurements justify it.
Do not add workers solely to increase the worker count.

A shared terrain worker has less setup but can delay terrain needed for collision.
Shared WASM memory avoids some copies but requires a larger concurrency design.
Both alternatives are outside this implementation.

## Profiling contract

Profile these scenarios separately:

- Startup with an empty cache.
- Startup with a populated cache.
- A settled garage with active simulation.
- Garage interaction with tools, menus, and camera input.
- Local driving with terrain and road preparation.
- Save and load with pending derived work.

Use the same seed, assets, viewport, DPR, resolution, and input route for matched runs.
Record the browser, GPU, build profile, commit, asset hashes, and package hashes.
Include every added worker JavaScript and WASM package in the recorded identity.
Do not use the dirty primary checkout as the baseline.

Reuse existing startup traces, frame counters, streaming timers, and benchmark scripts.
Add missing spans around main-thread startup, asset processing, save/load, and render preparation.
Measure physics and input handling to detect regressions.
Keep instrumentation opt-in.
Disabled instrumentation must not read clocks or allocate per frame.

For each candidate, record:

- Main-thread calculation time before offload.
- Input packing and allocation time.
- Main WASM to JavaScript copy time and byte count.
- Message submission time.
- Worker readiness and queue delay.
- Worker input decoding and WASM copy time.
- Worker calculation time.
- Worker result extraction and message submission time.
- Result delivery delay.
- Main-thread validation and result copy time.
- Main-thread publication time and delay.
- Total job latency.
- Stale results, errors, and peak outstanding bytes.

Use timestamps relative to each context's time origin for cross-context intervals.
Validate the timestamp conversion with a startup handshake.
Never subtract unrelated raw worker and window timestamps.
Report startup costs and retained worker memory separately from steady-state costs.
Count terrain contention, worker initialization, and frame-boundary waits in total latency.
Keep main-thread CPU saved separate from end-to-end job latency.

Extend the receipt writer and summarizer rather than creating a parallel benchmark system.
The result must be rerunnable from saved receipts.
Existing garage and driving receipt validation must still apply.
Add explicit startup and save/load measurements rather than labeling them settled garage samples.

## Task and result ownership

The main thread owns input, fixed-step simulation, random state, save snapshots, and world publication.
Workers receive immutable calculation inputs.
Workers never mutate live world state or choose the simulation step.

Each job carries the world session, request revision, task kind, and owner identity.
Each result repeats that identity.
Only the owner can accept or discard the result.
Represent idle, computing, and ready states as distinct states.
Do not use wrap-prone identifiers without a checked session boundary.

Install a worker's immutable source data once per dataset revision.
Send compact per-job inputs after installation.
Account for initialization copies and retained source data.
Do not send the full reference manifest or world snapshot for each job.
Do not share mutable WASM memory.

Transfer owned buffers between contexts.
Copy live main WASM views before transfer.
Copy returned data into main WASM only once.
Reserve all input, scratch, overlap, and output bytes before dispatch.
Account for worker WASM memory, immutable caches, and unread output separately.
Keep existing renderer and world reservations authoritative.
Do not allocate temporary job containers during idle per-frame polling.

Poll without blocking the main thread.
Do not use synchronous waits or busy loops.
Accept results only through the existing world or presentation publication gate.
Preserve owner order where the existing scheduler requires it.
Worker completion order must not choose random draws, source identities, or saved values.

Load, new game, world retirement, and request changes invalidate prior ownership.
Reject late, duplicate, or mismatched results.
Retain their memory charge until buffers and worker references are actually released.
On world replacement, drain old ownership or terminate its worker before releasing retained reservations.
Reinitialize immutable data before accepting jobs from a new dataset.

## Candidate boundaries

### Road mesh deformation

Provide the completed `RoadSegment` and immutable admitted road prototype.
Return mesh attributes, triangle indices, submesh ranges, and origin.
Keep road progression, source random state, admission, collision activation, and publication on the main thread.

Preserve arithmetic types and operation order.
Preserve bone order, influence order, and the existing coordinate reflection.
Use one shared calculation implementation for native and worker builds.
Remove the replaced browser calculation path after integration.
A native client is a test and platform adapter, not an error fallback.

The worker must not allow driving into missing collision.
Preserve existing required-world readiness rules.
Reject the candidate if safe preparation would introduce an input stall or change simulation behavior.

### Grass expansion

Provide immutable patches, domains, settings, and a captured view.
Return anchors, UVs, blade attributes, patch identities, and indices.
Keep materials, Bevy assets, GPU upload, and publication on the main thread.

Extract only calculation code from the current presentation module.
Reuse existing math dependencies.
Keep the worker build independent of the renderer.
Preserve patch order, domain order, tessellation rules, and emitted values.

Continue to display the accepted mesh until its replacement passes the existing publication gate.
Keep the existing per-parent ordering and retirement rules.
A camera change records the latest requested view.
Do not cancel every running job on camera movement and starve publication.
Finish the owned view, then schedule the latest view under the existing policy.
A world-session change still invalidates the running result.

## Errors and save/load

Worker initialization failure, panic, malformed output, and transport failure produce an explicit task failure.
Do not silently run failed worker calculations on the main thread.
Reject malformed lengths, invalid indices, and mismatched identities at the result boundary.
Reuse the existing runtime failure reporting path.

Save snapshots contain authoritative state only.
Worker buffers, message IDs, and timing data never enter the save schema.
Saving does not wait for a visual job.
Loading retains the existing prepare, cancel, and commit behavior.
Prepared worlds cannot publish output into the current world before commit.
Canceled worlds cannot publish after cancellation.
Generated output may be reconstructed from saved state.

Verify that changed completion timing cannot alter collision readiness, world progression, or save contents.
Do not weaken existing save validation to accept worker output.

## Modules

Keep pure calculations in small shared Rust modules or crates.
Create a task-specific worker entry and browser bridge only for a selected task.
Follow the existing terrain bridge pattern without adding a generic messaging framework.
Expose submit, poll, consume, and discard operations through the browser boundary.
Keep scheduling and publication in `WorldStream` or `GrassJobs`, as applicable.
Update `scripts/build.ps1` to emit selected worker packages.
Update package hashing and serving for those packages.

Exact extraction files and function signatures belong in the implementation plan.
The ownership and behavior contracts in this spec remain fixed.

## Measurement and acceptance

Release profiling and matched release A/B/A browser runs are approved milestones.
Run one game measurement at a time.
Use frozen assets and matching release settings.
Keep instrumented CPU receipts separate from uninstrumented frame-time receipts.

For each task, compare the baseline, candidate, and baseline recheck.
Report mean, p95, p99, and maximum frame intervals.
Report main-thread CPU costs and worker job latency separately.
Measure input event to simulation consumption, and consumption to submitted frame.
Use real input for browser acceptance.
Treat scripted input as a reproducible timing aid, not proof of normal interaction.

A worker qualifies when its main-thread saving exceeds both its added main-thread costs and baseline repeat variation.
The candidate must not worsen p95 or p99 frame time beyond that repeat variation.
Input latency must not worsen beyond the same metric's baseline repeat variation.
Calculate variation from the two baseline runs for each metric and scenario.
Repeat inconclusive runs before making a gain claim.
A lower worker calculation time alone is insufficient.

Report startup, garage, and local driving separately.
Include first-use cost and retained memory in the decision.
If neither task qualifies, retain the profiling tools and report the negative result.
Do not ship a slower worker path to satisfy the task title.

Critical checks cover:

- Bitwise output equality between same-build WASM inline and worker execution for fixed inputs.
- Existing native source parity and deterministic replay checks for changed Rust calculations.
- Different worker delays producing the same authoritative state and save values.
- Stale, duplicate, malformed, failed, and canceled results.
- Reservations returning to their prior values after consume, discard, retirement, and failure.
- Save and load while jobs are pending.
- Camera movement without grass job starvation.
- Active physics, submitted frames, real input, and local driving with valid collision.

Run targeted web tests for bridge and receipt changes.
For Rust changes, run targeted native tests and a workspace WASM compile check.
Do not run full checks unless separately requested.
A blocked browser scenario remains unverified.
A build, renderer-ready flag, or submitted frame alone does not prove acceptance.

## Non-goals

Do not move physics, input sampling, save ownership, GPU submission, or authoritative random progression into workers.
Do not add shared-memory threads, a configurable worker pool, save migrations, or compatibility fallbacks.
Do not change source generation limits, road count, visual quality, or the approved garage scope.
Do not fix unrelated renderer defects within this change.
Record any baseline runtime blocker separately.

## Review and handoff

This commit contains the design only.
No worker implementation or measured performance result is claimed.
After written-spec approval, create the implementation plan with the writing-plans skill.
The user reviews that plan and selects its execution method before implementation.
