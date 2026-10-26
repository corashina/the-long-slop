//! Car prototypes and engine parts of spawned world contents.
//! A car copy is a vehicle body, a car runtime and the engine and radiator parts on it.
use crate::interaction::items::ItemStore;
use crate::reference::{Bounds, ItemId, ReferenceError, WorldPrototypeReference, WorldReference};
use crate::simulation::powertrain::{EnginePart, PowertrainParts, RadiatorPart};
use crate::starting_area::VehicleRuntime;
use crate::vehicle::PhysicsWorld;
use crate::world::changes::ChangeRecords;
use crate::world::contents::ContentInstance;
use crate::world::spawn_rules::SourceRules;
use crate::world::{ObjectKey, StructureInstance};
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};
use std::collections::BTreeMap;

/// Binds the engine and radiator parts and the car of one bound content.
/// Call after slots, parts and wheels bind and before seats bind: seats find the vehicle.
#[allow(clippy::too_many_arguments)]
pub(super) fn bind(
    content: &ContentInstance,
    instance: &StructureInstance,
    prototype: &WorldPrototypeReference,
    world: &WorldReference,
    rules: &SourceRules,
    items: &ItemStore,
    physics: &mut PhysicsWorld,
    vehicles: &mut BTreeMap<ObjectKey, VehicleRuntime>,
    powertrain: &mut PowertrainParts,
    changes: &mut ChangeRecords,
) -> Result<(), ReferenceError> {
    let node_item = |node: u32| {
        crate::world::content_mechanisms::node_item(content, prototype, world, rules, node as usize)
    };
    // The car first: parts register only when nothing can fail.
    if let Some(record) = &prototype.vehicle {
        if content.item.is_none() {
            return Err(ReferenceError("Car prototype root has no item".into()));
        }
        let chassis = *instance
            .bodies
            .get(record.body as usize)
            .ok_or_else(|| ReferenceError("Car chassis body is not published".into()))?;
        let colliders = record
            .colliders
            .iter()
            .map(|&index| {
                let source = &prototype.colliders[index as usize].source;
                instance
                    .colliders
                    .iter()
                    .find(|(s, _)| s == source)
                    .map(|(_, handle)| *handle)
                    .ok_or_else(|| ReferenceError("Car chassis collider is not published".into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let index = world
            .prototypes
            .iter()
            .position(|p| p.source == prototype.source)
            .ok_or_else(|| ReferenceError("Car prototype missing".into()))?;
        spawn(
            ObjectKey::Generated(content.content.placement.id.clone()),
            index,
            prototype,
            chassis,
            &colliders,
            record.fuel_tank.and_then(node_item),
            physics,
            vehicles,
            changes,
        )?;
    }
    for record in &prototype.engines {
        // A random part rule can leave the engine node without an item.
        let Some(item) = node_item(record.node) else {
            continue;
        };
        let oil_tank = record.oil_tank.and_then(node_item);
        let mut part = EnginePart::new(item, record.clone(), oil_tank);
        if let Some(state) = changes.take_engine(&items.object_key(item)) {
            part.state = state;
        }
        powertrain.register_engine(part);
    }
    for record in &prototype.radiators {
        let Some(item) = node_item(record.node) else {
            continue;
        };
        let coolant_tank = record.coolant_tank.and_then(node_item);
        let mut part = RadiatorPart::new(item, record.clone(), coolant_tank);
        if let Some(fan_on) = changes.take_radiator(&items.object_key(item)) {
            part.fan_on = fan_on;
        }
        powertrain.register_radiator(part);
    }
    Ok(())
}

/// Makes `chassis` a vehicle and adds its car runtime with the retired state of `key`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn(
    key: ObjectKey,
    index: usize,
    prototype: &WorldPrototypeReference,
    chassis: RigidBodyHandle,
    colliders: &[ColliderHandle],
    fuel_tank: Option<ItemId>,
    physics: &mut PhysicsWorld,
    vehicles: &mut BTreeMap<ObjectKey, VehicleRuntime>,
    changes: &mut ChangeRecords,
) -> Result<(), ReferenceError> {
    let record = prototype
        .vehicle
        .as_ref()
        .ok_or_else(|| ReferenceError("Prototype has no vehicle record".into()))?;
    if vehicles.contains_key(&key) {
        return Err(ReferenceError("Car runtime already exists".into()));
    }
    physics.add_vehicle(key.clone(), chassis, colliders, prototype)?;
    let mut runtime =
        VehicleRuntime::world(index, record, fuel_tank, chassis_bounds(physics, colliders));
    if let Some(state) = changes.take_vehicle(&key) {
        runtime.restore(&state);
    }
    vehicles.insert(key, runtime);
    Ok(())
}

/// Enabled solid chassis colliders in the chassis frame.
fn chassis_bounds(physics: &PhysicsWorld, colliders: &[ColliderHandle]) -> Bounds {
    let mut bounds = Bounds {
        min: [f32::MAX; 3],
        max: [f32::MIN; 3],
    };
    for &handle in colliders {
        let collider = &physics.rapier().colliders[handle];
        if collider.is_sensor() || !collider.is_enabled() {
            continue;
        }
        let local = collider
            .position_wrt_parent()
            .copied()
            .unwrap_or(rapier3d::math::Pose::IDENTITY);
        let aabb = collider.shape().compute_aabb(&local);
        for i in 0..3 {
            bounds.min[i] = bounds.min[i].min(aabb.mins[i]);
            bounds.max[i] = bounds.max[i].max(aabb.maxs[i]);
        }
    }
    bounds
}

/// Keeps the state of a retiring content in the world change records and removes its car
/// and its engine and radiator parts. Call before its items unload.
pub(super) fn retire(
    content: &ContentInstance,
    items: &ItemStore,
    physics: &mut PhysicsWorld,
    vehicles: &mut BTreeMap<ObjectKey, VehicleRuntime>,
    powertrain: &mut PowertrainParts,
    changes: &mut ChangeRecords,
) {
    for item in content
        .item
        .into_iter()
        .chain(content.bindings.iter().map(|b| b.item))
    {
        retire_parts(item, items, powertrain, changes);
    }
    if content.item.is_some() {
        let key = ObjectKey::Generated(content.content.placement.id.clone());
        if vehicles.contains_key(&key) {
            retire_vehicle(&key, physics, vehicles, changes);
        }
    }
}

/// Keeps the engine and radiator state of `item` and removes its parts.
pub(crate) fn retire_parts(
    item: ItemId,
    items: &ItemStore,
    powertrain: &mut PowertrainParts,
    changes: &mut ChangeRecords,
) {
    if let Some(part) = powertrain.engine(item) {
        changes.retire_engine(items.object_key(item), part.state.clone());
    }
    if let Some(part) = powertrain.radiator(item) {
        changes.retire_radiator(items.object_key(item), part.fan_on);
    }
    powertrain.remove(item);
}

/// Keeps the car-side state of car `key` and removes its runtime and vehicle body data.
pub(crate) fn retire_vehicle(
    key: &ObjectKey,
    physics: &mut PhysicsWorld,
    vehicles: &mut BTreeMap<ObjectKey, VehicleRuntime>,
    changes: &mut ChangeRecords,
) {
    if let Some(runtime) = vehicles.remove(key) {
        changes.retire_vehicle(key.clone(), runtime.state());
    }
    physics.remove_vehicle(key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluids::{FluidContainer, FluidEntry};
    use crate::reference::SourceId;
    use crate::simulation::{DriveInput, FIXED_DT};
    use crate::vehicle::{DriveTorques, VehiclePose, to_rapier};
    use crate::world::{AbsolutePose, GeneratedId, WorldLayer};
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    const START: [f64; 3] = [0., 0.5, 0.];

    /// One car copy as the world binder spawns it: instance body, root item with the
    /// spawn fuel, then the car. Returns the root item.
    fn spawn_copy(
        id: &GeneratedId,
        prototype: &WorldPrototypeReference,
        physics: &mut PhysicsWorld,
        items: &mut ItemStore,
        vehicles: &mut BTreeMap<ObjectKey, VehicleRuntime>,
        changes: &mut ChangeRecords,
    ) -> ItemId {
        let pose = AbsolutePose {
            position: START,
            rotation: [0., 0., 0., 1.],
        };
        let (body, collider) = physics.insert(
            RigidBodyBuilder::dynamic()
                .pose(to_rapier(VehiclePose {
                    position: START.map(|v| v as f32),
                    rotation: pose.rotation,
                }))
                .additional_mass(900.),
            ColliderBuilder::cuboid(0.8, 0.25, 2.0).density(0.),
        );
        let item = items.insert_generated(id.clone(), prototype, pose).unwrap();
        let state = items.set_body(item, Some(body)).unwrap();
        state.fluid = Some(FluidContainer::new(
            60.,
            &[FluidEntry {
                fluid_type: 2,
                amount: 40.,
            }],
        ));
        spawn(
            ObjectKey::Generated(id.clone()),
            0,
            prototype,
            body,
            &[collider],
            Some(item),
            physics,
            vehicles,
            changes,
        )
        .unwrap();
        physics.refresh_queries();
        item
    }

    fn fuel(items: &ItemStore, item: ItemId) -> f32 {
        items.get(item).unwrap().fluid.as_ref().unwrap().total()
    }

    #[test]
    fn retired_car_returns_with_pose_gear_and_fuel() {
        let mut prototype = crate::vehicle::car_prototype_fixture();
        prototype.vehicle.as_mut().unwrap().fuel_tank = Some(0);
        let id = GeneratedId {
            layer: WorldLayer::Prop,
            generator: SourceId {
                file: "a".into(),
                path_id: 1,
            },
            unit: [0, 0],
            ordinal: 3,
            children: Vec::new(),
        };
        let key = ObjectKey::Generated(id.clone());
        let mut physics = PhysicsWorld::empty(1, 2);
        physics.insert_box([0., -0.5, 0.], [500., 0.5, 500.]);
        let mut items = ItemStore::from_states(Vec::new());
        let mut vehicles = BTreeMap::new();
        let mut changes = ChangeRecords::default();
        let item = spawn_copy(
            &id,
            &prototype,
            &mut physics,
            &mut items,
            &mut vehicles,
            &mut changes,
        );
        assert_eq!(vehicles[&key].fuel_tank, Some(item));
        assert_eq!(vehicles[&key].drivetrain.gear(), 1, "initial gear");

        // Shift to gear 2, drive, burn fuel, set the handbrake.
        vehicles.get_mut(&key).unwrap().drivetrain.restore_gear(2);
        for _ in 0..(1. / FIXED_DT) as usize {
            physics.apply_vehicle_forces(
                &key,
                DriveInput {
                    throttle: 1.,
                    ..Default::default()
                },
                DriveTorques {
                    motor_nm: 1600.,
                    engine_brake_nm: 0.,
                },
            );
            physics.step_physics();
        }
        vehicles.get_mut(&key).unwrap().handbrake = true;
        items
            .get_mut(item)
            .unwrap()
            .fluid
            .as_mut()
            .unwrap()
            .restore_entries(&[FluidEntry {
                fluid_type: 2,
                amount: 12.5,
            }]);
        items.sync_poses(&physics);
        let moved = items.get(item).unwrap().pose.position;
        assert!(moved[2].abs() > 0.5 || moved[0].abs() > 0.5, "{moved:?}");

        // Retire: the change record path keeps pose and fuel, the binder keeps the car state.
        let body = items.get(item).unwrap().body.unwrap();
        let mut snapshot = items.generated_snapshot(item, None).unwrap();
        snapshot.linear_velocity = physics.rapier().bodies[body].linvel().to_array();
        retire_vehicle(&key, &mut physics, &mut vehicles, &mut changes);
        assert!(vehicles.is_empty());
        assert_eq!(physics.vehicle_count(), 0);
        items.set_body(item, None);
        assert!(items.remove_generated(item));
        physics.remove_body(body);

        // Respawn at the source placement, then restore the change record.
        let item = spawn_copy(
            &id,
            &prototype,
            &mut physics,
            &mut items,
            &mut vehicles,
            &mut changes,
        );
        assert_eq!(fuel(&items, item), 40., "spawn fuel before the record");
        items.restore_generated(item, &snapshot).unwrap();
        crate::world::changes::restore_motion(item, &snapshot, &items, &mut physics);
        let body = items.get(item).unwrap().body.unwrap();
        assert_eq!(physics.vehicle_of(body), Some(&key));
        let car = &vehicles[&key];
        assert_eq!(car.drivetrain.gear(), 2);
        assert!(car.handbrake);
        assert_eq!(fuel(&items, item), 12.5);
        let position = physics.rapier().bodies[body].translation().to_array();
        for i in 0..3 {
            assert!(
                (position[i] - moved[i]).abs() < 1e-4,
                "{position:?} {moved:?}"
            );
        }
        // The respawned car drives on.
        assert!(physics.snapshot(&key).speed_mps > 1.);
    }
}
