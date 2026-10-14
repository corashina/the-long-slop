mod common;

// The fixture axle has two driven wheels with a 0.3 m radius.

use my_summer_slop::simulation::DriveInput;
use my_summer_slop::starting_area::GarageRuntime;
use my_summer_slop::vehicle::{DriveTorques, PhysicsWorld, VehiclePose};
use my_summer_slop::world::{
    ChunkData, ChunkKey, Station, WorldOrigin, WorldStream, active_chunks, generate_chunk,
    generate_chunk_in_band, nearest_station, road_sample,
};

/// These streaming tests drive one car.
fn lada_only() -> my_summer_slop::reference::ReferenceManifest {
    let mut reference = common::reference();
    reference.vehicles.truncate(1);
    reference
}

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
    let reference = common::reference().stations;
    for id in [-100, -1, 0, 1, 3000] {
        let station = nearest_station(42, Station::distance(id) + 100.0, &reference);
        assert_eq!(station.id, id);
        let road = road_sample(42, Station::distance(id));
        let lateral = (station.absolute_point[0] - road.center[0]) as f32;
        let side = reference.types[station.kind].sides.iter().any(|side| {
            (side.lateral_meters[0] - 0.01..=side.lateral_meters[1] + 0.01).contains(&lateral)
        });
        assert!(side, "station {id} lateral {lateral}");
        let pump = (station.pump_point[0] - road.center[0]) as f32;
        assert!(pump.signum() == lateral.signum() && pump.abs() < lateral.abs());
    }
}

#[test]
fn rebase_preserves_wheels_velocities_and_absolute_position() {
    let mut physics = PhysicsWorld::new(
        &lada_only(),
        &[VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        }],
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
            0,
            DriveInput {
                throttle: 1.0,
                steer: 0.05,
                ..Default::default()
            },
            DriveTorques {
                motor_nm: 2600.0 * 2.0 * 0.3,
                ..Default::default()
            },
        );
        physics.step_physics();
    }
    let before = physics.snapshot(0);
    let mut origin = WorldOrigin::default();
    let absolute = origin.absolute(before.pose.position);
    let shift = [0.0, 0.0, -4096.0];
    physics.rebase(shift);
    origin.shift(shift);
    let after = physics.snapshot(0);
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
    physics.apply_vehicle_forces(0, DriveInput::default(), Default::default());
    physics.step_physics();
    assert!(physics.snapshot(0).wheels_contact.into_iter().all(|c| c));
    assert!(physics.snapshot(0).speed_mps > 1.0);
}

