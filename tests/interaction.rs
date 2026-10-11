mod common;

use my_summer_slop::interaction::items::{ItemLocation, ItemStore};
use my_summer_slop::interaction::{
    AimTarget, InteractionAction, InteractionState, SLOT_SENSOR_FLAG,
};
use my_summer_slop::player::PlayerState;
use my_summer_slop::reference::{
    AnchorId, BodyId, ItemId, OpeningId, OpeningRecord, ReferenceManifest, TankRole, WheelSlotId,
};
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::vehicle::{PhysicsWorld, VehiclePose};
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
        collider: rapier3d::prelude::ColliderHandle::invalid(),
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
    for (id, tank) in [(100, 1), (101, 2)] {
        reference.openings.push(OpeningRecord {
            id: OpeningId(id),
            source: reference.player.source.clone(),
            tank: ItemId(tank),
            cap: ItemId(tank),
            anchor: AnchorId(1),
            valve: 0.0,
            can_tune: false,
            tune_rate: 0.0,
            visuals: my_summer_slop::reference::CapVisualReference {
                open_nodes: vec![],
                closed_nodes: vec![],
                cap_node: None,
                closed_node: None,
                open_node: None,
                euler_rotation: false,
                closed_euler_degrees: [0.0; 3],
                open_euler_degrees: [0.0; 3],
            },
            flow_liters_per_second: 0.5,
            tilt_degrees: 100.0,
            tilt_axis: [0.0, 0.0, -1.0],
            angle_disabled: false,
            spill_disabled: false,
            tube_connected: false,
            flow_radius: 0.02,
            can_toggle: true,
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
    interaction.bind_body(reference.vehicle.body, physics.chassis_handle());
    let player = PlayerState::new(&reference.player, pose([10.0, 1.0, 0.0]), &mut physics);
    (interaction, items, physics, player)
}
fn door_fixture() -> (InteractionState, ItemStore, PhysicsWorld, PlayerState) {
    fixture_with(|reference| {
        common::add_rotation(reference, 1, 90, [-30.0, 0.0]);
    })
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
    assert!(common::focus(&interaction, &physics, pose([10.0, 2.0, 0.0])).is_none());
}

#[test]
fn source_use_mask_skips_hidden_cabin_envelope() {
    let (interaction, _, mut physics, _) = fixture();
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, 2.0, -0.4)),
        ColliderBuilder::cuboid(0.5, 0.5, 0.05).collision_groups(
            rapier3d::prelude::InteractionGroups::new(
                rapier3d::prelude::Group::from_bits_retain(1 << 13),
                rapier3d::prelude::Group::ALL,
                rapier3d::prelude::InteractionTestMode::And,
            ),
        ),
    );
    physics.refresh_queries();
    let target = common::focus(&interaction, &physics, pose([10.0, 2.0, 0.0]));
    assert_eq!(target.map(|target| target.item), Some(ItemId(1)));
}

