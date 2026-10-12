# Source-Based Game Audio Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add source-derived vehicle sounds, interaction sounds, and Lada radio playback with audible output and safe cleanup.

**Architecture:** Rust owns audio signals, successful-action events, and radio state. Browser Web Audio owns buffers, routing, and playback. Derive playback data from the accepted library and contain audio failures outside gameplay initialization.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, Rapier 0.36.0, wasm-bindgen 0.2.129, serde 1.0.229, serde_json 1.0.151, Web Audio, Python 3.12, existing exporter.

**Spec:** [Approved source audio spec](../specs/2026-10-12-source-audio-design.md).

## Global Constraints

- Use installed Steam build `11140890`, Unity `2021.3.18f1`, as the reference.
- Keep pinned Rust dependencies. Add no runtime package and no Bevy audio feature.
- Keep simulation state separate from playback ownership.
- Initialize AudioContext through Start or Continue, before awaiting pointer lock.
- Keep driving usable during audio fetch, decode, validation, or context failure.
- Reuse decoded buffers and retained voice routing. Do not recreate active loops each frame.
- Avoid per-frame allocations and repeated setup.
- Use source-supported clips, controls, triggers, gains, pitch, loops, distances, and mixer behavior.
- Do not invent stations, recordings, pickup sounds, or external custom content.
- Retain at most four decoded radio buffers: two current clips and two upcoming clips.
- Replace obsolete fixtures. Do not add old-schema loading, fallback clips, or migrations.
- Implement only critical tests.
- Use short sentences and the stop-slop skill for documentation and UI text.

## Review Focus

- Audio-only malformed data must disable audio while valid gameplay loads. Test in Tasks 1 and 2.
- Restart or removal during decode must prevent an old callback from starting playback. Test in Tasks 2 and 4.
- Several fixed steps must preserve distinct actions without duplicate event delivery. Test in Tasks 3 and 4.
- Origin shifts must preserve listener-to-emitter distance and stereo placement. Test in Tasks 2 and 6.
- Late radio loads must join the current broadcast without exceeding the four-buffer limit. Test in Task 5.

## Workspace and verification commands

Start from `codex/audio`, including the approved spec commit `e5f82d3` and this plan.
Use the git-worktrees skill at execution time to isolate implementation from other active work.
Check attached worktrees before creating one. Use the reviewed `codex/audio` commit as the starting ref.
Report the implementation worktree and branch before coding. Do not switch or reset another chat's checkout.
Preserve unrelated changes, including `docs/research/2026-10-12-survival-sleep-weather-reference.md` if it remains present.
Use explicit file paths when staging commits.

Run commands from the selected checkout. Use the existing Python environment or its verified equivalent.
Do not install dependencies unless an existing required tool is missing.
Keep source archives, decoded assets, diagnostic scenarios, and captures ignored by Git.

```powershell
tools/reference/.venv/Scripts/python.exe -m unittest tools.reference.tests.test_audio -v
node --test tests/web/audio.test.mjs
./scripts/check.ps1 -Test audio
./scripts/check.ps1 -Test radio
./scripts/check.ps1
./scripts/build.ps1
tools/reference/.venv/Scripts/python.exe -m tools.reference.export --stage runtime --verify
git diff --check
```

Do not run every command after every task. Each task names its required checks.
The final task runs the full suite, build, export verification, and audible checks.
Expected automated success means exit code zero, passed assertions, and no skipped required scenario.

## File map

| File | Responsibility |
| --- | --- |
| `tools/reference/audio.py` | Selected audio derivation and export validation |
| `tools/reference/library/reader.py` | Hash-checked WAV access and selection tracking |
| `tools/reference/export.py` | Connect audio derivation, file closure, and receipt verification |
| `src/reference/audio.rs` | Typed audio contract and validation |
| `src/audio.rs` | Signals, emitter ownership, bounded events, and source formulas |
| `src/simulation/radio.rs` | Source radio controls, schedules, and station selection |
| `src/starting_area.rs` | Connect successful gameplay actions and audio state |
| `src/app.rs`, `src/browser.rs` | Publish audio state and events through WASM |
| `web/audio.js` | Audio lifecycle, mixer routing, and browser bridge entry points |
| `web/audio/voices.js` | Retained voice slots, spatial curves, and starter scheduling |
| `web/audio/clips.js` | Effect cache, bounded radio cache, and load cancellation |
| Existing interaction modules | Emit source-backed events after successful operations |
| `web/main.js`, `web/menu.js`, `web/input.js` | Connect loading, gestures, focus, and source input |
| Focused test files | Verify extraction, failure isolation, events, voices, and radio |

