# Task 5 report: source Lada radio

Status: DONE_WITH_CONCERNS.
Base: 5aff264. Branch: codex/audio-runtime.
Worktree: C:/Users/Corashina/.codex/worktrees/audio/the-long-drive-wasm.
Commit subject: Add source Lada radio controls and recordings.
This implementation commit includes this report.

## Implementation and current interfaces

I added source radio controls, station schedules, and browser recording playback.
Rust owns RadioState and RadioFrame. Web Audio owns decoding and playback.

RadioState::new(rules: &RadioAudioReference, clips: &[AudioClipReference], calendar: [u8; 2]) initializes retained station data.
The controller approved this direct constructor. I added no adapter type.
RadioState::control accepts VolumeUp, VolumeDown, TuneUp, TuneDown, and Band.
RadioState::advance(game_seconds: f64, electronics_output: f32) returns RadioFrame.
RadioState::calendar(month: u8, day: u8) updates the local calendar.
RadioFrame contains effective power, AM/FM, frequency, volume, static gain, primary playback, and secondary playback.
RadioPlayback contains station ID, clip ID, offset seconds, gain, and next clip ID.
Missing custom recordings retain absent clip IDs.

AudioState now retains radio and radio_frame. I removed the unused radio_constants map.
AudioState::radio_control applies successful source control actions and queues source power/band clicks.
AudioState::fixed_step advances broadcasts from its scaled game clock.
The runtime already freezes fixed steps during pause and focus loss.
The browser mutes playback during those states and resumes at the broadcast offset.
Radio power-off leaves station schedules running.

RadioControlReference contains source, node, ItemId, enabled, tune, and turnable.
I removed the obsolete radio mechanism pointer field.
RadioAudioReference requires its own electronics source/output, tuning repeat interval, and holiday ranges.
I updated the exporter, typed validation, and current fixture.
Malformed audio still stays outside gameplay validation.

MechanismState binds the real source radio controls without synthetic MechanismReference records.
InteractionState retains the existing aim, reach, and LMB eligibility rules.
I filter LMB from mechanism input when its source guard blocks use.
Feedback contexts 11 and 12 identify volume and tuning.
The browser retains source readouts even when AudioContext initialization or playback fails.
Unavailable readouts remain empty.
I used the stop-slop skill for UI text and documentation.

The scalar gameAudioRadio bridge accepts thirteen retained frame fields:
power, AM, frequency, volume, static gain,
primary clip/offset/gain/next, and secondary clip/offset/gain/next.
The browser retains its frame object, two playback objects, and current/next arrays.
The five-argument voice signature, including looping, remains mandatory.
I skip generic voice publication for the three radio voices.
VoiceBank marks recording slots and excludes them from effect load/resume.
I reuse their routing nodes and the source static voice.

ClipCache.radio(current: [number|null, number|null], next: [number|null, number|null], session: number) manages recordings.
It keeps effect buffers separate.
It shares duplicate recordings and holds at most four unique recording buffers, including pending decodes.
Canceled pending decodes keep their slots until completion.
When all slots remain occupied, it defers new demand.
Old sessions cannot replace current demand.
Restart, removal, and disposal reject stale playback.
Engine removal keeps the radio owner alive.
Late recordings start at the current broadcast offset.
I preserve the source correction threshold of 0.5 seconds and seek margin of 0.1 seconds.

I preserve music/talk transitions and random selection without replacement.
I reserve upcoming clips from existing source decks.
I defer a new deck until its source reset, so its holiday filter uses the current local day.
I refresh the browser calendar at local day changes without Date allocation each frame.
The runtime random sequence does not reproduce Unity's sequence.

## Focused source evidence

Accepted Steam build: 11140890. Unity: 2021.3.18f1.
Managed source: .cache/reference-library/records/managed/Assembly-CSharp.cs.
Managed SHA256: 22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee.

