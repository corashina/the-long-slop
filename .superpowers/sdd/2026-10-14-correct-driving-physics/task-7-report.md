# Task 7 integration report

Status: DONE_WITH_CONCERNS. The local merge, roadmap update, scoped review, and release package are complete.
The controller advanced master and preserved primary edits.

## Integration identity

Worktree: `C:/Users/Corashina/.codex/worktrees/correct-driving-physics/the-long-drive-wasm`.
Branch: `codex/correct-driving-physics`.
First parent: `b0d6e3b2b22178a9f60b271e67f131d978b91ecf`.
Second parent: `0a4ed1d60b3285cebde5ebecb05d5850d2f93547`.
At merge creation, master remained at the recorded second parent. We did not change its ref or primary files.
Merge commit: `5144158b8f1fee7cf43b013716887b9ed9943ba1`.
Merge tree: `22b271b7c89a0d72359ba5d5bf6d431a7c009f95`.
Roadmap clarification: `e494835bb6f468528ec8d8b0c08fde2d74c4d25e`.
The roadmap commit changes documentation only. Final package and critical checks use the merge's unchanged product source.
Evidence directory: `.cache/driving-physics/merge-master/`.

## Conflict decisions

We resolved 14 conflicts by region. We did not select an entire side through checkout.

- `PRODUCT.md`: retain implemented flashlights and plates, driving assists, and current exclusions.
- `src/reference.rs`: retain both driving fields and source spawn car identity.
- `tools/reference/manifest.py`: export both fields and retain source keyboard steering.
- `tests/common/reference.json`: retain driving records; remove the obsolete station section; add checked source spawn identities.
- `web/main.js`: retain source-world admission and measurements; initialize input with exported driving bindings.
- `tests/web/menu.test.mjs`: retain both assist and failed-world-admission checks.
- `src/presentation/lighting.rs`: retain the driving audit and source-world warmup predicate and pipeline isolation.
- `src/presentation/materials.rs`: retain source-world recipe readiness and caster capacity; remove obsolete prototype query ownership.
- `src/presentation/motion_draw.rs`: retain source-world renderer filtering, authored motion, capacity growth, and preparation guard.
- `src/presentation/world.rs`: retain the source-world modules and remove the obsolete terrain and station owners.
- `src/presentation/world/lamps.rs`: retain source-world lamp ownership; remove obsolete prototype helper calls.
- `src/presentation/world/prototype.rs` and `stations.rs`: retain master's deletion of the obsolete owners.
- `tests/world.rs`: retain master's remaining rebase check and connect the driving torque API.

The automatic merge duplicated `ReferenceManifest` imports in `src/bin/validate-reference.rs`. We combined those imports.
We retained source-world code, its vendor patch, and driving input, transmission, save, and assist owners.
We added no dependency, compatibility reader, fallback, or migration.

## Current save contract

Both parents used schema 2 for different contracts. The combined contract uses schema 3.
Schema 3 retains generated changes, spawn history, selected gears, and both assists.
The reader rejects schema 1 and both schema 2 variants before reading vehicle fields.
Browser database version remains unchanged.
We updated current browser/schema fixtures and normalize generated change tuples by their stable key.
The schema identity check now uses generated identities instead of the removed station identity.

## Verification

Exact commands and exits: `commands.jsonl`. Raw logs retain failed attempts.
The build uses `my-summer-slop-toolchain`, the existing target namespace, and `CARGO_BUILD_JOBS=2`.
Target namespace: `/target/f36ed296a678ad0f90768b12c9c8456e8e4d9a9ea255f947dcf1661199c24a14`.

- Release WASM build and `wasm-bindgen`: exit 0; 35m09s release compilation; 11 warnings.
- WASM `cargo check --locked --target wasm32-unknown-unknown --lib`: exit 0; 11 warnings.
- Web suite: exit 1; 92 passed, 1 failed.
- JavaScript syntax: exit 0 for seven integration files.
- Focused driving inputs: exit 0; 3 passed. Timing, independent ramps, and single-consumption edges pass.
- Schema rejection: exit 0; 1 passed. Versions 1 and 2 reject before vehicle parsing.
- Native library and binary `cargo check --locked --lib --bins`: exit 0.
- Scoped Rust formatting: exit 0.
- Source check: exit 0. Four source/library receipts and two managed-source identities match.
- Native all-target compile: blocked by removed APIs in existing master tests.
- Broad native run: incomplete. We stopped its owned container under memory pressure.
- Completed broad-run targets: 18. Observed results: 26 passed, 143 failed, 16 ignored.

