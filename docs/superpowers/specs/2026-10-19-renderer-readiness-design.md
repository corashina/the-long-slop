# Renderer readiness

Date: 2026-10-19.
Status: Approved by the user on 2026-10-19.
Branch: `codex/codebase-architecture`.
Base: `97783a10444c9d5faef9d1c5a10b2ff21469d33c`.

## Intent and scope

Give one renderer owner the current completion proof and its invalidation rules.
App code retains content preparation, failure reporting, and gameplay start.
The user approved this written specification. Implementation planning is authorized.
The stated base is authoritative. Changes on `master` are outside this work.

The scope remains one complete Lada garage experience.
Steam build `11140890` and its decompiled behavior remain authoritative.
This specification preserves inspected behavior. It does not establish source parity or browser acceptance.
Shader changes, visual changes, performance changes, and live mirrors are outside this section.

## Current ownership and evidence

| Source at the stated base | Current contract |
| --- | --- |
| `src/app.rs:329-591` | `check_ready` prepares content, checks renderer state, counts presentations, and starts readiness. |
| `src/app.rs:375,534-536` | App code clears content proof, then reports the prepared session and selection revision. |
| `src/app.rs:541-579` | App code checks camera extent, target extent, window extent, probe mask, and two later submissions. |
| `src/presentation/frame_graph.rs:69-193` | `SourceGraphProgress` owns loading proof, graph readiness, and audit state. |
| `src/presentation/source_lighting.rs:261-305,350-405` | `SourceLighting` duplicates completion as seven flags and shared status. |
| `src/presentation/source_lighting.rs:317-348` | Queue callbacks report GPU completion. Render `First` advances validated generations. |
| `vendor/bevy_render/src/error_handler.rs:180-204` | Bevy polls device errors before extraction and rendering. |
| `src/presentation/frame_graph.rs:920-973` | Graph completion records submission tokens and waits for validated probe publication. |
| `src/presentation/source_effects.rs:395-440` | Effects commit CPU history after submission and separately check validation. |
| `src/presentation/postprocessing/chain.rs:319-328` | `completed` commits exposure, LUT, and noise history. It does not report GPU completion. |
| `src/reference/loading_dispatch.rs:37-132` | Loading cadence and probe freshness bind session, scene, extent, revision, and enabled mask. |
| `src/save/fresh_permit.rs:56-80` | `PresentedFrames` requires two distinct submissions above the initial issued token. |

## Selected owner

Add framework-free `RendererReadiness` in `src/render_readiness.rs`.
Add its Bevy adapter in `src/presentation/readiness.rs`.
App code uses this adapter contract:

```rust
match renderer.check(content_preparation) {
	RendererStatus::Loading => {}
	RendererStatus::Ready => { drive.ready = true; game_ready(); }
	RendererStatus::Failed(error) => report_failure(error),
}
```

`RendererReadinessAccess` is a Bevy `SystemParam` in the adapter.
It reads renderer resources, camera extent, window extent, scale, epoch, and existing token counters.
Its `check` method reports content preparation and evaluates the matching proof.
App code never assembles graph flags or counts renderer submissions.
`Failed` returns a cloned `Arc<str>` for the retained error. It can outlive the adapter's lock guard.
Create this shared error once on terminal failure. Normal checks allocate no strings.

App code reports `Pending` for an incomplete content check and `Prepared` after complete preparation.
A single completed check reports one final result. A transient reset at system entry must not erase valid proof.
Early exits report `Pending`. App failures report pending content before the existing failure route returns.

## Identities and interfaces

These are new internal types and interfaces. Reuse `SceneScale` from `src/render_scale.rs`.
The framework-free module and `SceneScale` compile for WASM and native unit tests.

```rust
struct ContentKey { session: u64, selection_revision: u64 }
enum ContentPreparation { Pending(ContentKey), Prepared(ContentKey) }
struct RenderIdentity {
	session: u64,
	scene: u32,
	native_extent: [u32; 2],
	scene_extent: [u32; 2],
	scale: SceneScale,
}
struct ReadinessRequest {
	render: RenderIdentity,
	selection_revision: u64,
	probe_mask: u32,
	history_revision: u32,
}
enum LoadingMode { Cadenced, Full, Bypass }
struct LoadingDecision { captures_allowed: bool, restart_scene: bool }
enum RendererStatus { Loading, Ready, Failed(std::sync::Arc<str>) }

impl RendererReadiness {
	fn content_checked(&mut self, content: ContentPreparation);
	fn snapshot_loading(&mut self, request: ReadinessRequest,
		frame: u32, bypass: bool) -> LoadingDecision;
	fn allows_frame(&self, identity: RenderIdentity, frame: u32) -> bool;
	fn capture_started(&mut self, request: ReadinessRequest,
		frame: u32, probe: usize);
	fn publication_validated(&mut self, request: ReadinessRequest, probe: usize);
	fn observe_graph(&mut self, observation: GraphObservation);
	fn observe_effects(&mut self, observation: EffectsObservation);
	fn invalidate(&mut self, cause: Invalidation);
	fn fail(&mut self, error: String);
	fn check(&mut self, request: ReadinessRequest,
		issued: u32, submitted: u32) -> RendererStatus;
}
```

