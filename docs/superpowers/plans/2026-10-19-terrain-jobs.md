# Terrain job ownership Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give terrain execution one job owner while preserving stream policy and current terrain output.

**Architecture:** A per-stream `TerrainExecution` owns pending input, metadata, worker identity, cancellation, drain, and result consumption. `WorldStream` retains selection, admission, deadlines, failure handling, and publication. The browser bridge retains its single physical worker.

**Tech Stack:** Rust 2024, existing `terrain` crate, wasm-bindgen, native unit tests, and Node test runner.

**Spec:** [Approved terrain specification](../specs/2026-10-19-terrain-jobs-design.md).

Status: Awaiting user plan review. Product implementation is not authorized by this document alone.
Branch: `codex/codebase-architecture`. Plan base: `8ce8b0c`.
Worktree: `C:/Users/Corashina/.codex/worktrees/codebase-architecture/the-long-drive-wasm`.
Execution method: Subagent-driven development, as selected by the user.
Execution order: save, startup, terrain, pipeline, then readiness.
The coordinator owns commits. Do not edit shared production files concurrently with another section.

## Global Constraints

- All existing startup, save, and renderer interfaces remain unchanged.
- `JobId` never leaves the execution module.
- Keep encoded input until completion or cancellation, as today.
- Add no compatibility path, transport trait, generic queue, or new dependency.
- The executor does not make that selection decision.
- Transfer the same reservation into `PreparedSurface`; count it once.
- Browser drain without a logical job retains current accounting semantics.
- Keep the one-millisecond driving slice and the loading deadline, including descriptor time.
- Preserve current wire bytes and all six result arrays.
- Preserve surface origin, triangle indices, and the single submesh range.
- No per-frame allocation, repeated input encoding, or repeated worker setup is added.
- The branch remains unmerged until authorized.

`WorldStream` still decides whether unchanged context permits same-session revision updates.
Never update a pending job to another session.
Native cancellation drops its task. Browser cancellation must leave its physical task running until drain.
Keep sticky worker errors and the existing failure route. Add no retry or worker restart.
Full checks, release builds, and browser acceptance require a separate request or agreed milestone.

## Review Focus

1. A stale session or revision cannot deliver a surface. Tests: `terrain_jobs_reconcile_requests`, `terrain_jobs_reject_stale_delivery`.
2. A retained job can yield repeatedly without new input or submission. Test: `terrain_jobs_yield_and_read_once`.
3. A canceled browser job blocks reuse until physical completion. Test: `in-flight discard waits for drain before reuse`.
4. An older result cannot consume the current queued job or its reservation. Tests: `terrain_jobs_drain_orphan`, `stale replies cannot replace the current result`.
5. Completion transfers the reservation once. Tests: `terrain_jobs_yield_and_read_once` and the existing export-dependent admission test.

## File map

| File | Responsibility |
| --- | --- |
| `src/world/terrain/jobs.rs` | New execution owner, metadata, private submission state, and world-visible methods. |
| `src/world/terrain/jobs/tests.rs` | New small tests using actual encoded jobs and `TerrainTask`. |
| `src/world/terrain.rs` | Register the module, expose its world-visible types, and restrict the raw adapter. |
| `src/world/stream.rs` | Replace split state, migrate preparation and reconciliation, and retain resource policy. |
| `src/world.rs` | Remove obsolete raw adapter re-exports. |
| `src/world/stream_tests.rs` | Migrate the existing admission test to execution methods and observable completion. |
| `tests/web/terrain.test.mjs` | Extend the existing bridge tests using their existing worker fixture. |

No changes belong in terrain math, wire encoding, the worker WASM crate, browser bindings, or renderer interfaces.
Keep `web/terrain.js`, `web/terrain-worker.js`, and `web/main.js` unchanged unless a reproduced contract failure requires review.

## Task 1: Transfer the complete terrain lifecycle and pin both adapter contracts

