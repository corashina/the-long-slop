# Task 4 report: source interaction audio

Status: DONE_WITH_CONCERNS.
Base: c272561. Branch: codex/audio-runtime.
Worktree: C:/Users/Corashina/.codex/worktrees/audio/the-long-drive-wasm.
Commit subject: Add source interaction audio and emitter cleanup.

## Implementation and interfaces

I added AudioState::action, mechanism, remove_owner, and owner_alive.
I resolve craft bindings through each item pickup source ID.
I cache mechanism, cap, lamp, and emitter bindings during bind_game.
I update emitter positions through the existing anchor map.
I retain movement signals, lamp hum signals, and per-cap flow state.

InteractionState::step still returns Option<InteractionAction>.
InteractionState keeps completed source details in a fixed 64-slot queue.
The runtime drains this queue after each fixed step.
I queue deferred wheel actions after install, remove, join, or split succeeds.
Tire slot actions use the tire binding.
Slot removal keeps the item owner alive.
AudioState::remove_owner cancels owned signals and queued events.
The existing alive bridge rejects future publication of removed owners.
No current gameplay action deletes an ItemStore record; explicit owner cleanup remains available for deletion callers.

I preserve cap USE and TUNE details before later actions replace the returned action.
TUNE uses its prior valve and requested delta before clamp.
I record actual transfer and spill results per opening.
AudioState::action(Fill) adds no duplicate. fixed_step uses the selected cap details.
A transfer enables the selected destination cap only.
I preserve the three-update fill latch and the empty-pour stop rule.

The voice bridge now requires voice(id, playing, gain, pitch, looping).
AudioSignal.looping begins with the authored loop flag.
A successful PlayFillSound sets the flag false.
Later pouring preserves that flag.
I update source.loop without rebuilding the retained routing nodes.
I changed current callers and the current plan interface. I added no old-signature fallback.

Crafted effects follow their moving item.
Shared PlayClipAtPoint effects clamp gain above one and scale both distances by gain minus one.
They stop at the unpitched source clip length.
Direct PlayOneShot bindings keep their source voice settings and natural end.
Non-loop retained signals restart at offset zero after natural completion.
Pause uses the current loop flag. It retains loop offsets and clears non-loop starts.

## Focused source evidence

Accepted build: 11140890.
Managed source: .cache/reference-library/records/managed/Assembly-CSharp.cs.
Managed SHA256: 22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee.
I read focused methods and current typed records. I left borrowed inputs unchanged.

- partslotscript.Craft, 47814-47847, calls pickupable.Crafted after mounting.
- partslotscript.UnCraft, 47881-47953, calls Crafted after successful slot removal.
- pickupable.Crafted, 48523-48559, uses gain 0.25 and pitch [0.85, 1.15]. Its effect becomes an item child.
- tube script Detach, 65485-65499, has no Crafted call. Plain pickup, drop, and handbrake remain silent.
- usablescript.SlideMoveSound, 66346-66375, uses actual slide difference, multiplier 2, smoothing 3, and pitch [0.95, 1.05].
- usablescript.Update, 66324-66335, fades gain with deltaTime * 5 after 0.2 seconds without a slide update.
- usablescript.RotMoveSound, 66747-66784, uses rotation difference and multiplier 0.1. X movement starts without a chance gate.
- Y and slide movement use the exported squeak threshold, normally 0.75.
- RotSound and SlideSound use edge comparisons. Stationary values emit no endpoint event.
- StartEndStuff, 66823-66828, uses abs(requested_delta * audioM) / deltaTime * 0.01.
- usablescript.Turn, 66431-66440, chooses an authored turn clip and random pitch.
- tankcapscript.USE, 63211-63224, uses authored open and close clips and gains.
- tankcapscript.TUNE, 63234-63244, emits only when the prior valve is inside (0, 1).
- PlayFillSound, 63334-63347, sets loop false and gain to maxVolume * requested_gain.
- tankcapscript.Update, 63393-63478, retains filling for three updates and uses exported pour-speed bounds.
- Direct pouring retains the existing loop flag. Empty qualified pouring stops the source.
- mainscript.Fill, 18914-18918, calls Filling and PlayFillSound on the destination cap.
- fpscontroller, 17842-17858, finds a parent flashlightscript and calls TURN for eligible use or primary input.
- flashlightscript.TURN, 12744-12753, calls TurnOn or TurnOff with bsound true.
- TurnOn and TurnOff, 12756-12774, play authored clips. Refresh starts or stops selected lamp hum sources.
- The five selected lamp sound controls are House controls. Their aim colliders use the existing light control map.
- Restoration calls at 5629/5633 use bsound false. Headlight fades and mount updates remain silent.
- carscript.InteractSwitchLights, 8721-8734, uses SSounds.PlayOneShot at gain 1.
- mainscript.PlayClipAtPoint, 30247-30270, clamps gain above one, scales distances, and destroys at source clip length.

