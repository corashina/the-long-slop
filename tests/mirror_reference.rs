use my_summer_slop::reference::{MirrorRecord, ReferenceManifest};
#[test]
fn mirror_contract_rejects_bad_inputs() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let _: &MirrorRecord = &original.mirrors;
    for mutation in 0..4 {
        let mut m = original.clone();
        match mutation {
            0 => m.mirrors.mirrors[1].owner = m.mirrors.mirrors[0].owner,
            1 => m.mirrors.mirrors[0].camera_node.0 = u32::MAX,
            2 => m.mirrors.target.extent[0] = 0,
            _ => m.mirrors.mirrors[0].near_plane = f32::NAN,
        }
        assert!(m.validate().is_err());
    }
}
#[test]
fn obsolete_manifest_requires_fresh_export() {
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    value.as_object_mut().unwrap().remove("mirrors");
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
}
#[test]
#[ignore = "Requires installed source export"]
fn installed_mirrors_have_three_complete_bindings() {
    let m =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    assert_eq!(m.mirrors.mirrors.len(), 3);
    assert_eq!(m.mirrors.target.extent, [256, 256]);
    for mirror in &m.mirrors.mirrors {
        assert_eq!(mirror.culling_mask, 73113395);
        assert_eq!(mirror.live_texture_slots, ["_MainTex", "_EmissionMap"]);
    }
}
