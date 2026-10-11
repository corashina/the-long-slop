#![allow(dead_code)]

use my_summer_slop::reference::{ColliderGeometry, ColliderId, ReferenceManifest};
use my_summer_slop::vehicle::{PhysicsWorld, VehiclePose};

pub fn reference() -> ReferenceManifest {
    let mut reference = ReferenceManifest::from_json(include_bytes!("reference.json"))
        .expect("valid native fixture");
    reference.colliders[0].geometry = ColliderGeometry::Box {
        center: [0.0; 3],
        half_extents: [0.76, 0.24, 1.55],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    let mut cabin = reference.colliders[0].clone();
    cabin.id = ColliderId(2);
    cabin.source.path_id += 1;
    cabin.geometry = ColliderGeometry::Box {
        center: [0.0, 0.57, 0.2],
        half_extents: [0.65, 0.36, 0.72],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    reference.vehicle.colliders.push(cabin.id);
    reference.colliders.push(cabin);
    reference
}

pub fn world() -> PhysicsWorld {
    PhysicsWorld::new(
        &reference(),
        VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    )
    .expect("valid fixture physics")
}
