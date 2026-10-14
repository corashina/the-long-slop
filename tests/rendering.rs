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

fn standard_fixture(
    keyword: &str,
    slot: &str,
    default: my_summer_slop::reference::TextureDefault,
) -> ReferenceManifest {
    use my_summer_slop::reference::{ShaderFamily, TextureDefault};
    let mut manifest =
        ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let material = &mut manifest.rendering.materials[1];
    material.shader_family = ShaderFamily::Standard;
    material.keywords = vec![keyword.to_owned()];
    material.properties.textures.clear();
    material.properties.defaults.clear();
    material.properties.exclusions.clear();
    material.properties.floats = [
        "_Cutoff",
        "_Glossiness",
        "_GlossMapScale",
        "_Metallic",
        "_BumpScale",
        "_OcclusionStrength",
        "_DetailNormalMapScale",
        "_UVSec",
        "_SrcBlend",
        "_DstBlend",
        "_ZWrite",
    ]
    .into_iter()
    .map(|field| (field.to_owned(), 0.))
    .collect();
    material
        .properties
        .defaults
        .insert("_MainTex".to_owned(), TextureDefault::White);
    material
        .properties
        .defaults
        .insert("_OcclusionMap".to_owned(), TextureDefault::White);
    material
        .properties
        .defaults
        .insert(slot.to_owned(), default);
    manifest
}

#[test]
fn rejects_missing_emission_color() {
    use my_summer_slop::reference::TextureDefault;
    let mut manifest = standard_fixture("_EMISSION", "_EmissionMap", TextureDefault::White);
    manifest.rendering.materials[1]
        .properties
        .colors
        .insert("_EmissionColor".to_owned(), [1.; 4]);
    assert!(manifest.validate().is_ok());
    manifest.rendering.materials[1]
        .properties
        .colors
        .remove("_EmissionColor");
    let error = manifest.validate().unwrap_err().to_string();
    assert!(error.contains("_EmissionColor"), "{error}");
}