#[test]
fn source_seat_hit_node_routes_to_primary_seat() {
    let mut reference = common::reference();
    let alias = reference.items[0].node;
    reference.seats[0].hit_nodes = vec![alias];
    assert_eq!(
        my_summer_slop::starting_area::collider_item(&reference, alias),
        Some(reference.seats[0].item),
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
        common::focus(&interaction, &physics, pose([10.0, 2.0, 0.0]))
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
        common::resolved(target(1)),
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
        common::resolved(target(2)),
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
        common::resolved(target(2)),
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
        common::resolved(target(2)),
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
        common::resolved(target(1)),
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
        interaction.step(
            next,
            common::resolved(None),
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
    }
    interaction.step(
        input(keys::USE, keys::USE),
        common::resolved(target(1)),
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
        common::resolved(target(1)),
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
        common::resolved(target(2)),
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
        common::resolved(target(2)),
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
        common::resolved(None),
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
        common::resolved(target(1)),
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
        common::resolved(target(1)),
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
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    interaction.step(
        FrameInput::default(),
        common::resolved(None),
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
        common::resolved(target(1)),
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
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        common::resolved(None),
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
        common::resolved(target(1)),
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
        common::resolved(target(1)),
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
            common::resolved(None),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        physics.step_physics();
    }
    assert!(physics.rapier().bodies[body].translation().z > -1.7);
    assert!(common::focus(&interaction, &physics, pose([10.0, 2.0, 0.0])).is_none());
}

#[test]
fn seat_enters_on_release_and_exits_on_press() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let reference = common::reference();
    player.configure_seats(&reference.seats);
    interaction.step(
        input(keys::USE, keys::USE),
        common::resolved(target(17)),
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
        common::resolved(target(17)),
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
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(player.seat().is_none());
}

#[test]
fn door_release_over_a_seat_does_not_enter() {
    let (mut interaction, mut items, mut physics, mut player) = door_fixture();
    player.configure_seats(&common::reference().seats);
    interaction.step(
        FrameInput {
            held: keys::USE,
            pressed: keys::USE,
            mouse: [3.0, 0.0],
            ..Default::default()
        },
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.dragging_mechanism(), Some(ItemId(1)));
    let action = interaction.step(
        FrameInput {
            released: keys::USE,
            ..Default::default()
        },
        common::resolved(target(17)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(action, None);
    assert!(player.seat().is_none());
    assert_eq!(interaction.dragging_mechanism(), None);
}

#[test]
fn bare_seat_release_can_enter_after_an_empty_target_press() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    player.configure_seats(&common::reference().seats);
    interaction.step(
        input(keys::USE, keys::USE),
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        FrameInput {
            released: keys::USE,
            ..Default::default()
        },
        common::resolved(target(17)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(player.driving());
}

#[test]
fn seated_door_drag_keeps_the_player_in_the_seat() {
    let (mut interaction, mut items, mut physics, mut player) = door_fixture();
    let reference = common::reference();
    player.configure_seats(&reference.seats);
    assert!(player.enter(reference.seats[0].id, pose([10.0, 0.25, 0.0]), &mut physics));
    player.step(
        FrameInput {
            mouse: [100.0, 0.0],
            ..Default::default()
        },
        &mut physics,
        1.0 / 60.0,
    );
    let action = interaction.step(
        FrameInput {
            held: keys::USE,
            pressed: keys::USE,
            mouse: [3.0, 0.0],
            ..Default::default()
        },
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(action, Some(InteractionAction::Mechanism(ItemId(1))));
    assert_eq!(player.seat(), Some(reference.seats[0].id));
    assert_eq!(interaction.dragging_mechanism(), Some(ItemId(1)));
    assert_eq!(
        interaction
            .mechanisms
            .rotation_degrees(ItemId(1))
            .map(|v| v[0]),
        Some(-3.0)
    );
    interaction.step(
        FrameInput {
            held: keys::USE,
            mouse: [3.0, 0.0],
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(
        interaction
            .mechanisms
            .rotation_degrees(ItemId(1))
            .map(|v| v[0]),
        Some(-6.0)
    );
    assert_eq!(player.seat(), Some(reference.seats[0].id));
    interaction.step(
        FrameInput {
            released: keys::USE,
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::USE, keys::USE),
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(player.seat().is_none());
}

#[test]
fn seated_cap_tuning_keeps_the_player_in_the_seat() {
    let (mut interaction, mut items, mut physics, mut player) = fixture_with(|reference| {
        reference.openings[0].can_toggle = false;
        reference.openings[0].can_tune = true;
        reference.openings[0].tune_rate = 0.525;
    });
    let reference = common::reference();
    player.configure_seats(&reference.seats);
    assert!(player.enter(reference.seats[0].id, pose([10.0, 0.25, 0.0]), &mut physics));
    player.step(
        FrameInput {
            mouse: [100.0, 0.0],
            ..Default::default()
        },
        &mut physics,
        1.0 / 60.0,
    );
    let action = interaction.step(
        FrameInput {
            held: keys::USE,
            pressed: keys::USE,
            mouse: [0.0, 5.0],
            ..Default::default()
        },
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(action, Some(InteractionAction::Cap(OpeningId(100))));
    assert_eq!(player.seat(), Some(reference.seats[0].id));
    assert!(interaction.opening_valve(OpeningId(100)) > 0.0);
}

#[test]
fn car_tanks_cannot_pick_up_the_chassis() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    items.bind_body(BodyId(2), physics.chassis_handle());
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(target(2)),
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
    let (mut interaction, _, mut physics, player) =
        fixture_with(|reference| reference.player.aim_reach = 6.0);
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, -1.0, 0.0)),
        ColliderBuilder::ball(0.2).sensor(true).user_data(3),
    );
    physics.refresh_queries();
    interaction.bind_player_body(player.body_handle());
    let mut view = pose([10.0, 3.0, 0.0]);
    view.rotation = Rotation::from_rotation_x(-std::f32::consts::FRAC_PI_2).to_array();
    assert_eq!(
        common::focus(&interaction, &physics, view).unwrap().item,
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
    let selected = common::focus(&interaction, &physics, pose([10.0, 2.0, 0.0])).unwrap();
    assert_eq!(selected.item, ItemId(13));
    assert_eq!(
        selected.slot,
        Some(my_summer_slop::interaction::SlotTarget::Wheel(WheelSlotId(
            2
        )))
    );
    physics.rapier_mut().insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(10.0, 2.0, -0.3)),
        ColliderBuilder::ball(0.1).user_data(14),
    );
    physics.refresh_queries();
    assert_eq!(
        common::focus(&interaction, &physics, pose([10.0, 2.0, 0.0]))
            .unwrap()
            .slot,
        None
    );
}

#[test]
fn wheel_intents_preserve_state_for_the_wheel_owner() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let wheel_id = common::reference().wheel_slots[0].rim;
    let action = interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(target(wheel_id.0)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(action, Some(InteractionAction::WheelRemove(wheel_id)));
    assert_eq!(
        items.get(wheel_id).unwrap().location,
        ItemLocation::WheelSlot(WheelSlotId(0))
    );
    let body = items.get(ItemId(1)).unwrap().body;
    let wheel = items.get_mut(wheel_id).unwrap();
    wheel.location = ItemLocation::World;
    wheel.body = body;
    wheel.local_pose = pose([0.0; 3]);
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(target(wheel_id.0)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let slot = AimTarget {
        slot: Some(my_summer_slop::interaction::SlotTarget::Wheel(WheelSlotId(
            0,
        ))),
        ..target(13).unwrap()
    };
    let action = interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(Some(slot)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(
        action,
        Some(InteractionAction::WheelInstall {
            item: wheel_id,
            target: WheelSlotId(0)
        })
    );
    assert_eq!(interaction.held(), Some(wheel_id));
    let action = interaction.step(
        input(keys::ASSEMBLE, keys::ASSEMBLE),
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(action, Some(InteractionAction::WheelSplit(wheel_id)));
    assert_eq!(interaction.held(), Some(wheel_id));
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
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(target(2)),
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
        common::resolved(target(1)),
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
        common::resolved(target(2)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::FILL, 0),
        common::resolved(target(1)),
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
            common::resolved(target(1)),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        interaction.step(
            input(keys::USE, keys::USE),
            common::resolved(target(2)),
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
            common::resolved(target(2)),
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
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::USE, keys::USE),
        common::resolved(target(2)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::FILL, 0),
        common::resolved(target(2)),
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
    common::add_rotation(&mut reference, 4, 90, [-30.0, 0.0]);
    let mut physics = common::world();
    let (handle, collider) = physics.rapier_mut().insert(
        RigidBodyBuilder::kinematic_position_based().translation(Vector::new(0.0, 1.1, 0.0)),
        ColliderBuilder::cuboid(0.1, 0.5, 0.5).translation(Vector::X),
    );
    let mut interaction = InteractionState::new(&reference).unwrap();
    interaction.bind_body(BodyId(90), handle);
    interaction.bind_body(reference.vehicle.body, physics.chassis_handle());
    interaction.bind_body(BodyId(2), physics.chassis_handle());
    let mut items = ItemStore::new(&reference).unwrap();
    let mut player = PlayerState::new(&reference.player, pose([10.0, 1.0, 0.0]), &mut physics);
    let before = *physics.rapier().colliders[collider].position();
    interaction.step(
        FrameInput {
            held: keys::USE,
            mouse: [1.0, 0.0],
            ..Default::default()
        },
        common::resolved(target(4)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.dragging_mechanism(), Some(ItemId(4)));
    let first = interaction
        .mechanisms
        .rotation_degrees(ItemId(4))
        .map(|v| v[0])
        .unwrap();
    interaction.step(
        FrameInput {
            held: keys::USE,
            mouse: [1.0, 0.0],
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(
        interaction
            .mechanisms
            .rotation_degrees(ItemId(4))
            .map(|v| v[0])
            .unwrap()
            < first
    );
    interaction.step(
        FrameInput {
            released: keys::USE,
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.dragging_mechanism(), None);
    for _ in 0..120 {
        interaction.step(
            FrameInput {
                held: keys::USE,
                mouse: [100.0, 0.0],
                ..Default::default()
            },
            common::resolved(target(4)),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        physics.step_physics();
        interaction.sync_mechanisms(&mut physics);
        physics.refresh_queries();
    }
    assert_eq!(
        interaction
            .mechanisms
            .rotation_degrees(ItemId(4))
            .map(|v| v[0]),
        Some(-30.0)
    );
    assert_ne!(*physics.rapier().colliders[collider].position(), before);
    let before_translation = physics.rapier().bodies[handle].translation();
    let chassis = physics.chassis_handle();
    let mut parent = *physics.rapier().bodies[chassis].position();
    parent.translation += Vector::new(5.0, 0.0, 0.0);
    physics.rapier_mut().bodies[chassis].set_position(parent, true);
    interaction.sync_mechanisms(&mut physics);
    physics.refresh_queries();
    assert!(
        (physics.rapier().bodies[handle].translation()
            - before_translation
            - Vector::new(5.0, 0.0, 0.0))
        .length()
            < 1e-5
    );
    assert_eq!(
        interaction
            .mechanisms
            .rotation_degrees(ItemId(4))
            .map(|v| v[0]),
        Some(-30.0)
    );
}

#[test]
fn empty_slot_does_not_pick_up_a_remote_rim_and_tire_targets_its_assembly() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let reference = common::reference();
    let rim = reference.wheel_slots[0].rim;
    let tire = reference.wheel_slots[0].tire;
    let body = items.get(ItemId(1)).unwrap().body;
    let state = items.get_mut(rim).unwrap();
    state.location = ItemLocation::World;
    state.body = body;
    state.local_pose = pose([0.0; 3]);
    let slot = AimTarget {
        slot: Some(my_summer_slop::interaction::SlotTarget::Wheel(
            reference.wheel_slots[0].id,
        )),
        ..target(rim.0).unwrap()
    };
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(Some(slot)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.held(), None);
    let (_, collider) = physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, 2.0, -0.4)),
        ColliderBuilder::ball(0.1)
            .sensor(true)
            .user_data(tire.0 as u128 + 1),
    );
    interaction.bind_surface(
        collider,
        my_summer_slop::reference::ActionBinding {
            collider: my_summer_slop::reference::ColliderId(0),
            use_owner: None,
            pickup_owner: Some(tire),
            removal_owner: Some(tire),
            push_owner: None,
        },
    );
    physics.refresh_queries();
    let aim = interaction.resolve(&physics, pose([10.0, 2.0, 0.0]), &items);
    assert_eq!(aim.grab_target.map(|hit| hit.item), Some(rim));
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        aim,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.held(), Some(rim));
}

#[test]
fn runtime_pump_stays_open_and_fills_the_held_container() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let pump = ItemId(90);
    items.insert_supply(pump, pose([10.0, 2.0, -1.0]));
    interaction.register_pump(pump, 0.5).unwrap();
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::USE, keys::USE),
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::USE, keys::USE),
        common::resolved(target(pump.0)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(interaction.opening_is_open(OpeningId(pump.0)));
    let before = items
        .get(ItemId(1))
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    interaction.step(
        input(keys::FILL, 0),
        common::resolved(target(pump.0)),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert!(
        (items
            .get(ItemId(1))
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .total()
            - before
            - 0.05)
            .abs()
            < 1e-6
    );
}

#[test]
fn faucet_tuning_controls_fractional_fill_and_clamps() {
    let (mut interaction, mut items, mut physics, mut player) = fixture_with(|reference| {
        reference.items[0].kind = my_summer_slop::reference::ItemKind::Cap;
        reference.tanks[0].role = TankRole::Supply;
        reference.tanks[0].capacity_liters = 1000.0;
        reference.tanks[0].quantities_liters = [0.0, 0.0, 0.0, 69.0, 0.0, 0.0];
        let faucet = reference
            .openings
            .iter_mut()
            .find(|opening| opening.cap == ItemId(1))
            .unwrap();
        faucet.can_toggle = false;
        faucet.can_tune = true;
        faucet.tune_rate = 0.525;
        faucet.angle_disabled = true;
        faucet.flow_liters_per_second = 1.4137167;
    });
    let (body, _) = physics.rapier_mut().insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(10.0, 2.0, -0.5)),
        ColliderBuilder::ball(0.2),
    );
    items.bind_body(BodyId(2), body);
    items.get_mut(ItemId(2)).unwrap().local_pose = pose([0.0; 3]);
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(target(2)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::USE, keys::USE),
        common::resolved(target(2)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::USE, keys::USE),
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.opening_valve(OpeningId(100)), 0.0);
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
        common::resolved(target(1)),
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
        FrameInput {
            held: keys::USE,
            mouse: [0.0, 5.0],
            ..Default::default()
        },
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    assert!((interaction.opening_valve(OpeningId(100)) - 0.2625).abs() < 1e-6);
    assert_eq!(interaction.tuning_opening(), Some(OpeningId(100)));
    interaction.step(
        FrameInput {
            wheel: 1.0,
            ..Default::default()
        },
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        0.01,
    );
    let valve = interaction.opening_valve(OpeningId(100));
    assert!((valve - 0.3675).abs() < 1e-6);
    interaction.step(
        input(keys::FILL, 0),
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        0.1,
    );
    let accepted = items
        .get(ItemId(2))
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total()
        - before;
    assert!((accepted - 1.4137167 * valve * 0.1).abs() < 1e-6);
    for (wheel, expected) in [(1000.0, 1.0), (-1000.0, 0.0)] {
        interaction.step(
            FrameInput {
                wheel,
                ..Default::default()
            },
            common::resolved(target(1)),
            &mut player,
            &mut items,
            &mut physics,
            0.01,
        );
        assert_eq!(interaction.opening_valve(OpeningId(100)), expected);
    }
}

#[test]
fn cap_surface_resolves_use_and_carry_owner() {
    let mut reference = common::reference();
    reference.openings[0].can_tune = false;
    reference.openings[0].can_toggle = true;
    reference.action_bindings[0].pickup_owner = Some(reference.openings[0].tank);
    let cap = reference.openings[0].cap;
    let can = reference.openings[0].tank;
    let mut interaction = InteractionState::new(&reference).unwrap();
    let mut items = ItemStore::new(&reference).unwrap();
    let mut physics = common::world();
    let (body, collider) = physics.rapier_mut().insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(10.0, 2.0, -1.0)),
        ColliderBuilder::ball(0.2)
            .user_data(cap.0 as u128 + 1)
            .collision_groups(rapier3d::prelude::InteractionGroups::new(
                rapier3d::prelude::Group::from_bits_retain(1 << 22),
                rapier3d::prelude::Group::ALL,
                rapier3d::prelude::InteractionTestMode::And,
            )),
    );
    let body_id = reference
        .items
        .iter()
        .find(|item| item.id == can)
        .unwrap()
        .body;
    items.bind_body(body_id, body);
    interaction.bind_body(body_id, body);
    interaction.bind_surface(
        collider,
        my_summer_slop::reference::ActionBinding {
            collider: my_summer_slop::reference::ColliderId(0),
            use_owner: Some(cap),
            pickup_owner: Some(can),
            removal_owner: None,
            push_owner: None,
        },
    );
    physics.refresh_queries();
    let aim = interaction.resolve(&physics, pose([10.0, 2.0, 0.0]), &items);
    assert_eq!(aim.use_target.map(|hit| hit.item), Some(cap));
    assert_eq!(aim.grab_target.map(|hit| hit.item), Some(can));
    assert_eq!(aim.use_target.unwrap().collider, collider);
    assert_eq!(aim.grab_target.unwrap().collider, collider);
    assert_eq!(interaction.prompt(&aim, &items), "E: cap · F: carry");
    let mut player = PlayerState::new(&reference.player, pose([10.0, 1.0, 0.0]), &mut physics);
    assert_eq!(
        interaction.step(
            input(keys::USE, keys::USE),
            aim,
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0
        ),
        Some(InteractionAction::Cap(reference.openings[0].id))
    );
    assert_eq!(
        interaction.step(
            input(keys::GRAB, keys::GRAB),
            aim,
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0
        ),
        Some(InteractionAction::Pickup(can))
    );
}

#[test]
fn pickup_mask_does_not_cross_a_solid_wall() {
    let (mut interaction, items, mut physics, _) = fixture_with(|reference| {
        reference.player.action_masks.use_mask = 1;
        reference.player.action_masks.pickup_mask = 1 << 23;
    });
    let collider = physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, collider)| collider.user_data == 2)
        .unwrap()
        .0;
    physics.rapier_mut().colliders[collider].set_collision_groups(
        rapier3d::prelude::InteractionGroups::new(
            rapier3d::prelude::Group::from_bits_retain(1 << 23),
            rapier3d::prelude::Group::ALL,
            rapier3d::prelude::InteractionTestMode::And,
        ),
    );
    interaction.bind_surface(
        collider,
        my_summer_slop::reference::ActionBinding {
            collider: my_summer_slop::reference::ColliderId(0),
            use_owner: None,
            pickup_owner: Some(ItemId(1)),
            removal_owner: None,
            push_owner: None,
        },
    );
    physics.refresh_queries();
    let aim = interaction.resolve(&physics, pose([10.0, 2.0, 0.0]), &items);
    assert_eq!(aim.grab_target.map(|hit| hit.item), Some(ItemId(1)));
    assert!(aim.use_target.is_none());
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, 2.0, -0.4)),
        ColliderBuilder::cuboid(0.5, 0.5, 0.05),
    );
    physics.refresh_queries();
    let blocked_aim = interaction.resolve(&physics, pose([10.0, 2.0, 0.0]), &items);
    assert!(blocked_aim.grab_target.is_none());
}

