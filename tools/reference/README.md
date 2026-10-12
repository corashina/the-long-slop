# Installed reference export

Use Python 3.12. Run these commands from the repository root.
Prepare the host tools below before export.

```powershell
python -m venv tools/reference/.venv
tools/reference/.venv/Scripts/python.exe -m pip install -r tools/reference/requirements.txt
docker build --tag my-summer-slop-toolchain --file tools/Dockerfile .
tools/reference/.venv/Scripts/python.exe -m tools.reference.export --steam-root "C:/Program Files (x86)/Steam/steamapps/common/The Long Drive"
tools/reference/.venv/Scripts/python.exe -m unittest discover -s tools/reference/tests
./scripts/check.ps1 -Test reference
```

## Host managed-analysis tools

Install host PowerShell (`pwsh`). The verified host uses PowerShell 7.6.5 and .NET 10.0.11.
Python packages and the Docker image do not supply these host tools.
Use ICSharpCode.Decompiler NuGet package **11.1.0.9782**.
It needs System.Collections.Immutable and System.Reflection.Metadata version 9.0.0 or later.
The verified PowerShell host supplies version 10.0.0.0 for both assemblies.

Prepare an existing, trusted package archive:

```powershell
$packageArchive = Read-Host 'Path to ICSharpCode.Decompiler 11.1.0.9782 .nupkg'
$expected = '1f76df4d35193ba4eeb50ffcd84eae381a2c48c67f95c1104dc406759448a811'
if ((Get-FileHash -LiteralPath $packageArchive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) { throw 'Package hash mismatch' }
New-Item -ItemType Directory -Force .cache/ilspy | Out-Null
Copy-Item -LiteralPath $packageArchive -Destination .cache/ilspy/package.zip
Expand-Archive -LiteralPath .cache/ilspy/package.zip -DestinationPath .cache/ilspy/package
pwsh -NoProfile -File tools/reference/preflight-managed.ps1
```

Keep the package layout intact:

```text
.cache/ilspy/package/ICSharpCode.Decompiler.nuspec
.cache/ilspy/package/lib/netstandard2.0/ICSharpCode.Decompiler.dll
```

The nuspec SHA-256 is `73342037762503908bbb041748678e709071f82ff8568770920f63dbfe42847a`.
The DLL SHA-256 is `38e6abf7497845d79d9d01d56351a3203991356de1d3496b8c51142f495e9f9c`.
You can also copy the complete `package` directory from a prepared checkout, then run the preflight command.
The preflight loads tool assemblies and checks package, assembly, and dependency versions before missing library prerequisites run.
It does not read game assemblies or write analysis output.

Runtime derivation and `--stage runtime --verify` require Docker and the project toolchain image.
Build the image with `docker build --tag my-summer-slop-toolchain --file tools/Dockerfile .`.
Runtime verification sends manifest bytes to the native `validate-reference` command through stdin.
It uses the game's `ReferenceManifest` contract, the existing Cargo volumes, and a read-only checkout mount.
Missing Docker, failed validation, or an unavailable image stops acceptance before promotion.
Library-only verification does not use Docker.

The exporter accepts Steam app `1017180`, build `11140890`, with Unity `2021.3.18f1`.
It preserves the installed tree and complete object graph before converting development assets.
It derives the garage, Lada, supplies, road pole, sky, and selected world prototypes from the accepted library.
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
| Runtime | `--stage runtime` | `web/assets/reference/`, selected scenes, world prototypes, and gameplay records |

```powershell
python -m tools.reference.export --stage library --verify
python -m tools.reference.export --stage runtime --verify
```

Use the configured reference Python environment for these commands.
Library verification checks source hashes, typed records, dependencies, required media conversions, and installed counts.
Runtime verification also checks the native gameplay contract, selected file hashes, and exact scene dependencies.
Runtime derivation reads accepted records, glTF buffers, PNG files, and hash-checked archived lighting byte ranges.
It decodes each authored lighting mip without changing its size. It does not decode Unity meshes again.
Prerequisite library verification still reads archived source data for semantic checks.

Before writes, the exporter prints additional bytes, retained bytes, estimated peak bytes, and available disk bytes.
The free-space check charges only additional bytes. Existing files already consume disk space.
Fresh estimates cover build 11140890: 640 MiB source, 1 GiB records, 5 GiB categories, and 4 GiB assembly.
Category estimates include decoded media, managed analysis, and temporary native inputs.
Before assembly, complete category receipts include shared payloads, indexes, languages, and source text.
The assembly estimate adds 25 percent and 64 MiB for acceptance metadata and allocation overhead.
Runtime staging reserves 128 MiB. Existing output remains allocated until promotion succeeds.
Estimates cover file bytes and headroom; other processes can change available space during export.
Repeated catalog commands verify accepted records and keep managed analysis. Invalid stages fail without rebuilding accepted data.