`GraphObservation` contains render identity, frame, token, sky, world-view mask, resource flags, pipeline flags, and graph validation generation.
It contains fixed probe records for pipeline readiness and publication state.
`EffectsObservation` contains render identity, history revision, frame, token, input flags, pipeline flags, committed token, and validation generation.
Submission, completion, and validation observations retain their separate names and meanings.
The adapter supplies observed generations from the existing execution gate.

`Invalidation` has content, scene, extent, scale, history, and failure causes.
Each cause invalidates only its dependent proof, as defined below.
Invalid camera identity or missing extent produces `Loading` and resets the fresh-submission count.
Terminal failures retain the first error. Restart cannot clear a terminal device or renderer failure.

## Fixed state and completion proof

The owner retains one request, one content result, one graph observation, and one effects observation.
It retains the loading mode, decision frame, content floor, and fixed probe capture and publication arrays.
It retains validation generations, the fresh-submission floor, the last counted token, and a count capped at two.
Only terminal failure text has variable size. Normal frame operations use fixed-size values.

Move the existing `SourceGraphProgress` shared storage into the adapter's `RendererState`.
Keep its existing shared mutex and fixed audit records. Do not add a second readiness mutex.
Producers update readiness within their existing locked audit updates.
Reuse the current execution gate atomics and callback allocation pattern.
Do not add per-frame locks, queues, messages, vectors, or callback registrations beyond the current validation boundaries.
Diagnostic serialization remains an explicit request.
Update retained control-memory accounting for the changed fixed state.

A complete proof requires all current content, material, lighting, plate, particle, and road-lamp prerequisites.
Content preparation remains an app responsibility. Renderer resources and pipeline outcomes remain renderer inputs.
The world camera viewport must match its target and the primary window's physical extent.
The scene extent must equal `SceneScale::extent(native_extent)`.

Graph requirements retain source targets, uploaded lighting textures, surface, camera, sky, cloud, caster, probe, and effect pipeline readiness.
The current frame must contain sky, world-view mask `1`, a nonzero token, and final display encoding.
Effects must match the frame and token. Their commands must be submitted and their CPU history committed.
The required graph and effects validation generations must be validated.
Each enabled probe must have a validated publication from a capture at or above the current content floor.
Disabled probes do not require publication. Current probe pipeline requirements remain unchanged.

GPU completion alone never supplies validation.
Queue submission alone never supplies GPU completion.
Effect history commit alone supplies neither GPU completion nor validation.
Preserve current startup and resize validation boundaries. Do not require a new GPU wait for every later frame.

After these conditions hold, set the floor to the current issued main token.
Count only distinct submitted tokens above that floor.
Require two tokens before `Ready`. A duplicate token cannot advance the count.
Any failed current prerequisite resets the count.

## Schedule contract

Keep content preparation and readiness consumption in the existing app Update chain before input and save dispatch.
The adapter refreshes resource and failure observations before app readiness consumption.
App preparation does not depend on a renderer `Ready` result, which would block first probe capture.

Preserve `view_frame::snapshot` after camera updates, transform propagation, and lighting synchronization.
Preserve its position before frustum update, then publish source frusta before visibility checks.
See `src/presentation/view_frame.rs:446-468`.

Preserve render snapshot application after `ExtractCommands` and before `PrepareAssets`.
Preserve resource preparation before `RenderSystems::Render`.
Preserve frame initialization in `RenderGraphSystems::Begin`.
Preserve probe filtering after graph rendering and before graph submission.

In `RenderGraphSystems::Finish`, record graph submission before effect history commit.
Publish the final observation through the existing effects audit update after history commit.
See `src/presentation/frame_graph.rs:317-328` and `src/presentation/source_effects.rs:395-440`.

Keep validated-generation advancement in render `First`, after Bevy's error poll.
Callbacks update completion atomics only. They never set `Ready`.
The main world consumes the resulting observations on a later Update.

## Invalidation and history contract