#[test]
fn rejects_missing_parallax_scale() {
    use my_summer_slop::reference::TextureDefault;
    let mut manifest = standard_fixture("_PARALLAXMAP", "_ParallaxMap", TextureDefault::Black);
    manifest.rendering.materials[1]
        .properties
        .floats
        .insert("_Parallax".to_owned(), 0.02);
    assert!(manifest.validate().is_ok());
    manifest.rendering.materials[1]
        .properties
        .floats
        .remove("_Parallax");
    let error = manifest.validate().unwrap_err().to_string();
    assert!(error.contains("_Parallax"), "{error}");
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
        if asset.id == manifest.lighting.cycle.sky_geometry.asset {
            assert!(
                !manifest
                    .rendering
                    .materials
                    .iter()
                    .any(|record| record.asset == asset.id)
            );
            for mesh in gltf["meshes"].as_array().unwrap() {
                for primitive in mesh["primitives"].as_array().unwrap() {
                    assert!(
                        primitive.get("material").is_none(),
                        "sky geometry has a surface owner"
                    );
                }
            }
            continue;
        }
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
            assert_eq!(
                gltf["materials"][material.gltf_material as usize]["extras"]["asset"],
                serde_json::json!(asset.id.0)
            );
            assert_eq!(
                gltf["materials"][material.gltf_material as usize]["extras"]["gltf_material"],
                serde_json::json!(material.gltf_material)
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

#[test]
fn accepts_zero_alpha_standard_glass() {
    let mut input = fixture();
    let mut standard = standard_fixture(
        "_ALPHAPREMULTIPLY_ON",
        "_MainTex",
        my_summer_slop::reference::TextureDefault::White,
    );
    let glass = &mut standard.rendering.materials[1];
    glass.asset = my_summer_slop::reference::AssetId(1);
    glass.gltf_material = 0;
    glass.state.alpha_mode = my_summer_slop::reference::AlphaMode::Premultiply;
    glass.state.src_blend = 1;
    glass.state.dst_blend = 10;
    glass.state.z_write = false;
    input["rendering"]["materials"][0] = serde_json::to_value(glass).unwrap();
    input["rendering"]["materials"][0]["color"][3] = serde_json::json!(0.0);
    input["rendering"]["materials"][0]["properties"]["floats"]["_Mode"] = serde_json::json!(3.0);
    ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).unwrap();
}

#[test]
fn accepts_master_rope_family_without_source_color_or_textures() {
    use my_summer_slop::reference::{AlphaMode, ShaderFamily};
    let mut manifest =
        ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let rope = &mut manifest.rendering.materials[1];
    rope.shader_family = ShaderFamily::AmplifyRope;
    rope.condition = None;
    rope.color = [1.; 4];
    rope.keywords.clear();
    rope.properties.colors.clear();
    rope.properties.textures.clear();
    rope.properties.defaults.clear();
    rope.properties.exclusions.clear();
    rope.properties.floats = [
        ("_Sag".into(), 1.5),
        ("_SwaySpeed".into(), 0.1),
        ("_SwayStrength".into(), 1.5),
    ]
    .into();
    rope.state.alpha_mode = AlphaMode::Opaque;
    assert!(manifest.validate().is_ok());
    manifest.rendering.materials[1]
        .properties
        .colors
        .insert("_Color".into(), [1.; 4]);
    assert!(
        manifest
            .validate()
            .unwrap_err()
            .to_string()
            .contains("rope contract")
    );
}

#[test]
fn allows_only_sky_owned_geometry_without_surface_records() {
    let mut manifest =
        ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let sky = manifest.lighting.cycle.sky_geometry.asset;
    manifest
        .rendering
        .materials
        .retain(|record| record.asset != sky);
    assert!(manifest.validate().is_ok());
    let surface = manifest
        .assets
        .iter()
        .find(|asset| asset.id != sky)
        .unwrap()
        .id;
    manifest
        .rendering
        .materials
        .retain(|record| record.asset != surface);
    assert!(
        manifest
            .validate()
            .unwrap_err()
            .to_string()
            .contains("missing glTF material binding")
    );
}

#[test]
fn source_texture_banks_preserve_slots_and_alpha_caster() {
    use my_summer_slop::reference::MATERIAL_TEXTURE_SLOTS;
    let manifest = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let mut material = manifest.rendering.materials[0].clone();
    let template = material.properties.textures[0].clone();
    let set = |material: &mut my_summer_slop::reference::MaterialRecord, slots: &[usize]| {
        material.properties.textures = slots
            .iter()
            .map(|slot| {
                let mut texture = template.clone();
                texture.slot = MATERIAL_TEXTURE_SLOTS[*slot].to_owned();
                texture
            })
            .collect();
    };
    // Largest authored detail material. Input order does not change bank identity.
    set(&mut material, &[8, 7, 6, 3, 2, 1, 0]);
    assert_eq!(
        material.texture_banks().unwrap(),
        [
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            None,
            None,
            Some(4),
            Some(5),
            Some(6)
        ]
    );
    // The authored parallax material uses defaults for detail albedo and mask.
    set(&mut material, &[0, 1, 2, 3, 5, 8]);
    assert_eq!(
        material.texture_banks().unwrap(),
        [
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            None,
            Some(4),
            None,
            None,
            Some(5)
        ]
    );
    // Reserve absent MainTex so an alpha caster does not sample a normal map.
    set(&mut material, &[1, 6, 7, 8]);
    assert_eq!(
        material.texture_banks().unwrap(),
        [
            None,
            Some(1),
            None,
            None,
            None,
            None,
            Some(2),
            Some(3),
            Some(4)
        ]
    );
    set(&mut material, &[0, 1, 2, 3, 4, 5, 6, 7]);
    assert!(
        material
            .texture_banks()
            .unwrap_err()
            .to_string()
            .contains("seven texture banks")
    );
}