The controller approved source corrections for slot UnCraft, per-opening fill ownership, and the persistent loop bridge.
I corrected the research note's ambiguous detach sentence.
I changed no authored clips, typed audio records, exporter metadata, engine formula, horn formula, or radio playback.

## RED and GREEN

Initial native RED: ./scripts/check.ps1 -Test audio.
The compiler reported missing AudioState::action and remove_owner.
Log: task-4-native-red.log.

Initial browser RED: node --test tests/web/audio.test.mjs.
The crafted anchor was zero instead of emitter 57. Shared gain was 3 instead of 1.
Log: task-4-web-red.log.

Loop RED: node --test --test-name-pattern=cap_fill_loop_change tests/web/audio.test.mjs.
The retained source kept loop true after the requested fill transition.
Log: task-4-loop-red.log.

Offset RED: node --test --test-name-pattern=non_loop_fluid_completion tests/web/audio.test.mjs.
Fresh fluid starts had offsets [0, 0.5], rather than [0, 0].
Log: task-4-loop-offset-red.log.

Reset RED: cargo test --locked --test audio movement_reset_starts_at_source_zero_gain.
Reset restored movement gain 1. Source Start requires zero.
Log: task-4-reset-red.log.

Final focused native command used the existing Docker toolchain:

```powershell
. ./scripts/cargo-target.ps1
$taskTarget=Get-CargoTargetDirectory (Get-Location).Path
docker run --rm --mount "type=bind,source=$((Get-Location).Path),target=/workspace" --mount "type=bind,source=C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/.cache/reference-library/records,target=/workspace/.cache/reference-library/records,readonly" --mount type=volume,source=tld-wasm-cargo,target=/cargo --mount type=volume,source=tld-wasm-target,target=/target --env "CARGO_TARGET_DIR=$taskTarget" my-summer-slop-toolchain sh -c 'cargo fmt --all && cargo test --locked --test audio --test interaction --test mechanisms --test parts --test wheels --test fluids -- --include-ignored'
```

Result: 111 passed, zero failed, zero ignored.
Targets: audio 28, fluids 3, interaction 48, mechanisms 3, parts 20, wheels 9.
Log: task-4-source-final.log.
The records mount resolves the borrowed junction and permits reads only.

Node: node --test tests/web/audio.test.mjs tests/web/input.test.mjs tests/web/menu.test.mjs.
Result: 38 passed, zero failed, zero skipped.
Log: task-4-web-final.log.

I ran ./scripts/check.ps1 once before the final loop and reset corrections.
Its native portion passed 258 tests. It listed 45 installed scenarios for separate execution.
Its Node, formatting, and WASM portions passed. Log: task-4-check-final.log.
I then reran the changed native targets, including their installed scenarios, and the browser tests.

Final build: ./scripts/build.ps1 exited 0.
Release WASM and wasm-bindgen completed after the final reset correction.
Log: task-4-build-final.log.
Final cargo fmt --all -- --check and git diff --check exited 0.
Log: task-4-fmt-final.log and the direct Git command.

## Bounded browser check

I ran one Linux Chromium pass with the existing harness and sandbox profile.
Harness: task-4-browser.cjs. Log: task-4-browser.log.
Docker used pwuser, network none, init, shm-size=2g, and playwright-seccomp.json.
The checkout, dependency bundle, and accepted library mount allowed reads only.
The task evidence folder allowed writes.
The harness closed Chromium and its server. Docker removed its container.
Command:

```powershell
$taskRoot=(Get-Location).Path
$taskEvidence=Join-Path $taskRoot '.superpowers/sdd/2026-10-12-source-audio'
docker run --rm --user pwuser --init --network none --shm-size=2g --security-opt "seccomp=$taskEvidence/playwright-seccomp.json" --mount "type=bind,source=$taskRoot,target=/workspace,readonly" --mount "type=bind,source=C:/Users/Corashina/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules,target=/deps/node_modules,readonly" --mount "type=bind,source=C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/web/assets/library,target=/workspace/web/assets/library,readonly" --mount "type=bind,source=$taskEvidence,target=/evidence" mcr.microsoft.com/playwright:v1.62.1-noble node /workspace/.superpowers/sdd/2026-10-12-source-audio/task-4-browser.cjs
```

