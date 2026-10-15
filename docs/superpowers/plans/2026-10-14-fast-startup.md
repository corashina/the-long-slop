# Fast Startup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reach the complete garage within five seconds with cached assets and preserve source behavior during continued play.

**Architecture:** Use exported receipts and construction chunks to load required content through one checked asset reader.
Replace complete-world readiness with source generation frontiers. Allocate GPU lights from current controller outputs and receiver influence.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, Rapier 0.36.0, Serde, Python 3.12, JavaScript, WASM, and the existing Docker toolchain.

**Spec:** [Approved startup specification](../specs/2026-10-14-fast-startup-design.md).
**Base:** Master `4052a9c`, including the approved specification.
**Status:** Awaiting plan review and execution-method selection.
**Recommendation:** Native execution. The package, reference, generation, and save changes share interfaces and publication rules.

## Global Constraints

- Use installed Steam build `11140890` as the behavior authority.
- Keep the finite 50,000-section road and its sequential generation rules.
- Prepare the source's 100-section initial road sequence for a new game.
- Measure first-download time separately.
- Keep each layer working before adding the next layer.
- Implement in a separate worktree based on current master.
- Preserve existing CPU, GPU, staging, pending-work, and object limits.
- Preserve source range, intensity, cookies, shadow resolution, and shadow behavior.
- Keep a custom archive format, new rendering backend, quality reduction, and unrelated gameplay changes outside this work.
- Keep `MAX_LIGHTS = 42`, `SHADOW_SLOTS = 100`, two world jobs, 32 pending requests, and 32 MiB staging.
- Keep active CPU/GPU limits at 128 MiB each and asset CPU/GPU limits at 256/512 MiB.
- Add no compatibility reader, migration, unchecked asset path, or readiness shortcut.
- Reuse buffers during updates. Allocate construction records at admission, not during ordinary frame updates.
- Follow AGENTS.md and apply stop-slop to documentation and UI text.
- Preserve unrelated worktrees. Merge and push only when the user requests integration.

## Review Focus

1. A canceled load must not publish into a new session or repeat random draws. Task 4 tests both cases.
2. A frontier between sparse checkpoints must restore without generating the whole road. Task 6 tests section 100 and distant saves.
3. A light visible through a mirror must survive main-view culling and controller changes. Task 2 tests that union.
4. A short encoded file must not cause an unreserved decode allocation. Task 3 tests size, format, and reservation failures.
5. A replaced package or corrupt save must leave the active scene and stored save intact. Tasks 4 and 6 test rejection.

## File boundaries

| Files | Responsibility |
| --- | --- |
| `web/startup-trace.js`; `web/loading.js`; `src/app.rs`; `src/browser.rs` | Phase timing, required work, failure snapshots, and presented readiness |
| `src/lighting/allocation.rs`; `src/world/source_lights.rs` | Native-testable light selection and transactional slot allocation |
| `src/presentation/world/light_receivers.rs`; `lighting.rs`; `shadows.rs`; `world/lamps.rs` | Receiver collection, controller outputs, and GPU assignments |
| `tools/reference/runtime_package.py`; existing exporter and world projection modules | File receipts, prototype plans, package identity, and metadata chunks |
| `src/reference/package.rs`; `prototype_plan.rs`; `store.rs` | Runtime contracts and immutable chunk ownership |
| `web/world-assets.js`; `src/presentation/source_io.rs` | Bounded fetch, checksum checks, and one decoded-byte input path |
| `src/presentation/world/preflight.rs`; `prototype_loader.rs`; `prototypes.rs` | Replace network preflight with exported plans |
| Existing world, construction, presentation, and save consumers | Resolve required records through scoped reference leases |
| `src/world/checkpoints.rs`; `road.rs`; `placement.rs`; `stream.rs` | Prepared ranges, source updates, and replay checkpoints |
| `src/save/schema.rs`; existing capture, validation, and reconstruction modules | Versioned generator snapshots and atomic save replacement |
| `scripts/serve.mjs`; browser profile fixture | Real HTTP caching and repeatable startup measurements |

This is one plan because these owners share the required-region publication gate.
Keep their interfaces consistent across tasks. Add no parallel loading format between layers.

## Execution setup and commands

