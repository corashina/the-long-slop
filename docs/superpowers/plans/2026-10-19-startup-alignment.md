# Startup alignment ownership implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move body offset resolution into startup without changing alignment behavior.

**Architecture:** `StartupTerrain` resolves a temporary body offset vector after the existing runtime guards. `GarageRuntime` applies the offsets once and retains physical state ownership. A private resolver accepts compact source records for native tests.

**Tech Stack:** Rust 2024, Rapier 0.36.0, existing manifest types, native Rust tests, and WASM compilation.

**Spec:** [Approved startup alignment specification](../specs/2026-10-19-startup-alignment-design.md).

Status: Awaiting user review. The user approved the specification and selected subagent execution.
Date: 2026-10-19.
Worktree: `C:/Users/Corashina/.codex/worktrees/codebase-architecture/the-long-drive-wasm`.
Branch: `codex/codebase-architecture`.
Starting commit: `8ce8b0c4350f6f3c1cae332f1fd4f0274bbd990f`.
Root coordinates reviews and commits. Workers do not commit or merge.
Execute after save restore and before terrain jobs, pipeline ownership, and renderer readiness.
Do not edit shared production files concurrently with another section.

## Global Constraints

- The product scope remains one complete Lada garage experience.
- Steam build `11140890` and its decompiled behavior remain authoritative.
- No source algorithm, gameplay, scope, or save schema change belongs to this work.
- Each entry contains an existing body identity and its effective vertical offset.
- The result follows `reference.bodies` order and includes zero offsets.
- The existing `player_offset` field remains separate.
- The two guard paths allocate nothing and do not build lookup maps.
- Preserve both nested accumulation levels exactly.
- Do not flatten sums, change precision, sort entries, deduplicate entries, or cache resolved offsets.
- Do not add manifest revalidation or new hierarchy error handling inside this trusted resolver.
- Do not change root group order, shared offsets, foundation influence, holes, or player offset generation.
- Add no compatibility path or second implementation.
- Full checks, release builds, and browser acceptance remain deferred until explicitly requested or agreed.

The selected boundary adds no retained ancestry or runtime state.
Only targeted native tests and the WASM compile check are authorized during implementation.
The branch remains unmerged. Native checks do not establish browser acceptance or source parity.

## Review Focus

Each case maps to a critical test in Task 1.

1. A body has exported and external ancestors. All ancestors contribute once in the existing walk order.
2. One source has duplicate offsets. The inner `sum::<f32>()` preserves root entry order.
3. Large offsets cancel across ancestors. The outer accumulator preserves body-to-root order and exact `f32` bits.
4. Bodies have reversed identities or no matching roots. Output preserves body order and includes zero offsets.
5. Startup is pending, already applied, or skipped after restoration. Guards preserve poses before any resolver setup.

## File map

| File | Responsibility |
| --- | --- |
| `src/world/startup.rs` | Add the method, private resolver, and private native unit tests. |
| `src/starting_area.rs` | Replace inline resolution. Preserve guards and physical application. |
| `tests/startup_physics.rs` | Extend the existing ignored installed-export lifecycle test. |

No new production module, dependency, schema, or public export is required.
`StartupTerrain::new`, `PlayerState::align_startup`, save restoration, and callers retain their behavior.

---

### Task 1: Move resolution and preserve application

**Files:**

- Modify: `src/world/startup.rs`.
- Modify: `src/starting_area.rs:1613-1672` at the starting commit.
- Test: `src/world/startup.rs`, new `#[cfg(test)] mod tests`.
- Test: `tests/startup_physics.rs`, existing `installed_spawn_alignment_and_settling`.

**Interfaces:**

Consumes existing `ReferenceManifest`, `NodeRecord`, `BodyRecord`, `WorldSourceRecord`, `SourceId`, and `BodyId` types.
The manifest supplies `nodes`, `bodies`, and `world.records` directly.
The runtime interface remains unchanged:

```rust
pub fn align_startup(
	&mut self,
	reference: &ReferenceManifest,
) -> Result<bool, ReferenceError>;
```

Produces these interfaces in `src/world/startup.rs`:

