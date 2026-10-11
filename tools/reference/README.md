# Installed reference export

Use Python 3.12. Run these commands from the repository root.

```powershell
python -m venv tools/reference/.venv
tools/reference/.venv/Scripts/python.exe -m pip install -r tools/reference/requirements.txt
docker build --tag my-summer-slop-toolchain --file tools/Dockerfile .
tools/reference/.venv/Scripts/python.exe -m tools.reference.export --steam-root "C:/Program Files (x86)/Steam/steamapps/common/The Long Drive"
tools/reference/.venv/Scripts/python.exe -m unittest discover -s tools/reference/tests
./scripts/check.ps1 -Test reference
```

Runtime derivation and `--stage runtime --verify` require Docker and the project toolchain image.
Build the image with `docker build --tag my-summer-slop-toolchain --file tools/Dockerfile .`.
Runtime verification sends manifest bytes to the native `validate-reference` command through stdin.
It uses the game's `ReferenceManifest` contract, the existing Cargo volumes, and a read-only checkout mount.
Missing Docker, failed validation, or an unavailable image stops acceptance before promotion.
Library-only verification does not use Docker.

The exporter accepts Steam app `1017180`, build `11140890`, with Unity `2021.3.18f1`.
It preserves the installed tree and complete object graph before converting development assets.
It derives the selected garage, Lada, and supplies from the accepted library.
Git ignores recovered files and decompiled code. The exporter leaves Steam files unchanged.

## Stages and storage

Run commands from the repository root. Use one current schema; obsolete schemas fail validation.
The default stage is `runtime`. It runs missing prerequisites and verifies an existing library before derivation.
`--output` sets the selected runtime directory. Its sibling `library` directory stores development assets.
`--verify` checks accepted files without changing them.

| Stage | Command suffix | Output |
| --- | --- | --- |
| Source | `--stage source` | `.cache/reference-library/source/`, source hashes, and copied Steam manifest |
| Catalog | `--stage catalog` | `catalog.json`, complete typed records, schemas, and binary sidecars |
| Library | `--stage library` | `web/assets/library/`, category indexes, coverage, and acceptance manifest |
| Runtime | `--stage runtime` | `web/assets/reference/`, three selected scenes and gameplay records |

```powershell
python -m tools.reference.export --stage library --verify
python -m tools.reference.export --stage runtime --verify
```

Use the configured reference Python environment for these commands.
Library verification checks source hashes, typed records, dependencies, required media conversions, and installed counts.
Runtime verification also checks the native gameplay contract, selected file hashes, and exact scene dependencies.
Runtime derivation reads accepted typed records, glTF buffers, and PNG files. It does not decode Unity meshes again.
Prerequisite library verification still reads archived source data for semantic checks.

The accepted source archive contains 183 files and 529,237,089 bytes.
Shared records and managed analysis contain 268,954 files and 698,121,145 bytes.
The published library contains 8,570 files and 3,105,818,196 bytes.
The selected runtime contains 89 files and 73,261,688 bytes.

Category indexes: [images](../../web/assets/library/images/index.json), [models](../../web/assets/library/models/index.json),
[audio](../../web/assets/library/audio/index.json), [animations](../../web/assets/library/animations/index.json),
[UI](../../web/assets/library/ui/index.json), and [native](../../web/assets/library/native/index.json).
These local links require an accepted export.
Read [coverage](../../web/assets/library/coverage.json) for per-type native and source-empty counts.
Read each category record's `reason`, `runtime`, and `details` fields for conversion and adapter limits.
Keep the shared cache: native records depend on its bound `records/manifest.json` receipt.

## Selected runtime rules

The home link comes from `homesickscript.FirstHome`.
The Lada link comes from `TerrainGenerationSettings.StartCar02` and its `Car02Full` scene child.
The exporter checks the shared prefab hierarchy before reading scene startup values.

`Source` caches resolved object IDs and generated type trees.
It repairs the MonoBehaviour header and generated array container types.
UnityPy's Python reader verifies complete byte consumption and header pointers.

Geometry uses `C = diag(1, 1, -1)` once.
Quaternions use `(-x, -y, z, w)`. Triangle winding and tangent handedness change once.
Nodes retain signed scale. Collider points use body-local meters.
Body poses contain translation and rotation. `parent_body` identifies moving parent bodies.

Node extras contain the resolved source ID in `reference_id`.
Bind this ID to the manifest's numeric node ID once during loading.
Asset records include hashes for each referenced buffer and texture.
`parameter-evidence.json` records source fields, byte checks, startup rules, and material differences.

Startup conditions use a fixed build-based seed and original source ranges.
Rims and tires use the matching source material variants.
The exporter selects the highest source LOD and retains all transform IDs.

Mirror materials retain their authored factors. Live mirror camera output is omitted.
Plate text is omitted because serialized glyphs contain obsolete editor text.
Unity shaders map to glTF PBR. Verify their appearance during browser integration.

`tests/common/reference.json` contains recovered scalar values and synthetic geometry.
It supports native tests. It does not replace the installed export.

Run the installed export check after extraction:

```powershell
docker run --rm --mount "type=bind,source=$PWD,target=/workspace" --mount "type=volume,source=tld-wasm-cargo,target=/cargo" --mount "type=volume,source=tld-wasm-target,target=/target" my-summer-slop-toolchain cargo test --locked --test reference -- --ignored
```

Pinned package metadata: [pygltflib](https://pypi.org/pypi/pygltflib/json).
Unity defaults: [Physic Material](https://docs.unity3d.com/2021.3/Documentation/Manual/class-PhysicMaterial.html).

## Development support limits

Bevy loaded representative static and skinned glTF assets with all dependencies ready.
Chrome rendered the static cup. Skinned rendering and animation playback remain unverified.
Chrome loaded embedded Liberation Sans and decoded a representative WAV in the isolated media fixture.
The WAV reached its end with muted playback. This check does not establish audible output or game audio integration.
Six cubemap PNG previews decoded. Native BC6H DDS data preserves HDR values; browser HDR rendering remains unverified.
The synthetic rotated sprite matched six expected pixels. The installed sprite index contains no packed rotation.
All 42 source clips have validated normalized tracks. Sixteen have bound glTF packages.
The remaining 26 clips retain native records and normalized tracks with binding limits.
The source contains no morph channels. Synthetic morph fixtures cover geometry and channel validation.
Animation playback, humanoid retargeting, Unity shaders, particles, live reflections, and native UI adapters remain unverified.
See [final browser and command evidence](../../docs/verification.md).
