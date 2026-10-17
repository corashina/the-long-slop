# Local Startup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Start the complete cached garage within 30 seconds, with stable player and vehicle physics.

**Architecture:** Prepare the generated source prefix and required scene before play. Load checked texture batches with bounded ownership. Reuse the prepared first session and gate input on presented frames.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, Rapier 0.36.0, existing Python exporter, JavaScript bridge, Docker toolchain.

**Spec:** `docs/superpowers/specs/2026-10-17-local-startup-design.md`

## Global Constraints

- Preserve build `11140890` geometry, texture detail, lighting, interactions, and source generation order.
- Generate the initial 100 road sections before first play.
- Continue one section per ordinary application update until the finite source end.
- Do not reduce texture detail, source distances, object counts, effects, or graphics quality to meet the target.
- Do not increase resource limits to pass a scene that has not been scoped to required content.
- Retain the current shared limits: two jobs, 32 pending requests, and 32 MiB of staging.
- Retain the current world and asset CPU and GPU limits.
- Use a maximum decoded payload of one MiB and at most 32 chunk entries per batch.
- Increment save schema 5 to 6 for the generation frontier and partial index contract.
- Add no migration or old-format runtime path.
- Confirm two presented frames for the complete required scene before enabling input.
- Require each complete startup to take at most 30 seconds on this machine.
- Preserve completed fixes in `ecd2ed3`. Do not change the primary checkout or its generated assets.

## Review Focus

1. Stationary frontier growth must refresh selection. Task 2 tests revision and publication.
2. Partial empty queries must differ from complete empty queries. Task 1 tests coverage status.
3. Cancelled submitted uploads must retain memory. Task 4 tests acknowledgement, stale receipts, and shared owners.
4. Distant saves must restore without full generation. Task 1 tests partial checkpoints and damaged inputs.
5. First New Game cancellation must retain the scene. Task 5 tests permit discard and retry.

## Workspace and verification

Use the existing managed worktree on `codex/startup-physics`:
`C:/Users/Corashina/.codex/worktrees/startup-physics/the-long-drive-wasm`.

Use the existing Docker image `my-summer-slop-toolchain` and volumes `tld-wasm-cargo` and `tld-wasm-target`.
Run listed Cargo commands inside the worktree-mounted container. Set the target directory explicitly.
Use `/target/startup-physics` for native tests.
Use `/target/cb0c007f45bd6cbace728f511729a1bd4fc4bff8a92bbbeed110af46adcfe7d8` for release WASM.
Mount installed reference assets directly for ignored tests. Docker cannot follow Windows asset junctions.
Verify the installed Python environment path before execution.

Do not export into existing asset junctions. Stage a private package and bind it only to this worktree.
Preserve the original package for comparison. Never delete a resolved junction target.
Use normal `git add`. Check native exit codes before each commit.
Dispatch one fresh implementer and one fresh reviewer per task. Keep dependent writes sequential.

---

### Task 1: Checked generation frontiers and schema 6

**Files:**
- Modify: `tools/reference/world.py`, `src/reference/world.rs`, `src/reference/world/memory.rs`.
- Modify: `src/world.rs`, `src/world/road.rs`, `src/world/placement.rs`, `src/world/stream.rs`.
- Modify: `src/save/schema.rs`, `src/save/validate_world.rs`, and affected snapshot constructors.
- Create: `src/world/generation.rs`, `tests/world_frontiers.rs`.
- Test: `tools/reference/tests/test_world.py`, `tests/web/save-storage.test.mjs`.

**Interfaces:**
- Export required `genAtStart` and `genInOneFrame` in `WorldRoadReference`. Validate source values 100 and 1.
- Add `GenerationCoverage { road_end: u32, building_end: u32, revision: u64 }` and `QueryStatus::{Pending, Complete}`.
- Query completeness refers to the explicit generated prefix. It makes no claim about unknown future geometry.
- Add `RoadIndex::coverage() -> u32` and `PlacementIndex::coverage() -> u32`.
- Add `RoadIndex::capture_index() -> RoadIndexSnapshot` and `restore_index(&mut self, saved: &RoadIndexSnapshot) -> Result<(), ReferenceError>`.
- Add matching `PlacementIndexSnapshot`, `capture_index`, and `restore_index` methods.
- Snapshots include exact sequence state, sparse checkpoints, prepared block bounds, and indexed helper coverage.
- Add `GenerationSnapshot { generator_contract: u32, package_id: String, roads: RoadIndexSnapshot, placements: PlacementIndexSnapshot }`.
- Store `WorldSnapshot.generation: GenerationSnapshot`. Retain existing seed, changes, and spawn owners.
- Spatial queries consume coverage and return status beside existing output buffers. Remove old signatures after caller migration.