## Shared contracts

Use numeric clip, emitter, voice, and binding IDs resolved once during export.
Keep source IDs and method evidence alongside those IDs.
Use existing `ItemId`, `NodeId`, `CurveKey`, and `SourceHash` where their meanings match.
Preserve source curve wrap modes, weighted tangents, and interpolation rules when exporting attenuation curves.
Do not add a second transform hierarchy or duplicate action eligibility rules.

`AudioReference` contains clips, emitters, voices, mixers, event bindings, engine rules, horn rules, and radio rules.
Clip records contain path, SHA-256, source identity, channels, frequency, sample width, frame count, and duration.
Emitter records contain ID, node, and optional item owner.
Voice records contain emitter, clip, mixer, priority, loop, gain, pitch, spatial blend, distances, and rolloff curves.
Event bindings contain source trigger, emitter, clip choice, gain rule, pitch range, and priority.
Mixer records contain source group links, gains, effect types, parameters, and active snapshot values.
Engine and horn records contain the recovered source constants and voice IDs used by their formulas.
Radio records contain authored controls, ranges, steps, initial state, station definitions, and three voice IDs.
Station definitions contain band, frequency, source gain, custom flag, playlists, and source selection parameters.

Add a required raw `audio: serde_json::Value` field to the current `ReferenceManifest`.
Keep gameplay validation independent of typed audio validation.
`AudioReference::from_value(raw: &serde_json::Value, game: &ReferenceManifest) -> Result<Self, ReferenceError>` validates audio separately.
The export validator requires this result to succeed. Gameplay startup catches its error and continues with audio disabled.
Do not provide an absent-field default or accept the previous schema.

`AudioState::new(reference: AudioReference, session: u32) -> Self` initializes retained signals and event storage.
Each event stores session, sequence, binding, emitter, position, gain, pitch, and simulation time.
Use a fixed queue capacity of 64. On overflow, discard the lowest-priority event, including the incoming event.
Use source priority ordering. Keep sequence numbers monotonic within a session.
`AudioState::push_event(event: AudioEvent)` queues one completed action.
`AudioState::pop_event() -> Option<AudioEvent>` drains events once.
`AudioState::reset(session: u32)` clears events, source timers, and active signals.

`createAudio(raw: object) -> AudioController` lives in `web/audio.js`.
It returns the same controller interface when audio initialization fails; that controller disables playback for the session.
This is failure containment, not support for obsolete manifests.
Use these controller methods:

```text
unlock(): void
begin(session: number, started: boolean, paused: boolean, gameSeconds: number): void
listener(position: Float32Array, rotation: Float32Array): void
emitter(id: number, position: Float32Array, rotation: Float32Array, alive: boolean): void
voice(id: number, playing: boolean, gain: number, pitch: number, looping: boolean): void
event(session: number, sequence: number, binding: number, emitter: number,
      x: number, y: number, z: number, gain: number, pitch: number, timeSeconds: number): void
radio(frame: RadioFrame): void
finish(): void
pause(): void
restart(session: number): void
dispose(): void
```

Retain bridge arrays and the browser `RadioFrame` object. Fill them in place.
Publish through `gameAudioBegin`, `gameAudioListener`, `gameAudioEmitter`, `gameAudioVoice`, `gameAudioEvent`, `gameAudioRadio`, and `gameAudioEnd`.
Use scalar WASM parameters to update retained browser data. Do not serialize JSON each frame.
`finish()` stops owners absent from the current frame and releases their unreferenced resources.

### Task 1: Derive selected source audio and validate its contract

**Files:** Create `tools/reference/audio.py`, `src/reference/audio.rs`, and `tools/reference/tests/test_audio.py`.
Modify `tools/reference/library/reader.py`, `tools/reference/export.py`, `src/reference.rs`, and `src/bin/validate-reference.rs`.
Update `tests/common/reference.json`; create `tests/audio.rs` and `docs/research/2026-10-12-audio-reference.md`.

