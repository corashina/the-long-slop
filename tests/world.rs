mod common;

use my_summer_slop::reference::ReferenceManifest;
use my_summer_slop::simulation::DriveInput;
use my_summer_slop::vehicle::{PhysicsWorld, VehiclePose};
use my_summer_slop::world::generation::{DotNetRandom, perlin};
use my_summer_slop::world::{Generator, WorldOrigin, WorldStream};

fn physics(reference: &ReferenceManifest) -> PhysicsWorld {
    PhysicsWorld::new(
        reference,
        VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    )
    .expect("fixture physics")
}

fn close(a: [f32; 3], b: [f32; 3], tolerance: f32) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < tolerance)
}

#[test]
fn source_random_and_noise_match_engine_values() {
    // Published .NET Framework values for `new Random(0)` and `new Random(42).Next(100)`.
    let mut random = DotNetRandom::new(0);
    assert_eq!(random.next_double(), 0.7262432699679598);
    assert_eq!(random.next_double(), 0.8173253595909687);
    assert_eq!(DotNetRandom::new(42).range(0, 100), 66);
    // `Mathf.PerlinNoise` returns this value at every lattice point.
    for point in [[0.0, 0.0], [3.0, -7.0], [1000.0, 25.0]] {
        assert!((perlin(point[0], point[1]) - 0.4652731).abs() < 1e-6);
    }
}

#[test]
fn adjacent_tiles_share_edge_heights_and_normals() {
    let reference = common::reference();
    let mut generator = Generator::new(&reference.world);
    let edge = |tile: &my_summer_slop::world::Tile, axis: usize, value: f32| {
        let mut points: Vec<_> = tile
            .positions
            .iter()
            .zip(&tile.normals)
            .filter(|(p, _)| (p[axis] - value).abs() < 1e-3)
            .map(|(p, n)| {
                let absolute = [
                    p[0] + tile.center[0] as f32,
                    p[1],
                    p[2] + tile.center[2] as f32,
                ];
                (absolute, *n)
            })
            .collect();
        points.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]).then(a.0[2].total_cmp(&b.0[2])));
        points
    };
    for (key, next, axis, value) in [
        ([0, 0], [1, 0], 0, 50.0),
        ([3, 7], [3, 8], 2, -50.0),
        ([-4, -2], [-3, -2], 0, 50.0),
    ] {
        let a = generator.tile(key, &[]);
        let b = generator.tile(next, &[]);
        let left = edge(&a, axis, value);
        let right = edge(&b, axis, -value);
        assert_eq!(left.len(), 20);
        assert_eq!(left.len(), right.len());
        for ((p, n), (q, m)) in left.iter().zip(&right) {
            assert!(close(*p, *q, 1e-3), "{p:?} {q:?}");
            assert!(close(*n, *m, 1e-4), "{n:?} {m:?}");
        }
    }
}

#[test]
fn skinned_road_follows_the_generated_path() {
    let reference = common::reference();
    let mut generator = Generator::new(&reference.world);
    generator.extend_to(1000.0);
    let road = &reference.world.road;
    for index in 0..8 {
        let start = generator.start(index);
        let mesh = generator.segment(index);
        for bone in 0..road.bone_count {
            let point = generator.game(generator.path.point(index, bone).map(|v| v as f64));
            let expected = [
                (point[0] - start[0]) as f32,
                (point[1] - start[1]) as f32 + road.above_terrain,
                (point[2] - start[2]) as f32,
            ];
            // Fixture vertex pairs sit at each bone end, four meters to each side.
            let left = mesh.positions[2 * bone + 2];
            let right = mesh.positions[2 * bone + 3];
            let middle = std::array::from_fn(|i| (left[i] + right[i]) * 0.5);
            assert!(close(middle, expected, 2e-3), "{middle:?} {expected:?}");
            assert!(mesh.normals[2 * bone + 2][1] > 0.9);
        }
        let next = generator.start(index + 1);
        let end = generator.game(generator.path.point(index, 9).map(|v| v as f64));
        assert!((next[1] - end[1] - road.above_terrain as f64).abs() < 1e-3);
        assert!((next[0] - end[0]).abs() < 1e-3 && (next[2] - end[2]).abs() < 1e-3);
    }
}

#[test]
fn streaming_loads_tiles_roads_and_bounds_colliders() {
    let reference = common::reference();
    let mut physics = physics(&reference);
    let base = physics.collider_count();
    let mut stream = WorldStream::new(&reference);
    for z in [0.0, -150.0, -2000.0, -2050.0, 300.0, 0.0] {
        stream.sync_at([5.0, 0.0, z], &mut physics);
        let tiles = stream.tiles().count();
        assert!((19..=90).contains(&tiles), "{tiles}");
        let roads: usize = stream
            .roads()
            .map(|r| 1 + r.mesh.colliders.len() + 2 * r.station.is_some() as usize)
            .sum();
        assert_eq!(physics.collider_count(), base + tiles + roads);
        let size = reference.world.terrain.tile_size;
        let range = reference.world.terrain.remove_distance + size;
        for loaded in stream.tiles() {
            let center = loaded.tile.center;
            let near = |x: f64, z: f64| ((center[0] - x).powi(2) + (center[2] - z).powi(2)).sqrt();
            assert!(near(5.0, z) < range as f64 || near(0.0, 0.0) < range as f64);
        }
        let generate = reference.world.road.generate_distance as f64;
        for road in stream.roads() {
            let near =
                |x: f64, z: f64| ((road.start[0] - x).powi(2) + (road.start[2] - z).powi(2)).sqrt();
            assert!(near(5.0, z) <= generate || near(0.0, 0.0) <= generate);
        }
        assert!(stream.roads().count() >= 5);
    }
}

