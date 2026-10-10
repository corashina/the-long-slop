# Complete asset library

Date: 2026-10-10, Europe/Warsaw.
Status: The user approved the library design and requested this specification and plan together.
The user approved execution and selected subagents on 2026-10-10. Implementation follows the written plan.

## Intent and scope

Provide the installed game's recoverable assets for development in My Summer Slop.
Cover Steam app `1017180`, build `11140890`, Unity `2021.3.18f1`.
Include models, textures, audio, UI, fonts, animations, materials, scenes, and their dependencies.
Preserve the complete installed source before converting its assets.
Use the existing Python exporter, UnityPy, Pillow, and pygltflib.

Keep the current garage, fixed Lada, preparation items, and driving loop usable during library work.
Asset extraction does not add gameplay, audio playback, animation playback, or other vehicles to that loop.
The material and UI specifications continue to control their runtime implementations.
This specification controls library coverage and source provenance.
It corrects the UI font acquisition rule described below.

Scope includes assets packaged in this installed build, including inactive objects and unused variants.
It excludes assets absent from this installation, user downloads, and other Steam branches.
Original editor source, original shader source, and removed build data cannot be recovered by declaration.
Retain the shipped representation and document these distinctions.

## Evidence and baseline

Use the [asset audit](../../research/2026-10-10-full-asset-audit.md) as the baseline.
Its nine serialized files contain 262,784 objects.
Its payload checks establish decoding feasibility, not completed file export or browser parity.

| Type | Source records | Required library result |
| --- | ---: | --- |
| Texture2D | 1,035 | 981 stored images; 54 explicit source-empty font texture records |
| Mesh | 2,934 | 2,912 stored meshes; 22 explicit source-empty records |
| Material | 897 | Complete properties, texture links, shader links, and render state |
| AudioClip | 474 | Decoded WAV files, original stream links, and clip settings |
| Sprite | 155 | Decoded images, atlas links, crop, pivot, geometry, and borders |
| Font | 33 | 31 embedded files; two explicit records without font bytes |
| AnimationClip | 42 | Decoded curve data, bindings, events, and source settings |
| AnimatorController / Avatar | 10 / 13 | Typed records and resolved clip, rig, and state links |
| Cubemap | 2 | Six stored faces per cubemap and source settings |
| Shader / ComputeShader | 118 / 16 | Shipped bytes, parsed metadata, and explicit runtime support status |
| TextAsset | 2 | Exact payloads and text decoding metadata |

Also cover all GameObjects, components, colliders, physics settings, particles, lights, probes, cameras, and UI transforms.
Include the ten language files, `musicbox.txt`, `news.txt`, built-in resources, and managed assemblies.
Archive installed native files as source evidence. They remain outside browser runtime inputs.

For this fixed build, unexpected count changes must stop acceptance and identify the changed file or category.
Verify counts from a fresh scan. Do not use the audit JSON as an export input.

## Completion contract

Report three separate measurements:

| Measurement | Meaning | Required result |
| --- | --- | --- |
| Source preservation | Original installed files and serialized object bytes remain available | 100%, with verified file hashes and object byte ranges |
| Development availability | Asset payloads and dependency records have usable exported representations | 100% of the defined category requirements, with no unexplained gaps |
| Browser support | The selected runtime can load and use an asset's required behavior | Verified per asset; no full-game support claim |

Source-empty records must carry measured dimensions, counts, payload sizes, and their source IDs.
Do not create replacement pixels, geometry, fonts, or curves to fill these records.
Preserve runtime-created resources and the settings or code links that create them.
A source-empty object counts toward inventory coverage, not toward stored-image or stored-mesh totals.

Engine-specific records remain development assets through typed records and preserved native data.
This category includes compiled shader programs, animator logic, humanoid muscle tracks, and runtime render targets.
Do not label these records as browser-ready without a verified adapter.
An engine-specific classification cannot conceal a failed required decode, missing file, or broken source pointer.