```rust
impl StartupTerrain {
	pub(crate) fn body_offsets(
		&self,
		reference: &ReferenceManifest,
	) -> Vec<(BodyId, f32)>;
}

fn resolve_body_offsets(
	root_offsets: &[(SourceId, f32)],
	nodes: &[NodeRecord],
	bodies: &[BodyRecord],
	records: &[WorldSourceRecord],
) -> Vec<(BodyId, f32)>;
```

- [ ] **Step 1: Add four failing resolver tests.**

Place the tests inside startup's private test module. Import `super::resolve_body_offsets` and the existing reference types.
Tests access the private production resolver directly. No production visibility change or complete manifest fixture is required.
Define these test-only constructors in that module:

```rust
fn source(path_id: i64) -> SourceId;
fn node(id: u32, parent: Option<u32>) -> NodeRecord;
fn body(id: u32, node: u32) -> BodyRecord;
fn record(id: i64, parent: Option<i64>) -> WorldSourceRecord;
```

Use `SourceId { file: "startup-test".into(), path_id }` for `source`.
For `node`, set `id = NodeId(id)`, `source = source(i64::from(id))`, and `parent = parent.map(NodeId)`.
Set `name` and `hierarchy_path` to empty strings, `asset = AssetId(0)`, `gltf_node = 0`, and `active = true`.
Use an identity `Transform`: translation `[0.; 3]`, rotation `[0., 0., 0., 1.]`, and scale `[1.; 3]`.
For `body`, set `id = BodyId(id)`, `node = NodeId(node)`, and `source = source(i64::from(node))`.
Set `dynamic = true`, `mass_kg = 1.`, both damping values to zero, identity `pose`, and `parent_body = None`.
For `record`, use `source(id)`, class `"Transform"`, and deserialize `WorldFields` from the following JSON array.

```rust
let fields = match parent {
	Some(parent) => serde_json::json!([{
		"name": "m_Father",
		"value": { "kind": "link", "value": source(parent) }
	}]),
	None => serde_json::json!([]),
};
```

`serde_json::from_value(fields).unwrap()` creates the existing immutable `WorldFields` type.
The empty array represents a root with no linked parent. It does not require a full source record export.

Add each named test with these exact inputs and assertions:

1. `exported_and_external_ancestors_add_offsets` uses nodes `1 -> 2 -> 3`, with exported root `3`.
   Records link `3 -> 4 -> 5`; record `5` has no parent.
   Assign offsets `1., 2., 4., 8., 16.` to sources `1` through `5`.
   Resolve `[body(7, 1)]` and assert `vec![(BodyId(7), 31.)]`.
   This detects a lost external parent or an exported root counted twice.
2. `duplicate_root_entries_keep_inner_sum_order` uses one node `1`, one body `7`, and no source records.
   Root offsets for source `1` are `[16777216.0, 1.0, -16777216.0]` in that order.
   Assert one result, identity `BodyId(7)`, and offset bits equal to `0.0_f32.to_bits()`.
3. `ancestor_contributions_keep_outer_sum_order` uses nodes `1 -> 2` and the external link `2 -> 3`.
   Offsets are source `3 = 1.`, source `1 = 16777216.`, source `2 = -16777216.` in that vector order.
   Resolve `[body(7, 1)]` and assert offset bits equal to `1.0_f32.to_bits()`.
   This separates hierarchy order from vector order and ancestor-first accumulation.
4. `body_offsets_keep_body_order_and_unmatched_zero` uses independent nodes `1` and `2`, with no source records.
   Assign only source `1 = 2.` and resolve `[body(9, 1), body(3, 2)]`.
   Assert `vec![(BodyId(9), 2.), (BodyId(3), 0.)]` and positive zero bits for the unmatched body.

- [ ] **Step 2: Run the RED check.**

Run from the stated worktree:

```powershell
scripts/check.ps1 -Filter world::startup::tests
```

Expected: compilation fails because `resolve_body_offsets` does not exist.
Resolve unrelated formatting or environment failures before treating the RED check as useful evidence.

- [ ] **Step 3: Extend the installed lifecycle characterization.**

