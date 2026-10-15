mod common;

use my_summer_slop::reference::ItemId;
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::starting_area::GarageRuntime;
use my_summer_slop::vehicle::{VehiclePose, to_rapier};

#[test]
fn one_primary_press_across_steps_takes_one_bite() {
    let reference = common::reference();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    let id = reference.survival.starter_food;
    let item = runtime.area.items.get(id).unwrap();
    let body = item.body.unwrap();
    let local = item.local_pose;
    let mut position = runtime.area.player.view_pose(&runtime.physics).position;
    position[2] -= 0.7;
    let pose = VehiclePose {
        position,
        rotation: [0., 0., 0., 1.],
    };
    runtime.physics.rapier_mut().bodies[body]
        .set_position(to_rapier(pose) * to_rapier(local).inverse(), true);
    runtime.area.items.sync_poses(&runtime.physics);
    let point = runtime.area.items.get(id).unwrap().pose.position;
    runtime
        .area
        .interaction
        .pickup_item(
            id,
            point,
            &runtime.area.player,
            &mut runtime.area.items,
            &mut runtime.physics,
        )
        .unwrap();
    let available = runtime.area.interaction.available_actions(
        &Default::default(),
        &runtime.area.player,
        &runtime.area.items,
        &runtime.physics,
        runtime.survival.output(),
    );
    assert_ne!(
        available.actions.0 & my_summer_slop::interaction::feedback::ActionMask::CONSUME,
        0
    );
    let mut input = FrameInput::default();
    input.merge(FrameInput {
        held: keys::PRIMARY,
        pressed: keys::PRIMARY,
        ..Default::default()
    });
    for _ in 0..12 {
        runtime.step(&reference, input.take_step());
    }
    assert_eq!(runtime.area.items.get(id).unwrap().food.unwrap().bite, 1);
    assert!((runtime.survival.pending().food - 0.5).abs() < 0.001);
    input.merge(FrameInput {
        released: keys::PRIMARY,
        ..Default::default()
    });
    runtime.step(&reference, input.take_step());
    assert_eq!(runtime.area.items.get(id).unwrap().food.unwrap().bite, 1);
    input.merge(FrameInput {
        held: keys::PRIMARY,
        pressed: keys::PRIMARY,
        ..Default::default()
    });
    runtime.step(&reference, input.take_step());
    assert_eq!(runtime.area.items.get(id).unwrap().food.unwrap().bite, 2);
}

#[test]
fn final_bite_removes_all_bindings() {
    let reference = common::reference();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    let id = reference.survival.starter_food;
    let body = runtime.area.items.get(id).unwrap().body.unwrap();
    let colliders = runtime.physics.rapier().bodies[body].colliders().to_vec();
    for _ in 0..6 {
        assert!(runtime.consume(&reference, id).unwrap());
    }
    assert!(runtime.area.items.get(id).unwrap().removed);
    assert!(runtime.physics.rapier().bodies.get(body).is_none());
    for collider in colliders {
        assert!(runtime.physics.rapier().colliders.get(collider).is_none());
    }
    assert_eq!(runtime.survival.pending().food, 3.);
    assert_eq!(runtime.survival.pending().hp, 0.375);
    assert_eq!(runtime.survival.pending().water, -1.);
    assert_eq!(runtime.area.interaction.held(), None);
    assert!(!runtime.consume(&reference, id).unwrap());
}

#[test]
fn particle_contact_does_not_debit_source_twice() {
    let reference = common::reference();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let source = ItemId(18);
    assert!(runtime.drink(&reference, source, 0.1).is_err());
    let emitted = {
        let fluid = runtime
            .area
            .items
            .get_mut(source)
            .unwrap()
            .fluid
            .as_mut()
            .unwrap();
        my_summer_slop::fluids::spill(fluid, 0.1)
    };
    let before = runtime
        .area
        .items
        .get(source)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture;
    let result = runtime
        .drink_contact(&reference, source, emitted, [0., 0., 0., 1., 0., 0.])
        .unwrap();
    assert_eq!(result.debited_liters, emitted);
    assert!(result.effect.water > 0.);
    assert_eq!(
        runtime
            .area
            .items
            .get(source)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture,
        before
    );
}
