# Renderer readiness implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give one renderer owner the complete readiness proof and its invalidation rules without changing rendering behavior.

**Architecture:** Add a pure `RendererReadiness` and one Bevy adapter over the existing shared graph storage. Producers supply fixed observations. App code prepares content and starts gameplay after the adapter reports readiness.

**Tech Stack:** Rust 2024, Bevy 0.19.1, wgpu 29.0.4, existing atomics and mutex storage, native unit tests, and WASM compilation.

**Spec:** [Approved renderer readiness specification](../specs/2026-10-19-renderer-readiness-design.md).

Status: Awaiting written-plan review. Implementation is not authorized by this plan alone.
Date: 2026-10-19.
Worktree: `C:/Users/Corashina/.codex/worktrees/codebase-architecture/the-long-drive-wasm`.
Branch: `codex/codebase-architecture`.
Starting commit: `8ce8b0c`.
Execution method: Subagent-driven development, selected by the user.

## Global Constraints

- Steam build `11140890` and its decompiled behavior remain authoritative.
- The scope remains one complete Lada garage experience.
- App code retains content preparation, failure reporting, and gameplay start.
- Shader changes, visual changes, performance changes, and live mirrors are outside this section.
- Do not add per-frame locks, queues, messages, vectors, or callback registrations beyond the current validation boundaries.
- Diagnostic serialization remains an explicit request.
- Preserve current startup and resize validation boundaries. Do not require a new GPU wait for every later frame.
- Terminal failures retain the first error. Restart cannot clear a terminal device or renderer failure.
- Remove old readiness methods and all compatibility exports.
- Full checks, release builds, and browser acceptance remain deferred.

Execute the approved sections in this order: save, startup, terrain, pipeline ownership, then readiness.
Keep shared production edits serial. Recheck the earlier sections before editing their shared files.
Keep changes unmerged. Changes on `master` are outside this work.

## Review Focus

Each case maps to one named native test group below.

1. An absent resource or prerequisite must prevent readiness. Test `all_current_prerequisites_block_readiness`.
2. Old identities or probe publications must not satisfy a new proof. Test `stale_identities_and_publications_are_rejected`.
3. Submission, history commit, completion, and validation must remain distinct. Test `submission_history_completion_and_validation_are_distinct`.
4. Duplicate or old tokens must not count as two fresh submissions. Test `two_fresh_submissions_survive_no_invalidations`.
5. Restart, resize, motion, bypass, and rebase must preserve their current policies. Test `invalidation_preserves_capture_and_history_contracts`.

## File map

Paths are relative to the stated worktree. This map includes all current completion-state callers.

| File | Change |
| --- | --- |
| `src/render_readiness.rs` | Create pure state, observations, identities, policy, and fixed proof storage. |
| `src/render_readiness/tests.rs` | Create five critical native test groups and their local value fixture. |
| `src/presentation/readiness.rs` | Create shared owner, Bevy access, validation gate, errors, and observation registration. |
| `src/lib.rs` | Register pure state for WASM and native unit tests. |
| `src/render_scale.rs` | Keep native unit-test access and reuse the existing enum without another scale type. |
| `src/reference.rs` | Remove the obsolete loading-dispatch module registration. |
| `src/reference/loading_dispatch.rs` | Delete after moving policy and critical assertions. |
| `src/save/fresh_permit.rs` | Remove `PresentedFrames`. Retain `FreshSessionPermit` behavior. |
| `tests/fresh_session.rs` | Move presentation assertions to pure tests. Retain permit tests and their ignore flag. |
| `src/world/stream_tests.rs` | Migrate `complete_probe_union_preserves_revision_and_fresh_cycles` to the new proof API. |
| `src/presentation/mod.rs` | Register the adapter module. |
| `src/app.rs` | Admit shared storage, migrate scale errors, content checks, readiness consumption, and loading reports. |
| `src/presentation/frame_graph.rs` | Retain draw audits. Replace completion owner, observations, diagnostics, and control-memory accounting. |
| `src/presentation/source_lighting.rs` | Remove duplicate completion resource, status, gate, reducer, and error registration. Retain lighting preparation. |
| `src/presentation/view_frame.rs` | Supply identity, call loading policy, migrate warmup status and restart invalidation. |
| `src/presentation/source_effects.rs` | Supply effect observations after history commit. Keep history operations and target ownership. |
| `src/presentation/lighting_scene.rs` | Replace duplicate readiness resource and failure writes. Preserve activation timing. |
| `src/presentation/camera_depth.rs` | Migrate terminal failure reads and writes. |
| `src/presentation/caster.rs` | Migrate failure arguments, reads, and writes. |
| `src/presentation/clouds.rs` | Migrate failure reads and writes. |
| `src/presentation/shadow_atlas.rs` | Migrate failure reads and writes. |
| `src/presentation/shadow_collector.rs` | Migrate failure reads and writes. |
| `src/presentation/shadow_queue.rs` | Migrate failure reads and writes. |
| `src/presentation/shadow_scene.rs` | Migrate failure reads and writes. |
| `src/presentation/source_sky.rs` | Migrate failure reads and writes. |
| `src/presentation/surface_graph.rs` | Migrate failure reads and writes. |
| `src/presentation/surface_pass.rs` | Migrate failure reads and writes after pipeline-owner changes. |
| `src/presentation/surface_views.rs` | Migrate failure reads and writes. |
| `src/presentation/transparent.rs` | Migrate failure reads and writes. |

