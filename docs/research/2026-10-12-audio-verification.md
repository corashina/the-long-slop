# Source audio verification: 2026-10-12

The browser produced nonzero audio from the assembled game.
Device listening and full assembled acceptance remain unverified.
The user requested "skip tests" during the listening check. I stopped further checks and recorded the gaps below.
This request waives further checks for this task. It does not turn an incomplete check into a pass.

## Build and source identity

- Worktree: `C:/Users/Corashina/.codex/worktrees/audio/the-long-drive-wasm`.
- Branch: `codex/audio-runtime`. Task 6 base: `cb1b854`.
- Accepted source: Steam build `11140890`, Unity `2021.3.18f1`.
- Accepted library receipt: `92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99`.
- Runtime manifest SHA256: `e138f79dca6cab3c8edd363915b901e222a812530a1e1e1b38693efd83535229`.
- WASM SHA256: `57286d0ed3d149236a82ecbba809d296c74961a5887a6409eea057a97f1daa4d`.

## Collected checks

| Check | Recorded result |
| --- | --- |
| Python source audio suite | 7 passed |
| Browser audio suite after cache correction | 29 passed; zero skipped |
| Full `scripts/check.ps1` before Task 6 corrections | 273 native tests and 55 browser/server tests passed; 46 installed cases excluded by default |
| `scripts/build.ps1` | Release WASM and bindgen passed |
| Explicit `scripts/check.ps1 -Test audio` after warning correction | 29 passed; 2 installed cases excluded by default |
| Explicit `scripts/check.ps1 -Test radio` | 10 passed; 1 installed case excluded by default |
| Corrected native run with `--include-ignored` | 226 tests passed in completed targets; zero ignored there; stopped during reference target |
| Audio and radio within that native run | All 31 audio and all 11 radio cases passed, including installed cases |
| Normal runtime export `--stage runtime --verify` | Stopped after progress reached 260,000 of 262,784 records; no final verification receipt |
| Final whole-branch review | Controller-owned; pending at this report |

The first all-native run omitted the library bind mount needed to resolve a Windows junction inside Linux.
Its `production_export_matches_installed_contract` case failed when `sha256sum` could not open a required file.
I added the accepted library as a read-only mount. The user stopped the rerun before that target finished.
I did not change its assertions. I cannot claim a complete installed-source pass.

I corrected the reviewed radio cache churn. Unchanged demand now keeps its retained set.
Load completion permits deferred demand to use newly free slots. Pending cancellations still consume the four-slot budget.
The new regression failed with 61 set clears before the correction. It passed after the correction.
I removed one unused `mut` from the existing fluid audio test.
Expected failure-case diagnostics remain in test logs.

## Assembled capture

I used Chromium `151.0.7922.34` from `mcr.microsoft.com/playwright:v1.62.1-noble`.
The container used `pwuser`, the existing Playwright seccomp profile, network isolation, and Chromium sandboxing.
I mounted the checkout, dependency bundle, and accepted library read-only. Only the evidence folder allowed writes.
The existing `scripts/serve.mjs` served the game on container loopback port `48138`.
A trusted Start click obtained pointer lock. The simulation advanced.

The browser has no host audio device. Earlier device preflight identified `Headphones` on Windows.
I did not record that endpoint or the microphone.
The controller offered the sample for user listening. The user then requested "skip tests".
Device listening remains unverified.

I aimed at the front-left car door and held E while dragging left by 220 pixels.
The door moved. Source binding `57`, emitter `80`, selected clip `27` from `sharedassets1.assets:4332`.
The captured event used gain `9.107617378234863` and pitch `0.9449225664138794`.
The source began at offset zero. Its decoded duration was `0.6673242630385487` seconds.

The earlier analyser observer missed the short playback interval.
The first timed probe logged only four post-start polls across approximately 27 seconds.
Those zero counters do not establish silence.
I connected a MediaStreamDestination capture to the actual master graph beside the output route.
MediaRecorder collected game audio across playback. Chromium decoded that recording for measurement and WAV export.
This capture includes the game's mixing and spatial routing. It is not a source WAV preview or device recording.
MediaRecorder encoding can change sample values. The capture does not establish bit-exact parity.

