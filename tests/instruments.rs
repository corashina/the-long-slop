use my_summer_slop::{
    reference::ReferenceManifest,
    simulation::instruments::{InstrumentSample, update_high_beam},
};
#[test]
fn high_beam_selects_checked_dashboard_materials() {
    let r = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let lamp = &r.lighting.high_beam;
    assert_eq!(lamp.signal, "headlight2");
    let off = update_high_beam(lamp, InstrumentSample { high_beam: false });
    let on = update_high_beam(lamp, InstrumentSample { high_beam: true });
    assert_eq!(
        (off.renderer, off.asset, off.material_index),
        (lamp.node, lamp.off.asset, lamp.off.gltf_material)
    );
    assert_eq!(
        (on.renderer, on.asset, on.material_index),
        (lamp.node, lamp.on.asset, lamp.on.gltf_material)
    );
}
