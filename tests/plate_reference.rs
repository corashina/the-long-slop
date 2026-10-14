use my_summer_slop::reference::{ItemId, PlateRecord, ReferenceManifest};
#[test]
fn plate_contract_rejects_invalid_font_data() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let _: &PlateRecord = &original.plates;
    for mutation in 0..7 {
        let mut manifest = original.clone();
        match mutation {
            0 => manifest.plates.font.characters.retain(|c| c.unicode != 65),
            1 => manifest.plates.font.glyphs[0].rect[0] = manifest.plates.font.atlas_extent[0] + 1,
            2 => manifest.plates.plates[1].item = manifest.plates.plates[0].item,
            3 => manifest.plates.plates[1].same_as = Some(ItemId(u32::MAX)),
            4 => manifest.plates.font.glyphs[0].metrics[0] = f32::NAN,
            5 => manifest.plates.plates[0].same_as = Some(manifest.plates.plates[1].item),
            _ => manifest.plates.plates[0].startup_target.0 = u32::MAX,
        }
        assert!(manifest.validate().is_err(), "mutation {mutation}");
    }
}
#[test]
#[ignore = "Requires installed source export"]
fn installed_plates_have_two_complete_bindings() {
    let manifest =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    assert_eq!(manifest.plates.plates.len(), 2);
    assert_eq!(manifest.plates.font.glyphs.len(), 250);
    assert_eq!(
        manifest
            .plates
            .plates
            .iter()
            .map(|p| p.source.path_id)
            .collect::<Vec<_>>(),
        [78750, 73955]
    );
}

#[test]
fn plate_contract_requires_fresh_export() {
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    value.as_object_mut().unwrap().remove("plates");
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
}