#[test]
fn streaming_bounds_colliders_and_loads_teleport_destination() {
    let mut physics = PhysicsWorld::new(
        &lada_only(),
        &[VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        }],
    )
    .expect("fixture physics");
    let mut stream = WorldStream::new(42, &common::reference().stations).unwrap();
    let car_colliders = physics.collider_count();
    for distance in (0..40)
        .map(|i| i as f64 * 256.0)
        .chain([1_000_000.0, -1024.0, 0.0])
    {
        stream.sync_at(road_sample(42, distance).center, &mut physics);
        assert_eq!(stream.chunks().count(), 11);
        let expected_colliders = car_colliders
            + stream
                .chunks()
                .map(|c| {
                    1 + c
                        .data
                        .props
                        .iter()
                        .filter(|p| p.kind != my_summer_slop::world::PropKind::Cactus)
                        .count()
                        + c.data
                            .stations
                            .iter()
                            .map(|s| 1 + stream.stations().types[s.kind].colliders.len())
                            .sum::<usize>()
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
    physics.reset(0, pose);
    for _ in 0..120 {
        physics.apply_vehicle_forces(0, DriveInput::default(), Default::default());
        physics.step_physics();
    }
    assert!(physics.snapshot(0).wheels_contact.into_iter().all(|c| c));
}

#[test]
fn driving_crosses_chunk_seams_and_rebase_without_trip_jump() {
    let mut physics = PhysicsWorld::new(
        &lada_only(),
        &[VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        }],
    )
    .expect("fixture physics");
    let mut stream = WorldStream::new(42, &common::reference().stations).unwrap();
    let pose = stream.reset_pose(2000.0, false, &mut physics);
    physics.reset(0, pose);
    for _ in 0..120 {
        physics.apply_vehicle_forces(0, DriveInput::default(), Default::default());
        physics.step_physics();
    }
    let mut previous = stream.origin.absolute(physics.snapshot(0).pose.position);
    let mut trip = 0.0;
    let mut rebases = 0;
    for _ in 0..600 {
        let accelerate = physics.snapshot(0).speed_mps < 16.0;
        physics.apply_vehicle_forces(
            0,
            DriveInput {
                throttle: accelerate as u8 as f32,
                ..Default::default()
            },
            DriveTorques {
                motor_nm: if accelerate { 2600.0 * 2.0 * 0.3 } else { 0.0 },
                ..Default::default()
            },
        );
        physics.step_physics();
        if stream
            .rebase(physics.snapshot(0).pose.position, &mut physics)
            .is_some()
        {
            rebases += 1;
        }
        let snapshot = physics.snapshot(0);
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
    assert!(physics.snapshot(0).wheels_contact.into_iter().all(|c| c));
}

#[test]
fn lateral_streaming_supports_travel_beyond_the_old_edge() {
    let mut physics = PhysicsWorld::new(
        &lada_only(),
        &[VehiclePose {
            position: [55.0, 8.0, -64.0],
            rotation: [
                0.0,
                -std::f32::consts::FRAC_1_SQRT_2,
                0.0,
                std::f32::consts::FRAC_1_SQRT_2,
            ],
        }],
    )
    .expect("fixture physics");
    let mut stream = WorldStream::new(42, &common::reference().stations).unwrap();
    let car_colliders = physics.collider_count();
    stream.sync_at([55.0, 8.0, -64.0], &mut physics);
    for _ in 0..180 {
        physics.apply_vehicle_forces(0, DriveInput::default(), Default::default());
        physics.step_physics();
    }
    let mut band_changes = 0;
    let mut old_band = stream.lateral_band();
    for _ in 0..3600 {
        let absolute = stream.origin.absolute(physics.snapshot(0).pose.position);
        stream.sync_at(absolute, &mut physics);
        if stream.lateral_band() != old_band {
            band_changes += 1;
            old_band = stream.lateral_band();
        }
        let accelerate = physics.snapshot(0).speed_mps < 8.0;
        physics.apply_vehicle_forces(
            0,
            DriveInput {
                throttle: accelerate as u8 as f32,
                ..Default::default()
            },
            DriveTorques {
                motor_nm: if accelerate { 2600.0 * 2.0 * 0.3 } else { 0.0 },
                ..Default::default()
            },
        );
        physics.step_physics();
        let snapshot = physics.snapshot(0);
        assert!(
            snapshot.wheels_contact.iter().any(|c| *c),
            "Lost contact at {:?}, band {}",
            snapshot.pose.position,
            old_band
        );
        assert_eq!(stream.chunks().count(), 11);
        let expected = car_colliders
            + stream
                .chunks()
                .map(|c| {
                    1 + c
                        .data
                        .props
                        .iter()
                        .filter(|p| p.kind != my_summer_slop::world::PropKind::Cactus)
                        .count()
                        + c.data
                            .stations
                            .iter()
                            .map(|s| 1 + stream.stations().types[s.kind].colliders.len())
                            .sum::<usize>()
                })
                .sum::<usize>();
        assert_eq!(physics.collider_count(), expected);
    }
    assert!(stream.origin.absolute(physics.snapshot(0).pose.position)[0] > 362.0);
    assert!(band_changes >= 3);
}

#[test]
fn walking_focus_rebases_a_parked_vehicle_without_travel() {
    let mut physics = PhysicsWorld::new(
        &lada_only(),
        &[VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        }],
    )
    .expect("fixture physics");
    let mut stream = WorldStream::new(42, &common::reference().stations).unwrap();
    let before = physics.snapshot(0);
    let shift = stream.rebase([2300.0, 0.0, 0.0], &mut physics);
    assert_eq!(shift, Some([2304.0, 0.0, 0.0]));
    let after = physics.snapshot(0);
    let absolute = stream.origin.absolute(after.pose.position);
    assert_eq!(absolute, [0.0, 1.1_f32 as f64, 0.0]);
    assert_eq!(before.linear_velocity, after.linear_velocity);
    assert_eq!(before.angular_velocity, after.angular_velocity);
}

#[test]
fn garage_terrain_keeps_interiors_clear_and_connects_driveway() {
    use my_summer_slop::reference::{Bounds, ColliderGeometry};
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
    interior.id = my_summer_slop::reference::NodeId(999);
    interior.name = "interiorCellar".into();
    reference.nodes.push(interior);
    let mut collider = reference.colliders[0].clone();
    collider.node = my_summer_slop::reference::NodeId(999);
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
        &lada_only(),
        &[VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        }],
    )
    .unwrap();
    let mut world = WorldStream::new(42, &common::reference().stations).unwrap();
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
    for x in [34.9, 38.1] {
        let ray = rapier3d::prelude::Ray::new(
            rapier3d::math::Vector::new(x, 2.0, -65.0),
            -rapier3d::math::Vector::Y,
        );
        assert!(
            physics
                .queries(rapier3d::prelude::QueryFilter::default())
                .cast_ray(&ray, 3.0, true)
                .is_some(),
            "terrain must reach the exact room boundary at x{x}",
        );
    }
    let ray = rapier3d::prelude::Ray::new(
        rapier3d::math::Vector::new(35.1, 2.0, -65.0),
        -rapier3d::math::Vector::Y,
    );
    assert!(
        physics
            .queries(rapier3d::prelude::QueryFilter::default())
            .cast_ray(&ray, 3.0, true)
            .is_none(),
        "room must stay clear"
    );
}