**Interfaces:**
Produce `LibraryReader.audio(source: ObjectId) -> tuple[Path, dict]` and `LibraryReader.selected_audio`.
Produce `recover_audio(source, roots: dict, manifest: dict, library: LibraryReader, output: Path) -> dict`.
Produce `validate_audio_export(audio: dict, output: Path) -> None` and the shared `AudioReference` contract.

- [ ] Trace reachable AudioSources, clips, mixer groups, snapshots, source triggers, overrides, and control pointers.
  Read accepted records and managed code. Record exact numeric values and clip identities in the research document.
  Check the audio prefab, random-pitch helpers, priority ordering, and any enforced voice limit.
  Treat `maxAudioCount = 32` as a declaration until its call sites prove an enforced limit.
  Record silent actions and missing external content. Do not substitute inferred behavior.
- [ ] Add critical extraction tests using a small accepted-record fixture with source pointers and WAV receipts.
  `test_audio_uses_accepted_samples`: copied bytes and hashes equal the existing library bytes.
  `test_audio_closure_and_overrides`: each reachable voice resolves clip, owner, mixer, curve, and method evidence.
  `test_required_links_and_samples_fail`: broken pointers, changed WAV hashes, and invalid distances raise errors.
  `test_custom_content_is_external`: the source custom station retains frequency `1080` and has no invented playlist.
  Add native `audio_failure_does_not_reject_gameplay` with these assertions:

```rust
let mut value = serde_json::to_value(common::reference()).unwrap();
value["audio"] = serde_json::json!({"clips": "invalid"});
let game = ReferenceManifest::from_json(&serde_json::to_vec(&value).unwrap()).unwrap();
assert!(AudioReference::from_value(&game.audio, &game).is_err());
```

- [ ] Run Python audio tests and `./scripts/check.ps1 -Test audio`. Confirm failures identify missing audio interfaces.
- [ ] Implement selected-audio access without decoding Unity clips again. Extend category-index reading and selection receipts.
  Call audio derivation after gameplay and instrument bindings exist. Include selected audio files in exact runtime file closure.
  Validate audio in the export command and native validator, independently from gameplay validation.
  Replace the native fixture's schema and populate source-derived audio records.
- [ ] Run focused tests and derive the runtime with the existing export command.
  Run `--stage runtime --verify`. Confirm source hashes, selected WAV hashes, and native audio validation pass.
  Stop on unsupported required mixer behavior. Present the concrete source finding before changing the approved design.
- [ ] Commit only the Task 1 files. Use `Derive selected source audio and playback rules`.

### Task 2: Play one source effect with gesture initialization and lifecycle cleanup

**Files:** Create `src/audio.rs`, `web/audio.js`, `web/audio/voices.js`, `web/audio/clips.js`, and `tests/web/audio.test.mjs`.
Modify `src/lib.rs`, `src/browser.rs`, `src/app.rs`, `src/starting_area.rs`, `web/main.js`, `web/menu.js`, and `scripts/check.ps1`.
Extend `tests/audio.rs` and `tests/web/menu.test.mjs`.

**Interfaces:** Consume Task 1's audio records. Produce the shared `AudioState`, controller, and browser bridge methods.
Produce `ClipCache.effect(clip: number, session: number) -> Promise<AudioBuffer | null>` in `web/audio/clips.js`.
Produce `VoiceBank.update(id: number, playing: boolean, gain: number, pitch: number) -> void` in `web/audio/voices.js`.
Produce `VoiceBank.remove(emitter: number) -> void`. Let `AudioController` own both helpers and mixer nodes.

- [ ] Add critical controller tests with browser API fakes inside the test file.
  `gesture_precedes_pointer_lock`: AudioContext creation or resume occurs before the pointer-lock promise.
  `audio_failure_keeps_start_working`: failed fetch, decode, validation, and resume do not reject menu capture or gameplay startup.
  `restart_rejects_pending_decode`: finish an old decode after restart; assert no old voice starts.
  `restarted_session_can_unlock`: a failed old session does not prevent the next session's gesture from attempting initialization.
  `rebase_preserves_distance`: shift listener and emitter together; assert identical attenuation and stereo coordinates.
  `pause_requires_continue`: focus loss silences output; arbitrary game frames cannot resume AudioContext.
  Add the native event-queue assertions:

