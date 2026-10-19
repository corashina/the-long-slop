# Pipeline ownership Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `PipelineOwners` the complete candidate pipeline lifecycle while preserving current rendering and accounting rules.

**Architecture:** Expand the existing product owner with concrete base and companion entry points. Keep descriptor edits and draw lookups in pass modules. A small native transition policy drives the real Bevy preparation loop.

**Tech Stack:** Rust 2024, Rust 1.99, Bevy 0.19.1, wgpu 29.0.4, the existing local `bevy_render` patch, and native unit tests.

**Spec:** [Approved pipeline specification](../specs/2026-10-19-pipeline-ownership-design.md).

Status: Awaiting user plan review. Product implementation requires that review.
Date: 2026-10-19. Branch: `codex/codebase-architecture`. Plan base: `8ce8b0c`.
Worktree: `C:/Users/Corashina/.codex/worktrees/codebase-architecture/the-long-drive-wasm`.
Execution method: Subagent-driven development, selected by the user.
Execution order: save, startup, terrain, pipeline, then readiness.
The coordinator owns commits. Shared production edits run serially across these sections.

## Global Constraints

- Preserve the complete Lada garage scope and the approved specification.
- Keep `PipelineOwners`. Add no complete family registry, cache trait, or new dependency.
- Keep `CACHE_SLOTS = 2048`, `DESCRIPTOR_BYTES = 64 * 1024`, and `CACHE_BYTES = 16 * 1024 * 1024`.
- Keep Bevy `=0.19.1`, wgpu `=29.0.4`, Rust `1.99`, and edition `2024`.
- Keep the existing vendor cache mechanisms and `SOURCE_PATCH.md` unchanged.
- Keep persistent cache reserve admission, descriptor staging charges, batch limits, and submitted-work receipts.
- Acquire one-slot guards before specialization, descriptor cloning, and the current optional-pass decision.
- Never hold an accepted shared active-cache hit for a candidate.
- Retain and release each live family entry exactly once per ownership transfer.
- Keep retired IDs held and isolated until a later claim, active reclaim, or valid release.
- Preserve preparation through `owner.ready().is_ok()`. Outstanding scopes alone do not block another ID.
- Promote only ready pipelines. Keep the incomplete-promotion assertion.
- Keep pass discovery order, compile order, isolation timing, physical drain, and shader reload exclusion.
- Readiness consumes outcomes. It never acquires, claims, or queues pipelines.
- Use stack closures and the existing `Arc` guard. Add no per-frame vectors or retained snapshots.
- Remove obsolete public helpers and isolation resources after migrating all callers.
- Run targeted native checks and WASM compilation only. Full checks, release builds, and browser acceptance remain unauthorized.
- Keep the branch unmerged. Do not merge changes from `master`.

## Review Focus

1. Repeated cancellation retires once. Test: `cancellation_retires_once`.
2. Cancellation wins over simultaneous promotion and readiness. Test: `cancellation_has_priority`.
3. Incomplete promotion cannot release an ID. Test: `promotion_requires_ready_pipeline`.
4. Preparation uses the owner error gate, including error-free pending scopes. Test: `preparation_uses_error_gate`.
5. Failure and batch exhaustion cannot release admitted work before drain. Existing tests: `shared_receipt_withholds_errors_until_all_siblings_drain` and `whole_pair_prefix_spills_without_consuming_the_next_pair`.

Real cache claims, guard order, shader reload, and schedule behavior require source review and remain unverified at runtime.
The native tests below do not simulate a cache or claim GPU coverage.

---

## File map

| File | Change |
| --- | --- |
| `src/world/pipeline_ownership.rs` | Create the production held-entry policy and four native tests. |
| `src/world.rs` | Declare `pub(crate) mod pipeline_ownership`. |
| `src/presentation/pipeline_ownership.rs` | Add concrete APIs, own isolation, and apply policy actions to the real cache. |
| `src/presentation/world/pipelines.rs` | Use the base wrapper and owner isolation. Preserve admission and drain. |
| `src/presentation/surface_pass.rs` | Use companion queueing and owner readiness. Keep descriptor edits and draw lookup. |
| `src/presentation/caster.rs` | Use companion queueing and owner readiness. Keep optional passes and classification. |
| `src/presentation/lighting.rs` | Consume full-ID isolation from `PipelineOwners`. |
| `src/presentation/frame_graph.rs` | Supply the owner to surface and caster readiness. |
| `src/presentation/surface_graph.rs` | Supply the owner to both capture readiness checks. |
| `src/presentation/source_lighting.rs` | Preserve pass registration order. Change only affected registration arguments if needed. |
| `src/world/renderer_pending.rs` | Run existing accounting tests. No planned edits. |

