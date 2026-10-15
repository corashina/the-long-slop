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