```text
Push events with sequences 1 and 2; pop returns 1, then 2, then None.
Push 65 events; queue length remains 64 and keeps higher-priority events.
Reset to session 2; no session-1 event remains.
```

- [ ] Run `node --test tests/web/audio.test.mjs tests/web/menu.test.mjs` and the native audio test target.
  Confirm failures identify absent lifecycle behavior before implementing it.
- [ ] Implement clip loading, mixer routing, retained voice slots, source attenuation, and the shared bridge.
  Route ignition through one successful source state change as the first audible effect.
  Use listener pose from the active rendered camera. Resolve emitters through existing scene and item bindings.
  Build the graph during initialization. Update retained gain, pitch, position, and orientation during frames.
  Catch audio failures inside the controller. Deduplicate diagnostics without routing errors to `gameFailure`.
  Call `unlock()` synchronously in capture handlers. Keep driving independent of its async completion.
  Invalidate the old session before restarting gameplay. Cancel stale callbacks and stop absent owners.
  Add the new Node test target to `scripts/check.ps1`.
- [ ] Run focused tests, the WASM build, and a browser ignition check through a real Start gesture.
  Confirm the game drives with a failed audio request. Confirm restart removes the ignition voice and pending callbacks.
- [ ] Commit Task 2 files. Use `Connect source audio to browser gestures and lifecycle`.

### Task 3: Add persistent engine and horn voices with source starter cadence

**Files:** Modify `src/audio.rs`, `src/starting_area.rs`, `src/app.rs`, `src/browser.rs`, `src/simulation.rs`, and `web/input.js`.
Modify `web/audio.js`, `web/audio/voices.js`, and supported horn-control bindings in `src/interaction/mechanisms.rs`.
Extend `tests/audio.rs`, `tests/input.rs`, and `tests/web/audio.test.mjs`.

**Interfaces:**
Produce `VehicleAudioInput { rpm: f32, ignition: bool, running: bool, starter: bool, horn: bool, ambient_temperature: f32, electronics_output: f32, engine_present: bool }`.
Produce `AudioState::vehicle(input: VehicleAudioInput, dt: f32, simulation_seconds: f64)`.
Produce `VoiceBank.starter(emitter: number, enabled: boolean, simulationSeconds: number) -> void`.

- [ ] Add critical formula, cadence, and ownership tests from Task 1's source values.
  `engine_formula_boundaries`: assert source gains and pitch at idle, crossover, maximum RPM, and overspeed.
  `horn_uses_ambient_temperature`: assert source pitch at night/day bounds and gain changes of `10 * dt`.
  `starter_uses_source_interval`: assert one-shot deadlines follow clip duration times `engineStartTimeModifier`, including source strict comparisons.
  `vehicle_updates_retain_voices`: run 600 unchanged updates; assert two engine loop starts and one held-horn start.
  `engine_removal_preserves_car_audio`: assert engine and starter stop while a live radio or horn owner remains.
  `ignition_edges_survive_fixed_steps`: two separate ignition changes before publishing produce two events, drained once.
- [ ] Run native and browser audio tests. Confirm the expected source formulas or voices fail before implementation.
- [ ] Implement source formulas, engine stop clips, ignition edges, horn smoothing, source input, and starter scheduling.
  Keep engine and horn sources active across parameter updates. Create buffer sources only when playback starts or source clips change.
  Schedule starter one-shots against source deadlines. Cancel future starts on release while preserving source-supported active tails.
  Use existing lighting ambient temperature. Do not connect horn pitch to engine temperature.
  Trace horn keyboard bindings and switch eligibility before adding the input bit.
- [ ] Run audio, input, and existing engine targets. Build and listen to the assembled vehicle.
  Check idle-to-throttle transitions, starter release, horn release, loop boundaries, distance, pause, and restart.
- [ ] Commit Task 3 files. Use `Add source engine starter ignition and horn playback`.

### Task 4: Add source-backed interaction events and movement voices

