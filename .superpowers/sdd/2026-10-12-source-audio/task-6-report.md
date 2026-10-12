# Task 6 report: source audio verification

Status: DONE_WITH_CONCERNS after the user's instruction to skip further tests.
Base: cb1b854. Branch: codex/audio-runtime.
Worktree: C:/Users/Corashina/.codex/worktrees/audio/the-long-drive-wasm.
Commit subject: Verify source audio output and lifecycle.
This implementation commit includes this report.

## User instruction and remaining acceptance

The controller relayed the user's "skip tests" response during the door listening check.
I stopped new tests, builds, listening checks, and route scenarios.
Device listening is waived for this task and remains unverified.
I did not mark the ten assembled scenarios complete.
The scenario table in docs/research/2026-10-12-audio-verification.md lists each gap.
The controller owns whole-branch review. I dispatched no agent or reviewer.

## Changes

- Retain the four radio demand IDs in ClipCache. Skip unchanged demand until a load settles.
- Mark the cache dirty when a pending load finishes, so deferred demand can use its slot.
- Iterate pending entry values, avoiding per-frame Map entry arrays.
- Preserve the four-buffer budget, including canceled loads still decoding.
- Add one critical cache regression before the user requested the stop.
- Remove the unused mut at tests/audio.rs:733.
- Create the audio verification research report.
- Append the current checkpoint to docs/verification.md and update only the audio scope statement in docs/roadmap.md.

I used systematic-debugging for the observer timing and test harness failures.
I used verification-before-completion for all result claims and stop-slop for documentation.
No runtime dependency, audio schema, source clip, playback formula, or product interface changed.

## RED and GREEN

Command before the cache correction:

```powershell
node --test --test-name-pattern=radio_cache_retains tests/web/audio.test.mjs
```

Result: exit 1. The new test expected one demand-set clear; unchanged demand caused 61 clears.
Log: task-6-cache-red.log.

Command after the cache correction:

```powershell
node --test tests/web/audio.test.mjs
```

Result: exit 0; 29 passed, zero failed, zero skipped.
Log: task-6-web-final.log.
The regression also confirms that four canceled pending loads retain their slots.
After those loads settle, unchanged deferred demand starts the next four loads.

## Commands and collected output

All paths below are relative to the worktree unless stated otherwise.
Logs are in .superpowers/sdd/2026-10-12-source-audio.

```powershell
tools/reference/.venv/Scripts/python.exe -m unittest tools.reference.tests.test_audio -v
```

Exit 0: Ran 7 tests; OK. Log: task-6-python.log.

```powershell
node --test tests/web/audio.test.mjs
```

Initial exit 0: 28 passed. Log: task-6-web.log.
Final exit 0: 29 passed after the cache correction. Log: task-6-web-final.log.

```powershell
./scripts/check.ps1
./scripts/build.ps1
```

Both exited 0 before the Task 6 cache and warning corrections.
Full check: 273 native tests passed; 46 installed cases excluded by default.
Browser and server suites: 55 passed, zero skipped.
Formatting and release WASM passed. Build produced the wasm-bindgen package.
Logs: task-6-check.log and task-6-build.log.
The existing unused-mut warning appeared in the first check; the later focused command has no such warning.

```powershell
pwsh -NoProfile -File scripts/check.ps1 -Test audio
pwsh -NoProfile -File scripts/check.ps1 -Test radio
```

Both exited 0 after the warning correction.
Audio: 29 passed, 2 installed cases excluded by default.
Radio: 10 passed, 1 installed case excluded by default.
Logs: task-6-audio-command.log and task-6-radio-command.log.
An earlier Windows PowerShell invocation failed because SHA256.HashData requires the project's PowerShell 7 runtime.
I used the existing bundled pwsh. No project script changed.

First explicit installed run:

```powershell
. ./scripts/cargo-target.ps1
$taskTarget=Get-CargoTargetDirectory (Get-Location).Path
docker run --rm --mount "type=bind,source=$((Get-Location).Path),target=/workspace" --mount 'type=bind,source=C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/.cache/reference-library/records,target=/workspace/.cache/reference-library/records,readonly' --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' --mount 'type=volume,source=tld-wasm-target,target=/target' --env "CARGO_TARGET_DIR=$taskTarget" my-summer-slop-toolchain sh -c 'cargo test --locked -- --include-ignored'
```

Exit 101. Completed targets recorded 243 passed and one failure, zero ignored.
The reference target failed at tests/reference.rs:168 when sha256sum returned failure.
Its required web/assets/library path is a Windows junction that needs an explicit Linux bind mount.
The command above omitted that mount. Log: task-6-native-all.log.

Corrected command:

```powershell
. ./scripts/cargo-target.ps1
$taskTarget=Get-CargoTargetDirectory (Get-Location).Path
docker run --rm --mount "type=bind,source=$((Get-Location).Path),target=/workspace" --mount 'type=bind,source=C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/.cache/reference-library/records,target=/workspace/.cache/reference-library/records,readonly' --mount 'type=bind,source=C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/web/assets/library,target=/workspace/web/assets/library,readonly' --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' --mount 'type=volume,source=tld-wasm-target,target=/target' --env "CARGO_TARGET_DIR=$taskTarget" my-summer-slop-toolchain sh -c 'cargo fmt --all -- --check && cargo test --locked --no-fail-fast -- --include-ignored'
```

The user stopped this run during the reference target. Exit 137 followed the requested container stop.
Completed targets recorded 226 passed, zero failed, zero ignored.
Audio 31 and radio 11 passed, including their three installed cases.
Formatting passed before the tests. Log: task-6-native-final.log.
The mount correction has no complete reference-target result. I made no all-source pass claim.
A transient broad warning edit also removed four required mut declarations and caused compiler errors.
I restored those declarations before this final run. The final diff removes only the unused mut at line 733.