First capture: stereo, 44,100 Hz, peak `0.7154335379600525`.
The full 56.52-second capture had RMS `0.007547238214443748` and 34,073 frames in its nonzero span.
The trimmed WAV contains 42,892 frames, including short silence margins: `0.9726077097505669` seconds.
File: `.superpowers/sdd/2026-10-12-source-audio/task-6-door-assembled.wav`.
SHA256: `69f38ed960e3f4dd645c864eeb4c2cc0dc2ec597a25d01cb717bf1f35fe6ea6a`.
The second route capture remains separate as `task-6-route-door-assembled.wav`.

The source-supported F action detached the door and placed it in the player's hands.
Two further seat aims still selected the carried door. Seat entry remains unresolved.
I injected no gameplay state. The route finished before the stop command reached its container.

## Source loop measurements

These measurements use accepted source samples. They do not measure assembled loop playback.
The seam delta is the first PCM16 sample minus the last, divided by 32,768.

| Voice | Clip and source path ID | Duration, seconds | Seam delta | Authored distances, metres |
| --- | --- | --- | --- | --- |
| Engine idle, 43 | 12 / 4378 | 0.8174829931972789 | 0.00360107421875 | 1.5 to 1000 |
| Engine motor, 42 | 11 / 4228 | 1.017687074829932 | 0.10955810546875 | 1.5 to 1000 |
| Horn, 27 | 6 / 4068 | 0.4337641723356009 | -0.030426025390625 | 1 to 1000 |

All three clips come from `sharedassets1.assets`. Their voices use authored looping and logarithmic distance rolloff.
I recorded clip hashes and complete exported curves in `task-6-source-loops.json`.
No seam smoothing or replacement clip was added.

## Ten assembled scenarios

| Scenario | Observed result and remaining gap |
| --- | --- |
| 1. Ignition, starter, engine, RPM, stop | Not completed. Seat entry remains unresolved. Native formula and transition tests passed. |
| 2. Horn hold, release, temperature, single voice | Not completed in the browser. Native and browser unit cases passed. |
| 3. Doors, switches, attachment, fluids | Door movement and one source effect produced captured PCM. Door detachment succeeded. Closing, switches, attachment, and fluid playback remain unchecked. |
| 4. Radio stations, static, missing custom content | FM 94.5, volume zero, power off, and advancing broadcast offset were observed. Tuning and audible recordings remain unchecked. |
| 5. Repeated engine and horn loop boundaries | Source seams were measured. Assembled continuity and listening remain unchecked. |
| 6. Distance gain, stereo, origin rebase | Near-door output was captured. Authored distance comparisons, stereo positions, and a listening rebase remain unchecked. |
| 7. Pause, focus loss, Continue | No assembled playback cycle completed. Ten equivalent-state cycles and voice counts remain unchecked. |
| 8. Restart during playback, fetch, decode | Unit cancellation cases passed. Ten assembled restart cycles and pending-load cancellation remain unchecked. |
| 9. Remove emitters | Door detachment succeeded and changed carry state. Active or pending audio ownership removal remains unchecked in the browser. |
| 10. Audio validation, missing file, decode, context failure | Unit failure cases passed. Driving and restart under each assembled failure remain unchecked. |

No assembled evidence establishes zero stale voices or stable settled counts across ten cycles.
No result establishes device listening, complete audible source parity, or the native random sequence.
Custom AM 1080 has no embedded recordings. That source content limit remains.

## Evidence locations

All local logs and captures use `.superpowers/sdd/2026-10-12-source-audio/`.
The committed `task-6-report.md` records commands, failures, stopped jobs, changes, and self-review.
The browser harnesses are `task-6-browser.cjs`, `task-6-capture.cjs`, and `task-6-route.cjs`.
Their logs, PCM JSON, WAV files, and screenshots remain local and ignored.