- [ ] Use using-git-worktrees after plan approval and execution-method selection.
- [ ] Inspect attached worktrees. Create or reuse an isolated checkout on `codex/fast-startup` from current master.
- [ ] Record its base commit. Inspect later master changes before implementing this plan.
- [ ] Prepare ignored source evidence and browser assets through the existing reference workflow.
- [ ] Keep generated output in the implementation checkout. Preserve the primary source library and installed game files.
- [ ] Record baseline failures before editing product code.

Run commands from the implementation checkout in PowerShell:

```powershell
$startupRoot = (Get-Location).Path
$startupSource = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm'
$startupPython = Join-Path $startupSource 'tools/reference/.venv/Scripts/python.exe'
. ./scripts/cargo-target.ps1
$startupTarget = Get-CargoTargetDirectory $startupRoot
```

Use `./scripts/check.ps1 -Test <target>` for a named Rust target.
Use the following commands for exporter and final package checks:

```powershell
& $startupPython -m unittest discover -s tools/reference/tests
& $startupPython -m tools.reference.export --stage runtime
& $startupPython -m tools.reference.export --stage runtime --verify
./scripts/check.ps1
./scripts/build.ps1
node scripts/serve.mjs
```

The server defaults to `http://127.0.0.1:4173`. Use a free port and record it for browser tests.
Task 1 exposes browser fixtures at `/__tests__/` and keeps the real app at `/index.html`.
Keep checks, export, build, and server ownership explicit. Stop only processes started for this work.
Use the existing Docker image and worktree target namespace for ignored installed-export tests:

```powershell
docker run --rm --pull=never --mount "type=bind,source=$startupRoot,target=/workspace,readonly" --mount type=volume,source=tld-wasm-cargo,target=/cargo --mount type=volume,source=tld-wasm-target,target=/target --env "CARGO_TARGET_DIR=$startupTarget" my-summer-slop-toolchain cargo test --locked --test reference_loading -- --include-ignored
```

At the base, the full Rust suite has stale test API errors in `world_source` and `interaction`.
The full browser suite has five HUD fixture failures and one save-bridge failure.
Recheck these facts. Update affected tests during their owning task; report unrelated failures without claiming a green full suite.
Run native and browser suites independently if an earlier check-script gate prevents them from running.
Run formatting checks on touched files. Do not reformat unrelated source files.

## Task 1: Capture startup phases and the reported light failure

**Files:** Create `web/startup-trace.js`, `tests/web/startup-trace.test.mjs`, and `tests/browser/startup-profile.html` with its `.js` script.
Modify `web/main.js`, `web/loading.js`, `scripts/serve.mjs`, `src/app.rs`, `src/browser.rs`, `src/presentation/world/lamps.rs`, and affected VM test fixtures.

**Interfaces:**

- `createStartupTrace(now: () => number): StartupTrace` owns one attempt.
- `begin(stage)`, `end(stage)`, `fetched(record)`, and `fail(error)` record events outside ordinary frame loops.
- `finish({presentedFrames, inputReady})` rejects fewer than two presented frames or false input readiness.
- `snapshot()` returns phase durations, required/received bytes, cache observations, build identity, and failure details.
- `window.gameStartupTrace()` returns that snapshot on request. Store generated traces under ignored `.cache/startup/`.

- [ ] Add a fake-clock test: a phase from 10 to 35 records 25 ms; failure freezes its snapshot.
- [ ] Add a test that rejects `finish({presentedFrames: 0, inputReady: false})` and accepts two complete presented frames.
- [ ] Run `node --test tests/web/startup-trace.test.mjs` and confirm the new assertions fail before implementation.
- [ ] Implement the trace and connect current bootstrap, generation, decode/upload, and pipeline stages.
- [ ] Serve `/__tests__/` from `tests/browser`, with resolved-path containment checks and normal app routing unchanged.
- [ ] Use `/index.html` in the new profile iframe. Keep its observer script under `/__tests__/`.
- [ ] Add exact light-entry and shadow-face counts, available counts, source IDs, controller outputs, and retained overlap to the failure event.
- [ ] Run the trace tests and existing loading/startup tests. Update their VM imports for the new trace module.
- [ ] Capture one release baseline with the normal loading path. Record the reported 14-light scene before changing selection.
- [ ] Preserve that scene's source IDs and outputs in the ignored baseline trace. Derive the regression fixture from those values.
- [ ] Commit as `feat: trace startup phases and light admission failures`.

