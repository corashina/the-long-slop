# Complete Asset Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax. Wait for document review and execution choice.

**Goal:** Preserve the installed game and export its complete asset library with verified dependencies and conversion results.

**Architecture:** Extend the current exporter with staged source preservation, cataloging, conversion, and validation. Derive the current runtime selection from the accepted library. Keep library storage and browser startup loading separate.

**Tech Stack:** Python 3.12, UnityPy 1.25.4, TypeTreeGeneratorAPI 0.0.10, pygltflib 1.16.5, installed Pillow and audio converter, ILSpy 11.1.0.9782, Rust, serde, Bevy 0.19.1, WebGL2.

**Spec:** [Complete asset library](../specs/2026-10-10-complete-asset-library-design.md).

**Status:** The user requested the specification and plan together. Review and execution choice remain pending. No task has started.

## Global Constraints

- Cover Steam app `1017180`, build `11140890`, Unity `2021.3.18f1`.
- Use the existing Python exporter, UnityPy, Pillow, and pygltflib.
- Keep the current garage, fixed Lada, preparation items, and driving loop usable during library work.
- Asset extraction does not add gameplay, audio playback, animation playback, or other vehicles to that loop.
- Ignore recovered files and decompiled code in Git. Track exporters, tests, schema definitions, and documentation.
- Use one current schema. Reject obsolete schemas rather than adding migrations or compatibility paths.
- Apply the existing Unity-to-glTF handedness conversion once.
- Decode one clip at a time. Do not retain the complete decoded media dataset in memory.
- Keep source preservation, development availability, and browser support as separate measurements.
- The accepted library must contain no failed required conversions or unresolved non-null asset references.
- Preserve the user's waiver for the final continuous garage-to-road browser test.
- Follow `AGENTS.md`. Use stop-slop for documentation and keep instructions within 20 words.

## Review Focus

1. Steam changes during copying: reject the snapshot and preserve the accepted runtime. Task 1 tests this.
2. Different containers contain identical names or IDs: resolve through the owning file, including cyclic references. Task 2 tests this.
3. Empty font textures and meshes resemble decode failures: retain measured source-empty records without replacement payloads. Tasks 3 and 5 test this.
4. Clip headers contain no ordinary curves: reconstruct optimized tracks, binding dimensions, and sentinel frames. Task 6 tests this.
5. A late conversion or directory promotion fails: preserve accepted outputs and withhold new acceptance. Tasks 8 and 9 test this.

## File map

Paths in this map refer to source files. Recovered outputs use the specification's ignored directories.

| Files | Responsibility |
| --- | --- |
| `tools/reference/library/types.py` | Shared source, catalog, output, and stage record types |
| `tools/reference/library/archive.py` | Source snapshot and source-manifest verification |
| `tools/reference/library/inventory.py` | File discovery, object identities, counts, and raw ranges |
| `tools/reference/library/records.py` | Uncached typed records, dependency edges, and native metadata |
| `tools/reference/library/images.py` | Stored images, sprites, cubemaps, and embedded fonts |
| `tools/reference/library/audio.py` | WAV export and clip validation |
| `tools/reference/library/ui.py` | Complete source UI layouts, styles, and dependency roles |
| `tools/reference/library/models.py` | Library meshes, all hierarchies, skins, LODs, and morph data |
| `tools/reference/library/animations.py` | Optimized curves, bindings, normalized tracks, and glTF channels |
| `tools/reference/library/managed.py`, `tools/reference/decompile.ps1` | Managed index and ILSpy analysis outputs |
| `tools/reference/library/reader.py` | Accepted library records, mesh packages, and texture files for runtime selection |
| `tools/reference/library/validation.py`, `__init__.py` | Coverage validation, stage orchestration, and publication |
| `tools/reference/source.py`, `gltf.py` | Shared readers, canonical identities, type-tree repairs, and glTF helpers |
| `tools/reference/export.py`, `manifest.py` | Single CLI and selected runtime derivation |
| `src/reference.rs`, `tests/game/reference.rs`, `tests/game/common/reference.json` | Required library provenance and runtime validation |
| `scripts/serve.mjs`, `tests/web/server.test.mjs` | Development asset MIME types and server checks |
| `tools/reference/tests/test_library_*.py` | Critical library integrity and conversion tests |
| `.gitignore`, `README.md`, `DESIGN.md`, `PRODUCT.md`, `tools/reference/README.md`, `docs/verification.md` | Commands, availability, evidence, and scope |

## Shared interfaces

Define frozen data records in `tools/reference/library/types.py`.
Serialize paths with forward slashes and sort records by canonical source identity.

