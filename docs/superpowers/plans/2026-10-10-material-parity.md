# Garage Material Parity Implementation Plan

Status update, 2026-10-11: Tasks 1–3 are delivered at the stable material checkpoint.
Task 4 progress is preserved separately. Tasks 4–5 are deferred.
Checkpoint verification and documentation replace the remaining completion work for this delivery.
Use [the roadmap](../../roadmap.md) and [remaining plan](2026-10-11-rendering-completion.md) for current status.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking. Wait for the user's plan review and execution choice.

**Goal:** Restore source materials and lighting for the imported garage, fixed Lada, and preparation items.

**Architecture:** Retain glTF geometry and the existing reference manifest. Recover source rendering records offline. Bind shared Bevy materials once, then add source lighting and a separate stained-glass pass.

**Tech Stack:** Rust 1.99, Bevy 0.19.1, WebGL2, Rapier 0.36.0, wasm-bindgen 0.2.129, Python, UnityPy 1.25.4, pygltflib 1.16.5.

**Spec:** [Approved specification](../specs/2026-10-10-material-parity-design.md).

## Global Constraints

- Use installed Steam build `11140890`, Unity `2021.3.18f1`, as the source.
- Match material behavior and lighting for the imported home, fixed Lada 2101, and preparation items.
- Preserve the garage spawn, gameplay, physics, controls, and existing road.
- Live vehicle mirror cameras, plate text, audio, survival, other vehicles, saves, multiplayer, and deployment remain outside this stage.
- Use one accepted export format. Do not add migration or compatibility paths.
- Cache textures, materials, shader pipelines, and render targets during loading.
- Do not claim identical pixels or full-game parity from an unmatched screenshot.
- Follow `AGENTS.md`. Use stop-slop for UI text and documentation. Keep instructions within 20 words.
- Preserve unrelated edits. Stage explicit task paths. Create an isolated worktree at execution through the worktree skill.

## Review Focus

1. A shared texture used as both packed normals and color must retain separate conversion identities. Task 1 tests this.
2. A missing texture, unknown shader, or failed pipeline must keep Start disabled and report its source. Tasks 2 and 3 test this.
3. A part with zero base alpha must retain source specular reflections. Task 3 verifies this in WebGL2.
4. A reflected wheel or rebased probe must retain normal orientation and lighting position. Tasks 3 and 4 verify this.
5. A resize or glass view must avoid stale targets and recursive sampling. Task 5 verifies this in WebGL2.

## File map

| Files | Responsibility |
| --- | --- |
| `tools/reference/visibility.py`, `conditions.py` | Source startup renderer and condition rules |
| `tools/reference/materials.py` | Source shader records, texture conversion, channel rules |
| `tools/reference/lighting.py` | Source lighting, sky, probes, and output transform |
| `tools/reference/gltf.py`, `export.py`, `manifest.py` | Export integration and dependency hashes |
| `src/reference/rendering.rs`, `src/reference.rs` | Typed rendering contract and validation |
| `src/presentation/materials.rs`, `web/assets/shaders/source_surface.wgsl` | Material binding and layered shading |
| `src/presentation/lighting.rs`, `web/assets/shaders/source_lighting.wgsl` | Source lighting and reflection response |
| `src/presentation/glass.rs`, `web/assets/shaders/source_glass.wgsl` | Reusable scene-color target and glass composition |
| `src/presentation/reference.rs`, `mod.rs`, `src/app.rs`, `Cargo.toml` | Runtime registration and readiness |
| `tools/reference/tests/test_materials.py`, `test_lighting.py`, `test_export.py` | Critical export regressions |
| `tests/rendering.rs`, `tests/reference.rs`, `tests/common/reference.json` | Native record validation and installed-source gates |
| `README.md`, `PRODUCT.md`, `DESIGN.md`, `docs/verification.md` | Verified behavior and measured limits |

## Verification commands

Run commands from the execution worktree. Keep the existing Python environment accessible from that worktree.
Create it with the existing requirements if the worktree lacks one.

