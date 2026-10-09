mod common;

use desert_drive::interaction::items::{ItemLocation, ItemStore};
use desert_drive::interaction::{AimTarget, InteractionAction, InteractionState, SLOT_SENSOR_FLAG};
use desert_drive::player::PlayerState;
use desert_drive::reference::{
    AnchorId, BodyId, HingeRecord, ItemId, OpeningId, OpeningRecord, ReferenceManifest, TankRole,
    WheelSlotId,
};
use desert_drive::simulation::{FrameInput, keys};
use desert_drive::vehicle::{PhysicsWorld, VehiclePose};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

fn pose(position: [f32; 3]) -> VehiclePose {
    VehiclePose {
        position,
        rotation: [0.0, 0.0, 0.0, 1.0],
    }
}
fn target(id: u32) -> Option<AimTarget> {
    Some(AimTarget {
        item: ItemId(id),
        distance: 1.0,
        point: [10.0, 2.0, -1.0],
        slot: None,
    })
}
fn fixture() -> (InteractionState, ItemStore, PhysicsWorld, PlayerState) {
    fixture_with(|_| {})
}
fn fixture_with(
    rules: impl FnOnce(&mut ReferenceManifest),
) -> (InteractionState, ItemStore, PhysicsWorld, PlayerState) {
    let mut reference = common::reference();
    for (id, tank, open) in [(1, 1, false), (2, 2, false)] {
        reference.openings.push(OpeningRecord {
            id: OpeningId(id),
            source: reference.player.source.clone(),
            tank: ItemId(tank),
            cap: ItemId(tank),
            anchor: AnchorId(1),
            open,
            flow_liters_per_second: 0.5,
            tilt_degrees: 100.0,
            tilt_axis: [0.0, 0.0, -1.0],
            angle_disabled: false,
            spill_disabled: false,
            tube_connected: false,
            flow_radius: 0.02,
        });
        reference.tanks[(tank - 1) as usize]
            .openings
            .push(OpeningId(id));
    }
    let mut body = reference.bodies[1].clone();
    body.id = BodyId(90);
    body.parent_body = None;
    body.pose.translation = [10.0, 2.0, -1.0];
    reference.bodies.push(body);
    reference.items[0].body = BodyId(90);
    rules(&mut reference);
    let mut physics = common::world();
    let (handle, _) = physics.rapier_mut().insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(10.0, 2.0, -1.0)),
        ColliderBuilder::ball(0.2).user_data(2),
    );
    let mut items = ItemStore::new(&reference).unwrap();
    items.bind_body(BodyId(90), handle);
    items.get_mut(ItemId(1)).unwrap().local_pose = pose([0.0; 3]);
    let mut interaction = InteractionState::new(&reference).unwrap();
    interaction.bind_body(BodyId(90), handle);
    let player = PlayerState::new(&reference.player, pose([10.0, 1.0, 0.0]), &mut physics);
    (interaction, items, physics, player)
}
fn input(held: u32, pressed: u32) -> FrameInput {
    FrameInput {
        held,
        pressed,
        ..Default::default()
    }
}

#[test]
fn wall_blocks_target() {
    let (interaction, _, mut physics, _) = fixture();
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, 2.0, -0.4)),
        ColliderBuilder::cuboid(0.5, 0.5, 0.05),
    );
    physics.refresh_queries();
    assert!(
        interaction
            .aim(&physics, pose([10.0, 2.0, 0.0]), 3.0)
            .is_none()
    );
}

#[test]
fn target_sensor_is_usable_but_empty_trigger_is_skipped() {
    let (interaction, _, mut physics, _) = fixture();
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, 2.0, -0.4)),
        ColliderBuilder::ball(0.1).sensor(true),
    );
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, 2.0, -0.7)),
        ColliderBuilder::ball(0.1).sensor(true).user_data(3),
    );
    physics.refresh_queries();
    assert_eq!(
        interaction
            .aim(&physics, pose([10.0, 2.0, 0.0]), 3.0)
            .unwrap()
            .item,
        ItemId(2)
    );
}

