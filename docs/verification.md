# Verification record

## Test suite reduction: 2026-10-15

The user requested removal of the least useful 80 percent of tests.
Baseline: `fa2265c`, with 809 declared cases. We retained 162 and removed 647, or 79.98 percent.

| Suite | Before | Retained |
| --- | --- | --- |
| Rust, including tests inside source files | 494 | 100 |
| Python exporter tools | 212 | 40 |
| JavaScript browser interface | 103 | 22 |

We retained save transactions, cancellation, validation, ownership, fluid conservation, input boundaries, physics, and exporter publication safeguards.
We removed repeated formula, layout, metadata, and fixture checks, plus tests for obsolete station APIs.
We removed unused test helpers and imports. The check script now names only retained JavaScript suites.
Retained fixtures use the current sorted world fields and request-based candidate identity.

The JavaScript suite passed 22 tests. Python discovery passed 40 tests.
The native Rust suite compiled: 13 passed, 71 failed, and 13 remained ignored.
Three retained rendering tests require the WASM target and did not run in the native suite.
The shared fixture already lacked the required `world` field before this cleanup.
Most native failures stop at that boundary. The earlier integration report below records the same baseline defect.
Retained installed-source tests also require a matching current export. These results do not establish gameplay acceptance.

This reduction removes coverage for individual shader boundaries, UI labels, media variants, and source scalar combinations.
The retained suite provides narrower coverage. Failed fixture checks remain visible.

## Cleaning and current master integration: 2026-10-15

We merged source-world, source driving, startup, and cleaning owners under strict save schema 4.
Master already used version 3. The combined contract rejects versions 1, 2, and 3.
The contract includes world changes, source spawn state, selected gears, driving assists, and semantic conditions.
Imported items use ConditionStore. Generated items retain their source-world quality without a duplicate imported save scalar.
Candidate loads reserve condition records and commit the retained spare material bank at the scene boundary.

The required check stops at six web failures also reproduced on master `45d0474`.
The focused browser checks passed: 50 tests for feedback, save sessions, storage, and menus.
The full native command stops compiling obsolete save_capture world APIs, also reproduced on that master.
The unchanged world_source target uses removed field APIs and does not compile.
The remaining focused native set ran: 30 passed, 123 failed, and 35 ignored.
Master's fixture lacks the required world field. Most failed tests stop before their behavior assertions.
The new imported tire and generated quality tests also stop at that fixture boundary.
One reference test embeds the previous feature export and rejects its obsolete station_prototype usage.
Current-master lib and drivetrain runs reproduce the missing-world failure.
We did not replace source inputs, weaken assertions, or repair these baseline defects.

Python condition/world/export unit checks passed: 20 tests. They do not validate a complete merged export.
Fresh runtime export is blocked: the checked `.cache/world-source/samples.json` is missing.
The actual merged world verifier rejected that missing prerequisite in the primary retained directory.
Historical package receipts below do not establish a matching merged runtime/package or active gameplay acceptance.
The local merge report records compiler results, package hashes, and exact logs.
Cleaning and painting remains Partial. Active input, source comparisons, and retained GPU acceptance remain open.

These feature receipts predate the master integration. They do not verify the merged package.

## Historical feature package: Cleaning and painting, 2026-10-15

Branch: `codex/cleaning-painting`. Frozen code head: `1b51c38834aa88b907d53a684ba317211395e194`.
Frozen tree: `464373cf565e3e7c4bd235f638af6ce51554a01f`. Scoped re-review remains open.
The [approved specification](superpowers/specs/2026-10-14-cleaning-painting-design.md) governs this feature.
Read the [source record](research/2026-10-14-cleaning-painting-reference.md) for identities, values, and source rules.
We keep Cleaning and painting Partial. Implementation and menu readiness do not complete gameplay acceptance.