Result: exit 0. Trusted Start, pointer lock, and simulation passed.
I selected Car door through the source aim route.
I held E and applied one negative horizontal drag of 220 pixels.
The before and after captures show a changed door pose.
The HUD still selected Car door. I stopped the route there.
Captures: task-4-browser-before-door.png and task-4-browser-seat-aim.png.
The bridge queued startPosStart binding 57 at emitter 80, with source gain 9.233981132507324.
Pitch was 0.9449225664138794, inside the exported range.
The browser started clip 27, sharedassets1.assets:4332, at offset zero.
Its decoded duration was 0.6673242630385487 seconds.
Evidence: task-4-browser-audio.json.
Peak, RMS, and nonzero sample count remained zero at the capture.
I claim no measured PCM output or device output from this pass.
I injected no gameplay state and made no further route attempt.
The reversed drag calculation now has an observed door-pose result.
Task 6 still owns playback timing, PCM comparison, and source controls for the remaining categories.

## Self-review and scope

I checked final success hooks, per-opening ownership, source silence, moving anchors, and removal cancellation.
I fixed the non-loop offset and restart movement-gain defects during self-review.
I checked pause against the current loop flag and tested the preserved loop offset path.
I checked retained source nodes during loop changes.
I verified accepted lamp hum anchors 11->state 0, 12->state 0, and 13->state 1.
I added no package and no frame allocation.
I left the source, library, records, and Python environment unchanged.
I did not repeat the unchanged whole-library scan.

src/app.rs adds the owner-alive gate and the mandatory loop argument.
src/browser.rs and web/audio.js carry the same argument through the existing scalar bridge.
These files were required outside the brief's initial list. The controller approved both changes.
The controller also requested the current plan signature correction.
The audio module was already large. I extended its current structure without moving unrelated code.

Files: src/audio.rs, src/starting_area.rs, src/interaction.rs, src/interaction/mechanisms.rs,
src/app.rs, src/browser.rs, web/audio.js, web/audio/voices.js, tests/audio.rs, tests/web/audio.test.mjs,
docs/research/2026-10-12-audio-reference.md, docs/superpowers/plans/2026-10-12-source-audio.md, and this report.

## Acceptance gaps

Task 6 must check each interaction category in the assembled browser and compare its output with source clips.
Device output or a user listening check remains open.
I claim no audible device output from counters or automated tests.
Movement playback still requires matched timing and random-distribution acceptance in the assembled runtime.
The runtime samples source pitch and chance ranges; it does not reproduce Unity's random sequence.
Inherited failure tests print expected audio diagnostics. Git reports existing LF-to-CRLF notices.


## Review fix round 1

Fix base: 63d1253.
Commit subject: Reset stopped movement audio and test runtime attachment.
Status: DONE_WITH_CONCERNS. Task 6 retains the existing acceptance gaps.

I fixed both Important findings from task-4-review.md.
A stationary movement update now clears the retained playback timer.
Resume now samples a fresh pitch and the source chance gate.
I traced SlideMoveSound at 66359-66374 and RotMoveSound at 66759-66783 again.
Both methods test isPlaying before starting. Both stop a stationary source.

I added three move, stop, and resume tests through MechanismState and AudioState::fixed_step.
The X test requires a fresh pitch inside [0.95, 1.05].
The slide and Y tests use chance thresholds zero and one.
They reject a restart when its threshold rejects the source sample.
They permit a later restart when its threshold permits the source sample.

I replaced the disconnected attachment test.
The fixture now aims the runtime player at a real wheel slot sensor.
GarageRuntime::step removes the wheel through a grab gesture.
I drain that runtime's removal event before testing installation.
I then release acquisition and perform a distinct mount gesture.
Rejected installation retains the held rim and leaves that runtime's audio queue empty.
Successful installation reaches WheelSlot and queues one crafted event at gain 0.25.
The test uses the completed-action hook and the runtime audio drain.
It injects no InteractionAction or AudioEvent.

RED command, existing pinned Docker toolchain:

```text
cargo fmt --all && cargo test --locked --test audio
```

Result: 26 passed, three failed, two installed tests require explicit execution.
X reused pitch 0.9816408 after stop and resume.
Slide and Y resumed despite a chance threshold of one.
The new attachment integration passed against the existing correct success hook.
Log: task-4-fix-round-1-red.log.

GREEN command used the same toolchain and the resolved records mount:

```text
cargo fmt --all && cargo test --locked --test audio -- --include-ignored && cargo fmt --all -- --check
```

Result: 31 passed, zero failed, zero ignored. Formatting exited 0.
Log: task-4-fix-round-1-green.log.
I kept the read-only records mount from the original Task 4 command.
I changed no attachment success path, so I did not repeat unrelated gameplay targets.
I changed no browser code. I left the warning Minor and browser route unchanged.

I reviewed the diff and checked the source stop and restart paths.
The code change clears one stale timer. The tests use current public runtime paths.
Files: src/audio.rs, tests/audio.rs, and this report.
Release build: ./scripts/build.ps1 exited 0 after the focused checks.
Release WASM and wasm-bindgen passed. Log: task-4-fix-round-1-build.log.
git diff --check exited 0.