- Source volume pointer: level1:77810, node 922, ItemId 94.
- Source tuning pointer: level1:70967, node 923, ItemId 95.
- Source band pointer: level1:78052, node 931, ItemId 96.
- The first two controls have tuneAble=1. The band control has turnable=1.
- Each selected control has m_Enabled=1 and disable=0.
- fpscontroller.Update, 17770-17809, uses scroll direction or held Use/LMB with LOOKY.
- Its held step interval is Lerp(0.2, 0, abs(LOOKY)). Zero LOOKY produces no step.
- radioscript.UpdValues, 49909-49930, rounds authored frequency and formats source readouts.
- Volume uses two digits after multiplying by ten. FM uses one decimal. AM uses three digits.
- radioscript.Update, 49932-49999, clamps vSpeed steps and applies the selected band control.
- vSpeed and FMStep are 0.10000000149011612. AM uses step 5.
- FM range is [88, 108]. AM range is [540, 1600].
- The radio uses electronics level1:68491, separate from Rear electronics level1:68017.
- electronicsscript.Start/UpdOutPut, 11456-11485, computes output 1 from maxOutput 1 and jamValue 0.
- I reuse the lighting export's zero-jam normalization rule. Nonzero startup jam requires another source audit.
- SetRadio, 50023-50118, uses cutoff 0.2 and tolerances 0.4000000059604645 for FM and 20 for AM.
- It retains the prior best station as secondary. It does not choose generic nearest-two stations.
- Primary playback applies currentVolume. Secondary playback omits that playlist gain.
- Custom station gain uses 0.5.
- TurnRadio, 50001-50021, and Update at 49973 use SS.transform.position for source clicks.
- I keep logical radio owner 102 and use SS emitter 36/node 918 for the click position.
- I preserve shared PlayClipAtPoint settings, random pitch, and lifetime.
- radioCH selection/reset/advance, 27947-28052, draws without replacement and switches music/talk lists.
- mainscript.RADIO, 30303-30344, uses scaled Time.time. Multiplayer RADIO2 alone uses unscaled time.
- holidayDates, 28525-28528, and CheckHoliday, 29348-29380, define inclusive calendar ranges.
- The ranges are Oct 15 through Nov 15 and Dec 15 through Jan 15.

## RED and GREEN evidence

Native RED: ./scripts/check.ps1 -Test radio.
The compiler rejected the missing simulation::radio module before implementation.
Log: task-5-native-red.log.

Browser baseline RED: node --test --test-name-pattern=radio task-5-baseline/tests/web/audio.test.mjs.
I replayed four new tests against base 5aff264 in an ignored scratch tree.
All four failed: no recording decode, no ClipCache.radio, and no retained playback frame fields.
Log: task-5-web-baseline-red.log.
The first scratch run lacked ui-layout.js. I supplied that unchanged base module before the recorded regression run.

An intermediate radio test compared 88.200005 with 88.2 using exact equality.
I changed that assertion to a 0.00001 tolerance for source f32 rounding.
I preserved ties-to-even rounding and the authored step.

An intermediate installed SS anchor test failed at its unchanged 0.0001 per-axis assertion.
Its AudioState lacked bind_game/fixed_step, so retained positions remained zero.
I moved that setup into the installed fixture.
The assertion compares click.position with the source world position of SS node 918.
The click still owns emitter 102. I did not relax the assertion.
GREEN: cargo test --locked --test radio -- --include-ignored.
Result: 11 passed, zero failed, zero ignored. Log: task-5-anchor-final.log.

Final focused native command used the pinned Docker toolchain and a read-only records mount:

```powershell
. ./scripts/cargo-target.ps1
$taskTarget=Get-CargoTargetDirectory (Get-Location).Path
docker run --rm --mount "type=bind,source=$((Get-Location).Path),target=/workspace" --mount "type=bind,source=C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/.cache/reference-library/records,target=/workspace/.cache/reference-library/records,readonly" --mount type=volume,source=tld-wasm-cargo,target=/cargo --mount type=volume,source=tld-wasm-target,target=/target --env "CARGO_TARGET_DIR=$taskTarget" my-summer-slop-toolchain sh -c 'cargo fmt --all && cargo test --locked --test radio --test audio --test mechanisms --test feedback --test interaction -- --include-ignored && cargo fmt --all -- --check'
```

Result: 100 passed, zero failed, zero ignored.
Targets: audio 31, feedback 7, interaction 48, mechanisms 3, radio 11.
Formatting passed. Log: task-5-native-final.log.
The separate ./scripts/check.ps1 -Test radio command passed ten tests and listed the installed test for explicit execution.
Log: task-5-radio-command-final.log. The final command above includes that installed test.