Pin the trace contract with these assertions in `tests/web/startup-trace.test.mjs`:

```javascript
assert.equal(snapshot.phases.wasm.durationMs, 25);
assert.throws(() => trace.finish({presentedFrames: 0, inputReady: false}));
assert.deepEqual(trace.snapshot(), frozenFailureSnapshot);
```

The profile fixture observes the real app. It must not bypass catalog checks, rendering, or world readiness.
A failed baseline is evidence of the failure, not a completed startup measurement.

## Task 2: Allocate active lights and shadow faces

**Files:** Create `src/lighting/allocation.rs`, `src/presentation/world/light_receivers.rs`, and `tests/light_allocation.rs`.
Modify `src/lighting.rs`, `src/world/types.rs`, `source_lights.rs`, `src/app.rs`, `src/presentation/lighting.rs`, `shadows.rs`, `save.rs`, and `world/lamps.rs`.

**Interfaces:**

- `LightReceiver { bounds: WorldBounds, mask: u32 }` describes an actual required receiver.
- `required_world_lights(frame, receivers: &[LightReceiver], road_enabled: bool, output: &mut Vec<GeneratedId>)` filters controller state and influence.
- `LightKey` distinguishes fixed `LightId` from generated `GeneratedId`.
- `LightRequest { key: LightKey, faces: usize }` requests 0, 1, or 6 shadow faces.
- `LightAssignment` owns session, key, GPU slot, shadow start, and face count.
- `LightPool::reconcile(session: u64, requested: &[LightRequest], retiring: &[LightAssignment]) -> Result<&[LightAssignment], LightCapacityError>` commits all assignments or preserves the previous set.
- `LightCapacityError` identifies entries, faces, or contiguous-range exhaustion with requested/available counts and source keys.

- [ ] Add native tests for 26 disabled fixed lights, one enabled sun, and fourteen enabled shadowed point lights.
- [ ] Assert fifteen GPU entries and 84 shadow faces, within pools of 42 entries and 100 faces.
- [ ] Add true overflow, fragmented six-face ranges, stable assignment reuse, and retained-session overlap tests.
- [ ] Add selection tests for disabled road output, zero intensity, mismatched masks, and a mirror-only receiver.
- [ ] Run `./scripts/check.ps1 -Test light_allocation` and confirm the missing allocator/selection tests fail.
- [ ] Implement native selection and allocation. Move the existing pool constants to this owner and update imports.
- [ ] Separate CPU definition indices from optional GPU assignments in fixed and streamed light bindings.
- [ ] Prepare required shadow pipeline keys through canonical views, without enabling copies of the disabled fixed lights.
- [ ] Collect transformed receiver bounds from visible instances and descriptor metadata for main and active mirror views.
- [ ] Replace the fixed-prefix reservation and camera-frustum-AABB-only selection. Preserve sun cascades and source light properties.
- [ ] Check capabilities and descriptor allocations before heavy scene decoding. Keep valid scenes during rejected replacement.
- [ ] Run native tests, build WASM, and reproduce the captured source case with day/night and vehicle/garage switches.
- [ ] Commit as `fix: allocate source lights from active receiver influence`.

Pin the allocation contract in `tests/light_allocation.rs`:

```rust
assert_eq!(assignments.len(), 15);
assert_eq!(assignments.iter().map(|a| a.faces).sum::<usize>(), 84);
assert_eq!(pool.assignments(), before_failed_reconcile);
```

Expose `LightPool::assignments(&self) -> &[LightAssignment]` for this read-only inspection.

Do not discard an influencing enabled light to fit the pool.
If supported active scenes still exceed the pools, record the exact case and resolve it before marking this task complete.

## Task 3: Use exported receipts and one checked asset read