| Type | Fields |
| --- | --- |
| `ObjectId` | `file: str`, `path_id: int`; container-relative to `TheLongDrive_Data` |
| `FileRecord` | `file: str`, `bytes: int`, `sha256: str` |
| `Snapshot` | `root: Path`, `files: tuple[FileRecord, ...]`, `manifest_path: Path`, `manifest_sha256: str` |
| `ObjectRecord` | `source: ObjectId`, `type: str`, `class_id: int`, `offset: int`, `bytes: int`, `container_sha256: str` |
| `Catalog` | `objects: dict[ObjectId, ObjectRecord]`, `counts: dict[str, int]`, `dependencies: dict[ObjectId, tuple[ObjectId, ...]]` |
| `AssetExport` | `source: ObjectId`, `category: str`, `status: str`, `runtime: str`, `reason: str`, `files: tuple[FileRecord, ...]`, `dependencies: tuple[ObjectId, ...]`, `details: dict` |
| `LibraryReceipt` | `manifest_path: Path`, `manifest_sha256: str`, `catalog_sha256: str`, `source_manifest_sha256: str`, `exports: tuple[AssetExport, ...]` |

`status` accepts `converted`, `native_record`, `source_empty`, or `failed`.
`runtime` accepts `verified`, `engine_dependent`, or `unverified`.
Store only lightweight record descriptions in these collections. Write binary fields and large typed records as sidecars.
`Catalog` dependency keys include every object, including objects with no outgoing edges.
Category indexes reference typed source records rather than duplicate their complete contents.

## Commands and execution setup

Use the worktree skill at execution. Planning does not require a new worktree.
Use the installed Python environment. Keep generated output within the chosen execution workspace.
Verify local tool versions before using them. Resolve missing offline tools at execution without adding runtime packages.

```powershell
& tools/reference/.venv/Scripts/python.exe -m unittest discover -s tools/reference/tests -v
& tools/reference/.venv/Scripts/python.exe -m tools.reference.export --steam-root 'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive' --stage source
& tools/reference/.venv/Scripts/python.exe -m tools.reference.export --steam-root 'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive' --stage library
& tools/reference/.venv/Scripts/python.exe -m tools.reference.export --steam-root 'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive' --stage library --verify
& tools/reference/.venv/Scripts/python.exe -m tools.reference.export --steam-root 'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive'
./scripts/check.ps1
./scripts/build.ps1
node --test tests/web/server.test.mjs
```

The stage options appear during implementation. They are not available in the current exporter.
Each `--verify` invocation must read accepted files without rewriting them.
A lower stage's success establishes only that stage. Default export requires the complete library and selected runtime.
Commit explicit source and test paths after each passing task. Include new files in that task's commit.
Do not stage recovered files, `.cache`, or ignored asset directories.

### Task 1: Preserve and inventory the complete installed source

**Files:** Create `library/types.py`, `archive.py`, `inventory.py`, and `__init__.py` under `tools/reference/`.
Modify `tools/reference/export.py`, `source.py`, and `.gitignore`.
Test: `tools/reference/tests/test_library_source.py`.

**Interfaces:**

- `snapshot_source(steam_root: Path, work_root: Path) -> Snapshot` validates build identity and publishes a verified archive.
- `verify_snapshot(snapshot: Snapshot) -> None` verifies file lists, sizes, and hashes without writes.
- `inventory_source(source: Source, snapshot: Snapshot) -> Catalog` indexes every serialized object and its original range.
- `Source(snapshot.root)` loads discovered serialized files, `.resS`, `.resource`, and both built-in resource files.
- `Source.container_path(reader) -> str` returns the canonical data-root-relative container path.
- Keep `source_id(reader)` and `key(reader)` consistent with that path.
- Store copied Steam manifest evidence beside the snapshot manifest, outside the mirrored installation tree.
- Validate live Steam identity before snapshotting; validate archived identity through `source-manifest.json` afterward.

- [ ] **Step 1: Add source integrity and identity tests.**
  Assert `test_changed_source_rejects_snapshot` leaves the previous accepted runtime files unchanged.
  Assert `test_streams_and_builtins_discovered` includes both built-in files and `.resource` audio streams.
  Assert `test_escape_and_case_collision_rejected` rejects an external filesystem link and duplicate normalized paths.
  Assert `test_duplicate_path_id_in_distinct_containers` creates two distinct object identities.
- [ ] **Step 2: Run the tests and confirm failure before implementation.**
  Run `python -m unittest tools.reference.tests.test_library_source -v` through the installed environment.
  Expect missing interfaces or assertions to fail.