`postprocessing/chain.rs`, the vendor render loop, shader code, pipeline ownership, and save schema receive no behavior changes.
Existing graph audit records stay fixed-size. Keep their diagnostic values and explicit serialization route.

## Task 1: Migrate the complete renderer proof atomically

This is one independently working task. Partial owner migration would leave duplicate decisions or broken callers.

### Interfaces

Pure types below live in `src/render_readiness.rs`. All visibility is `pub(crate)` unless a field stays private to the owner.
Use `std::sync::Arc` for retained terminal text. Derive `Clone`, `Copy`, `Debug`, and equality for fixed value types.
`RendererStatus` is cloneable and comparable but cannot be `Copy` because it contains an `Arc`.

```rust
struct ContentKey { session: u64, selection_revision: u64 }
enum ContentPreparation { Pending(ContentKey), Prepared(ContentKey) }
struct RenderIdentity {
	session: u64, scene: u32, native_extent: [u32; 2],
	scene_extent: [u32; 2], scale: SceneScale,
}
struct ReadinessRequest {
	render: RenderIdentity, selection_revision: u64,
	probe_mask: u32, history_revision: u32,
}
enum LoadingMode { Cadenced, Full, Bypass }
struct LoadingDecision { captures_allowed: bool, restart_scene: bool }
enum RendererStatus { Loading, Ready, Failed(Arc<str>) }
enum Invalidation { Content, Scene, Extent, Scale, History, Failure }
struct Validation { requested: u32, completed: u32, validated: u32 }
struct ProbeObservation {
	pipelines_ready: bool, published: bool, validation: Validation,
}
struct GraphObservation {
	identity: RenderIdentity, frame: u32, token: u32,
	sky: bool, world_mask: u16, display_encoded: bool, submitted_token: u32,
	targets_ready: bool, lighting_textures: u32, lighting_gpu_textures: u32,
	surface_ready: bool, camera_ready: bool, sky_ready: bool,
	cloud_ready: bool, caster_ready: bool,
	probes: [ProbeObservation; 2], validation: Validation,
}
struct EffectsObservation {
	identity: RenderIdentity, history_revision: u32, frame: u32, token: u32,
	pipelines_ready: bool, inputs_ready: bool,
	encoded_token: u32, submitted_token: u32, committed_token: u32,
	validation: Validation,
}
```

`Validation` passes only when `requested != 0`, `completed >= requested`, and `validated >= requested`.
Requested generations identify a validation boundary, not the main render token.
Probe publication flags never replace the fresh-capture arrays.
Disabled probes bypass publication requirements, but both probe pipeline flags must remain ready.
Keep `PROBE_COUNT = 2` authoritative in `lighting_scene.rs`. Assert its equality with the pure array length at compilation.

The pure owner exposes these exact methods:

```rust
impl RendererReadiness {
	fn content_checked(&mut self, content: ContentPreparation);
	fn snapshot_loading(&mut self, request: ReadinessRequest,
		frame: u32, bypass: bool) -> LoadingDecision;
	fn allows_frame(&self, identity: RenderIdentity, frame: u32) -> bool;
	fn capture_started(&mut self, request: ReadinessRequest, frame: u32, probe: usize);
	fn publication_validated(&mut self, request: ReadinessRequest, probe: usize);
	fn observe_graph(&mut self, observation: GraphObservation);
	fn observe_effects(&mut self, observation: EffectsObservation);
	fn invalidate(&mut self, cause: Invalidation);
	fn fail(&mut self, error: String);
	fn failure(&self) -> Option<Arc<str>>;
	fn graph_ready(&self, request: ReadinessRequest) -> bool;
	fn check(&mut self, request: ReadinessRequest,
		issued: u32, submitted: u32) -> RendererStatus;
}
```