No Cargo manifest, lockfile, vendor, shader, or web change is planned.
The readiness section follows this section and consumes its stable queries.

### Task 1: Consolidate the complete pipeline lifecycle

This is one atomic task. The wrappers and all direct callers change together.
An intermediate commit must not leave two public lifecycle routes.

**Files:** Create and modify the files listed above, except the unchanged accounting tests.
**Tests:** `src/world/pipeline_ownership.rs::tests` and existing `src/world/renderer_pending.rs::tests`.

**Interfaces:** Consume existing `GpuPreparation`, `PipelineCache`, pass lookup tables, material specialization functions, and `WorldPipelines::owns`.
The vendor `OwnedRenderPipelineQueue` owns its `Arc` and has no cache lifetime parameter.
Produce these interfaces in `src/presentation/pipeline_ownership.rs`.

```rust
pub(crate) enum PipelineUse<'a> {
	Active,
	Candidate(&'a GpuPreparation),
}
pub(crate) fn specialize_base(
	world: &mut World,
	usage: PipelineUse<'_>,
	specialize: impl FnOnce(&mut World)
		-> Result<CachedRenderPipelineId, ReferenceError>,
) -> Result<Option<CachedRenderPipelineId>, ReferenceError>;
impl PipelineOwners {
	pub(crate) fn queue_companion(
		&self,
		cache: &PipelineCache,
		base: CachedRenderPipelineId,
		build: impl FnOnce() -> Option<RenderPipelineDescriptor>,
	) -> Result<Option<CachedRenderPipelineId>, ReferenceError>;
	pub(crate) fn isolated(&self, id: CachedRenderPipelineId) -> bool;
	pub(crate) fn base_isolated(&self, base: CachedRenderPipelineId) -> bool;
	pub(crate) fn cancelled(&self, base: CachedRenderPipelineId) -> bool;
	pub(crate) fn fail(&self, base: CachedRenderPipelineId, message: String) -> bool;
}
pub(crate) fn refresh_isolation(world: &mut World);
pub(crate) fn prepare(world: &mut World);
```

Base `Ok(None)` means cancellation. The world caller stops that job without a shader failure.
Companion `Ok(None)` means cancellation or an omitted source pass. The pass caller records no companion ID.
`Err` preserves the current error message and routing.
`fail` returns `true` after recording a candidate error. `false` directs the caller to the active lighting status.
Both `SurfacePasses` and `CasterPasses` expose this readiness signature.

```rust
pub fn ready(&self, cache: &PipelineCache, owners: &PipelineOwners) -> bool;
```

Produce this native policy in `src/world/pipeline_ownership.rs`.
Derive `Debug`, `PartialEq`, and `Eq` for the phase and action enums.

```rust
pub(crate) enum HeldEntryPhase { Held, Retired }
pub(crate) enum HeldEntryAction { Wait, Prepare, Retire, Release }
pub(crate) struct HeldEntryObservation {
	pub cancelled: bool,
	pub isolated: bool,
	pub pipeline_ready: bool,
	pub preparation_valid: bool,
}
pub(crate) fn held_entry_step(
	phase: &mut HeldEntryPhase,
	observation: HeldEntryObservation,
) -> HeldEntryAction;
```

- [ ] Step 1, RED: Add the native module declaration and these four policy tests before implementing the policy.