- [ ] **Step 3: Implement the snapshot and inventory interfaces.**
  Use streaming SHA-256 and checked Windows paths. Compare source hashes and lists before and after copying.
  Preserve the installation structure. Estimate required space and report copy failures with their paths.
  Register resource streams before decoding. Replace the hardcoded seven-file load list with verified discovery.
  Add `/web/assets/library/` to `.gitignore` and wire the source stage into the CLI.
  Keep the current accepted runtime untouched during source staging.
- [ ] **Step 4: Run source tests and the real source stage.**
  Expect matching copied hashes, nine serialized files, and exactly 262,784 objects for this build.
  Run the source stage with `--verify`; expect no file writes and the same counts.
- [ ] **Step 5: Commit the source stage.**
  Use commit message `Preserve complete installed asset source`.

### Task 2: Export typed records and close the dependency graph

**Files:** Create `tools/reference/library/records.py`.
Modify `source.py`, `library/inventory.py`, and `library/__init__.py`.
Test: `tools/reference/tests/test_library_records.py`.

**Interfaces:**

- `Source.read_record(reader) -> dict` performs verified decoding without retaining the result in the full-source data cache.
- `write_record(source: Source, reader, records_root: Path) -> FileRecord` writes canonical typed JSON and binary sidecars.
- `resolve_dependencies(source: Source, reader, data: dict) -> tuple[ObjectId, ...]` resolves typed PPtrs and stream ranges.
- `build_catalog(source: Source, snapshot: Snapshot, records_root: Path) -> Catalog` fills every object's dependency entry.
- `validate_graph(catalog: Catalog) -> None` checks non-null targets, byte ranges, and transform ownership.
- Export the catalog stage only after all required record reads and graph checks pass.

- [ ] **Step 1: Add graph closure tests.**
  Assert `test_external_pointer_uses_owner_table` resolves an external pointer to its declared container.
  Assert `test_null_and_cyclic_asset_pointers` accepts null fields and an A-to-B-to-A material graph.
  Assert `test_missing_pointer_and_transform_cycle_fail` rejects missing non-null targets and cyclic transform parents.
  Assert `test_binary_and_nonfinite_fields_preserved` retains ordered pairs, binary sidecars, and tagged non-finite scalars.
  Assert `test_custom_header_and_complete_read` rejects a mismatched MonoBehaviour header or incomplete byte consumption.
- [ ] **Step 2: Run the record tests and confirm failure.**
  Run `python -m unittest tools.reference.tests.test_library_records -v` through the installed environment.
- [ ] **Step 3: Implement typed record and graph export.**
  Reuse `normalize_arrays` and existing header repairs. Keep generated type-tree caching separate from payload caching.
  Resolve PPtrs using field types and owner tables. Do not infer pointers from arbitrary dictionary names.
  Preserve hierarchy, inactive flags, script links, item database fields, and source engine settings.
  Validate every archived object and streamed byte range against its container.
- [ ] **Step 4: Validate the real catalog stage.**
  Expect 262,784 object records, complete graph keys, no failed required reads, and no unresolved non-null references.
  Report parsing failures with source ID, class, and field; stop acceptance instead of skipping them.
- [ ] **Step 5: Commit typed records and dependencies.**
  Use commit message `Index complete asset records and dependencies`.

### Task 3: Export images, fonts, and complete UI records

**Files:** Create `tools/reference/library/images.py` and `ui.py`.
Modify `library/__init__.py`.
Tests: `tools/reference/tests/test_library_images.py` and `test_library_ui.py`.

**Interfaces:**

- `export_images(source: Source, catalog: Catalog, output: Path) -> tuple[AssetExport, ...]` exports Texture2D, Sprite, Cubemap, and Font assets.
- `export_ui(source: Source, catalog: Catalog, output: Path) -> tuple[AssetExport, ...]` exports full Canvas and RectTransform UI records.
- `convert_layout(record: dict, ancestry: tuple[dict, ...]) -> dict` preserves source values and adds derived browser coordinates.
- Keep source pixel exports separate from role-specific normal and channel conversions in the material pipeline.
- Use `Pillow` for image decoding and font-face checks; preserve embedded font bytes without transcoding.

- [ ] **Step 1: Add source-empty, image, and layout tests.**
  Assert `test_zero_payload_font_texture` emits `source_empty` with zero dimensions and no generated PNG.
  Assert `test_shared_texture_roles_distinct` keeps source pixels distinct from material-specific derived files.
  Assert `test_sprite_rotation_border_and_cubemap_faces` preserves packed rotation, nine-slice borders, and six labeled faces.
  Assert `test_canvas_specific_scaling` uses each canvas's settings and converts Y coordinates exactly once.
  Assert `test_embedded_font_bytes_unchanged` preserves the input font hash and face name.