**Files:** Create `tools/reference/runtime_package.py`, `tools/reference/tests/test_runtime_package.py`, `src/reference/package.rs`, and `src/reference/prototype_plan.rs`.
Create `tests/web/asset-receipts.test.mjs` and `tests/web/http-cache.test.mjs`.
Modify `tools/reference/export.py`, `gltf.py`, `materials.py`, `mips.py`, `src/reference.rs`, `rendering.rs`, `src/presentation/source_io.rs`, and `world/preflight.rs`, `prototype_loader.rs`, `prototypes.rs`.
Modify `web/world-assets.js`, `main.js`, `ui-layout.js`, `audio/clips.js`, `scripts/serve.mjs`, and affected export/loader tests.

**Interfaces:**

- `FileReceipt { file: String, bytes: u32, sha256: String, image_size: Option<[u32; 2]> }` uses a canonical package-relative path.
- Require `image_size` for PNG input. Validate its dimensions before invoking the existing image decoder.
- Keep `SourceHash` for source provenance. Replace runtime dependency entries with `FileReceipt`.
- `FileCatalog { files: Vec<FileReceipt>, dependency_staging_reserve: u32 }` is the exported receipt catalog.
- Build its path lookup once during admission. Charge the lookup and receipt storage to metadata ownership.
- Move `PrototypePlan`, its node/primitive records, and `reservation()` from presentation preflight into `reference::prototype_plan`.
- `build_file_catalog(manifest: dict, output: Path) -> dict` deduplicates receipts and exports the maximum dependency staging reservation.
- `build_prototype_plan(document: dict, receipts: dict) -> dict` emits the current plan fields and checked allocation estimates.
- `readAsset(contentBase, receipt, signal): Promise<Uint8Array>` bounds the response, checks size/hash/header, and returns those same bytes.
- `assetUrl(contentBase: string, path: string): string` is the shared browser URL constructor.
- `SourceIoPlugin::new(content_base: String, catalog: Arc<FileCatalog>) -> Self` configures the existing checked reader.
- Its reader returns verified bytes to existing Bevy decoders. Configure `AssetPlugin` with the same package base.

- [ ] Add exporter tests for shared-file deduplication, byte counts, PNG dimensions, buffer lengths, and checked plan arithmetic.
- [ ] Add loader tests: unused files receive zero fetches; a selected file receives one fetch; corruption fails before decoder invocation.
- [ ] Add declared-size overflow, truncated input, wrong decoded dimensions, and insufficient-reservation assertions.
- [ ] Run the new exporter and browser targets and confirm the missing-contract tests fail.
- [ ] Keep cache tests at the HTTP boundary: create one owned test-package fixture, start the child server, and inspect responses.
- [ ] Emit receipts from final exported files. Preserve existing conservative plan and PNG staging formulas.
- [ ] Replace `HttpWasmAssetReader` whole-body buffering with the bounded `readAsset` bridge in the existing source reader.
- [ ] Route glTF documents, buffers, PNGs, metadata, and project shaders through the receipt-aware reader.
- [ ] Remove repeated hashing in loaders once the common reader verifies their bytes. Keep structural and decoded-size validation.
- [ ] Remove `PrototypePreflight`, its network job state, and the `sourcePrototypePlan`/cancel bridge.
- [ ] Remove `admitWorldAssets`. Configure staging and input sizes from receipts, without payload requests.
- [ ] Preserve job/byte charges through cancellation until the physical load or upload releases ownership.
- [ ] Publish immutable files beneath `assets/runtime/<package_id>/`, keeping their existing logical paths and glTF relative URIs.
- [ ] Include project shaders in the package. Configure Bevy's asset base and browser UI/audio URLs to the same package directory.
- [ ] Compute package ID from canonical core data and receipt hashes before adding the derived content-base field.
- [ ] Serve immutable package paths with long-lived caching. Serve index, code, and WASM with ETags and revalidation.
- [ ] Add real HTTP assertions for immutable headers, unchanged-code 304 responses, and a new package selecting a new URL.
- [ ] Regenerate and verify the installed export. Run exporter tests, asset/cache tests, `reference_loading`, and a release build.
- [ ] Commit as `feat: load source assets once from exported receipts`.

Pin the loader contract in `tests/web/asset-receipts.test.mjs`:

```javascript
assert.equal(fetchCounts.get(unusedReceipt.file) ?? 0, 0);
assert.equal(fetchCounts.get(selectedReceipt.file), 1);
assert.equal(decoderCallsAfterInvalidReceipt, 0);
```

