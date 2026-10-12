mod common;

use my_summer_slop::interaction::items::ItemStore;
use my_summer_slop::reference::ItemId;
use my_summer_slop::vehicle::VehiclePose;
use rapier3d::prelude::RigidBodyHandle;

#[test]
fn engine_fluid_view_changes_the_owned_item_quantities() {
    let reference = common::reference();
    let mut items = ItemStore::new(&reference).expect("fixture item store");
    let fuel_id = reference.engine.fuel_tank;
    let initial = items
        .get(fuel_id)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    {
        let fluids = items.vehicle_fluids().expect("three engine tanks");
        fluids.fuel.mixture.quantities_liters[0] -= 0.1;
    }
    let current = items
        .get(fuel_id)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    assert!((current - initial + 0.1).abs() < 0.00001);
}

#[test]
fn item_pair_rejects_one_container_borrowed_twice() {
    let reference = common::reference();
    let mut items = ItemStore::new(&reference).expect("fixture item store");
    let fuel = reference.engine.fuel_tank;
    assert!(items.get_pair_mut(fuel, fuel).is_none());
    let initial_count = items.len();
    let (fuel_item, oil_item) = items.get_pair_mut(fuel, reference.engine.oil_tank).unwrap();
    assert_eq!(fuel_item.id, fuel);
    assert_eq!(oil_item.id, reference.engine.oil_tank);
    assert_eq!(items.len(), initial_count);
}

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
