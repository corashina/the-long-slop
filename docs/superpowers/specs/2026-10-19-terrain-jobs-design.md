# Terrain job ownership

Date: 2026-10-19.
Status: Approved by the user on 2026-10-19.
Branch: `codex/codebase-architecture`.
Base: `97783a10444c9d5faef9d1c5a10b2ff21469d33c`.

## Intent and scope

Give terrain execution one owner for queued input, worker identity, cancellation, and result consumption.
Keep terrain selection and global resource policy in `WorldStream`.

The user approved this design in chat.
The user approved this written specification. Implementation planning is authorized.

## Current behavior

`WorldStream` stores `terrain_work` beside `terrain_worker`.
`TerrainWork` holds reserved bytes, a world request, a surface key, encoded input, and an optional worker ID.
Selection cancels work when terrain context changes or the tile leaves the selected set.
Preparation polls the worker, checks ownership, reads a matching result, and creates `PreparedSurface`.

Native polling resumes `TerrainTask` through a yield callback.
Native discard drops the task immediately.
Browser discard clears ready buffers. It does not stop an in-flight job.
The browser bridge owns one worker across world sessions.

## Selected design and ownership

Add private module `src/world/terrain/jobs.rs` with `TerrainExecution`.
Replace `WorldStream::terrain_work` and `WorldStream::terrain_worker` with one `TerrainExecution` field.

| Owner | Responsibility |
| --- | --- |
| `WorldStream` | Tile selection, terrain context validity, input construction, budget admission, deadlines, failure policy, and publication. |
| `TerrainExecution` | Queued input, request metadata, worker ID, cancellation, stale-result drain, and one result consumption. |
| `TerrainWorker` | Native task execution or browser transport, and conversion into `WorldSurface`. |
| `terrain` crate | Existing job bytes, validation, terrain math, and resumable surface generation. |
| Browser bridge | One physical worker, copied input transfer, returned buffers, and transport errors. |

All existing startup, save, and renderer interfaces remain unchanged.

## Types and interface

```rust
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
struct JobId(u32);
pub(in crate::world) struct TerrainMetadata {
	pub request: WorldRequest,
	pub key: SurfaceKey,
	pub tile: TerrainTile,
	pub pending_bytes: usize,
}
pub(in crate::world) struct CompletedTerrain { pub metadata: TerrainMetadata, pub surface: WorldSurface }
```

`TerrainExecution` exposes these methods within `world`:

```rust
fn key(&self) -> Option<SurfaceKey>;
fn has_pending(&self) -> bool;
fn pending_bytes(&self) -> usize;
fn reconcile(&mut self, request: WorldRequest, retain: bool);
fn enqueue(&mut self, metadata: TerrainMetadata, input: Vec<u8>);
fn advance(
	&mut self,
	request: WorldRequest,
	should_yield: impl FnMut() -> bool,
) -> Result<Option<CompletedTerrain>, ReferenceError>;
```

`enqueue` requires no pending logical job. The internal caller establishes this precondition.
It records `Queued` without replacing an existing job.
Keep encoded input until completion or cancellation, as today.
`JobId` never leaves the execution module.
Raw worker methods become private to terrain after all callers migrate.
Remove public `TerrainPoll` and `TerrainWorker` re-exports when no caller needs them.
Add no compatibility path, transport trait, generic queue, or new dependency.

## Lifecycle and errors

Logical pending work and physical browser activity are separate facts.
The executor can hold one queued job while an earlier browser job drains.
It submits that job only after transport returns idle.

| State or event | Result |
| --- | --- |
| No pending job, transport idle | No completion and no submission. |
| Queued job, transport busy | Retain the input and reservation. Submit nothing. |
| Queued job, transport idle | Submit once and store `Submitted(JobId)`. |
| Submitted job, transport busy | Retain the job. Native execution honors the yield callback. |
| Matching ready ID and current request | Read once, clear pending state, and return `CompletedTerrain`. |
| Unmatched ready ID | Discard the old result. Keep any queued job and submit only after idle. |
| Stale pending request | Cancel the logical job. Never publish its result. |
| Worker poll error | Return the existing `ReferenceError`. The stream records its failure. |

`reconcile` cancels when `retain` is false.
A different request session also cancels, even when `retain` is true.
Within one session, `retain` updates the request revision for unchanged terrain context.
`WorldStream` computes `retain` from terrain context validity and the selected tile set.
The executor does not make that selection decision.
`advance` also rejects stale session or revision metadata before result delivery.

Cancellation releases the logical input and its reservation.
Native cancellation drops the task. Browser cancellation clears ready buffers but leaves in-flight work running.
A later `advance` discards that job's result before worker reuse.
Keep the current sticky browser error behavior. Add no retry, worker restart, or synchronous browser evaluation.

## Caller flow and resource rules

`WorldStream` keeps the existing preparation order:

1. Reconcile pending work after selection.
2. Advance terrain execution at the current geometry preparation point.
3. On completion, transfer the result into `PreparedSurface` and return.
4. When no logical job exists, select the next tile and check global admission.
5. Construct its current terrain context and encoded input once.
6. Enqueue the admitted job and call `advance` to submit only when transport is idle.

