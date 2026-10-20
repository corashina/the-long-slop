# G10 control verification

Date: 2026-10-20. Status: Verify.
Source authority: Steam build `11140890`.
Integrated into `master`. Original branch: `codex/g10-garage-controls`.
Original worktree: `C:/Users/Corashina/.codex/worktrees/g10-garage-controls/the-long-drive-wasm`.
Implementation production commit: `b23a4b580a67c3592061faac2e02d0793af02e10`.
The Task 5 commit adds tests and this record. It changes no product behavior.

## Master integration

G10 includes master `43f2eb5c9b76382e295ac6f452f26babf23cb44b` and its current renderer, inventory, ignition, posture, and save contracts.
Integration retains the instrument preparation gate and separate current and capture library receipts.
The superseded source-identity helper was removed.
G10 fixtures now use current ignition, source posture, and the real equip path.
Initial formatting and obsolete fixture API failures were corrected. No production compatibility path was added.

The normal exporter derived, validated, and promoted this matching package:

| Identity | Value |
| --- | --- |
| Package | `b05648ba0de407131d30c65320cb68be36eb025a46afa05eda18722d0faf812f` |
| Runtime manifest SHA-256 | `e4000dde993f617e90898b03c8c087fc71678cd112a8963614860b4e46389dc3` |
| Capture library SHA-256 | `92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99` |

Library, source, and catalog identities remain the values recorded below.
The current package has a separate capture receipt. No proof pin or source input changed.
Python passed 11 tests. Web passed 14 tests. Native passed 14 G10 tests with none ignored.
Formatting and both WASM backend checks passed. Existing compiler warnings remain.
Scoped integration review found no issue.
Evidence is retained in [`.cache/g10-integration/`](../.cache/g10-integration/).
Original implementation evidence remains in [`.cache/g10-control-verification/`](../.cache/g10-control-verification/).

Release builds, full checks, and browser acceptance were not repeated for integration.
The release receipts below describe the original implementation build.
All twelve normal-input scenarios remain open. Radio persistence remains a G13 dependency. G10 stays Verify.

Build before serving the merged code:

```powershell
Set-Location 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm'
./scripts/build.ps1
$env:PORT = '43125'
node scripts/serve.mjs
```

Retained source input for a later normal export is `.cache/g10-integration/source-input/library`.
The prior installed reference export is retained in `.cache/g10-integration/reference-before`.

## Implemented controls

The branch adds source endpoint reactions, LMB handbrake use, secondary-seat L eligibility, dashboard lighting, and radio visuals.
The existing modules own these changes.
Tests cover endpoint direction, intensity, body ownership, transforms, material links, input priority, and control roles.
These checks do not prove normal gameplay or audible output.

## Implementation package

| Identity | Value |
| --- | --- |
| Package | `524b389a4e117291c46d225fd7343ce10b2df66556cfa18b434d169d394455e3` |
| Runtime manifest SHA-256 | `5d97fc473de78e8752a0edc9239c710e4818fb49873bcf7bea2eab6d9897c190` |
| Library manifest SHA-256 | `3e7d8260600a09a74bb3d89c5c8a8772904dddeb6ab740b154af95745f58aeb4` |
| Source manifest SHA-256 | `254830925305facc97113479ecdea6f154389ed354afa42aa996ead492d5c04d` |
| Catalog SHA-256 | `ba572df2c46e84c2839c805655c252acb3ef8964a776c2eeb378bb3de0b1b13f` |

The final identity check matched all five values.
The exporter and Rust reference schema have no changes after the production commit.
Task 5 therefore reuses Task 4's independent normal verification, as approved by the controller.
Its command was:

```powershell
$g10Python = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/tools/reference/.venv/Scripts/python.exe'
$g10Library = 'C:/Users/Corashina/.codex/worktrees/g10-garage-controls/the-long-drive-wasm/.cache/g10-reference-input/library'
& $g10Python -m tools.reference.export --stage runtime --library-root $g10Library --output web/assets/reference --verify
```

