mod common;

use desert_drive::simulation::DriveInput;
use desert_drive::vehicle::{PhysicsWorld, VehiclePose};
use desert_drive::world::{
    ChunkData, ChunkKey, WorldOrigin, WorldStream, active_chunks, generate_chunk,
    generate_chunk_in_band, nearest_station, road_sample,
};

// Chunk origin is [band * 128, 0, -key * 128]. Road distance is -absolute Z.
fn seam_vertices(chunk: &ChunkData, end: bool) -> Vec<[f64; 3]> {
    let z = if end { -128.0 } else { 0.0 };
    let mut seam: Vec<_> = chunk
        .vertices
        .iter()
        .filter(|v| (v[2] - z).abs() < 0.001)
        .map(|v| {
            [
                v[0] as f64 + chunk.origin()[0],
                v[1] as f64,
                v[2] as f64 - chunk.key.0 as f64 * 128.0,
            ]
        })
        .collect();
    seam.sort_by(|a, b| {
        a[0].total_cmp(&b[0])
            .then(a[1].total_cmp(&b[1]))
            .then(a[2].total_cmp(&b[2]))
    });
    seam
}

#[test]
fn overlapping_lateral_bands_share_ground_triangles() {
    fn overlap(chunk: &ChunkData) -> Vec<[[f64; 3]; 3]> {
        let origin = chunk.origin();
        let columns = chunk.lateral_samples.len();
        chunk
            .indices
            .iter()
            .filter(|triangle| {
                triangle.iter().all(|i| {
                    (-32.0..=160.0).contains(&chunk.lateral_samples[*i as usize % columns])
                })
            })
            .map(|triangle| {
                triangle.map(|i| {
                    std::array::from_fn(|axis| {
                        chunk.vertices[i as usize][axis] as f64 + origin[axis]
                    })
                })
            })
            .collect()
    }
    for key in [-1, 0, 15, 100_000] {
        let left = overlap(&generate_chunk_in_band(42, ChunkKey(key), 0));
        let right = overlap(&generate_chunk_in_band(42, ChunkKey(key), 1));
        assert!(!left.is_empty());
        assert_eq!(left.len(), right.len());
        for (p, q) in left
            .iter()
            .flatten()
            .flatten()
            .zip(right.iter().flatten().flatten())
        {
            assert!((p - q).abs() < 0.001);
        }
    }
}

#[test]
fn adjacent_chunks_share_the_seam() {
    for key in [-13, -1, 0, 15, 32, 100_000] {
        let a = generate_chunk(42, ChunkKey(key));
        let b = generate_chunk(42, ChunkKey(key + 1));
        assert_eq!(a, generate_chunk(42, ChunkKey(key)));
        let left = seam_vertices(&a, true);
        let right = seam_vertices(&b, false);
        assert!(!left.is_empty());
        assert_eq!(left.len(), right.len());
        for (p, q) in left.iter().zip(&right) {
            for axis in 0..3 {
                assert!((p[axis] - q[axis]).abs() < 0.001);
            }
        }
    }
    assert_ne!(
        generate_chunk(42, ChunkKey(0)),
        generate_chunk(43, ChunkKey(0))
    );
}

#[test]
fn stations_and_active_window_follow_absolute_distance() {
    assert_eq!(
        active_chunks(128.0),
        std::array::from_fn(|i| ChunkKey(i as i64 - 2))
    );
    assert_eq!(active_chunks(-0.01)[3], ChunkKey(-1));
    for id in [-100, -1, 0, 1, 3000] {
        let station = nearest_station(42, id as f64 * 640.0 + 100.0);
        assert_eq!(station.id, id);
        assert_eq!(station.absolute_point[2], -(id as f64 * 640.0));
        let chunk = generate_chunk(42, ChunkKey(id * 5));
        assert_eq!(chunk.stations, vec![station]);
        let road = road_sample(42, id as f64 * 640.0);
        assert!((station.absolute_point[0] - road.center[0] - 10.0).abs() < 0.001);
    }
}