```powershell
./tools/reference/.venv/Scripts/python.exe -m unittest discover -s tools/reference/tests -v
./tools/reference/.venv/Scripts/python.exe -m tools.reference.export --steam-root 'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive'
./scripts/check.ps1
./scripts/build.ps1
node scripts/serve.mjs
```

The installed-export gate uses the current Docker image and cache volumes:

```powershell
docker run --rm --mount "type=bind,source=$PWD,target=/workspace" --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' --mount 'type=volume,source=tld-wasm-target,target=/target' my-summer-slop-toolchain cargo test --locked --test reference -- --ignored
```

Expected: exporter and native tests pass, installed gates pass, and the release browser package builds.
Use CUA for browser inspection. Keep captures, source recovery, and temporary diagnostics under ignored `.cache`.
Capture a browser view after each rendering layer. Record warm frame times and texture bytes with that capture.
Do not add production debug controls or a second asset-loading path.

### Task 1: Restore concrete visibility and decode packed normals

**Files:** Create `tools/reference/materials.py`, `tools/reference/tests/test_materials.py`. Modify `tools/reference/visibility.py`, `gltf.py`, `tests/test_export.py`, `tests/reference.rs`.

**Interfaces:**
- Produce `renderer_visible(serialized_enabled: bool, component_classes: set[str]) -> bool` in `visibility.py`.
- Produce `decode_normal_rgba(pixel: tuple[int, int, int, int], encoding: str) -> tuple[float, float, float]`.
- Produce `decode_normal_image(image: PIL.Image.Image, encoding: str) -> PIL.Image.Image` in `materials.py`.
- Extend `GltfExporter.texture(owner, environment, color_space, normal_encoding=None)` for explicit normal conversion.
- Accept `ag`, `rg`, and `rgb` encodings. Resolve encoding from source texture and shader evidence; reject unknown encoding.

- [ ] Add `test_packed_normal_uses_alpha_green`: decode `(255, 126, 126, 127)` with `ag`.
  Assert X equals `-1/255`, Y equals `-3/255`, and Z equals `sqrt(1-X*X-Y*Y)` within `1e-6`.
- [ ] Add `test_texture_roles_do_not_share_converted_pixels`. Export one source texture as color and as normals.
  Assert distinct image identities, unchanged color alpha, and converted normal Z above `0.99` for the sample.
- [ ] Add `test_terrain_hole_renderer_is_hidden`. Assert `renderer_visible(True, {'digholescript2'})` is false.
  Assert an enabled ordinary renderer remains visible. Keep hidden LOD and text rules intact.
  Use these assertions in the visibility and normal tests:

  ```python
  self.assertFalse(renderer_visible(True, {'digholescript2'}))
  self.assertTrue(renderer_visible(True, set()))
  x, y, z = decode_normal_rgba((255, 126, 126, 127), 'ag')
  self.assertAlmostEqual(x, -1 / 255, places=6)
  self.assertAlmostEqual(y, -3 / 255, places=6)
  self.assertAlmostEqual(z, math.sqrt(1 - x*x - y*y), places=6)
  ```
- [ ] Run exporter tests. Expect the new tests to fail before implementation.
- [ ] Implement the functions and role-aware texture cache. Apply image orientation once after decoding.
- [ ] Use source component classes to hide terrain helpers. Retain their nodes and terrain data.
- [ ] Add an ignored installed-export assertion: `level1:35014` has `renderer_enabled == false` and no visible primitive.
- [ ] Run exporter tests, regenerate assets, run the installed gate, and build the browser package.
- [ ] Inspect the garage. Expect concrete detail and stable normals instead of the white plane and extreme bumps.
- [ ] Commit this working layer as `Fix source helper visibility and packed normals`.

### Task 2: Export validated source material records and condition variants

**Files:** Modify `materials.py`, `conditions.py`, `gltf.py`, `export.py`, `manifest.py`, `src/reference.rs`, `src/lib.rs`, `tests/common/reference.json`. Create `src/reference/rendering.rs`, `tests/rendering.rs`. Extend `test_materials.py`.