That command exited 0. `task-4-local-verify.log` holds the original result.
`task-5-confirm-identities.py` and `task-5-identity.log` record the fresh identity check.
Task 5 did not derive or promote another source package.

## Targeted checks

All commands ran from this worktree.
Evidence is archived under [`.cache/g10-control-verification/`](../.cache/g10-control-verification/).
The execution scratch was removed after final review.

| Command | Exit | Result and evidence |
| --- | --- | --- |
| `$g10Python -m unittest tools.reference.tests.test_mechanisms tools.reference.tests.test_instruments tools.reference.tests.test_radio tools.reference.tests.test_audio tools.reference.tests.test_world -v` | 0 | 12 passed. `task-5-final-python.log`. |
| `./scripts/check.ps1 -Filter g10_` | 0 | 14 native tests passed. None ignored. Formatting and both WASM backends passed. `task-5-final-native.log`. |
| `./scripts/check.ps1 -Filter g10_control_restore_has_no_side_effects` | 0 | Final saved-motion assertion passed. Formatting and both WASM backends passed. `task-5-final-restore-motion.log`. |
| Same focused command after source endpoint inspection | 0 | Final changed-endpoint assertion passed. Formatting and both WASM backends passed. `task-5-final-restore-endpoint.log`. |
| `./scripts/check.ps1 -WebTest audio,input,menu` | 0 | 13 passed. `task-5-final-web.log`. |
| `task-5-confirm-identities.py` | 0 | Package, source, catalog, library, and schema checks passed. `task-5-identity.log`. |
| `./scripts/build.ps1` | 0 | One release build produced WebGL, WebGPU, and terrain worker modules. `task-5-build.log`. |
| `git diff --check` | 0 | No whitespace errors. `task-5-diff-check.log`. |

The release build completed WebGL in 9 minutes 38 seconds and WebGPU in 7 minutes 2 seconds.
Compression prepared 393 files. The runtime manifest hash remained unchanged after the build.
`task-5-build-receipts.json` records all generated JavaScript and WASM hashes and sizes.

| Generated WASM | Bytes | SHA-256 |
| --- | --- | --- |
| `my_summer_slop_bg.wasm` | 40,597,804 | `72fe4521d6f62ea41a3fe634b23a5e1d2028d39ce46fa73f6a6c77cf70d66bb4` |
| `my_summer_slop_webgpu_bg.wasm` | 39,092,634 | `b3f926ba283b4857c89d5be226c347d924a9a802d80ac956c0ab38acebf27868` |
| `terrain_worker_bg.wasm` | 43,979 | `dbf7906503765cfc5e906c3b834d81c80e42dbe978360cd13613c99621141701` |

The native lifecycle test uses the supported garage fixture and save methods.
It changes a mechanism endpoint, headlights, ignition, engine readings, handbrake, distance, and radio controls before saving.
It confirms saved mechanism, lighting, engine, headlight, handbrake, and distance values after restore.
It confirms zero new linear or angular impulse from pending endpoint reactions.
Restored body motion also matches saved body motion before that pending-reaction check.
It checks the actual audio event queue for replayed endpoint sounds.
It constructs an instrument sample from restored controls and checks derived visual state.
Saved ignition and low beams light the dashboard. The temperature needle matches its exported angle rule.
It checks restored radio controls and effective power. Visible radio materials and needle position remain unverified.

The web lifecycle tests observe the real `InputState` and `VoiceBank` outputs.
Capture loss clears held horn input. Resume retains no press.
Radio resume creates one playback source at the same offset.
Repeated frames and unlock calls do not duplicate that source.
The audio fixture uses accepted WAV bytes and a format-correct decoded buffer description.
These tests do not establish audible sound.