```rust
#[cfg(test)]
mod tests {
	use super::*;
	use HeldEntryAction::{Prepare, Release, Retire, Wait};
	use HeldEntryPhase::{Held, Retired};
	fn pending() -> HeldEntryObservation {
		HeldEntryObservation {
			cancelled: false, isolated: true,
			pipeline_ready: false, preparation_valid: true,
		}
	}
	#[test]
	fn cancellation_retires_once() {
		let mut phase = Held;
		let cancelled = || HeldEntryObservation { cancelled: true, ..pending() };
		assert_eq!(held_entry_step(&mut phase, cancelled()), Retire);
		assert_eq!(phase, Retired);
		assert_eq!(held_entry_step(&mut phase, cancelled()), Wait);
		assert_eq!(held_entry_step(&mut phase, pending()), Wait);
	}
	#[test]
	fn cancellation_has_priority() {
		let mut phase = Held;
		assert_eq!(held_entry_step(&mut phase, HeldEntryObservation {
			cancelled: true, isolated: false,
			pipeline_ready: true, preparation_valid: false,
		}), Retire);
		assert_eq!(phase, Retired);
	}
	#[test]
	fn promotion_requires_ready_pipeline() {
		let mut phase = Held;
		assert_eq!(held_entry_step(&mut phase, HeldEntryObservation {
			isolated: false, pipeline_ready: true, ..pending()
		}), Release);
		let incomplete = std::panic::catch_unwind(|| {
			let mut phase = Held;
			held_entry_step(&mut phase, HeldEntryObservation {
				isolated: false, ..pending()
			});
		});
		assert!(incomplete.is_err());
	}
	#[test]
	fn preparation_uses_error_gate() {
		let mut phase = Held;
		assert_eq!(held_entry_step(&mut phase, pending()), Prepare);
		assert_eq!(held_entry_step(&mut phase, HeldEntryObservation {
			preparation_valid: false, ..pending()
		}), Wait);
		assert_eq!(held_entry_step(&mut phase, HeldEntryObservation {
			pipeline_ready: true, ..pending()
		}), Wait);
		assert_eq!(phase, Held);
	}
}
```

The pending fixture represents a valid preparation gate, not completion of outstanding GPU scopes.
Do not add a fake owner, fake cache, or duplicate policy to these tests.

- [ ] Step 2, RED: Run `scripts/check.ps1 -Filter world::pipeline_ownership::tests`.

Expected: Native compilation fails because the policy types or function do not exist.
A toolchain or Docker failure is a blocked check, not the expected RED result.

- [ ] Step 3, implement: Add `held_entry_step` with the specified transition order.

Retired entries return `Wait`. Cancellation sets `Retired` and returns `Retire`.
Promotion asserts `pipeline_ready` with `Promoted pipeline is not ready`, then returns `Release`.
An isolated incomplete valid entry returns `Prepare`. Other observations return `Wait`.
Keep this function free of Bevy, cache calls, allocations, and scope counts.

- [ ] Step 4, implement: Make the real Bevy `prepare` loop use this policy for every entry with an owner.

Replace `Entry::retired` with `Entry::phase`. Preserve unowned entries without preparation.
Preserve the current short-circuit order when building observations.
For retired entries, supply fixed false flags. Do not read the owner or cache for that retained entry.
For held entries, read cancellation first. Read isolation and pipeline readiness only when not cancelled.
Evaluate `owner.ready().is_ok()` only when isolated, not cancelled, and incomplete.
Keep other preparation flags false. This avoids new error-string clones or locks for retired and complete entries.
`Retire` invokes `retire_held_render_pipeline` and releases one live owner count.
`Release` invokes `release_held_render_pipeline`, releases one live count, and removes the entry.
`Prepare` opens `owner.scope(&device)` and calls `prepare_held_render_pipeline` synchronously.
`Wait` keeps the entry. Update claim, replacement, and reclaim logic to use the same phase.
Do not retain the old conditional preparation loop beside the new policy.

- [ ] Step 5, implement: Add `specialize_base` and migrate the specialization branch in `world/pipelines.rs`.

Check candidate cancellation first. Acquire the candidate guard before invoking the closure once.
Clone the required mesh layout, material properties, and candidate owner before borrowing the mutable world through the wrapper.
Map the existing base or prepass specialization error inside the closure without changing its message.
Claim only held candidate IDs. Transfer registered companions and release each previous live count once.
Reset retired entries to `Held` before retaining the new owner once per entry.
Revisit both pass tables after a changed claim or active reclaim.
Use `World::resource_scope` for owner access with mutable cache access during active reclaim.
Keep the queue guard alive through the operation. Release the owner mutex before invoking the closure.
Do not capture a world resource reference inside that closure.
Keep active-cache hits unheld and preserve the current material-key and dispatch accounting.