`Default` creates pending proof, no observations, zero counters, `Cadenced`, and no failure.
`graph_ready` checks all current prerequisites except the two fresh submissions. It does not advance their count.
Use that result for preparation warmup and pipeline-stage reporting. Never use final `Ready` to permit first probe capture.
`failure` clones only the retained error pointer. `fail` stores the first `Arc<str>` once and invalidates loading proof.

The owner retains optional request, content result, graph observation, effects observation, and terminal error.
It retains mode, decision frame, content floor, `[u32; 2]` capture starts, and `[u32; 2]` validated publication starts.
It retains the presentation identity, issued-token floor, last counted submitted token, and a count capped at two.
No dynamic collection belongs to the proof. Existing execution atomics stay outside this pure owner.

The Bevy adapter defines these exact owner types and access methods:

```rust
struct RendererState { proof: RendererReadiness, audit: FrameAudit }
#[derive(Resource, Clone)]
struct RendererStateHandle(Arc<Mutex<RendererState>>);
struct RendererContentCheck { preparation: ContentPreparation, history_revision: u32 }
#[derive(SystemParam)]
struct RendererReadinessAccess<'w, 's> {
	state: Res<'w, RendererStateHandle>, epoch: Res<'w, SourceSceneEpoch>,
	clock: Res<'w, SourceViewClock>, submissions: Res<'w, SourceSubmissionAcks>,
	activation: Res<'w, ProbeActivation>, scale: Res<'w, SceneScale>,
	cameras: Query<'w, 's, (&'static SourceSurfaceView, &'static Camera)>,
	windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
}
impl RendererStateHandle {
	fn lock(&self) -> MutexGuard<'_, RendererState>;
	fn failure(&self) -> Option<Arc<str>>;
	fn fail(&self, error: String);
}
impl RendererReadinessAccess<'_, '_> {
	fn check(&mut self, content: RendererContentCheck) -> RendererStatus;
	fn graph_ready(&self, content: RendererContentCheck) -> bool;
	fn request(&self, content: RendererContentCheck) -> Option<ReadinessRequest>;
}
fn install(app: &mut App);
fn observe_main_readiness(state: Res<RendererStateHandle>,
	materials: Option<Res<materials::SourceMaterials>>,
	master: Option<Res<lighting::SourceLighting>>);
fn control_bytes() -> usize;
```

`FrameAudit` is the current `FrameProgress` audit without loading policy, prepared content, and readiness decision fields.
Keep its draw, CPU, GPU, probe, and effects records in `frame_graph.rs` with crate visibility where needed.
`RendererStateHandle::default` allocates the existing graph shared block once before fixed admission completes.
`control_bytes` accounts for that block, handle copies, snapshots, and retained required views exactly as the current function does.
`lock` reuses the existing graph mutex. Existing producers update audit and proof under the same acquired guard.
`failure` and `fail` replace existing status lock sites. Do not nest a handle lock inside another handle lock.

App code constructs `RendererContentCheck` on the stack from its final preparation result and `drive.runtime.audio_motion_revision`.
The adapter reads session from that preparation key. It never borrows `Drive`, which app code already borrows mutably.
`request` builds the current identity from those values and the listed resources and queries. It performs no proof mutation.
`graph_ready` is a noncounting query for loading reports and warmup. `check` reports content and advances only the startup submission count.
It verifies active camera index zero, nonzero extent, matching viewport, target, and window, then builds `ReadinessRequest`.
Absent or conflicting camera identity invalidates the submission count and returns `Loading`.
A failure returns `RendererStatus::Failed(Arc<str>)` after the guard is released.

Move `SourceExecutionGate` and its current `ExecutionProgress` atomics into `presentation/readiness.rs` without changing their public method signatures.
Add `fn snapshot(&self, requested: u32) -> Validation` for fixed completion and validation observations.
Load each global completed and validated counter once with Acquire ordering. Read them inside the existing observation lock boundary.
Use the supplied boundary generation as `requested`. Do not load the global requested counter again or add a new lock.
Keep `watch_submission`, `validated_generation`, `submission_validated`, and `validated` behavior.
Move `validate_completed_submissions` and `source_render_error` registration into `install`.
Call `install` after `DefaultPlugins` creates `RenderApp`, before source-lighting and frame plugins. Share the admitted handle and gate across both worlds.
The error handler retains `StopRendering`, original error text, and material failure propagation.

### RED: Pin the pure contract

