# Selected audio Doppler

Build 11140890 uses Unity 2021.3.18f1 (3129e69bc0c7).
The accepted UnityPlayer.dll has SHA-256 `553c1e7560e1b42bd4638c53d0168dcd1b4b9eb5b9df324b0a44f14b3f18bc86`.
Its RSDS record names `UnityPlayer_Win64_player_mono_x64.pdb`, GUID `31071DF7C3F54B5E8A4EC3F5E342FBAB`, age 1.
The matching [Unity symbols](https://symbolserver.unity3d.com/UnityPlayer_Win64_player_mono_x64.pdb/31071DF7C3F54B5E8A4EC3F5E342FBAB1/UnityPlayer_Win64_player_mono_x64.pd_) identify these methods:

- `AudioSource::CacheFrameParameters`, RVA `0x9a5840`, 1704 bytes.
- `AudioSource::CachePositional`, RVA `0x9a5ef0`, 777 bytes.
- `AudioListener::DoUpdate`, RVA `0x9b8b20`, 701 bytes.
- `SoundChannelInstance::UpdatePitch`, RVA `0x9cfd70`, 84 bytes.

`CachePositional` subtracts listener velocity from source velocity. It also subtracts listener position from source position.
`CacheFrameParameters` computes radial velocity from their dot product, divided by distance. Zero distance gives zero radial velocity.
Instructions `0x9a5af2` through `0x9a5b2f` apply:

```
shift = max(1e-6, (340 - radial_velocity * manager_doppler * source_doppler) / 340)
multiplier = 1 - spatial_blend + spatial_blend * shift
rate = selected_pitch * mixer_pitch * multiplier
```

The native constants reside at RVAs `0x17c61c8` (340) and `0x16c1bbc` (float32 1e-6).
AudioManager `globalgamemanagers:4` supplies Doppler Factor 1. Each selected AudioSource supplies DopplerLevel 1.
All selected source GameObjects lack a local Rigidbody. Strict export checks this condition.
Unity therefore derives their velocity from transform displacement and frame delta.
The listener uses the same displacement rule. See [Unity velocity update mode](https://docs.unity3d.com/2021.3/Documentation/ScriptReference/AudioSource-velocityUpdateMode.html).

The shared VoiceBank uses accepted simulation seconds plus the interpolation fraction and current source and listener positions.
The bridge publishes origin shifts and the listener before survival events and loops.
The clock excludes dropped stall time. Playback uses normal simulation seconds, including sleep time scale.
Origin shifts, recovery, camera changes, sleep transitions, and session reset clear velocity sampling.
New voices start with zero velocity until a second position sample exists.
One shared effects pool retains source mixer routes and the selected pitch.

The browser calculates Doppler for playback only. Native toilet gates still use mandatory source duration divided by selected pitch.
The bounded browser pool and event queue remain runtime adapters.
Tests and browser acceptance were waived; source recovery does not establish audible device parity.
