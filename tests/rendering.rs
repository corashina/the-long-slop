use my_summer_slop::reference::ReferenceManifest;

fn fixture() -> serde_json::Value {
    serde_json::from_slice(include_bytes!("common/reference.json")).unwrap()
}

#[test]
fn rejects_invalid_material_bindings() {
    let mut missing = fixture();
    missing.as_object_mut().unwrap().remove("rendering");
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&missing).unwrap()).is_err());
    let mut input = fixture();
    input["rendering"]["materials"][0]["asset"] = serde_json::json!(u32::MAX);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    for (field, value) in [
        ("slot", serde_json::json!("_Unknown")),
        ("file", serde_json::json!("../unsafe.png")),
    ] {
        let mut input = fixture();
        input["rendering"]["materials"][0]["properties"]["textures"][0][field] = value;
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    }
    let mut input = fixture();
    let duplicate = input["rendering"]["materials"][0].clone();
    input["rendering"]["materials"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    let mut manifest =
        ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    manifest.rendering.materials[0].properties.textures[0].scale[0] = f32::NAN;
    assert!(manifest.validate().is_err());
}

#[test]
fn rejects_missing_mips_and_invalid_dimensions() {
    for (field, value) in [
        ("file", serde_json::json!("reference/missing.png")),
        ("width", serde_json::json!(7)),
    ] {
        let mut input = fixture();
        input["rendering"]["materials"][0]["properties"]["textures"][0]["mips"][1][field] = value;
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    }
}

#[test]
#[ignore = "Requires the local installed-build export"]
fn installed_materials_match_gltf_and_mips() {
    use my_summer_slop::reference::{ShaderFamily, TextureEncoding};
    let manifest =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    for asset in &manifest.assets {
        let gltf: serde_json::Value =
            serde_json::from_slice(&std::fs::read(format!("web/assets/{}", asset.path)).unwrap())
                .unwrap();
        for mesh in gltf["meshes"].as_array().unwrap() {
            for primitive in mesh["primitives"].as_array().unwrap() {
                assert!(
                    primitive["material"].as_u64().is_some(),
                    "unresolved source material"
                );
            }
        }
        let records: Vec<_> = manifest
            .rendering
            .materials
            .iter()
            .filter(|record| record.asset == asset.id)
            .collect();
        assert_eq!(records.len(), gltf["materials"].as_array().unwrap().len());
        for material in records {
            assert_eq!(
                serde_json::to_value(&material.source).unwrap(),
                gltf["materials"][material.gltf_material as usize]["extras"]["source"]
            );
            for texture in &material.properties.textures {
                if texture.slot == "_BumpMap" || texture.slot == "_DetailNormalMap" {
                    assert_eq!(texture.encoding, TextureEncoding::Rgb);
                }
                for mip in &texture.mips {
                    let bytes = std::fs::read(format!("web/assets/{}", mip.file)).unwrap();
                    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
                    assert_eq!(
                        u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
                        mip.width
                    );
                    assert_eq!(
                        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
                        mip.height
                    );
                }
            }
        }
    }
    let concrete = manifest
        .rendering
        .materials
        .iter()
        .find(|record| record.source.file == "sharedassets1.assets" && record.source.path_id == 393)
        .unwrap();
    assert_eq!(concrete.shader_family, ShaderFamily::Standard);
    assert!(concrete.keywords.iter().any(|key| key == "_DETAIL_MULX2"));
    for slot in [
        "_MainTex",
        "_BumpMap",
        "_DetailAlbedoMap",
        "_DetailNormalMap",
    ] {
        assert!(
            concrete
                .properties
                .textures
                .iter()
                .any(|texture| texture.slot == slot)
        );
    }
}