**Files:** Modify `src/audio.rs`, `src/starting_area.rs`, and `src/interaction.rs`.
Modify `src/interaction/mechanisms.rs`, `src/interaction/parts.rs`, `src/interaction/wheels.rs`, and `src/interaction/items.rs` where successful actions occur.
Extend `web/audio/voices.js`, `tests/audio.rs`, and `tests/web/audio.test.mjs`.

**Interfaces:**
Produce `AudioState::action(action: InteractionAction, simulation_seconds: f64)` for confirmed completed actions.
Produce `AudioState::mechanism(item: ItemId, previous: f32, current: f32, dt: f32)` for source movement and endpoint triggers.
Produce `AudioState::remove_owner(emitter: u32)` for actual removal, distinct from detachment.
Reuse the shared event and voice contracts. Use cached source binding maps.

- [ ] Add critical action and removal tests.
  `failed_attachment_is_silent`: a rejected attachment emits no event; a successful source-matching craft emits gain `0.25`.
  `silent_pickup_has_no_substitute`: an item without a source pickup trigger creates no playback.
  `mechanism_endpoints_emit_once`: repeated stationary updates create no endpoint duplicate or new movement voice.
  `fluid_events_require_actual_flow`: blocked transfer emits nothing; valid flow uses the recovered source rule.
  `removal_rejects_late_load`: remove an owner during fetch; assert its completed load cannot start playback.
  `multiple_steps_preserve_actions`: six fixed steps keep distinct successful events in source order without replay.
- [ ] Run audio tests and existing interaction, mechanism, part, wheel, and fluid targets.
  Confirm new tests fail because source-backed audio events are absent.
- [ ] Emit sounds at final success points. Do not infer completion from the returned action before deferred installation succeeds.
  Preserve the existing `InteractionState::step` return contract. Add event storage only where it avoids lost successful actions.
  Recover door movement, endpoint, cap, switch, pickup, attachment, and fluid rules from the audit.
  Update retained movement voices and source gains. Keep random pitch within the exported source ranges.
  Follow moving owners for attached voices. Stop removed owners and invalidate their pending loads.
- [ ] Run changed-target tests and browser audio tests. Build and listen to each source-supported interaction category.
  Compare the research audit's silent-action list against runtime event counts.
- [ ] Commit Task 4 files. Use `Add source interaction audio and emitter cleanup`.

### Task 5: Add Lada radio controls and bounded source playback

**Files:** Create `src/simulation/radio.rs` and `tests/radio.rs`.
Modify `src/simulation.rs`, `src/starting_area.rs`, `src/audio.rs`, `src/interaction/mechanisms.rs`, and `src/interaction/feedback.rs`.
Modify `src/browser.rs`, `src/app.rs`, `web/audio.js`, and `web/audio/clips.js`.
Extend `tests/web/audio.test.mjs` and the source audio research document.

**Interfaces:**
Produce `RadioState::new(reference: &RadioReference) -> Self`.
Produce `RadioState::control(control: RadioControl)` where the enum contains `VolumeUp`, `VolumeDown`, `TuneUp`, `TuneDown`, and `Band`.
Produce `RadioState::advance(game_seconds: f64, electronics_output: f32) -> RadioFrame`.
`RadioFrame` contains power, band, frequency, volume, static gain, and primary/secondary station playback fields.
Each playback field contains clip ID, offset seconds, gain, and next clip ID; absent recordings have no clip ID.
Produce `ClipCache.radio(current: [number|null, number|null], next: [number|null, number|null], session: number) -> void`.
Reuse `AudioController.radio(frame)`. Keep the browser frame object and playback fields retained.

- [ ] Add critical source control and schedule tests.
  `volume_controls_power`: source `vSpeed` steps clamp to `[0, 1]`; zero turns off and a positive step turns on.
  `tuning_uses_authored_ranges`: source rounding and step size match at both endpoints and halfway cases.
  `secondary_station_preserves_source_order`: a non-sorted frequency fixture follows the decompiled selection algorithm.
  `radio_signal_uses_electronics`: gains match source tolerances `0.4000000059604645` for FM and `20` for AM.
  `custom_1080_has_no_recording`: the external station keeps source static behavior and has no invented clip.
  `scaled_schedule_follows_game_time`: power-off does not stop broadcasts; pause and focus loss freeze the single-player clock.
  `late_radio_decode_joins_broadcast`: late playback starts at the current offset, within the source `0.5`-second correction rule.
  `radio_cache_is_bounded`: rapid tuning retains at most four unique decoded radio buffers and rejects stale requests.