- [ ] **Step 2: Run image and UI tests and confirm failure.**
  Run their two unittest modules through the installed environment.
- [ ] **Step 3: Implement images, font files, and UI records.**
  Use UnityPy image APIs. Record original color space, sampler, mip, and stream settings before conversion.
  Keep original sprite atlas and geometry links alongside cropped images.
  Recover Image, RawImage, Text, TMP, mask, selectable, and event data through typed records.
  Preserve all 100 canvases, 5,140 RectTransforms, text records, and inactive UI objects.
  Link glyph tables and fonts through source IDs. Export all title artwork and language files into the library.
- [ ] **Step 4: Verify installed image, font, and UI outputs.**
  Expect 981 stored images, 54 source-empty textures, 155 sprites, two six-face cubemaps, and 31 embedded fonts.
  Expect two font records without font bytes; retain their source glyph and texture dependencies.
  Verify Liberation Sans Regular at `sharedassets0.assets:769`: 350,200 bytes and the specification's exact SHA-256.
  Decode every image output and check its dimensions against its recorded conversion.
- [ ] **Step 5: Commit image, font, and UI conversion.**
  Use commit message `Export complete images fonts and UI records`.

### Task 4: Export complete audio

**Files:** Create `tools/reference/library/audio.py`; modify `library/__init__.py`.
Test: `tools/reference/tests/test_library_audio.py`.

**Interfaces:**

- `export_audio(source: Source, catalog: Catalog, output: Path) -> tuple[AssetExport, ...]` writes each clip's decoded samples and playback metadata.
- `validate_wav(path: Path, clip_record: dict) -> dict` checks sample format, channel count, frequency, and duration.
- Keep AudioSource, mixer, group, and snapshot settings in typed records with resolved links.
- Preserve original streamed audio through verified archive ranges.

- [ ] **Step 1: Add audio integrity tests.**
  Assert `test_duplicate_clip_names_do_not_overwrite` keeps clips from different IDs in separate output paths.
  Assert `test_missing_stream_and_truncated_wav_fail` rejects a missing stream and a corrupt sample file.
  Assert `test_pcm_and_float_headers_match_source` validates PCM and floating-point WAV formats without altering samples.
- [ ] **Step 2: Run audio tests and confirm failure.**
  Run `python -m unittest tools.reference.tests.test_library_audio -v` through the installed environment.
- [ ] **Step 3: Implement audio conversion and metadata validation.**
  Use UnityPy's installed `AudioClip.samples` converter. Decode, write, verify, and release one clip at a time.
  Preserve loops, load settings, compression, spatial curves, and mixer links through source records.
  Compare decoded duration with source duration and report any codec padding or unexplained mismatch.
  Do not add Bevy audio features or gameplay audio systems.
- [ ] **Step 4: Verify every installed audio clip.**
  Expect 474 decoded clips and no empty sample dictionaries or failed conversions.
  Reconcile output bytes with the audit's 2,030,938,742-byte WAV result; explain any decoder-version difference.
  Read every WAV header and verify the output hash.
- [ ] **Step 5: Commit complete audio export.**
  Use commit message `Export complete source audio library`.

### Task 5: Export every model, hierarchy, skin, and LOD

**Files:** Create `tools/reference/library/models.py`.
Modify `tools/reference/gltf.py` and `library/__init__.py`.
Test: `tools/reference/tests/test_library_models.py`.

**Interfaces:**

- `export_models(source: Source, catalog: Catalog, output: Path) -> tuple[AssetExport, ...]` exports mesh assets and authored hierarchy packages.
- `collect_model_hierarchy(source: Source, root: ObjectId, catalog: Catalog) -> tuple[ObjectId, ...]` includes required skeleton ancestors.
- `write_library_model(source: Source, nodes: tuple[ObjectId, ...], output: Path) -> AssetExport` writes geometry without startup filtering.
- Reuse `convert_point`, `convert_rotation`, `convert_triangles`, glTF accessors, and `MeshHandler`.
- Extract shared glTF helpers within `gltf.py`; keep selected condition and visibility rules in its runtime scene writer.

- [ ] **Step 1: Add geometry, rig, and source-empty tests.**
  Assert `test_signed_scale_skin_and_winding` applies handedness once across triangles, tangents, joints, and bind poses.
  Assert `test_inactive_nodes_and_all_lods_retained` keeps inactive meshes and each authored LOD.
  Assert `test_empty_mesh_has_no_fabricated_geometry` preserves a zero-vertex record without a replacement mesh.
  Assert `test_external_bone_and_morph_links` includes needed skeleton ancestors and resolves morph target indices.
  Assert `test_missing_bone_and_bad_bind_pose_fail` rejects an invalid skin.