We implemented six authored startup tools, live surface/wash/tool state, brush wear, and carried-nozzle spray painting.
We retain colors and condition state through part ownership and save schema 2.
The loader rejects old schemas and invalid candidates. It preserves live bindings during failed candidate preparation.
New Game restores exported startup values. Recovery preserves preparation; Load clears transient contact and valve output.
We removed duplicate scalar ownership and temporary cleaning diagnostic routes. The checked startup initializer remains necessary.

### Final particle fix checks

We verified the recorded logs and package hashes on 2026-10-15. We did not repeat completed builds or suites.

| Target | Result | Evidence log |
| --- | --- | --- |
| particles / cleaning / conditions_reference | 7 / 26 / 9 passed | `final-fix-green-wasm.log` |
| Sensor amendment: particles / affected formatting / WASM library | 7 passed / passed / passed | `final-fix-sensor-wasm.log` |
| Source conditions / weather | 3 passed / 2 passed, 1 baseline failure | `final-fix-source-tests.log` |
| Native survival_reference | 1 passed, 2 baseline failures | `final-fix-focused.log`, `final-fix-baseline-weather.log` |
| Final source export and verification | Passed; 663 colliders, 232 items | `final-fix-export-final.log` |
| Release and browser package | Exit 0; release compile 4m29 | `final-fix-build.log` |

The native base and current fix fail the same assertions in `tests/survival_reference.rs`.
At line 75, the shelter count is 4 rather than 3.
At line 131, validation accepts a duplicate shelter root.
Python base and current fix fail `test_rejects_rebound_shelter_links` at line 27: no ValueError.
The Python baseline log is `final-fix-baseline-source-weather.log`.
We checked exact reviewed base `6d9179a3d64fc4819273df6b28bc0249233654f6` with unchanged assertions.
Base and live reference fixtures share SHA-256 `afab4f80586b1f2cb5c6b51a1b1ef5c6eff65dad85f825feeba10b2f26a8b12b`.
The controller accepted these baseline limits. Full survival and Python weather acceptance remain open.

### Historical Task 5 integrated checks

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

We retain the named Task 4 evidence for earlier checks. Final particle fix results above supersede affected targets:

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

The final particle fix supersedes the Task 4 fix1 package and its Task 5 source export.
The prior WASM hash was `d5a554a973f94be9e98fe226f0eefd318c8d25aa8d1a096e45f2f1dc1b258425`.
The prior manifest hash was `46db456b73b7d78d4ba532750ce4b39fd6fcc12a89b4cd80a2f03731926d72ac`.
We checked frozen code head `1b51c388` and confirmed no later production edits.
The final export receipt and current source hashes match.
The release log records exit 0 and 4m29 compilation. Package files have timestamp 2026-10-14T20:08:32Z.

| File | Bytes | SHA-256 |
| --- | --- | --- |
| `web/pkg/my_summer_slop_bg.wasm` | 60757157 | `cf80cd66d0cf6526594c045cd26545d9e862116741e7e6c971c9d33741c24a6f` |
| `web/pkg/my_summer_slop.js` | 108929 | `dff466b79a8ef77b0c0c0c8811406ebc03b3e6697216c955b39a37695e5febd1` |
| `web/feedback.js` | 11512 | `5d464dd6adbf5fce3cdedf47c986cfc1625018fbed3d5ec908bafb92b1014328` |
| `web/assets/reference/manifest.json` | 25307587 | `9fed7506b9f84511c0918e23917d7bb3e606b8c96f678a8e7a29698b6e1ec517` |
| `web/assets/reference/conditions-evidence.json` | 2678512 | `99a18f97e7dc4cd1c23bca18b105d552c537e467806ae973b31be4609b4b1b68` |
| `web/assets/reference/library-selection.json` | 1239655 | `821a747ac8b02c4dcc385b32be6081f642bfc9c2247db53fe0217b4d02093a4b` |

The accepted receipt hash remains `a87176a07c287a9c8ccf7ae977c12c46dd1dc811017d965d8b5b44e599d30b41`.
Generated assets and source receipts remain outside Git.
The source identity changed. The controller used a fresh same-export save for final Load acceptance.
The controller owns the scoped re-review.