**Interfaces:**
- Produce `recover_material(source: Source, reader: ObjectReader, color_override: dict | None, condition: dict | None) -> dict`.
- Produce `export_material_textures(source: Source, owner: ObjectReader, record: dict, output: Path) -> list[dict]`.
- Produce `RenderingRecord { materials: Vec<MaterialRecord> }`, required as `ReferenceManifest.rendering`.
- Produce `RenderingRecord::validate(&self, assets: &[AssetRecord]) -> Result<(), ReferenceError>`.
- A `MaterialRecord` contains asset ID, glTF material index, source ID, shader family, color, condition, keywords, and shader properties.
- Shader properties retain typed float/color maps and `textures: Vec<TextureRecord>` with named source slots.
  Serialize the maps as `properties.floats` and `properties.colors`; serialize top-level color as `color: [f32; 4]`.
- A `TextureRecord` contains source ID, file, color space, encoding, effective UV channel/scale/offset, and sampler settings.
- Use shader-family enum values `standard`, `unlit_color`, and `stained_glass`. Add another family only after source inventory proves it required.
- Preserve `_MainTex`, `_BumpMap`, `_DetailAlbedoMap`, `_DetailMask`, `_DetailNormalMap`, `_MetallicGlossMap`, `_OcclusionMap`, `_EmissionMap`, and `_ParallaxMap` where active.

- [ ] Read installed shader keywords and passes. Compare required formulas with Unity's official built-in shader source for this editor version.
  Store source hashes, active operations, blend/depth state, and channel evidence in ignored recovery records.
- [ ] Add `test_material_keeps_detail_mask_alpha`: source mask alpha `128` remains `128`; it must not become opaque.
- [ ] Add `test_missing_active_texture_names_source` and `test_unknown_required_shader_is_rejected`.
  Assert errors contain the source ID and required property or shader name.
- [ ] Add `test_condition_old_color_uses_rusty_color` and `test_condition_force_color_uses_part_color`.
  Assert source `defRusty.color` for states `3` and `4` unless `forceColor`; preserve `disableColor`.
- [ ] Add `rejects_invalid_material_bindings` in `tests/rendering.rs`.
  Assert rejection of duplicate asset/index pairs, absent asset IDs, unsafe paths, unknown slots, and nonfinite transforms.
  Define `fixture() -> serde_json::Value` by parsing `include_bytes!("common/reference.json")` in this test target.
  Use this missing-asset assertion:

  ```rust
  let mut input = fixture();
  input["rendering"]["materials"][0]["asset"] = serde_json::json!(u32::MAX);
  assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
  ```
- [ ] Run the affected Python tests and `./scripts/check.ps1 -Test rendering`. Expect new assertions to fail.
- [ ] Implement record recovery, Rust types, and required manifest validation.
  Resolve effective UV behavior from shader operations; serialized unused texture transforms must not alter shading.
- [ ] Correct source condition inheritance and mesh/object variant selection. Preserve the current deterministic seed and physical bindings.
- [ ] Hash every active rendering texture through asset dependencies. Update the native fixture with explicit synthetic records.
  Keep corrected base glTF materials usable while this layer introduces the final source record contract.
  Omit excluded live mirror RenderTextures through an explicit source exclusion record. Retain their authored surface factors.
- [ ] Run exporter tests, regenerate assets, validate the installed export, and build/render the garage.
- [ ] Commit as `Export validated source material records`.

### Task 3: Bind layered source shaders and gate scene readiness

**Files:** Create `src/presentation/materials.rs`, `web/assets/shaders/source_surface.wgsl`. Modify `src/presentation/mod.rs`, `reference.rs`, `src/app.rs`, `Cargo.toml`, `tools/reference/gltf.py`. Extend `tests/rendering.rs` and `test_materials.py`.