`native-summary.json` lists each completed target exit, observed failure name, and compiler message.
These are partial-run counts. They are not a replacement complete-suite baseline.
The earlier 108 failures describe a different commit.
The first broad attempt stopped after a stale fixture section surfaced. We retained both raw attempts.

## Package identity

The package uses the product source from merge `5144158b8f1fee7cf43b013716887b9ed9943ba1`.
Its tree is `22b271b7c89a0d72359ba5d5bf6d431a7c009f95`.
HEAD at completion was `e494835bb6f468528ec8d8b0c08fde2d74c4d25e`.
That commit and this final report update change documentation only.
The product diff from the reviewed merge is empty.
`package-identity.json` records all four output identities. `package.log` records the completed build.

| Package file | Bytes | SHA256 |
| --- | ---: | --- |
| `my_summer_slop_bg.wasm` | 65,300,371 | `5bf55a549797f9e6cfb7bc0139962758c75f827aa4201809b450ffe46f2636de` |
| `my_summer_slop.js` | 111,384 | `0150e8c1410927d8cadc9fee78bf570ed9f069f1f0c59313dedd6d9632d754ef` |

The package remains in this worktree's `web/pkg` directory. It was not installed into primary.
The controller received PACKAGE READY after both release compilation and binding generation completed.
The clean package retains the 128 MiB cap. Build success does not establish startup or later manifest validation.

## Known master blockers

Committed master retains tests using removed station and synchronous stream APIs.
These include `insert_supply`, `register_pump`, `sync_at`, `reset_pose`, `road_sample`, and `chunks`.
The recorded master-parent fixture lacked the world section and source spawn car fields.
This merge adds checked spawn fields. The required world fixture remains absent.
Current fixture-dependent checks fail before simulation. Their results do not establish a new driving defect.
We preserved supported assertions in `save_world`, `save_capture`, `interaction`, and `starting_area`.
We added no ignored tests, no compatibility helper, and no tolerance changes.
Master already deleted the obsolete generator tests from `tests/world.rs`; the merge retains that deletion.
The remaining current rebase check retains its assertions and uses the driving torque API.

The following deletions already exist in master. Each test depended on the listed removed path.

| Test in `tests/world.rs` | Obsolete path |
| --- | --- |
| `overlapping_lateral_bands_share_ground_triangles` | `generate_chunk_in_band` and `ChunkData` |
| `adjacent_chunks_share_the_seam` | `generate_chunk` and `ChunkKey` |
| `stations_and_active_window_follow_absolute_distance` | `Station`, `nearest_station`, `active_chunks`, and `road_sample` |
| `streaming_bounds_colliders_and_loads_teleport_destination` | synchronous `sync_at`, `chunks`, and station `reset_pose` |
| `driving_crosses_chunk_seams_and_rebase_without_trip_jump` | synchronous `sync_at`, `chunks`, and `reset_pose` |
| `lateral_streaming_supports_travel_beyond_the_old_edge` | synchronous lateral chunk generation and `chunks` |
| `walking_focus_rebases_a_parked_vehicle_without_travel` | `WorldStream::new(seed, stations)` and the deleted station fixture |
| `garage_terrain_keeps_interiors_clear_and_connects_driveway` | `configure_starting_area`, generated `ChunkData`, and synchronous `sync_at` |
| `road_lamp_intervals_have_one_owner_at_seams` | `road_lamp_range` |
| `road_lamp_placement_is_absolute_after_rebase` | `RoadLampId`, `road_lamp_placement`, and `road_sample` |
| `road_lamp_window_bound_comes_from_actual_chunks` | `road_lamp_range`, synchronous `sync_at`, and `chunks` |
| `station_food_stays_consumed_after_stream_rebase_and_recovery` | `sync_station_food` and prototype station records |

