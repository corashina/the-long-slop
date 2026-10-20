# G08 inventory and equipment

Date: 2026-10-20. Branch: `codex/g08-inventory`.
The branch remains unmerged and unpushed.
The [approved design](superpowers/specs/2026-10-20-g08-inventory-design.md) defines the scope.
Steam build `11140890` and its decompiled code define behavior.

## Changed behavior

A short Tab release equips eligible carry into empty equipment.
Eligible carry recenters after the Tab hold gate. Other carry keeps its immediate recenter path.
With empty carry, Tab releases equipment while standing. MMB releases it while standing or seated.
MMB is browser `Mouse1`, Rust `keys::ROTATE`, value `4096`. RMB cannot release equipment.
Equipment release creates carry ownership and keeps slot membership.
Occupied carry prevents equipment release.

Number release selects a slot. Holding a number swaps slots using the imported timing.
Released slots run before held slots. The lowest number wins each phase.
Pause, input loss, reset, and load cancel unfinished gestures.
A fresh Tab press works immediately after cancellation. A stale release cannot equip.

Each gameplay frame processes gestures, slot actions, equipment use, equipment release, and carry use in source order.
Hand interpolation advances once per frame. Fixed steps only synchronize the hand pose.
Equipment and carry have separate tool roles. Brushes require equipment.
Both owned sprays can fire. Shared particles, audio, and model updates run once per frame.
Equipped flashlights reserve LMB before carried flashlights.

Transfers validate ownership and physical state before mutation.
Rejected transfers keep slots, owners, bodies, colliders, poses, joints, contents, and tool quantity unchanged.
Slot membership remains separate from equipment ownership.
Save and Load preserve independent equipment, carry, stored items, fluids, and tool quantity.
Restore waits for real publication readiness and then synchronizes the final hand pose.
No save schema change, compatibility path, item, or acquisition route was added.

## Controls and HUD

The controls panel documents number release, number hold, Tab equip, held recenter, and seated MMB release.
The live HUD uses four new bits in the existing action mask. The browser bridge has no new field.
Rows show `Tap Tab: equip`, `Hold Tab: recenter`, `Tab or MMB: release`, or `MMB: release` when applicable.
Occupied carry suppresses release. Ineligible carry suppresses equip. Seated equipment release shows only MMB.
Movement, rotation, scroll, tool, and use hints retain their existing roles.
The scroll text now updates row 18. Its old row 19 assignment overwrote the flashlight hint.

The plan assumed `InteractionState::prompt` drove the HUD. It had no production caller or seat state.
The approved Task 5 correction removed that helper and changed live `ActionAvailability` and `web/feedback.js` instead.
The native regression checks availability bits. The web regression checks rendered row text and preserved tool hints.
Static rows are created at HUD setup. The change adds no allocation to the frame path.

## Verification

