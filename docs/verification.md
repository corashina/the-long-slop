# Verification record

## Cleaning and painting: 2026-10-14

Branch: `codex/cleaning-painting`. Reviewed code head: `6ba18866f20e00126feafcf4aa1863b3b50bde8c`.
The [approved specification](superpowers/specs/2026-10-14-cleaning-painting-design.md) governs this feature.
Read the [source record](research/2026-10-14-cleaning-painting-reference.md) for identities, values, and source rules.
We keep Cleaning and painting Partial. Implementation and menu readiness do not complete gameplay acceptance.

We implemented six authored startup tools, live surface/wash/tool state, brush wear, and carried-nozzle spray painting.
We retain colors and condition state through part ownership and save schema 2.
The loader rejects old schemas and invalid candidates. It preserves live bindings during failed candidate preparation.
New Game restores exported startup values. Recovery preserves preparation; Load clears transient contact and valve output.
We removed duplicate scalar ownership and temporary cleaning diagnostic routes. The checked startup initializer remains necessary.

### Fresh integrated checks

We ran the source condition suite once with the primary reference Python executable and `PYTHONPATH=.`.
Command: `python.exe -m unittest tools.reference.tests.test_conditions`. Result: exit 0; three tests, OK.
We ran `.cache/retry-cleaning.py` once through that executable.
It reads the accepted library receipt, then calls `load_snapshot`, `derive_reference`, and `verify_reference`.
Result: exit 0; 663 colliders, 232 items; `fresh export and verification passed`.
The exporter checks the production Rust contract, condition evidence, and selected resource receipts.
It removed its own staging directory after promotion. Steam and accepted library inputs remain unchanged.

We corrected a Windows PowerShell launcher error before the integrated suite began.
The old launcher lacks `SHA256.HashData`; it stopped in `scripts/cargo-target.ps1:4` before any check.
We then ran `pwsh -NoProfile -File ./scripts/check.ps1` with the installed PowerShell 7 executable.
Result: exit 1; 93 web tests, 92 passed, one failed.
The existing `toilet_keys_and_primary_release_are_distinct` assertion failed at `tests/web/input.test.mjs:18`.
Actual control bits: 1573376. Expected control bits: 786944.
This failure matches the recorded baseline input limit. We kept its assertion and production controls unchanged.
The script stopped before formatting, native tests, and release WASM compilation.

We retain the named Task 4 evidence for unchanged native code:

| Target | Result | Evidence log |
| --- | --- | --- |
| cleaning | 25 passed | `task4-final-cleaning.log`, `task4-final-owner-check.log` |
| conditions / conditions_reference | 5 / 8 passed | `task4-affected-final.log` |
| save_roundtrip | 12 passed, 4 ignored | `task4-save-final.log` |
| save_schema / wheels | 4 / 7 passed, 1 / 2 ignored | `task4-affected-final.log` |
| interaction | 29 passed, 19 failed, 1 ignored | `task4-interaction.log` |
| full native / audio | Stale test API compile errors | `task4-full.log`, `task4-affected.log` |

The exact interaction failure set matches the Task 3 set and its Task 2 fixture-only counterfactual.
Full native and audio checks remain open because unchanged tests use removed APIs.
We did not repeat unchanged broad suites or restore obsolete APIs.
Task 4 fix1 passed 14 web tests and the WASM library check after the integer count cache correction.
Those logs are `task4-fix1-web.log` and `task4-fix1-wasm.log`.
Local logs remain in `.superpowers/sdd/2026-10-14-cleaning-painting/` outside Git.

### Final package identity

The final Task 4 fix1 release build passed: exit 0; release compile 4m47; wasm-bindgen completed.
Log: `task4-fix1-build.log`. The approved ruling permits reuse for docs-only Task 5.
We checked the following SHA256 hashes before and after source verification. They match the frozen fix1 package.

| File | SHA256 |
| --- | --- |
| `web/pkg/my_summer_slop_bg.wasm` | `d5a554a973f94be9e98fe226f0eefd318c8d25aa8d1a096e45f2f1dc1b258425` |
| `web/pkg/my_summer_slop.js` | `f9052bc0688392d973daeabbc9ef549684c60e5e6397106f0f245089603a8ec8` |
| `web/feedback.js` | `5d464dd6adbf5fce3cdedf47c986cfc1625018fbed3d5ec908bafb92b1014328` |
| `web/assets/reference/manifest.json` | `46db456b73b7d78d4ba532750ce4b39fd6fcc12a89b4cd80a2f03731926d72ac` |
| `web/assets/reference/conditions-evidence.json` | `f5eb8096ec605346535e7c992a6f5234f490dc4a7f4ec71fd7e574d68f48e03c` |

The final `library-selection.json` hash is `f65428109d326b96f456976de5213868790706141ddd9663d06d9cdf1291a75f`.
The accepted receipt hash is `a87176a07c287a9c8ccf7ae977c12c46dd1dc811017d965d8b5b44e599d30b41`.
Generated assets and source receipts remain outside Git.

### Browser evidence and open gates

The controller checked the final package at `http://127.0.0.1:4178/` in IAB tab 2.
Fresh navigation reached garage readiness. New Game and same-export Load reached enabled pause controls.
Continue after Load showed `Mouse capture failed` and retained pause. The controller performed no active movement or tool action.
Fresh browser inventory still omitted Chrome despite the user's connection reports.
The current error-level query returned no entries. Startup event capture remains incomplete.
Bevy can publish warnings at log level. The error query does not establish warning-free operation.

Earlier Task 2 diagnostic packages proved retained receiver/caster GPU identities, independent material changes, and candidate isolation.
They retained 1118 meshes, 122 owners, 168 receivers, 1680 condition materials, 2266 source materials, and capacity 5812.
Those named packages predate spray effects. They do not prove final can buffers or active particle/binding counts.

Complete the following route before marking the feature complete:

- Use all six tools on eligible surfaces of both cars. Remove and reinstall changed parts, then drive both cars.
- Check wheels, doors, fluids, lights, plates, storage, and existing save behavior through active input.
- Check pause, death gating, recovery, save/load, repeated restart, and origin changes during preparation and driving.
- Compare matched source actions, controls, surfaces, tool state, camera, main view, shadows, mirrors, reflections, and audio.
- Measure final material, particle, and binding counts after repeated active work and load/restart.

The fixed build hash color policy does not reproduce Unity's random sequence.
The controller owns independent whole-branch review and branch integration.

## Rendering merge: 2026-10-12

The user authorized a local merge of rendering commit `67798ec` into master `a5b9778`.
The merge retains audio, save/load, both source vehicles, and live mirrors.
Merged verification remains pending. Earlier branch results describe their recorded packages only.
The user did not authorize a push. Retain the rendering worktree and its local evidence.

## Save lifecycle review fixes: 2026-10-12

The fix range starts at `028bc222f0609f1e45923b25b3ec2cf0cd92ecd0` on `codex/save-load`.
The frame reads lifecycle input before save dispatch. Dispatch precedes lighting bindings and fixed simulation steps.
Capture clears input gestures. Commit clears the sampled frame and replaces the prepared runtime before simulation resumes.

The session calls `commit(token, onDispatch)` after preparation.
The bridge calls `onDispatch` when Rust takes the queued request for synchronous processing.
The callback remains outside the serialized request.
Before dispatch, cancellation removes Commit and puts Discard before later requests.
After dispatch starts, cancellation preserves pending status and the operation result.
Pause and Continue clear the engine starter gesture, runtime ignition latch, and cached starter output.
The native regressions compare Save/Continue against Save/Load/Continue after short and engaged ignition holds.
They also check that gesture clearing preserves captured engine state, item fluids, progress, and recoil.

The current IndexedDB record contains an object payload and its checksum.
Storage serializes and detaches the object before hashing or opening a transaction.
Verification hashes the stored field order and returns that exact checked snapshot.
String payloads fail validation. The schema remains version 1, with no migration.

Use `Check lifecycle boundaries` in `tests/browser/save-runtime.html` for same-frame pause operations and Commit cancellation.
This control works with the default build. It uses fixture input and queue gates, without new runtime hooks.
The storage fixture checks real object payloads, field order, corruption, string rejection, aborts, interruption, and blocked access.
Earlier artifact hashes and browser results below describe their original runs.

Before the fix, two bridge tests exposed queued Commit dispatch after cancellation and lost pending status after dispatch.
Two storage tests exposed the old string payload contract. All four passed after the fixes.
The native short-hold test reproduced a false-to-true ignition change after neutral Continue.
The engaged-starter test reproduced active cached starter output after input clear.
Both native tests passed after the narrow gesture clear.

The new lifecycle fixture failed against the old default package with `Pause the game first`.
Fixed default WASM `3180fb3674e6ae5f638ee406d7bc878fdf0f61603d5bdaf557319691bbf86b36` passed all six lifecycle checks.
Those checks cover Pause/Save, Pause/New game, Pause/Load, queued cancellation, dispatched completion, and usable Continue.
The first Continue check used four animation-frame callbacks and failed after the other five checks passed.
An immediate standalone Continue check passed.
Continue clears the step clock and skips its first frame's elapsed time.
The fixture now waits for an actual fixed step, with a ten-second limit and started/paused assertions.
The complete lifecycle rerun passed after this fixture timing correction. No product change followed that failure.

The fixed default package also passed a real object-slot save and complete page reload.
Maximum position error was 0.000003337860107421875 m; maximum quaternion component error was 0.0000001199999999368373.
Continue passed after reload. Feature-only controls stayed disabled.
Real IndexedDB checks passed object type, property order/checksum, abort preservation, corruption, string rejection, and page interruption.
The normalization check retained exact absolute coordinates.
Second-tab blocking, late successful-open closure, and incompatible-database rejection also passed: nine named storage checks in total.
The parent reset only the fixture origin to database version 1 before further runtime checks.

`scripts/check.ps1` passed formatting, 55 Node tests, 256 native tests, and default release WASM compilation.
It ignored 46 source-dependent cases. A separate 27-test save selection passed with all four installed-source save cases included.
`scripts/build.ps1` passed default compilation and binding generation.
The default WASM has 54504608 bytes.
Its generated JS remains `8952a24bfbc1aa1218016128df6a00720c61ff8cc793f468e2d2b3f7d8cc33b0`, 104757 bytes.
The source manifest remains `d62630828b68c154c9351203b11f67427916390bd9a3eac1b911fda8efb5c74e`, 5712654 bytes.
Pointer-lock play, exact browser version, physical quota exhaustion, and 42 unrelated installed-source cases remain unverified.
Controlled inputs and fixture gates do not establish unrestricted captured-mouse play or Unity physics parity.

The final-wave feature build and binding generation passed in 1m 40s.
The feature package and final fixtures have these identities:

| File | Bytes | SHA256 |
| --- | ---: | --- |
| Feature WASM | 54542158 | `9c321c663a33a13721e4a493cf116a68c868b552fd68170524c4f2c78d606430` |
| Feature generated JS | 104757 | `93050b64d3674688e758dd716f334e3b8e8068ae7849148411321c8735f5b796` |
| `tests/browser/save-runtime.js` | 23107 | `8adc482f3e1ef0ffd29a4b89f45f22e2080ae9329ee2b17f938df963897c0a07` |
| `tests/browser/save-load.js` | 9544 | `7a0e7c3228e4f17f1a2bbdc38ea2712136366f260d9367e4432e066429396c42` |
| `tests/browser/save-runtime.html` | 1590 | `43d4e1386ff7a1d2a3cdf47a2dcec350f4e0fce5401219efa59bc0ada96e1872` |

This feature package passed driving, real rebase, object-slot save, full page reload, and resumed collision checks.
Throttle moved the chassis 42.89838212814972 m. Gasoline fell from 22.5 to 22.48727962 L.
Trip distance reached 42.91876908789497 m.
Reload errors were 0.0000016987323760986328 m for positions and 0.0000001299999999870849 for quaternion components.
The reconstructed runtime had 66 bodies, 481 colliders, four wheel supports, zero bad owners, and zero duplicate bodies.
Four contacts returned after Continue. Chassis Z moved from -2086.8976287841797 to -2102.4931130111217 m with stable height.

The same final feature package passed parts, fluids, carry, cycle, full reload, resumed interaction, and normal F release.
Reload errors were 0.000003337860107421875 m and 0.0000001299999999870849 quaternion component error.
The runtime had 68 bodies, 481 colliders, three supports, one carry joint, zero bad owners, and zero duplicates.
The fixture checked five rim, two hubcap, and four tire colliders against their current owners.
Source rim `level1:24797`, hubcap `level1:26305`, and tire `level1:40000` correctly retained their shared former chassis `level1:50613`.
The first 0.25 L transfer changed totals from [2, 0.5] to [1.75, 0.75] L.
The resumed transfer changed them to [1.5, 1] L.
Cycle time wrapped from 1499.95 to 0.1709 seconds. The fixture asserted an actual fade-state change.
Normal F release cleared carry, throw charge, and throw-active state after reload.

Final feature fault checks passed for aborted writes, corruption, source mismatch, unavailable storage, and missing required metadata.
Each check retained the exact active normalized state, camera, origin, and required stored record.
The parent closed the feature page before restoring the verified default package bytes.
The restored default identity is the 3180fb36 WASM and 8952a24b generated JS recorded above.