The schema identity test replaces `InstanceId::Station` with `InstanceId::Generated` and preserves its round-trip assertion.
It removes `restored.station() == (u64::MAX, i64::MIN)`, which called the deleted station identity accessor.

`late_prepare_after_cancel_is_discarded` is the single web failure.
It expects token 7. The unchanged master bridge discards request identity 1 after cancellation.
Both the bridge and that test match committed master. We did not change their behavior.

The accepted primary runtime manifest exceeds the committed 128 MiB limit.
The primary checkout has a separate uncommitted limit edit, which the controller preserved.
Another active task owns that edit. This merge does not copy it or change the gate.
`baseline-contract-evidence.json` records the committed-master limit and old test callers.

## Runtime composition

A complete export attempt failed because retained world probe files are absent.
We did not create samples or bypass the world-input gate.
The controller authorized a snapshot of the accepted primary runtime with before/after file hashes.
We extract fresh driving values from the unchanged source library into a staged copy.
The composition preserves the serialized world section and existing asset bytes.
Only `manifest.json` and new `driving-evidence.json` formed the runtime promotion delta.
`runtime-input-hashes.json` records the stable input snapshot.
Composition finished. We copied and checked 7,886 input files before and after the snapshot.
The existing world bytes and 7,885 non-manifest files match their input hashes.
`composition.json` records the identities. The staged directory is `runtime-composed/` under the evidence directory.

Base manifest: 136,567,103 bytes; SHA256 `2106ae7b5c5b8856af6ef623e76e9f70b0c28b6186ed75fb0116371050ba3803`.
Combined manifest: 136,571,197 bytes; SHA256 `dc951346da64dcbd30feb252e4ce18dcd0383f27c09b9d2368b37c3593c17522`.
Driving evidence: 19,732 bytes; SHA256 `d8b5e85d5912bdcf81f342cc66cb45c8b18a6a1a54bc8b6c99d6c9fafb55b731`.
Serialized world section SHA256: `1415246242502f4817981895f8a44ff2402e6f3be1320d7dbf20e5e7d4a935aa`.

Strict validation exited 1: `Runtime manifest contract: Invalid reference field: manifest size`.
The combined manifest exceeds the unchanged 134,217,728-byte cap. The base manifest already exceeds it.
Validation stops at that cap. We claim no validation result for fields beyond that check.
The pre-existing primary change uses a 256 MiB cap. We did not copy or apply it.
The controller checked the base hash before applying the two-file delta.
We did not promote assets or packages to primary.

## Scoped review

The controller's reviewer checked merge `5144158` and found no Critical or Important integration defect.
See `task-7-review.md`. We dispatched no reviewer or subagent from this task.

## Controller integration

The controller advanced local master to `e494835bb6f468528ec8d8b0c08fde2d74c4d25e`.
It restored 23 primary snapshot files and verified their exact three-way result.
The index is empty. No unmerged paths remain.
The only restore conflict was `web/main.js`. It retains merged `let input` and the primary timeout removal.
Recovery stash `329dde664f30663038826496fa13f48c2d0de087` remains available.
The primary receipt is `.cache/driving-physics/merge-master/master-merge.json`.

The controller matched the base manifest hash, then promoted the two-file runtime delta.
It retained the original manifest in its primary backup and left the other 7,885 assets unchanged.
The primary receipt is `.cache/driving-physics/merge-master/runtime-promotion.json`.
No clean-worktree package was installed into primary. Another owner's primary build remains separate.

The controller reports 59 passing primary web checks and a passing `web/main.js` syntax check.
These cover input, menu, feedback, startup, save-session, and save-storage.
The startup test is another owner's preserved untracked file.
The primary log is `.cache/driving-physics/merge-master/primary-web.log`.
These results include restored local work. They are separate from the clean merge's 92-pass, one-failure web run.

## Limits

Native source driving measurements remain absent. The stock wheel solver remains in place.
Live launch, continuous driving, both-car routes, and source parity remain unverified.
Road seams, source-world revisits, rebasing routes, and memory plateau need runtime acceptance.
This implementer made no source capture, primary edit, master update, push, or worktree cleanup.
The roadmap keeps Driving physics Partial and preserves source-world status.
