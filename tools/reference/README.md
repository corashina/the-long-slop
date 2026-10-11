# Installed reference export

Use Python 3.12. Run these commands from the repository root.

```powershell
python -m venv tools/reference/.venv
tools/reference/.venv/Scripts/python.exe -m pip install -r tools/reference/requirements.txt
tools/reference/.venv/Scripts/python.exe -m tools.reference.export --steam-root "C:/Program Files (x86)/Steam/steamapps/common/The Long Drive"
tools/reference/.venv/Scripts/python.exe -m unittest discover -s tools/reference/tests
./scripts/check.ps1 -Test reference
```

The exporter reads Steam app `1017180`, build `11140890`.
It writes home, car, supplies, and road pole glTF files into `web/assets/reference/`.
The same export writes the manifest, buffers, textures, decoded mip chains, and lighting evidence.
Git ignores these files. Steam files remain unchanged.

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

## Lighting authority and checks

Use selected scene component links from build 11140890.
The manifest retains cycle source `level1:64920`, ten controllers, seventeen light records, and eleven lighting textures.
The cycle has day 1200 seconds, night 300 seconds, dawn 240 seconds, and dusk 180 seconds.
Dawn and dusk lie within the day interval. Startup begins at 240 seconds.
The headlight controllers retain 50-metre low beams and 250-metre high beams.
Road poles retain 100-metre source spacing, 20-metre lamp range, and a 120-degree full cone.
Road pole source root 1230 and light node 1240 remain prototype-local during loading.

Run installed lighting and lifecycle checks in the Docker toolchain:

```sh
cargo test --locked --test lighting_reference --test lighting_interaction --test starting_area -- --include-ignored
```

`production_lighting_records_match_selected_scene` checks source values, material links, and exported file hashes.
It uses the toolchain's `sha256sum` utility. It adds no runtime dependency.
The validator filters one exact pinned pygltflib provisional notice. Other warnings and validation errors remain visible.
Keep source recovery, analysis, scenario patches, and captures outside Git.