**Files:** Create the two job files above. Modify the other five files in the file map.
This task is one complete owner migration. Do not introduce a temporary second execution path.

**Interfaces consumed:**

```rust
TerrainJob::encode(&self, out: &mut Vec<u8>);
TerrainWorker::submit(&mut self, input: &[u8]) -> u32;
TerrainWorker::poll(&mut self, should_yield: impl FnMut() -> bool)
	-> Result<TerrainPoll, ReferenceError>;
TerrainWorker::read(&mut self, unit: [i64; 2], tile: TerrainTile,
	surface: &mut WorldSurface);
TerrainWorker::discard(&mut self);
```

These existing raw methods become private after migration. Their native and WASM bodies keep their current behavior.
The executor uses existing `WorldRequest`, `SurfaceKey`, `TerrainTile`, `WorldSurface`, and `ReferenceError` types.

**Interfaces produced:**

```rust
#[derive(Default)]
pub(in crate::world) struct TerrainExecution {
	worker: TerrainWorker,
	pending: Option<PendingTerrain>,
}
struct PendingTerrain {
	metadata: TerrainMetadata,
	input: Vec<u8>,
	submission: Submission,
}
enum Submission { Queued, Submitted(JobId) }
#[derive(Clone, Copy, PartialEq, Eq)]
struct JobId(u32);
pub(in crate::world) struct TerrainMetadata {
	pub request: WorldRequest,
	pub key: SurfaceKey,
	pub tile: TerrainTile,
	pub pending_bytes: usize,
}
pub(in crate::world) struct CompletedTerrain {
	pub metadata: TerrainMetadata,
	pub surface: WorldSurface,
}
```

The methods below have `pub(in crate::world)` visibility inside `jobs`:

```rust
fn key(&self) -> Option<SurfaceKey>;
fn has_pending(&self) -> bool;
fn pending_bytes(&self) -> usize;
fn reconcile(&mut self, request: WorldRequest, retain: bool);
fn enqueue(&mut self, metadata: TerrainMetadata, input: Vec<u8>);
fn advance(&mut self, request: WorldRequest,
	should_yield: impl FnMut() -> bool)
	-> Result<Option<CompletedTerrain>, ReferenceError>;
```

`enqueue` requires no pending logical job. It records `Queued` and never replaces an existing job.
Use an internal assertion for this caller invariant. Add no public validation type.
Keep the module available on native and WASM builds, as `world::stream` is today.
Expose the three execution types through `world::terrain`, restricted to `world`.
Keep the new tests private. They can access the ancestor adapter without new production hooks.

- [ ] **RED: Add the concrete native fixture and five lifecycle tests.**

Register `mod jobs;` in `terrain.rs` and `#[cfg(test)] mod tests;` in the new `jobs.rs`.
Add tests before execution types, so the RED command compiles the new assertions.
Define these private helpers in `src/world/terrain/jobs/tests.rs`:

```rust
fn request(session: u64, revision: u64) -> WorldRequest;
fn input(request: WorldRequest, unit: [i64; 2]) -> (TerrainMetadata, Vec<u8>);
fn finish(execution: &mut TerrainExecution, request: WorldRequest) -> CompletedTerrain;
fn assert_mesh(actual: &WorldSurface, expected: &terrain::TerrainMesh);
```

`request` uses zero player and vehicle positions.
`input` uses seed `42`, edge `100000.0`, and edge gradient `5000.0`.
Use tier `0`, size `100`, resolution `4`, draw distance `1`, y `0.0`, and material scale `1.0`.
Encode with `TerrainJob`, no biome override, and empty roads, foundations, and holes.
Metadata uses `WorldLayer::Terrain`, the supplied unit, and `terrain_output_bound(tile) + 512` reserved bytes.
`finish` calls `advance(request, || true)` at most `1000` times. It fails if no completion arrives.
`assert_mesh` compares all five float arrays through `to_bits()`, and compares triangles and origin exactly.
It also checks `submeshes == vec![0..actual.triangles.len()]`.
No manifest, fake executor, sleep, or wall-clock deadline belongs in these tests.