The accepted library must contain no failed required conversions or unresolved non-null asset references.
Missing stored animation curves block development availability even when raw clip bytes remain archived.
Each acceptance receipt must state which measurement passed.

## Storage and entry points

Keep one export entry point: `python -m tools.reference.export`.
Retain `--steam-root` and `--output`; `--output` selects the runtime reference directory.
Add `--stage source|catalog|library|runtime` and `--verify`.
The default stage is `runtime`, which runs all prerequisite stages.
`--verify` checks an accepted stage without altering its files.
Stage receipts describe completed layers; they do not select alternate export formats.

| Path | Contents | Browser startup use |
| --- | --- | --- |
| `.cache/reference-library/source/` | Complete installed directory tree | None |
| `.cache/reference-library/steam-manifest.acf` | Copied Steam manifest evidence | None |
| `.cache/reference-library/source-manifest.json` | Original paths, sizes, SHA-256 hashes, build identity | None |
| `.cache/reference-library/catalog.json` | Complete object index, file identities, byte ranges, and dependency edges | None |
| `.cache/reference-library/records/` | Typed native records and managed analysis outputs | None |
| `.cache/reference-library/work/` | Unaccepted stage outputs | None |
| `web/assets/library/` | Converted assets, category indexes, coverage report, and `manifest.json` | Development access only |
| `web/assets/reference/` | Selected runtime glTF, records, buffers, images, and acceptance manifest | Current game selection |

Ignore recovered files and decompiled code in Git. Track exporters, tests, schema definitions, and documentation.
Use one current schema. Reject obsolete schemas rather than adding migrations or compatibility paths.
Use source IDs in output paths. Keep original names as labels.
Use deterministic ordering and hashes. Repeat exports from unchanged source must produce identical accepted files.

## Components and data flow

Keep focused modules under `tools/reference/library/`.

| Module | Responsibility |
| --- | --- |
| `types.py` | Shared identity, catalog, conversion, and receipt records |
| `archive.py` | Snapshot installed files, reject unsafe paths, and verify source hashes |
| `inventory.py` | Discover serialized files, stream files, object identities, and byte ranges |
| `records.py` | Serialize typed source data and resolve object and stream dependencies |
| `images.py` | Decode textures, sprites, cubemap faces, and embedded fonts |
| `audio.py` | Export audio payloads and source playback records |
| `models.py` | Export complete geometry, hierarchies, skins, morph data, and LOD records |
| `animations.py` | Decode clip tracks and export animation, Avatar, and controller links |
| `ui.py` | Recover canvas ancestry, layouts, text, images, fonts, and interaction references |
| `managed.py` | Index archived assemblies and record ILSpy analysis provenance |
| `reader.py` | Read accepted decoded assets and category indexes for runtime selection |
| `validation.py` | Check files, graph closure, conversions, category totals, and stage readiness |
| `__init__.py` | Run stages and publish their receipts |

Keep `tools/reference/source.py` responsible for source reads and generated type-tree repairs.
Keep `gltf.py` responsible for shared glTF primitives and the selected runtime scene writer.
Extract shared geometry helpers within that module when needed. Do not apply startup selection to library models.
Keep `manifest.py` responsible for the selected game's runtime contract.

Process the archive after snapshot verification. Avoid mixing reads from live Steam files and the archive.
Load `.resS`, `.resource`, and built-in resource files explicitly.
Cache generated type trees and object identities. Release decoded payloads after writing and validation.
Do not retain the complete decoded image, audio, or component dataset in memory.

## Source snapshot and identity

Validate the Steam manifest before copying. Copy the installation into staging, then verify the copy.
Include the executable, engine files, data files, managed assemblies, resources, plugins, and authored text.
Record paths relative to the installed game root. Record the Steam manifest as separate evidence.
Compare source file lists and hashes before and after copying. Reject concurrent Steam updates or file changes.
Promote the verified source snapshot only after its files match.