#[test]
fn road_lamp_intervals_have_one_owner_at_seams() {
    use my_summer_slop::world::road_lamp_range;
    assert_eq!(road_lamp_range(0., 128., 64.).collect::<Vec<_>>(), [0, 1]);
    assert_eq!(road_lamp_range(128., 256., 64.).collect::<Vec<_>>(), [2, 3]);
    assert!(road_lamp_range(-128., 0., 64.).is_empty());
    assert_eq!(road_lamp_range(-64., 64., 64.).collect::<Vec<_>>(), [0]);
}

#[test]
fn road_lamp_placement_is_absolute_after_rebase() {
    use my_summer_slop::world::{RoadLampId, road_lamp_placement};
    let reference = common::reference().lighting.road_lamp;
    for id in [RoadLampId(13), RoadLampId(1_000_001)] {
        let placement = road_lamp_placement(42, id, &reference);
        let a = WorldOrigin {
            offset: placement.absolute_point,
        };
        let b = WorldOrigin {
            offset: [
                placement.absolute_point[0] + 4.,
                placement.absolute_point[1],
                placement.absolute_point[2] - 8.,
            ],
        };
        let local_a = a.local(placement.absolute_point);
        let local_b = b.local(placement.absolute_point);
        assert_ne!(local_a, local_b);
        for origin in [a, b] {
            for (actual, expected) in origin
                .absolute(origin.local(placement.absolute_point))
                .into_iter()
                .zip(placement.absolute_point)
            {
                assert!((actual - expected).abs() < 1e-6);
            }
        }
        assert_eq!(road_lamp_placement(42, id, &reference), placement);
        assert_eq!(placement.id, id);
        let road = road_sample(42, id.0 as f64 * reference.spacing_meters as f64);
        let offset_length = placement
            .absolute_point
            .into_iter()
            .zip(road.center)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        let authored_length = reference
            .road_pose
            .translation
            .into_iter()
            .map(|v| (v as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            (offset_length - authored_length).abs() < 1e-6,
            "Apply the authored offset once"
        );
        assert!(placement.rotation.iter().all(|v| v.is_finite()));
        if id.0 > 1_000_000 {
            assert!(placement.absolute_point[2].abs() > 100_000_000.);
        }
    }
}

#[test]
fn road_lamp_window_bound_comes_from_actual_chunks() {
    use my_summer_slop::world::road_lamp_range;
    let reference = common::reference();
    let mut physics = PhysicsWorld::new(
        &lada_only(),
        &[VehiclePose {
            position: [0., 1.1, 0.],
            rotation: [0., 0., 0., 1.],
        }],
    )
    .unwrap();
    let mut stream = WorldStream::new(42, &common::reference().stations).unwrap();
    let mut maximum = 0;
    for distance in (-8..128).map(|i| i as f64 * 128.).chain([100_000_100.]) {
        stream.sync_at(road_sample(42, distance).center, &mut physics);
        let mut ids = Vec::new();
        for chunk in stream.chunks() {
            let start = chunk.data.key.0 as f64 * 128.;
            ids.extend(road_lamp_range(
                start,
                start + 128.,
                reference.lighting.road_lamp.spacing_meters as f64,
            ));
        }
        let count = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(count, ids.len());
        assert!(count <= 15);
        maximum = maximum.max(count);
    }
    assert_eq!(maximum, 15);
}

#[test]
fn station_food_stays_consumed_after_stream_rebase_and_recovery() {
    let reference = common::reference();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    let starter = reference.survival.starter_food;
    let station_food = runtime
        .area
        .items
        .iter()
        .find(|item| item.food.is_some() && item.id != starter)
        .unwrap()
        .id;
    let body = runtime.area.items.get(station_food).unwrap().body.unwrap();
    let absolute = runtime
        .stream
        .origin
        .absolute(runtime.area.items.get(station_food).unwrap().pose.position);
    let station = runtime
        .stream
        .chunks()
        .flat_map(|chunk| &chunk.data.stations)
        .find(|station| {
            (station.pump_point[0] - absolute[0]).abs() < 0.001
                && (station.pump_point[2] + 1. - absolute[2]).abs() < 0.001
        })
        .unwrap();
    let mut food_min = [f32::INFINITY; 3];
    let mut food_max = [f32::NEG_INFINITY; 3];
    for &handle in runtime.physics.rapier().bodies[body].colliders() {
        let aabb = runtime.physics.rapier().colliders[handle].compute_aabb();
        for axis in 0..3 {
            food_min[axis] = food_min[axis].min(aabb.mins[axis]);
            food_max[axis] = food_max[axis].max(aabb.maxs[axis]);
        }
    }
    let pump = runtime.stream.origin.local(station.pump_point);
    assert!(
        food_min[2] > pump[2] + 0.35,
        "food={food_min:?} pump={pump:?}"
    );
    assert!(
        food_max[2] < pump[2] + 2.,
        "food={food_max:?} pump={pump:?}"
    );
    assert!(
        food_min[1] > pump[1] - 0.1,
        "food={food_min:?} pump={pump:?}"
    );
    for _ in 0..6 {
        assert!(runtime.consume(&reference, station_food).unwrap());
    }
    assert!(runtime.physics.rapier().bodies.get(body).is_none());
    runtime
        .stream
        .sync_at([0., 0., -10_000.], &mut runtime.physics);
    my_summer_slop::world::sync_station_food(
        &mut runtime.stream,
        &reference,
        &mut runtime.area,
        &mut runtime.physics,
    )
    .unwrap();
    runtime.stream.sync_at(absolute, &mut runtime.physics);
    my_summer_slop::world::sync_station_food(
        &mut runtime.stream,
        &reference,
        &mut runtime.area,
        &mut runtime.physics,
    )
    .unwrap();
    if runtime
        .stream
        .rebase([1024., 0., 0.], &mut runtime.physics)
        .is_some()
    {
        runtime.area.sync_origin(runtime.stream.origin.offset);
    }
    runtime.recover(&reference);
    assert_eq!(
        runtime
            .area
            .items
            .iter()
            .filter(|item| item.id == station_food)
            .count(),
        1
    );
    assert!(runtime.area.items.get(station_food).unwrap().removed);
    assert!(runtime.area.items.get(station_food).unwrap().body.is_none());
}