### Browser evidence and open gates

The controller checked the final particle fix package on 2026-10-15 at `http://127.0.0.1:4178/` in IAB tab 2.
Receipt: `final-fix-browser.md` in the local feature evidence directory.
Fresh navigation reached garage readiness. New Game, a fresh same-export Save, and Load reached enabled pause controls.
Continue showed `Mouse capture failed`: the root document was not valid for pointer lock.
The menu stayed paused. The controller performed no active movement, tool action, or driving.
The screenshot confirmed the paused garage and capture message. The tool supplied no saved local image path.
Fresh browser inventory still omitted Chrome despite the user's connection reports.
The current error-level query returned no entries. Startup event capture remains incomplete.
Bevy can publish warnings at log level. The error query does not establish warning-free operation.

Earlier Task 2 diagnostic packages proved retained receiver/caster GPU identities, independent material changes, and candidate isolation.
They retained 1118 meshes, 122 owners, 168 receivers, 1680 condition materials, 2266 source materials, and capacity 5812.
First diagnostic WASM SHA256: `4aa3a23d50a0e68a4118f109018f065d570e0051b1d7b693a1fa3f23fa5d5379`.
Overlap diagnostic WASM SHA256: `bd567814d33ef9b13e60e242bf84a539b6ad36dc2a6f95c4a0c51157e0e8077a`.
Those named packages predate spray effects. They do not prove final can buffers or active particle/binding counts.

The controller accepted two bounded shared-renderer deferrals. Both remain known visual differences.
Spray material `sharedassets1.assets:481` lacks scene-color refraction; its checked distortion strength is 50.
Brush material `sharedassets1.assets:50` lacks sampled-depth soft-particle fading; intersections keep hard edges.
The shared rendering work must resolve these differences before matched source particle acceptance.
We keep Cleaning and painting Partial.

Complete the following route before marking the feature complete:

- Use all six tools on eligible surfaces of both cars. Remove and reinstall changed parts, then drive both cars.
- Check wheels, doors, fluids, lights, plates, storage, and existing save behavior through active input.
- Check pause, death gating, recovery, save/load, repeated restart, and origin changes during preparation and driving.
- Compare matched source actions, controls, surfaces, tool state, camera, main view, shadows, mirrors, reflections, and audio.
- Measure final material, particle, and binding counts after repeated active work and load/restart.

The fixed build hash color policy does not reproduce Unity's random sequence.
The particle sweep retains the shared FluidContact non-sensor filter as an explicit adaptation.
Native particle trigger membership needs matched source proof; ray queriesHitTriggers does not establish it.
Native cache/shape ordering, collision messages, and world tank-fill callbacks remain outside the shared particle route.
The controller owns independent whole-branch review and branch integration.

## Driving branch verification: 2026-10-14

Status: partial acceptance on `codex/correct-driving-physics`.
The final fix follows review base `6f8118162ece6d9ee24c811719c0935af433487a`.
The whole-branch review required seat-transition and suspension-bound fixes.
The controller also traced missing-engine ownership and required an automatic-control fix.
The implementation addresses these findings. Scoped re-review and current-package browser checks remain with the controller.
The earlier full suite ran on `05bbf7bf5726d91bc1a0cdec2adf900753028dc9`, tree `be156d38886e62c60180c5a948db30e923631e53`.
Final-fix checks cover the changed behavior; they do not replace that full-suite record.
Earlier entries below describe their named revisions. The current driving save schema is 2.

### Final fix checks

Seat changes clear pedal ramps, pending shift edges, and ignition holds before vehicle input routing.
The engine clears its input without a release event. Selected gears and mechanical state remain per car.
The transition path retains the existing push-grip owner.
`DrivetrainState::step` takes `Option<EngineOutput>` from the existing mounted-engine check.
`None` skips automatic gear, pedal inversion, ignition, and RPM clutch rules. Manual rules keep their source conditions.
Rust rejects suspension targets outside `[0,1]` and negative spring or damping values.
It accepts both target endpoints and zero coefficients. Radius and travel must remain positive.
The exporter calls this same validator before accepting runtime output.
The feedback test captures its expected invalid-audio warning during setup and restores `console.warn` in `finally`.