**Interfaces:**
- Produce `SourceMaterialPlugin: Plugin`, `SourceSurface: MaterialExtension`, and `type SourceMaterial = ExtendedMaterial<StandardMaterial, SourceSurface>`.
- Produce `SourceMaterials` resource holding loaded textures, record-indexed material handles, and pipeline state.
- Produce `load_materials(commands: &mut Commands, server: &AssetServer, rendering: &RenderingRecord)`.
- Produce `bind_materials` as a Bevy system using `GltfMaterialExtras`, existing primitive handles, and `SourceMaterials`.
- Produce `material_readiness` as a Bevy system; it sets `SourceMaterials.state: RenderingState`.
- Define `RenderingState` with `Loading`, `Ready`, and `Failed(String)`. `check_ready` requires `Ready` before `game_ready`.
- Put asset ID and material index into glTF material extras. Resolve primitive bindings through those keys, not material names.

- [ ] Add `test_material_identity_includes_color_and_condition` using exporter records with shared source IDs and distinct colors.
  Assert distinct glTF indices for distinct colors or conditions; identical complete records reuse one index.
  Check shared runtime handles for identical records during browser loading.
- [ ] Add `accepts_zero_alpha_standard_glass` to native record validation.
  Use a fixture glass record with source mode `3`, then assert zero alpha remains valid:

  ```rust
  let mut input = fixture();
  input["rendering"]["materials"][0]["color"][3] = serde_json::json!(0.0);
  input["rendering"]["materials"][0]["properties"]["floats"]["_Mode"] = serde_json::json!(3.0);
  assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_ok());
  ```
- [ ] Add source channel fixtures: a gamma detail sample `0.2` with mask `0.5` multiplies base albedo by `0.7`.
  A main-alpha smoothness sample `0.75` with scale `0.4` produces smoothness `0.3` when that source keyword is active.
  Retain these expected samples in exporter evidence for GPU comparison; do not add a second CPU renderer.
- [ ] Run affected tests. Expect the new identity/channel assertions to fail before their implementation.
- [ ] Implement source texture transforms, detail multiplication, alpha-mask sampling, and normal blending in WGSL.
  Decode metallic, smoothness, occlusion, emission, and height/parallax according to active source shader operations.
  Preserve source bump scale, tangents, signed scale, color space, unlit output, cutoff, culling, blend, and depth writes.
- [ ] Reuse Bevy forward-light resources. Implement source reflectance calculations where its shader formulas differ.
  Specialize pipeline state through the installed `MaterialExtension::specialize` signature.
- [ ] Bind materials once. Remove the generic Unity-to-PBR material construction for the covered source shaders.
  Retain glTF material entries for index identity; source records own their final rendered behavior.
- [ ] Check generated fragment texture/uniform limits against the WebGL2 device before enabling Start.
  Deduplicate texture resources and pack uniform data. Reject a required binding layout that exceeds the device limits.
- [ ] Replace the two-frame readiness assumption with texture, binding, and pipeline readiness checks.
  Surface resource errors and failed shader compilation through `game_failure` with material/source identity.
  Prewarm required mesh/material pipeline variants while the scene is hidden. Do not wait for hidden meshes to queue draw calls.
  Poll their cached pipeline IDs in the render world and transfer readiness/failure to `SourceMaterials.state`.
- [ ] Run tests, installed export checks, and the release build. Inspect the browser shader log and channel samples.
- [ ] Verify zero-alpha Standard glass retains specular response; cutout cobwebs retain stable edges.
  Check reflected wheel normals from both sides and repeated restart for unchanged material counts.
- [ ] Block one required material texture in a temporary browser test. Expect Start disabled and a named failure.
  Introduce a temporary shader syntax fault, then restore it. Expect a named pipeline failure without a false ready state.
- [ ] Commit as `Render layered source materials in WebGL2`.

### Task 4: Recover and render startup lighting and reflections

**Files:** Create `tools/reference/lighting.py`, `tools/reference/tests/test_lighting.py`, `src/presentation/lighting.rs`, `web/assets/shaders/source_lighting.wgsl`. Modify `export.py`, `manifest.py`, `src/reference/rendering.rs`, `src/presentation/materials.rs`, `mod.rs`, `src/app.rs`, `tests/common/reference.json`, `tests/rendering.rs`.

