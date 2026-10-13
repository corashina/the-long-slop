# Selected source particles

This record covers Steam build 11140890, Unity 2021.3.18f1.
The accepted source library remains read-only.

## Evidence

UnityPlayer.dll SHA256:
`553c1e7560e1b42bd4638c53d0168dcd1b4b9eb5b9df324b0a44f14b3f18bc86`.
Assembly-CSharp SHA256:
`9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.

Keep raw evidence in `.cache/survival-weather/particles-evidence/`:

- `shape-type-native.log` and `shape-enabled-native.log` preserve the controller's native field checks.
- `task8b-native.log` covers ShapeModule::Start and the managed simulation-space getter.
- `task8b-shape-native.log` covers StartHemiSphere, EmitterStoreData, and GetSimulationSpace.
- `task8b-source.log` preserves initial selected particle records.
- `task8b-active.log` records object activity, material keywords, and source quality settings.
- `task8b-export.log` records final emitter-relative poses and selected module identities.

ShapeModule type 2 means Hemisphere. Type 4 means Cone; ConeVolume is 8.
The [Unity 2021.3 enum source](https://raw.githubusercontent.com/Unity-Technologies/UnityCsReference/2021.3/Modules/ParticleSystem/Managed/ParticleSystemEnums.cs) defines these values.
The native shape getter at RVA 0x1a55f0 returns data+0x220 without remapping.
ShapeModule::Transfer stores `type` at module+0x10.
ShapeModule::Start at RVA 0xed7c40 dispatches the selected hemisphere branch to StartHemiSphere.
StartHemiSphere at RVA 0xecbbb0 uses the radius and cubed inner radius from `1-radiusThickness`.
Its volume sample takes a cube root. Hemisphere angle and length are dormant.
EmitterStoreData at RVA 0xed6200 adds a random unit vector times randomPositionAmount.
The selected amount is 1 for storms and 0 for fluid outlets.
The native path then transforms and normalizes the direction, including shape scale.
ParticleSystem::GetSimulationSpace at RVA 0xab4dc0 returns the stored value.
The selected value 2 means Custom, not Local.

## Owners and values

| Role | Particle source | Capacity | Active source modules |
|---|---|---:|---|
| Large storm | sharedassets1.assets:128154 | 4000 | Initial, Emission, Shape, Force, Rotation, Size, Color, Trigger |
| Small storm | sharedassets1.assets:128153 | 4000 | Initial, Emission, Shape, Force, Rotation, Size, Color, Trigger |
| Urine | level1:58924 | 10000 | Initial, Shape, Size, Color, Collision |
| Urine splash | level1:59006 | 500 | Object inactive; authored modules remain dormant |
| Container outlets | Eleven opening-linked systems | Source capacity per opening | Initial, Shape, Size, Color, Collision |
| Container splashes | Eleven opening-linked systems | Source capacity per opening | Object activity gates authored modules |

The large storm has radius 750, lifetime 2–20, size 50–500, and speed 2–50.
The small storm has radius 60, lifetime 2–5, size 5–30, and speed 1–5.
Both shapes use scale [2,1,1], thickness 1, and zero relative translation.
Both emitter rotations convert to [0.7071068,0,0,0.7071068].
The exporter removes the prefab position [-3.71,-20,14.26].
The model scales shape radius by storm size. It retains source time and distance emission multipliers.
Large particles use pre-Move life strength. Small particles use the current distance influence.
The small emitter uses the source player prediction expression and yoffset.
Custom simulation space keeps existing particles fixed when an emitter moves.
Absolute storm and splash coordinates survive world-origin changes.
The existing FluidContact owner rebases its local trajectories with the physics world.

The model evaluates selected constant, two-constant, curve, and two-curve fields.
It uses source gradient keys, force ranges, rotation rates, and size curves.
Each particle retains its random curve selection during its lifetime.
Visual random streams do not consume storm gameplay draws.
Storm destruction removes its model particles. Render slots become empty and can serve a later storm.
New game and Load replace simulation owners and clear old visible batches.
Pause accepts no simulation step and emits no particles.

Container trajectories remain in FluidContact. Rendering reads their current positions, sizes, colors, and rotations.
The player uses that same owner for urine, with the source cone, gravity, collision mask, bounce, and dampening.
Collision lifetime loss is 0.5 for the selected urine source.
Source fluid colors blend by mixture fraction. The urine outlet then applies its source forced-color blend.
The source player positions urine at the body capsule center and aims toward HlookPoint plus up*0.25.
The model uses the existing capsule, view ray, source use mask, and source reach for that pose.
The existing player head/crouch model remains an adapter to Unity's smooth head transforms.
ToiletOutput owns the bladder debit. Particle creation performs no additional debit.
Mouth and urine share tank level1:73403. Source Tankolas excludes effects for this same-tank contact.

Urine splash GameObject level1:12756 is inactive. Its particle source is level1:59006.
Urine source level1:58924 has SubModule disabled.
Managed source only assigns `sps.main` and customSimulationSpace; no supported Emit or activation link exists.
The model therefore retains the splash owner and authored records without making a visible collision effect.
This supersedes the initial interpretation of its one-particle startup burst.

## Rendering and limits

The renderer uses one static quad mesh and fixed batches of 128 particles.
Each batch has a retained uniform buffer, mesh handle, material handle, and entity.
Normal rendering copies values into those buffers. It does not rebuild meshes or source textures.
The source texture owner supplies decoded mip chains and the same source sampler.
Each particle material binds one image and one sampler.
The shader uses source texture transforms, tint, color, alpha blending, billboard size limits, and existing fog values.
The shader derives each view's fog projection from its current clip matrix. It adds no sky, light, cloud, or audio owner.
The renderer sorts storm particles using the current camera and each batch center.
The adapter maps source sorting fudge to negative Bevy depth bias.
The current pass order subtracts this bias from negative squared camera distance.
This preserves the selected fluid bias (-1). Native particle cross-batch distance arithmetic remains unverified.
Different transparent batches can still overlap in a different order from Unity's native particle renderer.

| Source | Unsupported or adapted setting | Visible consequence |
|---|---|---|
| Both storm systems, material sharedassets1.assets:50 | Legacy soft particles; source Ultra has softParticles=true and _InvFade=1 | Dust keeps hard depth intersections with scene geometry. |
| Urine and all opening-linked systems using material sharedassets1.assets:481 | EFFECT_BUMP / _NORMALMAP; _DistortionEnabled=1, strength=50, scaled strength=5, blend=0.1 | Fluid sprites do not refract the scene background. |
| Both storm TriggerModules | Native collider overlap uses retained shelter AABBs and source radiusScale=0.75 | Curved or rotated shelter edges can remove different particles. |
| Selected fluid CollisionModules | Rapier swept spheres replace Unity's native particle collision cache | Contact timing, repeated contacts, and bounce paths can differ. |
| Selected fluid CollisionModules | Generic world-tank filling has no selected contact routing | Urine does not fill other world tanks through particle hits. |
| Selected ShapeModules | Deterministic scalar random sampling replaces Unity's SIMD random sequence | Individual positions, directions, colors, and lifetime choices differ. |
| Selected Initial, Force, Rotation, Size, and Color modules | Fixed-step scalar evaluation replaces native particle integration | Sub-frame emission placement and interpolation can differ. |
| Selected renderers | Retained camera-facing batches replace Unity particle draws | Cross-batch transparency order can differ. |

I checked the current camera and material paths before declaring the material limits.
SourceMaterial disables prepass and removes Bevy transmission bindings above slot 12.
The current path exposes neither a sampled scene-depth image nor a copied scene-color image to these materials.
The particle shader does not invent a blur, opacity adjustment, or substitute distortion.
Particles/Standard Unlit has _SoftParticlesEnabled=0, so fluid soft-depth fading is dormant.
No other selected active module is omitted.

Compilation and source validation do not establish matched runtime appearance or native random parity.
The user waived tests and browser acceptance for this work.