#[test]
fn recovery_places_the_car_on_the_road_and_at_stations() {
    let reference = common::reference();
    let mut physics = physics(&reference);
    let mut stream = WorldStream::new(&reference);
    stream.sync_at([0.0, 0.0, 0.0], &mut physics);
    physics.reset(VehiclePose {
        position: [40.0, 30.0, -1200.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
    });
    for at_station in [false, true] {
        let pose = stream.reset_pose(at_station, &mut physics);
        physics.reset(pose);
        for _ in 0..120 {
            physics.apply_vehicle_forces(DriveInput::default(), 0.0);
            physics.step_physics();
        }
        let snapshot = physics.snapshot();
        assert!(snapshot.wheels_contact.into_iter().all(|c| c));
        assert!(snapshot.speed_mps < 0.5);
        let absolute = stream.origin.absolute(snapshot.pose.position);
        let station = stream.nearest_station(absolute);
        let distance = ((absolute[0] - station.point[0]).powi(2)
            + (absolute[2] - station.point[2]).powi(2))
        .sqrt();
        assert_eq!(distance < 2.0, at_station, "{distance}");
        assert!(
            stream
                .roads()
                .any(|r| r.station.is_some_and(|s| s.id == station.id))
                || !at_station
        );
    }
}

#[test]
fn driving_crosses_tiles_and_rebase_without_trip_jump() {
    let reference = common::reference();
    let mut physics = physics(&reference);
    let mut stream = WorldStream::new(&reference);
    physics.reset(VehiclePose {
        position: [0.0, 30.0, -2000.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
    });
    let pose = stream.reset_pose(false, &mut physics);
    physics.reset(pose);
    for _ in 0..120 {
        physics.apply_vehicle_forces(DriveInput::default(), 2600.0);
        physics.step_physics();
    }
    let mut previous = stream.origin.absolute(physics.snapshot().pose.position);
    let mut trip = 0.0;
    let mut rebases = 0;
    for _ in 0..900 {
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
        assert!(snapshot.wheels_contact.iter().any(|c| *c));
    }
    assert!(trip > 150.0, "Trip: {trip}");
    assert!(rebases > 0);
}

#[test]
fn walking_focus_rebases_a_parked_vehicle_without_travel() {
    let reference = common::reference();
    let mut physics = physics(&reference);
    let mut stream = WorldStream::new(&reference);
    let before = physics.snapshot();
    let shift = stream.rebase([2300.0, 0.0, 0.0], &mut physics);
    assert_eq!(shift, Some([2304.0, 0.0, 0.0]));
    let after = physics.snapshot();
    let absolute = stream.origin.absolute(after.pose.position);
    assert_eq!(absolute, [0.0, 1.1_f32 as f64, 0.0]);
    assert_eq!(before.linear_velocity, after.linear_velocity);
    assert_eq!(before.angular_velocity, after.angular_velocity);
    let mut origin = WorldOrigin::default();
    origin.shift([128.0, 0.0, -256.0]);
    assert_eq!(
        origin.local(origin.absolute([1.0, 2.0, 3.0])),
        [1.0, 2.0, 3.0]
    );
}

#[test]
fn home_terrain_keeps_interiors_clear_and_holds_flat_zones() {
    use my_summer_slop::reference::ColliderGeometry;
    let mut reference = common::reference();
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
    let mut physics = physics(&common::reference());
    let mut world = WorldStream::new(&reference);
    world.sync_at([40.0, 0.0, -64.0], &mut physics);
    // The fixture flat zone holds Unity height 1 within 10 meters of X 40, Z 65.
    let flat = rapier3d::prelude::Ray::new(
        rapier3d::math::Vector::new(40.0, 20.0, -64.0),
        -rapier3d::math::Vector::Y,
    );
    let hit = physics
        .queries(rapier3d::prelude::QueryFilter::default())
        .cast_ray(&flat, 40.0, true)
        .unwrap();
    assert!((20.0 - hit.1 - 1.0).abs() < 1e-3, "{}", 20.0 - hit.1);
    for loaded in world.tiles() {
        let tile = &loaded.tile;
        for triangle in &tile.indices {
            let centroid = |axis: usize| {
                triangle
                    .iter()
                    .map(|i| tile.positions[*i as usize][axis])
                    .sum::<f32>()
                    / 3.0
                    + tile.center[axis] as f32
            };
            assert!(
                !(35.0..=38.0).contains(&centroid(0)) || !(-67.0..=-64.0).contains(&centroid(2))
            );
        }
    }
    for x in [34.9, 38.1] {
        let ray = rapier3d::prelude::Ray::new(
            rapier3d::math::Vector::new(x, 20.0, -65.0),
            -rapier3d::math::Vector::Y,
        );
        assert!(
            physics
                .queries(rapier3d::prelude::QueryFilter::default())
                .cast_ray(&ray, 40.0, true)
                .is_some(),
            "terrain must reach the exact room boundary at x{x}",
        );
    }
    let ray = rapier3d::prelude::Ray::new(
        rapier3d::math::Vector::new(35.1, 20.0, -65.0),
        -rapier3d::math::Vector::Y,
    );
    assert!(
        physics
            .queries(rapier3d::prelude::QueryFilter::default())
            .cast_ray(&ray, 40.0, true)
            .is_none(),
        "room must stay clear"
    );
}