- [ ] **Step 1:** Add failing critical checks.
  `incremental_matches_sequential` compares geometry, road state, building IDs, pools, random state, and draw counts at 100, 255, 256, 257, and a distant checkpoint.
  `empty_query_requires_coverage` asserts unresolved empty queries return `Pending`; checked empty queries return `Complete`.
  `partial_checkpoint_round_trip` checks identical continuation after restoring a partial block.
  `distant_restore_is_bounded` asserts restore does not prepare 50,000 sections.
  `invalid_frontier_is_rejected` covers nonfinite bounds, checkpoint order/counts, generator contract, and package identity.
  Assert schema 5 rejection preserves stored bytes.
- [ ] **Step 2:** Run `cargo test --locked --test world_frontiers` and affected exporter tests. Confirm failure.
- [ ] **Step 3:** Implement count export, checked snapshots, coverage-aware queries, and schema 6.
  Reuse separate sequential and replay states. Preserve spacing draws and initialization ownership.
  Keep full startup preparation until Task 2 changes readiness. Update the private package before installed checks.
- [ ] **Step 4:** Run frontier, exporter, and storage checks. Require matching continuation and unchanged checkpoint limits.
- [ ] **Step 5:** Commit as `feat: persist checked source generation frontiers`.

### Task 2: Required scene, local collision, and pending recovery

**Files:**
- Modify: `src/world/stream.rs`, `src/world/metrics.rs`, `src/starting_area.rs`, `src/starting_area/recovery.rs`.
- Modify: `src/app.rs`, `src/presentation/world/runtime.rs`, `src/presentation/save.rs`.
- Create: `src/world/required.rs`.
- Test: `tests/world_frontiers.rs`, `tests/startup_physics.rs`.

**Interfaces:**
- Consume Task 1 coverage and query status.
- Add `RequiredContent` with session/request identity, coverage, selection revision, required source identities, and collision ranges.
- Add `WorldStream::advance_generation(&mut self)` for one ordinary Update advance after initial preparation.
- Add `WorldStream::required_ready(&self) -> bool` for main, candidate, Load, and recovery preparation.
- Retain `WorldStream::supports(WorldBounds) -> bool`; require coverage and published collision.
- Recovery distinguishes pending preparation from failure. Partial nearest-supply results cannot claim global completeness.

- [ ] **Step 1:** Add failing readiness and collision checks.
  Assert initial generation stops at 100; repeated prepare calls do not perform ordinary Update advances.
  Assert each ordinary Update advances one section with identical source output.
  Assert stationary frontier growth changes selection revision and publishes newly required content.
  Assert unresolved empty startup candidates cannot complete selection.
  Assert geometry, lights, casters, effects, controls, interactions, and collision are published before required readiness.
  Assert swept player and vehicle bounds stop before unresolved collision.
  Assert pending recovery retains poses and cannot fail the stream or return a partial global nearest result.
- [ ] **Step 2:** Run frontier checks. Confirm the new readiness checks fail.
- [ ] **Step 3:** Implement collision-first required selection and frontier revision handling.
  Reuse source culling and light influence rules. Keep distant preparation outside the startup gate.
  Migrate all `indices_ready` consumers. Remove the obsolete global readiness metric.
- [ ] **Step 4:** Run frontier tests and `cargo test --locked --test startup_physics -- --ignored` with direct installed asset mounts.
  Build WASM. Confirm required preparation precedes full finite generation. Keep later texture failures visible.
- [ ] **Step 5:** Commit as `feat: prepare the required world region before play`.

### Task 3: Checked texture batch package and loader

