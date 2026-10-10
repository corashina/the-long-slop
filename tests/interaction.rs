mod common;

use my_summer_slop::interaction::items::{ItemLocation, ItemStore};
use my_summer_slop::interaction::{
    AimTarget, InteractionAction, InteractionState, SLOT_SENSOR_FLAG,
};
use my_summer_slop::player::PlayerState;
use my_summer_slop::reference::{
    AnchorId, BodyId, HingeRecord, ItemId, OpeningId, OpeningRecord, ReferenceManifest, TankRole,
    WheelSlotId,
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
            presentation: my_summer_slop::reference::FluidPresentation {
                has_name: false,
                name: String::new(),
                no_text: false,
            },
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
    let player = PlayerState::new(&reference.player, pose([10.0, 1.0, 0.0]), &mut physics);
    (interaction, items, physics, player)
}
fn door_fixture() -> (InteractionState, ItemStore, PhysicsWorld, PlayerState) {
    fixture_with(|reference| {
        reference.hinges.push(HingeRecord {
            item: ItemId(1),
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
            motor_degrees_per_second: None,
        });
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
    assert!(
        interaction
            .aim(&physics, pose([10.0, 2.0, 0.0]), 3.0)
            .is_none()
    );
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
    let target = interaction.aim(&physics, pose([10.0, 2.0, 0.0]), 3.0);
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
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.dragging_hinge(), Some(ItemId(1)));
    let action = interaction.step(
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
    assert_eq!(action, None);
    assert!(player.seat().is_none());
    assert_eq!(interaction.dragging_hinge(), None);
}

#[test]
fn bare_seat_release_can_enter_after_an_empty_target_press() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    player.configure_seats(&common::reference().seats);
    interaction.step(
        input(keys::USE, keys::USE),
        None,
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
        target(17),
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
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(action, Some(InteractionAction::Hinge(ItemId(1))));
    assert_eq!(player.seat(), Some(reference.seats[0].id));
    assert_eq!(interaction.dragging_hinge(), Some(ItemId(1)));
    assert_eq!(interaction.hinge_angle(ItemId(1)), Some(-3.0));
    interaction.step(
        FrameInput {
            held: keys::USE,
            mouse: [3.0, 0.0],
            ..Default::default()
        },
        None,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.hinge_angle(ItemId(1)), Some(-6.0));
    assert_eq!(player.seat(), Some(reference.seats[0].id));
    interaction.step(
        FrameInput {
            released: keys::USE,
            ..Default::default()
        },
        None,
        &mut player,
        &mut items,
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
        target(1),
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
    let wheel_id = common::reference().wheel_slots[0].rim;
    let action = interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(wheel_id.0),
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
        target(wheel_id.0),
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
        None,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let slot = AimTarget {
        slot: Some(WheelSlotId(0)),
        ..target(13).unwrap()
    };
    interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            held: keys::GRAB,
            ..Default::default()
        },
        Some(slot),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let action = interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        Some(slot),
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
    interaction.step(
        FrameInput {
            held: keys::USE,
            mouse: [1.0, 0.0],
            ..Default::default()
        },
        target(4),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.dragging_hinge(), Some(ItemId(4)));
    let first = interaction.hinge_angle(ItemId(4)).unwrap();
    interaction.step(
        FrameInput {
            held: keys::USE,
            mouse: [1.0, 0.0],
            ..Default::default()
        },
        None,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert!(interaction.hinge_angle(ItemId(4)).unwrap() < first);
    interaction.step(
        FrameInput {
            released: keys::USE,
            ..Default::default()
        },
        None,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.dragging_hinge(), None);
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
        slot: Some(reference.wheel_slots[0].id),
        ..target(rim.0).unwrap()
    };
    interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        Some(slot),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.held(), None);
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        target(tire.0),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(interaction.held(), None);
}

#[test]
fn runtime_pump_stays_open_and_fills_the_held_container() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let pump = ItemId(90);
    items.insert_supply(pump, pose([10.0, 2.0, -1.0]));
    interaction.register_pump(pump, 0.5).unwrap();
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
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    interaction.step(
        input(keys::USE, keys::USE),
        target(pump.0),
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
        target(pump.0),
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
        target(2),
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
        input(keys::USE, keys::USE),
        target(1),
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
        FrameInput {
            held: keys::USE,
            mouse: [0.0, 5.0],
            ..Default::default()
        },
        target(1),
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
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        0.01,
    );
    let valve = interaction.opening_valve(OpeningId(100));
    assert!((valve - 0.3675).abs() < 1e-6);
    interaction.step(
        input(keys::FILL, 0),
        target(1),
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
            target(1),
            &mut player,
            &mut items,
            &mut physics,
            0.01,
        );
        assert_eq!(interaction.opening_valve(OpeningId(100)), expected);
    }
}

#[test]
fn one_f_gesture_does_not_remount_pickup() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let count = items.iter().count();
    let action = interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            released: keys::GRAB,
            ..Default::default()
        },
        target(1),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(action, Some(InteractionAction::Pickup(ItemId(1))));
    assert_eq!(interaction.held(), Some(ItemId(1)));
    assert_eq!(items.iter().count(), count);
}
#[test]
fn source_mount_occurs_on_release() {
    let (mut interaction, mut items, mut physics, mut player) = fixture();
    let r = common::reference();
    let rim = r.wheel_slots[0].rim;
    let body = items.get(ItemId(1)).unwrap().body;
    let part = items.get_mut(rim).unwrap();
    part.location = ItemLocation::World;
    part.body = body;
    part.local_pose = pose([0.; 3]);
    interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            ..Default::default()
        },
        target(rim.0),
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
        None,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    let at = Some(AimTarget {
        slot: Some(r.wheel_slots[0].id),
        ..target(rim.0).unwrap()
    });
    let pressed = interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            held: keys::GRAB,
            ..Default::default()
        },
        at,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(pressed, None);
    let released = interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        at,
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
    );
    assert_eq!(
        released,
        Some(InteractionAction::WheelInstall {
            item: rim,
            target: r.wheel_slots[0].id
        })
    );
}