Fresh logs and exact commands reside in [final-fix evidence](../.cache/driving-physics/acceptance/final-fix/commands.jsonl).
Red checks reproduce pending entry shifts, an exit ignition toggle, invalid suspension acceptance, and automatic shifts without an engine.
Green checks cover both cars through real `GarageRuntime::step` entry and exit, plus normal engine-part removal.
The lifecycle fixture isolates collisions and door access. It does not establish native driving or garage access parity.
The suspension table validates both current exported cars through the public manifest contract.
Three Python checks pass, including accepted exports and rejection through the exporter's Rust validation path.
The audio and feedback selection passes 37 tests. Production warnings remain enabled.
Full native compilation and the release package checks use the final fix; see the final-fix command receipt.
The earlier full native result remains 261 passed, 108 failed, and 66 ignored on `05bbf7b`.
The final fix does not repeat that suite or classify its unresolved failures.

### Gameplay rules

Tasks 1–3 passed their scoped code reviews.
The branch adds required source controls/wheel records, fixed-step pedal ramps, manual gears, clutch, and two saved assists.
The engine owner applies clutch engagement once. The drivetrain applies gear and final-drive ratios once.
The wheel owner distributes final axle torque by active driven-wheel count and installed radius.
Ordinary wheel updates add no allocation or physics object creation.
The branch removes obsolete gear/force APIs and adds no old save reader or migration.