Define these exact test cases:

- `terrain_jobs_yield_and_read_once`: enqueue session `10`, revision `3`, unit `[2, -1]`.
  Assert one pending job and its exact reservation before submission and through every incomplete poll.
  Count yield callback calls. Require more than one poll and at least one callback invocation.
  Compare completion with `terrain::evaluate(&bytes)` saved before the input moves.
  Assert request, key, tile fields, and reserved bytes survive completion.
  Assert `execution.pending_bytes() == 0`, `!execution.has_pending()`, and another advance returns `None`.
- `terrain_jobs_reconcile_requests`: submit a real task, then retain revision `4` in session `10`.
  Assert the returned request uses revision `4` and the reservation stays unchanged until completion.
  Repeat with `retain = false` and with session `11` plus `retain = true`.
  Assert each cancellation clears pending bytes and no completion arrives.
- `terrain_jobs_reject_stale_delivery`: submit revision `3`, then advance with revision `4` without reconciliation.
  Repeat with another session. Assert neither task returns a completion and each logical reservation clears.
- `terrain_jobs_drain_orphan`: privately submit an older real task through `execution.worker` before enqueueing the current job.
  Use different tile units for old and current input. Drive the actual task with yielding polls.
  Assert the older task never becomes a completion and never clears the queued current reservation.
  Assert only the current surface and metadata emerge, followed by idle execution.
- `terrain_jobs_error_route`: clear the encoded input, enqueue it, and advance once to submit.
  The next advance must return an error with the `Terrain worker:` prefix and `Terrain job input is truncated`.
  Assert the error through its `.0` message. Do not add recovery behavior.

- [ ] **RED: Add browser contract cases using existing `fakeWorker()` and `result(id)`.**

Keep the existing transfer-copy, read-once, ready-discard, and worker-error tests.
Add `in-flight discard waits for drain before reuse` with these assertions:

```javascript
const worker=fakeWorker(),terrain=createTerrain(worker),bytes=new Uint8Array([1]);
const first=terrain.submit(bytes);
terrain.discard();
assert.equal(terrain.poll(),-2);
assert.throws(()=>terrain.submit(bytes),/busy/);
worker.reply(result(first));
assert.equal(terrain.poll(),first);
terrain.discard();
assert.equal(terrain.poll(),-1);
const second=terrain.submit(bytes);
assert.notEqual(second,first);
```

Add `stale replies cannot replace the current result`.
Submit and drain the first job, then submit the second job.
Reply with the first ID and assert `poll() === -2`.
Reply with the second ID and assert `poll() === second`.
Reply with the first ID again and assert the second ready result remains.
Read using the existing correctly sized arrays. Assert polling returns idle and a second read throws.
These browser cases pin existing behavior. They may pass before the Rust refactor.
Do not create a failing bridge behavior or change discard semantics to force RED.

- [ ] **RED: Run `./scripts/check.ps1 -Filter terrain_jobs` and `./scripts/check.ps1 -WebTest terrain`.**

First format the test changes with `cargo fmt --all` inside the configured Rust toolchain.
Expect native compilation to fail on missing execution types or methods.
Record the browser baseline. Confirm its failures, if any, concern the asserted contract.
The native script also requests formatting and WASM compilation, but compilation stops at the first failure.

- [ ] **Implement the executor and register it in `src/world/terrain.rs`.**

Poll at the existing preparation point. Submit queued input only after transport is idle.
For a ready ID, consume only matching submitted metadata with the current session and revision.
Discard unmatched results, retain a queued current job, and submit it only after idle.
Reject stale pending metadata before result delivery or new submission.
`reconcile` retags only same-session work when the caller passes `retain = true`.
Other reconciliation cancels logical state and calls the existing adapter discard method.
Keep worker errors as `ReferenceError`; the executor does not set stream failure state.
Keep the caller's yield callback borrowed through polling. Do not store it or allocate a callback.