For the HTTP test, use one owned fixture file and a child server on `PORT=0`.
Remove that file, empty owned fixture directories, and that child in cleanup. Do not delete another directory or stop another server.
Keep the single current manifest shape during this task; Task 4 replaces its construction-data transport.

## Task 4: Stream construction records through one reference store

**Files:** Create `src/reference/store.rs`, `src/presentation/reference_chunks.rs`, and `tests/reference_store.rs`.
Extend `runtime_package.py`, `package.rs`, `world_fields.py`, `world_spawn.py`, `export.py`, and their tests.
Modify `src/reference.rs`, `src/reference/world.rs`, its initialization/memory modules, `src/bin/validate-reference.rs`, and native fixtures.
Modify `src/app.rs`, `starting_area.rs`, `starting_area/save.rs`, `src/world.rs`, and these world consumers:
`contents`, `content_bindings`, `content_lamps`, `instances`, `props`, `placement`, `road`, `source_lights`, `startup`, `terrain`, `structure_helpers`, `building_extras`, `changes`, and `stream`.
Modify these spawn consumers: `spawn_rules`, `spawn_contents`, `spawn_initialization`, `spawn_appearance`, `spawn_conditions`, `spawn_lamps`, `spawn_slots`, `spawn_starts`, and `spawn_state`.
Modify presentation reference/material/save consumers and `world/instances`, `lod`, `prototypes`, `runtime`, `source`, and `surface_bindings`.
Modify `src/save/world.rs`, `validate_world.rs`, `web/main.js`, and `web/save-storage.js` where they consume source records.

**Interfaces:**

- `StartupIndex` replaces the monolithic root contract: feature settings, `WorldCore`, startup chunk IDs, receipts, and chunk ownership.
- `WorldCore` holds generation/selection inputs, compact placement templates, prototype bounds, and road-light recipes, without construction graphs.
- `ChunkId(u32)` identifies a `ChunkReceipt { id, input: FileReceipt, dependencies, owners }` in the root catalog.
- `MetadataChunk` holds projected construction, scene, asset, material, initialization, and prototype records for its declared owners.
- `RuntimeBootstrap { index: StartupIndex, startup: Vec<MetadataChunk> }` carries only verified startup chunks into `run`.
- `ReferenceStore::from_bootstrap(bootstrap: RuntimeBootstrap) -> Result<Self, ReferenceError>` installs the initial required closure.
- `ReferenceStore::acquire(owner: &SourceId, session: u64) -> ReferenceAvailability<ReferenceLease>` returns pending, ready, or failed.
- `ReferenceAvailability<T>` has `Pending(ChunkId)`, `Ready(T)`, and `Failed(ReferenceError)` variants.
- Acquisition queues the missing dependency closure. Pending identifies its first unresolved chunk; ready requires the complete closure.
- `ChunkTicket { id: ChunkId, serial: u64 }` identifies one physical metadata request.
- `drain_requests(&mut self, output: &mut Vec<ChunkTicket>)`, `publish_verified(ticket, Arc<MetadataChunk>) -> Result<bool, ReferenceError>`, and `cancel_session(session)` govern publication.
- `ReferenceLease::view() -> ReferenceView<'_>` exposes the core and scoped `record`, `prototype`, `asset`, `material`, and initialization lookups.
- `retire_unused() -> usize` releases unused payloads; `retained_bytes() -> usize` reports owned payload and control allocations.
- `GarageRuntime::new(view: &ReferenceView<'_>)` and construction consumers use the leased records. Numeric generation uses `&WorldCore`.