- [ ] Run `./scripts/check.ps1 -Test radio` and browser audio tests. Confirm absent controls or playback cause failures.
- [ ] Connect source control pointers to existing mechanism tuning and feedback.
  Include band switching only if `BChangeAMFM` exists on the selected Lada.
  Preserve source music/talk transitions, gains, random-selection rules, and clocks without claiming the native random sequence.
  Advance the station clock from scaled game time, matching single-player `mainscript.RADIO`.
  Freeze that clock and mute output during pause and focus loss. Resume at the retained broadcast offset.
  Load only current and upcoming recordings. Share duplicate buffers and count pending decodes against the four-slot budget.
  Keep short-effect buffers separate. If a load is slow, defer it rather than evict a live engine buffer.
  Record missing external custom content in research. Add no file scanner or import controls.
- [ ] Run radio, mechanism, feedback, and browser audio tests. Build and listen to available bands, recordings, tuning, and static.
  Check power cycling, electronics gating, pause, repeated tuning, restart, and missing custom content.
- [ ] Commit Task 5 files. Use `Add source Lada radio controls and recordings`.

### Task 6: Verify assembled audible output and branch readiness

**Files:** Create `docs/research/2026-10-12-audio-verification.md`.
Update `docs/verification.md` and the audio scope line in `docs/roadmap.md`.
Fix only defects established by verification. Extend a critical regression test for each corrected behavior.

**Interfaces:** Consume the completed audio export, runtime, and browser controller. Add no product interface.

- [ ] Run Python source audio tests, native audio/radio tests, and browser audio tests.
  Confirm required cases pass without skipped lifecycle or source assertions.
- [ ] Run `./scripts/check.ps1`, `./scripts/build.ps1`, runtime export verification, and `git diff --check`.
  Confirm exit code zero and no invalid selected audio receipt or WASM build failure.
- [ ] Serve the assembled game through the project's existing development-server workflow.
  Use a real Start gesture, an identified browser, and an identified output device.
  Perform all ten scenarios in the spec. Record actual listening results and capture audio when supported.
  Check repeated engine/horn boundaries against source clip seams, not an invented fade.
  Measure near/far gains against exported curves. Rebase while listening and compare unchanged relative positions.
  Check stale voices after ten pause/resume cycles and ten restarts, including restart during fetch and decode.
  Require zero stale voices and unchanged settled voice counts after equivalent states.
  Force audio-only validation, missing-file, decode, and context failures; drive and restart in each case.
- [ ] Obtain a user listening check if the environment cannot verify audible device output.
  Record that limitation. Do not mark audible acceptance complete from waveform or API evidence alone.
- [ ] Resolve confirmed defects and repeat only affected checks. Keep remaining source or device limits explicit.
- [ ] Update verification and roadmap documentation to match the results. Leave unrelated feature exclusions intact.
- [ ] Run one independent whole-branch review after the selected execution method's task reviews.
  Resolve actionable findings and rerun affected checks before claiming completion.
- [ ] Commit verification documentation and fixes. Use `Verify source audio output and lifecycle`.

## Plan self-review

The six tasks cover source mapping, export reuse, playback selection, vehicle sounds, interactions, radio, cleanup, and audible acceptance.
The shared contracts use one audio record, one owner map, one event queue, and one browser controller.
The Review Focus cases have named tests in their owning tasks.
No source-dependent numeric playback value is invented; Task 1 recovers those values before playback implementation.
The queue capacity and radio cache bounds define resource limits, not new source behavior.
Each implementation task ends with working-game checks and an explicit commit.
The final task requires listening evidence or a user listening check.

## Execution choice

Recommend Native execution in this chat. The tasks share export contracts and lifecycle state, so one implementer limits interface churn.
Native execution includes one independent whole-branch review after implementation.
Subagent-driven execution remains available with a fresh implementer and reviewer for each task.
Wait for the user's plan review and execution choice before implementation.