**Interfaces:**
- Produce `recover_lighting(source: Source, roots: list[ObjectReader], output: Path) -> dict`.
- Add required `lighting: LightingRecord` to `RenderingRecord`; regenerate both production and native records.
- `LightingRecord` contains project color space, output transform/exposure, ambient SH, sky, lights, probes, and conversion evidence.
- `LightRecord` contains source ID, optional node ID, world pose, type, color, intensity, range, mask, enable state, and shadows.
- `ProbeRecord` contains source ID, world pose, bounds, box projection, influence, cubemap files, resolution, and refresh mode.
- Produce `SourceLightingPlugin: Plugin` and `SourceLighting` resource with preallocated lighting/capture resources and `RenderingState`.
- Produce `load_lighting(commands: &mut Commands, server: &AssetServer, lighting: &LightingRecord)`.
- Produce `sync_lighting` as a Bevy system driven by existing node transforms, origin changes, and source refresh rules.

- [ ] Trace source startup light scripts, `RenderSettings`, reflection components, graphics settings, and camera postprocessing.
  Record source-to-Bevy unit conversion and output-transform evidence. Reject unresolved required startup fields.
- [ ] Add `test_startup_light_state_overrides_serialized_state`: disabled source startup lights remain disabled despite luminous material emission.
- [ ] Add `test_sky_and_probe_faces_keep_source_orientation`: verify known face-center directions after the existing handedness conversion.
- [ ] Add `rejects_invalid_lighting_resources`: reject nonfinite values, missing probe faces, and invalid node or texture links.
  Put one synthetic sun and probe in the native fixture, including their hashed texture dependencies.
  Include this negative-intensity assertion:

  ```rust
  let mut input = fixture();
  input["rendering"]["lighting"]["lights"][0]["intensity"] = serde_json::json!(-1.0);
  assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
  ```
- [ ] Define browser check `probe_position_tracks_origin`: compare probe and home relative positions before and after a floating-origin change.
  Expect the same relative position. Use temporary diagnostics outside product code to measure it.
- [ ] Run Python lighting tests and native rendering tests. Expect failures before implementation.
- [ ] Implement recovered startup lights, ambient SH, sky, output transform, and source shadow flags.
  Remove fixed sun, ambient, exposure, and tonemapping assumptions from `src/app.rs`.
- [ ] Export probe cubemap faces and mip data using source orientation and radiance encoding.
  Use `EnvironmentMapLight` and `LightProbe` where they match the source. Implement missing source probe projection in WGSL.
- [ ] Preallocate any source-required realtime probe cameras and targets. Reuse them and follow source capture timing.
  Do not add live vehicle mirror cameras or a day/night simulation.
- [ ] Bind light/probe positions to existing transform and origin updates. Require lighting resources and pipelines before Start.
- [ ] Run exporter and native tests, regenerate assets, run installed checks, and build the browser package.
- [ ] Compare paint, metal, glass, and concrete under the same startup lighting profile.
  Verify shadow casting, interior light state, and probe position after driving far enough to rebase.
- [ ] Commit as `Restore source startup lighting and reflections`.

### Task 5: Reproduce stained-glass distortion

**Files:** Create `src/presentation/glass.rs`, `web/assets/shaders/source_glass.wgsl`. Modify `src/presentation/mod.rs`, `materials.rs`, `src/app.rs`, `src/reference/rendering.rs`. Extend `tests/rendering.rs`, `test_materials.py`.

**Interfaces:**
- Produce `SourceGlassPlugin: Plugin` and `SourceGlass: MaterialExtension` with source glass uniforms and scene-color binding.
- Produce `SceneColorCapture` resource with one persistent target per active view, physical extent, and `RenderingState`.
- Produce `resize_scene_color` and `sync_scene_color_view` as Bevy systems.
- Compose glass after the source-required preceding scene content. Exclude glass from its own sampled scene-color target.