- [ ] **Step 2: Run model tests and confirm failure.**
  Run `python -m unittest tools.reference.tests.test_library_models -v` through the installed environment.
- [ ] **Step 3: Implement complete model export.**
  Export stored orphan meshes and reusable model hierarchies using source IDs, not name matching.
  Preserve UV channels, colors, normals, tangents, topology, submeshes, weights, bounds, and native geometry metadata.
  Retain all material and texture dependencies. Export glTF skin and supported morph data.
  Preserve unsupported topology as typed native data with a specific conversion reason.
  Keep the current runtime startup selection separate from authored library data.
- [ ] **Step 4: Validate all installed model outputs.**
  Expect 2,912 stored meshes, 22 source-empty meshes, and records for all 317 SkinnedMeshRenderers.
  Verify the 25 meshes with skin weights and each exported skin's source bones and bind poses.
  Validate every glTF buffer, accessor, primitive, node, skin, and morph reference.
- [ ] **Step 5: Commit complete model export.**
  Use commit message `Export complete model hierarchies and rigs`.

### Task 6: Decode optimized animations and bind them to source rigs

**Files:** Create `tools/reference/library/animations.py`; modify `models.py`, `gltf.py`, and `library/__init__.py`.
Test: `tools/reference/tests/test_library_animations.py`.

**Interfaces:**

- `AnimationTrack` contains `binding: dict`, `times: tuple[float, ...]`, `values: tuple[tuple[float, ...], ...]`, and `interpolation: str`.
- `AnimationTrack.in_tangents` and `out_tangents` use `tuple[tuple[float, ...], ...]`, with the same component widths as `values`.
- `AnimationRecord` contains `source: ObjectId`, `tracks: tuple[AnimationTrack, ...]`, `events: tuple[dict, ...]`, and `settings: dict`.
- `decode_clip(source: Source, reader, catalog: Catalog) -> AnimationRecord` reconstructs optimized clip data and source bindings.
- `sample_track(track: AnimationTrack, time: float) -> tuple[float, ...]` evaluates the preserved interpolation.
- `export_animations(source: Source, catalog: Catalog, models: tuple[AssetExport, ...], output: Path) -> tuple[AssetExport, ...]` exports tracks and glTF channels.

- [ ] **Step 1: Add streamed, dense, constant, and binding tests.**
  Assert `test_optimized_clip_with_empty_top_level_arrays` reconstructs non-empty normalized tracks.
  Assert `test_stream_sentinels_and_binding_dimensions` skips dummy frames and groups Transform components correctly.
  Assert `test_dense_then_constant_curve_offsets` uses streamed and dense curve counts when mapping constant bindings.
  Assert `test_path_hash_collision_and_missing_target_fail` rejects ambiguous generic Transform targets.
  Assert `test_muscle_event_and_object_tracks_retained` keeps non-glTF tracks and their dependency links.
  Include these numeric assertions using synthetic normalized tracks:

```python
self.assertEqual(sample_track(constant_track, 0.5), (7.0,))
self.assertEqual(sample_track(linear_translation, 0.5), (0.0, 0.0, 1.0))
self.assertEqual(convert_point(sample_track(linear_translation, 0.5)), (0.0, 0.0, -1.0))
```

- [ ] **Step 2: Run animation tests and confirm failure.**
  Run `python -m unittest tools.reference.tests.test_library_animations -v` through the installed environment.
- [ ] **Step 3: Implement optimized clip reconstruction and animation outputs.**
  Read `reader.parse_as_object().m_MuscleClip.m_Clip.data` as UnityPy's typed `Clip`.
  Consume `m_StreamedClip.data`, `m_DenseClip`, `m_ConstantClip`, and `m_ClipBindingConstant`.
  Follow the specification's AssetRipper format reference for curve ordering, sentinels, component dimensions, and slope recovery.
  Reuse installed binary readers. Record the format reference revision and any required attribution.
  Resolve generic path hashes from source hierarchies and Avatar tables; retain humanoid muscle bindings as typed tracks.
  Export generic Transform and supported morph channels to their model package; preserve other tracks as animation records.
  Preserve root motion, sample rate, wrap, duration, loop settings, events, and object-reference keys.
  Keep all 13 Avatar and ten AnimatorController records and their source links.
