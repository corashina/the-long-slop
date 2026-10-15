mod common;

use my_summer_slop::interaction::items::ItemStore;
use my_summer_slop::reference::ItemId;
use my_summer_slop::vehicle::VehiclePose;
use rapier3d::prelude::RigidBodyHandle;

#[test]
fn consumable_ids_reject_duplicates_and_keep_source_bites() {
    let reference = common::reference();
    let mut items = ItemStore::new(&reference).unwrap();
    let starter = items.get(reference.survival.starter_food).unwrap();
    assert_eq!(starter.food.unwrap().bite, 0);
    assert!(!starter.removed);
    let id = ItemId(0x8000_0001);
    let pose = VehiclePose {
        position: [1., 2., 3.],
        rotation: [0., 0., 0., 1.],
    };
    items
        .insert_consumable(
            id,
            1,
            &reference.survival.foods[1],
            pose,
            RigidBodyHandle::invalid(),
        )
        .unwrap();
    assert_eq!(items.get(id).unwrap().food.unwrap().prototype_index, 1);
    assert!(
        items
            .insert_consumable(
                id,
                1,
                &reference.survival.foods[1],
                pose,
                RigidBodyHandle::invalid()
            )
            .is_err()
    );
}