- [ ] Add the five named test groups in `src/render_readiness/tests.rs` and register them under the pure module.

Use only value fixtures. Do not build a simulated GPU, fake Bevy schedule, or full manifest fixture.
Define local test helpers with these exact signatures and defaults:

- `request() -> ReadinessRequest`: session `11`, scene `3`, native and scene extent `[800,600]`, `Native`, revision `5`, mask `3`, history `7`.
- `graph(r: ReadinessRequest, frame: u32, token: u32) -> GraphObservation`: all flags true, mask `1`, uploaded counts `2`, submitted token equal to token.
- `effects(r: ReadinessRequest, frame: u32, token: u32) -> EffectsObservation`: all flags true, all effect tokens equal to token, current history revision.
- Both observation helpers use `Validation { requested: 4, completed: 4, validated: 4 }`, including both probe validations.
- `prepared(r: ReadinessRequest) -> RendererReadiness`: report prepared content, snapshot frame `20`, capture both probes at `20`, validate both publications.
- `observe(p: &mut RendererReadiness, r: ReadinessRequest, frame: u32, token: u32)`: submit the two helper observations to their exact owner methods.

The tests use these exact assertions. Each loop starts from a fresh prepared owner.

1. `all_current_prerequisites_block_readiness`: with complete observations, `graph_ready(r)` is true. Flip each boolean prerequisite separately.
   Also test uploaded counts `0/0`, `2/1`, mask `0`, missing display encoding, token `0`, and pending content.
   Assert `!graph_ready(r)` and `check(r,30,32) == Loading` for each case.
   With mask `1`, set probe 1 `published=false` and its validation generations to `0`. Assert `graph_ready(r)` while both pipelines remain ready.
   Then set disabled probe 1 `pipelines_ready=false` and assert `!graph_ready(r)`. Disabled probes skip publication validation too.
2. `stale_identities_and_publications_are_rejected`: independently change session `12`, scene `4`, revision `6`, native extent `[900,600]`, scale `ThreeQuarter`, and mask `1`.
   Supply old observations and assert `Loading`. For scale, use scene extent `[600,450]`.
   Also compare Native `[800,600]` with ThreeQuarter native `[1067,800]`, both scene extent `[800,600]`. Reject the old identity.
   Snapshot prepared content at frame `20` after a capture at `19`. Validate publication and assert `Loading` until both new captures start at `20` or later.
   Change history to `8` and assert old effects block readiness. Supply new effects and assert `graph_ready(r)` without recapture.
3. `submission_history_completion_and_validation_are_distinct`: assert `Loading` for submitted token `0`, committed token `0`, completion `3`, and validation `3` separately.
   For requested generation `4`, `completed=4, validated=3` still blocks. With both at `4`, `graph_ready(r)` passes.
   Request `0` never passes. A probe publication with validation `3` cannot supply current freshness.
4. `two_fresh_submissions_survive_no_invalidations`: complete observations at token `30`. Assert `check(r,30,30) == Loading` twice.
   Observe token `31` and assert `Loading` twice. Observe token `32` and assert `Ready`.
   Invalidate content, restore complete proof, and use floor `40`. Assert `40 -> Loading`, `41 -> Loading`, duplicate `41 -> Loading`, `42 -> Ready`.
   Fail with `"first"`, then `"second"`. Assert every status retains `"first"`, including after scene invalidation.
5. `invalidation_preserves_capture_and_history_contracts`: pending frames `1..=4` allow only frame `4` and forbid captures.
   Full snapshot `20` then content, revision, mask, extent, or scale change requests restart. New scene alone does not request another restart.
   History change preserves capture freshness but clears the effect proof and presentation count. Scene change rejects all old proof.
   Invalidate extent and scale separately. Old graph validation cannot pass the new identity.
   Bypass snapshot stays bypass after later `bypass=false`, permits frames and captures, and cannot grant readiness without complete observations.
   A floating-origin change supplies identical readiness identity and history revision. Assert no restart and unchanged freshness and presentation count.

- [ ] Run `.\scripts\check.ps1 -Filter render_readiness` and record the expected missing-type or missing-method failure.

Formatting must pass before the RED command reaches compilation. An unrelated toolchain failure is not a valid RED result.

### Implement the complete owner migration

- [ ] Implement the pure interfaces and policy with fixed storage. Move existing loading and presentation rules without compatibility exports.
- [ ] Implement the adapter, shared state admission, memory accounting, terminal error ownership, and atomic generation snapshots.
- [ ] Migrate app preparation, all producer observations, all failure callers, diagnostics, and export-backed test callers from the file map.
- [ ] Delete old completion resources, methods, imports, and loading-dispatch module in the same task.