- [ ] Add `test_stained_glass_preserves_distortion`: exported materials `sharedassets1.assets:48` and `:47` retain `_BumpAmt` values `128` and `64`.
  Assert bump UV scale `[2, 2]`, source tint, texture links, and authored pass state.
  For the two recovered material records, use these assertions:

  ```python
  self.assertEqual(door2['properties']['floats']['_BumpAmt'], 128)
  self.assertEqual(door1['properties']['floats']['_BumpAmt'], 64)
  bump = next(t for t in door2['textures'] if t['slot'] == '_BumpMap')
  self.assertEqual(bump['scale'], [2, 2])
  ```
- [ ] Define browser check `glass_target_extent_uses_physical_pixels` with a logical `(1280, 720)` view and scale factor `2`.
  Expect physical target extent `(2560, 1440)`. Minimized views must suspend capture without creating a zero-sized target.
- [ ] Run the affected tests. Expect failures before implementation.
- [ ] Allocate the reusable target and ordered render pass. Use the actual view projection, visibility, and depth.
  Match source grab-pass content/order instead of substituting a flat glass tint.
  Compile glass and capture pipelines before marking `SceneColorCapture.state` ready. Add it to the existing Start readiness gate.
- [ ] Implement source distortion channel decoding, projected scene sampling, bump amount, tint, and blend output.
- [ ] Resize resources on physical extent changes; release the old target after rebinding.
  Keep the same target during steady frames. Prevent capture feedback and duplicate main-view lighting.
- [ ] Run tests and the release build. Check both doors from front and rear at several view angles.
  Resize the browser, enter/leave fullscreen, and restart. Expect correct distortion and no stale image or recursion.
- [ ] Commit as `Render source stained-glass distortion`.

### Task 6: Verify matched views and document final behavior

**Files:** Modify `README.md`, `PRODUCT.md`, `DESIGN.md`, `docs/verification.md`, `tools/reference/README.md`. Extend installed checks in `tests/reference.rs` only where a source contract still lacks coverage.

**Interfaces:** Consume the accepted export, all rendering readiness states, and existing gameplay controls. Produce verification evidence and accurate documentation.

- [ ] Run exporter tests, installed-export gates, formatting, native regressions, and release package build.
  Expect passing output. Record tested commit, source hashes, browser backend, and shader errors.
- [ ] Capture the supplied garage angle, both door views, and the cockpit in WebGL2.
  Match installed-game camera pose, field of view, resolution, condition, and startup lighting before comparison.
  Use independent material detail, shadow, glass, and reflection comparisons; an unmatched image cannot prove 1:1 parity.
- [ ] Verify doors, seating, restart, and recovery with source materials bound. Check preserved fluid, wheel, and vehicle state.
- [ ] Measure warm frame intervals, texture memory, and handle/target counts at each stored layer capture.
  Expect stable resource counts during steady frames; explain any remaining library allocations or source capture cost.
- [ ] Review all source shader families and active operations against the export inventory.
  Report any visual mismatch or missing matched source capture. Withhold the parity claim until those checks pass.
- [ ] Update documentation with verified behavior and remaining limits. Remove obsolete material descriptions.
  Keep recovered assets, decompiled code, and captures outside version control.
- [ ] Commit as `Document verified garage material parity`.

## Execution handoff

The user must review this plan and select execution before product changes start.
Recommend native execution: these tasks share one material contract and require sequential browser checks.
Subagent execution remains available if the user prefers independent task reviews.

## Plan review

Coverage: Tasks 1 and 2 handle source visibility, normals, condition, and the record contract.
Task 3 handles layered materials, active shader operations, alpha behavior, resource reuse, and loading errors.
Task 4 handles source lighting, output transforms, shadows, sky, probes, and floating-origin updates.
Task 5 handles scene-color ordering, glass, resize, and WebGL2 checks.
Task 6 handles matched views, gameplay checks, performance evidence, and documentation.
The five Review Focus cases each have named tests or browser checks above.