The installed-source selection command was:
`cargo test --locked --test save_roundtrip --test save_capture --test save_schema --test save_world -- --include-ignored`.
Feature validation used `cargo build --locked --release --features save-fixture --target wasm32-unknown-unknown --lib`, then wasm-bindgen.
All commands used the existing Docker image, Cargo volume, and worktree-specific target directory.

## Browser saves: 2026-10-12

Save work uses branch `codex/save-load`, based on `be698aaec0b5a203ec94f452485d371b687c2824` for combined acceptance.
Installed Steam build 11140890 supplies the source assets.
The runtime manifest SHA256 is `d62630828b68c154c9351203b11f67427916390bd9a3eac1b911fda8efb5c74e`.
The manifest contains 5712654 bytes. The task reused the checked source export without copying or changing its assets.
The [approved inventory](superpowers/specs/2026-10-12-save-load-design.md#current-state-inventory) defines persistent and rebuilt state.
This task adds no survival or weather system.

The fixture serves `tests/browser/` and the worktree's `web/` from loopback port 4180.
Browser checks use Codex In-app Browser and its real IndexedDB manual slot.
The browser tool does not expose its version; the version query was unsupported.
Controlled setup and input use the `save-fixture` build. Default builds exclude its setup, inspection, and asset-fault commands.

The tested feature source is the stated base plus these Task 8 files.
No other runtime source changed during Task 8.
Hashes identify the exact pre-commit inputs and generated artifacts.

| File | SHA256 |
| --- | --- |
| `src/app.rs` | `2594f8e652b46036728a10fb315a2c8aa7ef3bbcad256ce14cdb721b38393358` |
| `src/app/save.rs` | `d16b3c12744874a6b56cdb2f48d7dec35acd4562352a119d1aa7a08c89e766ac` |
| `src/app/save_fixture.rs` | `cc0a89c664d963d8ac185b1287c04e12a832b15274c1f978a797ed80b6c9e3b1` |
| `tests/browser/save-runtime.js` | `6e96d3c7e9e4d798fcb93009cc6e7cddddb783e58dae181c106a38594c01f5ee` |
| `tests/browser/save-runtime.html` | `a1b2be24724a89cf5c901cda2fdc306204ad590f9bde5454eadfa6293c28fc23` |
| Feature `web/pkg/my_summer_slop_bg.wasm` (54,541,711 bytes) | `c8686c077ec7babc67f081194a465da09ecc62ddcec78dbbff0dee8db0ea5f56` |
| Feature `web/pkg/my_summer_slop.js` (104,757 bytes) | `93050b64d3674688e758dd716f334e3b8e8068ae7849148411321c8735f5b796` |
| Default `web/pkg/my_summer_slop_bg.wasm` (54,504,161 bytes) | `c5f86f89033c8d6ef584416779fdd128814137c59edc05f39a064ea65dd20228` |
| Default `web/pkg/my_summer_slop.js` (104,757 bytes) | `8952a24bfbc1aa1218016128df6a00720c61ff8cc793f468e2d2b3f7d8cc33b0` |
The driving fixture places the car on the source road near the rebase threshold, with zero chassis velocity.
It enters the source driver seat, then supplies ignition and throttle through the normal runtime input boundary.
It requires chassis displacement, gasoline consumption, wheel rotation, wheel contact, and an origin change.
A counter change alone cannot pass this check.
The interaction fixture removes a source wheel and hubcap, carries the hubcap, and transfers source gasoline through existing APIs.
It seeds cycle time near wrap and starts an existing lamp fade.
These controlled setups do not prove mouse aiming or pointer-lock play.

Each scenario uses the production save session and manual slot, then reloads the complete page.
The fixture loads the stored record and compares normalized snapshots without rounding.
Pose positions allow at most 0.001 m error. Quaternion components allow at most 0.00001 error.
All other logical values must match exactly. Failed and canceled operations require exact active-state equality.
The fixture inspects actual body ownership, collider ownership, wheel support, and carry joints after reconstruction.
It then checks resumed movement, collision, carry, fluid transfer, cycle wrap, and fade updates.

The final storage rerun passed all eight real IndexedDB checks on the current storage modules.
These include transaction abort and page interruption while a write remains pending.
It checks the stored checksum and property order, corrupt records, blocked access, and incompatible database versions.
It also checks unrounded absolute normalization and closure of a late successful open.
The runtime fixture checks the stored record and active session after corrupt, source-mismatch, unavailable-storage, and missing-asset failures.
Missing-asset checks remove actual loaded metadata or material handles before candidate preparation.
The fixture restores each removed asset after its check.
Node tests cover quota failure and delayed storage callbacks through injected storage boundaries.
They do not establish browser quota exhaustion.

Initial live records from Tasks 5 through 7 cover storage faults, candidate cancellation, and menu actions.
Those records also cover 1280x720 and 640x720 menu layouts.
The embedded browser rejected the menu's pointer-lock request. The menu retained its actions and showed the capture error.
Controlled Continue input verifies simulation progress. Continuous captured-mouse play remains unverified.
Earlier no-URL MutationObserver errors have no identified owned-source match; their cause remains unresolved.

The first feature artifact was `c1592c73f1ecb5e406039037190f16668cd5a691c5a826b8274ac4b72ed8c7c9` (54,531,255 bytes).
It passed the driving save, full page reload, and resumed physics checks.
Normal throttle moved the chassis 42.89875220176754 m and triggered the origin shift.
Total gasoline fell from 22.5 L to 22.48727456 L. Trip distance increased from zero to 42.91913915765759 m.
The reload's maximum position error was 0.0000015795230865478516 m.
Its maximum quaternion component error was 0.0000001299999999870849.
The loaded runtime had 66 bodies, 481 colliders, four wheel supports, and zero duplicate assembly bodies or invalid owners.
Paused wheel contacts remained empty, as required before the first resumed vehicle query.
After Continue, all four contacts returned. Chassis Z moved from -2086.8979988098145 m to -2102.4984621703625 m.
Chassis Y changed from 0.169841 m to 0.154864 m; the car retained road support.

The final c868 feature artifact also passed the complete driving scenario with fixture JS 6e96.
Displacement was 42.89838212814972 m. Gasoline fell from 22.5 to 22.48727962 L.
Trip distance increased from zero to 42.91876908789497 m, and real motion triggered the rebase.
Reload errors were 0.0000016987323760986328 m for positions and 0.0000001299999999870849 for quaternion components.
The loaded runtime had 66 bodies, 481 colliders, four wheel supports, zero ownership errors, and zero duplicate independent bodies.
All four contacts returned after Continue.
Chassis Z moved from -2086.8976287841797 to -2102.4931130111217 m; Y changed from 0.169843 to 0.154869 m.

The earlier c159 artifact passed abort, corrupt-record, source-mismatch, unavailable-storage, and missing-asset checks.
Each check preserved the required stored record, active snapshot, camera, and origin exactly.
The missing-asset case removed actual loaded metadata before preparation.

The initial interaction fixture falsely counted the shared chassis as three duplicate bodies after part removal.
Rim `level1:24797`, hubcap `level1:26305`, and tire `level1:40000` share chassis body `level1:50613` in the source export.
The corrected fixture retains visible ownership changes and checks each changed item's colliders against its current root.
It counts obsolete independent bodies separately from the shared chassis. This required no product physics change.

The corrected interaction artifact passed the original carry baseline before Save.
The fixture places the removed hubcap 0.35 m ahead of the view, then uses the existing pickup API.
Twelve normal steps must preserve that original joint before the fixture seeds fluids, cycle time, and fade.
The earlier setup dropped the original item before any save or load; this was a fixture placement error.

After full page reload, the maximum position error was 0.000003337860107421875 m.
The final run's maximum quaternion component error was 0.00000014000000003733248.
The runtime retained 68 bodies, 481 colliders, three wheel supports, one carry joint, and item 47 in hand.
All three former owners were the shared chassis. Five rim, two hubcap, and four tire colliders had correct current owners.
Invalid owners and duplicate independent bodies both measured zero.
Twelve resumed steps preserved carry, three wheel contacts, and garage collision.
Fluid transfer moved 0.25 L before Save and another 0.25 L after Load.
Source and destination totals changed from [2, 0.5] to [1.75, 0.75], then [1.5, 1] L.
In the final run, cycle time wrapped from 1499.95 to 0.14279999999999998 seconds.
The fade check required changed controller energy or switch motion after resumed updates.
A focused Load then normal F tap released the restored item at total step seven.
The complete final interaction scenario also passed normal F drop at total step twenty.
The input fixture waits neutral, held, and released fixed steps before pause, so the acquisition latch clears.
The final `scripts/check.ps1` run exited 0: 52 Node tests and 254 native tests passed.
Formatting and default release WASM compilation passed.
The default native run skips 46 tests that require the installed source export.
The selected installed-source save run passes all 25 tests in save_capture, save_roundtrip, save_schema, and save_world.
This includes the four save tests listed below. The other 42 skipped source cases were not rerun for Task 8.
The production_lighting_records_match_selected_scene case also requires Docker sha256sum.

<details>
<summary>Default-run source-dependent tests</summary>

```text
input::exported_brake_lights_sample_pedal_before_physics
input::exported_driver_lights_use_real_edges
input::exported_low_startup_keeps_source_outputs_through_pause
input::exported_reverse_lights_follow_selected_reverse_gear
interaction::source_push_wakes_car_without_bypassing_handbrake
lighting_interaction::exported_cabin_and_headlight_controls_work_from_driver_seat
lighting_interaction::exported_cabin_door_values_keep_all_four_source_mappings
lighting_interaction::exported_house_controls_use_native_confirmed_poses
lighting_reference::production_lighting_has_checked_source_links
lighting_reference::production_lighting_records_match_selected_scene
parts::covered_loose_wheel_mass_conserves_add_remove_and_transfer
parts::covered_loose_wheel_resolve_and_f_pick_up_rim_and_tire_surfaces
parts::detached_door_children_survive_rebase_and_recovery
parts::detached_lamps_turn_off_and_compatible_remount_restores_pose
parts::door_detach_resets_its_source_usable_and_keeps_window_state
parts::engine_and_radiator_removal_preserve_contents_and_change_operation
parts::failed_carry_acquisition_keeps_part_mounted
parts::garage_door_use_labels_keep_passive_house_hidden
parts::hubcap_cover_blocks_wheel_removal_until_removed
parts::loose_mass_tracks_nested_attachment_and_fluid_changes
parts::loose_wheel_mass_includes_nested_general_parts
parts::missing_door_does_not_power_the_cabin_lamp
parts::nested_part_moves_with_parent
parts::new_game_clears_constraints_and_restores_source_parts
parts::occupied_seat_cannot_be_removed
parts::removable_mechanism_prompt_shows_drag_and_removal
parts::source_parts_scan_then_f_installs_after_removal
parts::transferred_hubcap_blocks_only_its_current_wheel
parts::wrong_or_occupied_slot_keeps_held_part_and_contents
reference::installed_mechanisms_keep_source_controls_and_window_tracks
reference::production_export_matches_installed_contract
rendering::installed_materials_match_gltf_and_mips
save_capture::installed_source_capture
save_roundtrip::installed_source_roundtrip
save_schema::installed_source_identity_and_schema
save_world::installed_world_hibernation_preserves_source_inventory
starting_area::production_cap_surface_pickup_uses_source_owner
starting_area::production_dry_recovery_keeps_source_can_and_cargo
starting_area::production_fuel_can_lifts_and_follows_after_recenter
starting_area::production_garage_bindings_fill_start_and_move
starting_area::production_garage_exits_and_reverse_to_road
starting_area::production_keyboard_steering_uses_recovered_default
starting_area::production_mechanism_nodes_share_physics_and_render_pose
starting_area::production_parked_garage_car_stays_stable
wheels::production_wheel_bindings_detach_and_restore
wheels::production_wheel_removal_acquires_the_source_rim
```

</details>

The final `scripts/build.ps1` run exited 0 and generated the default artifacts listed above.
The final default package passed the real manual-slot save and complete page reload.
The record used schema 1 and the current source stamp through the production save session.
Feature-only fault, setup, and inspection controls were disabled.

| Default browser check | Result |
| --- | --- |
| Runtime ready | Passed; fixture commands unavailable |
| Manual-slot save and full page reload | Passed |
| Maximum position error | 0.000003337860107421875 m |
| Maximum quaternion component error | 0.0000001199999999368373 |
| Paused Load and controlled Continue | Passed |
The earlier checkpoint results below retain their own revision and scope limits.
## Rendering completion, 2026-10-12

Tested branch: `codex/rendering-completion`, before the authorized local merge.
These results do not verify the merged master package.
Source authority: installed Steam build 11140890, Unity 2021.3.18f1.

All seven layers passed their required implementation and native hardware gates.
The root inspected upright shadow, effect, glass, and controlled door images.
Full audits record issued draws, GPU completion, current tokens, resize, and stable storage.
The glass fixture preserves 98 shipping source files and the shipping glass/order shaders.
Its overlapping real doors issue two draws and two captures, then restore the original scene.

Layer evidence: `.cache/layer2-browser/` through `.cache/layer6-browser/` in the rendering worktree.
Task 7 evidence: `.cache/layer7-browser/` and `.cache/lifecycle-check/`.
The lifecycle contract passed three extracted state tests and one large-delta transition test.
The large-delta test covers loading, Start, pause, and Continue.

| Final command check | Result |
| --- | --- |
| Native suite | Final rerun: 240 passed; 42 installed cases ignored by default |
| Installed-source tests | 42 passed |
| Exporter suite | 171 passed; one installed UI test skipped |
| Skipped installed UI test | Passed separately with only input and output paths changed |
| Web suite | 24 passed |
| Context fault handlers | Two exact-handler tests passed |
| Motion slot churn and history | Three exact-source tests passed; 4,096 membership changes |
| Formatting and WASM all-target check | Passed |
| Release build and wasm-bindgen | Passed |
| Fresh selected export | 1,388 checks passed; all 1,380 shipping files matched |
| Frozen source and package hashes | Matched the corrected build receipts |

The installed-source run first lacked library receipts. Read-only input mounts resolved that environment failure.
Export regeneration used a separate output directory. No regenerated files replaced shipping files.
The final WASM contains 56,772,820 bytes.
Its SHA256 is `365a657aad3ca3586b8d40c2b14190df5566ad7191b9ebc1b8461fad12c11d60`.

Corrected stock readiness was observed by 72,746 ms, within the 120-second gate.
Start stayed disabled until the complete renderer passed.
The browser rejected pointer capture. Continuous mouse control remains unverified.
Lifecycle checks used existing game actions after genuine rendering readiness.

| Native lifecycle check | Result |
| --- | --- |
| Pause | Physics and renderer delta stopped; probe checks held |
| Resize | Main targets changed to 960x540; probe capture stayed 64x64; graph validation passed |
| Restart | Scene 2 closed readiness, then restored cold histories and a fresh validated probe |
| Restart storage | Compatible allocation counts stayed fixed |
| Missing texture | Required texture 735 produced a named failure and closed all controls |
| Invalid pipeline | Uber binding 15 failed GPU pipeline creation and closed all controls |
| Context loss | Shared state became Failed; the complete later render audit stayed identical |
| Probe distance | Real player bodies disabled and restored the probe through its scaled-time checks |
| Actual rebases | Forward and return runtime rebases matched motion and area origins |
| Streaming storage | All five stages retained 780 GPU slots; 910 slot reuses by the return stage |

The context-loss test used the actual WebGL context-loss extension after active physics.
Frame 3,677, token 3,673, all histories, and probe publication stayed fixed after failure.
Physics stayed paused at 922 steps. Start, Continue, and Restart stayed disabled.
The terminal wrapper stops extraction and rendering, while Bevy still sends time to the main app.

All five physics fixture stages passed. The fixture changes only app.rs.
It preserves 98 Rust source files and all 27 shipping shaders.
It uses real player bodies and the normal runtime rebase path. It cannot bypass rendering readiness.
Forward origin was [2304, 0, -128]. Return origin was [0, 0, -128].
Runtime and motion rebase counts both advanced from zero to two. Stream and area origins agreed.
The returned probe was enabled and published. All motion tokens matched the submitted graph token.
Live dither indices matched the source sequence in all three camera chains.
The far body fell outside existing ground. The distance check includes that real vertical displacement.
No fixture code wrote renderer clocks, histories, readiness, or origin values.

Whole-branch review found one motion streaming fault. Fixed GPU slots replaced the map capacity check.
The scoped correction review passed. Three exact-source tests and the native rerun passed.
The final correction receipts are in `.cache/final-motion-fix/`.
The native physics receipt is `.cache/layer7-browser/physics3-native-receipt.json`.
Final native rerun: `.cache/final-motion-fix/native-final.log`; 240 passed, zero failed.
The 42 installed-source cases passed separately before the final WASM-only corrections.
Source and package hashes remained unchanged through final native acceptance.
The [execution ledger](research/2026-10-12-rendering-completion-ledger.md) preserves the source rulings and evidence paths.

These results establish the tested target behavior. Matched native pixels remain unavailable.
See [the roadmap](roadmap.md) for limits and current scope.

The following sections preserve earlier checkpoint results.

## Earlier material checkpoint, 2026-10-11

Delivery branch: `codex/material-parity-checkpoint`.
Material base: `7009f62b4a757069286115617bf38d538e3db5c0`.
The only later runtime-source change removes the obsolete binary entrypoint.
The browser still enters through the wasm-bindgen library.

| Fresh check | Result |
| --- | --- |
| Exporter suite | 21 passed |
| Native suite | 96 passed |
| Installed-source gates | Seven passed |
| Formatting | Passed |
| WASM all-target check | Passed |
| Release build and wasm-bindgen | Passed |
| Matched web artifact audit | All 1,145 files matched |
| Browser startup | Garage visible; garage-ready status; Start enabled |

The fresh browser loaded the Node-served checkpoint on port 4213.
This startup reached readiness without a recovery reload.
A static screenshot is saved as `material-checkpoint-final.jpg` in the task visualization directory.
This check does not establish continuous pointer-lock gameplay or matched native pixels.

WASM size: 51,603,518 bytes.
WASM SHA256: `4ecef4e02520bb660612ed05c3729b619fa65065fdf92144f1ff8a14adb88c2d`.
Web audit SHA256: `7aefc0918a62daa32f98fc269964b342188765a1979484c7f08f3ac2b9fea2e6`.
Build logs and the audit remain in `.superpowers/sdd/2026-10-10-material-parity/` and `.cache/`.

At this earlier checkpoint, texture corrections and the full rendering graph remained deferred.
The rendering completion section above records the later approved integration.
See [the roadmap](roadmap.md) and [remaining plan](superpowers/plans/2026-10-11-rendering-completion.md).


These sections record separate asset library, interaction, and source UI checks.
Each section names its tested branch or commit. Their results do not replace merged-tree checks.

## Asset library verification

Target: Steam app `1017180`, build `11140890`, Unity `2021.3.18f1`.
Date: 2026-10-11, Europe/Warsaw. Asset branch final commit: `50455b8`.
These checks precede integration with the current interaction and source UI work.
The [asset audit](research/2026-10-10-full-asset-audit.md) contains complete counts, hashes, indexes, and conversion limits.

### Task 10 command checks

Use the reference Python environment and run commands from the repository root.
Runtime checks require Docker, `my-summer-slop-toolchain`, `tld-wasm-cargo`, and `tld-wasm-target`.

```powershell
python -m tools.reference.export --stage runtime --verify
python -m unittest discover -s tools/reference/tests -v
node --test tests/web/server.test.mjs
./scripts/check.ps1
./scripts/build.ps1
docker run --rm --pull=never --mount "type=bind,source=$PWD,target=/workspace,readonly" --mount type=volume,source=tld-wasm-cargo,target=/cargo --mount type=volume,source=tld-wasm-target,target=/target my-summer-slop-toolchain cargo test --locked --test reference -- --ignored
```

The runtime verification command calls complete `verify_library`, checks installed counts, then runs the native runtime contract validator.
Use `python -m tools.reference.export --stage library --verify` for library-only verification without Docker.
The final proof records unchanged accepted source, record, library, and runtime paths, sizes, and modification times.
It compares all selected runtime hashes with Task 9 and checks the unchanged accepted library manifest hash.
The complete verifier validates accepted file content against source, typed-record, and library receipts.

| Check | Result |
| --- | --- |
| Complete library and native runtime verification | Passed; 262,784 owners; 642.802 seconds |
| Accepted-file protection | 277,800 paths, lengths, and modification times unchanged |
| Exporter suite | 109 passed; no warning output |
| Node server suite | One passed |
| Project checks | Formatting, 94 native tests, and release WASM compilation passed; six source gates skipped by default |
| Browser package build | Passed |
| Installed reference gate | One passed |

Command exits were zero. Logs use `task-10-final-*.log` under `.cache/reference-library/work/`.
`task-10-final-checks.json` records commands and durations; `task-10-final-proof.json` records hashes, counts, and protection.
`task-10-validator-revision.json` records the exporter source hashes used by the final verifier.

### Isolated browser fixtures

Temporary source, build output, and pages live under ignored `.cache/reference-library/browser-check/`.
A separate server uses loopback port 4201 and an explicit asset allowlist.
The fixture checks library file hashes before serving the selected files.
It leaves `web/index.html`, the production entry, and accepted asset bytes unchanged.
The fixture uses Bevy 0.19.1 and the exact product Bevy feature list, including glTF, PNG, and WebGL2.
It does not enable animation playback.

The capture uses Playwright 1.62.1 and isolated headless Chrome 155.0.8059.40 with `chromiumSandbox: true`.
It uses no user profile or security-bypass flags. Each capture has a 55-second external deadline.
Captures retain all console levels, request outcomes, completed response-body lengths, and screenshots.

Bevy loaded the static `pohar` hierarchy (`sharedassets1.assets:21505`): three nodes, one mesh, and one scene.
It loaded skinned `RandomBat` (`sharedassets1.assets:32394`): 25 nodes, one mesh, one skin, and one scene.
Both passed `AssetServer::is_loaded_with_dependencies` and exposed their loaded glTF assets.
The loader capture completed all 21 HTTP 200 response bodies with no fatal loader or page error.
Bevy logged expected WebGL compute and storage capability limits.
A separate render capture displayed the static cup with the production camera's Reinhard tonemapping and Msaa::Off.
That capture completed all 21 HTTP 200 bodies without console errors or page errors.
I inspected the screenshot. This establishes representative static rendering, not Unity material parity.
Skinned loading passed; skinned rendering and animation playback remain unverified.

The media capture completed 13 response bodies with HTTP 200 and no console or page errors.
Chrome loaded embedded Liberation Sans and used it for the font sample.
The WAV `sharedassets1.assets:4164` decoded to 1,868 mono frames at 8 kHz and reached its muted playback end.
Its duration was 0.2335 seconds. Audible output and game audio integration remain unverified.
Six labeled 128 by 128 cubemap PNG faces decoded and appeared in the screenshot.
Those previews clip to 8-bit color. Native BC6H DDS/HDR rendering remains unverified.

The rotated sprite uses six known pixels through UnityPy's sprite rotation and flip operations.
Chrome's canvas pixels match the expected 2 by 3 image.
This fixture is synthetic because the installed index has zero sprites with nonzero packing rotation.
The installed source also contains zero morph channels.
Synthetic morph tests cover deltas and glTF channels; complete source checks validate all 42 clips and their normalized samples.
These checks do not establish animation playback or humanoid retargeting.

Evidence lives under `.cache/reference-library/work/`:
`task-10-media.json/png`, `task-10-loader.json/png`, and `task-10-render.json/png`.
The fixture source and allowlist remain in `browser-check/` for local reproduction.

### Garage startup limit

Task 9 captured 92 startup requests with HTTP 200, including the exact 86 selected asset URLs.
It found no library, catalog, unused-model, or audio preload.
The bounded readiness check remained at the loading screen; Start stayed disabled.
A corrected prior-payload comparison also remained loading, with all 94 response bodies completed.
It retained the current validated manifest, so obsolete schema differences did not affect that comparison.

Chrome used WebGL2 through ANGLE/D3D11 on NVIDIA GeForce RTX 4070 Ti.
A 40.256-second CPU profile attributed 38.832 seconds, about 96.5%, to synchronous WebGL program-link status checks.
The caller chain runs through wgpu's GLES pipeline creation and Bevy's pipeline cache.
No fatal loader or page error occurred. The evidence does not justify an exporter correction.
This task retains those measured results and does not repeat the long stalled capture.
Garage readiness remains unverified on this path.
The user's continuous garage-to-road browser route waiver remains in effect.

Evidence: `task-9-startup-network.json`, `task-9-prior-payloads.json`, `task-9-startup.cpuprofile`, and `task-9-profile-summary.json`.
These files live under `.cache/reference-library/work/`.

### Verification fixes

The typed-schema cache fix from Task 8 remains covered by the complete suite.
The type-tree test now restores its prior boost state through test cleanup.
A long WAV fixture covers binary32 duration error above one frame and rejects error outside that bound.
The glTF helper filters only pygltflib's exact provisional-validator notice. Unrelated warnings and validation exceptions remain visible.
The normal-map helper uses Pillow's supported `get_flattened_data` API.
The runtime audio exclusion test now passes the scene gate before adding a WAV dependency.

Final safety fixes passed 117 Python tests, the Node server test, 94 native tests, and the installed reference gate.
Formatting and release WASM compilation passed. The final review found no open findings.
The proof checked 277,800 protected paths and unchanged accepted manifest hashes.
Evidence: `.cache/reference-library/work/final-fix-proof.json` and `final-fix-bound-before.json`.

## Garage and interaction verification

Reference: installed Steam build 11140890. Vehicle: fixed Lada 2101.
Verification date: 2026-10-11, Europe/Warsaw. Final product commit: `d402a09`.
Final `d402a09` code passed fresh independent checks: 137 native tests and 31 installed-source tests.
Fresh independent formatting, release WASM, and 12 exporter tests also passed.
The earlier full checks and focused label regression remain recorded below.
Worktree: `C:/Users/Corashina/.codex/worktrees/interaction-parity/the-long-drive-wasm`.
Read the [approved interaction scope](superpowers/specs/2026-10-10-interaction-parity-design.md), [garage scope](superpowers/specs/2026-10-09-garage-start-design.md), and [decision record](interaction-decisions.md).

## Source and build checks

The exporter follows selected scene links to the home and Car02Full hierarchy.
It converts handedness once and preserves nested scale and reflected wheel transforms.
The manifest retains source IDs and input hashes. Generated assets stay outside version control.
Startup contains 0.5 L gasoline, 2.5 L oil, and 3.6 L water.
The car has 955 kg dry mass and 6.6 kg initial fluid mass.

| Check | Result |
| --- | --- |
| Native suite | 137 passed |
| Installed-source gates | 31 passed |
| Exporter suite | 12 passed |
| HUD callback test | 1 passed |
| Formatting | Passed |
| Release WASM and browser package | Passed |
| Task, full-branch, and final code reviews | Passed |

The final label-only change passed its installed label regression, formatting, and WASM/package build.
The independent final exporter rerun passed all 12 tests in 22.016 seconds. Earlier logs remain preserved.
Native checks cover fluids, ignition, heat, collisions, carry, mechanisms, lamps, parts, push, throw, pause, recovery, and rebasing.
Source tests use the actual export. Some tests stage clear player positions.
They do not prove a continuous browser route.
The source route covers garage exit, nested parts, transferred hubcaps, engine and radiator operation, and lifecycle cancellation.
Carry regressions cover floor separation, recovered collision groups, installed-can pickup, recenter, and walking.

Final logs are in `.superpowers/sdd/2026-10-10-interaction-parity/`:
`final-fix-full.log`, `final-fix-python.log`, `final-fix-hud.log`, and `final-fix-report.md`.
Earlier RED/GREEN logs and screenshots remain in the ignored `.cache` directory.

## Package and browser conditions

The root-owned Windows server serves this worktree at `http://127.0.0.1:56191`.
The earlier 53a9a4e Windows preparation package matched its checkout at that time.
Its WASM was 50,916,156 bytes, SHA256 `b0f7325325a8ec276bdc07f91fc44de952b008d14d0b113a7a00ec5ef9035fd4`.
Final d402a09 WASM is 50,916,241 bytes, SHA256 `99bd67a677d5afbfd20771a5f04a14d960e563783c6d7e0e686541add455c4d4`.
The manifest is 3,849,428 bytes, SHA256 `ed43d7d49fcdb825c364b2a2ae4ac0b5043924dece3449bb1347ab863fa8c855`.
Served final Windows and Linux package bytes matched the final checkout.

Windows routes used Chrome 154.0.8037.95 with ANGLE SwiftShader.
Flags: `--use-angle=swiftshader --enable-unsafe-swiftshader`.
Playwright clicked Start and used normal pointer lock, keyboard, and mouse input.
The helper read public `gameState` and `gameKeys`. It did not inject game state.
Windows captures used 1280 by 720, with functional checks at 640 by 360 and 960 by 540.

The 53a9a4e browser closed before a closer grip retry. No close command was sent; the cause is unknown.
The helper read an already-closed page and exited. Evidence remains in `.cache/task8-53a9a4e-interrupted/`.
The earlier abd2f5c helper had read a partial command file. Atomic command writes now prevent that harness failure.
The d402a09 Windows session later lost its graphics context after a GPU process exit.
Windows headless pointer lock also confined the desktop cursor. Testing stopped, and only the owned browser/helper were closed.
Evidence remains in `.cache/task8-d402a09-context-lost/`. No Windows browser was launched afterward.

The active Windows harness entries `.cache/task8-session.cjs` and `.cache/task8-route-input.cjs` are retired.
Both fail before loading Playwright. Their scan and sequence callers inherit that failure.
Historical harness copies remain unchanged as evidence and are for inspection only.
The supported runner is `.cache/task8-run-linux.ps1`.
Its Linux helper rejects Windows, non-container Linux, and DISPLAY or WAYLAND_DISPLAY environments.
A normal `shutdown` command releases pointer lock and closes only the browser owned by that session.
Startup failures and session cleanup use the same owned-browser close path.
Four syntax checks and nine stub guard/cleanup checks passed. No real browser was launched for these checks.
Guard evidence: `.cache/task8-harness-guard-check.cjs` and `.cache/task8-harness-guard-check.log`.

Preparation-to-road continuity is unavailable. These split routes do not prove full continuous acceptance.

The remaining route used Linux Chromium 151.0.7922.34, Playwright 1.62.1, and ANGLE SwiftShader.
Image: `mcr.microsoft.com/playwright:v1.62.1-noble`.
Digest: `sha256:dcc5531e97840b9b5e794f2814476b21571c5124a3fca2267d73041f56e7580e`.
Docker used `--init`, `--network none`, and `--shm-size=2g`, with no host display mount.
The checkout and existing dependencies were read-only. A separate evidence directory was writable.
The server and browser ran inside the container. Root checked full desktop cursor bounds three times during Linux pointer lock.
Evidence: `.cache/cursor-linux-isolation.jsonl`, `.cache/task8-linux-isolation.txt`, and `.cache/task8-run-linux.ps1`.
Linux headless shell emitted opposing trusted mouse warps and failed calibration. It was closed.
Full Chromium new-headless passed the normal mouse-delta check in the same container.
A read-only native event observer recorded movement. It injected no events or game state.

## Action evidence

| Action | Browser evidence | Limit |
| --- | --- | --- |
| Can-cap pickup, carry, recenter, fill, drop | 53a9a4e Windows route | Direct crouched path passed after a blocked bench approach |
| Mounted hubcap, rim, engine removal and installation | 53a9a4e Windows route | Engine operation passed after installation |
| Charged throw and meter | 53a9a4e Windows route | Charge reached 0.45; release cleared state |
| Garage doors, window, mirrors, glovebox, visors | Paired Windows captures; Linux garage route | Window scroll was not proven |
| Cabin lamp | Windows bright/dark pair | No rendered Unity comparison |
| Vehicle headlights | Linux 1280 off/on pair with engine running | Two beam pools appeared on the brick wall after E and 60 steps |
| Side mirror label | d402a09 Windows route | Label-only package change |
| Ordinary push | d402a09 Windows route | F release stopped applied push |
| Latched grip | d402a09 Linux route | Release and look-away retained grip; next F released |
| Held-item pause | d402a09 Linux route | Fuel can remained held; charge and keys cleared |
| Restart with removed part | Windows removed door; Linux removed hubcap | Separate from held-item Restart |
| Restart with held item and shifted origin | Linux returned Fuel can | Cleared hands, origin, rebases, ignition, and keys; restored fuel 0.5 |
| Seat entry, start, garage exit | Windows seat actions; Linux driving exit | Driver door required visible closure |
| Recovery | d402a09 Linux route | Normal R preserved fuel and engine state |
| Origin shift | d402a09 Linux normal drive | One rebase at absolute x2102.222 during off-road driving |
| Return home | Physical Linux run after rebasing | Garage leaves remained open; loose-item persistence has native proof only |
| Covered loose-wheel pickup | Native installed-source regression | No browser proof |

Filling increased gasoline from 0.5 to 1.128319 L. Collision rules were unchanged after the first blocked approach.
The final held-can pause recorded charge 0.55, active throw, and keys 256 before Escape.
Pause cleared charge and keys. Resume retained the can without a delayed throw after 30 simulation steps.
A removed-part Restart restored fuel 0.5, origin 0, handbrake, and empty hands.
The first shifted-car reverse timed out. In the fresh route, the open driver door caught the garage frame.
A negative seated drag left it open. The opposite positive drag visibly closed it, then reverse cleared the garage.
The stage label `closed-door reverse` identifies a failed closure attempt, not proof of closure.
The passing image is `task8-linux-driver-opposite-close.png`; `task8-linux-verified-exit.png` shows the car fully outside.

Normal R aligned the car on the road at its current distance. R did not restore garage pose.
Initial travel followed negative Z. The car later changed direction near z-974 with only W held.
The cause was not isolated. The subsequent drive proves off-road travel and rebasing, not sustained road following.
At absolute [2102.222, 2.208, -965.356], the origin became [2048, 0, -1024], with finite vehicle state.
After another R, the engine was stopped and the car parked. Normal input returned the player home on foot.
At [32.683, 2.465, -69.490], the garage leaves remained open and the car remained parked remotely.
The passing capture is `task8-linux1280-returned-garage-leaves.png`.
The earlier `task8-linux1280-home-persistence.png` was a missed approach and proves no leaf state.
No Restart occurred during that return. No loose home item was moved after the preceding Restart.
The home capture therefore proves mechanism persistence. Loose-item persistence remains native/source proof.

The final headlight pair is `task8-linux1280-engine-headlight-off.png` and `task8-linux1280-engine-headlight-on.png`.
Both use the same parked view with the engine at 800 rpm. The controller also confirmed the visible beam change.
After capture, Escape paused the game with keys 0 and pointer lock released.
Linux Chromium closed normally with exit code 0. Its container exited with code 0 and was removed.
The root-owned Windows server remained available.

## Measurements and limits

Linux functional actions used 640 by 360, 480 by 270, and 320 by 180 for long travel.
The same browser was resized for 960 by 540 and 1280 by 720 captures.
At 1280 by 720, 30 RAF intervals had median 1833.3 ms and p95 2083.3 ms.
The range was 1699.9-2099.9 ms. This software-rendering result does not establish hardware or Windows performance.
Linux loading enabled Start in 5703 ms and acquired pointer lock in 11713 ms.
Its captured page and HTTP error arrays were empty. The Windows GPU failure is recorded separately.
Observed Windows request URLs map to 121,014,961 bytes of local files. This is not network transfer size.
The inventory includes 60,230,351 bytes of PNG textures and 50,916,241 bytes of WASM.
Route logs, screenshots, input history, and public-state snapshots remain in `.cache/task8-linux/` and `.cache/task8-linux-route.log`.

All screenshots show the port. No rendered Unity comparison was captured.
Recovered source records and native checks support source behavior; screenshots do not establish Unity visual parity.
Hardware ANGLE Direct3D11 stalled before Start for 90 seconds in the Windows setup.
The evidence does not identify the exact graphics operation. Software rendering does not establish hardware performance.

## Source rules and limits

Use, grab, fill, and push resolve recovered owners and masks.
General parts scan near the held part with source mask 513 and a 0.2 m base radius.
Source multipliers and nearest compatible slot selection follow recovered `partscript.Scan`.
Wheels retain their separate aim query. No general-part layer mask was widened.
The HUD names the resolved action owner. Remove the installed hubcap before its wheel.
Carry supports distance, recenter, translation, rotation, roll, and charged throw.
Source pickup rules can disable rotation; the garage fuel can has that rule.
Ordinary push stops on F release. Grip stays latched until next F, excess distance, owner loss, or lifecycle cancellation.
Changing aim does not release grip. Owner loss means an invalid physical owner or collider.
Pause, capture loss, recovery, and restart clear sampled input before another simulation step.

Rapier raycast suspension replaces Unity WheelCollider physics. Identical handling is not established.
Recovered dimensions, springs, damping, brakes, condition, and drivetrain ratios control the adapter.
The fixed garage spawn follows the user's request. Seeded wheel condition makes startup repeatable.
The exporter selects one high detail LOD and maps Unity materials to glTF.
Unity shader effects, live mirror reflections, runtime plate text, and audio remain outside this stage.
Selected Lada seats lack source pickup links. Plate slots start empty; no removable seats or plates were invented.
Earlier allocation checks measured Rapier movement allocations. The player wrapper added none above that path.
These results do not claim zero allocation for the game.

Read the [garage recovery record](research/2026-10-09-garage-reference.md) and [interaction comparison](research/2026-10-10-interaction-reference.md).
Installed-build evidence controls conflicting secondary descriptions.
The [decision record](interaction-decisions.md) preserves all twelve ledger rulings and their rework costs.

## Source UI verification

These checks cover the source UI branch before the interaction merge.
Its controls and headlight limits describe that branch.

Scope: approved current features in docs/superpowers/specs/2026-10-10-source-ui-design.md.
Source: installed Steam build 11140890.
Worktree: source-ui. Base: 743ef8b.
Date: 2026-10-10, Europe/Warsaw.

The UI now uses recovered menu sprites, canvas layouts, action icons, and fluid presentation flags.
The old serif shell, persistent vehicle card, X assembly action, and P/R shortcuts are removed.
F press picks up, drops, or dismounts. F release mounts.
RMB moves a held item. MMB rotates it.
Esc opens the menu. Backspace toggles feedback.
Recover remains in the pause menu. Fullscreen remains in Controls.

### Final checks

| Check | Result |
| --- | --- |
| Rust formatting and native suite | 109 passed; six installed-source gates skipped by default |
| Explicit installed-source gates | Six passed |
| Browser module suite | 14 passed |
| Python exporter suite | 11 passed |
| Complete installed-build export | Passed |
| Release WASM and browser package | Passed |
| Independent source review | One Important finding fixed; no Critical findings |
| Final diff and source-asset exclusions | Checked before the integration commit |

Commands use the source-ui worktree and its separate Cargo target volume.
The ignored .cache/check.ps1 and .cache/build.ps1 pin that volume.
The tracked scripts contain the same checks with the normal project cache.

- Python: unittest discover -s tools/reference/tests -v.
- Browser modules: node --test tests/web/menu.test.mjs tests/web/input.test.mjs tests/web/feedback.test.mjs.
- Native and release: .cache/check.ps1.
- Installed gates: cargo test --locked -- --ignored in the toolchain container.
- Export: tools.reference.export with the installed Steam root.
- Package: .cache/build.ps1.

Final logs are retained under ignored .cache/source-ui-verification.
They include final-check.log, final-source.log, final-python.log, final-node.log, final-export.log, and final-package.log.

### Review corrections

The independent reviewer found emission on the source warning lamps' Off materials.
Unity's _EMISSION keyword now controls both emission factor and texture export.
A focused regression test failed before the fix and passed afterward.
Regenerated Off materials have zero emission for fuel, oil, battery, and headlight2.
Their On materials retain the recovered emission values.

Browser startup exposed a late glTF material load that reloaded the vehicle scene.
The renderer now retains raw glTF handles during initial loading.
A startup adapter creates and caches the used lamp materials.
The final browser reaches the ready menu with the vehicle present.

Static review found clipped carrying text.
Its bottom alignment now follows the recovered lower-right text rectangle.
The confirmation capture shows all three lines above the bottom margin.

### Browser evidence

The final browser starts at http://127.0.0.1:4174/.
Desktop review used 1280 by 720 pixels. Narrow review used 390 by 720.
Start and Controls were inspected over the actual loaded garage.
Pause, failure, and fluid feedback were inspected with an isolated UI fixture.
That fixture contains no WASM runtime, simulated input, or product capture bypass.
It was removed after review.

Captures are under ignored .impeccable/review:

- source-ui-start-final.png: actual ready menu and vehicle.
- source-ui-controls-desktop.png and source-ui-controls-narrow.png: actual Controls.
- source-ui-pause-fixture.png: source pause geometry and session distance.
- source-ui-error-fixture.png: disabled actions and a readable failure.
- source-ui-fluid-fixture.png: source fluid text, fill icon, and complete carrying help.

The Impeccable detector ran once over web.
It reported three Arial warnings and one unrecorded error color.
Arial is the recovered gameplay font. The error color is now recorded in the palette.
Two files had invalid ellipsis bytes. They were repaired and validated as UTF-8.
The detector could not inspect those files before repair. It was not rerun.

The embedded browser rejects pointer lock.
Live pause and contextual gameplay remain covered by state and native tests, plus static layout review.
The approved final continuous browser route waiver remains.
No live pixel-parity claim is made.

### Instruments and limits

| Instrument | Source range | Unity Y angle |
| --- | --- | --- |
| Speed | 0 to 160 km/h | -64 to 64 degrees |
| Temperature | 10 to 130 degrees Celsius | -30 to 30 degrees |
| Fuel | 0 to 39 L | -26 to 26 degrees |

Needles use source signal smoothing at 5 times dt and value smoothing at 8 times dt.
The five odometer digits use base 10, one fractional digit, and source carry rules.
They use source X rotations and angle smoothing at 20 times dt.
The display starts from the recovered value and adds session travel.
Recovery and coordinate rebasing preserve it. Restart resets it.
The source fuel, oil, and battery guards use local ignition and recovered thresholds.
The unsupported headlight indicator remains off. The Lada has no RPM needle.

Entity and material binding occurs during loading.
Instrument updates use fixed arrays and cached handles.
Feedback retains display values and changes DOM text only when needed.
Browser text uses Liberation Sans and Arial. It differs from Unity's SDF renderer.
Liberation Sans 2.1.5 provenance and license remain in web/fonts.
Generated original assets remain excluded from Git.

### Execution rulings

- PowerShell briefs replace unavailable Bash bookkeeping helpers.
- A separate Cargo target cache prevents stale metadata from concurrent work.
- Menu synchronization uses three scalars to avoid a new frame object.
- Recovered Gas, empty, and (full) labels override illustrative plan wording.
- Cached startup material conversion replaces late subasset loading. Bevy's converter is private.
- Design documentation ran inline under the approved native workflow.

The design record uses the actual source palette, menu and gameplay fonts, canvas rules, and source text shadows.
Its sidecar records color metadata, motion, breakpoints, and standalone text components.
Original sprite previews require the ignored local export and remain outside the sidecar.
No independent-review Minor findings remain deferred.

## Local interaction merge, 2026-10-11

Master `5d4cc16` integrates interaction worktree commit `c2d0fe6`.
The merge keeps source menus, feedback, instruments, mechanisms, parts, carry, push, grip, and throw.
Source `fpscontroller.MountFunc` lines 5544�5557 install on F release.
Its callers at lines 3396 and 4325 require charge below the throw minimum.
Charged release over a valid slot throws. Blocked installation keeps the held part and clears charge.
Target names and source fluid labels use separate fields.
Wheel compatibility and occupancy share one rule for selection, feedback, and execution.

Headlight state now drives the high-beam indicator and odometer illumination.
Source evidence: `carscript.cs` lines 1197�1208, 2437�2445, and 2559�2584, plus `forgoszamlap.TurnOn/TurnOff`.
Digit material handles remain cached.

Fresh checks passed: 157 native tests, 31 installed-source tests, 20 browser unit tests, and 19 exporter tests.
Formatting, the release WASM build, and browser package generation passed.
The combined export contains 417 colliders and 127 items.

An isolated Linux Chromium 151.0.7922.34 check passed at 960 � 540.
Start enabled after 6 seconds. A trusted click captured the mouse. Physics advanced at least 12 steps.
Escape paused play and released capture. The browser and internal server then closed.
No page errors or gameplay asset HTTP errors occurred. The console recorded one 404 for the excluded favicon request.
Docker had no network or host display. Only the evidence directory was writable.
Evidence is in ignored `.cache/interaction-merge/`: logs, `smoke.json`, and three screenshots.
This check covers startup and pause. Earlier route and performance limits still apply.

Verified browser package SHA-256:

- WASM: `cd16e48a1495a9dff8fbf9265af03a0c2acdc9df96f0cf8cd7dfca2b55f203b2`.
- JavaScript: `1c6ac36bffef8185ec161b4535b222a22f04a587baf3a79d77f0e092a617a084`.
- Manifest: `826fc55277c235a062acacfd0a47cebe3dff3a297577cb0acd32c644ff6ac0c0`.

The managed interaction worktree and its local evidence remain available.

## Keyboard steering verification

Date: 2026-10-11, Europe/Warsaw. Scope: source keyboard steering for the selected Lada.
The [REA audit](research/2026-10-11-rea-driving-audit.md) records the recovered default and separate remaining driving gaps.

The critical steering regression failed on instant full lock before implementation.
It passed after adding the fixed-step steering state.
Checks cover rise, release, reversal, seat changes, capture clearing, pause, recovery, and restart.
The clock test gives equal steering at 30, 60, and 144 render frames per second.
These are native clock checks, not browser frame-rate measurements.

Fresh checks passed: 162 native tests, 32 installed-source checks, 20 browser unit tests, and 19 exporter tests.
The 32 installed-source checks include the existing 31 gates and one added steering gate.
That gate enters the source driver seat, verifies the first steering step, reaches full lock, and returns to center.
Formatting, the release WASM build, and browser package generation passed.
The regenerated export contains `vehicle.keyboard_steering_rate = 1.25` and records the slider source.

Isolated Linux Chromium 151.0.7922.34 passed startup, pointer lock, simulation progress, and pause at 960 by 540.
Start enabled in 6892 ms. No page errors or gameplay asset HTTP errors occurred.
The console recorded the excluded favicon 404.
The container had no network or host display. Cleanup closed its owned browser and internal server.
This smoke check does not establish a continuous browser driving route or identical Unity handling.

Evidence is in ignored `.cache/rea-audit/`: `steering-red.log`, `steering-green.log`, `steering-checks.log`,
`steering-installed.log`, `steering-python.log`, `steering-web.log`, `steering-export.log`, and `browser/smoke.json`.

Verified package SHA-256:

- WASM: `0aa817ef29683d1c0c9ce3e54f34a841629825445dc9f8c454b2ded5034440ea`.
- Manifest: `0bd31e0f609768345f8a336a7bea2b305424264eec8684d95c66096cc5f8f33a`.

## Day cycle and lamps: 2026-10-11

The user approved [the lighting specification](superpowers/specs/2026-10-10-day-night-lighting-design.md) on 2026-10-10.
Installed Steam build 11140890 remains the source authority.
The selected cycle component is `level1:64920`.
Assembly-CSharp SHA-256 is `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
The manifest retains fifteen source hashes, ten controllers, seventeen light records, and eleven lighting textures.
The checked source metadata run matches all fifteen installed input hashes.

Final command checks use the isolated lighting checkout and Cargo target `/target/0aeee55818e76061`.
Python passes 26 tests. Native Rust passes 147 tests; all 15 installed-export cases pass in the explicit ignored run.
Formatting, release WASM, and matching wasm-bindgen pass after observation removal.
The final browser package timestamp is 2026-10-11T13:08:25UTC. Its WASM size is 53065640 bytes.
The final production source contains no temporary Task 7 observation system.
The final production browser reaches Ready, enables Start, shows L, and reports no current GPU error.
Its final seated night-low case renders wall beams and horizontal occlusion at 55 steps, with ignition on and engine off.
Root captured `lighting-task7-final-night-low.jpg` and accepted the recorded limits.


Source durations are day 1200, night 300, dawn 240, and dusk 180 seconds.
Dawn and dusk lie within the day interval. Production startup begins at 240 seconds.
The native lifecycle test freezes cycle time, fades, and switch movement during pause.
It preserves those states through recovery and restores startup state through restart.
The installed acceptance test checks source IDs, ranges, cones, shadow flags, material links, and exported file hashes.
Native source tests cover both physical headlight aliases, cabin modes, five house switches, brake input, and reverse input.

One `SourceLighting` owner retains imported scene nodes, textures, meshes, prepared material choices, and GPU buffers.
Restart removes fixed light proxies and owned pole hierarchies before rebinding.
Road pole identities use absolute source segment indices. Each active interval owns each pole once.
Eleven 128-metre intervals permit at most fifteen poles with source 100-metre spacing.
Road geometry and road lamps share the existing lateral admission rule: bands minus 1 through 1.
Movement and rebasing update existing poses. Stream changes replace instances with shared handles.
The steady lighting and sky paths update persistent uniform buffers and prepared material selections.
The source audit found no resource creation in those covered steady paths. It does not measure complete-game heap allocation.

### Browser evidence

Root records the browser checks in ignored Task 3 through Task 7 reports and captures.
The baseline Task 7 restart retains 38 of 38 old light and pole entities. This proves the renderer reset defect.
The production-offset scenario preserves startup 240 and advances the existing elapsed cycle after Ready.
The GREEN reset removes all 38 original entity IDs and restores cycle 240 with elapsed 0.
Eighteen rebound snapshots retain 16 unique fixed IDs and 11 unique poles, with no original IDs.
Imported node links and eleven source texture handles retain their exact identities.
The existing restart input owns this transition.
This elapsed advance tests renderer lifecycle. Native cycle tests provide timing rule evidence.
Synthetic phase captures change the served startup offset. They provide image evidence, separate from production reset proof.

Root captured walking and cockpit images at dawn, noon, dusk, and night from the same browser package.
The walking poses match. The night cockpit differs by 2.85 mm after one extra physics step.
Local driver pose, camera settings, source materials, and low-beam selection match across cockpit views.
The four images show the parked car, closed garage, and source sky changes.
Cockpit wall patches show low beams and interruption by horizontal geometry.
The dark night image does not prove distinct parked-car or closed-door moon shadow edges.
The noon procedural apron still clips.

Earlier accepted views show wall beams, tail emission, white reverse lamps, house 1 through 4 output, cabin modes, and roadside patches.
House 5 accepts aimed input, but its upward view lacks clear lamp pixels. Installed tests prove its output state.
The two physical headlight records share one usable source action; the browser uses its canonical alias.
Source ranges 50 and 250 metres remain numerical source checks. Close wall images do not measure those distances.
The matched night road run crosses seams and rebases once. It retains 14 poles, 30 light registrations, and 68 shadow views.
The run beyond 100 million metres retains those counts and shows distant warm patches.
The synthetic dawn threshold view shows poles without warm lamp patches.

The actual upload payload has one active radiance slot in daytime and twelve at night with eleven road lamps.
The source enabled-proxy count matches those values. The parked fixture has 27 registrations.
Reset clears registrations and radiance; rebound restores one active daytime slot out of 27 registrations.
The loading marker count stays zero after Ready and after restart.
These counts read source payload RGB and SourceLight.enabled. They exclude Bevy's zero-intensity native light fields.
The temporary observer performs no driver GPU-buffer readback.

Resource observations count asset storage and current instance registrations.
Image mip payload bytes count retained CPU image data, including decoded mip chains.
They exclude GPU alignment, render targets, framebuffer storage, and driver overhead.
The custom 4000-square Depth32Float atlas holds 64000000 bytes; hard and soft bindings share that allocation.
The source uniform holds 14800 bytes. GPU limits report 16384-pixel textures and 65536-byte uniform bindings.
The garage observations retain 126 source materials, 126 standard materials, 97 images, 244 meshes, 11 targets, and 27 prepared choices.
Their image mip payload totals 309788668 bytes through recovery and restart.

| Settled phase at 1280 by 720 | Mean ms | Median ms | p95 ms |
| --- | --- | --- | --- |
| Night before recovery |31.5683|27.9|41.7|
| Night after recovery |19.6475|20.8|27.8|
| Day after restart |18.1421|20.8|27.7|

Frame measurements use 240 consecutive requestAnimationFrame intervals in each settled phase.
They measure browser frame pacing, including scheduling and application work. They do not isolate GPU time.
The night movement run completes 190 metres with one rebase and 14 poles, 30 light registrations, and 68 shadow views.
Its 240-frame rebase window spans originZ 0 to -2048, at steps 719 through 895.
That window has mean 12.2396 ms, median 13.9 ms, p95 14 ms, and maximum 21 ms.
Twelve movement batch means range 10.6196 through 16.8983 ms.
Batch 9 retains a single 1118.1 ms interval of unproved cause. These measurements do not isolate GPU work.


### Startup gates and expected warnings

Required cookie failures and shader failures keep Start disabled and name the failed resource.
Healthy production loads reach Ready.
The final cookie gate names its missing SMIP. The final shader gate reports a Composer error.
Both keep Start disabled. Root captured both failures, then accepted the healthy phase views.
I stopped both dedicated fault servers after their checks. Production files retain their healthy resources.
Temporary fault routes and observations stay outside production files.
The exporter filters only the exact pinned pygltflib provisional-validator notice.
It still runs the same schema validation and shows other warnings and validation errors.

Bevy warns that its native WebGL shadow path supports one directional cascade.
The custom source atlas consumes the four checked cascade transforms. Keep those four source bounds.
The imported geometry also produces an unused TEXCOORD_2 warning.
Covered source lighting and cloud shaders use UV0; the exporter retains extra source UV channels.
These two named warnings remain expected. GPU validation errors, panics, and other warnings require inspection.

### Source rendering limits

No matched running original-game screenshot set exists. Source pixel parity remains unverified.
The source uses a camera stack; this renderer uses one combined camera with source near and far bounds.
Its 398-square shadow cells differ from source resolution. Native bias units and cascade edge blending remain unverified.
The sky sphere uses 64 by 32 segments. Native sky geometry and star RNG remain unverified.
Flare UV inset, exact physics queries, flare fog, and final color quantization remain unverified.
The renderer uses zero extra flare inset and no extra flare fog term.
Source mip bytes retain their original rows; native GPU cookie upload orientation remains unverified.
The synthetic noon view clips the bright procedural apron. Full procedural surface parity remains outside this task.

Dashboard node 861 selects source on material 325, with checked emission, a loaded texture, and visible entity flags.
Nine checked seated-eye rays cross opaque Doboz before the back-facing icon. Its source texture crop contains the correct symbol.
Part 74816 on root 34046 retains localRotationAfterMount [0, 180, 0] and a null saved slot.
The checked Craft rule applies that angle, but the selected slot and executed mount path remain unresolved.
No guessed panel rotation, opacity, culling, winding, or color change repairs that gap.
The dashboard icon has no proved visible pixels in the current cockpit view.

Close beam images do not establish far cutoff or all terrain, wall, and closed-door occlusion cases.
Four-phase parked-car and closed-door shadow acceptance requires observable edges in each captured phase.
Keep any dark or hidden edges as an acceptance limit.
The embedded browser rejects ordinary pointer lock. Deterministic ignored input scenarios do not prove real mouse capture.
Engine heat, fluids, portable flashlights, survival, sleep, weather events, audio, saves, multiplayer, and deployment retain the approved scope.

## Local lighting merge: 2026-10-11

Base: `af7d3af`. Lighting branch: `8af831f`. Target: local `master`.
The merge retains the asset library, source menus, instruments, part mounting, and keyboard steering.
Source lighting owns the cycle and lamp state. Removed headlights and doors retain their mounting guards.
L uses bit 65536. Source light feedback uses context 10.
Recovery runs from the pause menu. Lifecycle actions discard the sampled input frame.
The exporter retains the source/catalog/library/runtime stages and exact archive paths.

| Check | Result |
| --- | --- |
| Native suite | 225 passed; 41 installed-source cases skipped by default |
| Installed-source cases | All 41 passed |
| Exporter suite | 155 passed |
| Browser unit and server tests | 23 passed |
| Formatting and release WASM | Passed |
| Matched wasm-bindgen package | Passed; version 0.2.129 |
| Complete accepted library | 262,784 owners verified; accepted receipt unchanged |
| Regenerated selected runtime | Five scenes; native contract, dependency hashes, lighting files, and exact file set passed |
| Production browser | Ready; Start enabled; headlights control row present; no graphics errors or sampler conflict |
| Integration review | No unresolved Critical or Important findings |

The complete library receipt is `92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99`.
Runtime promotion followed verification against that unchanged receipt.
The first export stopped on an archive path mismatch. The strict consumer now uses the complete archive path.
The final native fixture uses the complete source receipt list and unique synthetic node bindings.
Installed dashboard checks resolve material source identity through the generated glTF index.

The server test now requests a free port. The existing material research server on port 4199 remains running.
The production preview uses loopback port 4181.
The in-app browser still rejects mouse capture. This check establishes startup and rendering readiness.
The earlier source pixel parity, dashboard visibility, and continuous browser route limits remain open.
The managed lighting worktree remains available with ignored source exports and research files.
Remote master has separate commits. The local merge does not integrate those commits or push changes.

Final package hashes:

- `web/pkg/my_summer_slop.js`: `aada77f02f281bcae1381481cd525e5e03fe785534a4ad00b2ddee6a3f938447`; 103,747 bytes.

- `web/pkg/my_summer_slop_bg.wasm`: `7e1f3e2cf21060da504cf6399072f70b8b21e1142fc221c25b159dbf579045e7`; 53,552,956 bytes.

- `web/assets/reference/manifest.json`: `ac94af1b9f30ef97a5e1a88fef911ce96ab2a328a9b54213b8beddc1fbf9d7b1`; 4,085,882 bytes.

Logs use `.cache/lighting-merge-*.log`. The browser capture is `lighting-merged-master-ready.png` in the local visualization folder.

## Local material checkpoint merge: 2026-10-11

Base master: `39d9e3fc04bef4e09d4066fd864e6adf88fed523`.
Checkpoint: `e3625b8199b05c6aec7c70383c989ac09ef39541`.
The merge retains master lighting, the accepted library, menus, instruments, mounting, and driving controls.
The unfinished rendering branch remains separate.

| Check | Result |
| --- | --- |
| Native suite | 233 passed |
| Installed-source checks | 42 passed |
| Exporter suite | 170 tests; one skipped |
| Browser unit and server tests | 23 passed |
| Formatting, WASM check, release and bindgen | Passed |
| Accepted library | 262,784 owners verified; receipt unchanged |
| Selected runtime export | Five scenes; native contract and dependency validation passed |
| Production browser | Garage ready; Start enabled; no GPU errors |
| Shader readiness | 11 source material pipelines ready; 13 receiver variants |

Nine semantic texture slots share seven image banks.
The complete WebGL2 layout uses 16 textures, 16 samplers, and 12 uniform buffers.
The merge preserves source image bytes, mip levels, samplers, UV transforms, and channel rules.
MainTex keeps bank zero for alpha shadow passes.
Critical checks cover bank order, absent MainTex, overflow, and parallax binding.

Road lamps retain the accepted AmplifyRope material behavior.
Sky meshes use their existing sky material, outside surface material binding.
The merge removes the obsolete source_lighting shader after combining its calculations with source_surface.
The final shader removes an unused shadows import that caused ShaderImportNotYetAvailable.
The production browser compiled the fixed shader and reached the existing readiness gate.

The embedded browser rejects pointer lock. Startup and visible garage rendering passed.
This check does not establish continuous mouse capture or matched native pixel parity.
Source texture storage reconciliation and exact Task 4 rendering remain deferred in the roadmap.

Logs and failed diagnostic evidence remain in the managed material-checkpoint-merge worktree under `.cache/merge-*`.
The final browser logs are `merge-final-device-logs.json` in the local visualization folder.
The garage image is `merged-master-garage.png` in that folder.

Package and export hashes:

- `web/pkg/my_summer_slop.js`: `23b8f17a3a62954cfe5a59a30450c060c899af451adc51ca815840db57c15abd`; 103747 bytes.
- `web/pkg/my_summer_slop_bg.wasm`: `02c9c4a33f9f3e2b5c96b22155c4187fef9a3977f8e1f8a98b31408f0fc71451`; 53733353 bytes.
- `web/assets/reference/manifest.json`: `d62630828b68c154c9351203b11f67427916390bd9a3eac1b911fda8efb5c74e`; 5712654 bytes.

## Source audio checkpoint: 2026-10-12

The audio branch implements source vehicle sounds, interaction sounds, and the Lada radio.
The assembled browser produced a captured door effect after trusted Start.
Device listening and the ten full acceptance scenarios remain incomplete.
The user requested "skip tests" during the listening check. Further checks stopped; no missing result counts as passed.

The Python audio suite passed seven cases. The corrected browser audio suite passed 29 cases with zero skipped.
The earlier full check and release build passed. That default native check excluded 46 installed cases.
A later run included installed cases: all 31 audio and 11 radio cases passed.
The user stopped that run during reference validation, after 226 tests passed in completed targets.
Normal runtime verification stopped at 260,000 of 262,784 records without a final receipt.
Whole-branch review remains with the controller.

The [audio verification report](research/2026-10-12-audio-verification.md) lists evidence and each remaining scenario.
Earlier verification entries retain their historical scope and results.
## Live mirror acceptance, 2026-10-12

Status: Equipment branch implementation passed source, native, GPU, lifecycle, and full product checks. Whole-branch review passed at `9abb5e7`.
The user approved the mirror spec and plan and chose subagent execution.
The selected source is Steam build `11140890`, Unity `2021.3.18f1`.
The checked managed source hash is `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee`.

| Source link | Checked ID |
| --- | --- |
| Mirror components | `level1:66131`, `level1:67237`, `level1:70638` |
| Source cameras | `40260`, `40324`, `40222` |
| Screen renderers | `43665`, `40735`, `44371` |
| Plane transforms | `28414`, `38060`, `32190` |
| Camera transforms | `39254`, `31061`, `26965` |
| Render target | `sharedassets1.assets:4495` |
| Live and disabled materials | `sharedassets1.assets:624`, `sharedassets0.assets:116` |

The exporter regenerated five selected scenes from the checked accepted library.
It verified 262,784 library owners against receipt `92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99`.
The worktree manifest hash is `4ac150e8880ef3cef9f7e7e0ca7e465acac2677ca1ac10215a1d3214c987fe8e`.
The Lada glTF and binary hashes are `b2e4a65efe377ef85f82cabd45fbe6845fa8f84647266eda1645455c0a3e27d0` and `f79b68feb818e7489f67ec0fa70561f4f2f2338a50ed2391ce3d80bd4beb81e7`.
The primary source hashes and modification times stayed unchanged.
The export receipt is `.cache/equipment-checks/task-5-export-receipt.json`.

`MirrorViewRole` distinguishes the normal view from each `Capture(index)` view.
The runtime reads plane and camera nodes after physics, mechanism updates, and origin transport.
It updates capture poses after transform propagation and before camera and frustum updates.
It tests normal-view visibility before scheduling captures by source rank ahead of the normal view.
The producer uses the player body for rank and the active normal camera for reflection and FOV.
It retains the source five-mirror cutoff and previous-far capture order.
Loading computes current pose validity without advancing source rank or far state, including started restarts.
Warmup and gameplay reject invalid vectors and invalid clipping. Valid hidden mirrors can warm.
Loading prepares mesh/material pairs, hidden lamp choices, owner substitutions, sky banks, and both view keys.
Normal frames select cached pipelines through three fixed view slots. Pipeline failures remain visible after readiness.

Each selected instance owns one 256 by 256 linear RGBA8 target, one D24S8 capture depth, and live and disabled materials.
The source mask is `73113395`; source camera postprocessing, depth effects, and heat haze are empty.
Sky time and world lighting stay shared. Each view computes its own flare projection and occlusion.
Normal exposure, jitter, motion history, and completion state stay separate.
The owner screen uses its disabled source material during its capture.
Other mirrors can use completed targets. The complete material-bank audit found no writable target in sampled bindings.
Mirror reset advances the instance ID and releases old cameras, images, materials, and depth storage.
Preparation and completion reject stale IDs.

| Check | Result and evidence |
| --- | --- |
| Recovery | Four Python mirror/staging tests passed; `.cache/equipment-checks/final-python.log` |
| Final suite | Formatting, 22 web tests, 245 native tests, and release WASM passed; `.cache/equipment-checks/final-full-check.log` |
| Installed parts | Ten mirror tests passed; `.cache/equipment-checks/final-native-check.log`. Installed binding checks: `.cache/equipment-checks/final-installed.log` |
| Finite clip | GPU projection tests covered near, far, and outside bounds; invalid source far suspended capture, with RED 26 and GREEN 0 failures |
| Feedback | Task 3 drew all nine owner/view pairs; three changing targets had zero GL errors; each owner used its disabled screen |
| Vectors | Invalid-axis RED failed; corrected GPU GREEN rejected coincident, invalid-axis, and nonfinite inputs for ten frames each, then recovered |
| Source scene | Final current run passed 45 actual-Lada stages, including part changes, carry/drop, rebase, pause, resize, and restart |
| Epochs | Final current run rejected two stale epochs and released prior cameras and images across three lifecycle cycles |
| Sky | Opposite normal and capture directions changed flare visibility while shared cycle time stayed fixed |
| Final product startup | `/diagnostic?task=final-product` reached ready=true, errors=0, glErrors=0, and 377 warmup capture draws; the idle count stayed fixed |

The labelled actual-Lada driving scene used the production seat entry pose, vehicle physics, and mirror producer.
The diagnostic held FORWARD and IGNITION after seating; the normal input sampler still ran each frame.
At tick 60 it set a post-seat baseline. At tick 300 the car moved 0.549 m, the mounted camera anchor moved 0.491 m, and the capture pose moved 0.491 m.
Two captures were eligible. Root saw the cabin and mirrors, with zero browser and GL errors.
The run had a running engine and released handbrake, but did not measure gear or output torque.
Its near-zero final speed does not prove sustained powered driving. Trusted driving remains unproved.

The final current run kept three capture cameras, three owned color targets, three depth attachments, six mirror materials, and 32 sky materials.
Loading prepared 46 capture variants and 109 total pipeline IDs. Those counts stayed fixed from first readiness through three transaction cycles.
All post-readiness backend samples kept 56 GL programs. Three new source mesh assets reused supported layouts without pipeline growth.
Cold pair snapshots held 498 entries at startup and 504 on each restart. Owner mappings stayed at 6.
Settled backend counts were 149 textures, 7 renderbuffers, and 11 target textures.
Requested target storage was 82,268,768 bytes. Six 256-square capture and HDR textures used 2,359,296 bytes.
These are requested GL storage bytes, not measured driver memory.
Mirror inputs retain capacity 3. The renderer uses three fixed view slots and has no render job Vec.
No whole-engine allocation trace was run. See `.cache/equipment-checks/final-current-result.json` and `final-current-browser.jsonl`.

Task 3's isolated scene used a moving red cube and three spaced mirror screens.
Its target readbacks changed, and the finite projection test clipped outside near and far bounds.
The native signed-transform test checks capture direction under asymmetric parent and local rotations.
These checks do not establish a matched source image or every production view orientation.

Controller rulings applied during implementation:

1. Runtime export staging uses the selected output parent. Otherwise export could write into the read-only accepted cache.
2. Signed scale affects hierarchy positions and meshes. Unity `Transform.up` and `Transform.forward` use rotation alone. A wrong ruling would change camera direction.
3. Mirror inputs use runtime coordinates. A second source-axis Z conversion could reverse capture orientation.
4. Invalid source far at or below near suspends capture and selects the disabled material. Clamping would change source clipping.
5. Individual mirror removal keeps its world plane and resets controls. Door removal retains mirror controls. A wrong ruling would change mounting behavior.
6. Native critical interaction tests and labelled actual-Lada browser checks satisfy equipment implementation acceptance. Trusted input, sustained powered travel, and matched appearance remain unproved. A real-input or integrated driving defect may need follow-up.

The installed Unity handling of invalid far is unknown.
The in-app browser refused pointer lock after a trusted Start click. Windows UI control stopped at its URL-confidence limit.
The visible part, lifecycle, and seated-motion scenes used labelled deterministic input. Native critical interaction tests passed. These checks satisfy equipment implementation acceptance.
Trusted walk, adjustment, removal, mounting, and driving traversal remain unproved. Sustained powered road travel also remains unproved.
No matched native and target camera images exist, so source appearance parity remains unclaimed.
Export and verification failures now remove only their fresh staging directory.
Injected checks preserved accepted output, prior recovery folders, and failed-promotion staging. Existing recovery folders were not deleted.
The final scoped whole-branch review passed at `9abb5e7`.
No plate or portable flashlight work belongs to this mirror acceptance.

Final product package: WASM SHA-256 `4bfd7937d3eb621e252b72c299b5d06eca7bb6c166ad14a16cee5bfa6d243bf0`.
The product JS SHA-256 is `66ed491671ed5b9c04daee673d371aab42bbaa01dc26b2e70701fe8a632a2c75`. Fresh release compilation took 2m 15s after source restoration. The WASM contains no diagnostic marker.
The source and package hashes are in `.cache/equipment-checks/final-package-pairing.json`.
The command report is `.superpowers/sdd/2026-10-12-live-mirrors/final-fix-report.md`.
Controller screenshots are in `C:/Users/Corashina/.codex/visualizations/2026/10/12/01a12ea6-fdca-7740-aa22-558ee084f1ee/`.
Current lifecycle proof is `mirror-final-lifecycle-pass.jpg`. Earlier checks retain `mirror-gpu-scene.jpg`, `mirror-invalid-clip-pass.jpg`, and `mirror-seated-motion-pass.jpg`.


## Save integration with both vehicles

The master merge retains the Lada, Skoda, and three existing Lada mirror captures.
Each vehicle has one save record for its body, engine, gear, handbrake, trip distance, and headlights.
Vehicle and seat records use source instance identities. The selected vehicle returns after an on-foot save.
Load checks assets for both dashboards. New game and Load reset the mirror capture epoch.

The merged checks passed: 55 web tests, 270 native tests, formatting, and the release WASM build.
The normal native run skipped 54 installed-source checks.
Twelve Python mirror recovery and export staging tests passed.
Logs: `.cache/save-merge/check-capacity-final.log` and `.cache/save-merge/export-tests.log`.

The driven/rebase test found a false changed-body record for source `level1:1`.
Changing the origin during load added `5.96e-8` metres to its recovered absolute Y coordinate.
Capture now compares active source poses in the current local frame. Dormant records keep their absolute positions.
The existing snapshot comparison passed after this fix.

The new installed-source fleet checks cover distinct vehicle states, reordered records, Skoda selection on foot, seats, dormancy, and recovery.
Cross-car wheel checks restore each rim under its current vehicle owner.
The seven installed save checks passed against the merged source export.
The Skoda driving and recovery check, three mirror checks, and existing dormant cargo recovery check also passed.
Logs: `.cache/save-merge/final-source-package.log`, `.cache/save-merge/installed-tests.log`, and `.cache/save-merge/recovery-tests.log`.

The accepted export contains both vehicles, 633 colliders, and 223 items.
Its manifest SHA-256 is `11fcb21d76fba0222a13582996e2eac669a1dfa8e53424e21c367dc6008c5e57`.
The exporter verified the accepted library and selected inputs before native validation and atomic promotion.
Both vehicle scenes now use the shared source mirror material mapping.

Skoda source mechanism `level1:71713` starts at 90 degrees outside its drag range.
Save validation accepts the exact source startup value or a value inside that range.
A focused check rejects the adjacent value of 89 degrees.

Distant recovery now selects the absolute road destination before waking a dormant vehicle.
It hibernates source bodies before moving the origin to that destination.
The fleet check recovers the Skoda from coordinates of ten billion metres and preserves the dormant Lada record.
The browser fixture adds `Both cars, save, reload` for distinct engine, trip, brake, and headlight states.
These controlled fixtures do not establish uninterrupted trusted mouse capture or source appearance parity.

The merged source needs 25 fixed lights and 15 streamed road lamp slots.
The CPU and shader arrays now hold 40 lights. The 100 shadow slots remain sufficient.
A focused check confirms matching array counts and a 16,080-byte uniform within the 16,384-byte WebGL2 limit.
Log: `.cache/save-merge/lighting-capacity-test.log`.

The production package passed startup and six browser session checks with both source vehicles.
The WASM SHA-256 was `06ef87e571bfbc7e0564d20f518767eaf19faa1dc271d0ba46e5e391bd56ee0c`.
Repeated session replacement reset mirrors and still allowed Continue.
A manual save survived a full page reload. Continue resumed that restored session.
The snapshot comparison measured maximum position error of `3.33786e-6` metres and quaternion component error of `3e-7`.
The visible Load action returned to pause with `Game loaded`. Save returned `Save complete`, with actions enabled.
No GPU error appeared during these checks. The default package had no fixture hooks.

The controlled fleet checks used fixture WASM SHA-256 `7a784ad083d8e965ebc67775a301b11d18a656120b8d0fb4c87042369033af6b`.
They used the same accepted source manifest listed above.
Both distinct vehicle records survived a manual save and full page reload.
The Skoda remained selected by stable source identity `level1:69517` while the player stayed on foot.
Both engines resumed from elapsed time 10 to 10.216663 seconds.
The two trip values remained 1234.5 and 1634.5000001192093 metres.
The restored runtime had 114 bodies, 701 colliders, four wheel supports, no owner errors, and no duplicate supports.
Maximum position error was `3.33786e-6` metres; quaternion component error was `3e-7`.

The controlled driving check covered a rebase, manual save, full page reload, and resumed road contact.
Travel reached 46.78494750846679 metres. Total fuel changed from 23 to 22.98723856 litres.
The selected trip reached 46.86606230050236 metres.
Maximum position error was `2.49744e-5` metres; quaternion component error was `3e-7`.
The runtime retained 114 bodies and four wheel supports, with no owner errors or duplicate supports.
Its 704 colliders became 701 after the streamed window updated on resume.
All four wheels reported contact before resume. Resumed motion preserved stable chassis height.

The parts check covered removal, held parts, fluids, cycle state, full page reload, resume, and the normal F drop action.
It restored 116 bodies, 701 colliders, three wheel supports, and one carry joint.
Eleven part colliders had the correct source owner. No owner errors or duplicate supports appeared.
Maximum position error was `3.33786e-6` metres; quaternion component error was `2.9e-7`.
Fluid transfer moved 0.25 litres from volumes [2, 0.5] to [1.75, 0.75].
A second transfer after load reached [1.5, 1].
Cycle time crossed from 1499.95 to 0.1886 seconds with its fade state preserved.
The F action removed carry state and left zero throw charge. The dropped body remained active.

Active-session storage fault checks also passed on the fixture package.
An aborted write preserved the previous slot and active session.
Corrupt data, source mismatch, unavailable storage, and missing metadata each preserved the active session.
The checks compared the complete normalized snapshot, camera, origin, and required source record.
These controlled inputs do not prove uninterrupted trusted pointer lock or matched source appearance.

The scoped integration review found no issues in code commit `6935e23f751222296085c228d99f7768f6c3b44a`.
The review covered merge resolutions, fleet save ownership, dormant recovery, mirror integration, and the light-capacity fix.
Later commits record verification only.

## Rendering merge verification, 2026-10-13

The user authorized merging rendering commit `67798ec` into local master `a5b9778`.
The merge combines source rendering, both vehicles, save/load, audio assets, and three Lada mirrors.
The verification date is 2026-10-13. Logs dated 2026-10-12 record earlier checks in this merge.

| Check | Result and evidence |
| --- | --- |
| Native suite | 318 passed; 58 installed checks ignored in the normal run; `.cache/rendering-merge/native-final.log` |
| Installed source | 58 passed across `.cache/rendering-merge/package-fixed.log` and `package-final2.log` |
| Web suite | 87 passed; `.cache/rendering-merge/web-final.log` |
| Exporter suite | 182 passed; `.cache/rendering-merge/exporter-tests-final.log` |
| Final export fixes | Six library checks and one renderer metadata check passed; `export-finish-unit.log` and `renderer-metadata-after.log` |
| Asset validation | Native manifest, receipts, geometry, audio, lighting, and inventory passed; `.cache/rendering-merge/verify-candidate-v4.log` |

The native log contains a later failed installed garage check.
After correcting that check, the team reran it and the two remaining wheel checks in `package-final2.log`.
Those three checks passed. The earlier installed run passed the other 55 checks.

The accepted source manifest SHA-256 is `deeb0fbcf4e5fd933a78a0c2906f15ca5f6d320e05b002160d7ff1fc2be3f2cd`.
The export contains 1,632 files, two vehicles, three mirrors, 196 audio clips, 258 materials, and two probes.
The accepted library manifest SHA-256 remains `92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99`.
The team preserved the accepted library and prior runtime export under `.cache/rendering-merge/reference-before`.

The merged renderer has 12 fixed views: three main views, six shared probe capture views, and three mirror views.
Each vehicle probe owns six 64-square faces and seven mip levels. Each probe keeps its own publication state.
The runtime requests exact eight-sample HDR and D24S8 depth storage on the accepted hardware.
Source fetches share an eight-request limit through the existing `async-lock` dependency.

The team corrected paused mirror warmup and reset probe elapsed time during scene replacement.
Atomic face stamps removed conflicting mutable probe queries.
Load now warms live mirror mesh IDs and removes stale terrain pairs from the prior scene.
The renderer retains supported material variants during that reset.

Production Load and repeated Load passed before the final default rebuild.
The mirror audit reached stage 5, with bank mask 63, 23 pipelines, and three validated submissions.
Load removed 11 obsolete mesh/material pairs. Both probes published, and all three main views completed.
The motion pool retained 953 slots. Reuse increased from 698 to 1,047 across the two Load audits.
Evidence: `.cache/rendering-merge/mirror-fixed-load.json` and `browser-repeated-load.json`.

The final fixture lifecycle check passed six checks on 2026-10-13.
The checks covered same-frame pause with Save, New Game, and Load.
They also covered queued cancellation, dispatched completion, and usable Continue.
Evidence: `.cache/rendering-merge/lifecycle-final.json`.

The controlled fleet check passed a manual save and full page reload on 2026-10-13.
It restored both vehicle records and kept Skoda source identity `level1:69517` selected while on foot.
Maximum position error was `3.337860107421875e-6` metres. Maximum quaternion component error was `3e-7`.
The restored scene had 114 bodies, 701 colliders, and four wheel supports.
The audit found zero bad collider owners and zero duplicate assembly bodies.
Both engines resumed from elapsed time 10 to 10.23333 seconds.
Their distinct trip values reached 1634.5000423569968 and 1234.500024261401 metres.
The renderer returned to Ready, completed all three main views, and published both probes.
Its mirrors reached stage 5 with three validated submissions. The motion pool retained 953 slots.
Evidence: `.cache/rendering-merge/fleet-final.json` and `fleet-renderer-final.json`.

The final default release build and `wasm-bindgen` packaging passed on 2026-10-13 in 3m 08s.
The build used `cargo build --release --target wasm32-unknown-unknown --locked --lib`.
The default JS exports contain no `testsFixture` or `saveFixture` hooks.
The WASM has 58,725,410 bytes and SHA-256 `484317a045ea038630dff6c0732baf430713f4135d9e8529578e007ec5f4f49d`.
The JS SHA-256 is `e3f0f40cd8369b4a27a1aae2de7674dd75a22eb1f710bae3504fe7560f46f2a6`.
Build output: `.cache/rendering-merge/stock-final-build-current.log`.
Source and package hashes: `.cache/rendering-merge/stock-final-package-current.json`.

The final default package passed browser startup and the visible Load action on 2026-10-13.
The garage reached Ready. Load returned `Game loaded` with Continue enabled.
Load restored the saved Skoda record with trip distance 1634.5 metres.
Both audits reported Ready, main-view mask 7, two published probes, and three validated mirror submissions.
The Load audit reached mirror stage 5 and removed 11 obsolete mesh/material pairs.
The team found zero browser ERROR entries during the final default checks.
Evidence: `.cache/rendering-merge/stock-startup-final.json` and `stock-load-final.json`.
The final Load screenshot is `.cache/rendering-merge/final-rendering.jpg`.

Matched native pixels, exact native blend rounding, and continuous trusted pointer capture remain unverified.
The earlier audible acceptance limit remains unchanged. See `research/2026-10-12-audio-verification.md`.
## Source plate lifecycle, 2026-10-12

The equipment branch retains two imported text entities and two mesh handles.
Both plates share the checked source font atlas and one plate material.
Restart replaces each mesh payload under its existing handle.
Bevy frees old vertex and index allocations before replacement in `MeshAllocator::free_meshes`.

Restart now clears game readiness. Input and physics wait for current plate and mirror preparation.
The first ready callback opens the start menu. Later ready transitions keep the existing restart menu state.
`GarageRuntime::restart` starts the new game. The player can pause it through the existing menu.
Plate binding uses an exclusive world system. It has no queued asynchronous binding work.
Plate reset advances its instance and invalidates mesh revisions. Mirror preparation rejects earlier instances.

The user asked to skip further tests. No new assertions, Python suites, installed checks, or full checks ran.
The release product and ignored browser probe builds completed.
The controller uses held browser stages for loading restart, plate transactions, pause, resize, and three restart cycles.
The ignored observer counts live GL buffers and requested bytes without changing GL state.
The source and product package receipts are in `.cache/equipment-checks/plate-lifecycle-pairing.json`.
Source library, font, atlas, and scene receipts are in `plate-lifecycle-source-receipts.json` in the same directory.

These browser checks use scripted camera poses and item transactions.
Trusted mouse capture and matched Unity appearance remain unproved.
The source TMP shader uses the supported forward path. Full Unity deferred shader behavior remains outside this work.

The controller observed four ready states across three restart cycles, with plate instances 3, 4,5,6.
Each retained 251 meshes, 130 images, one plate material, 297 source materials, 1373 GL buffers, and 11029400 requested buffer bytes.
The backend retained 152 textures, 58 programs, 11 target textures, and three RGBA8 capture targets plus three HDR intermediates.
Current main targets resized from 1280x720 to 1024x576 and returned. Target storage returned to 82268768 bytes.
The readiness logs retained 48 capture variants. The controller found no warning, runtime error, or GL error.
The visible front, rear, carry, drop, remount, bumper, plate removal, and origin-shift labels remained readable.
The loading hold covered game readiness; synchronous text binding could finish before release.
The first mirror placement screenshot proves placement only. It does not prove readable text in a capture.

The final held resume view reported paused=false twice. Physics steps increased from 44 to 1527.
The active source mirror showed partial DG/DGR glyphs from the selected plate through the existing capture path.
Full mirror label readability remains unproved because the string was clipped or occluded.
The controller restored production and observed ready=true with Start and Controls enabled, errors 0, and glErrors 0.
The controller verified all 85 source/package/asset hash entries after restoration.
Final screenshots include plate-lifecycle-resumed.png, plate-source-mirror-text-settled.png, and plate-final-production.png.

## Portable flashlights, 2026-10-13

Source: Steam build `11140890`, Unity `2021.3.18f1`. Branch: `codex/equipment`.
The selected export includes the older active garage flashlight and the inactive newer scene object.
The older controller starts OFF. The newer controller and body start disabled.
The newer model has no gameplay acquisition route.

The older source spot uses 0.85 intensity, a 12 m range, a 90 degree cone, and cookie 3.
The newer source spot uses 1.5 intensity, a 15 m range, an 80 degree cone, and cookie 4.
Both lights use source shadow strength 1, bias 0.05, normal bias 0.4, and near plane 0.2 m.
The renderer keeps 34 light slots, 100 shadow slots, and the 4000 pixel source atlas.
One 512 by 512 three-layer cookie array holds the prior custom cookie and both flashlight cookies.
Normal, mirror, and plate materials share its checked array binding.

Source storage uses mask `8388609`, a 1 second drop delay, and a scan interval above 1.5 seconds.
The source locks a flashlight only when its collider appears in the preceding and current scans.
The trunk and hood slots apply source restrictions. Storage types 5 and 7 cannot lock.
Loading yields twice and runs three QuickFreeze passes. Only the first pass prescans.
Readiness waits for all five frames, source lighting, plates, and three mirror targets.
The runtime syncs wheel mass and item poses only when a storage frame changes ownership.
The constructor sets initial poses before those loading frames.

The user skipped further automated tests and assertions. Task 4 uses release builds and visible browser review.
The corrected release build passed. WASM SHA-256: `216e4317410fa35e37d5c31664fe9573cb8d7d1e1eb9a711d9baba4106949d52`.
JS SHA-256: `a6a58cad9b9e2df83c335255e6a6238f60da9a702c640bd075d04bb9e57be40e`.
The current export manifest is `3e1914fe710147da8b7d39cc17518a78c8a5502ca9b94adbd96a49bfe33db7f2`.
The source library stays at `92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99`.
The model glTF is `015a25dfb58694d7ca8700b40886b6aa43165a186bbbf9251f88593432158191`.
The 36-entry `.cache/equipment-checks/flashlight-final-fix-receipts.json` records code, models, cookies, and source shaders together.
The ignored probe restore record is `.cache/equipment-checks/flashlight-lifecycle-stage-pairing.json`.
The ignored lifecycle route labels scripted placement, fixed input, active physics, and the inactive-model prototype.
Its reports use `.cache/equipment-checks/`; captures use the task visualization directory.
In the labelled newer-model stage, the yellow and black model and a white wall beam were visible.
The model root, body, and item pose matched `[37.781, 3.415, -69.666]`; the gameplay body stayed disabled.
Two model meshes were visible. The garage startup still kept the newer item inactive.
The captures are `flashlight-task3-new-model.png` and `flashlight-task3-new-beam.png` in the task visualization directory.
The active mirror showed the reflected car, interior, and lighting. Its direct white beam detail was not readable.
The prototype does not prove source-matched beam pixels, cone, cookie, or shadow shape.
The first ready route had five models, 309 source materials, 137 images, 29 bound lights, and 257 mesh assets.
It had 1,317 mesh entities and three mirror cameras. The backend retained 159 textures and 58 programs.
The 11 target textures requested 82,268,768 bytes; 1,422 buffers requested 11,217,648 bytes.
The targets included one 4000 by 4000 shadow atlas and three 256 by 256 RGBA8 mirror colors.
Three 256 by 256 HDR intermediates came from Bevy. These byte counts describe requests, not driver memory.
The mirror cache held 48 capture variants and 113 total pipeline IDs at first readiness.
The startup queue skipped 1,826 unknown capture keys. That count stayed fixed during the observed ready interval.
Three native restart cycles restored old/new OFF, the older World, newer inactive, mass 961.60004 kg, and origin zero.
The matching garage snapshots after all three cycles kept the same model, material, image, light, mesh, buffer, target, and program counts.
All three cycles kept 48 mirror variants, 113 total pipeline IDs, and the same unknown-key counter.
Texture creation and deletion reached 186 and 27; 159 textures remained live.
At 1024 by 576, live textures, programs, targets, and buffers stayed at 159, 58, 11, and 1,422.
Requested target storage fell to 78,287,456 bytes. Buffer requests stayed at 11,217,648 bytes.
At the restored 1280 by 720 size, requested target storage returned to 82,268,768 bytes.
The six 256 by 256 mirror and HDR targets kept their size. Texture totals reached 192 created and 33 deleted.
Live textures stayed at 159. Capture variants stayed at 48, pipeline IDs at 113, and unknown keys at 1,826.
The visible startup had old OFF, newer OFF and inactive, and storage ready after its fifth frame.
World E and held LMB selected old control 10. Held LMB changed ON to OFF to ON.
Three scripted native carry steps changed the held item rotation. Competing world LMB changed only control 10.
An actual drop stayed loose 0.45 seconds later in the earlier build.
The earlier trunk lock and first-scan hood lock used an inverted overlap rule. These results are superseded.
In the corrected build, an actual drop occurred at native clock `2.0162654` seconds.
The first eligible scan ran after `1.5333319` seconds and found the torch in the current scan only.
The torch stayed World. The next scan ran after `3.0166783` seconds and found it in both scans.
That scan locked it to owner 27 in trunk volume 6.
The probe held the loose body at the source trunk center during active physics.
Its scripted pose hold kept the collider inside the volume across both scans.
In a second drop, the early scan ran after `0.34999466` seconds with both memberships true.
The torch stayed World until a later scan at age `1.8499718` seconds locked it.
The native clock and storage scheduler advanced through resumed frames in both routes.
The corrected trunk lock changed owner 27, collider parent, and chassis mass by 1 kg.
The prior build checked trunk release, blocked type 5, allowed hood type 4, and origin shift.
Those checks did not run again after the overlap fix.
The corrected build rejected stored pickup with the old light ON and kept owner, body, collider parent, mass, and ON.
Valid pickup restored Held, loose body 7, collider parent 7, mass `961.60004` kg, and ON.
Held LMB changed OFF to ON once per press. Native restart returned both lights OFF and restored the starting item state.
It reached storage readiness with origin zero and mass `961.60004` kg.
The prior build kept its clock fixed during pause with the old light ON and item Held. Resume advanced physics.
After corrected restart, the route retained five models, 309 materials, 137 images, 29 lights, and 257 mesh assets.
It retained 1,317 mesh entities, three capture cameras, 48 capture variants, and 113 pipeline IDs.
It kept 1,422 buffers, 159 textures, 58 programs, and 11 targets at 1280 by 720.
The buffers requested 11,217,648 bytes; targets requested 82,268,768 bytes.
The targets included one 4000-square shadow atlas and six 256-square mirror and HDR textures.
The unknown capture-key count stayed at 1,826 during corrected play.
The parent rehashed all 36 current receipts without mismatch.
The restored corrected production route reached garage readiness with Start and Controls enabled and three mirror previews.
It reported 387 capture draws, zero browser errors, and zero GL errors.
The corrected captures are in the task visualization directory.
The earlier scripted origin shift checked item ownership, physics, and the garage area. It does not prove full world streaming.
These checks cannot prove trusted continuous input, exact native callback timing, or matched Unity pixels.

## Equipment integration into master, 2026-10-13

The merge joins equipment `b9749b7` with master `c924e89`.
It retains both vehicles and the current rendering, audio, save, survival, and weather systems.
The selected source export has two plate objects, two flashlight records, and eight storage volumes.
The Lada plate slots remain empty. The checked Skoda plate objects retain their source targets.

Flashlight feedback uses bit 2097152. Food and drink keep their existing bits.
The shared light uniform has 42 entries and requests 16,368 bytes, below the 16 KiB limit.
Stored item mass and collider flags use the actual parent vehicle.
Plate glyphs use the current camera, lighting, sorting, and depth bindings in scene and mirror passes.
Required sorting values come from the source renderer even when its editor glyph mesh is omitted.

The browser check found two current renderer lifecycle faults during integration.
Cube copies passed `TEXTURE_CUBE_MAP` as a framebuffer face and produced GL error 1280.
Retained render copies now use native cube face bases and nearest texel centers for all three copy boundaries.
New food scenes could become visible before source material conversion.
They now stay hidden until every native mesh has its source material binding.

Fresh source derivation, native contract validation, and the release build passed.
The runtime manifest SHA-256 is `5a09eede8ca78fd95ded997ccabaf6c5e32d8c69e58941f1a147c236d3408963`.
The WASM SHA-256 is `377b5f1102540c70b9d82101ad135378e5f404dacd3f360190a254f6109ac75b`.
The JS SHA-256 is `8cf3aea78cd0b6dda377e8168225b91eab9449be5d6ef55760c6686f736f1173`.
Command logs, source receipts, and browser records are in `.cache/equipment-merge/`.

The final browser route reached readiness, New game, Save complete, and Game loaded.
It reported zero browser errors and zero GL errors after each lifecycle change.
The final draw-only observer uses no framebuffer readback or fault-consuming trace.
The screenshot is `equipment-master-merged.png` in the task visualization directory.
Automated tests and assertions were skipped under the user's instruction.
These checks do not prove matched Unity pixels, trusted continuous input, or changed equipment ownership persistence.