Preserve nearby collision priority, required terrain rules, and terrain material readiness gates.
Preserve the renderer's single prepared-surface slot and road-versus-terrain order.
A completion retains request, key, tile, and pending-byte metadata.

Use `has_pending()` wherever current metrics count `terrain_work.is_some()`.
Use `pending_bytes()` wherever current metrics read the work reservation.
Transfer the same reservation into `PreparedSurface`; count it once.
Browser drain without a logical job retains current accounting semantics.
Do not relabel the existing main-thread reservation as total worker memory.

Pass the current yield callback through the executor to native polling.
Keep the one-millisecond driving slice and the loading deadline, including descriptor time.
Browser math continues in its worker without a main-thread yield callback.
Preserve current wire bytes and all six result arrays.
Preserve surface origin, triangle indices, and the single submesh range.
No per-frame allocation, repeated input encoding, or repeated worker setup is added.

## File map and section interfaces

| File | Change |
| --- | --- |
| `src/world/terrain/jobs.rs` | Own the executor, lifecycle types, and small native tests. |
| `src/world/terrain.rs` | Declare the executor module and restrict raw worker access. |
| `src/world/stream.rs` | Migrate lifecycle calls and metrics. Keep selection, budgets, and publication. |
| `src/world.rs` | Remove obsolete raw worker re-exports. |
| `src/world/stream_tests.rs` | Update direct-state assertions to the new owner. |
| `tests/web/terrain.test.mjs` | Cover in-flight cancellation, stale replies, drain, and reuse. |

`crates/terrain/src/job.rs`, `crates/terrain-worker/src/lib.rs`, and browser bindings retain their contracts.
`web/terrain.js` retains its physical single-flight and discard rules.
Startup continues to provide foundations and holes through existing stream data.
Save candidates continue to use distinct world sessions and existing overlap accounting.
Terrain jobs remain transient. No save schema changes are included.
The renderer still uses `prepared_surface()` and existing reservation and publication methods.

## Critical verification

Use small native tile inputs and the real `TerrainTask`. Avoid installed-export fixtures for new execution tests.
Keep new native test names under the `terrain_jobs` filter.

| Check | Required result |
| --- | --- |
| Native yield and completion | A task spans multiple polls, produces one completion, and preserves origin and submesh metadata. |
| Reconciliation and stale requests | Same-session retained work updates its revision. Changed context or session cancels without completion. |
| Reservation transfer | Pending bytes and work counts remain stable through completion, then decrease after prepared work is released. |
| Browser cancellation and drain | In-flight discard stays busy, blocks submission, drains once, and then permits reuse. |
| Browser identity and errors | Stale replies cannot replace a current result. Results are read once. Existing worker errors reach polling. |

Run after implementation:

```powershell
./scripts/check.ps1 -Filter terrain_jobs
./scripts/check.ps1 -WebTest terrain
```

The native command includes formatting and the workspace WASM compile check.
Also run `cargo test --locked -p terrain encoded_job_matches_direct_surface` in the configured Rust toolchain.
The existing admission test and `terrain_source_fingerprints` require installed source exports and explicit `--ignored`.
Report them as deferred unless their fixtures exist and they run explicitly.
Full checks, release builds, and browser acceptance remain deferred until separately requested or agreed.
No check ran while writing this specification. Compilation alone does not prove runtime behavior or source parity.

## Alternatives, risks, and completion criteria

An application-wide terrain service would own session handles and the physical worker globally.
It would widen startup and save candidate changes without improving current single-flight requirements.
Keeping raw worker coordination in `WorldStream` would leave result ownership split.
The selected per-stream executor removes that split with the existing browser transport.

The main risk is confusing logical cancellation with physical worker availability.
Current preparation scheduling remains unchanged. This section adds no concurrent stream scheduler.

Completion requires one lifecycle owner, private raw worker operations, and unchanged section interfaces.
No stream code compares worker IDs or consumes raw worker buffers.
The lifecycle contract and targeted checks pass. The branch remains unmerged until authorized.

## Inspected evidence

Line numbers refer to the base commit named above.

- `src/world/stream.rs:110-117`, `209-210`: Split input and worker ownership.
- `src/world/stream.rs:2086-2092`: Selection cancellation and retained request update.
- `src/world/stream.rs:2337-2378`, `2491-2506`: Poll, ownership, result consumption, and queued submission.
- `src/world/stream.rs:2380-2423`: Selection priority and admission before input construction.
- `src/world/stream.rs:886-898`, `951-976`, `4295-4296`: Reservations and logical job accounting.
- `src/world/stream.rs:1373-1391`: Shared native deadline and driving slice.
- `src/world/terrain.rs:105-140`, `147-190`: Native and browser execution, read, and discard.
- `web/main.js:20`, `web/terrain.js:13-29`: Global browser worker and physical single-flight rules.
- `web/terrain-worker.js:3-11`, `crates/terrain-worker/src/lib.rs:10-34`: Worker evaluation and six result arrays.
- `crates/terrain/src/job.rs:20-50`, `193-209`, `227-313`: Wire bytes, resumable math, and existing equality test.
- `src/presentation/world/source.rs:193-211`: Existing renderer preparation interface.
- `src/world/stream_tests.rs:12-67`, `src/world/terrain_tests.rs:55-57`: Ignored export-dependent tests.