```powershell
tools/reference/.venv/Scripts/python.exe -m tools.reference.export --stage runtime --verify
```

This was the normal command, without the earlier export shortcut.
The user stopped it after the log reached "Verified records: 260,000/262,784".
The shell returned -1 after termination. No final runtime verification receipt exists for this run.
Log: task-6-runtime-verify.log.
I did not run further tests, builds, or git diff --check after the user's instruction.

## Browser, sample, and device evidence

Command template for each owned browser pass:

```powershell
$taskRoot=(Get-Location).Path
$taskEvidence=Join-Path $taskRoot '.superpowers/sdd/2026-10-12-source-audio'
docker run --rm --name tld-audio-task6-route --user pwuser --init --network none --shm-size=2g --security-opt "seccomp=$taskEvidence/playwright-seccomp.json" --mount "type=bind,source=$taskRoot,target=/workspace,readonly" --mount 'type=bind,source=C:/Users/Corashina/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules,target=/deps/node_modules,readonly' --mount 'type=bind,source=C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/web/assets/library,target=/workspace/web/assets/library,readonly' --mount "type=bind,source=$taskEvidence,target=/evidence" mcr.microsoft.com/playwright:v1.62.1-noble node /workspace/.superpowers/sdd/2026-10-12-source-audio/task-6-route.cjs
```

The first two passes used names tld-audio-task6-probe and tld-audio-task6-capture.
Their harnesses were task-6-browser.cjs and task-6-capture.cjs. Each used the same mounts and sandbox profile.
All three browser processes exited 0. Their harnesses closed Chromium and the owned development server.
Chromium version: 151.0.7922.34, logged by the route pass from the same installed image.
No host display, host output device, or user browser profile was mounted.
The source server used scripts/serve.mjs and container loopback port 48138.
The first viewport was 960 by 540. Capture and route passes used 480 by 270.

Trusted Start, pointer lock, and simulation progress passed.
A source door action emitted binding 57, emitter 80, clip 27, sharedassets1.assets:4332.
The first timed analyser check returned zero but sampled only four times across 27 seconds after playback began.
That observer could miss a 0.667-second clip. It did not prove silence.
The master-graph MediaRecorder capture returned nonzero stereo PCM.
The verification research report contains source event values, sample metrics, hashes, and the exact signal path.
This is assembled graph capture, not device recording, source preview, or bit-exact source comparison.

Primary sample:
C:/Users/Corashina/.codex/worktrees/audio/the-long-drive-wasm/.superpowers/sdd/2026-10-12-source-audio/task-6-door-assembled.wav

I restored that first capture from task-6-capture-pcm.json after the later route reused its output filename.
The second sample is task-6-route-door-assembled.wav. Both remain ignored.
The original sample hash is 69f38ed960e3f4dd645c864eeb4c2cc0dc2ec597a25d01cb717bf1f35fe6ea6a.

The F dismount action detached the door and made it the carried item.
Two subsequent seat aims still selected the carried door. Seat entry remains unresolved.
The final route screenshot is task-6-route-after-dismount.png.
The route log contains the actual carry state and Chromium version.
I used source controls and injected no gameplay state.

Earlier preflight found Windows Headphones. This isolated browser could not send sound to that endpoint.
The controller offered the primary sample for user listening, then received "skip tests".
No microphone, unrelated user audio, or system endpoint was recorded.
Device listening remains unverified, including the door sample.

## Owned cleanup at the user stop

I checked container command lines and mounts before stopping jobs.
I stopped native container 05a98a2face6, mounted from this audio worktree.
I rechecked process identities and stopped export Python PIDs 27220 and 36120.
Their parent chain and exact command matched this task's runtime verification.
The named route container finished before its stop request, so Docker reported no such container.
Final docker inventory contained only unrelated survival-weather and toolchain jobs, which I left untouched.
No verification job owned by this task remains running.

## Source measurements and remaining gaps

I measured the accepted idle, motor, and horn clip seams before the stop request.
Evidence: task-6-source-loops.json, including clip hashes, source IDs, and complete exported curves.
The research report lists sample durations and deltas. I added no smoothing.
These source measurements do not establish assembled loop continuity.

All ten required assembled scenarios retain some missing acceptance.
Only door movement, door detachment, initial radio publication, and a door graph sample have assembled evidence here.
Engine and horn playback, radio controls, distance/stereo/rebase, repeated cycles, pending removal, and failure-safe driving remain open.
Ten pause/resume and restart cycles were not performed. Stable settled counts and zero stale voices remain unproved in the assembled game.
Normal export verification and complete installed-source execution also remain incomplete after the stop.

## Self-review and commit

I read the cache diff and its regression after the green run.
The dirty flag changes demand work only; cancellation still holds pending budget slots.
Session checks, generation checks, effect cache ownership, and source records retain their existing rules.
I checked the warning diff and restored unrelated mut declarations.
I kept historical verification claims and unrelated roadmap exclusions.
The report distinguishes unit, installed-source, assembled graph, and device evidence.
I did not claim that a waveform proves listening.

Files staged explicitly:
web/audio/clips.js, tests/web/audio.test.mjs, tests/audio.rs,
docs/research/2026-10-12-audio-verification.md, docs/verification.md, docs/roadmap.md,
and this task-6-report.md.
Commit subject: Verify source audio output and lifecycle.
The controller receives the resulting SHA in the final status message.
