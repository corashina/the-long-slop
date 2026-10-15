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
