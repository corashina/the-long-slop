# Complete asset library audit

Target: Steam app `1017180`, build `11140890`, Unity `2021.3.18f1`.
Final verification date: 2026-10-11, Europe/Warsaw.

The accepted library preserves the installed source and covers all 262,784 serialized objects.
Its 721,957 dependency links contain no unresolved required non-null reference.
Required conversions contain no failed result. Source-empty and native records retain explicit reasons and identities.
The game still selects the garage, fixed Lada, and preparation items.

## Accepted storage

| Layer | Files | Bytes |
| --- | ---: | ---: |
| Source archive | 183 | 529,237,089 |
| Typed records and managed analysis | 268,954 | 698,121,145 |
| Published development library | 8,570 | 3,105,818,196 |
| Selected runtime | 89 | 73,261,688 |

The archive includes managed assemblies, built-in resources, resource streams, language files, and source text.
The earlier data-directory-only inventory counted 160 files. The complete archive includes additional installed files.
The selected runtime uses 182 source meshes and 79 source textures.
Its scenes contain 434 home nodes, 697 Lada nodes, and 98 item nodes.
It retains 417 colliders and 73 gameplay items.

## Development coverage

| Category | Converted | Native record | Source empty | Total |
| --- | ---: | ---: | ---: | ---: |
| Animations | 16 | 49 | 0 | 65 |
| Audio | 474 | 0 | 0 | 474 |
| Images | 1,169 | 2 | 54 | 1,225 |
| Models | 3,462 | 0 | 22 | 3,484 |
| Native | 0 | 241,391 | 0 | 241,391 |
| Ui | 0 | 16,145 | 0 | 16,145 |
| Total | 5,121 | 257,587 | 76 | 262,784 |

The source-empty set contains 54 Texture2D records and 22 Mesh records.
The fonts `kongtext` and built-in Arial have no embedded bytes. Their native records retain font dependencies.
Native status can include useful decoded sidecars, such as normalized animation tracks and source text.
It does not mean that the browser can execute Unity behavior.

| Source type | Records |
| --- | ---: |
| Texture2D | 1,035 |
| Mesh | 2,934 |
| Material | 897 |
| AudioClip | 474 |
| Sprite | 155 |
| Font | 33 |
| AnimationClip | 42 |
| Avatar | 13 |
| AnimatorController | 10 |
| Cubemap | 2 |
| Shader | 118 |
| ComputeShader | 16 |
| TextAsset | 2 |

## Indexes and diagnostics

Read the local category indexes after export:
[images](../../web/assets/library/images/index.json), [models](../../web/assets/library/models/index.json),
[audio](../../web/assets/library/audio/index.json), [animations](../../web/assets/library/animations/index.json),
[UI](../../web/assets/library/ui/index.json), and [native](../../web/assets/library/native/index.json).
Read [coverage.json](../../web/assets/library/coverage.json) for all source counts, native types, and source-empty counts.
Category `status`, `reason`, `runtime`, and `details` fields identify each conversion and adapter limit.
Typed records live under `.cache/reference-library/records/`; their paths use source container names and object IDs.

The [managed diagnostics](../../.cache/reference-library/records/managed/diagnostics.json) retain unresolved declarations and optional shader-text failures.
Four optional shader text exports failed: `globalgamemanagers.assets:3`, `:14`, `:16`, and `sharedassets0.assets:714`.
The library preserves their original records and byte ranges. These optional diagnostics are not required conversion failures.
The other shader text exports do not establish working browser shaders.