- [ ] **Step 4: Verify all 42 installed clips and their bindings.**
  Require decoded optimized tracks for every stored clip; top-level empty arrays cannot satisfy this check.
  Verify binding dimensions, monotonic times, valid duration, and exact controller-to-clip identity.
  Compare source and exported samples at endpoints, authored keys, and interval midpoints.
  Preserve native quaternion polynomial curves in typed records.
  For glTF rotation tracks, require angular error at most `0.01` degrees at authored keys and interval checks.
  Keep translation and scale interpolation error within `0.00001` source units.
  Check interval quarter, midpoint, and three-quarter samples; subdivide until they pass.
  Preserve discrete steps and reject zero-length quaternions.
  Record any channel that cannot meet these limits as a blocking conversion failure.
- [ ] **Step 5: Commit animation conversion.**
  Use commit message `Decode complete optimized animation library`.

### Task 7: Complete engine metadata and managed analysis

**Files:** Create `tools/reference/library/managed.py` and `tools/reference/decompile.ps1`.
Modify `library/records.py` and `library/__init__.py`.
Test: `tools/reference/tests/test_library_metadata.py`.

**Interfaces:**

- `export_native_assets(source: Source, catalog: Catalog, records_root: Path) -> tuple[AssetExport, ...]` indexes native engine records and their preserved bytes.
- `index_managed(snapshot: Snapshot, output: Path) -> tuple[FileRecord, ...]` writes assembly identity, type, method, and asset-call indexes.
- `decompile.ps1 -AssemblyPath <path> -OutputPath <path>` uses `ICSharpCode.Decompiler` version `11.1.0.9782`.
- `write_analysis_report(catalog: Catalog, managed_files: tuple[FileRecord, ...], output: Path) -> FileRecord` records proven and unproven dynamic links.
- Use a PowerShell host for the existing .NETStandard decompiler package; verify its package and dependency versions before execution.

- [ ] **Step 1: Add native metadata coverage tests.**
  Assert `test_shader_text_failure_keeps_native_bytes` retains raw range, parsed shader fields, and explicit diagnostic.
  Assert `test_runtime_texture_is_descriptor_not_fake_image` preserves creation references without invented pixels.
  Assert `test_material_and_particle_links_close` resolves material textures, particle meshes, and native shader records.
  Assert `test_decompiler_error_cannot_claim_complete_code` keeps original assemblies and reports incomplete analysis.
- [ ] **Step 2: Run metadata tests and confirm failure.**
  Run `python -m unittest tools.reference.tests.test_library_metadata -v` through the installed environment.
- [ ] **Step 3: Implement native indexes and managed analysis.**
  Preserve all material properties, shader programs, ComputeShaders, lighting, particles, physics, and project settings.
  Export the two TextAsset payloads and authored language/news/music files without changing source bytes.
  Index all archived managed assemblies and fully decompile `Assembly-CSharp.dll` into ignored analysis storage.
  Map MonoScripts to owning assemblies and types. Retain decompiler diagnostics and input/output hashes.
  Trace Resources names and runtime-created assets where source code establishes links.
  Keep unresolved dynamic analysis separate from unresolved serialized PPtrs.
- [ ] **Step 4: Verify native categories and analysis provenance.**
  Expect 897 materials, 118 shaders, 16 ComputeShaders, seven RenderTexture descriptors, and 1,380 particle records.
  Verify all four known shader text failures still have complete preserved bytes and available parsed metadata.
  Verify ten language files, original text payload hashes, and managed assembly hashes.
  Expect 637 `Assembly-CSharp.dll` type definitions; report full-module decompiler diagnostics rather than assuming success.
- [ ] **Step 5: Commit complete native metadata and analysis tooling.**
  Use commit message `Preserve engine asset metadata and managed analysis`.

### Task 8: Validate and publish the complete development library

**Files:** Create `tools/reference/library/validation.py`.
Modify `library/types.py`, `__init__.py`, and `tools/reference/export.py`.
Test: `tools/reference/tests/test_library_validation.py`.

**Interfaces:**

- `validate_library(snapshot: Snapshot, catalog: Catalog, exports: tuple[AssetExport, ...], output: Path) -> dict` produces category totals and support limits.
- `publish_library(snapshot: Snapshot, catalog: Catalog, exports: tuple[AssetExport, ...], work: Path, output: Path) -> LibraryReceipt` publishes validated output only.
- `verify_library(output: Path, source_root: Path) -> LibraryReceipt` rechecks accepted hashes, pointers, raw ranges, and coverage without writes.
- Library manifest schema version is `1`; include source-manifest and catalog hashes, category indexes, coverage totals, and converter versions.
- Wire all implemented stages into the single CLI, including `--verify`.