Resolve source and destination paths before any recursive replacement.
Reject destinations inside Steam, filesystem links that leave the installation, and paths that leave the output root.
Reject ambiguous normalized paths and case collisions.
Estimate output space from source sizes and decoded payload sizes. Report estimates and actual bytes.
Keep the accepted game export intact when copying, decoding, or staging fails.

An object identity is `{file, path_id}`.
`file` is the normalized container path relative to `TheLongDrive_Data`, using forward slashes.
Thus `level1` remains `level1`; built-in files use `Resources/unity default resources` and `Resources/unity_builtin_extra`.
Resolve PPtr file IDs through the owning serialized file's external-reference table.
Never resolve a pointer through an object name or a convenient matching path ID.

Record each object's type, byte offset, byte size, source hash, decoded-record path, and dependency list.
Validate object and streamed-payload byte ranges against their archived files.
Retain original bytes through verified archive ranges rather than duplicating every object into separate binary files.

## Typed records and dependency closure

Decode native records through UnityPy. Decode MonoBehaviours through the existing generated type-tree repairs.
Verify complete byte consumption and script/header pointers.
Decode the full custom-component set, including UI and third-party assemblies.
Represent binary fields as hashed sidecar payloads. Preserve ordered pairs and repeated fields without flattening them into dictionaries.
Encode non-finite scalar values with explicit tags and retain their original bytes.
Require finite values in converted glTF and runtime records.

Build dependency edges from typed PPtr fields, streamed ranges, scene hierarchy, and script references.
Keep null pointers as explicit null fields. Do not report them as missing objects.
Allow cycles in ordinary asset references. Reject transform parent cycles and invalid hierarchy ownership.
Retain active flags, renderer flags, enabled components, and all authored LOD levels.

Index the complete item database and terrain configuration, including buildings, cars, parts, and spawn variants.
Index Resources names and source-linked dynamic asset creation where the shipped data establishes those links.
Preserve original managed assemblies and index their types, methods, and asset-related calls.
Use existing ILSpy `11.1.0.9782` for complete `Assembly-CSharp.dll` decompilation and required behavior references.
Store decompiler version, input hash, diagnostics, and output hashes.
Treat decompiled code as analysis data. Do not compile it into the Rust game.
Record unproven dynamic-code links as explicit analysis limits, not resolved dependencies.

## Images, materials, and fonts

Export stored Texture2D images without resizing or baking material-specific factors into the source image.
Record dimensions, source format, color space, sampler settings, mip count, and original stream range.
Preserve original mip payloads in the source archive. Record regenerated browser mip behavior as a conversion difference.
Separate source pixels from shader-specific derived images, including packed-normal decoding and metallic/smoothness channel maps.
Use distinct conversion identities when one source texture serves different channel or color-space roles.

Export sprite images and retain original atlas, texture rectangle, packing rotation, mesh outline, pivot, and nine-slice borders.
Export cubemaps as six labeled faces. Preserve Unity face order and record the browser orientation conversion.
Retain complete material keywords, texture environments, UV channels, factors, passes, queues, blend, cull, and depth settings.
Link materials to their original shaders and all texture dependencies.
The existing material-parity specification controls selected-runtime material adapters.

Export the 31 embedded font files byte-for-byte. Validate file signatures and face metadata.
Keep both font records without embedded bytes and their texture/glyph dependencies.
Include TextMeshPro font records, atlas data, glyph tables, fallback links, and source settings.
Use embedded Liberation Sans Regular from `sharedassets0.assets:769` for the pending source UI work.
Its file contains 350,200 bytes; SHA-256 is `e5b0af421ea2bfbc1ac8d251d647268087ae82786234c57f757d1f0b90fa8b49`.
Retain applicable attribution and license evidence with each recovered font.
Upon written-spec approval, this acquisition rule supersedes the UI specification's upstream-download rule.
Its layout and font-rendering requirements remain unchanged.

