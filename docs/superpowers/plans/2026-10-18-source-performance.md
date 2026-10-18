# Source performance implementation plan

> For agentic workers: Use superpowers:subagent-driven-development. Implement and review each task before proceeding.

**Goal:** Remove repeated renderer and physics work while preserving source behavior.
**Architecture:** Reuse existing material, draw, shadow, and Rapier ownership. Retain only data with explicit invalidation.
**Tech Stack:** Rust, Bevy 0.19.1, wgpu 29.0.4, WebGL2, Rapier 0.36.0.
**Spec:** docs/superpowers/specs/2026-10-18-source-performance.md

## Global constraints

- Use build 11140890 as source authority.
- Preserve view masks, cascades, offscreen casters, material mutations, and motion history.
- Avoid new allocations and repeated setup during stable frames.
- Use existing dependencies. Remove obsolete code paths.
- Keep each component working before adding the next change.
- Do not alter unrelated dirty files from the primary checkout.
- Leave the branch unmerged. Do not push.
- Do not claim FPS gains from builds or draw counts.

## Review focus

- Material mutation must affect one intended object, including LOD fades and lamps.
- Motion batches must preserve coverage, per-view history, and forced-zero mode.
- Shadow caches must invalidate after streaming, transform, origin, visibility, or material changes.
- Query bounds must follow moves, shape changes, enablement, insertion, removal, and handle reuse.
- Fixed timing must advance the same simulation duration across normal and catchup frames.

## Final verification owned by the controller

- [x] Review each task against its brief and changed code.
- [x] Run native tests, Rust formatting, and a locked release WASM build on the combined branch.
- [x] Review material, motion, shadow, query, and timing interactions across the branch.
- [ ] Run exclusive matched release garage and driving checks with frozen assets where the environment permits. No accepted garage FPS sample or completed driving run exists.
- [x] Record hashes, runtime failures, timing limits, and unmeasured outcomes.

### Task 1: Share immutable generated materials

**Files:** src/presentation/world/instance_materials.rs; SourceMaterials ownership and mutation sites; focused material tests.
**Interfaces:** Preserve InstanceMaterials::bind inputs and RendererMaterial outputs. Reuse existing material keys and owner cleanup.

- [x] Trace mutation paths for generated primary and caster handles, including lamps, paint, dirt, and LOD fades.
- [x] Share handles only when complete rendering state is identical and immutable.
- [x] Isolate mutable state before mutation. Include reflection, layer, receive-shadow, and caster culling in identity keys.
- [x] Release scene-owned entries on reset and source release. Prevent stale handles across scene owners.
- [x] Add critical checks for sharing, key differences, independent mutations, and owner reset.
- [x] Run focused checks and a release WASM check. Commit this task. Report commands and output.

### Task 2: Batch motion vectors

**Files:** src/presentation/motion_draw.rs; src/presentation/motion_vectors.rs; motion shaders; necessary render call sites; focused motion checks.
**Interfaces:** Preserve MotionModelHistory semantics and rendering acknowledgement boundaries.

- [x] Replace per-object UniformBuffer uploads with reusable batched instance data supported by WebGL2.
- [x] Replace one-instance split_draw dispatch with draws over compatible contiguous eligible instances.
- [x] Preserve previous/current models, forced-zero mode, and instance-to-model mapping.
- [ ] Preserve source alpha/deformation motion behavior.

> Current source motion coverage is position-only. Alpha/deformation parity remains unverified; this branch makes no such parity claim.
- [x] Preserve camera-only and stationary-object exclusions. Reset on origin, scene, and renderer lifecycle changes.
- [x] Add critical checks for mixed eligibility, instance order, history completion, and forced-zero data.
- [x] Run focused checks and a release WASM check. Commit this task. Report commands and output.

### Task 3: Retain shadow caster preparation

**Files:** src/presentation/shadow_scene.rs; src/reference/shadow_frame.rs; adjacent shadow helpers; focused shadow checks.
**Interfaces:** Preserve ShadowScene consumers and exact prepare_frame results. Change internal APIs with their callers.

- [x] Retain world bounds for unchanged caster transforms and local bounds. Invalidate removed or changed entities.
- [x] Track material, layer, caster eligibility, visibility, scene, and origin changes. Preserve camera-visible versus shadow-only membership.
- [x] Validate caster data once per changed set instead of once per active view.
- [x] Reuse view results only with complete unchanged view/light/caster inputs. Avoid a speculative spatial index.
- [x] Add oracle checks for transforms, offscreen casters, view changes, origin changes, and removal.
- [x] Run focused checks and a release WASM check. Commit this task. Report commands and output.

### Task 4: Update changed physics query bounds

**Files:** src/vehicle.rs; direct PhysicsWorld mutation sites; focused physics-query tests.
**Interfaces:** Keep PhysicsWorld::refresh_queries correctness and call order. Reuse Rapier modification tracking where its public API supports this.

- [x] Inspect Rapier query-update APIs before adding custom state.
- [x] Replace unconditional AABB updates with changed-collider updates.
- [x] Preserve body changes needed by the next physics step. Retain island and pipeline notifications.
- [x] Handle collider moves, body moves, attachments, shape edits, enablement, insertion, removal, and handle reuse.
- [x] Preserve query ordering for weather, wheels, interaction, recovery, and post-physics synchronization.
- [x] Add critical mutation-boundary checks and repeated unchanged refresh checks.
- [x] Run focused checks and the native suite. Commit this task. Report commands and output.

### Task 5: Restore source fixed timing

**Files:** src/simulation.rs; duplicated physics-step constants and timing tests, where present.
**Interfaces:** One authoritative FIXED_DT = 0.02 seconds supplies StepClock and Rapier.

- [x] Trace interval consumers and remove hardcoded 60 Hz simulation intervals.
- [x] Match source TimeManager Fixed Timestep 0.01999999955 through one authoritative constant.
- [x] Preserve bounded catchup, with five steps per accepted 0.1-second frame.
- [x] Update elapsed-time tests using simulation seconds instead of frame-count assumptions.
- [x] Check suspension, impulses, engine, fluids, and save timing through existing tests. Do not retune source parameters.
- [x] Run focused checks and the native suite. Commit this task. Report commands and output.
