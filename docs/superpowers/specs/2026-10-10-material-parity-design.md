# Garage material parity

Date: 2026-10-10, Europe/Warsaw.
Status: The user approved this written specification in chat on 2026-10-10.

## Intent and scope

The user wants the current garage and car to match The Long Drive's original appearance, with a 1:1 target.
Use installed Steam build `11140890`, Unity `2021.3.18f1`, as the source.
Match material behavior and lighting for the imported home, fixed Lada 2101, and preparation items.
Preserve the garage spawn, gameplay, physics, controls, and existing road.

This stage replaces the current material approximation for those imported assets.
It includes paint, rust, concrete, brick, wood, tires, upholstery, cobwebs, and glass.
It includes scene reflections on these surfaces and stained-glass distortion.
Live vehicle mirror cameras, plate text, audio, survival, other vehicles, saves, multiplayer, and deployment remain outside this stage.
The original garage specification still controls gameplay.

The target is source material parity and verified visual parity in matched views.
Do not claim identical pixels or full-game parity from an unmatched screenshot.

## Evidence and faults

The user's screenshot shows a white floor, strong surface bumps, flat paint, and harsh lighting.
An [original garage screenshot](https://gtrainers.com/_ld/130/92658876.jpg) shows concrete detail, softer lighting, and surface reflections.
That screenshot contains a different car. Use it for garage appearance, not Lada paint or condition.
Installed-build assets and startup code control conflicting values.

The investigation established these faults:

| Fault | Source evidence | Current behavior |
| --- | --- | --- |
| White floor | `level1:28128`, `haz02/HelpPos (3)`, belongs to component `79970` (`terrainHeightAlignToBuildingScript`). Its `FStart` deactivates each `helpPosCoords` GameObject. `TerrainGenerationSettings.FStart` calls it. | The exporter keeps the marker active. Its untextured white mesh covers the concrete. |
| Terrain hole helper | `level1:35014`, `haz02/Luk`, has `digholescript2`, component `68463`. Its `Refresh` disables its renderer. `TerrainGenerationSettings.FStart` calls `Refresh`. | The exporter marks this separate helper renderer enabled. |
| Strong bumps | Source normal textures store X in alpha and Y in green. | The exporter copies packed pixels into an RGB normal texture. |
| Missing surface detail | `Lada1Used`, `Lada2Middle`, and `HouseConcrete` enable `_DETAIL_MULX2` and `_NORMALMAP`. | The exporter omits detail albedo, masks, and detail normals. |
| Wrong material response | Source properties include metallic maps, smoothness channels, occlusion, and reflection switches. | The exporter copies scalar metallic and roughness values while omitting required maps and switches. |
| Wrong lighting | Source `RenderSettings` contain ambient data, skybox, reflections, and a sun link. | The app uses one fixed light, default shadow settings, and Reinhard tonemapping. It disables imported lights. |

A sampled Lada normal decodes as `(1.0000, -0.0118, -0.0118)` through the current RGB path.
The source alpha/green decoding gives approximately `(-0.0039, -0.0118, 0.9999)` before bump scaling.

Bevy's [glTF documentation](https://docs.rs/bevy_gltf/latest/bevy_gltf/) limits texture transforms to the base-color texture.
Retaining each source transform in glTF extras does not apply it during rendering.

## Architecture and data flow

Keep the existing Python exporter, glTF geometry loader, Rust manifest validation, and Bevy renderer.
Use Bevy material extensions and its lighting features where their behavior matches the source.
Add a focused material module and WGSL shader code for source behavior that Bevy does not supply.
Keep lighting recovery and runtime lighting separate from material recovery and runtime material binding.
Use the existing dependencies. Add a package only if its documented capability reduces the implementation.

Keep geometry export in `tools/reference/gltf.py`.
Put material recovery in `tools/reference/materials.py` and lighting recovery in `tools/reference/lighting.py`.
Put runtime material binding in `src/presentation/materials.rs` and runtime lighting in `src/presentation/lighting.rs`.
Keep startup renderer rules in `tools/reference/visibility.py`.

The exporter resolves startup visibility, condition, material selection, and required texture channels.
It writes glTF geometry plus typed material and lighting records in the existing reference export.
Records use source IDs and explicit texture references. Names serve as labels, not identities.
The manifest hashes every new texture, buffer, and record file.
Bind each material record through its asset path and glTF material index.
Include the selected color and condition in the material cache key.

The runtime validates records, loads their dependencies, and binds material handles to imported primitives once.
The same source material and condition share one runtime material handle.
The runtime loads lighting and reflection resources before revealing the scene or enabling Start.
Remove the obsolete generic Unity-to-PBR path for the covered shaders. Regenerate the local export.

## Startup visibility and condition

Recover renderer changes from source component behavior, including `digholescript2.Refresh`.
Hide the terrain helper's visual mesh while preserving its node identity and terrain-related data.
Do not select hidden meshes through object-name heuristics.

Preserve the current deterministic startup seed and recovered condition ranges.
Follow `partconditionscript.FStart` and `Refresh` for material choice, color inheritance, and selected mesh or object variants.
Honor shader keywords when selecting texture operations.
For old and rusty parts, apply the source `defRusty.color` rule unless `forceColor` selects the part color.
Preserve `disableColor`, material-list behavior, and renderer material assignments.
Do not recolor the car to match a screenshot from another vehicle or random seed.

## Material behavior

Export color textures as sRGB and data textures as linear.
Decode source packed normals into tangent-space normals. Apply the source bump scale and normal blend operation.
Preserve handedness, tangent orientation, image orientation, mesh UV channels, and signed node scale.
Each texture operation must use its source UV channel, scale, offset, wrap mode, and filter mode.
Preserve mip behavior, anisotropy, and mip bias within WebGL2 support. Record any unsupported source setting.

Reproduce Standard shader detail multiplication, mask channels, and detail-normal blending.
Decode metallic and smoothness maps using the source channel and scale rules.
Apply occlusion strength, emission, reflection switches, and specular switches.
Implement source lighting and reflectance calculations where Bevy's calculations differ.
Convert source color factors according to the recovered project color-space setting.
Use source alpha modes, cutoff, blend factors, culling, and depth-write behavior.
Preserve transparent specular output where the source base alpha is zero.
Use unlit shading for source unlit materials.

Implement `FX/Glass/Stained BumpDistort` as a separate material path.
Use its authored texture, distortion map, UV transforms, tint, and distortion amount.
Sample a reusable scene-color target containing the preceding opaque scene.
Exclude stained glass from that input target to prevent recursive rendering.
Match the source grab-pass ordering and projection when composing the glass.
Validate the required shader bindings and render targets against WebGL2 limits.

## Lighting and reflections

Recover startup sun, home lights, shadow settings, ambient data, skybox, and relevant reflection probes.
Trace source startup scripts where they override serialized values.
Keep the source light enable state; a luminous texture alone does not establish an enabled light.
Export source transforms, light types, colors, ranges, intensities, and layer masks.
Record intensity conversion between Unity and Bevy units.

Enable source shadow casting and receiving on the correct renderers.
Recover camera exposure, project color space, and postprocessing before choosing the output transform.
Replace the fixed lighting values with the recovered startup profile.
This stage reproduces the startup lighting state. It does not add a day/night simulation.

Use source probe data and sky reflections for paint, metal, and glass.
Recover probe influence regions, box projection, and refresh rules required by the selected area.
Allocate reflection resources once. Update them according to source rules and reuse their buffers.
Keep live vehicle mirror-camera output outside this scope.

## Loading, errors, and frame cost

Fail export on unresolved required shader data, texture channels, startup values, or unsupported required shader families.
Name the source ID and property in the error.
Fail loading on missing or invalid material records, resources, or shader compilation.
Keep Start disabled until required dependencies and pipelines are ready.
Use one accepted export format. Do not add migration or compatibility paths.

Cache textures, materials, shader pipelines, and render targets during loading.
Do not rebuild material descriptions, search the source hierarchy, or allocate capture resources each frame.
Per-frame changes update only existing transforms, uniforms, and source-required reflection captures.
Measure frame time and texture memory after each rendering layer.

## Acceptance and delivery

Build working layers in this order: visibility and normal decoding; layered materials; lighting and reflections; stained glass.
Each layer must compile and render the garage before the next layer starts.

Use critical tests for packed-normal decoding, detail-mask channels, UV transforms, smoothness channels, condition rules, and helper visibility.
Run exporter tests, installed-export validation, native regression checks, and the release WASM build.
Confirm runtime shader compilation and loading in WebGL2.
Verify that the concrete appears, surface normals remain stable, and paint wear follows the source materials.
Check glass, cobweb edges, and reflected wheel transforms for rendering errors.
Verify door movement, seating, restart, and recovery after material binding changes.

Capture browser views at the supplied garage angle, beside the doors, and inside the car.
Compare with installed-game captures using the same camera pose, field of view, resolution, condition, and lighting state.
Keep screenshots and recovered assets in ignored local storage.
Record measured differences and unresolved visual limits in `docs/verification.md`.
If matched source captures cannot be obtained, report that gap and withhold a 1:1 visual-parity claim.

Update `README.md`, `PRODUCT.md`, and `DESIGN.md` to describe the verified final behavior.
Do not publish a parity claim before the source and browser checks pass.