#[test]
fn fill_requires_destination_open() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    items
        .get_mut(ItemId(1))
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [1.0; 6];
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let before = items
        .get(ItemId(2))
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    interaction.step(
        input(keys::FILL, 0),
        target(2),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert_eq!(
        items
            .get(ItemId(2))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total(),
        before
    );
    interaction.step(
        input(keys::USE, keys::USE),
        target(2),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    let total = items
        .get(ItemId(1))
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total()
        + before;
    interaction.step(
        input(keys::FILL, 0),
        target(2),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    let destination = items
        .get(ItemId(2))
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    assert!(destination > before);
    assert!((destination - before - 0.05).abs() < 1e-6);
    let source = items.get(ItemId(1)).unwrap().fluid.as_ref().unwrap();
    for quantity in source.mixture.quantities_liters {
        assert!((quantity - source.mixture.total() / 6.0).abs() < 1e-6);
    }
    assert!(
        (items
            .get(ItemId(1))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total()
            + destination
            - total)
            .abs()
            < 1e-5
    );
    interaction.step(
        input(keys::FILL, 0),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert_eq!(
        items
            .get(ItemId(2))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total(),
        destination
    );
    for next in [
        FrameInput::default(),
        FrameInput {
            held: keys::FILL,
            ..Default::default()
        },
        FrameInput {
            released: keys::FILL,
            ..Default::default()
        },
    ] {
        interaction.step(next, None, &mut player, &mut items, &mut physics, 0.1);
        assert_eq!(
            items
                .get(ItemId(2))
                .unwrap()
                .fluid
                .as_ref()
                .unwrap()
                .mixture
                .total(),
            destination
        );
    }
    interaction.step(
        input(keys::USE, keys::USE),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    let source_before = items
        .get(ItemId(1))
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    interaction.step(
        input(keys::FILL, 0),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert_eq!(
        items
            .get(ItemId(1))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total(),
        source_before
    );
    let destination_state = items.get_mut(ItemId(2)).unwrap().fluid.as_mut().unwrap();
    destination_state.capacity_liters = destination_state.mixture.total() + 0.001;
    interaction.step(
        input(keys::FILL, 0),
        target(2),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    let full = items
        .get(ItemId(2))
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    interaction.step(
        input(keys::FILL, 0),
        target(2),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert_eq!(
        items
            .get(ItemId(2))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total(),
        full
    );
}

#[test]
fn spill_requires_open_tilted_source() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let body = items.get(ItemId(1)).unwrap().body.unwrap();
    physics.rapier_mut().bodies[body].set_position(
        Pose::from_parts(
            Vector::new(10.0, 2.0, -1.0),
            Rotation::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        ),
        true,
    );
    let before = items
        .get(ItemId(1))
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    interaction.step(
        FrameInput::default(),
        None,
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert_eq!(
        items
            .get(ItemId(1))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total(),
        before
    );
    interaction.step(
        input(keys::USE, keys::USE),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    let after = items
        .get(ItemId(1))
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    assert!(after < before);
    interaction.step(
        input(keys::USE, keys::USE),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert_eq!(
        items
            .get(ItemId(1))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total(),
        after
    );
    physics.rapier_mut().bodies[body]
        .set_position(Pose::from_translation(Vector::new(10.0, 2.0, -1.0)), true);
    interaction.step(
        input(keys::USE, keys::USE),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    interaction.step(
        FrameInput::default(),
        None,
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert_eq!(
        items
            .get(ItemId(1))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total(),
        after
    );
}

#[test]
fn pickup_drop_preserves_identity_and_collision() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let count = items.len();
    let body = items.get(ItemId(1)).unwrap().body.unwrap();
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.held(), Some(ItemId(1)));
    assert_eq!(items.get(ItemId(1)).unwrap().location, ItemLocation::Held);
    assert!(physics.rapier().bodies[body].is_dynamic());
    assert!(
        physics.rapier().colliders[*physics.rapier().bodies[body].colliders().first().unwrap()]
            .is_enabled()
    );
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        None,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.held(), None);
    assert_eq!(items.len(), count);
    assert_eq!(items.get(ItemId(1)).unwrap().location, ItemLocation::World);
    assert!(physics.rapier().impulse_joints.is_empty());
    let bodies = physics.rapier().bodies.len();
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(physics.rapier().bodies.len(), bodies);
    interaction.drop_held(&mut items, &mut physics);
}

#[test]
fn carry_stops_at_a_solid_wall() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let body = items.get(ItemId(1)).unwrap().body.unwrap();
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, 2.0, -1.8)),
        ColliderBuilder::cuboid(2.0, 2.0, 0.1),
    );
    physics.refresh_queries();
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    for _ in 0..120 {
        interaction.step(
            FrameInput {
                wheel: 100.0,
                ..Default::default()
            },
            None,
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        physics.step_physics();
    }
    assert!(physics.rapier().bodies[body].translation().z > -1.7);
    assert!(
        interaction
            .aim(&physics, pose([10.0, 2.0, 0.0]), 1.0)
            .is_none()
    );
}

#[test]
fn seat_enters_on_release_and_exits_on_press() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let reference = common::reference();
    player.configure_seats(&reference.seats);
    interaction.step(
        input(keys::USE, keys::USE),
        target(17),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(player.seat().is_none());
    interaction.step(
        FrameInput {
            released: keys::USE,
            ..Default::default()
        },
        target(17),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(player.driving());
    player.step(
        FrameInput {
            mouse: [100.0, 0.0],
            ..Default::default()
        },
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::USE, keys::USE),
        None,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(player.seat().is_none());
}

#[test]
fn car_tanks_cannot_pick_up_the_chassis() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    items.bind_body(BodyId(2), physics.chassis_handle());
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(2),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(interaction.held().is_none());
    assert!(physics.rapier().impulse_joints.is_empty());
}

#[test]
fn aim_excludes_the_player_body() {
    let (mut interaction, _, mut physics, player) = fixture();
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, -1.0, 0.0)),
        ColliderBuilder::ball(0.2).sensor(true).user_data(3),
    );
    physics.refresh_queries();
    interaction.bind_player_body(player.body_handle());
    let mut view = pose([10.0, 3.0, 0.0]);
    view.rotation = Rotation::from_rotation_x(-std::f32::consts::FRAC_PI_2).to_array();
    assert_eq!(
        interaction.aim(&physics, view, 6.0).unwrap().item,
        ItemId(2)
    );
}

#[test]
fn aim_slot_flag_does_not_mark_a_loose_rim() {
    let (interaction, _, mut physics, _) = fixture();
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, 2.0, -0.6)),
        ColliderBuilder::ball(0.1)
            .sensor(true)
            .user_data(SLOT_SENSOR_FLAG | (2u128 << 96) | 14),
    );
    physics.refresh_queries();
    let selected = interaction
        .aim(&physics, pose([10.0, 2.0, 0.0]), 1.0)
        .unwrap();
    assert_eq!(selected.item, ItemId(13));
    assert_eq!(selected.slot, Some(WheelSlotId(2)));
    physics.rapier_mut().insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(10.0, 2.0, -0.3)),
        ColliderBuilder::ball(0.1).user_data(14),
    );
    physics.refresh_queries();
    assert_eq!(
        interaction
            .aim(&physics, pose([10.0, 2.0, 0.0]), 1.0)
            .unwrap()
            .slot,
        None
    );
}