Preserve these policy details during implementation:

- Match graph and effects identity, frame, and token. Require sky, world mask `1`, display encoding, all pipeline flags, and texture counts.
- Match effects input, encoded token, submitted token, committed token, and history revision. Keep startup validation generation reuse.
- Bind capture freshness to session, scene, native extent, scene extent, scale, revision, and probe mask. Exclude motion history revision from capture identity.
- Preserve the content floor, fourth-frame cadence, queued startup restart, and sticky gameplay or pending-save bypass.
- Set the presentation floor only after complete prerequisites. Count two distinct later CPU submissions, without adding two new GPU waits.
- Keep monotonic restart tokens, cleared acknowledgements, reset probe schedules, and warmup. Never reset issued tokens to zero.
- Preserve scene `SourcePostChain::recreate`, motion `reset_history`, and extent-driven target replacement with validation reset.
- Preserve floating-origin image history and probe job order. Do not add motion-driven probe recapture, LUT rebuild, or noise reset.
- Preserve pipeline producer outcomes. `owner.ready().is_ok()` permits pending scope preparation but never grants renderer readiness.

Register this exact ordering in the adapter and existing plugins:

1. Register adapter `observe_main_readiness.before(check_ready)` from `app.rs`. Copy existing master and material failures without changing messages.
   Content preparation and readiness consumption remain before input and save dispatch.
2. Report one final content result per check. Every incomplete early exit reports pending. Do not transiently invalidate prepared content at entry.
3. Keep snapshot after camera updates, transform propagation, and lighting synchronization, before frustum update.
4. Keep source frustum publication after frustum update and before visibility checks.
5. Apply render snapshots after `ExtractCommands`, before `PrepareAssets`. Prepare resources before `RenderSystems::Render`.
6. Begin graph state in `RenderGraphSystems::Begin`. Filter probes after rendering and before submission.
7. In `RenderGraphSystems::Finish`, record graph submission, then commit effects history, then publish the final effect observation within its current lock.
8. Keep callback completion atomic-only. Advance validation in render `First` after Bevy's pre-extract error poll. Consume proof on later main Update.

Native tests exercise proof values, not the real scheduling or history backend. Inspect those production paths during review.

### GREEN: Verify the atomic result

- [ ] Run `.\scripts\check.ps1 -Filter render_readiness`. Require all five groups and WASM compilation to pass.
- [ ] Run `.\scripts\check.ps1 -Test fresh_session`. Require nonignored permit matching and WASM compilation to pass.
- [ ] Search for obsolete callers with the command below. Require no matches in `src` or `tests`.

```powershell
rg -n 'SourceLightingStatus|SourceGraphProgress|PresentedFrames|loading_dispatch|sync_lighting_readiness' src tests
```

`rg` exit code `1` means the obsolete-symbol search passed. The master lighting owner in `presentation/lighting.rs` remains valid.
Inspect all `source_lighting::SourceLighting` imports separately. The readiness resource must be gone, while its plugin name can remain.

- [ ] Review `git diff --check` and the complete changed-file diff. Require no whitespace errors or unrelated changes.
- [ ] Report deferred export-backed checks and GPU limits before claiming completion.

`web/assets/reference/manifest.json` is absent in this worktree at plan writing.
The regular `fresh_session` command verifies only nonignored permit identity matching after presentation-test migration.
When exports exist, the real commit test requires this explicit command inside the configured Docker toolchain:

```text
cargo test --locked --test fresh_session first_permit_retains_runtime_and_commits_once_paused -- --ignored
```

The migrated world probe-union test remains export-backed and ignored. Report it as deferred without the installed export.
With exports installed, run `cargo test --locked --lib complete_probe_union_preserves_revision_and_fresh_cycles -- --ignored` inside the configured toolchain.
Do not restore full exports or invent a reduced manifest for this refactor.
Native tests do not prove GPU callbacks, actual history commits, cache outcomes, device errors, resize allocation, or Bevy system order.
WASM compilation checks types and registration syntax. It does not execute renderer wiring or prove runtime parity.
Do not run full checks, release builds, or browser acceptance under this plan.

### Commit and handoff

- [ ] Root stages only reviewed files from the file map and inspects the staged diff.
- [ ] Root commits the atomic result with `git commit -m "refactor: centralize renderer readiness proof"`.
- [ ] Report changed ownership, passed checks, failures, ignored export tests, and unverified renderer behavior.

Written-plan review remains required before implementation. Subagent-driven development is already selected.