**Files:**
- Modify: `tools/reference/lighting.py`, `tools/reference/runtime_package.py`, `tools/reference/export.py`.
- Modify: `src/reference/rendering.rs`, `src/reference/world/memory.rs`, `src/presentation/mod.rs`, `src/presentation/lighting.rs`.
- Create: `src/reference/texture_batch.rs`, `src/presentation/texture_batches.rs`.
- Create: `tools/reference/tests/test_texture_batches.py`, `tests/texture_batches.rs`.

**Interfaces:**
- Add `LightingTextureRecord.batches: Vec<TextureBatchReference>` beside checked raw chunks.
- Each reference records descriptor receipt, decoded bytes, largest raw input bytes, and region count.
- Canonical content-addressed `.stexbatch` descriptors contain source identity, texture dimensions, ordered receipts, mip/row regions, and decoded offsets.
- Add typed asset `SourceTextureBatch { regions: Arc<[TextureBatchRegion]>, payload: Arc<Vec<u8>> }`.
- Add `SourceTextureBatchLoader` and `SourceTextureBatchSettings { descriptor_bytes: usize, decoded_bytes: usize }`.
- Put shared descriptor validation in the reference module for native tests.
- Reuse `LoadContext::read_asset_bytes()` and checked source IO. Do not add a fetch path.

- [ ] **Step 1:** Add failing batch contract checks.
  Assert deterministic addresses, exact 1 MiB and 32-entry boundaries, grouped tail mips, and identical reconstructed bytes.
  Reject missing, duplicate, reordered, overlapping, oversized, truncated, and incorrect-dimension regions.
  Reject bad receipts, SMIP headers, offsets, and totals before publication.
- [ ] **Step 2:** Run `python -m unittest tools.reference.tests.test_texture_batches` and `cargo test --locked --test texture_batches`. Confirm failure.
- [ ] **Step 3:** Implement descriptors, package identity, and typed loading.
  Bound descriptor input before parsing. Allocate one output payload. Read and drop raw inputs in order.
  Preserve source/chunk identity in errors. Create no temporary GPU Images.
  Retain current world chunk consumption until Task 4 replaces it. Keep fixed-image loading unchanged.
- [ ] **Step 4:** Run affected checks. Produce and validate the private installed package.
  Confirm unchanged source pixels, complete byte coverage, new catalog receipts, and untouched primary assets.
- [ ] **Step 5:** Commit as `feat: export and load checked texture batches`.

### Task 4: Bounded batch upload ownership

**Files:**
- Modify: `src/presentation/materials/world_textures.rs`, `src/presentation/materials/world_texture_upload.rs`.
- Modify: `src/presentation/materials/primary.rs`, `src/presentation/materials.rs`, `src/presentation/world/runtime.rs` and affected cancellation callers.
- Create: `src/world/texture_batch_ownership.rs`.
- Test: `tests/texture_upload.rs`.

**Interfaces:**
- Consume Task 3 strong asset handles and validated immutable regions/payload.
- Add `BatchState::{Queued, Loading, Ready, Submitted, Complete, Failed}`.
- Add `BatchIdentity { session: u64, request: u64 }` to owners and completion receipts.
- Share one current slot and at most one prefetch slot across active and candidate sessions.
- Cancelled submitted owners retain destination, payload, reservation, and slot until acknowledgement.
- Derive reservations from descriptor input, largest raw input, decoded payload, handles, and submission overlap.
- Reserve pair memory before loading. Launch requests within available global job capacity; do not require two free job slots.
- Keep `prepare_world_deferred()` as the accounting boundary for jobs, pending requests, staging, shared bytes, and replacement overlap.

- [ ] **Step 1:** Add failing ownership checks.
  Assert promotion order, pair admission, two-job maximum, and 32 MiB staging maximum.
  Assert submitted cancellation retains bytes until completion. Stale session/request receipts cannot release a new owner.
  Assert cancellation preserves another session's strong handle and shared bytes count once.
  Assert permanent admission failure differs from a wait that an in-flight owner can resolve.
- [ ] **Step 2:** Run `cargo test --locked --test texture_upload`. Confirm failure.
- [ ] **Step 3:** Integrate typed loading and GPU writes for every batch region.
  Share payload without cloning bytes. Keep the single decoded Vec behind Arc ownership. Register one callback after existing preparation-owner submission.
  Release CPU data only after validated acknowledgement. Update fork, cancel, drain, release, retirement, and candidate accounting together.
  Remove obsolete world chunk-to-Image loading. Keep fixed-image loading.