- [ ] Add exporter tests that close required links, group local reference cycles, and reject cycles between chunk owners.
- [ ] Extend existing runtime field projection from audited consumer field names. Retain full evidence in ignored provenance files.
- [ ] Test that an unused prototype's records and construction assets stay outside the startup closure.
- [ ] Add native tests: two leases charge one payload; an unknown owner fails; a declared unloaded owner returns pending.
- [ ] Assert a canceled ticket cannot publish, including after a new request for the same chunk.
- [ ] Assert canceling one subscriber keeps a shared request alive for another active session.
- [ ] Test an owner selecting a child before a pending load. Resume with the same child and unchanged random draw count.
- [ ] Test that a wrong chunk hash, source stamp, or package catalog entry leaves the active store unchanged.
- [ ] Run the new exporter/store targets and confirm they fail before the replacement exists.
- [ ] Implement field projection and chunk ownership in the existing exporter. Keep compact selection data in `WorldCore`.
- [ ] Reject any chunk whose parse/retained reservation cannot fit the existing budgets. Repair its projection instead of raising limits.
- [ ] Implement the store, leases, verified metadata loader, and session-aware request coalescing.
- [ ] Use existing `SpawnBoundary.initialization.selectors` and continuation state across pending construction loads.
- [ ] Replace direct whole-world scans with scoped lookups or exported numeric indices. Reject undeclared construction dependencies.
- [ ] Adapt consumers together, including save validation and hidden candidates. Do not return empty data for pending chunks.
- [ ] Fetch and verify the startup closure while WASM initializes. Pass the small `RuntimeBootstrap` JSON and root hash into `run`.
- [ ] Publish the compact root at the existing manifest URL. Its identity covers all chunk hashes and complete package receipts.
- [ ] Charge chunks once, retain leases for active/pending owners, and retire unused records through existing session cleanup.
- [ ] Regenerate fixtures and installed output. Reject the previous root contract; remove its complete-graph reader.
- [ ] Run `reference_store`, affected source/save targets, installed-export checks, exporter tests, and a release garage/exit check.
- [ ] Commit as `feat: stream scoped source construction records`.

Pin the store contract in `tests/reference_store.rs` with catalog and chunk fixtures derived from the exporter:

```rust
assert_eq!(payload_bytes_after_two_leases, payload_bytes_after_one_lease);
assert!(!store.publish_verified(canceled_ticket, late_chunk).unwrap());
assert_eq!(spawn_draws_after_resume, spawn_draws_after_selection);
```

Keep fixture builders in the test target. Move one into `tests/common` only when another critical target needs it.

Keep chunks package-relative. Their payloads contain source identity, not a circular hash of their own final package path.
The root catalog binds each chunk hash into package identity.
Activate this contract and its consumers together. Do not publish a half-migrated package or an eager-load fallback.

## Task 5: Prepare the source initial range and continue generation

**Files:** Modify `tools/reference/world.py`, its tests, `src/reference/world.rs`, `src/world/road.rs`, `placement.rs`, `stream.rs`, `metrics.rs`, and `src/app.rs`.
Create `tests/world_frontiers.rs`; update affected existing world and loading tests.

**Interfaces:**

- Export `gen_at_start = 100`, `gen_in_one_frame = 1`, `generate_road`, and the source update threshold of 10.
- `RoadIndex::prepare_until(target: u32, work_limit: usize) -> bool` replaces complete-road `prepare`.
- `RoadIndex::prepared_segments()` remains the generated frontier, bounded by 50,000.
- `WorldStream::initial_range_ready() -> bool` describes initial coverage, not total-road completion.
- Keep `window_ready()` and `supports(swept)` as complete current-region checks. Remove the global `indices_ready` dependency.
- Store the source update counter separately from random and road position state.

- [ ] Add a test that the initial target stops at 100 and enables required asset work without preparing section 101.
- [ ] Test the source counter: check `counter > 10` before reset/generation, then increment as the source does.
- [ ] Compare source samples at sections 0, 99, 100, 255, 256, and the finite end using separate replay state.
- [ ] Test entering an unresolved collision region: preparation continues, but physics cannot enter unsupported space.
- [ ] Test recovery beyond the prepared road and nearest-supply lookup without accepting a partial global search.
- [ ] Run `./scripts/check.ps1 -Test world_frontiers` and confirm the complete-index gate fails the new tests.
- [ ] Implement prepared ranges, initial coverage, source update batching, and bounded continuation.
- [ ] Retain affordable numeric placement/recovery tables from `WorldCore`; request detailed construction only for selected owners.
- [ ] Remove end-of-road gates from external jobs, terrain, collision support, recovery, and loading progress.
- [ ] Run frontier and affected world tests. Build and verify garage exit and driving beyond the initial required region.
- [ ] Commit as `feat: start from the source initial generation range`.