#[test]
fn wheel_intents_preserve_state_for_the_wheel_owner() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let action = interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(5),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(action, Some(InteractionAction::WheelRemove(ItemId(5))));
    assert_eq!(
        items.get(ItemId(5)).unwrap().location,
        ItemLocation::WheelSlot(WheelSlotId(0))
    );
    let body = items.get(ItemId(1)).unwrap().body;
    let wheel = items.get_mut(ItemId(5)).unwrap();
    wheel.location = ItemLocation::World;
    wheel.body = body;
    wheel.local_pose = pose([0.0; 3]);
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(5),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let slot = AimTarget {
        slot: Some(WheelSlotId(0)),
        ..target(13).unwrap()
    };
    let action = interaction.step(
        input(keys::GRAB, keys::GRAB),
        Some(slot),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(
        action,
        Some(InteractionAction::WheelInstall {
            item: ItemId(5),
            target: WheelSlotId(0)
        })
    );
    assert_eq!(interaction.held(), Some(ItemId(5)));
    let action = interaction.step(
        input(keys::ASSEMBLE, keys::ASSEMBLE),
        None,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(action, Some(InteractionAction::WheelSplit(ItemId(5))));
    assert_eq!(interaction.held(), Some(ItemId(5)));
}

#[test]
fn supply_fills_only_an_open_held_container() {
    let (mut interaction, mut items, mut physics, mut player) =
        fixture_with(|reference| reference.tanks[0].role = TankRole::Supply);
    let (body, _) = physics.rapier_mut().insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(10.0, 2.0, -0.5)),
        ColliderBuilder::ball(0.2),
    );
    items.bind_body(BodyId(2), body);
    items.get_mut(ItemId(2)).unwrap().local_pose = pose([0.0; 3]);
    interaction.step(
        input(keys::USE, keys::USE),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(2),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let before = items
        .get(ItemId(2))
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    interaction.step(
        input(keys::FILL, 0),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert_eq!(
        items
            .get(ItemId(2))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total(),
        before
    );
    interaction.step(
        input(keys::USE, keys::USE),
        target(2),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::FILL, 0),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert!(
        items
            .get(ItemId(2))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total()
            > before
    );
}

#[test]
fn fill_applies_source_blocks_and_slow_oil() {
    for blocked_source in [false, true] {
        let (mut interaction, mut items, mut physics, mut player) = fixture_with(|reference| {
            reference.tanks[0].no_fill_from = blocked_source;
            reference.tanks[1].no_fill_this = !blocked_source;
        });
        interaction.step(
            input(keys::GRAB, keys::GRAB),
            target(1),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        interaction.step(
            input(keys::USE, keys::USE),
            target(2),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        let before = items
            .get(ItemId(2))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total();
        interaction.step(
            input(keys::FILL, 0),
            target(2),
            &mut player,
            &mut items,
            &mut physics,
            0.1,
        );
        assert_eq!(
            items
                .get(ItemId(2))
                .unwrap()
                .fluid
                .as_ref()
                .unwrap()
                .mixture
                .total(),
            before
        );
    }
    let (mut interaction, mut items, mut physics, mut player) = fixture_with(|reference| {
        reference.tanks[1].slow_oil = true;
        reference.tanks[0].quantities_liters = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        reference.tanks[1].quantities_liters = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    });
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::USE, keys::USE),
        target(2),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::FILL, 0),
        target(2),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert!(
        (items
            .get(ItemId(2))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total()
            - 1.001)
            .abs()
            < 1e-6
    );
}

#[test]
fn hinge_clamps_and_moves_its_collider() {
    let mut reference = common::reference();
    let mut body = reference.bodies[1].clone();
    body.id = BodyId(90);
    body.parent_body = Some(BodyId(2));
    reference.bodies.push(body);
    reference.hinges.push(HingeRecord {
        item: ItemId(4),
        source: reference.player.source.clone(),
        anchor: AnchorId(1),
        body: BodyId(90),
        axis: [0.0, 1.0, 0.0],
        closed_degrees: 0.0,
        min_degrees: -30.0,
        max_degrees: 0.0,
        drag_axis: [1.0, 0.0, 0.0],
        drag_input_axis: 0,
        drag_sensitivity: 1.0,
        input_inverted: [false, false],
        rotation_axes: [[0.0, 1.0, 0.0], [0.0; 3]],
        relative_drag: false,
        drag_anchor: AnchorId(1),
        motor_degrees_per_second: Some(60.0),
    });
    let mut physics = common::world();
    let (handle, collider) = physics.rapier_mut().insert(
        RigidBodyBuilder::kinematic_position_based().translation(Vector::new(0.0, 1.1, 0.0)),
        ColliderBuilder::cuboid(0.1, 0.5, 0.5).translation(Vector::X),
    );
    let mut interaction = InteractionState::new(&reference).unwrap();
    interaction.bind_body(BodyId(90), handle);
    interaction.bind_body(BodyId(2), physics.chassis_handle());
    let mut items = ItemStore::new(&reference).unwrap();
    let mut player = PlayerState::new(&reference.player, pose([10.0, 1.0, 0.0]), &mut physics);
    let before = *physics.rapier().colliders[collider].position();
    for _ in 0..120 {
        interaction.step(
            FrameInput {
                held: keys::USE,
                mouse: [100.0, 0.0],
                ..Default::default()
            },
            target(4),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        physics.step_physics();
        interaction.sync_hinges(&mut physics);
        physics.refresh_queries();
    }
    assert_eq!(interaction.hinge_angle(ItemId(4)), Some(-30.0));
    assert_ne!(*physics.rapier().colliders[collider].position(), before);
    let before_translation = physics.rapier().bodies[handle].translation();
    let chassis = physics.chassis_handle();
    let mut parent = *physics.rapier().bodies[chassis].position();
    parent.translation += Vector::new(5.0, 0.0, 0.0);
    physics.rapier_mut().bodies[chassis].set_position(parent, true);
    interaction.sync_hinges(&mut physics);
    physics.refresh_queries();
    assert!(
        (physics.rapier().bodies[handle].translation()
            - before_translation
            - Vector::new(5.0, 0.0, 0.0))
        .length()
            < 1e-5
    );
    assert_eq!(interaction.hinge_angle(ItemId(4)), Some(-30.0));
}