#[test]
fn held_container_at_wheel_slot_shows_incompatibility_and_stays_held() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let can = ItemId(1);
    assert_eq!(
        interaction.step(
            input(keys::GRAB, keys::GRAB),
            common::resolved(target(can.0)),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0
        ),
        Some(InteractionAction::Pickup(can))
    );
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, 2.0, -0.5)),
        ColliderBuilder::ball(0.1)
            .sensor(true)
            .user_data(SLOT_SENSOR_FLAG | 14),
    );
    physics.refresh_queries();
    let aim = interaction.resolve(&physics, pose([10.0, 2.0, 0.0]), &items);
    assert_eq!(
        aim.installation_block,
        Some(my_summer_slop::interaction::InteractionBlock::Incompatible)
    );
    assert!(aim.installation_target.is_none());
    assert_eq!(interaction.prompt(&aim, &items), "This part does not fit.");
    let joints = physics.rapier().impulse_joints.len();
    assert_eq!(
        interaction.step(
            input(keys::GRAB, keys::GRAB),
            aim,
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0
        ),
        None
    );
    assert_eq!(interaction.held(), Some(can));
    assert_eq!(items.get(can).unwrap().location, ItemLocation::Held);
    assert_eq!(physics.rapier().impulse_joints.len(), joints);
}

