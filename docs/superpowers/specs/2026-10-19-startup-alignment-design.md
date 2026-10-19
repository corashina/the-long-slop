# Startup alignment ownership

Date: 2026-10-19.
Status: Written specification awaiting user review.
Branch: `codex/codebase-architecture`.
Base: `97783a10444c9d5faef9d1c5a10b2ff21469d33c`.

## Intent and scope

Move effective source hierarchy resolution into `StartupTerrain`.
Keep physical application and its lifecycle in `GarageRuntime`.
The product scope remains one complete Lada garage experience.
Steam build `11140890` and its decompiled behavior remain authoritative.
This refactor preserves the inspected behavior. It does not establish source parity.
No source algorithm, gameplay, scope, or save schema change belongs to this work.

The user approved the design in chat.
This written specification requires user review before an implementation plan or code changes.

## Current behavior and evidence

The following locations describe the stated base commit.

- `src/world/startup.rs:112-135` processes shared roots, player roots, and fixed roots in that order.
- `src/world/startup.rs:83-105` applies earlier root offsets while it resolves source points.
- `src/world/stream.rs:1393-1444` prepares roads before it publishes `StartupTerrain`.
- `src/starting_area.rs:1613-1620` skips completed alignment and waits for startup publication.
- `src/starting_area.rs:1621-1657` resolves exported ancestry, then external source ancestry.
- `src/starting_area.rs:1658-1672` applies offsets, refreshes queries, commits poses, and marks completion.
- `src/player.rs:687-689` updates both the player pose and the physical player body.
- `src/starting_area/save.rs:548` marks a restored runtime as already aligned.

Normal preparation calls alignment at `src/app.rs:852`.
Saved preparation calls alignment at `src/presentation/save.rs:361`.
The native preparation adapter calls alignment at `src/world/stream_tests.rs:331`.
Their order and error flow remain unchanged.

## Selected interface

Add this method in `src/world/startup.rs`:

```rust
impl StartupTerrain {
	pub(crate) fn body_offsets(
		&self,
		reference: &ReferenceManifest,
	) -> Vec<(BodyId, f32)>;
}
```

Each entry contains an existing body identity and its effective vertical offset.
The result follows `reference.bodies` order and includes zero offsets.
The method reads the checked manifest and does not change runtime state.
The existing `player_offset` field remains separate.

Use a private resolver with narrow input slices for focused native tests:

```rust
fn resolve_body_offsets(
	root_offsets: &[(SourceId, f32)],
	nodes: &[NodeRecord],
	bodies: &[BodyRecord],
	records: &[WorldSourceRecord],
) -> Vec<(BodyId, f32)>;
```

`body_offsets` supplies the existing fields to this private resolver.
The resolver owns the existing node and source record lookup maps.
Tests call this same resolver with compact source records.
No public hierarchy type, stored ancestry, or additional runtime state is required.

Keep the existing runtime interface:

```rust
pub fn align_startup(
	&mut self,
	reference: &ReferenceManifest,
) -> Result<bool, ReferenceError>;
```

## Ownership and call flow

`StartupTerrain` owns root offset generation and effective body offset resolution.
`GarageRuntime` owns readiness guards, one-time application, and all physical changes.
`PlayerState` retains its existing player pose update.

The runtime flow remains:

1. If `startup_aligned` is true, return `Ok(true)` immediately.
2. If the stream has no startup data, return `Ok(false)` immediately.
3. Resolve the complete body offset vector once.
4. Apply each offset through the existing body handle and `PhysicsWorld::edit_body`.
5. Apply `startup.player_offset` through `PlayerState::align_startup`.
6. Call `PhysicsWorld::refresh_queries`.
7. Call `ItemStore::sync_poses`.
8. Call `StartingArea::commit_poses` with `true`.
9. Set `startup_aligned` and return `Ok(true)`.

The two guard paths allocate nothing and do not build lookup maps.
A restored save takes the first guard, even when startup data is unavailable.
Body movement retains `set_position(pose, true)`.
Player movement retains its separate `set_position(self.pose, false)` behavior.
The temporary vector and lookup maps expire after resolution and application.
There is no repeated setup after successful alignment.

## Exact arithmetic and hierarchy contract

For each body, begin at its exported `BodyRecord::node` with an `f32` zero accumulator.
Walk exported nodes from that node through `NodeRecord::parent` to the exported root.
At each node, filter all matching entries from `root_offsets` in their existing order.
Compute that node's `sum::<f32>()`, then add the sum to the body accumulator.

Continue from the exported root's source record through external `m_Father` links.
Each external parent contributes its own ordered `sum::<f32>()`, followed by accumulator addition.
The exported root contributes through the first walk only.
Retain the current stop behavior when a source record has no linked parent.

Preserve both nested accumulation levels exactly.
Do not flatten sums, change precision, sort entries, deduplicate entries, or cache resolved offsets.
Do not add manifest revalidation or new hierarchy error handling inside this trusted resolver.
Do not change root group order, shared offsets, foundation influence, holes, or player offset generation.

## File map and alternatives

| File | Change |
| --- | --- |
| `src/world/startup.rs` | Add the one-shot method, private resolver, and focused resolver tests. |
| `src/starting_area.rs` | Replace hierarchy resolution with the method call. Retain guards and physical application. |
| `tests/startup_physics.rs` | Retain the installed-export test and extend lifecycle assertions where required. |

No change is required in the callers, save restoration, source schema, or module exports.
Remove the original runtime hierarchy resolver after the move.
Add no compatibility path or second implementation.

Retained ancestry was the alternative design.
It would build source chains during runtime construction and store them until startup publication.
The selected design needs no retained chains because physical alignment runs once.
Its temporary result is the smallest complete boundary between resolution and application.

## Critical tests and checks

Use compact `NodeRecord`, `BodyRecord`, and `WorldSourceRecord` values for the four resolver tests.
No complete manifest fixture or installed source export is required for these tests.

1. `exported_and_external_ancestors_add_offsets` covers multiple ancestors in both walks and the exported-root boundary.
2. `duplicate_root_entries_keep_inner_sum_order` compares exact bits for matching offsets `[16777216.0, 1.0, -16777216.0]`.
3. `ancestor_contributions_keep_outer_sum_order` uses successive contributions `[16777216.0, -16777216.0, 1.0]` and expects exact `1.0` bits.
4. `body_offsets_keep_body_order_and_unmatched_zero` uses reversed body identities and a body without matching roots.

Keep `installed_spawn_alignment_and_settling` as the fifth test.
Extend its assertions for pending alignment, repeated application, and the restored-save skip if current coverage lacks them.
Preserve its player synchronization, vehicle settling, wheel contact, and persistent pose assertions.
This test requires the installed source export and is ignored by default.
It uses native physics. It does not establish browser acceptance or source parity.

The targeted project command includes formatting, native unit tests, and the WASM compile check:

```powershell
scripts/check.ps1 -Filter world::startup::tests
```

Run the installed-export test explicitly inside the existing project toolchain when the export is available:

```text
cargo test --locked --test startup_physics installed_spawn_alignment_and_settling -- --ignored
```

`scripts/check.ps1 -Test startup_physics` alone skips that ignored test.
Report missing exports, failed checks, and deferred checks separately.
Full checks, release builds, and browser acceptance remain deferred until explicitly requested or agreed.

## Completion criteria

Startup owns both hierarchy walks and their exact arithmetic.
Runtime retains all guards, physical mutations, player application, query refresh, and pose commits.
Pending and restored paths perform no resolution or allocation.
The critical native tests and WASM compile check pass, or their concrete blockers are reported.
No runtime parity claim follows from this refactor or its native checks.
