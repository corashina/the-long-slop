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

### Task 1: Share immutable generated materials

**Files:** src/presentation/world/instance_materials.rs; SourceMaterials ownership and mutation sites; focused material tests.
**Interfaces:** Preserve InstanceMaterials::bind inputs and RendererMaterial outputs. Reuse existing material keys and owner cleanup.

- [ ] Trace mutation paths for generated primary and caster handles, including lamps, paint, dirt, and LOD fades.
- [ ] Share handles only when complete rendering state is identical and immutable.
- [ ] Isolate mutable state before mutation. Include reflection, layer, receive-shadow, and caster culling in identity keys.
- [ ] Release scene-owned entries on reset and source release. Prevent stale handles across scene owners.
- [ ] Add critical checks for sharing, key differences, independent mutations, and owner reset.
- [ ] Run focused checks and a release WASM check. Commit this task. Report commands and output.

### Task 2: Batch motion vectors

**Files:** src/presentation/motion_draw.rs; src/presentation/motion_vectors.rs; motion shaders; necessary render call sites; focused motion checks.
**Interfaces:** Preserve MotionModelHistory semantics and rendering acknowledgement boundaries.

- [ ] Replace per-object UniformBuffer uploads with reusable batched instance data supported by WebGL2.
- [ ] Replace one-instance split_draw dispatch with draws over compatible contiguous eligible instances.
- [ ] Preserve previous/current models, forced-zero mode, source alpha/deformation, and instance-to-model mapping.
- [ ] Preserve camera-only and stationary-object exclusions. Reset on origin, scene, and renderer lifecycle changes.
- [ ] Add critical checks for mixed eligibility, instance order, history completion, and forced-zero data.
- [ ] Run focused checks and a release WASM check. Commit this task. Report commands and output.

### Task 3: Retain shadow caster preparation

**Files:** src/presentation/shadow_scene.rs; src/reference/shadow_frame.rs; adjacent shadow helpers; focused shadow checks.
**Interfaces:** Preserve ShadowScene consumers and exact prepare_frame results. Change internal APIs with their callers.

- [ ] Retain world bounds for unchanged caster transforms and local bounds. Invalidate removed or changed entities.
- [ ] Track material, layer, caster eligibility, visibility, scene, and origin changes. Preserve camera-visible versus shadow-only membership.
- [ ] Validate caster data once per changed set instead of once per active view.
- [ ] Reuse view results only with complete unchanged view/light/caster inputs. Avoid a speculative spatial index.
- [ ] Add oracle checks for transforms, offscreen casters, view changes, origin changes, and removal.
- [ ] Run focused checks and a release WASM check. Commit this task. Report commands and output.

### Task 4: Update changed physics query bounds

**Files:** src/vehicle.rs; direct PhysicsWorld mutation sites; focused physics-query tests.
**Interfaces:** Keep PhysicsWorld::refresh_queries correctness and call order. Reuse Rapier modification tracking where its public API supports this.

- [ ] Inspect Rapier query-update APIs before adding custom state.
- [ ] Replace unconditional AABB updates with changed-collider updates.
- [ ] Preserve body changes needed by the next physics step. Retain island and pipeline notifications.
- [ ] Handle collider moves, body moves, attachments, shape edits, enablement, insertion, removal, and handle reuse.
- [ ] Preserve query ordering for weather, wheels, interaction, recovery, and post-physics synchronization.
- [ ] Add critical mutation-boundary checks and repeated unchanged refresh checks.
- [ ] Run focused checks and the native suite. Commit this task. Report commands and output.

### Task 5: Restore source fixed timing

**Files:** src/simulation.rs; duplicated physics-step constants and timing tests, where present.
**Interfaces:** One authoritative FIXED_DT = 0.02 seconds supplies StepClock and Rapier.

- [ ] Trace interval consumers and remove hardcoded 60 Hz simulation intervals.
- [ ] Match source TimeManager Fixed Timestep 0.01999999955 through one authoritative constant.
- [ ] Preserve bounded catchup, with five steps per accepted 0.1-second frame.
- [ ] Update elapsed-time tests using simulation seconds instead of frame-count assumptions.
- [ ] Check suspension, impulses, engine, fluids, and save timing through existing tests. Do not retune source parameters.
- [ ] Run focused checks and the native suite. Commit this task. Report commands and output.

## Final verification

- [ ] Review each task against its brief and changed code.
- [ ] Run native tests, Rust formatting, and a locked release WASM build on the combined branch.
- [ ] Review material, motion, shadow, query, and timing interactions across the branch.
- [ ] Run exclusive matched release garage and driving checks with frozen assets where the environment permits.
- [ ] Record hashes, runtime failures, timing limits, and unmeasured outcomes.
