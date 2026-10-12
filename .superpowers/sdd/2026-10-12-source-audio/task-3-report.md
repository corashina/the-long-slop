# Task 3 report: source vehicle audio

Status: DONE_WITH_CONCERNS.
Base: 556f988. Branch: codex/audio-runtime.
Worktree: C:/Users/Corashina/.codex/worktrees/audio/the-long-drive-wasm.
Commit subject: Add source engine starter ignition and horn playback.

## Implementation

I added VehicleAudioInput and AudioState::vehicle(input, dt, simulation_seconds).
Rust updates retained idle, motor, and horn signals after each fixed step.
The horn uses the lighting cycle visual_temperature value.
I retain ignition events across fixed steps. A running-to-stopped edge queues one engine stop effect.
Engine removal stops engine loops and starter owners. It preserves car horn and radio signals.
I resolve engine stop positions through the existing emitter anchor map.

I added VoiceBank.starter(emitter, enabled, simulationSeconds) and the scalar gameAudioStarter bridge.
The browser checks scaled simulation time against the source deadline.
It starts one shot when elapsed time exceeds clip length times engineStartTimeModifier.
It uses no future lookahead. Release cancels pending decode starts and preserves active tails.
Starter and stop effects follow the existing engine emitter.
I reuse retained voice slots and decoded buffers. Parameter updates do not replace active sources.

I added H and Numpad8 to the held horn bit.
I added required SeatReference.driving2, source export, fixture, and PlayerState.driving2().
The controller approved this correction after I found a missing source seat flag.
Horn and ignition accept driver or driving2. Throttle still requires driver.
The accepted Lada HornSwitch array is empty. I added no mechanism binding.

## Source evidence

Accepted Steam build: 11140890. Managed source SHA256: 22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee.
Source file: .cache/reference-library/records/managed/Assembly-CSharp.cs.

- enginescript.Update, lines 12053-12225: idle and motor gains, unclamped pitches, strict starter comparison, and engine stop edge.
- carscript.Update, lines 7451-7458: MoveTowards horn gain with 10 * dt.
- carscript.Update, lines 7837-7848: held horn loop and ambient temperature pitch.
- carscript.GetPlayerInput, lines 8523-8533: driver-only throttle and driver/driving2 horn eligibility.
- saveload defaultInput, lines 55062-55063: H and Keypad8.
- seatscript.GetIn/GetOut, lines 53313-53360: driverSeat0 and driverSeat1 ownership.

Source idle limits are 600 and 1400 RPM. Motor gain limits are 500 and 1300 RPM.
Both maximum gains are 0.699999988079071. Maximum RPM is 6600.
Idle pitch endpoints are 0.8999999761581421 and 1.2000000476837158.
Motor pitch endpoints are 0.800000011920929 and 1.5.
Horn pitch endpoints are 0.8999999761581421 and 1.0. Ambient bounds are -9 and 35.
Starter binding 96 uses clip 195, source length 0.5338775515556335 seconds, and modifier 1.
Stop binding 97 uses clip 196, source length 1.5181859731674194 seconds.
Both effect gains are 0.6000000238418579.
I use exported constants without alternate values.

The accepted seat records give driverSeat1=1 for level1/69296 and level1/66466.
The latter has driverSeat0=0. The other three primary seats have both flags zero.
The installed test compares driving2 with the accepted record for each exported seat.

## RED and GREEN

RED: node --test tests/web/audio.test.mjs.
The two cadence tests failed with bank.starter is not a function.
Log: task-3-web-red.log.

RED: ./scripts/check.ps1 -Test audio.
The native build failed because VehicleAudioInput, AudioState::vehicle, and starter_enabled did not exist.
Log: task-3-native-red.log.

GREEN:

| Command | Result | Log |
| --- | --- | --- |
| node --test tests/web/audio.test.mjs tests/web/input.test.mjs tests/web/menu.test.mjs | 31 passed, zero skipped | task-3-web-final.log |
| cargo test --locked --test audio, pinned Docker toolchain | 14 passed; one installed test requires explicit execution | task-3-audio-final.log |
| cargo test --locked --test input, pinned Docker toolchain | 12 passed; five installed tests require explicit execution | task-3-input-final.log |
| ./scripts/check.ps1 -Test engine | 7 passed | task-3-engine-green.log |
| cargo test --locked --test input -- --ignored, with read-only records mount | 5 passed, including the source seat mapping | task-3-source-final.log |
| cargo test --locked --test audio -- --ignored, with read-only records mount | 1 passed | task-3-source-final.log |
| ./scripts/check.ps1 | 248 native tests passed; 44 existing source tests require explicit execution; Node and WASM passed | task-3-check-final.log |
| ./scripts/build.ps1 | release WASM and wasm-bindgen exit 0 | task-3-build-final.log |
| cargo fmt --all -- --check | exit 0 after final test edits | task-3-input-final.log |
| git diff --check | exit 0 | direct command |

