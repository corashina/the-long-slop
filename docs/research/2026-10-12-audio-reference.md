# Source audio reference

Date: 2026-10-12. Steam build: `11140890`. Unity: `2021.3.18f1`.

We traced accepted records and the complete managed decompilation.
We copied accepted WAV bytes. We did not decode Unity clips again.

## Source receipts

- Assembly-CSharp SHA-256: `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
- Decompiled C# SHA-256: `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee`.
- UnityPlayer SHA-256: `553c1e7560e1b42bd4638c53d0168dcd1b4b9eb5b9df324b0a44f14b3f18bc86`.
- ILSpy version: `11.1.0.9782`. Receipt: `managed/decompilation.json`.

## Selected closure

The selected closure contains 196 clips and 50 AudioSources.
It contains 126 emitters, 125 event bindings, and 10 mixer groups.
The WAV files contain 1,853,681,564 bytes.
Clip usage separates 31 effect clips from 165 radio clips.
Each runtime clip retains its source identity, SHA-256, format, frames, and exact WAV duration.
The runtime manifest contains the complete selected clip and method receipts.
The native test fixture rebinds source audio to its synthetic gameplay owners.
The fixture retains source gains, pitches, ranges, curves, clips, and station definitions.

## Voice settings

Priority below means the effective priority. `authored_priority` retains the serialized priority.
Engine Start changes both engine voices from 128 to 80.
Radio Start changes SS and SC from 120 to 80. SC2 remains 120.
Curve keys retain wrap modes, slopes, weights, and weighted modes.
All selected rolloff modes are serialized values. Browser defaults are not source evidence.

| AudioSource | Clip | Mixer | Gain | Pitch | Loop | Priority | Minimum | Maximum | Blend |
| --- | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| level1:56384 | sharedassets1.assets:4259 | Main | 1.0 | 1 | False | 128 | 1.0 | 100.0 | 1.0 |
| level1:56694 | sharedassets1.assets:4259 | Main | 1.0 | 1 | False | 128 | 1.0 | 100.0 | 1.0 |
| level1:56469 | sharedassets1.assets:4259 | Main | 1.0 | 1 | False | 128 | 1.0 | 100.0 | 1.0 |
| level1:56594 | sharedassets1.assets:4259 | Main | 1.0 | 1 | False | 128 | 1.0 | 100.0 | 1.0 |
| level1:56791 | sharedassets1.assets:4259 | Main | 1.0 | 1 | False | 128 | 1.0 | 100.0 | 1.0 |
| level1:56686 | sharedassets1.assets:4259 | Main | 1.0 | 1 | False | 128 | 1.0 | 100.0 | 1.0 |
| level1:56412 | sharedassets1.assets:4259 | Main | 1.0 | 1 | False | 128 | 1.0 | 100.0 | 1.0 |
| level1:56755 | sharedassets1.assets:4259 | Main | 1.0 | 1 | False | 128 | 1.0 | 100.0 | 1.0 |
| level1:56253 | sharedassets1.assets:4346 | Main | 1.0 | 1 | False | 128 | 1.0 | 100.0 | 1.0 |
| level1:56072 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |
| level1:56593 | sharedassets1.assets:4339 | OutSideMenu | 0.5 | 1.0 | True | 128 | 0.5 | 20.0 | 1.0 |
| level1:56751 | sharedassets1.assets:4339 | OutSideMenu | 0.5 | 1.0 | True | 128 | 0.5 | 20.0 | 1.0 |
| level1:56675 | sharedassets1.assets:4339 | OutSideMenu | 0.5 | 1.0 | True | 128 | 0.5 | 20.0 | 1.0 |
| level1:56668 | sharedassets1.assets:4035 | OutSideCar | 1.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56669 | sharedassets1.assets:4350 | OutSideCar | 0.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56671 | sharedassets1.assets:4490 | OutSideCar | 0.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56667 | sharedassets1.assets:4035 | OutSideCar | 1.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56656 | sharedassets1.assets:4350 | OutSideCar | 0.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56657 | sharedassets1.assets:4490 | OutSideCar | 0.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56659 | sharedassets1.assets:4035 | OutSideCar | 1.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56661 | sharedassets1.assets:4350 | OutSideCar | 0.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56655 | sharedassets1.assets:4490 | OutSideCar | 0.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56654 | sharedassets1.assets:4035 | OutSideCar | 1.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56665 | sharedassets1.assets:4350 | OutSideCar | 0.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56666 | sharedassets1.assets:4490 | OutSideCar | 0.0 | 1.0 | True | 128 | 1.0 | 500.0 | 1.0 |
| level1:56662 | runtime one-shots | OutSideCar | 1.0 | 1.0 | False | 120 | 1.0 | 1000.0 | 1.0 |
| level1:56658 | sharedassets1.assets:4068 | OutSideCar | 1.0 | 1.0 | True | 120 | 1.0 | 1000.0 | 1.0 |
| level1:56670 | sharedassets1.assets:4068 | OutSideCar | 1.0 | 1.0 | False | 120 | 1.0 | 500.0 | 1.0 |
| level1:56663 | sharedassets1.assets:4465 | OutSideCar | 0.0 | 1.0 | True | 120 | 0.5 | 500.0 | 1.0 |
| level1:56660 | sharedassets1.assets:4487 | OutSideCar | 0.0 | 1.0 | True | 120 | 1.0 | 2000.0 | 1.0 |
| level1:56664 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |
| level1:56702 | sharedassets1.assets:4280 | Ear | 1.0 | 1 | False | 128 | 1.0 | 500.0 | 1.0 |
| level1:56216 | sharedassets1.assets:4280 | Ear | 1.0 | 1 | False | 128 | 1.0 | 500.0 | 1.0 |
| level1:56174 | sharedassets1.assets:4280 | Ear | 1.0 | 1 | False | 128 | 1.0 | 500.0 | 1.0 |
| level1:56520 | sharedassets1.assets:4280 | Ear | 1.0 | 1 | False | 128 | 1.0 | 500.0 | 1.0 |
| level1:56311 | sharedassets1.assets:4456 | Radio | 0.4000000059604645 | 1.0 | True | 80 | 1.0 | 600.0 | 1.0 |
| level1:56280 | sharedassets1.assets:4456 | Radio | 0.4000000059604645 | 1.0 | True | 80 | 1.5 | 600.0 | 0.9900000095367432 |
| level1:56608 | sharedassets1.assets:4456 | Radio | 0.4000000059604645 | 1.0 | True | 120 | 1.5 | 600.0 | 0.9900000095367432 |
| level1:56217 | sharedassets1.assets:4181 | OutSideCar | 1.0 | 1.0 | True | 128 | 1.0 | 20.0 | 1.0 |
| level1:56279 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |
| level1:56711 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |
| level1:56674 | sharedassets1.assets:4228 | OutSideCar | 1.0 | 1.0 | True | 80 | 1.5 | 1000.0 | 1.0 |
| level1:56643 | sharedassets1.assets:4378 | OutSideCar | 1.0 | 1.0 | True | 80 | 1.5 | 1000.0 | 1.0 |
| level1:56629 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |
| level1:56752 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |
| level1:56527 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |
| level1:56524 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |
| level1:56404 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |
| level1:56526 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |
| level1:56363 | sharedassets1.assets:4426 | Main | 1.0 | 1.0 | True | 128 | 1.0 | 50.0 | 1.0 |

## Engine and horn

Engine component: `level1:68282`. Idle clip: `sharedassets1.assets:4378`.
Motor clip: `sharedassets1.assets:4228`. Starter: `sharedassets1.assets:4423`. Stop: `sharedassets1.assets:4357`.
`enginescript.Update`, lines 12053-12225, owns the formulas and starter cadence.
Both gains use clamped InverseLerp. Both pitches use unclamped Lerp.
Idle gain: `InverseLerp(SMaxAlapRpm, SMinAlapRpm, rpm) * SMaxAlapSound`.
Motor gain: `InverseLerp(SMinMotorRpm, SMaxMotorRpm, rpm) * SMaxMotorSound`.
Idle pitch uses `InverseLerp(0, SMaxAlapRpm, rpm) + InverseLerp(SMaxAlapRpm, 2*SMaxAlapRpm, rpm)`.
Motor pitch uses `InverseLerp(SMinMotorRpm, maxRpm, rpm) + InverseLerp(maxRpm, 2*maxRpm, rpm)`.
We retain these exact source constants:

| Source field | Value |
| --- | ---: |
| `SMinAlapRpm` | `600.0` |
| `SMaxAlapRpm` | `1400.0` |
| `SMinMotorRpm` | `500.0` |
| `SMaxMotorRpm` | `1300.0` |
| `SMaxAlapSound` | `0.699999988079071` |
| `SMaxMotorSound` | `0.699999988079071` |
| `minAlapPitch` | `0.8999999761581421` |
| `maxAlapPitch` | `1.2000000476837158` |
| `minMotorPitch` | `0.800000011920929` |
| `maxMotorPitch` | `1.5` |
| `maxRpm` | `6600.0` |
| `engineStartTimeModifier` | `1.0` |
| `SStartVolume` | `0.6000000238418579` |
| `SStopVolume` | `0.6000000238418579` |

Starter one-shots require `Time.time - lastStarted > SStart.length * engineStartTimeModifier`.
The source attaches starter and stop objects to the engine transform.
`carscript.Update` moves normal horn gain toward zero or one at 10 units per second.
It sets horn pitch from ambient temperature, with bounds `-9` and `35` Celsius.
Horn pitch bounds are `0.8999999761581421` and `1`.
Ignition and light switching use `turnedRadioSOund`, clip `sharedassets1.assets:4437`, with PlayOneShot gain `1`.

## Shared one-shots

`mainscript.PlayClipAtPoint`, lines 30172-30270, creates the source audio prefab.
Prefab: `sharedassets1.assets:21250` GameObject. The export retains its AudioSource identity.
Its gain and pitch are `1`. It does not loop. Distances are `0.5` and `200` meters.
The source random-pitch helper samples `[0.85, 1.15]`.
Gain above one clamps playback gain to one and scales both distances by `requested_gain - 1`.
The source destroys one-shot objects after the source clip duration, independent of pitch.
Source priority order is `[10, 25, 40, 55, 70, 80, 85, 100]`.
Lower priority numbers mean higher playback priority.
We found one `maxAudioCount` declaration with value `32` and no reads.
`OneShotAudios` has its declaration and an Add call. We found no managed culling or eviction.
AudioManager `globalgamemanagers:4` declares 128 real voices and 512 virtual voices.
It enables effect virtualization. Rolloff scale and Doppler factor are both `1`.
The browser event queue capacity of 64 is a product resource limit.

## Interactions

`usablescript.RotMoveSound` assigns SMove, disables looping, and samples pitch `[0.95, 1.05]`.
Rotation gain uses `audioM * movement_difference * 0.1`, with gain smoothing `deltaTime * 3`.
Y movement requires a random sample above the source squeak threshold, normally `0.75`.
Slide gain uses `audioM * audioMSlide * movement_difference * 2`. Zero audioMSlide becomes one.
Slide audio stops after `0.2` seconds without a movement update.
Endpoint gain uses `abs(movement_delta * audioM) / deltaTime * 0.01`.
Endpoint and switch bindings retain their exact source clip choices and method evidence.
`pickupable.Crafted` uses its collision clip at gain `0.25`, with pitch `[0.85, 1.15]`.
Slot Craft and UnCraft call Crafted after success at managed lines 47847 and 47953.
Plain pickup and tube Detach do not call Crafted. Tube Detach appears at lines 65485-65499.
We added no generic pickup, drop, or tube detach sound.
`carscript.InteractHandbrake` changes state without playing a sound.
Cap USE uses authored open and close clips and gains.
Cap TUNE gain uses `abs(input * mainscript.fluidTuneSpeed * tuneSpeed)`.
Cap PlayFillSound disables looping and sets gain to `maxVolume * requested_gain`.
Cap pouring gain uses `InverseLerp(minPourSpeedSound, maxPourSpeedSound, speed)`.
The export retains the source three-step fill latch and both pour-speed bounds.
Five flashlightscript controls belong to selected lamps. They provide ten on/off sound bindings.
The export binds lamp voices to existing light controllers and `flashlightscript.Refresh`.

## Mixer and native ParamEQ

Mixer: `sharedassets0.assets:810`. Active snapshot: `sharedassets0.assets:821`.
We resolve group GUIDs to compiled group indices. We retain the source parent graph.
Bell gain is `-5.323071479797363` dB. Other snapshot group gains are zero dB.
All snapshot group pitches are one. Radio is group index 9.
Radio effects are attenuation type `-2`, then active ParamEQ type `10`.
ParamEQ parameter indices are `[20, 21, 22]`.
Values are `[140, 2.4600000381469727, 0.46000000834465027]`.
We identified type 10 through installed UnityPlayer native registration, not through parameter resemblance.
Registration function VA: `0x1814436f0`. Type write VA: `0x181443798`.
It registers `FMOD ParamEQ` and writes type `0x0a`.
Its parameter descriptor starts at VA `0x181a8d520`.
Parameter names are `Center freq`, `Octave range`, and `Frequency gain`.
Coefficient function VA: `0x181444ff0`.
The source computes `w = 2*pi*center/sample_rate` and `alpha = sin(w)*bandwidth/2`.
Its numerator is `[1+alpha*A, -2*cos(w), 1-alpha*A]`.
Its denominator is `[1+alpha/A, -2*cos(w), 1-alpha/A]`, where `A` is the authored gain.
A Web Audio peaking filter can match with `Q = 1/bandwidth` and `gain_db = 40*log10(A)`.
The parameter label says octave range. The native function uses reciprocal Q.
We found no unsupported required mixer effect.

## Runtime mixer controls and interiors

Exposed names resolve by CRC32 to the source name hashes:

| Name | Hash | Parameter index |
| --- | ---: | ---: |
| CarSounds | 569239599 | 16 |
| PlayerEar | 917204611 | 8 |
| MasterVolume | 410686301 | 0 |
| Outside | 2052572218 | 14 |
| InGame | 499806853 | 6 |

`mainscript.SetVolume` computes `ln(clamp(value, 0.001, 1)) * 20`, using the natural logarithm.
`fpscontroller.SetOutSideAudio` applies this conversion to Outside and CarSounds.
Player defaults are interior gain `0.5`, smoothing `2`, and car sound multiplier `1.399999976158142`.
Initial outside gain is zero. Without an interior, the target gain is one.
Inside an interior, the target starts at default interior gain.
It adds `(1 - authored_or_default_interior_value) * minimum_overlap_transmission`.
`interiorscript.GetValue` reads existing door mechanisms and removable door slots.
The export resolves six interiors to existing collider, door, and slot IDs.
We preserve parent links, individual inversion flags, limits, and source field values.
Player sensor `level1:80121` uses sens2. Radius: `0.05000000074505806`. Layer mask: `8192`.
Its transform `level1:39826` has zero local position under player transform `level1:39811`.
It queries OverlapSphere, retains its returned order, and uses the first overlap as currentInterior.
It retains sensed state for three fixed steps after the last overlap.
Rapier and native PhysX can return different overlap orders. We do not claim matched native ordering.
Route component `level1:65624` controls 18 selected voices through audioBasedOnInteriorsensscript.
It selects Main, OutSideCar, or CarSounds from player and emitter interior sensing.
The export retains its trigger sensor, default interiors, voice IDs, and mixer IDs.

## Radio

Radio: `level1:69420`. Controls: Bvolume `77810`, Btune `70967`, BChangeAMFM `78052`.
All three controls resolve. The export retains their source pointers and scene node IDs.
Initial power is off. Initial volume is zero. Initial band is FM.
Initial tuning fraction is `0.32499998807907104`.
AM range: `540` to `1600`, step `5`. FM range: `88` to `108`, step `0.10000000149011612`.
Volume step is `0.10000000149011612`. Control click gain is `1`.
Tolerance is `0.4000000059604645` for FM and `20` for AM.
Electronics cutoff is `0.2`. Time correction threshold is `0.5` seconds.
Late seek margin is `0.1` seconds. Custom recording gain multiplier is `0.5`.
The station search promotes the previous nearest station when it finds a closer station.
It does not compute a separate second-nearest search.
The primary voice applies station currentVolume. The secondary voice does not apply that multiplier.
Station selection alternates music and talk when both lists exist.
Each category uses a random bag without replacement. Source holiday flags filter each bag.
Short or zero gain arrays imply gain one for that playlist entry.
Single-player radio timing uses scaled `Time.time`.
`radioCH.SelectRandomMusic` starts this clock at lines 27947-27968.
`radioCH.SelectRandomTalk` starts it at lines 27970-27991.
`mainscript.RADIO` updates clip time at lines 30303-30344.
`mainscript.RADIO2` uses `Time.unscaledTime` at lines 30346-30352 for multiplayer.
The export selects `RADIO` and retains `RADIO2` as a source receipt.
We do not claim matched native random sequences.

| Band | Frequency | Name | Music clips | Talk clips | Content |
| --- | ---: | --- | ---: | ---: | --- |
| fm | 89.19999694824219 | Accoustic | 11 | 0 | embedded |
| fm | 90.5 | Classic | 11 | 0 | embedded |
| fm | 91.30000305175781 | Dance | 11 | 0 | embedded |
| fm | 93.0 | hungare | 7 | 13 | embedded |
| fm | 94.0999984741211 | Rock | 14 | 16 | embedded |
| fm | 98.4000015258789 | Synth | 25 | 13 | embedded |
| fm | 105.0 | EE | 0 | 1 | embedded |
| fm | 100.5 | Poem | 4 | 8 | embedded |
| fm | 95.0 | CTYD | 0 | 11 | embedded |
| fm | 97.5 | ubermensch | 0 | 7 | embedded |
| fm | 88.80000305175781 | magyarvicc | 7 | 0 | embedded |
| fm | 96.5999984741211 | angrygreg | 0 | 9 | embedded |
| am | 630.0 |  | 3 | 0 | embedded |
| am | 800.0 | oldie | 3 | 5 | embedded |
| am | 1080.0 | custom | 0 | 0 | external |
| am | 1330.0 | matyas | 3 | 3 | embedded |

Custom AM frequency `1080` has no embedded recordings.
custommusicscript loads external `.ogg`, `.wav`, and `.mp3` user files.
We did not scan user folders or invent a playlist.
The export retains this unavailable external station and its source static rule.

## Validation limits

The export validates source receipts, WAV hashes, WAV metadata, file closure, and native audio types.
Malformed audio remains outside ReferenceManifest gameplay validation.
The native export validator requires typed audio validation to pass.
These checks do not establish audible browser output, native random parity, or native overlap-order parity.
Later tasks must verify device output and lifecycle behavior in the assembled game.

## External references

- [Unity AudioSource priority](https://docs.unity3d.com/2021.3/Documentation/ScriptReference/AudioSource-priority.html).
- [Unity ParamEQ editor source](https://github.com/Unity-Technologies/UnityCsReference/blob/master/Editor/Mono/Audio/Effects/ParamEQGUI.cs).
- [Stop-slop writing skill](https://github.com/hardikpandya/stop-slop/blob/main/SKILL.md).
