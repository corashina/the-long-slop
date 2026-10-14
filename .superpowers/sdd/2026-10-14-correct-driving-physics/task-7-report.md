# Task 7 integration report

Status: IN_PROGRESS. Product merge is ready for review. Release build and focused checks remain active.
The controller owns the final master fast-forward and preservation of primary edits.

## Integration identity

Worktree: `C:/Users/Corashina/.codex/worktrees/correct-driving-physics/the-long-drive-wasm`.
Branch: `codex/correct-driving-physics`.
First parent: `b0d6e3b2b22178a9f60b271e67f131d978b91ecf`.
Second parent: `0a4ed1d60b3285cebde5ebecb05d5850d2f93547`.
Master remains at the recorded second parent. We did not change its ref or primary files.
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

## Verification so far

Exact commands and exits: `commands.jsonl`. Raw logs retain failed attempts.
The build uses `my-summer-slop-toolchain`, the existing target namespace, and `CARGO_BUILD_JOBS=2`.
Target namespace: `/target/f36ed296a678ad0f90768b12c9c8456e8e4d9a9ea255f947dcf1661199c24a14`.

- WASM `cargo check --locked --target wasm32-unknown-unknown --lib`: exit 0; 11 warnings.
- Web suite: exit 1; 92 passed, 1 failed.
- JavaScript syntax: seven integration files checked. See `js-syntax.log` and command receipt.
- Scoped Rust formatting: exit 0.
- Source check: exit 0. Four source/library receipts and two managed-source identities match.
- Native all-target compile: blocked by removed APIs in existing master tests.
- Broad native run: incomplete. We stopped its owned container under memory pressure.
- Completed broad-run targets: 18. Observed results: 26 passed, 143 failed, 16 ignored.

`native-summary.json` lists each completed target exit, observed failure name, and compiler message.
These are partial-run counts. They are not a replacement complete-suite baseline.
The earlier 108 failures describe a different commit.
The first broad attempt stopped after a stale fixture section surfaced. We retained both raw attempts.

## Known master blockers

Committed master retains tests using removed station and synchronous stream APIs.
These include `insert_supply`, `register_pump`, `sync_at`, `reset_pose`, `road_sample`, and `chunks`.
The tracked master fixture also lacks the required world section and source spawn car fields.
Current fixture-dependent checks fail before simulation. Their results do not establish a new driving defect.
We preserved supported assertions in `save_world`, `save_capture`, `interaction`, and `starting_area`.
We added no ignored tests, no compatibility helper, and no tolerance changes.
Master already deleted the obsolete generator tests from `tests/world.rs`; the merge retains that deletion.
The remaining current rebase check retains its assertions and uses the driving torque API.

`late_prepare_after_cancel_is_discarded` is the single web failure.
It expects token 7. The unchanged master bridge discards request identity 1 after cancellation.
Both the bridge and that test match committed master. We did not change their behavior.

The accepted primary runtime manifest exceeds the committed 128 MiB limit.
The controller owns a separate local limit edit. This merge does not copy that edit or change the gate.
`baseline-contract-evidence.json` records the committed-master limit and old test callers.

## Runtime composition

A complete export attempt failed because retained world probe files are absent.
We did not create samples or bypass the world-input gate.
The controller authorized a snapshot of the accepted primary runtime with before/after file hashes.
We extract fresh driving values from the unchanged source library into a staged copy.
The composition preserves the serialized world section and existing asset bytes.
Only `manifest.json` and new `driving-evidence.json` form the proposed runtime promotion delta.
`runtime-input-hashes.json` records the stable input snapshot.
Composition, strict validation, package hashes, and final commit receipts will follow in this report.

## Limits

Native source driving measurements remain absent. The stock wheel solver remains in place.
Live launch, continuous driving, both-car routes, and source parity remain unverified.
Road seams, source-world revisits, rebasing routes, and memory plateau need runtime acceptance.
No source capture, primary edit, master update, push, or worktree cleanup occurred.
The roadmap keeps Driving physics Partial and preserves source-world status.