The accepted source archive contains 183 files and 529,237,089 bytes.
Shared records and managed analysis contain 268,954 files and 698,121,145 bytes.
The published library contains 8,570 files and 3,105,818,196 bytes.
Run runtime derivation to add lighting files to the accepted library export.

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
Lighting records include separate hashes for shader records, native bytes, sprites, textures, and SMIP chains.
The runtime verifier checks these files before promotion.
Runtime lighting shader evidence retains accepted typed JSON and native program bytes.
`lighting-evidence.json` binds its records to the accepted library receipt and checked UnityPlayer.dll.
`parameter-evidence.json` records source fields, byte checks, startup rules, and material differences.

Startup conditions use a fixed build-based seed and original source ranges.
Rims and tires use the matching source material variants.
The exporter selects the highest source LOD and retains all transform IDs.

Mirror materials retain their authored factors. Live mirror camera output is omitted.
Plate text is omitted because serialized glyphs contain obsolete editor text.
The exporter maps supported Unity materials to glTF PBR and retains source properties and keywords.
The source renderer ports the approved lighting, cookie, shadow, sky, cloud, and flare paths.
Verify rendered appearance against a matched live source view before claiming pixel parity.

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
## Lighting authority and checks

Use selected scene component links from build 11140890.
The manifest retains cycle source `level1:64920`, ten controllers, seventeen light records, and eleven lighting textures.
The cycle has day 1200 seconds, night 300 seconds, dawn 240 seconds, and dusk 180 seconds.
Dawn and dusk lie within the day interval. Startup begins at 240 seconds.
The headlight controllers retain 50-metre low beams and 250-metre high beams.
Road poles retain 100-metre source spacing, 20-metre lamp range, and a 120-degree full cone.
Road pole and light transforms stay local to the prototype during loading.

Run installed lighting and lifecycle checks in the Docker toolchain:

```sh
cargo test --locked --test lighting_reference --test lighting_interaction --test starting_area -- --include-ignored
```

`production_lighting_records_match_selected_scene` checks source values, material links, and exported file hashes.
It uses the toolchain's `sha256sum` utility. It adds no runtime dependency.
The validator filters one exact pinned pygltflib provisional notice. Other warnings and validation errors remain visible.
Keep source recovery, analysis, scenario patches, and captures outside Git.

## Checked world export

Use an accepted library from another checkout without changing its files:

```powershell
$referencePython = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/tools/reference/.venv/Scripts/python.exe'
$libraryRoot = 'C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/web/assets/library'
& $referencePython -B -m tools.reference.export --library-root $libraryRoot
& $referencePython -B -m tools.reference.export --library-root $libraryRoot --verify
```

The runtime stage checks the external library before reading selected records.
It stages files under the current checkout's `.cache/reference-library/work` and promotes only its runtime directory.
`--library-root` is valid only for the runtime stage.
The required `world` contract rejects old manifests and unresolved source links.
Native and browser loaders share a 128 MiB manifest limit.
All prototype node indices belong to one prototype instance.
Road control counts and skin bone counts are separate.

The world contract uses the saved 5,000-metre source settings profile.
It requires Task 1's checked local samples and retained source runs.
Source captures, managed methods, and exported assets remain outside Git.
Synthetic fixtures test contract rejection; they do not prove world generation parity.

World selection includes the active object pools, active building types, and supported recursive physical props.
The catalog retains source weights, recursive category indices, null categories, and invalid source category outcomes.
Unsupported vehicles, actors, and part behavior have explicit exclusions.
Physical props retain their source identity when an action, such as eating or firing, is excluded.
The three terrain prefabs have null source mesh links.
Their TerrainBiome shader, textures, mip levels, and parameters remain available for generated terrain.
Dormant LOD bindings retain their original shaders, parameters, textures, mip levels, and render target descriptors.
Explicit null material slots and zero-vertex meshes remain distinct from unresolved links.
Later rendering work must admit each visible source binding before its LOD appears.
This export adds data. Later tasks connect the world generator and runtime instances.

Startup checks every world prototype file and dependency before Start becomes available.
Two workers check bounded file buffers and release them after SHA-256 validation.
Only the five existing scene, lamp, and sky assets load into Bevy at startup.
World prototype records remain available for later bounded loading.