| Change | Required result |
| --- | --- |
| Content pending, selection revision change, or probe-mask change during full startup | Clear freshness and submission count. Preserve the existing queued scene-restart decision. |
| New session or scene epoch | Reject old observations and probe publications. Reset the startup proof. |
| Physical extent or scale change | Reject old extent proof. Preserve target replacement and startup recapture behavior. Reset affected validation. |
| Scene restart | Preserve monotonic main and probe tokens. Reset acknowledgements, probe schedules, and warmup. |
| Scene restart in effects | Preserve `SourcePostChain::recreate` and its existing history reset. |
| Motion revision change | Preserve exposure-history reset and pending-token clear. Require an effects observation for the new revision. |
| Floating-origin rebase | Preserve image history and probe job order. Do not force recapture or recreate effects. |
| Renderer or device failure | Keep terminal failure. Invalidate loading freshness and the submission count. |

History revision does not become a probe capture key.
Motion-history invalidation must not add probe recapture, noise reset, or LUT rebuild.
See `source_effects.rs:186-205`, `postprocessing/chain.rs:253-270`, and `view_frame.rs:479-503`.

Preserve `Cadenced`, `Full`, and sticky `Bypass` behavior from `LoadingDispatch`.
Before complete content, the current snapshot renders every fourth frame and cannot start probe capture.
Bypass remains active for gameplay and pending save requests. Bypass never grants initial app readiness by itself.
Snapshot decisions use current selection revision because preparation can change it after app content checks.

## File map and removals

| File | Responsibility after the change |
| --- | --- |
| `src/render_readiness.rs` | Pure proof, loading cadence, identities, invalidation, and submission count. |
| `src/presentation/readiness.rs` | Bevy adapter, shared state, execution gate, errors, and registration. |
| `src/presentation/frame_graph.rs` | Draw execution and graph audit production. |
| `src/presentation/source_effects.rs` | Effect preparation, history operations, and effect observations. |
| `src/presentation/view_frame.rs` | View snapshot, tokens, probe scheduling, and scene restart mechanics. |
| `src/presentation/source_lighting.rs` | Source lighting preparation and draw commands. |
| `src/app.rs` | Content preparation and final app readiness action. |
| `src/save/fresh_permit.rs` | `FreshSessionPermit` only. |

Remove `PresentedFrames`, `SourceLighting` readiness flags, and the independent `SourceLightingStatus` completion state after caller migration.
Move execution-gate handling into the adapter without changing callback semantics.
Remove `reference/loading_dispatch.rs` after its proof and tests move into `render_readiness.rs`.
Remove old readiness methods and all compatibility exports.

Pipeline ownership supplies ready, failed, isolated, and cancelled outcomes through existing producers.
Readiness never acquires pipelines, prepares descriptors, releases owners, or changes candidate promotion.
The pipeline ownership specification defines that lifecycle first.
Its `owner.ready().is_ok()` permits pending GPU scopes during preparation.
That permission does not satisfy completion or later error validation.

## Rejected alternative

Bevy messages could carry content, captures, submissions, and completion events to a reducer.
That design adds queues, event ordering, and stale-event retention to existing shared-state communication.
The selected owner keeps fixed storage and uses the existing observation boundaries.

## Critical checks and limits

Five native tests cover the pure owner.

1. Every current content, graph, probe, and effect prerequisite blocks readiness when absent.
2. Old sessions, scenes, revisions, extents, scales, masks, and publications cannot satisfy a new proof.
3. Submission, CPU history commit, GPU completion, and later error validation remain distinct.
4. Two distinct fresh submissions are required. Duplicate tokens and invalidation cannot advance the count.
5. Restart, resize, motion revision, bypass, and floating-origin cases preserve their stated history and capture behavior.

After implementation, run these targeted commands from this worktree:

```powershell
.\scripts\check.ps1 -Filter render_readiness
.\scripts\check.ps1 -Test fresh_session
```

Both commands include native tests, formatting checks, and the WASM compile check.
The second command verifies nonignored `FreshSessionPermit` identity matching.
The export-backed commit test remains ignored. Run it explicitly in the configured Docker toolchain when exports exist:

```text
cargo test --locked --test fresh_session first_permit_retains_runtime_and_commits_once_paused -- --ignored
```

Report that commit check as deferred when its exports are unavailable.
Move presentation tests out of `fresh_session` into the pure-owner tests.

Native tests cannot exercise actual renderer scheduling, GPU callbacks, target replacement, or browser device errors.
`src/lib.rs:29-30` gates presentation to WASM. A WASM compile check does not execute that schedule.
Browser acceptance remains a separate requested check. No current runtime or visual claim follows from these tests.
Full checks, release builds, and browser acceptance remain deferred.