- [ ] **Step 4:** Run loader and ownership checks. Build WASM and test installed startup plus cancellation.
  Confirm counters stay within unchanged caps. Admission errors must include source, stage, requested bytes, and available bytes.
- [ ] **Step 5:** Commit as `feat: upload world texture batches with bounded ownership`.

### Task 5: First-session permit and presented-frame readiness

**Files:**
- Modify: `src/app/save.rs`, `src/app.rs`, `src/presentation/view_frame.rs`.
- Create: `src/save/fresh_permit.rs`, `tests/fresh_session.rs`.
- Test: `tests/web/save-session.test.mjs`, `tests/web/save-bridge.test.mjs`.

**Interfaces:**
- Add `FreshSessionPermit { request_id: u32, session: u64, generation: u32 }`.
- Bind it to `WorldFrame.request.session` and `SourceSceneEpoch.0`. Keep the numeric browser token API.
- Pending preparation holds a fresh permit or the existing replacement candidate.
- Add `SourceSubmissionAcks::main_token() -> u32`. Observe tokens recorded only after `state.presented`.
- Replace Update counting with two distinct submitted tokens after complete readiness, bound to the current scene.

- [ ] **Step 1:** Add failing permit and submission checks.
  Assert first prepare creates no runtime/presentation. Discard retains the scene and permits retry.
  Assert changed session/generation and reused tokens reject commit.
  Assert first commit preserves scene/readiness, starts paused, and clears input and elapsed time.
  Assert later New Game/Load retain replacement validation and failure preservation.
  Assert two Updates without submission cannot enable input. Readiness loss or scene replacement resets the baseline.
- [ ] **Step 2:** Run `cargo test --locked --test fresh_session` and affected Node tests. Confirm failure.
- [ ] **Step 3:** Implement permit and submission gates. Remove duplicate first-session preparation.
  Preserve Continue/pointer-lock transitions and pause guards. Renderer preparation cannot advance paused simulation.
- [ ] **Step 4:** Run permit, save lifecycle, and installed spawn checks. Build WASM and test first New Game and cancellation through the menu.
- [ ] **Step 5:** Commit as `fix: reuse the prepared first session and await presented frames`.

### Task 6: Browser timing and physics acceptance

**Files:**
- Create: `docs/superpowers/verification/2026-10-17-startup.md`.
- Modify demonstrated bottlenecks only through a scoped implementer dispatch and review.

**Interfaces:**
- Use `gameStartupTrace()`, `measureWorld()`, `gameState`, and actual menu/input controls.
- `gameStartupDone()` includes failure. Require complete scene, no failure, presented frames, and input response.
- The trace ends at `gameReady()`. Add New Game click-to-commit and first input timing separately; exclude menu dwell.

- [ ] **Step 1:** Run exporter critical checks, all default native tests, installed spawn checks, and all web tests.
  Run `cargo fmt --all -- --check` and standard locked release WASM build. Generate bindings with wasm-bindgen 0.2.129.
- [ ] **Step 2:** Prime the private package. Measure five normal cached browser starts with the standard build.
  Include first New Game commit and input response. Require every result at most 30 seconds.
  Record every result, median, maximum, phase timings, and peak resources. Measure first downloads and first shader preparation separately.
- [ ] **Step 3:** Verify walking, jumping, garage exit, car settling, driving, and the initial generation boundary.
  Pause during movement and mid-air. Compare absolute poses, retained dormant poses, and step counts across a pause interval.
  Resume without accumulated simulation steps or a pose jump.
- [ ] **Step 4:** Verify repeated New Game, garage Load, distant Load, and cancellation during batch preparation.
  Confirm the valid current scene survives replacement failure and cancellation.
- [ ] **Step 5:** Reproduce failures and dispatch scoped fixes. Repeat affected checks and invalidated timing runs.
  Never waive the target through larger caps, changed quality, partial scenes, or synthetic readiness.
- [ ] **Step 6:** Record evidence and limitations. Commit as `docs: record startup and physics acceptance`.
  Run whole-branch review and resolve material findings before proposing integration.