## Models and scenes

Export every stored Mesh, including meshes outside the current runtime selection.
Keep positions, normals, tangents, available UV channels, colors, topology, submeshes, bounds, weights, and bone indices.
Retain bind poses and authored blendshape frames and weights.
Export supported glTF morph targets and their source-to-target map.
Keep topology that glTF cannot represent in typed native records and report the conversion limit.

Export authored model hierarchies with materials, skeletons, skins, and all LOD records.
Retain inactive nodes, disabled renderers, source transforms, and components.
Keep scene placement separate from reusable model geometry.
Include skeleton ancestors required by a skin, even when they fall outside the selected visual subtree.
Bind bones through source identities and hierarchy paths. Reject missing bones and mismatched bind-pose counts.
Export orphan meshes as mesh assets with their source rig metadata; do not invent skeleton nodes.

Apply the existing Unity-to-glTF handedness conversion once.
Keep signed scale, triangle winding, tangent handedness, inverse bind matrices, and animation coordinates consistent.
Library conversion must not randomize condition, choose the highest LOD, suppress editor glyphs, or override material assignments.
Apply these source startup rules only when deriving the selected runtime scene.

## Animations and controllers

All 42 installed clips store animation through `m_MuscleClip.m_Clip`.
The inspected clips have no ordinary position, rotation, compressed-rotation, or float curves in their top-level arrays.
UnityPy object parsing exposes `m_MuscleClip.m_Clip.data` as a typed `Clip`.
Decode its streamed, dense, and constant data. An empty top-level curve array does not mean an empty clip.

