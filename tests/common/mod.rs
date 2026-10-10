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
    cabin.id = ColliderId(
        reference
            .colliders
            .iter()
            .map(|collider| collider.id.0)
            .max()
            .unwrap()
            + 1,
    );
    cabin.source.path_id += 1;
    cabin.geometry = ColliderGeometry::Box {
        center: [0.0, 0.57, 0.2],
        half_extents: [0.65, 0.36, 0.72],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    reference.vehicle.colliders.push(cabin.id);
    reference
        .action_bindings
        .push(my_summer_slop::reference::ActionBinding {
            collider: cabin.id,
            use_owner: None,
            pickup_owner: None,
            removal_owner: None,
            push_owner: None,
        });
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

pub fn resolved(
    target: Option<my_summer_slop::interaction::AimTarget>,
) -> my_summer_slop::interaction::ResolvedAim {
    my_summer_slop::interaction::ResolvedAim {
        use_target: target,
        grab_target: target,
        installation_target: target,
        filling_target: target,
        focus_target: target,
        ..Default::default()
    }
}

pub fn focus(
    interaction: &my_summer_slop::interaction::InteractionState,
    physics: &PhysicsWorld,
    view: VehiclePose,
) -> Option<my_summer_slop::interaction::AimTarget> {
    interaction
        .resolve(
            physics,
            view,
            &my_summer_slop::interaction::items::ItemStore::new(&reference()).unwrap(),
        )
        .focus_target
}