Pin the frontier contract in `tests/world_frontiers.rs`:

```rust
assert!(road.prepare_until(100, 100));
assert_eq!(road.prepared_segments(), 100);
assert!(stream.initial_range_ready());
assert!(!stream.supports(unpublished_swept_bounds));
```

An initial target of 100 is a generation target, not a shortened world or a limit on later prepared sections.
Do not use independent segment seeds or a large batch that silently prepares the complete road during startup.

## Task 6: Save generator frontiers and restore distant regions

**Files:** Create `src/world/checkpoints.rs` and `tests/world_checkpoints.rs`.
Modify `src/world.rs`, `random.rs`, `road.rs`, `placement.rs`, `stream.rs`, `src/save/schema.rs`, `validate.rs`, `validate_world.rs`, `world.rs`, `src/starting_area/save.rs`, and `src/presentation/save.rs`.
Update save fixtures and affected schema/roundtrip/browser tests.

**Interfaces:**

- `WorldGenerationSnapshot` stores generator version 1, seed, road frontier, sparse blocks/checkpoints, placement state, and source update counter.
- `RoadIndexSnapshot` includes the live frontier, including an unfinished 256-section checkpoint block.
- `PlacementIndexSnapshot` includes selection pools, random state, draw count, numeric index state, and replay checkpoints.
- `WorldStream::generation_snapshot() -> WorldGenerationSnapshot` captures state at a stable save boundary.
- `WorldStream::restore_generation(saved: &WorldGenerationSnapshot) -> Result<(), ReferenceError>` validates before publication.
- Add the snapshot to `WorldSnapshot` and increment `SAVE_SCHEMA_VERSION` from 3 to 4.
- Use existing lossless save helpers. Encode the 56-value subtractive random state as a checked sequence without another dependency.

- [ ] Test capture at road frontier 100, restore, and generate the next section; compare with uninterrupted source progression.
- [ ] Test restored building pools, draw counts, Unity random boundaries, and the source update counter.
- [ ] Reject unordered/out-of-range blocks, wrong seed/version/package, non-finite state, and malformed 56-value random arrays.
- [ ] Test rejection before candidate commit: active physics, items, controller state, and stored save bytes remain intact.
- [ ] Run `world_checkpoints` and save targets and confirm the missing snapshot/schema checks fail.
- [ ] Implement snapshot capture/validation and direct index restoration. Rebuild local replay buffers from the saved sparse state.
- [ ] Restore required metadata leases before reconstructing the saved region. Preserve existing object changes and spawn history.
- [ ] Request the saved local region without replaying sections 0 through the saved frontier.
- [ ] Test garage and section-40,000 save fixtures, including changed items and a pending replacement canceled before commit.
- [ ] Reject schema 3 with the existing incompatible-save path. Keep its stored record intact; add no migration.
- [ ] Update existing browser save fixtures to use `/index.html`, `/__tests__/` scripts, and root-relative app module imports.
- [ ] Run native save/ checkpoint tests, installed-export checks, and the real browser save/load fixture.
- [ ] Commit as `feat: restore world generation from saved checkpoints`.

Pin the checkpoint contract in `tests/world_checkpoints.rs`:

```rust
assert_eq!(restored_next_segment, uninterrupted_next_segment);
assert_eq!(restored_draw_count, saved_draw_count);
assert_eq!(active_scene_after_rejected_save, active_scene_before_load);
assert_eq!(SAVE_SCHEMA_VERSION, 4);
```

## Task 7: Gate readiness and pipeline work on required content

**Files:** Modify `src/app.rs`, `src/browser.rs`, `src/presentation/materials.rs`, `lighting.rs`, `mirrors.rs`, `world/pipelines.rs`, `world/runtime.rs`, `web/main.js`, `loading.js`, and affected startup/loading tests.

**Interfaces:**

- Keep required content as session-owned sets of record chunks, assets, scene bindings, and pipeline tickets.
- `gameLoadingProgress` reports the current required generation range, not a 50,000-section completion target.
- Extend the existing loading snapshot with required/complete files and bytes. Serialize detailed traces only on request or transition.
- Connect `createStartupTrace.finish` from Task 1 after the complete ready scene has presented twice.