Use UnityPy's parsed clip structures and existing binary readers.
Implement only the missing curve reconstruction for this pinned build.
Use the maintained [AssetRipper curve converter](https://github.com/AssetRipper/AssetRipper/blob/master/Source/AssetRipper.Processing/AnimationClips/AnimationClipConverter.cs) as a format reference.
Record the reference revision and attribution during implementation. Do not introduce a second runtime engine.

Export normalized tracks with binding type, path hash, resolved target, property, key times, values, tangents, and interpolation.
Preserve sample rate, clip bounds, wrap mode, duration, root motion, loop settings, events, and object-reference curves.
Recover path hashes from source rig/Avatar tables and verify matches against the owning hierarchy.
Do not choose the first path when hashes collide.
Export generic Transform and supported morph tracks to glTF animation channels.
Retain material, object-reference, event, and humanoid muscle tracks in typed animation records.
Humanoid retargeting and Animator execution require separate runtime adapters; the library must still decode their source tracks.

Preserve native quaternion curves alongside glTF output.
Require rotation error at most `0.01` degrees at authored keys and interval quarter, midpoint, and three-quarter checks.
Require translation and scale interpolation error at most `0.00001` source units at those checks.
Subdivide converted intervals until they pass. Reject zero-length quaternions and channels that cannot meet these limits.
Keep source step interpolation as steps. Do not smooth discrete animation keys.

Export all 13 Avatars and ten AnimatorControllers with their original rig, state, transition, parameter, and clip links.
Retain avatar rest poses and skeleton mappings. Record root motion separately from node transforms.
Do not claim complete clip conversion from a record parse alone.

## Audio and UI

Decode all 474 audio clips to WAV through UnityPy's installed audio converter.
Retain clip channel count, frequency, duration, load settings, compression metadata, and original byte ranges.
Keep sample filenames under their clip identity to prevent name collisions.
Retain AudioSource settings, loops, spatial values, curves, mixer groups, snapshots, and clip links.
Decode one clip at a time. The audit's decoded audio totals 2,030,938,742 bytes.
This stage adds no gameplay audio system or audio preload to the current browser game.

Export the full source UI, including title art, menu variants, inactive panels, controls, icons, and text.
Preserve Canvas and CanvasScaler settings, RectTransform ancestry, anchors, pivots, offsets, scale, borders, and masks.
Preserve typography, font links, text content, alignment, wrapping, colors, outlines, and shadows.
Retain button targets, navigation, animation links, and source event references as data.
Keep source layout values and derived browser layout values separate.
Convert bottom-origin browser layout coordinates once. Resolve scaling for each canvas.
Retain multilingual source files without adding a localization system to the product.
Keep My Summer Slop's displayed title under the existing runtime UI specification.

## Engine records and conversion limits

Preserve shader and ComputeShader program bytes, parsed properties, variants, and material links.
Do not treat UnityPy diagnostic shader text as executable shader source.
The four known text-export failures must retain their full native representation and an explicit diagnostic.
These IDs are `globalgamemanagers.assets:3`, `:14`, `:16`, and `sharedassets0.assets:714`.
Recover all available shader metadata even when its text exporter fails.
Missing required material metadata blocks library acceptance. Missing original HLSL does not justify invented source.

Preserve lighting, reflection, particle, physics, camera, and project settings as typed records and dependencies.
Preserve the seven RenderTexture descriptors and their source creation links.
A runtime camera image is generated output, not a stored image available for extraction.
Keep renderer support limits in the coverage report and selected-runtime adapters.

## Validation and publication

Each category index records source IDs, output files, hashes, byte lengths, status, dependencies, and conversion diagnostics.
Use status values `converted`, `native_record`, `source_empty`, or `failed`.
Use runtime values `verified`, `engine_dependent`, or `unverified`.
Require an explanatory reason for `native_record` and `source_empty`.
The library manifest contains schema version `1`, build identity, source-manifest hash, catalog hash, category indexes, and coverage totals.
The runtime manifest gains required library provenance. Validate provenance and dependency closure during offline selection.
Do not make browser startup download the full library catalog or unused library assets.
Reuse accepted mesh accessors and decoded image files during runtime derivation.
Read archived source rules for startup behavior; do not decode the full payload corpus again.

Write category outputs into staging. Check all required conversions, pointers, file hashes, and counts before publishing acceptance.
Publish the complete library manifest last. Never replace an accepted receipt with a partial-success receipt.
Build runtime output in staging and promote its complete directory after validation.
A failed promotion must preserve or restore the previous accepted runtime directory.
Validate directories before replacement. Report Windows file-lock errors without deleting an accepted directory.

Use critical tests for source integrity, identity collisions, graph closure, source-empty records, channel conversion, rigs, and animation decoding.
Check WAV headers and sample metadata; check font files and sprite geometry against their source records.
Validate every exported glTF file and its binary, texture, skin, morph, and animation references.
Compare curve samples at endpoints and interior times with reconstructed source curves.
Check complete installed-build counts and verify every output hash.

Build and inspect the current garage after selected-runtime derivation.
Confirm that startup requests remain limited to selected assets and that the full library adds no per-frame work.
Use temporary development fixtures for media and model loading checks; keep them outside the product UI.
Validate animation data through curve sampling and glTF checks. Claim browser animation support only after a playback check.
Preserve the user's waiver for the final continuous garage-to-road browser test.
Record browser capture limits and unverified adapters in `docs/verification.md`.

## Delivery order and records

Deliver working stages in order: source snapshot; catalog and graph; decoded media; models; animation data; validated library; selected runtime.
Keep the accepted game output usable until the final selected-runtime promotion.
Add each stage's critical checks before proceeding to the next stage.
Update `tools/reference/README.md`, `README.md`, `DESIGN.md`, and `docs/verification.md` with commands, counts, and measured limits.
Update the source UI specification and plan only for the recovered Liberation Sans acquisition rule.
Keep gameplay exclusions in `PRODUCT.md`; describe library availability separately.
Commit code and documentation. Keep recovered original assets, assemblies, captures, and decompiled outputs outside Git.