- [ ] **Step 1: Add acceptance and publication failure tests.**
  Assert `test_unconverted_clip_blocks_library` rejects a preserved clip with missing required normalized tracks.
  Assert `test_hash_or_required_pointer_failure_blocks_library` rejects a corrupt file or missing non-null target.
  Assert `test_native_reason_cannot_hide_required_decode_failure` rejects misuse of `native_record` classification.
  Assert `test_late_failure_preserves_accepted_manifest` keeps the existing accepted library and runtime outputs unchanged.
  Assert `test_repeat_export_is_deterministic` produces identical manifests and file hashes from the same source.
- [ ] **Step 2: Run validation tests and confirm failure.**
  Run `python -m unittest tools.reference.tests.test_library_validation -v` through the installed environment.
- [ ] **Step 3: Implement coverage and stage publication.**
  Check every original file, object range, converted file, category record, and dependency edge.
  Reject unknown schemas and invalid statuses. Require measured evidence for source-empty and engine-dependent records.
  Publish the manifest last after complete validation. Keep incomplete work under `.cache/reference-library/work/`.
  Report source preservation and development availability separately; keep browser support per asset.
- [ ] **Step 4: Export and independently verify the real complete library.**
  Run `--stage library`, then `--stage library --verify`.
  Expect exact baseline totals, no failed required conversions, and no unresolved non-null references.
  Enforce these exact source counts in the installed acceptance check:

```python
expected_counts = {
    'Texture2D': 1035, 'Mesh': 2934, 'Material': 897,
    'AudioClip': 474, 'Sprite': 155, 'Font': 33,
    'AnimationClip': 42, 'Avatar': 13, 'AnimatorController': 10,
    'Cubemap': 2, 'Shader': 118, 'ComputeShader': 16, 'TextAsset': 2,
}
self.assertEqual(sum(catalog.counts.values()), 262784)
for category, count in expected_counts.items():
    self.assertEqual(catalog.counts[category], count)
```

  Run a second export; compare accepted manifests and output hashes.
  Record elapsed time, peak process memory, disk bytes, and source-empty/native categories.
- [ ] **Step 5: Commit full library acceptance.**
  Use commit message `Validate and publish complete development asset library`.

### Task 9: Derive the current runtime from the accepted library

**Files:** Create `tools/reference/library/reader.py`.
Modify `tools/reference/export.py`, `manifest.py`, `source.py`, `gltf.py`, and `src/reference.rs`.
Update `tests/game/reference.rs`, `tests/game/common/reference.json`, `scripts/serve.mjs`, and the existing source UI specification and plan.
Tests: `tools/reference/tests/test_library_runtime.py`, `tests/game/reference.rs`, and `tests/web/server.test.mjs`.

**Interfaces:**

- `derive_reference(snapshot: Snapshot, library: LibraryReceipt, output: Path) -> None` derives the existing home/Lada/item selection.
- `promote_reference(staged: Path, accepted: Path) -> None` replaces a complete validated runtime directory and restores it on failure.
- `LibraryReader(receipt: LibraryReceipt)` opens validated category indexes for offline selection.
- `LibraryReader.record(source: ObjectId) -> dict` reads typed source data and checks its recorded hash.
- `LibraryReader.mesh(source: ObjectId) -> tuple[GLTF2, Path, int]` returns a validated mesh document, buffer directory, and mesh index.
- `LibraryReader.texture(source: ObjectId) -> Path` returns the validated stored image path.
- `GltfExporter(source, output, library: LibraryReader)` requires accepted geometry and image inputs for runtime scene export.
- `ReferenceManifest` gains required `library: LibraryProvenance`.
- `LibraryProvenance` contains `schema_version: u32`, `manifest_sha256: String`, `catalog_sha256: String`, and `source_manifest_sha256: String`.
- Read accepted geometry, images, and typed records through `LibraryReader`; never reread live Steam during derivation.
- Keep authored library data unchanged. Apply selected startup conditions, material roles, visibility, and current gameplay rules during derivation.
- `scripts/serve.mjs` accepts a port from `PORT`, defaulting to `4173`, to support isolated server tests.

- [ ] **Step 1: Add runtime provenance, selection, and promotion tests.**
  Assert `test_runtime_requires_accepted_library` rejects missing or mismatched provenance and obsolete manifests.
  Assert `test_only_selected_assets_in_runtime` excludes full-catalog preload and unrelated models/audio.
  Assert `test_failed_runtime_promotion_restores_previous_directory` preserves the accepted runtime after a rename error.
  In native tests, reject non-hex provenance hashes and unsupported library schema versions.
  In server tests, request selected WAV and font fixtures with GET and HEAD; check MIME types and byte lengths.