#[test]
fn rebase_preserves_wheels_velocities_and_absolute_position() {
    let mut physics = PhysicsWorld::new(
        &common::reference(),
        VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    )
    .expect("fixture physics");
    physics.insert_mesh(
        &[
            [-500.0, 0.0, -500.0],
            [500.0, 0.0, -500.0],
            [500.0, 0.0, 500.0],
            [-500.0, 0.0, 500.0],
        ],
        &[[0, 2, 1], [0, 3, 2]],
    );
    for _ in 0..180 {
        physics.apply_vehicle_forces(
            DriveInput {
                accelerate: true,
                steer: 0.05,
                ..Default::default()
            },
            2600.0,
        );
        physics.step_physics();
    }
    let before = physics.snapshot();
    let mut origin = WorldOrigin::default();
    let absolute = origin.absolute(before.pose.position);
    let shift = [0.0, 0.0, -4096.0];
    physics.rebase(shift);
    origin.shift(shift);
    let after = physics.snapshot();
    assert!((after.pose.position[2] - before.pose.position[2] - 4096.0).abs() < 0.001);
    assert_eq!(before.linear_velocity, after.linear_velocity);
    assert_eq!(before.angular_velocity, after.angular_velocity);
    assert_eq!(before.pose.rotation, after.pose.rotation);
    for (p, q) in origin
        .absolute(after.pose.position)
        .into_iter()
        .zip(absolute)
    {
        assert!((p - q).abs() < 0.001);
    }
    for i in 0..4 {
        assert_eq!(before.wheels[i].rotation, after.wheels[i].rotation);
        for axis in 0..3 {
            let relative_before = before.wheels[i].position[axis] - before.pose.position[axis];
            let relative_after = after.wheels[i].position[axis] - after.pose.position[axis];
            assert!((relative_before - relative_after).abs() < 0.001);
        }
    }
    physics.apply_vehicle_forces(DriveInput::default(), 2600.0);
    physics.step_physics();
    assert!(physics.snapshot().wheels_contact.into_iter().all(|c| c));
    assert!(physics.snapshot().speed_mps > 1.0);
}

#[test]
fn streaming_bounds_colliders_and_loads_teleport_destination() {
    let mut physics = PhysicsWorld::new(
        &common::reference(),
        VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    )
    .expect("fixture physics");
    let mut stream = WorldStream::new(42);
    for distance in (0..40)
        .map(|i| i as f64 * 256.0)
        .chain([1_000_000.0, -1024.0, 0.0])
    {
        stream.sync_at(road_sample(42, distance).center, &mut physics);
        assert_eq!(stream.chunks().count(), 11);
        let expected_colliders = 2 + stream
            .chunks()
            .map(|c| {
                1 + c
                    .data
                    .props
                    .iter()
                    .filter(|p| p.kind != desert_drive::world::PropKind::Cactus)
                    .count()
                    + c.data.stations.len() * 2
            })
            .sum::<usize>();
        let terrain_count = physics
            .rapier()
            .colliders
            .iter()
            .filter(|(_, collider)| collider.shape().as_trimesh().is_some())
            .count();
        assert!((11..=14).contains(&terrain_count));
        assert_eq!(
            physics.collider_count(),
            expected_colliders + terrain_count - stream.chunks().count()
        );
        assert!(physics.collider_count() <= 66);
        for key in active_chunks(distance) {
            assert!(stream.chunks().any(|c| c.data.key == key));
        }
    }
    let pose = stream.reset_pose(3200.0, true, &mut physics);
    assert!(
        stream
            .chunks()
            .any(|c| c.data.stations.iter().any(|s| s.id == 5))
    );
    physics.reset(pose);
    for _ in 0..120 {
        physics.apply_vehicle_forces(DriveInput::default(), 0.0);
        physics.step_physics();
    }
    assert!(physics.snapshot().wheels_contact.into_iter().all(|c| c));
}

#[test]
fn driving_crosses_chunk_seams_and_rebase_without_trip_jump() {
    let mut physics = PhysicsWorld::new(
        &common::reference(),
        VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    )
    .expect("fixture physics");
    let mut stream = WorldStream::new(42);
    let pose = stream.reset_pose(2000.0, false, &mut physics);
    physics.reset(pose);
    for _ in 0..120 {
        physics.apply_vehicle_forces(DriveInput::default(), 2600.0);
        physics.step_physics();
    }
    let mut previous = stream.origin.absolute(physics.snapshot().pose.position);
    let mut trip = 0.0;
    let mut rebases = 0;
    for _ in 0..600 {
        let accelerate = physics.snapshot().speed_mps < 16.0;
        physics.apply_vehicle_forces(
            DriveInput {
                accelerate,
                ..Default::default()
            },
            2600.0,
        );
        physics.step_physics();
        if stream
            .rebase(physics.snapshot().pose.position, &mut physics)
            .is_some()
        {
            rebases += 1;
        }
        let snapshot = physics.snapshot();
        let absolute = stream.origin.absolute(snapshot.pose.position);
        let travel =
            ((absolute[0] - previous[0]).powi(2) + (absolute[2] - previous[2]).powi(2)).sqrt();
        assert!(travel < 1.0, "Trip jump: {travel}");
        trip += travel;
        previous = absolute;
        stream.sync_at(absolute, &mut physics);
        assert_eq!(stream.chunks().count(), 11);
        assert!(snapshot.pose.position.iter().all(|v| v.is_finite()));
    }
    assert!(trip > 100.0, "Trip: {trip}");
    assert!(rebases > 0);
    assert!(physics.snapshot().wheels_contact.into_iter().all(|c| c));
}