Critical native checks cover engine/stall rules, transmission order, both-car seat transitions, saved gears/assists, and obsolete-save rejection.
Rust validates a save before candidate creation and scene publication.
The browser Prepare decoder rejects obsolete fields through strict decoding before candidate creation.
That browser path bypasses `SaveSnapshot::from_json`'s version preflight; the independent native check covers the preflight.
See [source controls and adapter limits](vehicles.md#driving-controls) and the
[Task 3 review](../.superpowers/sdd/2026-10-14-correct-driving-physics/task-3-review.md).

At exact zero driven-wheel RPM, the endpoint-clamped port can retain neutral.
Its fallback samples exceed both cars' curve endpoints. The native direction check uses 100 RPM.
Source `CalcTorque` calls `torqueCurve.Evaluate(_wrpm)` without a clamp or floor wrapper.
Serialized `m_PreInfinity` and `m_PostInfinity` values of 2 do not establish the public `WrapMode` mapping.
We have no measured native curve behavior outside the source key times. Source boundary parity remains conditional.
The Skoda Rust test passes movement assertions within a failing test. We observed no launch deadlock.
Normal-input browser launch for both cars remains unverified.

### Commands and exits

Workdir: `C:/Users/Corashina/.codex/worktrees/correct-driving-physics/the-long-drive-wasm`.
Fresh outputs and retained Task 3 evidence reside under `.cache/driving-physics/acceptance/`.
The [exact commands](../.cache/driving-physics/acceptance/commands.md) record Docker mounts, toolchain identity, and bounded concurrency.
The [gate receipt](../.cache/driving-physics/acceptance/gates.json) records each fresh command and exit.

| Gate | Exit | Result | Evidence under acceptance/ |
| --- | ---: | --- | --- |
| `./scripts/check.ps1` | 1 | 96 web checks pass; existing module-order format failure stops native/build stages | `check.log` |
| Python exporter discovery | 1 | 194 run: 181 pass, four failures, eight errors, one skip | `exporter.log`, `exporter-triage.md` |
| Source archive `--stage source --verify` | 0 | Accepted archive verifies without a source receipt change | `source-verify.log` |
| Runtime `--stage runtime --verify` | 0 | Full library and runtime contract verify; 798.635 seconds | `runtime-verify.log` |
| Installed-source ignored selection | 101 | Initial 13 pass/five fail; two mount retries pass; three physical failures remain | `installed-source.log` |
| Two mount-blocked source checks, corrected read-only mounts | 0 each | Both pass; original selection remains failed | `mount-retry.json`, two retry logs |
| Full native compile and suite, reused final code | 101 | Compile passes; 261 pass, 108 fail, 66 ignored | `reused-layer-3/native-full.log` |
| Critical driving sequence and WASM check, reused final code | 0 | Engine nine, drivetrain six, driving saves two, obsolete schema one pass | `reused-layer-3/critical-green.log` |
| Full web/HUD, reused final code | 0 | 97 pass | `reused-layer-3/web-full.log` |
| Release build and wasm-bindgen, reused final package | 0 | Same `scripts/build.ps1` command body, with two compiler jobs | `reused-layer-3/package.log` |
| Documentation links, fragments, scope, and whitespace | 0 | New links/fragments, five-file scope, and whitespace pass | `docs-check.log`, `diff-check.log` |

The 108 native failure names match the retained Task 0 inventory through the Task 2/3 comparisons.
The [native comparison](../.cache/driving-physics/acceptance/reused-layer-3/native-comparison.json) has empty added/removed lists.
The aggregate check remains unmet. Focused checks do not replace its failed format/native gates.

The exporter suite has no complete Task 0 baseline.
Its failures concern audio fixture data, resource hashes, selected scenes, fleet counts, a switch link, and shelter validation.
The failing test files and error-producing validator functions are unchanged across the branch.
Relevant shared fixture sections also match the pre-task values.
These facts do not establish an unchanged full exporter baseline.
The [exporter triage](../.cache/driving-physics/acceptance/exporter-triage.md) preserves all twelve names and causes.
The installed-source selection ran eighteen Rust cases using exported assets.
Two checks passed after corrected read-only mounts for external Windows junction targets.
The initial aggregate exit remains 101; fifteen test cases pass across the run and affected retries.
Three physical cases remain unresolved: parked drift, failure to sleep before the exit route, and inactive Lada trip accumulation.
The Skoda test passes its movement assertions before failing the inactive Lada bound.
These ignored cases have no complete prior source-test baseline. They remain separate from the 108-name native comparison.
Read the [source-test triage](../.cache/driving-physics/acceptance/installed-source-triage.md) for names, values, and retry commands.
Task 6 changes no product code or failing assertion.

### Source and physical comparison

Reference: Steam build `11140890`, Unity `2021.3.18f1`.
Primary evidence: [driving receipt](../web/assets/reference/driving-evidence.json),
[source recovery](../tools/reference/driving.py), and [driving research](research/2026-10-11-rea-driving-audit.md#driving-branch-update-2026-10-14).
Accepted source identities:

| Input | SHA256 |
| --- | --- |
| `Assembly-CSharp.dll` | `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e` |
| `Assembly-CSharp.cs` | `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee` |

The [source measurement audit](../.superpowers/sdd/2026-10-14-correct-driving-physics/source-measurement-audit.md)
found zero qualifying repeated native observations for each required metric, for both cars.
The user confirmed that no source measurements are available.
Acceleration, coast-down, service braking, combined braking, turning, suspension settling, tire/rim changes, and empty hubs lack measured intervals.
Static fields, formula probes, Rust fixtures, and browser audits supply no native repeatability or measurement resolution.
Task 4 cannot pass. Task 5 remains unscheduled. Measured source deltas are unavailable.
The retained [audit copy](../.cache/driving-physics/acceptance/source-measurement-audit.md) records inspected locations and source gate gaps.

The adapter supplies eligible summed brake inputs.
The stock solver excludes simultaneous motor force and brake impulse on one wheel.
Tire/rim friction, hub support, suspension, and wheel spin remain incomplete.
We cannot claim Unity WheelCollider behavior, matched stopping distances, or source handling parity.

### Browser input and lifecycle

Earlier controller route: Codex IAB `browser2/tab2`, loopback port 4187, Task 3 package.
These observations predate the final fix. The controller owns the current-package browser check.
The tool did not provide an exact browser version.
Desktop viewport: 1280x720. Narrow viewport: 800x600. The controller restored the default viewport afterward.
The controller used normal UI actions and the existing read-only render audit. The controller injected no product state.
Read the [route log](../.cache/driving-physics/acceptance/reused-layer-3/controller-browser.md) and retained screenshots.

| Check | Observed result |
| --- | --- |
| Reload and New game | Ready garage; both visible assists true |
| Obsolete schema 1 Load | Rejected before candidate publication; paused scene retained |
| Disable Automatic clutch | Both choices false; Automatic gears disabled |
| Paused Save and full reload Load | Save complete; Game loaded; false/false restored |
| Desktop and narrow Controls | Controls fit; source key help wraps within the scroll area |
| Pause-menu New game after Load | Visible assists reset from false/false to true/true |
| Recover | Selected once; no visible result proved recovery |
| Continue | IAB refused real pointer capture |
| Read-only render audit | Ready, scene 2, master state 1; warmup false, mirrors true, motion validated |

Capture error: `The root document of this element is not valid for pointer lock`.
The read-only motion flag describes renderer readiness. It does not establish a driving route.
No normal launch, both-car continuous driving, manual/assisted shifts, pedal releases, neutral coast, reverse, or live stall/restart route passed.
The 30/60/120 FPS checks cover fixed-step input samples in native tests. Browser handling routes remain unavailable.
Live gear feedback, lights/instruments/audio during driving, part exchange, seat switching, recovery, restart, and rebasing remain unverified here.
The old-save error persists when Controls opens until another menu action resets it; the controller retained that screenshot.
The schema 2 test save remains in localhost:4187 storage.

### Code and artifact identities

The final fix rebuilds WASM: 60,419,042 bytes, SHA256 `c271113f1d771898216832d9ea3e1edb560e8f580c07d8ffc5525d230963906f`.
The bindings remain 109,237 bytes, SHA256 `eb7ebf28feb5c37de871cd76d6255ba4ba851a4f894867697a884db894b0662f`.
Eleven of the twelve artifact hashes match Task 3; only WASM changed. The current package receipt is [final-fix hashes](../.cache/driving-physics/acceptance/final-fix/hashes.json).
Its [source receipt check](../.cache/driving-physics/acceptance/final-fix/source-receipts.json) retains the accepted inputs.
We reuse the unchanged 262,784-record source archive scan.

The following table records the earlier Task 3 package. Task 6 rechecked its twelve hashes and byte lengths.
They matched at that revision. The [full hash receipt](../.cache/driving-physics/acceptance/hashes-checked.json) identifies each file.
Four accepted source/library receipts also remain unchanged; see
[source receipts](../.cache/driving-physics/acceptance/source-receipts-checked.json).

| File | Bytes | SHA256 |
| --- | ---: | --- |
| `web/assets/reference/manifest.json` | 15962156 | `ac34fccde1d23e0ae773d1aa22ec6134d0822670c81518b0ea6125c63a238bf4` |
| `web/assets/reference/driving-evidence.json` | 19734 | `8cebfdc2b0c040437c2cfc9e490b929b565500ea9f0ea3fb6d13d6b918d1d04c` |
| `web/assets/reference/library-selection.json` | 1214615 | `7ea4d10cd024fedb66d89330ce8ad3eb375560d3f8eb918e789133552ecda666` |
| `tests/common/reference.json` | 10534215 | `c81a1900e641854d8ff3cfd74b1a983d56b89992a1e0cc92578f47209bcc0e50` |
| `web/pkg/my_summer_slop.js` | 109237 | `eb7ebf28feb5c37de871cd76d6255ba4ba851a4f894867697a884db894b0662f` |
| `web/pkg/my_summer_slop_bg.wasm` | 60418464 | `f2585cc290d6e3b3a377c4367c006e33a8884e2d8e98b92c8bf6bf7126d658b1` |

The whole-branch review reported changes required. The final fix addresses its confirmed findings.
The controller owns one scoped re-review. Driving acceptance remains incomplete.
Retain the isolated worktree and evidence. Merge and push remain outside this task.

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

## Source-world implementation: 2026-10-13

The source-world branch replaces the old sine road, fixed stations, chunk bands, simple props, and lamp sampling.
It keeps one generator and one required world contract. Startup randomness retains its existing arithmetic in `starting_area.rs`.
The checked context uses build 11140890, Unity 2021.3.18f1, seed 42, and saved 5,000-metre default settings.
Generation and replay retain the frozen checked initialization frame. Runtime rebasing does not change that frame.
The road contains 50,000 segments. Off-road grids have no finite extent.

The final layer uses static source tracing and native/WASM product compilation.
Current-source native compilation passed in 3.81 seconds on 2026-10-14, without warnings.
Release WASM compilation passed in 2m02s. `scripts/build.ps1` also completed wasm-bindgen packaging with exit code 0.
Final logs are `.cache/world-source/task-9-resume-final-native.log` and `.cache/world-source/task-9-resume-final-package.log`.
The earlier Task 9 final logs predate two source edits and do not cover the committed source.
The user directed Ignore tests. No final tests, fixtures, captures, source comparisons, browser checks, or traversal ran.
No 100-kilometre route, full-extent traversal, continuous drive, seam, rebase, recovery, restart, or revisit acceptance ran.
Historical browser evidence above describes earlier layers. It does not validate the current source-world runtime.
Task 2's historical startup capture still contains an unresolved 404 URL and GPU warnings.
The accepted qualification remains in the retained Task 2 report. No recapture ran.

World numeric, runtime, visual, and physics parity remain unverified.
The supported initialization order does not prove Unity scheduling or original game-wide random timing.
Terrain uses the accepted shared illumination API. Global reflection and SH-volume contributions remain outside this world layer.
Canonical grass ordering does not prove source-device primitive ordering.

### Ownership and bounds

| Owner | Limit |
| --- | ---: |
| Active geometry, CPU and logical GPU | 128 MiB each |
| Shared world prototypes, CPU / logical GPU | 256 MiB / 512 MiB |
| Pending world work | 32 MiB, two jobs, 32 requests |
| Checkpoints, spatial indices, and first-call source history | 16 MiB |
| Road / near terrain / each distant tier | 512 / 512 / 256 |
| Generated object instances | 8192 |
| Registered light proxies, including fixed lights | 32 |
| Shadow slots | 100 |
| Actual change records | No fixed cap |

Road checkpoints occur every 256 segments. Building checkpoints occur every 32 selections.
First-call source history stores parent/caller identity, random state, phase, and required cursor inputs.
This history grows with first encounters and can exhaust its limit before full-extent travel.
Actual changed identities have a separate map and byte counter. Physics motion counts as a change.

Task 7's decoded world, catalog, and material ownership was 132,704,763 bytes.
Its conservative static selected-owner peak was 227,638,888 bytes before Task 9's additional owner-table and identity charges.
Those figures describe allocation formulas for the checked inputs. They are not process, WASM, driver, or route measurements.
Task 9 also charges stream maps, supply metadata, structure handle buffers, and retained placement identities.
Runtime admission remains required for each selected region.

Pending staging overlaps active/prototype reservations. Do not add those counters as disjoint process memory.
The JavaScript bootstrap has separate text, parse, fetch, and hash lifetimes.
One dependency verifier replaces the earlier two workers.
Input chunk views and their contiguous copy can overlap by twice the dependency length, at most 64 MiB.
This bounds visible payload bytes. Chunk backing buffers and hash copies can require more memory.
The 32 MiB world staging limit does not bound JavaScript bootstrap memory.
The full manifest text and JavaScript object can overlap Rust input and decoded ownership.
Browser internals, allocator high-water memory, and driver overhead remain unavailable.

### Development snapshots

Call `await window.measureWorld()` from development tools after the runtime starts.
The next application frame returns the snapshot. No technical player interface was added.
`world` contains capacity charges, reservations, source history, changes, preparation counts, and road replay call timings.
`assets` counts unique mesh/image handles across imported baseline, prototypes, active world, pending world, and other renderer assets.
Shared caster and instance handles count once. Mesh byte metadata survives CPU extraction and retires with its asset.
Texture bytes include complete mip/layer dimensions. CPU payload counts use retained vector capacities.
Logical handle bytes do not prove GPU upload completion, allocator storage, or resident VRAM.
Missing mesh metadata or unsupported texture formats increment `unknown_gpu_handles`.
The imported baseline covers asset payloads. It excludes complete imported reference, ECS, physics, skins, and uniform ownership.
The snapshot names those limits and keeps process and driver fields null.
Measurement-cache capacity has its own counter. Requested snapshot maps and JSON are transient development allocations.

Browser fields include WASM linear memory, available JavaScript heap data, initial Start readiness, and peak application-frame interval.
WASM linear memory is committed capacity. JavaScript heap data depends on browser support.
Frame intervals include pauses and background scheduling. Replay timings cover road checkpoint replay calls only.
No snapshot values were collected during this task.

### Durable evidence

Completion evidence remains ignored under `.cache/world-source/completion/2026-10-13/`.
The `sdd` copy preserves the ledger, task briefs, reports, review records, and rulings with their recorded costs.
The same directory keeps copies of the approved specification and plan.
The Task 9 report records final commands, package hashes, deleted coverage, static review, and unperformed acceptance.
Immutable original records and receipts remain under `.cache/world-source/task-7-input-provenance/`.
The original receipt sidecar SHA256 is `2c20e3d2c2d62a4123eeb0e1208c806ae9e0f0c7cdf4094699110321d02e6a8c`.
The original record sidecar SHA256 is `65a0309fa1fca84d78a8d4051e3c8a0fb54d5ef70745a675a4eea8f076c20b96`.
Recovered assets, source, captures, and local completion evidence stay outside Git.

## Source-world final review fixes: 2026-10-14

The Task 9 evidence above covers the 2026-10-13 implementation before final review fixes.
This section records the later F1, F2, and F3 corrections.

- Resource admission now uses verified glTF input sizes and bounded PNG decode overlap.
  Retained generation protects 19,826,124 bytes for resource preparation within the shared 32 MiB cap.
  Replay keeps that headroom. Existing grass work can release its slot while displacement writers wait.
  Synchronous collision and binding work can use total free staging after dependencies are ready.
- Generated render transforms subtract the double-precision origin before float conversion.
  Retained local matrices handle bodies, parent transforms, nodes, rebases, and displacement writers.
  Separate source matrices preserve procedural shader inputs.
- Both startup loaders and scene spawning select the three imported asset usages explicitly.
  Painting materials load for selected variants and release with their last dynamic owner.

The final native build passed with `native_exit=0` in 0.21 seconds.
The final `scripts/build.ps1` run passed with `package_exit=0`; release compilation took 2 minutes 6 seconds.
That script built release WASM and ran wasm-bindgen browser packaging.
Both JavaScript files passed syntax parsing. Git whitespace validation passed.
These builds cover every final code edit. No shader source changed.

Final logs and package hashes remain ignored under `.cache/world-source/`:

- `final-fix-complete-native.log`
- `final-fix-complete-package.log`
- `final-fix-js-parse.log`
- `final-fix-input-metadata.json`
- `final-fix-package-evidence.json`

The full fix report remains at `.superpowers/sdd/2026-10-12-source-world/final-fix-report.md`.
Earlier failed and intermediate logs remain historical evidence.
No tests, simulations, captures, browser checks, or driving checks ran under the current user instruction.
Runtime parity, memory plateau, long travel, restart, revisit, and rebase acceptance remain unverified.