The commands below use the read-only reference export and worktree-specific Docker target.
The [plan's Docker setup](superpowers/plans/2026-10-20-g08-inventory.md#verification-commands) defines the reproducible environment.
The toolchain image build prepares dependencies. It is separate from a product release build.
The Task 5 milestone at `c9cc930` ran on 2026-10-20. It executed 19 native tests, with zero failures.
Every filter executed at least one test. The two targeted web feedback tests also passed.

| Exact Cargo command inside Docker | Passed | Failed | Log |
| --- | ---: | ---: | --- |
| `cargo test --locked --lib interaction::equipment_input::tests` | 4 | 0 | `cargo-20261020-103929-971.log` |
| `cargo test --locked --lib inventory::tests` | 1 | 0 | `cargo-20261020-103955-302.log` |
| `cargo test --locked --lib g08_frame_ -- --ignored` | 5 | 0 | `cargo-20261020-103956-825.log` |
| `cargo test --locked --lib g08_save_ -- --ignored` | 1 | 0 | `cargo-20261020-104041-709.log` |
| `cargo test --locked --lib source_transfers_reject_atomically_and_restore_both_hands -- --ignored` | 1 | 0 | `cargo-20261020-104101-631.log` |
| `cargo test --locked --lib roadside_possession_survives_owner_retirement_without_duplicate_spawn -- --ignored` | 1 | 0 | `cargo-20261020-104123-144.log` |
| `cargo test --locked --lib fixed_decoration_imported_inventory_reload_waits_for_real_readiness -- --ignored` | 1 | 0 | `cargo-20261020-104156-733.log` |
| `cargo test --locked --test inventory -- --ignored` | 5 | 0 | `cargo-20261020-104210-484.log` |

The save test and migrated transfer test live in `src/world/stream_tests.rs`.
They reuse private `prepare_ready` publication helpers. Both remain explicitly included above.
No public readiness hook or fake ready flag was added.

`scripts/check.ps1 -WebTest feedback` executed two tests. Both passed, with zero skipped tests.
Input handling did not change. The input web suite was not run.

| Exact Cargo command inside Docker | Result | Log |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Exit 0 | Runner named `cargo-20261020-104232-837.log`. No output file was created. |
| `cargo check --locked --target wasm32-unknown-unknown --workspace --lib` | Exit 0 | `cargo-20261020-104239-260.log` |
| `cargo check --locked --target wasm32-unknown-unknown --lib -p my-summer-slop --no-default-features --features webgpu` | Exit 0 | `cargo-20261020-104251-917.log` |

## Task 5 review fix 1

Date: 2026-10-20. Base: `c9cc930`.
The first Task 5 review found an equip hint without the transfer's full physical checks.
Disabling an eligible carried body left the hint visible, although `equip_carried` rejected it.
The behavioral RED ran one test and failed on the disabled body.
Log `cargo-20261020-104855-651.log` records `EQUIP = 16777216` instead of zero.

`InteractionState::can_equip_carried` now owns the complete existing validation.
Both `equip_carried` and `available_actions` use that predicate. Transfer acceptance is unchanged.
The source rule-only `EquipmentContext` and recenter timing remain unchanged.
The helper adds no allocations or retained state.
The hint regression also checks missing bodies, removed items, pending attachments, and wrong body types.
Each state suppresses the equip hint and rejects the transfer. Restored valid carry advertises equip again.

The fix run executed all five ignored inventory integration tests. All five passed with zero failures.
Log `cargo-20261020-105006-795.log` records that run.

| Exact fix check inside Docker | Result | Log |
| --- | --- | --- |
| `cargo test --locked --test inventory -- --ignored` | 5 passed, 0 failed | `cargo-20261020-105006-795.log` |
| `cargo fmt --all -- --check` | Exit 0 | Runner named `cargo-20261020-105045-144.log`. No output file was created. |
| `cargo check --locked --target wasm32-unknown-unknown --workspace --lib` | Exit 0 | `cargo-20261020-105049-759.log` |
| `cargo check --locked --target wasm32-unknown-unknown --lib -p my-summer-slop --no-default-features --features webgpu` | Exit 0 | `cargo-20261020-105106-075.log` |
The earlier 19-test milestone remains separate. Unchanged native filters and HUD web tests were not repeated.
JavaScript did not change. Browser acceptance, full checks, and product release builds remain deferred.
## Final review recovery fix

Date: 2026-10-20. Base: `5425807`.
Recovery completion calls application `clear_input` after physics. It arms the next frame's elapsed-time skip.
The old active skip discarded fresh Tab and number edges. Ruling 9 is superseded for that path.
The RED used real recovery and publication readiness. Fresh Tab failed to release retained equipment.
It executed one test, with one failure. Log `cargo-20261020-110358-698.log` records the assertion.

`Drive.frame_input` now merges raw samples until the coordinator consumes the buffer.
The shared `FrameInput::take_frame` consumes the skip flag once.
An active skipped frame retains input. A blocked skipped frame clears input.
A normal frame takes and clears the buffer, including held values, before the coordinator runs.
The coordinator still handles normal blocked frames and preserves death-look input.
Blocked skipped frames also clear the fixed buffer and runtime gesture state.
Pending save, pause, reset, load, and recovery retain their cancellation paths.
Camera changes still inspect each fresh raw sample once.
PRIMARY reservation and fixed-input merging now occur only after the coordinator.
The fix adds no queue, state field, allocation, or zero-time simulation update.

Three new native regressions use that production buffer method and the real inventory coordinator.
They cover completed recovery, retained equipment Tab release, number release, and stale canceled input.
They check once-only USE and mouse input, PRIMARY reservation, tool quantity, particles, and complete buffer consumption.
Normal blocked death-look input also remains covered.
These tests reuse private `prepare_ready`; they add no readiness hook.

| Exact final-fix command | Result | Log |
| --- | --- | --- |
| `cargo test --locked --lib g08_app_input_ -- --ignored` | 3 passed, 0 failed | `cargo-20261020-110551-859.log` |
| `cargo test --locked --lib g08_frame_ -- --ignored` | 5 passed, 0 failed | `cargo-20261020-110626-750.log` |
| `cargo fmt --all -- --check` | Exit 0 | Runner named `cargo-20261020-110703-518.log`; no output file. |
| `cargo check --locked --target wasm32-unknown-unknown --workspace --lib` | Exit 0 | `cargo-20261020-110717-616.log` |
| `cargo check --locked --target wasm32-unknown-unknown --lib -p my-summer-slop --no-default-features --features webgpu` | Exit 0 | `cargo-20261020-110751-815.log` |
| `scripts/check.ps1 -WebTest input` | 4 passed, 0 failed, 0 skipped | Terminal output. |

The input web suite checks bridge edge cancellation and number masks because application input handling changed.
It does not run the application scheduler. JavaScript remains unchanged.
These eight native tests are separate from the earlier milestone. Unaffected passing suites were not repeated.
Existing native and material warnings remain. No new warning was introduced.

## Failures and repairs

The initial inventory unit baseline passed one test.
The old export integration baseline failed with `ReconstructionFailure("Save restore is pending")` during immediate recapture after load.
Task 4 repaired the test through real publication readiness and retained the reconstruction guard.
That migrated regression passes in the final native run.
Initial and generated equipment now synchronize the final hand pose after startup alignment and body readiness.

Task 5 first ran the requested text-helper test. One test failed because eligible carry omitted `Tap Tab: equip`.
Log `cargo-20261020-103429-246.log` records that expected failure.
Inspection found that the helper was unused, so the approved correction moved coverage to the live path.
The live availability regression then failed once because the G08 mask bits were absent.
Log `cargo-20261020-103621-404.log` records that expected failure.
The first feedback web run failed both tests because rows were missing and scroll text overwrote the flashlight row.
After the changes, the focused native test and both web tests passed.
The milestone native run also includes the availability regression.

## Limits and warnings

Existing native warnings remain for `PhysicalPipelineState`, `pipeline_states_drained`, `RendererRetirement`, and its methods.
The integration build also reports unused `startup_job_audit`. Existing material warnings remain in WASM checks.
These warnings are outside G08.

The production input buffer lifecycle now has native recovery and coordinator regression coverage.
Native tests do not run Bevy `read_input`, `advance_physics`, or `sync_world` scheduling.
Their application wiring and camera/cancellation call sites have diff review and WASM compile evidence.
Full checks, product release builds, and browser acceptance were not run.
Normal browser Tab, number, tool, and Save/Load input remains unverified on a packaged build.
Native tests, DOM test doubles, and compilation do not establish browser acceptance or source parity.
G08 has code coverage for `Verify`. Browser acceptance remains open.

Local logs, task reports, and the logged runner are preserved under `.cache/g08-validation/2026-10-20/` in this worktree.
That ignored directory is local evidence. This report retains the command results in Git.
The copied runner still names the old scratch directory. The plan's Docker setup remains the reproducible recipe.

## Rulings I made

1. Read assigned task briefs plus shared execution context, rather than the entire plan in each worker. This avoids repeated context and preserves exact requirements. Cost if wrong: a missed cross-task contract, checked at task review.
2. Use GPT-6.1 Sol with high reasoning for implementation and review, as AGENTS.md requests. Never use Astra. Cost if wrong: review may need a fix round.
3. Skip full-suite instructions in generic skill templates. AGENTS.md permits targeted native tests and WASM checks only here. Cost if wrong: broader regressions remain unverified and are reported.
4. Root owns the ledger and documentation contract. Only one implementer writes product files or runs shared-target builds at a time. Cost if wrong: shared writes conflict or execution is slower.
5. Continue Task 1 with focused transfer checks. Repair old test readiness in Task 4 using existing native publication helpers, rather than weaken restore guards. Cost if wrong: delayed discovery of an unrelated restore issue, retained in the baseline report.
6. Add InventoryState::record_swap after validated successful Swap only. Rejected transfers retain progress and release-selection eligibility, matching completed-swap semantics. Cost if wrong: another input transition needs a targeted correction.
7. Extend the task to presentation/conditions.rs and anchor it after app::sync_imported, with crate visibility if needed. Old advance_tools ordering also implied imported entity sync; advance_physics alone would drop that dependency. Cost if wrong: condition visuals could update in the wrong frame; browser timing remains unverified.
8. Clear the new Drive.frame_input during the existing app/save.rs load reset in Task 3. The new transient field needs a complete lifecycle before save tests. Cost if wrong: restored input can retain stale actions; Task 4 tests cancellation.
9. On an active skip_elapsed frame, retain unrelated raw fixed-step edges after stripping RECENTER and inventory keys. Keep blocked-state cancellation for recovering, unsupported, sleeping, or dead states. The flag skips elapsed time rather than fresh input. Cost if wrong: a fresh USE edge is lost or a blocked action is queued; targeted pending-input coverage checks this.
10. Limit seated clutch inventory suppression to an actual first-slot binding collision. Cache Clutch/Digit1 collision during setup, since InventoryStuff compares the source bindings. Keep normal exported clutch usable with number slots. Cost if wrong: inventory is wrongly blocked or receives a conflicting slot action; test both binding states.
11. Put the new save regression and migrated source_transfers regression in world/stream_tests.rs to reuse real prepare_ready publication. Keep both tests explicitly invoked in final verification; do not add public readiness hooks. Cost if wrong: a filter can omit save coverage; report the exact lib commands and executed counts.
12. Replace the unused InteractionState::prompt path with live ActionAvailability bits and matching static HUD rows through the existing actions mask. Add feedback.rs, web/feedback.js, and a targeted web feedback suite to scope; no bridge field or binding change. Migrate the requested hint regression to live availability and remove obsolete prompt code. The plan incorrectly assumed prompt drove the HUD. Cost if wrong: displayed actions disagree with ownership or seat state; native availability and targeted web feedback tests check this.