Browser command: node --test tests/web/audio.test.mjs tests/web/feedback.test.mjs tests/web/input.test.mjs tests/web/menu.test.mjs.
Result: 51 passed, zero failed, zero skipped. Log: task-5-web-final.log.
Python command: tools/reference/.venv/Scripts/python.exe -m unittest tools.reference.tests.test_audio -v.
Result: seven passed. Log: task-5-python-final.log.
The recovery test checks required source control fields, electronics, interval, and holiday ranges.
Native tests reject missing links and invalid radio metadata.

I ran ./scripts/check.ps1 once before the final SS anchor and LMB guard corrections.
It passed 273 native tests, Node tests, formatting, and the release WASM check.
It listed 46 installed scenarios for separate execution. Log: task-5-check-final.log.
I reran all affected focused targets, including installed scenarios, after those corrections.
Final build: ./scripts/build.ps1 exited 0 after the source and LMB corrections.
Release WASM and wasm-bindgen passed. Log: task-5-build-final.log.
git diff --check passed.
Expected audio failure tests print diagnostics. The inherited audio test has one unused-mut warning.

## Runtime regeneration

I used the existing fingerprint-checked Task 3 derivation path:

```powershell
$env:PYTHONPATH=(Get-Location).Path
& tools/reference/.venv/Scripts/python.exe '.superpowers/sdd/2026-10-12-source-audio/task-3-export.py'
```

It asserts the accepted Task 1 library and provenance fingerprints, then calls derive_reference.
The library manifest fingerprint remains 92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99.
Selected runtime file closure and native typed audio validation passed.
Log: task-5-export.log. I did not repeat the unchanged whole-library verification.
Runtime assets remain ignored. I left borrowed source, records, library, and Python environment unchanged.

## Bounded browser check

I reused the Task 4 route with three aim steps and one negative door drag.
Harness: task-5-browser.cjs. Log: task-5-browser.log.
Docker used pwuser, network none, init, shm-size=2g, and the existing seccomp profile.
It mounted the checkout, dependencies, and accepted library read-only.
Only the task evidence folder permitted writes.
The harness closed Chromium and its server. Docker removed the container.

Result: exit 0. Trusted Start, pointer lock, and simulation passed.
The HUD selected Car door. Negative drag changed the door pose.
The source radio bridge published FM 94.5, volume zero, power false, and offset 2.0833334419876337 seconds.
This shows publication of the running broadcast clock while radio power is off.
It does not establish radio playback.
I made no seat search and injected no gameplay state.
The door effect started clip 27 at offset zero.
Its analyser counters remained zero. The inherited observer does not establish post-start capture timing.
I claim no measured PCM output or device output from this pass.
Captures: task-5-browser-before-door.png and task-5-browser-seat-aim.png.
Evidence: task-5-browser-audio.json and task-5-browser-state.json.
The browser reported unrelated 404 resource diagnostics and no page exception.

## Self-review, files, and remaining acceptance

I checked source control ownership, source station order, source gains, clock freezing, and missing custom content.
I checked cache bounds during decoding, duplicate demand, old sessions, removal, pause, and restart.
I corrected the source SS click anchor and the LMB guard during self-review.
I checked the final diff, source receipt fields, retained frame objects, and retained routing.
I added no runtime package, Bevy audio feature, scanner, import control, keyboard shortcut, station, or recording.

Files: src/simulation/radio.rs, tests/radio.rs, src/simulation.rs, src/starting_area.rs, src/audio.rs,
src/interaction.rs, src/interaction/mechanisms.rs, src/interaction/feedback.rs, src/interaction/targets.rs,
src/browser.rs, src/app.rs, src/reference/audio.rs, tools/reference/audio.py,
tools/reference/tests/test_audio.py, tests/common/reference.json, tests/web/audio.test.mjs,
tests/web/feedback.test.mjs, web/audio.js, web/audio/clips.js, web/audio/voices.js,
web/feedback.js, web/main.js, the source audio research document, current plan interface, and this report.
The controller approved the source metadata, direct constructor, voice slot flag, and extra feedback files.
The existing audio and interaction modules remain large. I kept their current structure.

Task 6 must establish a trusted route to the radio controls. Seat access remains unresolved.
It must check both bands, available recordings, tuning, source static, power cycling, and electronics gating.
It must check pause, focus loss, repeated tuning, restart, and missing custom AM 1080 in the assembled browser.
It must compare timed PCM output with source recordings and establish device output or user listening.
The runtime tests do not establish Unity random sequence parity or audible source parity.
Custom AM 1080 remains unavailable. Exact tuning keeps source static gain zero; detuning permits source static.