- [ ] Step 6, implement: Add `queue_companion` and migrate both companion builders.

Resolve the base owner and reject cancellation before the builder runs.
For candidates, acquire the one-slot guard before the builder's clone and optional-pass checks.
Keep the transparent camera-depth clone and skip decision inside the builder in their current order.
Preserve guard-capacity failure priority even when that source pass would be omitted.
For `Some(descriptor)`, queue, hold, register, and retain in that order.
Replace a prior live companion count once. Never release a retired count twice.
Active builders run without a candidate guard or hold.
Keep pass descriptor edits, discovery cursors, caster classification, and draw lookup tables in their current modules.
Route errors through `fail`, then through active lighting status only when `fail` returns `false`.

- [ ] Step 7, implement: Move the base isolation set into `PipelineOwners` and migrate all isolation consumers.

Call `refresh_isolation` at the existing compile entry point. Remove bases owned by the active bank at that time.
Add newly owned bases at the existing specialization point.
Remove `PipelineIsolation`, its resource initialization, and both pass-local isolation copies.
Use `base_isolated` in pass readiness and active pass error scans. Use `isolated` for full-ID lighting readiness, including retired IDs.
Supply `PipelineOwners` to the affected `frame_graph.rs` and `surface_graph.rs` calls.
Keep the surface-before-caster chain, compile after `PrepareBindGroups`, and preparation after compile before render.
Keep publication, compilation, companion, classification, and physical-drain checks separate.
Keep cancelled-owner drain and submitted-work receipt completion unchanged.

- [ ] Step 8, implement: Hide `queue_scope`, `claim`, `companion`, `owner`, and `reclaim_active` within the ownership module.

Remove obsolete caller branches. Keep vendor mechanisms and metadata constants unchanged.
Readiness remains a consumer of outcomes. Do not add acquisition work to readiness queries.

- [ ] Step 9, GREEN: Run `scripts/check.ps1 -Filter world::pipeline_ownership::tests`.

Expected: Four new native tests pass. Formatting and the script's WASM compile check pass.
The real Bevy owner must import and call `held_entry_step`, not a separate transition implementation.

- [ ] Step 10, GREEN: Run `scripts/check.ps1 -Filter world::renderer_pending::tests`.

Expected: Enabled accounting, batch, diagnostic, and receipt tests pass. The WASM check passes.
Review Focus 5 maps to the two existing tests named above. Keep their assertions intact.
`batch_admission_and_reconciliation_keep_global_limits_and_parent_claims` is ignored because it requires the installed source export.
Report that skipped test. Do not claim its stale-owner assertions ran.

- [ ] Step 11, review: Inspect the complete diff and search remaining ownership operations.

Run `git diff --check` and `rg -n 'PipelineIsolation|queue_scope|reclaim_active|held_entry_step|queue_companion|specialize_base' src`.
Expected: No whitespace errors, no `PipelineIsolation`, and no raw lifecycle caller outside the owner.
Inspect guard-before-clone order, complete-family count changes, active reuse, and schedule registration against the approved spec.
This inspection records wiring evidence. It does not establish runtime behavior.

- [ ] Step 12, commit: Let the coordinator stage only this task's changed files and commit after review.

Suggested message: `refactor: centralize candidate pipeline ownership`.
Do not stage another section's files or merge the branch.

## Completion and evidence limits

Report the four native policy results, existing accounting results, WASM compilation, and any blocked check separately.
Native tests prove transition decisions and existing receipt accounting. They do not execute Bevy cache operations.
WASM compilation proves real resource, closure, guard, and borrow types. It does not execute the render schedule.
Actual guard admission, claim transfer, retain counts, active reuse, shader reload exclusion, and GPU completion remain unverified at runtime.
Companion discovery and readiness across frames also remain unverified at runtime.
Do not run full checks, release builds, or browser acceptance under this plan.
The later readiness section consumes these queries and preserves physical drain. It does not acquire pipelines.
