# Full asset audit

Date: 2026-10-10, Europe/Warsaw.
Status: Source inspection complete. The user approved the library design. Full transfer has not started.

## Target

The user wants the installed game's assets available for development in My Summer Slop.
The requested library includes textures, models, audio, UI, and other asset types.
Use installed Steam app `1017180`, build `11140890`, Unity `2021.3.18f1`.
Define coverage against this build. This audit does not cover other Steam builds or user content.

Separate source preservation, decoded files, and browser support when measuring completion.
An archived shader does not establish equivalent Bevy rendering.
An empty source object must retain its identity and an explanation.

## Method and evidence

I checked the Steam manifest, existing exporter, glTF outputs, Rust loader, and cached decompiled classes.
I scanned nine serialized source files with the installed UnityPy `1.25.4`.
I included `Resources/unity default resources` and `Resources/unity_builtin_extra`.
I loaded `.resS` and `.resource` files before repeating payload checks.
I decoded images and audio into memory. I did not write these assets into the project.

Detailed local evidence: [full-asset-audit.json](../../.cache/full-asset-audit.json).
Git ignores this evidence file. The report below retains the results.

## Inventory and project coverage

The source files contain **262,784 serialized objects**.
The game's data directory contains 160 files and 489,130,676 bytes, including managed assemblies.
The current reference export contains 92 files and 76,413,754 bytes.
Its scenes contain 434 home nodes, 697 Lada nodes, and 98 item nodes.

| Asset type | Installed records | Unique source records in current export |
| --- | ---: | ---: |
| Texture2D | 1,035 | 79 |
| Mesh | 2,934 | 182 |
| Material | 897 | 87 |
| AudioClip | 474 | 0 |
| Sprite | 155 | 0 |
| Font | 33 | 0 |
| AnimationClip | 42 | 0 |
| Cubemap | 2 | 0 |
| Shader | 118 | 0 standalone shader exports |
| ComputeShader | 16 | 0 |
| TextAsset | 2 | 0 |

The 83 PNG files represent 79 source textures. Some textures have separate color-space outputs.
The source also contains 70,288 GameObjects, 48,870 MonoBehaviours, and 5,140 RectTransforms.
It contains 317 SkinnedMeshRenderers, 13 Avatars, and 10 AnimatorControllers.
It contains 1,380 ParticleSystems, 683 Lights, 87 ReflectionProbes, and seven RenderTextures.
These records define dependencies and behavior beyond standalone meshes and images.
The data directory also contains ten language files, `musicbox.txt`, and `news.txt`.

## Payload checks

| Type | Result | Limit |
| --- | --- | --- |
| Audio | All 474 clips decode to WAV | Decoded output totals 2,030,938,742 bytes. Browser playback remains untested. |
| Textures | All 981 stored images decode | The other 54 records have zero dimensions and no stored pixels. Their names identify font textures or atlases. |
| Sprites | All 155 sprites decode | Library export must also retain source crop, borders, pivot, and atlas links. |
| Meshes | All 2,934 records parse through MeshHandler | Twenty-two have no vertices. This check does not validate glTF skinning or animation. |
| Animation clips | All 42 serialized clips parse | All use the non-legacy format. Curve conversion and browser playback remain untested. |
| Fonts | All 33 records parse; 31 contain font bytes | `kongtext` and built-in Arial contain no font bytes. |
| Shaders | 114 produce diagnostic text; four fail export | The 114 outputs state that they are invalid shader files. None establish WGSL support. |

MeshHandler reports skin weights in 25 mesh records.
It parses static data in 2,909 mesh records, including the empty meshes.
Preserve skeletons, bind poses, curves, controller links, and any authored blendshape data during model export.
Do not substitute OBJ export for models that require these fields.

The four shader failures have these source IDs:

- `globalgamemanagers.assets:3`
- `globalgamemanagers.assets:14`
- `globalgamemanagers.assets:16`
- `sharedassets0.assets:714`

UnityPy reports unsupported GPU program values or a buffer-length error for these records.
Preserve their original bytes and identify them in the conversion report.
Shader text export cannot serve as a browser shader implementation.

## Source code and dependency gaps

`Assembly-CSharp.dll` contains 637 type definitions, including nested types.
The cache contains 44 decompiled C# files, including four TextMeshPro classes.
This cache does not establish complete decompilation or complete asset dependency tracing.
The inspected game classes include `itemdatabase`, `TerrainGenerationSettings`, and the current gameplay and UI controllers.

Assembly SHA-256:

`9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`