- [ ] **Migrate stream state, selection, preparation, and accounting together.**

Replace both terrain fields with `terrain_execution: TerrainExecution` and initialize it once.
Remove `TerrainWork` and all stream comparisons with raw worker IDs.
At selection, compute retention from the existing context-dirty flag and selected tile set.
At preparation, advance before new admission. On completion, build the existing `PreparedSurface` and return.
Retain its bounds, key, request, reservation, and existing publication fields.
For new work, preserve collision priority, tier readiness, global capacity checks, and byte admission before context construction.
Keep road filtering, startup foundations, holes, settings, and `TerrainJob::encode` unchanged.
Enqueue the admitted input, then call `advance` to submit when idle.
Keep `prepare` and `prepare_geometry` deadlines, gates, and road priority unchanged.
Replace every work count and audit flag with `has_pending()`, including overlap and external-job calculations.
Replace every work reservation read with `pending_bytes()` and transfer it once into ready state.

- [ ] **Migrate the existing admission test and remove obsolete raw access.**

Update `terrain_admission_respects_pending_capacity_and_continues_at_capacity` in `src/world/stream_tests.rs`.
Keep its at-capacity continuation, pending count, reservation release, and stale-request assertions.
Replace raw ID assertions with `has_pending()`, bounded preparation, and observable `ready` completion.
Keep `#[ignore]` and its installed-export requirement.
Make raw worker methods and types private to terrain. Remove their `world.rs` re-exports and imports.
Search `rg -n 'terrain_work|terrain_worker|TerrainPoll|TerrainWorker' src`.
Only private adapter and execution references may remain. No stream test may poll the raw worker.

- [ ] **GREEN: Run targeted native, WASM, and web checks.**

```powershell
./scripts/check.ps1 -Filter terrain_jobs
./scripts/check.ps1 -WebTest terrain
```

Require all five new native tests and all terrain web tests to pass.
The native script also runs formatting and `cargo check --locked --target wasm32-unknown-unknown --workspace --lib`.
Run the existing encoder test inside the configured Rust Docker toolchain:

```sh
cargo test --locked -p terrain encoded_job_matches_direct_surface
```

Require the named test to execute and pass. It compares encoded, direct, and resumed terrain output.
When installed exports exist, also run these explicit commands in that toolchain:

```sh
cargo test --locked --lib terrain_admission_respects_pending_capacity_and_continues_at_capacity -- --ignored
cargo test --locked --lib terrain_source_fingerprints -- --ignored
```

Otherwise record these two export tests as deferred. A normal filter does not execute ignored tests.
Native fixture tests prove lifecycle decisions and the native adapter. Web fixtures prove bridge message handling.
They do not run the Rust executor against a real browser worker or prove global stream budgets without exports.
WASM compilation proves the adapter and visibility paths compile. It does not prove browser execution or source parity.
Do not run full checks, release builds, or browser acceptance under this plan.

- [ ] **Review the complete diff and commit through the coordinator.**

Check every changed field use and all three module visibility boundaries against the approved spec.
Confirm no new frame allocation, input encoding loop, worker setup, wire change, or renderer API change exists.
Use the existing native deadline callback. Do not replace it with an elapsed-time fixture assertion.
Stage only this task's two created files and five modified files. The coordinator runs:

```powershell
git add src/world/terrain/jobs.rs src/world/terrain/jobs/tests.rs src/world/terrain.rs src/world/stream.rs src/world.rs src/world/stream_tests.rs tests/web/terrain.test.mjs
git commit -m "refactor: give terrain execution ownership of job lifecycle"
```
Keep the branch unmerged and report passing, failed, deferred, and unverified checks separately.

## Handoff

This plan awaits user review. Subagent-driven development is already selected.
No product edit, test, build, or Git mutation occurred while writing the plan.
