mod common;

use my_summer_slop::interaction::items::{ItemLocation, ItemStore};
use my_summer_slop::interaction::{AimTarget, InteractionState};
use my_summer_slop::player::PlayerState;
use my_summer_slop::reference::{
    AnchorId, BodyId, ItemId, OpeningId, OpeningRecord, ReferenceManifest,
};
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::vehicle::{PhysicsWorld, VehiclePose};
use rapier3d::math::Vector;
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
fn fixture_reference() -> ReferenceManifest {
    let mut r = common::reference();
    if r.items[0].pickup.is_none() {
        let mut pickup = r.items.iter().find_map(|i| i.pickup.clone()).unwrap();
        pickup.source = r.items[0].source.clone();
        r.items[0].pickup = Some(pickup);
    }
    r
}
fn fixture_world(reference: &ReferenceManifest) -> PhysicsWorld {
    let mut poses = my_summer_slop::starting_area::initial_vehicle_poses(reference).unwrap();
    poses[0] = pose([0., 1.1, 0.]);
    PhysicsWorld::new(reference, &poses).unwrap()
}
fn fixture() -> (InteractionState, ItemStore, PhysicsWorld, PlayerState) {
    fixture_with(|_| {})
}
fn fixture_with(
    rules: impl FnOnce(&mut ReferenceManifest),
) -> (InteractionState, ItemStore, PhysicsWorld, PlayerState) {
    let mut reference = fixture_reference();
    for (id, tank) in [(100, 1), (101, 2)] {
        for prefix in ["outlet", "splash"] {
            let mut particle = reference
                .weather
                .particles
                .iter()
                .find(|p| p.role == format!("{prefix}_{}", reference.openings[0].id.0))
                .unwrap()
                .clone();
            particle.role = format!("{prefix}_{id}");
            reference.weather.particles.push(particle);
        }
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
            particles: reference.openings[0].particles.clone(),
            can_toggle: true,
        });
        reference.tanks[(tank - 1) as usize]
            .openings
            .push(OpeningId(id));
        for prefix in ["outlet", "splash"] {
            let mut particle = reference
                .weather
                .particles
                .iter()
                .find(|p| p.role == format!("{prefix}_{}", reference.openings[0].id.0))
                .unwrap()
                .clone();
            particle.role = format!("{prefix}_{id}");
            reference.weather.particles.push(particle);
        }
    }
    let mut body = reference.bodies[1].clone();
    body.id = BodyId(90);
    body.parent_body = None;
    body.pose.translation = [10.0, 2.0, -1.0];
    reference.bodies.push(body);
    reference.items[0].body = BodyId(90);
    rules(&mut reference);
    let mut physics = fixture_world(&reference);
    let (handle, _) = physics.rapier_mut().insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(10.0, 2.0, -1.0)),
        ColliderBuilder::ball(0.2).user_data(2),
    );
    let mut items = ItemStore::new(&reference).unwrap();
    items.bind_body(BodyId(90), handle);
    items.get_mut(ItemId(1)).unwrap().local_pose = pose([0.0; 3]);
    let mut interaction = InteractionState::new(&reference).unwrap();
    interaction.bind_body(BodyId(90), handle);
    interaction.bind_body(reference.vehicles[0].body, physics.chassis_handle(0));
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
    assert!(common::focus(&interaction, &physics, pose([10.0, 2.0, 0.0])).is_none());
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
        Default::default(),
        1.,
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
        Default::default(),
        1.,
    );
    interaction.step(
        input(keys::GRAB, keys::GRAB),
        common::resolved(None),
        &mut player,
        &mut items,
        &mut physics,
        1.0 / 60.0,
        Default::default(),
        1.,
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
        Default::default(),
        1.,
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
        Default::default(),
        1.,
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
        Default::default(),
        1.,
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
            Default::default(),
            1.,
        );
        physics.step_physics();
    }
    assert!(physics.rapier().bodies[body].translation().z > -1.7);
    assert!(common::focus(&interaction, &physics, pose([10.0, 2.0, 0.0])).is_none());
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
        Default::default(),
        1.,
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
        Default::default(),
        1.,
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
        Default::default(),
        1.,
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
        Default::default(),
        1.,
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
        Default::default(),
        1.,
    );
    assert_eq!(physics.rapier().bodies[body].linvel(), velocity);
}