The exporter selects fixed home, car, and item roots through source IDs.
It does not enumerate the complete item database, vehicle variants, buildings, or menu-world content.
Its hardcoded source list omits both built-in resource files.
Its input hashes omit `.resource` audio streams and the built-in resource files.
Its generic PBR conversion omits source material behavior covered by the approved material specification.
Its loader disables imported animations, lights, and cameras.
Cargo currently enables PNG and glTF support but does not enable Bevy audio features.

An audit setup error changed UnityPy's search folder after loading built-in resources.
Explicit stream loading resolved audio and streamed-mesh failures.
The 54 image errors remain because their source records contain no pixel payload.
The exporter must classify these records before requesting an image decode.

## Font correction for the pending UI work

The source includes an embedded Liberation Sans Regular font at `sharedassets0.assets:769`.
Pillow identifies its face as `Liberation Sans`, `Regular`.
The font contains 350,200 bytes.
Its SHA-256 is `e5b0af421ea2bfbc1ac8d251d647268087ae82786234c57f757d1f0b90fa8b49`.

The UI specification identifies a null source-font pointer on the TMP asset.
That pointer does not establish that the build lacks the underlying font file.
The next approved specification should use the recovered font and record this correction.
This audit leaves the existing approved specification unchanged.

## Proposed library design

Extend the current Python exporter with a separate offline asset library.
Keep the current runtime selection as the garage, Lada, and supported items.
Asset availability does not add gameplay outside the current product scope.

Use three output layers:

1. Preserve source containers, streams, built-in resources, managed analysis inputs, and authored text under ignored local storage.
2. Export decoded textures, sprites, audio, fonts, geometry, and typed source records into an indexed development library.
3. Build the browser's selected assets from that library through the existing validated reference manifest.

Store the source archive at `.cache/reference-library/source/`.
Store the full object index and analysis records at `.cache/reference-library/catalog.json`.
Store decoded development assets and their manifest under ignored `web/assets/library/`.
Keep selected runtime outputs under `web/assets/reference/`.
Extend `tools/reference/export.py` as the single export entry point.
Keep inventory, asset conversion, and runtime selection in separate modules.

Use source-relative file paths and object IDs as identities. Treat names as labels.
Preserve prefab and scene hierarchies, all authored LODs, and dependency links.
Keep complete material properties, UI layouts, animation data, audio settings, lighting, and particle records.
Allocate and load only the assets that a browser scene needs.

Give each record explicit status for preservation, decoding, and runtime support.
Record source hashes, output hashes, byte lengths, dependencies, and conversion errors.
Publish the completed library manifest only after validating its files and links.
An unresolved required dependency must stop completion. Do not count failed conversions as usable assets.
Treat source-empty objects and runtime-generated resources as explicit categories.

Build each layer while keeping the current game usable.
First preserve and index the source. Then export images, fonts, audio, and metadata.
Then export complete model hierarchies and animation dependencies.
Validate runtime resources under the existing material and UI specifications.

## Approach comparison

| Approach | Benefit | Cost |
| --- | --- | --- |
| Extend the existing exporter | Reuses tested source IDs, type-tree repairs, glTF conversion, and validation | Requires complete indexing and converters for uncovered asset types |
| Export a recovered Unity project with AssetRipper | Supplies another route to native Unity assets and dependency analysis | Requires a separate conversion stage for Bevy and browser formats |

I recommend extending the exporter. The payload checks support this choice for images, audio, fonts, and meshes.
Animation conversion and shader recovery require further investigation before claiming complete runtime support.
[UnityPy documentation](https://github.com/K0lb3/UnityPy) describes its asset extraction APIs.
[AssetRipper documentation](https://github.com/AssetRipper/AssetRipper) describes native Unity conversion and dependency analysis.
The AssetRipper option has not been tested against this installation.

## Completion criteria for the specification

Count and preserve the installed source files and the complete object inventory.
Produce a conversion result or a verified source-empty classification for each applicable asset.
Report raw preservation, decoded availability, and browser support as separate measurements.
Validate source pointers, resource ranges, output hashes, and model dependencies.
Check decoded audio headers, font files, sprite metadata, glTF structure, and animation bindings.
Keep unresolved shader behavior visible until its browser implementation passes verification.
Do not equate preserved Unity assets with identical rendering, physics, or a completed game port.

The user requested the specification and plan together after approving the library design.
Review the [specification](../specs/2026-10-10-complete-asset-library-design.md) and [plan](../plans/2026-10-10-complete-asset-library.md) before implementation.