Use the existing `reference` and `run` variables in `tests/startup_physics.rs`.
Before its startup preparation loop, snapshot the vehicle pose and player body pose.
Assert `run.stream.frame().startup.is_none()` and `!run.align_startup(&reference).unwrap()`.
Assert both poses and `run.startup_aligned == false` remain unchanged.
Set `run.startup_aligned = true` to model the flag set by save restoration at `src/starting_area/save.rs:548`.
Assert alignment returns true with startup still absent and both poses unchanged. Reset the flag before existing preparation.
Retain the existing post-publication player synchronization, vehicle offset, repeat call, settling, and persistent pose assertions.
After first application, assert each body render tuple has equal previous and current poses.
For each active body, assert its current render pose equals `from_rapier(run.physics.rapier().bodies[run.area.bodies[id]].position())`.
Use `run.physics.body_active(run.area.bodies[id])` for this predicate, consistent with `StartingArea::commit_poses`.
For items with `inactive_pose.is_none()`, assert equal previous and current render poses and agreement with `run.area.items.get(*id).unwrap().pose`.
These assertions use existing public fields and methods. No test interface or fresh full manifest fixture is needed.
This characterization preserves existing behavior. It is not expected to fail before extraction.

- [ ] **Step 4: Implement the extraction and runtime call.**

Move both lookup maps and hierarchy walks from `GarageRuntime::align_startup` into `resolve_body_offsets`.
For each body, retain `offset = 0.0_f32` and the exported node-to-root walk.
At every node, preserve ordered `filter`, `map`, `sum::<f32>()`, then `offset += sum`.
Continue from the exported root through external `m_Father` parents with the same nested accumulation.
Preserve stopping at a missing source record or a source record without a linked parent.
Return every body identity with its resolved offset, in `reference.bodies` order.
Make `body_offsets` pass its roots and the manifest's existing slices to the private resolver.
Keep `StartupTerrain::new` unchanged, including source root order and player offset generation.

In `GarageRuntime::align_startup`, retain the completed guard before the startup availability guard.
Call `startup.body_offsets(reference)` only after both guards, then apply the returned vector once.
Keep body handle lookup and `edit_body` in the runtime, with `set_position(pose, true)`.
Keep separate player application through `PlayerState::align_startup`, which retains `set_position(self.pose, false)`.
Keep this order: body changes, player change, query refresh, item pose synchronization, pose commit, completion flag.
Remove the inline hierarchy resolver. Retain shared imports still used elsewhere in `starting_area.rs`.
No cached offsets, retained ancestry, validation pass, compatibility function, or new error branch is permitted.

- [ ] **Step 5: Run GREEN checks and review the diff.**

Run `scripts/check.ps1 -Filter world::startup::tests` again.
Expected: four native resolver tests pass, formatting passes, and the workspace WASM library check passes.
If the installed export is available, run the ignored lifecycle test in the configured Docker toolchain:

```powershell
$startupRoot = (Get-Location).Path
. ./scripts/cargo-target.ps1
$startupTarget = Get-CargoTargetDirectory $startupRoot
docker run --rm `
	--mount "type=bind,source=$startupRoot,target=/workspace" `
	--mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
	--mount 'type=volume,source=tld-wasm-target,target=/target' `
	--env "CARGO_TARGET_DIR=$startupTarget" my-summer-slop-toolchain `
	cargo test --locked --test startup_physics installed_spawn_alignment_and_settling -- --ignored --exact
```

The project check builds this toolchain image first. The bind mount supplies `web/assets/reference/manifest.json` and installed fixtures.
Expected: one executed lifecycle test passes. Zero executed tests does not satisfy this check.
If the export or required fixtures are absent, report this integration check as deferred with the concrete missing path.
`scripts/check.ps1 -Test startup_physics` alone skips the ignored lifecycle test.
Report toolchain failures separately. Run no full checks, release builds, or browser acceptance.
Review unchanged startup generation, guards before allocations, exact arithmetic, physical order, restored skip, and unchanged callers.

- [ ] **Step 6: Root reviews and commits the tested change.**

Root stages only `src/world/startup.rs`, `src/starting_area.rs`, and `tests/startup_physics.rs` after independent review.
Use commit message `refactor: resolve startup body offsets in startup`.
Report targeted results, failures, deferred integration checks, and unverified browser behavior.

## Review and execution boundary

The plan covers the approved interface, both ancestry walks, arithmetic order, guards, and physical application.
The four compact tests run without installed exports. The lifecycle test retains its explicit fixture requirement.
No implementation starts until the user reviews this plan.
Continue with the already selected subagent method after approval. Do not ask the user to select an execution method again.
