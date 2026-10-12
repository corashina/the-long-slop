# Source-based game audio

Date: 2026-10-12, Europe/Warsaw.
Status: The user approved the chat design. This written spec awaits review.
Branch: `codex/audio`.

## Intent and scope

Add source-based audio to the existing garage-to-road game.
Use installed Steam build `11140890`, Unity `2021.3.18f1`, as the reference.
Keep the selected Lada, existing physics, world, lighting, and interaction rules.

Include ignition, starter, engine, horn, and source-supported interaction sounds.
Include door movement, switches, pickup, attachment, and fluid actions when source code supports a sound.
Include the selected Lada's radio controls and available source recordings.
Handle pause, focus loss, restart, removal, and audio errors.
Require audible browser output and lifecycle checks before acceptance.

This spec replaces earlier audio exclusions for this local development scope.
It also replaces the first-slice requirement for original or synthesized audio.
Keep other exclusions: survival, weapons, other vehicles, saves, multiplayer, mirrors, and deployment.
Do not add stations, recordings, custom-file import, an audio settings menu, or control rebinding.
Do not add sounds to actions that lack source support.

## Reference authority and current evidence

Use accepted typed records and decompiled control flow together.
Resolve source pointers through container dependencies. Do not select clips by their names.
Keep object identities, hashes, field names, and method evidence in the export.

The current library contains 474 clips and 474 WAV files, totaling 2,030,938,742 bytes.
The catalog contains 2,156 AudioSources, one mixer, ten mixer groups, and one mixer snapshot.
These counts describe the full library. They do not describe runtime loading or verified playback.

Use these existing inputs:

| Input | Purpose |
| --- | --- |
| `web/assets/library/audio/index.json` | Clip identities, decoded WAV paths, hashes, formats, and durations |
| `.cache/reference-library/records/` | AudioSources, mixer records, selected hierarchies, and component fields |
| `.cache/reference-library/records/managed/Assembly-CSharp.cs` | Complete managed control flow |
| `.cache/reference-library/records/managed/decompilation.json` | Managed-analysis receipt |
| `tools/reference/library/audio.py` | Existing clip conversion and WAV validation |
| `tools/reference/library/reader.py` | Accepted-record and payload access |

