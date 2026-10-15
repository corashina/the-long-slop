use my_summer_slop::reference::ReferenceManifest;
#[test]
fn rejects_missing_cookie_and_invalid_controller_links() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let mut m = original.clone();
    let cookie = m.lighting.lights.iter().find_map(|l| l.cookie).unwrap();
    m.lighting.textures.retain(|t| t.id != cookie);
    assert!(m.validate().is_err());
    let mut m = original.clone();
    m.lighting.controllers[0].node.0 = 999999;
    assert!(m.validate().is_err());
    let mut m = original.clone();
    m.lighting
        .controllers
        .push(m.lighting.controllers[0].clone());
    assert!(m.validate().is_err());
    let mut m = original;
    m.lighting.cycle.day_angle_min = f32::NAN;
    assert!(m.validate().is_err());
}