The first web run passed the horn test and failed the radio fixture.
The shared fake decoder returned an ignition duration for radio clips.
Normal audio validation rejected those decoded samples.
The corrected WAV fixture passed without a product change.
`task-5-first-web.log` records that fixture failure.

The first native command stopped at formatting before it ran the test.
After formatting, the restore test passed without a product change.
`task-5-first-native.log`, `task-5-format.log`, and `task-5-restore-native.log` preserve those results.
Self-review found that the first fixture assigned the control's initial maximum endpoint.
The final fixture assigns its opposite endpoint and asserts changed source angles before saving.
It passed the focused check. This corrects test setup, not product behavior.
Native checks retain four existing dead-code warnings.
Both WASM checks retain four existing material warnings.

## Original browser acceptance

The implementation test used this command:

```powershell
Set-Location 'C:/Users/Corashina/.codex/worktrees/g10-garage-controls/the-long-drive-wasm'
$env:PORT = '43125'
node scripts/serve.mjs
```

URL: `http://127.0.0.1:43125/`.
The server reported that URL. No existing server was stopped.
No save was created or overwritten. No existing browser tab was changed.

The browser inventory exposed the in-app browser with no tabs.
Its browser and local development documentation were read.
Native Chrome was absent in the fresh native inventory.
The supported `sky.launch_app` call started Chrome.
The next inventory returned `New Tab - Google Chrome`, window `22022646`.
The following window observation stopped Computer Use with this message:

> Computer Use has been stopped for this turn because it could not determine the current browser URL on Windows with enough confidence to enforce policy.

The tool required UI work to stop. No UI retry or alternate input path followed.
No game page or control input ran before that stop.
There is no runtime screenshot or audio recording.
`task-5-browser.log` holds the exact tool result and server command.

| # | Required scenario | Result | Evidence and remaining check |
| --- | --- | --- | --- |
| 1 | Start, capture, active physics, submitted frames | Blocked | UI stopped before navigation. No physics or frame evidence. |
| 2 | Four doors, hood, trunk, collider alignment | Blocked | Requires an active session and normal drag input. |
| 3 | Endpoint transitions and sustained endpoint hold | Blocked | Native reactions pass. Visible reaction behavior remains unverified. |
| 4 | Windows and all five home switches | Blocked | No normal input reached these controls. |
| 5 | Enter and exit all five seats; role controls | Blocked | No seat was reached. G09 owns seat transition corrections. |
| 6 | Handbrake E, LMB, and Space | Blocked | Native press and priority checks pass. Normal input remains unverified. |
| 7 | Physical lights, L, cabin modes, high-beam indicator | Blocked | Native role and instrument checks pass. Visible results remain unverified. |
| 8 | Rear lamps under ignition, brake, and reverse | Blocked | No source-condition runtime check. Driving physics stayed unchanged. |
| 9 | Audible horn, release, pause, capture loss, resume | Blocked | Web input lifecycle passes. No audible horn evidence. |
| 10 | Radio volume, tuning, band, playback, screen, needle | Blocked | Contract and playback lifecycle checks pass. Visible and audible results remain unverified. |
| 11 | Gauge ranges, warnings, readability, lighting matrix | Blocked | Native matrix passes. Gauge readability remains unverified. |
| 12 | Save restore visuals, endpoint silence, radio voices | Blocked | Native lifecycle passes for supported controls. Radio persistence remains a G13 dependency. |

## Dependencies and limits

`SaveSnapshot` has no radio power, volume, band, or tuning fields.
Restore initializes the existing radio state. G10 does not change the G13 save schema.
The lifecycle test records that actual behavior and derives visuals from it.
It does not replay radio input or claim persisted radio restoration.
Required radio restore acceptance remains open under G13.

No current runtime result establishes a capture, asset, render, physics, or audio failure in this package.
The observed blocker belongs to the UI tool's URL check.
All twelve normal-input scenarios remain open. G10 stays at Verify.
Full checks were not run. They were outside the approved task.