- [ ] Add tests that unused prototype/material variants cannot block ready state or trigger startup requests.
- [ ] Assert Continue stays disabled until required collision, controls, visible effects, and pipelines are ready.
- [ ] Test a canceled attempt's late ready event, repeated New Game/Load, and a pending mirror-only resource.
- [ ] Run the startup/loading targets and confirm the old whole-world/warmup gates fail the tests.
- [ ] Start root fetching and WASM initialization in parallel. Share their validated startup package identity.
- [ ] Request pipeline/material variants for required startup content. Admit later variants before publishing their regions.
- [ ] Include reachable controller variants for startup owners. Use canonical shadow keys without reserving disabled-light GPU entries.
- [ ] Preserve visible mirror, shadow, plate, sky, and postprocess readiness. Do not postpone visible effects beyond measured readiness.
- [ ] Replace full-catalog labels with required work and resource waits. Freeze the exact phase/error data after failure.
- [ ] Run browser/native loading checks, package release WASM, and verify a complete ready scene with input after Continue.
- [ ] Commit as `perf: prepare only required startup content and pipelines`.

Pin the readiness contract in the existing startup tests:

```javascript
assert.equal(unusedPrototypeFetches, 0);
assert.equal(continueButton.disabled, true); // Required collision or visible effects remain pending.
assert.equal(trace.snapshot().presentedFrames, 2);
```

## Task 8: Run acceptance checks and record delivery evidence

**Files:** Complete the browser profile fixture from Task 1.
Modify `scripts/check.ps1`, `README.md`, and create `docs/research/2026-10-14-fast-startup-validation.md`.

**Interfaces:** The profile fixture reads `window.gameStartupTrace()` from the actual release app and exports attempt records.
It labels priming, cached reload, first download, hard refresh, and saved load as separate measurements.

- [ ] Add a profile-result test that rejects a 5,001 ms cached attempt, missing presented frames, or missing input readiness.
- [ ] Run it before adding result validation, then implement the checks and confirm it passes.
- [ ] Run exporter verification, full project checks, installed-export tests, and a fresh release package build.
- [ ] Compare full-suite failures against the recorded baseline. Fix regressions in this branch and list unrelated failures.
- [ ] Prime the current package once. Record five normal cached reloads on the user's machine, browser, and viewport.
- [ ] Require all five complete starts at or below 5,000 ms. Report their median and maximum.
- [ ] Verify cache behavior from HTTP/resource timing, with code/WASM revalidation and immutable package reuse.
- [ ] Measure first download and hard refresh separately. Include transferred bytes and phase durations.
- [ ] Measure cached garage and distant saves from Load request to committed ready scene; target 5,000 ms for the fixed fixtures.
- [ ] Verify garage exit, initial-region crossing, day/night, switches, vehicle lights, mirror receivers, and repeated scene replacement.
- [ ] Check peak CPU/GPU/staging charges and cleanup. Confirm unused catalog records and files did not load during startup.
- [ ] Record commands, package/build identities, machine/browser details, traces, source checks, and remaining limits in the validation document.
- [ ] Complete a fresh whole-branch review through the selected execution workflow. Repair its findings and repeat affected checks.
- [ ] Commit as `docs: record measured startup and streaming validation`.

Pin the acceptance result in `tests/web/startup-trace.test.mjs`:

```javascript
assert.equal(cachedAttempts.length, 5);
assert.ok(cachedAttempts.every(attempt => attempt.durationMs <= 5000));
assert.ok(cachedAttempts.every(attempt => attempt.inputReady && attempt.presentedFrames >= 2));
```

Do not mark the plan complete while the reported light case or the cached-start gate still fails.
Do not replace measured failures with a compilation result, synthetic ready flag, reduced scene, or a larger resource limit.
Leave the branch and worktree available for user testing and requested integration.

## Plan review and execution handoff

Read the approved specification and this plan together.
Native execution keeps the shared package/store/generation interfaces in one session and ends with a fresh whole-branch review.
Subagent-driven execution uses a fresh implementer and reviewer for each task, with greater context and review cost.
Implementation starts after the user reviews this plan and selects the execution method.