The managed receipt reports complete decompilation with ILSpy `11.1.0.9782`.
Its Assembly-CSharp SHA-256 is `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
Check that receipt during derivation.

Before playback work, trace every reachable audio binding in the selected scenes.
Record AudioSource settings, clip pointers, transforms, mixer routing, triggers, and runtime overrides.
Trace shared sound creation through `mainscript.PlayClipAtPoint` and the referenced audio prefab.
Audit mixer gains, effects, snapshot values, exposed controls, and settings call sites.
Identify required browser equivalents and record any unresolved audible behavior.
An unresolved required behavior blocks an audio parity claim.

## Playback path

Use browser Web Audio through the existing wasm-bindgen bridge.
Keep pinned Rust dependencies. Add no runtime package and no Bevy audio feature.
The browser path gives direct control over source curves, routing, timing, and gesture initialization.
Bevy audio was considered. It requires enabled audio features and separate source-distance verification.

Use AudioBufferSourceNode for short effects and radio recordings.
Use retained GainNodes and PannerNodes for voice gain and spatial position.
Implement authored attenuation curves from exported keys when browser distance models do not match.
Avoid applying attenuation twice. Preserve source stereo and spatial-blend rules.
Use supported Web Audio nodes for reachable mixer processing recovered from source.
Do not substitute browser defaults for recovered source values.

AudioBufferSourceNodes permit one start. Create replacements only for playback starts, clip changes, or scheduled one-shots.
Reuse decoded buffers and retained voice routing. Do not recreate active loops each frame.
Use the audio clock to schedule clips. Preserve source cadence without accumulating render-frame timing error.

Create or resume AudioContext inside Start or Continue's user-gesture handler.
Do this before awaiting pointer lock. Begin gameplay sounds only after gameplay starts.
Decode selected effects once. Do not delay driving while audio loads or resumes.
Catch fetch, decode, context, and playback errors inside the audio module.

## Components and ownership

| Component | Responsibility |
| --- | --- |
| `tools/reference/audio.py` | Derive selected bindings, playback settings, mixer rules, and radio definitions |
| `tools/reference/library/reader.py` | Read and hash-check selected audio payloads from the accepted library |
| `src/reference/audio.rs` | Validate the current audio manifest contract |
| `src/audio.rs` | Derive audio signals and retain successful-action events without browser objects |
| `src/simulation/radio.rs` | Store tuning, volume, power, and source station selection and timing |
| `src/browser.rs` | Publish fixed audio fields and queued events through WASM |
| `web/audio.js` | Own AudioContext, clip loading, voice routing, playback, and cleanup |
| Existing interaction and input modules | Apply source horn and radio controls through existing eligibility rules |
| `web/main.js` and `web/menu.js` | Connect audio to loading, gestures, pause, focus, and restart |

Rust owns simulation state and action eligibility. JavaScript owns playback resources.
Audio failures cannot change fuel, engine output, item state, physics, or permitted actions.
Publish RPM, running state, starter state, ignition, horn level, ambient temperature, and electronics output.
Publish listener pose and emitter poses in one coordinate space.
Apply origin changes to listener and emitters together to prevent sound jumps.

Emit events where successful actions occur. Do not infer events from HUD text or input alone.
Use retained storage for events between fixed steps and presentation updates.
Drain events once, including when several fixed steps run before a browser update.
Keep a bounded event queue. Drop the lowest-priority audio event on overflow without blocking simulation.
Use session and emitter identities to reject stale events.
Build binding maps and voice slots during loading. Avoid per-frame allocations and setup.

## Export contract

Extend the existing runtime export and manifest. Keep one export command and one current schema.
Read accepted WAV files before considering changes to clip conversion.
Select clips through reachable source bindings, including station playlists and shared audio prefabs.
Copy selected files into the runtime export and preserve their hashes and source identities.
Validate selected WAV metadata against the accepted library.

Export source volume, pitch, loop, spatial blend, rolloff mode, curve keys, distances, priority, and mixer routing.
Export emitter bindings to stable scene node or item identities.
Export runtime formulas and their source constants through typed records.
Export radio ranges, steps, control pointers, startup values, station frequencies, playlists, and playback rules.
Classify missing custom recordings as external content, distinct from failed required source exports.
Reject missing required records, unresolved non-null pointers, invalid ranges, and non-finite playback values.
Replace obsolete fixtures. Do not add old-schema loading, fallback clips, or migrations.

Runtime audio loading can fail without stopping driving.
Reference derivation must still reject incomplete required audio data before publishing an accepted manifest.
Keep runtime audio validation and initialization inside a separate failure boundary from gameplay loading.
Malformed audio records must disable audio without rejecting valid gameplay records.

## Vehicle behavior

`enginescript` controls two engine voices: `SAlap` and `SMotor`.
Recover their AudioSource clips, loop flags, positions, and mixer settings.
Apply source RPM ranges, gain multipliers, and pitch interpolation to these existing voices.
Preserve the source's clamped InverseLerp terms and unclamped pitch interpolation.
Stop the voices when the engine stops or its emitter disappears.
Play `SStop` on the source running-to-stopped transition when the selected engine supplies that clip.

The starter uses repeated `SStart` one-shots, not an assumed continuous loop.
Preserve the source interval based on clip duration and `engineStartTimeModifier`.
Keep source eligibility, gain, priority, emitter attachment, and completion rules.
Do not create one starter voice for each fixed or render update.

`carscript` plays `turnedRadioSOund` for ignition and light-switch changes.
Emit one event per successful state change, including multiple changes within one frame.

`carscript` sets horn gain from `currenthorn` and stops playback at zero.
Its normal horn level moves toward zero or one at ten units per second.
Recover supported input and switch eligibility from source.
Derive pitch from `HornPitchMin`, `HornPitchMax`, and source ambient-temperature bounds.
Use the existing day-cycle ambient temperature, not engine temperature.
Keep one horn voice for each existing horn emitter.

## Interaction behavior

Trace `usablescript`, `pickupable`, cap handlers, player action methods, and fluid call sites.
Link sound events to existing door, switch, pickup, attachment, and fluid operations.
Preserve movement-dependent gains, endpoint clips, random-pitch ranges, and source trigger thresholds.
Update movement voices from mechanism motion instead of replaying them each frame.
Stop movement voices when motion ends or the item disappears.

`pickupable.Crafted` uses the item's collision clip at gain `0.25` with random pitch.
Apply that rule only where the current attachment action matches the recovered source call.
Do not treat collision clips as generic pickup sounds.
Record unsupported or silent actions in the audit without adding substitute sounds.

Shared one-shots must retain source priority and emitter behavior.
Preserve `PlayClipAtPoint` gain handling, including its distance changes when requested gain exceeds one.
Recover the source one-shot budget and its eviction rule before choosing pool behavior.
Support overlapping legitimate actions while preventing duplicate delivery of the same event.

## Lada radio

Bind controls to the selected radio's exported `Bvolume`, `Btune`, and optional `BChangeAMFM` pointers.
Expose only controls that exist on this Lada and work through its source interaction rules.
Reuse existing prompts, aim targets, switch motion, and tuning input.
Do not introduce station-picker buttons or a separate radio panel.

`radioscript` changes volume in `vSpeed` steps between zero and one.
Zero volume turns power off. Raising volume above zero turns power on.
Playback also requires electronics output above the source cutoff.
Use authored tuning ranges and frequency steps. Keep source tuning clamping and rounding.
Apply band switching only when the selected radio provides that control.
Play the source control click at the authored gain and pitch range.

The source radio uses static plus two station voices.
Preserve the exact source station-search algorithm, including its secondary-station selection order.
Derive signal gain from frequency distance, electronics output, volume, and station gain.
Apply each station-gain multiplier only to the source voice that uses it.
The source tolerance is `0.4000000059604645` for FM and `20` for AM.
Use exported values rather than duplicating these constants in playback code.

The main source record defines twelve FM entries and four AM entries.
The custom AM entry is at `1080` and contains no embedded playlist.
`custommusicscript` reads user files with `.ogg`, `.wav`, or `.mp3` extensions.
Do not scan user directories, fetch replacement recordings, or invent custom content.
Retain that source tuning location if reachable. Report unavailable content in the audit.
Preserve its source static behavior and silence its missing recording voice.

Use source music/talk selection rules and source clip durations for station schedules.
Keep station timing independent of whether the player has powered the radio on.
Use an unscaled station clock, as source `mainscript` does.
Mute radio output during pause and focus loss; resume at the current broadcast position.
Preserve the source time-correction threshold of `0.5` seconds.
Do not claim the same native random sequence without matching runtime evidence.

Retain at most four decoded radio buffers: two current clips and two upcoming clips.
Share duplicate clip buffers. Evict unreferenced radio buffers on station or clip changes.
Read playlist durations from metadata without decoding unrelated recordings.
If a clip loads late, seek to its current station position. Do not restart its broadcast timeline.
Keep short effects in a separate cache so radio eviction cannot interrupt the engine or horn.

## Lifecycle and failure handling

Pause or focus loss must silence output and suspend active effect playback.
Resume only through the existing Continue gesture and gameplay state.
Do not replay expired action events after suspension.
Retain effect loop position across pause where the source voice remains active.
Reconcile current engine, horn, mechanism, and radio state on resume.

Restart increments the audio session identity before replacing gameplay state.
Stop and disconnect old voices, clear pending events, and invalidate old load callbacks.
Reset radio controls and station schedules to the exported startup rules.
Retain immutable short-effect buffers. Release radio buffers that no longer have an owner.

Removing a vehicle or item ends its owned voices and rejects its pending loads.
Removing an engine ends its engine and starter ownership without removing the car's radio or horn ownership.
Use source attachment rules for moving emitters and one-shot tails while their owner exists.

An audio error disables the affected clip or route and records a diagnostic.
Catch rejected promises before the global runtime-failure handler receives them.
Prevent repeated error logs and retry loops during frame updates.
If AudioContext fails, disable audio for that session and continue gameplay.
Show no blocking audio dialog. Keep startup, driving, interactions, and restart usable.

## Verification and acceptance

Implement only critical tests for source extraction, event delivery, and resource lifecycle.
Check exported pointer resolution, hashes, WAV metadata, formulas, and reachable radio definitions.
Check engine gains and pitch at idle, crossover, maximum RPM, and overspeed.
Check ignition changes, starter cadence, horn release, and successful-action event delivery.
Check duplicate suppression without suppressing separate legitimate actions.
Check pause, focus loss, restart, removal, stale loads, and failed decode/context behavior.
Check radio tuning, power, electronics gating, station selection, clocks, and missing custom content.

Run the existing native and browser checks after integration.
Use the assembled game for audible acceptance, not only isolated fixtures.
Start audio with a real user gesture and verify sound at the output device.
Capture a listening sample when the environment permits system-audio capture.
Record browser, output device, scenario, and result in the verification report.
Waveforms, voice counters, and successful API calls alone do not prove audible device output.
If output listening is unavailable, report that acceptance gap and obtain a user listening check.

Perform these browser scenarios:

1. Toggle ignition, hold the starter, start the engine, change RPM, and stop the engine.
2. Hold and release the horn. Confirm temperature-dependent pitch and one active horn voice.
3. Open and close doors, use switches, attach items, and perform supported fluid actions.
4. Tune available radio controls across stations and static. Verify source recordings and missing custom content.
5. Listen across repeated engine and horn loop boundaries. Check continuity against the source clip seams.
6. Walk away from emitters. Check gain and stereo placement at authored distance points.
7. Pause and lose focus during playback, then resume through Continue.
8. Restart during playback and during an audio fetch. Confirm old voices never return.
9. Remove supported emitters. Confirm their voices and pending loads lose ownership.
10. Force a missing clip, decode failure, and context failure. Confirm driving and restart still work.

Compare voice counts before and after repeated pause, restart, and removal cycles.
Require zero stale voices and no growth after equivalent settled states.
Use source samples and expected gains to check attenuation; do not accept browser defaults without comparison.
Keep remaining source or audible-output differences explicit.

## Implementation order

First derive and validate source audio data from the accepted library.
Then connect gesture initialization and one source effect to the running game.
Add vehicle voices and lifecycle cleanup before interaction audio and radio.
Add interaction sounds through existing successful-action paths.
Add the Lada radio with bounded loading and source station timing.
Finish with failure checks and assembled audible acceptance.
Keep the playable game working after each layer.

## API references

- [AudioBufferSourceNode](https://developer.mozilla.org/en-US/docs/Web/API/AudioBufferSourceNode): playback, looping, and single-use source nodes.
- [Web Audio practices](https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API/Best_practices): initialize or resume from a user gesture.
- [PannerNode](https://developer.mozilla.org/en-US/docs/Web/API/PannerNode): source positioning and available distance models.
