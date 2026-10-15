mod common;
use common::aim_at;
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::starting_area::GarageRuntime;

#[test]
#[ignore = "requires installed reference export"]
fn production_left_click_drag_moves_door_and_holds_camera() {
    let reference = my_summer_slop::reference::ReferenceManifest::from_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    runtime.step(&reference, FrameInput::default());
    let door = reference
        .mechanisms
        .iter()
        .find(|m| m.source.path_id == 76010)
        .unwrap();
    aim_at(&mut runtime, &reference, door.item);
    let yaw = runtime.area.player.yaw_degrees();
    let pitch = runtime.area.player.pitch_degrees();
    let before = runtime
        .area
        .interaction
        .mechanisms
        .rotation_degrees(door.item);
    for step in 0..3 {
        runtime.step(
            &reference,
            FrameInput {
                held: keys::PRIMARY,
                pressed: if step == 0 { keys::PRIMARY } else { 0 },
                mouse: [-10.0, -10.0],
                ..Default::default()
            },
        );
        assert_eq!(
            runtime.area.interaction.dragging_mechanism(),
            Some(door.item)
        );
        assert_eq!(runtime.area.player.yaw_degrees(), yaw);
        assert_eq!(runtime.area.player.pitch_degrees(), pitch);
    }
    let after = runtime
        .area
        .interaction
        .mechanisms
        .rotation_degrees(door.item);
    assert_ne!(after, before);
    runtime.step(
        &reference,
        FrameInput {
            released: keys::PRIMARY,
            mouse: [1.0, 1.0],
            ..Default::default()
        },
    );
    assert_eq!(runtime.area.interaction.dragging_mechanism(), None);
    assert_ne!(runtime.area.player.yaw_degrees(), yaw);
    assert_ne!(runtime.area.player.pitch_degrees(), pitch);
    assert_eq!(
        runtime
            .area
            .interaction
            .mechanisms
            .rotation_degrees(door.item),
        after
    );
}