#[test]
fn carry_translation_roll_and_recenter_respect_collision() {
    use keys::{RECENTER, TRANSLATE};
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let body = items.get(ItemId(1)).unwrap().body.unwrap();
    physics.rapier_mut().bodies[body].set_gravity_scale(0.0, true);
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let anchor = physics
        .rapier()
        .bodies
        .iter()
        .find(|(id, rb)| *id != player.body_handle() && rb.is_kinematic())
        .unwrap()
        .0;
    let before = *physics.rapier().bodies[anchor].next_position();
    interaction.step(
        FrameInput {
            held: TRANSLATE,
            mouse: [20.0, 0.0],
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let translated = *physics.rapier().bodies[anchor].next_position();
    assert!(translated.translation.x > before.translation.x);
    assert_eq!(translated.rotation, before.rotation);
    interaction.step(
        FrameInput {
            held: keys::ROTATE | TRANSLATE,
            mouse: [20.0, 0.0],
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let rolled = *physics.rapier().bodies[anchor].next_position();
    assert_eq!(rolled.translation, translated.translation);
    assert!((rolled.rotation * Vector::Z - Vector::Z).length() < 0.0001);
    assert_ne!(rolled.rotation, translated.rotation);
    assert!((rolled.rotation * Vector::X).y < 0.0);
    interaction.step(
        FrameInput {
            held: keys::ROTATE,
            mouse: [0.0, 20.0],
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let pitched = *physics.rapier().bodies[anchor].next_position();
    assert_eq!(pitched.translation, rolled.translation);
    assert_ne!(pitched.rotation * Vector::Z, rolled.rotation * Vector::Z);
    interaction.step(
        FrameInput {
            pressed: RECENTER,
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let centered = *physics.rapier().bodies[anchor].next_position();
    assert!((centered.translation.x - 10.0).abs() < 0.001);
    assert_eq!(centered.rotation, Rotation::IDENTITY);
    interaction.step(
        FrameInput {
            held: keys::ROTATE,
            mouse: [20.0, 0.0],
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let yawed = *physics.rapier().bodies[anchor].next_position();
    assert_eq!(yawed.translation, centered.translation);
    assert!((yawed.rotation * -Vector::Z).x < 0.0);

    for mode in 0..5 {
        let mut free_pose = Pose::IDENTITY;
        for blocked in [false, true] {
            let (mut interaction, mut items, mut physics, mut player) = fixture_with(|reference| {
                let head_height = reference.player.standing_height * 0.5;
                let rule = reference.items[0].pickup.as_mut().unwrap();
                rule.use_recenter_position = true;
                rule.recenter_position = [0.75, 1.0 - head_height, 0.0];
            });
            let body = items.get(ItemId(1)).unwrap().body.unwrap();
            let collider = physics.rapier().bodies[body].colliders()[0];
            physics.rapier_mut().bodies[body].set_gravity_scale(0.0, true);
            if mode >= 2 {
                let half = if mode == 4 {
                    [0.4, 0.08, 0.08]
                } else {
                    [0.08, 0.08, 0.4]
                };
                physics.rapier_mut().colliders[collider].set_shape(
                    rapier3d::prelude::SharedShape::cuboid(half[0], half[1], half[2]),
                );
            }
            let wall_pose = Pose::from_translation(if mode < 2 {
                Vector::new(10.5, 2.0, -1.0)
            } else if mode == 2 {
                Vector::new(10.3, 2.0, -1.0)
            } else {
                Vector::new(10.0, 2.3, -1.0)
            });
            let wall_shape = rapier3d::prelude::SharedShape::cuboid(
                if mode <= 2 { 0.05 } else { 2.0 },
                if mode <= 2 { 2.0 } else { 0.05 },
                2.0,
            );
            if blocked {
                physics.rapier_mut().insert(
                    RigidBodyBuilder::fixed().pose(wall_pose),
                    ColliderBuilder::new(wall_shape.clone()),
                );
            }
            physics.refresh_queries();
            interaction.step(
                input(keys::GRAB, keys::GRAB),
                common::resolved(target(1)),
                &mut player,
                &mut items,
                &mut physics,
                1.0 / 60.0,
            );
            let anchor = physics
                .rapier()
                .bodies
                .iter()
                .find(|(id, rb)| *id != player.body_handle() && rb.is_kinematic())
                .unwrap()
                .0;
            let requested = match mode {
                0 => FrameInput {
                    held: TRANSLATE,
                    mouse: [180.0, 0.0],
                    ..Default::default()
                },
                1 => FrameInput {
                    held: RECENTER,
                    ..Default::default()
                },
                2 => FrameInput {
                    held: keys::ROTATE,
                    mouse: [67.5, 0.0],
                    ..Default::default()
                },
                3 => FrameInput {
                    held: keys::ROTATE,
                    mouse: [0.0, 67.5],
                    ..Default::default()
                },
                _ => FrameInput {
                    held: keys::ROTATE | TRANSLATE,
                    mouse: [67.5, 0.0],
                    ..Default::default()
                },
            };
            for frame in 0..120 {
                let input = if frame == 0 {
                    requested
                } else {
                    FrameInput {
                        held: requested.held,
                        ..Default::default()
                    }
                };
                interaction.step(
                    input,
                    common::resolved(None),
                    &mut player,
                    &mut items,
                    &mut physics,
                    1.0 / 60.0,
                );
                physics.step_physics();
                if blocked {
                    let shape = &physics.rapier().colliders[collider];
                    if let Some(contact) = rapier3d::parry::query::contact(
                        shape.position(),
                        shape.shape(),
                        &wall_pose,
                        wall_shape.as_ref(),
                        0.0,
                    )
                    .unwrap()
                    {
                        assert!(
                            contact.dist >= -0.001,
                            "mode {mode}, frame {frame}: {contact:?}"
                        );
                    }
                }
                assert_eq!(interaction.held(), Some(ItemId(1)), "mode {mode}");
            }
            let final_pose = *physics.rapier().bodies[anchor].next_position();
            if !blocked {
                free_pose = final_pose;
                if mode < 2 {
                    assert!(free_pose.translation.x > 10.6, "mode {mode}");
                } else {
                    assert_ne!(free_pose.rotation, Rotation::IDENTITY, "mode {mode}");
                    let contact = rapier3d::parry::query::contact(
                        &free_pose,
                        physics.rapier().colliders[collider].shape(),
                        &wall_pose,
                        wall_shape.as_ref(),
                        0.0,
                    )
                    .unwrap()
                    .expect("Requested rotation crosses the wall");
                    assert!(contact.dist < -0.01, "mode {mode}: {contact:?}");
                }
            } else if mode < 2 {
                assert!(
                    final_pose.translation.x > 10.0,
                    "mode {mode}: movement was attempted"
                );
                assert!(final_pose.translation.x < 10.26, "mode {mode}");
                assert!(
                    final_pose.translation.x < free_pose.translation.x,
                    "mode {mode}"
                );
            } else {
                assert_eq!(final_pose.rotation, Rotation::IDENTITY, "mode {mode}");
                assert_ne!(final_pose.rotation, free_pose.rotation, "mode {mode}");
            }
        }
    }
}

#[test]
fn carry_uses_item_recenter_pose_and_source_pickup_restrictions() {
    let rotation = Rotation::from_rotation_y(0.4);
    let (mut interaction, mut items, mut physics, mut player) = fixture_with(|reference| {
        let rule = reference.items[0].pickup.as_mut().unwrap();
        rule.use_recenter_position = true;
        rule.recenter_position = [0.2, 0.1, 0.0];
        rule.use_recenter_rotation = true;
        rule.recenter_rotation = rotation.to_array();
    });
    let body = items.get(ItemId(1)).unwrap().body.unwrap();
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        FrameInput {
            pressed: keys::RECENTER,
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let anchor = physics
        .rapier()
        .bodies
        .iter()
        .find(|(id, rb)| *id != player.body_handle() && rb.is_kinematic())
        .unwrap()
        .1;
    let view = my_summer_slop::vehicle::to_rapier(player.view_pose(&physics));
    let current = physics.rapier().bodies[body].translation();
    let desired = view.translation + Vector::new(0.2, 0.1, -1.0);
    let expected = current + (desired - current) * 10.0 / 60.0;
    assert!((anchor.next_position().translation - expected).length() < 0.0001);
    assert!((anchor.next_position().rotation * Vector::Z - rotation * Vector::Z).length() < 0.0001);
    for restriction in [0, 1] {
        let (mut interaction, mut items, mut physics, mut player) = fixture_with(|reference| {
            if restriction == 0 {
                reference.items[0].pickup.as_mut().unwrap().cant_pickup = true;
            } else {
                reference.items[0].pickup = None;
            }
        });
        assert_eq!(
            interaction.step(
                input(keys::GRAB, keys::GRAB),
                common::resolved(target(1)),
                &mut player,
                &mut items,
                &mut physics,
                1.0 / 60.0
            ),
            None
        );
        assert_eq!(interaction.held(), None);
        assert_eq!(items.get(ItemId(1)).unwrap().location, ItemLocation::World);
        assert!(physics.rapier().impulse_joints.is_empty());
    }
}

#[test]
fn carry_force_cap_uses_the_source_mass_gain() {
    for (mass, expected_distance) in [(2.0, 0.1), (43.5, 0.05), (85.0, 0.0)] {
        let (mut interaction, mut items, mut physics, mut player) = fixture();
        let body = items.get(ItemId(1)).unwrap().body.unwrap();
        let collider = physics.rapier().bodies[body].colliders()[0];
        let world = physics.rapier_mut();
        world.colliders[collider].set_density(0.0);
        world.bodies[body].set_additional_mass(mass, true);
        world.bodies[body].recompute_mass_properties_from_colliders(&world.colliders);
        let initial = *world.bodies[body].position();
        interaction.step(
            input(keys::GRAB, keys::GRAB),
            common::resolved(target(1)),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        interaction.step(
            FrameInput {
                wheel: 100.0,
                ..Default::default()
            },
            common::resolved(None),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        let anchor = physics
            .rapier()
            .bodies
            .iter()
            .find(|(id, rb)| *id != player.body_handle() && rb.is_kinematic())
            .unwrap()
            .1;
        let distance = (anchor.next_position().translation - initial.translation).length();
        assert!(
            (distance - expected_distance).abs() < 0.00001,
            "{mass} kg: {distance}"
        );
        assert_eq!(interaction.held(), Some(ItemId(1)));
    }
}

#[test]
fn combined_carry_buttons_apply_source_pitch_exception() {
    for move_with_rotation in [false, true] {
        let (mut interaction, mut items, mut physics, mut player) = fixture_with(|reference| {
            reference.items[0]
                .pickup
                .as_mut()
                .unwrap()
                .move_with_rotation = move_with_rotation;
        });
        interaction.step(
            input(keys::GRAB, keys::GRAB),
            common::resolved(target(1)),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        interaction.step(
            FrameInput {
                held: keys::ROTATE | keys::TRANSLATE,
                mouse: [20.0, 10.0],
                ..Default::default()
            },
            common::resolved(None),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        let anchor = physics
            .rapier()
            .bodies
            .iter()
            .find(|(id, rb)| *id != player.body_handle() && rb.is_kinematic())
            .unwrap()
            .1;
        let roll = Rotation::from_rotation_z((-20.0_f32 * 4.0 * 10.0 / 60.0).to_radians());
        let expected = if move_with_rotation {
            roll
        } else {
            Rotation::from_rotation_x((10.0_f32 * 4.0 * 10.0 / 60.0).to_radians()) * roll
        };
        for axis in [Vector::X, Vector::Y, Vector::Z] {
            assert!(
                (anchor.next_position().rotation * axis - expected * axis).length() < 0.00001,
                "moveWithRot={move_with_rotation}, axis={axis:?}"
            );
        }
    }
}

#[test]
fn seated_window_drag_preserves_seat_and_driver_state() {
    let (mut interaction, mut items, mut physics, mut player) = fixture_with(|r| {
        common::add_rotation(r, 1, 90, [-30.0, 0.0]);
        let node = r.mechanisms[0].driven_nodes[0];
        let mut end = r.nodes.last().unwrap().clone();
        end.id = my_summer_slop::reference::NodeId(190);
        end.transform.translation[1] -= 0.4;
        r.nodes.push(end);
        r.mechanisms[0].rotation = None;
        r.mechanisms[0].slide = Some(my_summer_slop::reference::SlideReference {
            initial_value: 0.0,
            speed: 0.1,
            sensitivity: 0.02,
            axes: [false, false, false, false, true, true],
            multiplier: [1.0, 1.0, 10.0],
            scroll_without_use: true,
            animation_source: None,
            paths: vec![my_summer_slop::reference::SlidePath {
                node,
                rotate: true,
                points: vec![
                    my_summer_slop::reference::SlidePoint { node, value: 0.0 },
                    my_summer_slop::reference::SlidePoint {
                        node: my_summer_slop::reference::NodeId(190),
                        value: 1.0,
                    },
                ],
            }],
            rotations: vec![],
        });
    });
    let reference = common::reference();
    player.configure_seats(&reference.seats);
    assert!(player.enter(reference.seats[0].id, pose([10.0, 0.25, 0.0]), &mut physics));
    let seat_before = player.seat();
    let driving_before = player.driving();
    interaction.step(
        FrameInput {
            held: keys::USE,
            pressed: keys::USE,
            mouse: [0.0, 125.0],
            ..Default::default()
        },
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!((interaction.mechanisms.value(ItemId(1), false).unwrap() - 0.5).abs() < 1e-6);
    // Cross a seat on release. It must not consume the mechanism's release.
    interaction.step(
        FrameInput {
            released: keys::USE,
            ..Default::default()
        },
        common::resolved(target(17)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(player.seat(), seat_before);
    assert_eq!(player.driving(), driving_before);
    assert_eq!(interaction.dragging_mechanism(), None);
}

#[test]
fn pickup_release_does_not_drop_or_throw_acquired_item() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    for frame in [
        input(keys::GRAB, keys::GRAB),
        input(keys::GRAB, 0),
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
    ] {
        interaction.step(
            frame,
            common::resolved(target(1)),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        assert_eq!(interaction.held(), Some(ItemId(1)));
        assert_eq!(interaction.throw_charge(), 0.0);
    }
}

#[test]
fn held_f_charges_and_release_throws_once() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        0.01,
    );
    interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        0.01,
    );
    let body = items.get(ItemId(1)).unwrap().body.unwrap();
    physics.rapier_mut().bodies[body].set_linvel(Vector::new(2.0, 0.0, 1.0), true);
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        0.2,
    );
    assert_eq!(interaction.held(), Some(ItemId(1)));
    interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        0.01,
    );
    assert_eq!(interaction.held(), None);
    let velocity = physics.rapier().bodies[body].linvel();
    assert_eq!(velocity.x, 2.0);
    let mass = physics.rapier().bodies[body].mass();
    assert!((velocity.z - (1.0 - 0.3 * 1000.0 * 0.02 / mass)).abs() < 0.0001);
    interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        0.01,
    );
    assert_eq!(physics.rapier().bodies[body].linvel(), velocity);
}

#[test]
fn source_push_wakes_and_applies_point_impulse() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let chassis = physics.chassis_handle();
    physics.rapier_mut().bodies[chassis].sleep();
    let aim = my_summer_slop::interaction::ResolvedAim {
        push_target: target(5),
        ..Default::default()
    };
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        aim,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(!physics.rapier().bodies[chassis].is_sleeping());
    assert!((physics.rapier().bodies[chassis].linvel().z + 500.0 / 60.0 * 0.02).abs() < 1e-5);
    assert_eq!(physics.rapier().bodies[chassis].linvel().y, 0.0);
    assert!(physics.rapier().bodies[chassis].angvel().length() > 0.0);
    player.step(FrameInput::default(), &mut physics, 1.0 / 60.0);
    assert!(player.body_pose(&physics).position[2] > 0.0);
}

#[test]
fn throw_charge_rises_falls_stops_and_capture_cancels_release() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    interaction
        .pickup_item(
            ItemId(1),
            [10.0, 2.0, -1.0],
            &player,
            &mut items,
            &mut physics,
        )
        .unwrap();
    interaction.step(
        FrameInput::default(),
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        0.02,
    );
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        0.02,
    );
    assert!((interaction.throw_charge() - 0.03).abs() < 1e-6);
    for _ in 0..33 {
        interaction.step(
            input(keys::GRAB, 0),
            common::resolved(None),
            &mut player,
            &mut items,
            &mut physics,
            0.02,
        );
    }
    let maximum = interaction.throw_charge();
    assert!((1.0..1.031).contains(&maximum));
    interaction.step(
        input(keys::GRAB, 0),
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        0.02,
    );
    assert!((interaction.throw_charge() - (maximum - 0.03)).abs() < 1e-6);
    for _ in 0..40 {
        interaction.step(
            input(keys::GRAB, 0),
            common::resolved(None),
            &mut player,
            &mut items,
            &mut physics,
            0.02,
        );
    }
    let minimum = interaction.throw_charge();
    assert!(minimum <= 0.2);
    interaction.step(
        input(keys::GRAB, 0),
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        0.02,
    );
    assert_eq!(interaction.throw_charge(), minimum);
    interaction.cancel_input();
    let body = items.get(ItemId(1)).unwrap().body.unwrap();
    let velocity = physics.rapier().bodies[body].linvel();
    interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        0.02,
    );
    assert_eq!(interaction.held(), Some(ItemId(1)));
    assert_eq!(interaction.throw_charge(), 0.0);
    assert_eq!(physics.rapier().bodies[body].linvel(), velocity);
}

fn grip_fixture() -> (
    InteractionState,
    ItemStore,
    PhysicsWorld,
    PlayerState,
    AimTarget,
) {
    let (mut interaction, items, mut physics, mut player) = fixture_with(|reference| {
        let node = reference.nodes[0].id;
        reference.pushables[0].grip = Some(my_summer_slop::reference::PushGripReference {
            source: reference.player.source.clone(),
            node,
            hand_node: node,
            colliders: vec![my_summer_slop::reference::ColliderId(1)],
            local_pose: my_summer_slop::reference::Transform {
                translation: [10.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0; 3],
            },
            in_use: false,
        });
    });
    physics.insert_box([10.0, -0.5, 0.0], [20.0, 0.5, 20.0]);
    for _ in 0..60 {
        player.step(FrameInput::default(), &mut physics, 1.0 / 60.0);
        physics.step_physics();
    }
    assert!(player.grounded());
    let chassis = physics.chassis_handle();
    let collider = physics.rapier_mut().insert_collider(
        ColliderBuilder::ball(0.1)
            .translation(Vector::new(10.0, 0.0, 0.0))
            .sensor(true)
            .user_data(1_u128 << 64),
        Some(chassis),
    );
    interaction.bind_surface(
        collider,
        my_summer_slop::reference::ActionBinding {
            collider: my_summer_slop::reference::ColliderId(1),
            use_owner: None,
            pickup_owner: None,
            removal_owner: None,
            push_owner: Some(ItemId(5)),
        },
    );
    let aim = AimTarget {
        collider,
        item: ItemId(5),
        distance: 1.0,
        point: [10.0, 1.0, 0.0],
        slot: None,
    };
    (interaction, items, physics, player, aim)
}

#[test]
fn steering_grip_retains_release_and_look_away_then_next_press_releases() {
    let (mut interaction, mut items, mut physics, mut player, target) = grip_fixture();
    let aim = my_summer_slop::interaction::ResolvedAim {
        push_target: Some(target),
        ..Default::default()
    };
    interaction.step(
        input(keys::GRAB | keys::FORWARD | keys::LEFT, keys::GRAB),
        aim,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(interaction.push_gripping());
    assert_eq!(interaction.push_steering(), Some(1.0));
    assert_eq!(physics.rapier().impulse_joints.len(), 1);
    let joint = physics.rapier().impulse_joints.iter().next().unwrap().1;
    assert_eq!(
        joint.data.locked_axes,
        rapier3d::prelude::JointAxesMask::LIN_X | rapier3d::prelude::JointAxesMask::LIN_Z
    );
    assert!(physics.rapier().bodies[physics.chassis_handle()].linvel().z < 0.0);
    interaction.step(
        FrameInput {
            held: keys::FORWARD,
            released: keys::GRAB,
            ..Default::default()
        },
        my_summer_slop::interaction::ResolvedAim::default(),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(interaction.push_gripping());
    assert!(interaction.prompt(&aim, &items).contains("release grip"));
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        aim,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(!interaction.push_gripping());
    assert!(physics.rapier().impulse_joints.is_empty());
}

#[test]
fn steering_grip_distance_collider_and_capture_cancel() {
    for cancel in 0..3 {
        let (mut interaction, mut items, mut physics, mut player, target) = grip_fixture();
        let aim = my_summer_slop::interaction::ResolvedAim {
            push_target: Some(target),
            ..Default::default()
        };
        interaction.step(
            input(keys::GRAB, keys::GRAB),
            aim,
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        assert!(interaction.push_gripping());
        match cancel {
            0 => {
                let chassis = physics.chassis_handle();
                let mut pose = *physics.rapier().bodies[chassis].position();
                pose.translation.x += 2.0;
                physics.rapier_mut().bodies[chassis].set_position(pose, true);
            }
            1 => physics.rapier_mut().colliders[target.collider].set_enabled(false),
            _ => interaction.cancel_input(),
        }
        interaction.step(
            FrameInput::default(),
            my_summer_slop::interaction::ResolvedAim::default(),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        assert!(!interaction.push_gripping());
        assert!(physics.rapier().impulse_joints.is_empty());
        assert!(physics.rapier().bodies[player.body_handle()].is_kinematic());
    }
}

#[test]
#[ignore = "requires installed source export"]
fn source_push_wakes_car_without_bypassing_handbrake() {
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let mut runtime = my_summer_slop::starting_area::GarageRuntime::new(&r).unwrap();
    let push = &r.pushables[0];
    let binding = r
        .action_bindings
        .iter()
        .find(|b| {
            b.push_owner == Some(push.owner)
                && !push.grip.as_ref().unwrap().colliders.contains(&b.collider)
                && !r
                    .part_slots
                    .iter()
                    .any(|s| s.colliders.contains(&b.collider))
        })
        .unwrap();
    let collider = runtime
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == binding.collider.0)
        .unwrap()
        .0;
    let point = runtime.physics.rapier().colliders[collider]
        .position()
        .translation
        .to_array();
    let target = AimTarget {
        collider,
        item: push.owner,
        distance: 1.0,
        point,
        slot: None,
    };
    let chassis = runtime.physics.chassis_handle();
    runtime.physics.rapier_mut().bodies[chassis].sleep();
    assert!(runtime.handbrake);
    runtime.area.interaction.step(
        input(keys::GRAB, keys::GRAB),
        my_summer_slop::interaction::ResolvedAim {
            push_target: Some(target),
            ..Default::default()
        },
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
    );
    assert!(!runtime.physics.rapier().bodies[chassis].is_sleeping());
    assert!(runtime.handbrake);
}

#[test]
fn steering_grip_moves_car_and_keeps_player_joint_across_physics_steps() {
    let (mut interaction, mut items, mut physics, mut player, target) = grip_fixture();
    let aim = my_summer_slop::interaction::ResolvedAim {
        push_target: Some(target),
        ..Default::default()
    };
    let chassis = physics.chassis_handle();
    // The native chassis rests on a box. Real wheel contacts use the vehicle tire solver.
    for (_, collider) in physics.rapier_mut().colliders.iter_mut() {
        if collider.parent() == Some(chassis) {
            collider.set_friction(0.0);
        }
    }
    let before = physics.rapier().bodies[chassis].position().translation;
    let local_anchor = physics.rapier().bodies[chassis].position().inverse()
        * Vector::from_array(player.body_pose(&physics).position);
    interaction.step(
        input(keys::GRAB | keys::FORWARD, keys::GRAB),
        aim,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(physics.rapier().bodies[player.body_handle()].is_dynamic());
    for _ in 0..60 {
        let input = input(keys::FORWARD, 0);
        interaction.step(
            input,
            my_summer_slop::interaction::ResolvedAim::default(),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        player.step(input, &mut physics, 1.0 / 60.0);
        physics.step_physics();
        assert!(interaction.push_gripping());
        let anchor = *physics.rapier().bodies[chassis].position() * local_anchor;
        let position = Vector::from_array(player.body_pose(&physics).position);
        assert!(Vector::new(anchor.x - position.x, 0.0, anchor.z - position.z).length() < 0.1);
    }
    assert!((physics.rapier().bodies[chassis].position().translation - before).length() > 0.02);
    let position = player.body_pose(&physics).position;
    interaction.cancel_input();
    interaction.step(
        FrameInput::default(),
        my_summer_slop::interaction::ResolvedAim::default(),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(!interaction.push_gripping());
    assert!(physics.rapier().bodies[player.body_handle()].is_kinematic());
    assert_eq!(player.body_pose(&physics).position, position);
    assert!(physics.rapier().impulse_joints.is_empty());
    let before = player.body_pose(&physics).position;
    player.step(input(keys::FORWARD, 0), &mut physics, 1.0 / 60.0);
    physics.step_physics();
    assert_ne!(player.body_pose(&physics).position, before);
}

#[test]
fn steering_grip_requires_ground_and_ordinary_push_rejects_support_root() {
    let (mut interaction, mut items, mut physics, mut player, grip_target) = grip_fixture();
    player.step(input(keys::SPACE, keys::SPACE), &mut physics, 1.0 / 60.0);
    assert!(!player.grounded());
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        my_summer_slop::interaction::ResolvedAim {
            push_target: Some(grip_target),
            ..Default::default()
        },
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(!interaction.push_gripping());
    assert!(physics.rapier().impulse_joints.is_empty());
    let (mut interaction, mut items, mut physics, _) = fixture();
    let r = common::reference();
    let mut player = PlayerState::new(
        &r.player,
        pose([0.0, 1.34 + r.player.standing_height * 0.5, 0.0]),
        &mut physics,
    );
    for _ in 0..30 {
        player.step(FrameInput::default(), &mut physics, 1.0 / 60.0);
    }
    assert!(player.grounded());
    assert_eq!(player.support_body(), Some(physics.chassis_handle()));
    let velocity = physics.rapier().bodies[physics.chassis_handle()].linvel();
    let action = interaction.step(
        input(keys::GRAB, keys::GRAB),
        my_summer_slop::interaction::ResolvedAim {
            push_target: target(5),
            ..Default::default()
        },
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(action, None);
    assert_eq!(
        physics.rapier().bodies[physics.chassis_handle()].linvel(),
        velocity
    );
}

#[test]
fn pickup_surface_preempts_root_push_in_shared_resolver() {
    let (mut interaction, items, mut physics, _) = fixture();
    let handle =
        physics.rapier().bodies[items.get(ItemId(1)).unwrap().body.unwrap()].colliders()[0];
    interaction.bind_surface(
        handle,
        my_summer_slop::reference::ActionBinding {
            collider: my_summer_slop::reference::ColliderId(0),
            use_owner: None,
            pickup_owner: Some(ItemId(1)),
            removal_owner: None,
            push_owner: Some(ItemId(5)),
        },
    );
    physics.refresh_queries();
    let aim = interaction.resolve(&physics, pose([10.0, 2.0, 0.0]), &items);
    assert_eq!(aim.grab_target.map(|target| target.item), Some(ItemId(1)));
    assert_eq!(aim.push_target, None);
}

#[test]
fn grip_forward_force_uses_car_direction_after_look_change() {
    let (mut interaction, mut items, mut physics, mut player, target) = grip_fixture();
    let aim = my_summer_slop::interaction::ResolvedAim {
        push_target: Some(target),
        ..Default::default()
    };
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        aim,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    player.step(
        FrameInput {
            mouse: [90.0, 0.0],
            ..Default::default()
        },
        &mut physics,
        1.0 / 60.0,
    );
    let chassis = physics.chassis_handle();
    let r = common::reference();
    for keys in [keys::FORWARD, keys::FORWARD | keys::LEFT, keys::BACK] {
        let before = physics.rapier().bodies[chassis].linvel();
        let steer = if keys & keys::LEFT != 0 { 1.0 } else { 0.0 };
        let sign = if keys & keys::BACK != 0 { -1.0 } else { 1.0 };
        let direction = Rotation::from_rotation_y(steer * r.vehicle.max_steer_degrees.to_radians())
            * (physics.rapier().bodies[chassis].position().rotation * -Vector::Z)
            * sign;
        let up = physics.rapier().bodies[chassis].position().rotation * Vector::Y;
        let expected = (direction - up * direction.dot(up)).normalize_or_zero()
            * r.player.grip_speed
            * r.player.grip_force
            * r.player.source_force_interval
            / physics.rapier().bodies[chassis].mass();
        interaction.step(
            input(keys, 0),
            my_summer_slop::interaction::ResolvedAim::default(),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        let delta = physics.rapier().bodies[chassis].linvel() - before;
        assert!(
            (delta - expected).length() < 1e-5,
            "actual {delta:?}, expected {expected:?}"
        );
    }
}

#[test]
fn grip_steering_without_forward_or_backward_applies_no_push() {
    let (mut interaction, mut items, mut physics, mut player, target) = grip_fixture();
    let chassis = physics.chassis_handle();
    let before = physics.rapier().bodies[chassis].linvel();
    interaction.step(
        input(keys::GRAB | keys::LEFT, keys::GRAB),
        my_summer_slop::interaction::ResolvedAim {
            push_target: Some(target),
            ..Default::default()
        },
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(interaction.push_gripping());
    assert_eq!(interaction.push_steering(), Some(1.0));
    assert_eq!(physics.rapier().bodies[chassis].linvel(), before);
}

#[test]
fn grip_jump_retains_joint_and_dynamic_body() {
    let (mut interaction, mut items, mut physics, mut player, target) = grip_fixture();
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        my_summer_slop::interaction::ResolvedAim {
            push_target: Some(target),
            ..Default::default()
        },
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    player.step(input(keys::SPACE, keys::SPACE), &mut physics, 1.0 / 60.0);
    assert!(!player.grounded());
    assert!(physics.rapier().bodies[player.body_handle()].linvel().y > 0.0);
    interaction.step(
        FrameInput::default(),
        my_summer_slop::interaction::ResolvedAim::default(),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(interaction.push_gripping());
    assert_eq!(physics.rapier().impulse_joints.len(), 1);
    assert!(physics.rapier().bodies[player.body_handle()].is_dynamic());
    physics.step_physics();
    interaction.step(
        FrameInput::default(),
        my_summer_slop::interaction::ResolvedAim::default(),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(interaction.push_gripping());
}

#[test]
fn carry_lifts_a_grounded_item_out_of_contact() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let body = items.get(ItemId(1)).unwrap().body.unwrap();
    // A small resting penetration matches a settled dynamic body.
    physics.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10.0, 1.71, -1.0)),
        ColliderBuilder::cuboid(2.0, 0.1, 2.0),
    );
    physics.refresh_queries();
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(target(1)),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    for _ in 0..60 {
        interaction.step(
            FrameInput {
                held: keys::TRANSLATE,
                mouse: [0.0, -10.0],
                ..Default::default()
            },
            common::resolved(None),
            &mut player,
            &mut items,
            &mut physics,
            1.0 / 60.0,
        );
        physics.step_physics();
    }
    assert_eq!(interaction.held(), Some(ItemId(1)));
    assert!(
        physics.rapier().bodies[body].translation().y > 2.2,
        "held item must separate from floor contact: {:?}",
        physics.rapier().bodies[body].translation()
    );
}

#[test]
fn throw_hud_state_tracks_release_block_cancel_and_lost_hand() {
    use my_summer_slop::interaction::{InteractionBlock, ResolvedAim};
    for finish in 0..5 {
        let (mut interaction, mut items, mut physics, mut player) = fixture();
        interaction
            .pickup_item(ItemId(1), [10., 2., -1.], &player, &mut items, &mut physics)
            .unwrap();
        interaction.step(
            FrameInput::default(),
            ResolvedAim::default(),
            &mut player,
            &mut items,
            &mut physics,
            0.02,
        );
        assert!(!interaction.throw_active());
        interaction.step(
            input(keys::GRAB, keys::GRAB),
            ResolvedAim::default(),
            &mut player,
            &mut items,
            &mut physics,
            0.02,
        );
        assert!(interaction.throw_active());
        assert!((interaction.throw_charge() - 0.03).abs() < 1e-6);
        match finish {
            0 => {
                interaction.step(
                    FrameInput {
                        released: keys::GRAB,
                        ..Default::default()
                    },
                    ResolvedAim::default(),
                    &mut player,
                    &mut items,
                    &mut physics,
                    0.02,
                );
            }
            1 => {
                interaction.step(
                    input(keys::GRAB, 0),
                    ResolvedAim {
                        installation_block: Some(InteractionBlock::Occupied),
                        ..Default::default()
                    },
                    &mut player,
                    &mut items,
                    &mut physics,
                    0.02,
                );
            }
            2 => interaction.cancel_input(),
            4 => {
                interaction.step(
                    FrameInput {
                        released: keys::GRAB,
                        ..Default::default()
                    },
                    ResolvedAim {
                        installation_block: Some(InteractionBlock::Occupied),
                        ..Default::default()
                    },
                    &mut player,
                    &mut items,
                    &mut physics,
                    0.02,
                );
            }
            _ => {
                interaction.drop_held(&mut items, &mut physics);
            }
        }
        assert!(!interaction.throw_active(), "finish {finish}");
        assert_eq!(interaction.throw_charge(), 0., "finish {finish}");
    }
}