Managed indexing covers 125 DLLs, 21,092 types, and 201,872 methods.
The accepted decompilation covers all 637 `Assembly-CSharp` types and 3,124 methods without decompiler diagnostics.
The source assembly SHA-256 is `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
The analysis records 790 asset calls. Proven links include two Resources.Load links and six controller assignments.
It retains 815 unmatched, unused MonoScript declarations: 119 reference absent assemblies; 696 lack exact declared types.
These limits do not establish additional runtime asset bindings.

## Media, models, and animation

The library contains 981 decoded Texture2D images, 155 sprites, and 31 embedded font files.
It contains 474 WAV clips with 2,030,938,742 total bytes.
The source includes embedded Liberation Sans Regular at `sharedassets0.assets:769`.
Its 350,200 bytes retain SHA-256 `e5b0af421ea2bfbc1ac8d251d647268087ae82786234c57f757d1f0b90fa8b49`.
The TMP asset's null source-font pointer does not establish absent font bytes.
The pending source UI specification now names the recovered font. Current shell typography stays unchanged.

Model validation checks geometry, materials, rigs, bind poses, bone weights, and referenced buffers.
It preserves all authored hierarchy and LOD records. The game selects its existing high-detail runtime subset.
The installed corpus contains no morph channels. Synthetic fixtures validate morph deltas and glTF channels.
All 42 clips have validated normalized tracks and source curve samples.
Sixteen clips have bound glTF packages; 26 retain native records and normalized data with binding limits.
The animation category also retains 13 Avatars and ten AnimatorControllers.
Browser animation playback and humanoid retargeting remain unverified.

## Browser support and limits

The extraction manifest records 257,643 engine-dependent and 5,141 unverified assets.
These immutable extraction receipts do not claim browser verification.
The isolated fixture results in [verification](../verification.md) cover representative files only.
Bevy loaded representative static and skinned glTF assets with all dependencies ready.
Chrome displayed the static cup. Skinned rendering and animation playback remain unverified.
Chrome loaded Liberation Sans, decoded a WAV, and reached the WAV's muted playback end.
It decoded six cubemap PNG previews. The native BC6H DDS faces preserve HDR data and mip levels.
Browser HDR cubemap rendering remains unverified.
The synthetic rotated sprite matched six expected pixels. No installed sprite has nonzero packing rotation.

The current garage requested only its selected runtime assets. No full-library or catalog preload occurred.
The bounded Chrome readiness check remained loading during synchronous ANGLE/NVIDIA shader linking.
A 40.256-second CPU profile spent 38.832 seconds, about 96.5%, in WebGL program-link status checks.
Prior payloads reproduced that stall. The evidence does not justify an exporter correction.
Garage readiness and the user-waived continuous garage-to-road route remain unverified.

Unity shaders, particles, live reflections, lighting adapters, native UI behavior, and game audio need separate implementation.
Library availability does not add those features or expand [product scope](../../PRODUCT.md).

## Provenance and reproduction

| Receipt | SHA-256 |
| --- | --- |
| Source manifest | `254830925305facc97113479ecdea6f154389ed354afa42aa996ead492d5c04d` |
| Catalog | `ba572df2c46e84c2839c805655c252acb3ef8964a776c2eeb378bb3de0b1b13f` |
| Typed manifest | `f046979a98796cb4b63d968ae23caf6d94478b7d82e8110cb89925dec0470d95` |
| Library manifest | `92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99` |
| Runtime manifest | `86381f84bddc44f019dd2103c29aa94a980437ba88434bd601773558859d412a` |

Converter receipts record UnityPy 1.25.4, Pillow 12.3.0, and pygltflib 1.16.5.
Task 8's repeat publication matched all 8,570 paths, lengths, and hashes.
That comparison repeated assembly, semantic validation, and publication from accepted category outputs.
Earlier category reports hold upstream converter repeat evidence.

Use [the export commands](../../tools/reference/README.md) and [final verification commands](../verification.md).
Runtime derivation and verification require the existing Docker toolchain image and its Cargo volumes.
The native `validate-reference` command uses the same manifest contract as the game.
Library-only verification does not require Docker.
The ignored `task-10-final-proof.json` records final hashes, counts, protection checks, and storage totals.
It lives under `.cache/reference-library/work/`, beside command logs and browser evidence.