- [ ] **Step 2: Run runtime and native tests and confirm failure.**
  Run the new Python module, `./scripts/check.ps1 -Test reference`, and the explicit Node test file.
- [ ] **Step 3: Implement accepted-library derivation and provenance.**
  Derive glTF and current reference records from accepted library outputs and archived source rules.
  Reuse stored decoded payloads and geometry data. Do not run a second complete live-source extraction.
  Copy validated mesh accessors and attribute bytes while remapping glTF buffer and accessor indices.
  Apply material-specific channel conversion to stored source images without changing the library's source-image files.
  Keep runtime paths under `reference/` and validate each selected dependency.
  Add `.wav`, `.ttf`, `.otf`, and `.glb` MIME types to the local development server.
  Change the pending UI font acquisition rule to the embedded Liberation Sans asset and exact recorded hash.
  Preserve its typography, title, and supported UI behavior. Do not implement the separate UI or material plans in this task.
- [ ] **Step 4: Verify the selected runtime export and build.**
  Run default export, exporter tests, native reference tests, `./scripts/check.ps1`, and `./scripts/build.ps1`.
  Run the installed-export gate below; expect valid provenance, reference data, and selected output hashes.

```powershell
docker run --rm --mount "type=bind,source=$PWD,target=/workspace" --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' --mount 'type=volume,source=tld-wasm-target,target=/target' my-summer-slop-toolchain cargo test --locked --test game reference:: -- --ignored
```

  Inspect startup network requests. Expect selected reference assets, with no full catalog or unused library preload.
- [ ] **Step 5: Commit library-backed runtime derivation.**
  Use commit message `Derive selected game assets from complete library`.

### Task 10: Verify development use and record final coverage

**Files:** Update `README.md`, `tools/reference/README.md`, `DESIGN.md`, `PRODUCT.md`, and `docs/verification.md`.
Update the audit report with final measured results.
Keep temporary media and model check pages under ignored local storage.

**Interfaces:**

- Use `verify_library` and the installed runtime gate from Tasks 8 and 9.
- Keep temporary browser fixtures outside `web/index.html` and the production entry point.
- Serve temporary fixtures from `.cache/reference-library/browser-check/` through a separate loopback test server.
- Give that server a separate port and read library files through validated fixture paths.
- The final coverage report links to category indexes, conversion diagnostics, source-empty records, and runtime support evidence.

- [ ] **Step 1: Verify complete library hashes and counts after runtime derivation.**
  Expect unchanged library hashes and the full category totals from the specification.
- [ ] **Step 2: Inspect temporary media and model fixtures in the browser.**
  Check representative static and skinned model loading through the existing Bevy glTF loader.
  Check morph and animation data through validated glTF channels and source curve samples.
  Record animation playback as unverified unless a separate playback check succeeds.
  Check a WAV clip, embedded Liberation Sans, a rotated sprite, and a six-face cubemap.
  Inspect the existing garage and its loading state. Preserve the continuous route-test waiver.
  Record unverified engine-dependent adapters without labeling them browser-ready.
- [ ] **Step 3: Run final checks once after the last relevant implementation change.**
  Run exporter tests, Node server tests, complete project checks, WASM build, and installed-export validation.
  Record commands, exit results, source hashes, output bytes, and conversion diagnostics.
- [ ] **Step 4: Update documentation with verified scope and commands.**
  Describe complete development availability and selected runtime use as separate facts.
  Preserve gameplay exclusions in `PRODUCT.md` and record remaining rendering, retargeting, and browser limits.
  Remove obsolete selected-only extraction instructions and claims that the build lacks embedded Liberation Sans.
- [ ] **Step 5: Commit final library verification and documentation.**
  Use commit message `Document verified complete asset library`.
  Confirm Git contains only source, tests, schemas, and documentation from this work.

## Plan self-review and handoff

Tasks 1 and 2 establish source preservation and the complete graph.
Tasks 3 through 7 cover decoded media, models, optimized animation, UI, native engine records, and managed analysis.
Task 8 enforces full development availability. Task 9 derives the working product selection.
Task 10 records browser evidence and remaining adapter limits.
The five review-focus failures each have an owning critical test.

Review both documents before implementation and choose an execution method.
Native execution keeps the shared catalog and converter interfaces in one session.
Subagent-driven execution adds independent task reviews at higher context cost.
Native execution is the recommended method because each stage depends on the previous stage's accepted outputs.