#[test]
fn lateral_streaming_supports_travel_beyond_the_old_edge() {
    let mut physics = PhysicsWorld::new(
        &common::reference(),
        VehiclePose {
            position: [55.0, 8.0, -64.0],
            rotation: [
                0.0,
                -std::f32::consts::FRAC_1_SQRT_2,
                0.0,
                std::f32::consts::FRAC_1_SQRT_2,
            ],
        },
    )
    .expect("fixture physics");
    let mut stream = WorldStream::new(42);
    stream.sync_at([55.0, 8.0, -64.0], &mut physics);
    for _ in 0..180 {
        physics.apply_vehicle_forces(DriveInput::default(), 2600.0);
        physics.step_physics();
    }
    let mut band_changes = 0;
    let mut old_band = stream.lateral_band();
    for _ in 0..3600 {
        let absolute = stream.origin.absolute(physics.snapshot().pose.position);
        stream.sync_at(absolute, &mut physics);
        if stream.lateral_band() != old_band {
            band_changes += 1;
            old_band = stream.lateral_band();
        }
        let accelerate = physics.snapshot().speed_mps < 8.0;
        physics.apply_vehicle_forces(
            DriveInput {
                accelerate,
                ..Default::default()
            },
            2600.0,
        );
        physics.step_physics();
        let snapshot = physics.snapshot();
        assert!(
            snapshot.wheels_contact.iter().any(|c| *c),
            "Lost contact at {:?}, band {}",
            snapshot.pose.position,
            old_band
        );
        assert_eq!(stream.chunks().count(), 11);
        let expected = 2 + stream
            .chunks()
            .map(|c| {
                1 + c
                    .data
                    .props
                    .iter()
                    .filter(|p| p.kind != desert_drive::world::PropKind::Cactus)
                    .count()
                    + c.data.stations.len() * 2
            })
            .sum::<usize>();
        assert_eq!(physics.collider_count(), expected);
    }
    assert!(stream.origin.absolute(physics.snapshot().pose.position)[0] > 362.0);
    assert!(band_changes >= 3);
}

#[test]
fn walking_focus_rebases_a_parked_vehicle_without_travel() {
    let mut physics = PhysicsWorld::new(
        &common::reference(),
        VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    )
    .expect("fixture physics");
    let mut stream = WorldStream::new(42);
    let before = physics.snapshot();
    let shift = stream.rebase([2300.0, 0.0, 0.0], &mut physics);
    assert_eq!(shift, Some([2304.0, 0.0, 0.0]));
    let after = physics.snapshot();
    let absolute = stream.origin.absolute(after.pose.position);
    assert_eq!(absolute, [0.0, 1.1_f32 as f64, 0.0]);
    assert_eq!(before.linear_velocity, after.linear_velocity);
    assert_eq!(before.angular_velocity, after.angular_velocity);
}

#[test]
fn garage_terrain_keeps_interiors_clear_and_connects_driveway() {
    use desert_drive::reference::{Bounds, ColliderGeometry};
    let mut reference = common::reference();
    reference.area.home_bounds = Bounds {
        min: [30.0, -2.0, -75.0],
        max: [50.0, 7.0, -45.0],
    };
    reference.area.driveway_bounds = Bounds {
        min: [30.0, 1.0, -70.0],
        max: [40.0, 1.0, -60.0],
    };
    reference.area.driveway_height = 1.0;
    let mut interior = reference.nodes[0].clone();
    interior.id = desert_drive::reference::NodeId(999);
    interior.name = "interiorCellar".into();
    reference.nodes.push(interior);
    let mut collider = reference.colliders[0].clone();
    collider.node = desert_drive::reference::NodeId(999);
    collider.body = reference.bodies[0].id;
    collider.geometry = ColliderGeometry::Convex {
        points: vec![
            [35.0, -2.0, -67.0],
            [38.0, -2.0, -64.0],
            [38.0, 0.0, -67.0],
            [35.0, 0.0, -64.0],
        ],
    };
    reference.bodies[0].pose.translation = [0.0; 3];
    reference.bodies[0].pose.rotation = [0.0, 0.0, 0.0, 1.0];
    reference.colliders.push(collider);
    let mut physics = PhysicsWorld::new(
        &common::reference(),
        VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    )
    .unwrap();
    let mut world = WorldStream::new(42);
    world.configure_starting_area(&reference);
    world.sync_at([40.0, 0.0, -64.0], &mut physics);
    let data = &world
        .chunks()
        .find(|chunk| chunk.data.key == ChunkKey(0))
        .unwrap()
        .data;
    assert!(data.vertices.iter().any(|p| p[0] > 30.0
        && p[0] < 40.0
        && p[2] < -60.0
        && p[2] > -70.0
        && (p[1] - 1.0).abs() < 0.001));
    for triangle in &data.indices {
        let x = triangle
            .iter()
            .map(|i| data.vertices[*i as usize][0])
            .sum::<f32>()
            / 3.0;
        let z = triangle
            .iter()
            .map(|i| data.vertices[*i as usize][2])
            .sum::<f32>()
            / 3.0;
        assert!(!(35.0..=38.0).contains(&x) || !(-67.0..=-64.0).contains(&z));
    }
}