The full check ran once. I then strengthened the ignition edge and driving2 tests and reran their focused targets.
The tests cover source formulas at idle, crossover, maximum RPM, and overspeed.
They cover horn gain and ambient pitch, strict starter deadlines, release during decode, retained loops, engine removal, and stop edges.
The 600-frame retention test requires exactly two engine starts and one horn start.
The ignition edge test performs two gameplay ignition changes before draining two events once.
The driving2 test verifies horn and ignition while forward input cannot trigger throttle ignition.

Expected failure diagnostics remain in the inherited Node failure tests.
Git reports existing LF-to-CRLF notices. Neither represents a failed assertion.

## Runtime regeneration

Exact regeneration command:

PYTHONPATH=<worktree>; tools/reference/.venv/Scripts/python.exe .superpowers/sdd/2026-10-12-source-audio/task-3-export.py

The scratch command uses derive_reference from the existing export pipeline.
I used the Task 1 accepted library manifest and category indexes.
LibraryReader checks category receipts, selected records, asset hashes, and source provenance.
derive_reference verifies selected output closure and the native runtime contract before promotion.
The command exited 0. Log: task-3-export.log.
The generated manifest includes the required driving2 fields.

I checked the accepted Task 1 library fingerprint and both provenance fingerprints.
The library SHA256 remains 92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99.
The check exited 0. Log: task-3-library-fingerprint.log.
I added the same assertions to the scratch regeneration command for reuse.
I left the source, records, Python environment, and accepted library junctions unchanged.
Task 6 still owns normal runtime --verify for the complete library.

## Browser evidence and gaps

I ran one bounded Linux Chromium pass with the Task 2 harness and sandbox profile.
Harness: task-3-browser.cjs. Log: task-3-browser.log.
Docker used pwuser, network none, init, shm-size=2g, and the existing official seccomp profile.
I mounted the checkout and dependency bundle read-only. Only this task evidence folder allowed writes.
The harness passed trusted Start, pointer lock, and simulation progress.
Three aim attempts selected Car door. One door drag and three corrections still selected the door.
The harness exited 1 with Driver seat was not reached by trusted inputs.
I kept its static source seat aim point. I stopped after this bounded route.
The harness closed Chromium and its server. Docker removed the owned container.

Task 6 must check the assembled idle-to-throttle transition, starter release, horn release, loop boundaries, distance, pause, and restart.
Task 6 must reach a supported seat through controls and capture source PCM output.
Device output or a user listening check remains open.
I claim no audible vehicle output from analyser values, voice counts, or automated assertions.

## Files and self-review

Code: src/audio.rs, src/app.rs, src/browser.rs, src/starting_area.rs, src/simulation.rs, web/audio.js, web/audio/voices.js, web/input.js.
Seat correction: src/reference.rs, src/player.rs, tools/reference/manifest.py, tests/common/reference.json.
Tests: tests/audio.rs, tests/input.rs, tests/web/audio.test.mjs, tests/web/input.test.mjs.

I checked the source formulas, strict deadlines, active tail ownership, reset, and engine removal.
I narrowed moving effect anchors to starter and stop clips. Other shared effects retain their existing behavior.
I restored fixture formatting after a JSON rewrite produced unrelated changes.
The initial installed seat check could not see the Windows junction in Docker.
I added a read-only mount of its resolved records target and reran the original assertion.
I added no package or Bevy audio feature. I added no radio or interaction effect.
Other prior driving2 controls remain outside this audio task.


## Review fix round 1

Fix base: 4bfa5d4.
Commit subject: Reset reused vehicle effect playback offsets.
Status: DONE_WITH_CONCERNS. Task 6 retains the assembled and device acceptance gaps.

I fixed the Important effect-slot finding in task-3-review.md.
VoiceBank.event now calls stop(slot, false) before each fresh assignment.
This resets the effect offset and pending callback token, including after natural completion.
I kept retained loop pause and resume behavior unchanged.

I added fresh_effects_start_at_zero_after_natural_completion.
The test advances AudioContext time and updates the active effect parameters.
It completes each source through onended, then reuses the same slot.
It checks two starter shots and one engine stop effect start at offset zero.

RED command:
node --test --test-name-pattern=fresh_effects_start_at_zero_after_natural_completion tests/web/audio.test.mjs
The assertion failed: actual [0, 0.5, 1], expected [0, 0, 0].
Log: task-3-fix-round-1-red.log.

GREEN command:
node --test tests/web/audio.test.mjs tests/web/input.test.mjs tests/web/menu.test.mjs
Result: 32 passed, zero failed, zero skipped; exit 0.
Log: task-3-fix-round-1-green.log.
The existing loop pause/resume offset test also passed.
git diff --check exited 0.

I reviewed the diff. I changed only web/audio/voices.js, tests/web/audio.test.mjs, and this report.
Rust and the bridge remain unchanged. I did not repeat the WASM build or browser route.
The inherited warning-output Minor and Task 6 acceptance gaps remain open.
