//! Car prototypes and engine parts of spawned world contents.
//! A car copy is a vehicle body, a car runtime and the engine and radiator parts on it.
use crate::interaction::items::ItemStore;
use crate::reference::{
    Bounds, EnginePartReference, ItemId, RadiatorPartReference, ReferenceError,
    WorldPrototypeReference, WorldReference,
};
use crate::simulation::car_lights::CarLightLinks;
use crate::simulation::powertrain::{EnginePart, PowertrainParts, RadiatorPart};
use crate::starting_area::VehicleRuntime;
use crate::vehicle::PhysicsWorld;
use crate::world::changes::ChangeRecords;
use crate::world::contents::ContentInstance;
use crate::world::spawn_rules::{SourceRules, number};
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
    skid: &crate::reference::SkidReference,
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
        let electronics_of = |node: Option<u32>| -> Result<Option<f32>, ReferenceError> {
            Ok(node
                .map(|node| electronics_output(world, rules, prototype, node))
                .transpose()?
                .flatten())
        };
        // The nearest item that holds a node: a part, a usable or the car root.
        let owner_item = |node: u32| {
            let mut cursor = Some(node);
            while let Some(n) = cursor {
                if let Some(item) = node_item(n) {
                    return Some(item);
                }
                cursor = prototype.nodes.get(n as usize).and_then(|n| n.parent);
            }
            None
        };
        let lights = record
            .lights
            .iter()
            .map(|light| {
                Ok(CarLightLinks {
                    electronics: electronics_of(light.electronics)?,
                    owner: owner_item(light.node).filter(|item| Some(*item) != content.item),
                    doors: light
                        .cabin
                        .iter()
                        .flat_map(|c| &c.door_slots)
                        .map(|&node| owner_item(node))
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, ReferenceError>>()?;
        let electronics = electronics_of(record.electronics)?;
        spawn(
            CarCopy {
                key: ObjectKey::Generated(content.content.placement.id.clone()),
                prototype: index,
                chassis,
                colliders: &colliders,
                bodies: instance
                    .bodies
                    .iter()
                    .copied()
                    .filter(|body| *body != chassis)
                    .collect(),
                fuel_tank: record.fuel_tank.and_then(node_item),
                electronics,
                lights,
                light_switch: record
                    .light_switch
                    .as_ref()
                    .and_then(|s| node_item(s.usable)),
            },
            prototype,
            skid,
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
        register_engine(item, record, oil_tank, items, powertrain, changes)?;
    }
    for record in &prototype.radiators {
        let Some(item) = node_item(record.node) else {
            continue;
        };
        let coolant_tank = record.coolant_tank.and_then(node_item);
        register_radiator(item, record, coolant_tank, items, powertrain, changes);
    }
    Ok(())
}

/// Registers the engine part of `item` with its kept state.
pub(crate) fn register_engine(
    item: ItemId,
    record: &EnginePartReference,
    oil_tank: Option<ItemId>,
    items: &ItemStore,
    powertrain: &mut PowertrainParts,
    changes: &mut ChangeRecords,
) -> Result<(), ReferenceError> {
    let mut part = EnginePart::new(item, record.clone(), oil_tank);
    if let Some(state) = changes.take_engine(&items.object_key(item)) {
        part.state
            .restore_save(&state)
            .map_err(|e| ReferenceError(format!("Saved engine state: {e}")))?;
    }
    powertrain.register_engine(part);
    Ok(())
}

/// Registers the radiator part of `item` with its kept fan state.
pub(crate) fn register_radiator(
    item: ItemId,
    record: &RadiatorPartReference,
    coolant_tank: Option<ItemId>,
    items: &ItemStore,
    powertrain: &mut PowertrainParts,
    changes: &mut ChangeRecords,
) {
    let mut part = RadiatorPart::new(item, record.clone(), coolant_tank);
    if let Some(fan_on) = changes.take_radiator(&items.object_key(item)) {
        part.fan_on = fan_on;
    }
    powertrain.register_radiator(part);
}

/// Source `electronicsscript` output of prototype node `node`: `maxOutput` less the jam share.
/// None: the node has no electronics script.
pub(crate) fn electronics_output(
    world: &WorldReference,
    rules: &SourceRules,
    prototype: &WorldPrototypeReference,
    node: u32,
) -> Result<Option<f32>, ReferenceError> {
    let node = prototype
        .nodes
        .get(node as usize)
        .ok_or_else(|| ReferenceError("Car electronics node missing".into()))?;
    let Some(record) = rules.component(world, &node.source, "electronicsscript")? else {
        return Ok(None);
    };
    let f = record.fields;
    Ok(Some(
        number(f, "maxOutput")? * crate::world::spawn_lamps::electronics_factor(f)?,
    ))
}

/// One car copy of a world instance, as the binder spawns it.
pub(crate) struct CarCopy<'a> {
    pub key: ObjectKey,
    /// Index of the car prototype in `WorldReference::prototypes`.
    pub prototype: usize,
    pub chassis: RigidBodyHandle,
    /// Live handles of the record chassis colliders.
    pub colliders: &'a [ColliderHandle],
    /// Other bodies of the car instance.
    pub bodies: Vec<RigidBodyHandle>,
    /// Car root `tankscript` item.
    pub fuel_tank: Option<ItemId>,
    /// `carscript.electronics` output. None: the car has no electronics.
    pub electronics: Option<f32>,
    /// Links of each record light controller.
    pub lights: Vec<CarLightLinks>,
    /// The item of the `carscript.InteractSwitchLights` usable.
    pub light_switch: Option<ItemId>,
}

/// Makes the chassis a vehicle and adds its car runtime with the retired state of its key.
pub(crate) fn spawn(
    copy: CarCopy,
    prototype: &WorldPrototypeReference,
    skid: &crate::reference::SkidReference,
    physics: &mut PhysicsWorld,
    vehicles: &mut BTreeMap<ObjectKey, VehicleRuntime>,
    changes: &mut ChangeRecords,
) -> Result<(), ReferenceError> {
    let record = prototype
        .vehicle
        .as_ref()
        .ok_or_else(|| ReferenceError("Prototype has no vehicle record".into()))?;
    let key = copy.key;
    if vehicles.contains_key(&key) {
        return Err(ReferenceError("Car runtime already exists".into()));
    }
    // Plate binder: each copy draws its labels from its key. A reload draws the same texts.
    let plates = crate::plates::copy_texts(&record.plates, key.seed())?;
    physics.add_vehicle(key.clone(), copy.chassis, copy.colliders, prototype)?;
    let mut runtime = VehicleRuntime::world(
        copy.prototype,
        record,
        skid,
        crate::starting_area::WorldCarParts {
            seed: key.seed(),
            fuel_tank: copy.fuel_tank,
            recovery_bounds: chassis_bounds(physics, copy.colliders),
            bodies: copy.bodies,
            electronics: copy.electronics,
            lights: copy.lights,
            light_switch: copy.light_switch,
            plates,
        },
    );
    if let Some(state) = changes.take_vehicle(&key) {
        if state.gear >= record.gear_ratios.len() {
            return Err(ReferenceError("Saved car gear is out of range".into()));
        }
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
        changes.keep_engine(items.object_key(item), part.state.capture_save());
    }
    if let Some(part) = powertrain.radiator(item) {
        changes.keep_radiator(items.object_key(item), part.fan_on);
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
        changes.keep_vehicle(key.clone(), runtime.state());
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
            CarCopy {
                key: ObjectKey::Generated(id.clone()),
                prototype: 0,
                chassis: body,
                colliders: &[collider],
                bodies: Vec::new(),
                fuel_tank: Some(item),
                electronics: None,
                lights: Vec::new(),
                light_switch: None,
            },
            prototype,
            &crate::wheel_effects::tests::skid(),
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

    #[test]
    fn plate_texts_differ_per_copy_and_survive_reload() {
        let mut prototype = crate::vehicle::car_prototype_fixture();
        let identity = [0., 0., 0., 1.];
        let plate = |node: u32, same_as: Option<u32>| {
            serde_json::from_value(serde_json::json!({
                "source": {"file": "a", "path_id": 40 + node}, "node": node, "text_node": node,
                "same_as": same_as, "initial_text": "", "initial_same": false,
                "font_source": {"file": "f", "path_id": 1}, "material_source": {"file": "f", "path_id": 2},
                "sorting": [0, 0],
                "layout": {"bounds": [1.0, 0.2], "pivot": [0.5, 0.5], "anchor_min": [0.5, 0.5],
                    "anchor_max": [0.5, 0.5], "font_size": 10.0, "font_size_base": 10.0, "font_size_min": 1.0,
                    "font_size_max": 20.0, "font_style": 1, "font_weight": 400, "horizontal_alignment": 2,
                    "vertical_alignment": 512, "auto_sizing": false, "kerning": true, "word_wrapping": false,
                    "extra_padding": false, "orthographic": true, "rich_text": false, "overflow_mode": 0,
                    "max_visible_descender": true, "spacing": [0.0, 0.0, 0.0, 0.0],
                    "margin": [0.0, 0.0, 0.0, 0.0], "color": [1.0, 1.0, 1.0, 1.0],
                    "transform": {"translation": [0.0, 0.0, 0.0], "rotation": identity, "scale": [1.0, 1.0, 1.0]}}
            }))
            .unwrap()
        };
        // The front label copies its text to the rear label.
        prototype.vehicle.as_mut().unwrap().plates = vec![plate(1, Some(2)), plate(2, None)];
        let copy = |ordinal| GeneratedId {
            layer: WorldLayer::Prop,
            generator: SourceId {
                file: "a".into(),
                path_id: 1,
            },
            unit: [0, 0],
            ordinal,
            children: Vec::new(),
        };
        // One session: spawn the copy and read its plate texts.
        let texts = |id: &GeneratedId| {
            let mut physics = PhysicsWorld::empty(1, 2);
            let mut items = ItemStore::from_states(Vec::new());
            let mut vehicles = BTreeMap::new();
            let mut changes = ChangeRecords::default();
            spawn_copy(
                id,
                &prototype,
                &mut physics,
                &mut items,
                &mut vehicles,
                &mut changes,
            );
            vehicles
                .remove(&ObjectKey::Generated(id.clone()))
                .unwrap()
                .plates
        };
        let first = texts(&copy(3));
        assert_eq!(first.len(), 2);
        assert_eq!(first[0], first[1], "sameAsThis shares the text");
        let bytes = first[0].as_bytes();
        assert!(
            first[0].len() == 7
                && bytes[..3].iter().all(u8::is_ascii_uppercase)
                && bytes[3] == b'-'
                && bytes[4..].iter().all(u8::is_ascii_digit),
            "{first:?}"
        );
        assert_ne!(texts(&copy(4)), first, "each copy draws its own plate");
        assert_eq!(texts(&copy(3)), first, "a reload draws the same plate");
    }

    #[test]
    fn saved_car_returns_with_engine_gear_and_handbrake() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Saved {
            #[serde(with = "crate::save::identity::lossless")]
            changes: ChangeRecords,
        }
        let prototype = crate::vehicle::car_prototype_fixture();
        let engine_record: EnginePartReference =
            serde_json::from_value(crate::reference::engine_part_fixture()).unwrap();
        let radiator_record: RadiatorPartReference = serde_json::from_value(serde_json::json!({
            "source": {"file": "a", "path_id": 15}, "node": 2, "coolant_tank": null,
            "fan_node": 3, "thermostat_range": [80.0, 90.0], "fan_temperature_on": 95.0,
            "fan_temperature_off": 90.0, "fan_on_when_engine_runs": false, "max_value": 1.0,
            "max_fan_value": 1.0, "max_fan_speed": 1.0, "max_fluid_loss": 1.0
        }))
        .unwrap();
        let car = GeneratedId {
            layer: WorldLayer::Prop,
            generator: SourceId {
                file: "a".into(),
                path_id: i64::MIN + 1,
            },
            unit: [i64::MAX - 1, -3],
            ordinal: 3,
            children: Vec::new(),
        };
        let engine = GeneratedId {
            children: vec![(
                SourceId {
                    file: "a".into(),
                    path_id: -9,
                },
                7,
            )],
            ..car.clone()
        };
        let key = ObjectKey::Generated(car.clone());
        let engine_key = ObjectKey::Generated(engine.clone());
        // One session: a car copy with an engine and a radiator part.
        let run = |changes: &mut ChangeRecords| {
            let mut physics = PhysicsWorld::empty(1, 2);
            physics.insert_box([0., -0.5, 0.], [500., 0.5, 500.]);
            let mut items = ItemStore::from_states(Vec::new());
            let mut vehicles = BTreeMap::new();
            let mut powertrain = PowertrainParts::default();
            spawn_copy(
                &car,
                &prototype,
                &mut physics,
                &mut items,
                &mut vehicles,
                changes,
            );
            let pose = AbsolutePose {
                position: START,
                rotation: [0., 0., 0., 1.],
            };
            let part = items
                .insert_generated(engine.clone(), &prototype, pose)
                .unwrap();
            assert_eq!(items.object_key(part), engine_key);
            register_engine(part, &engine_record, None, &items, &mut powertrain, changes).unwrap();
            register_radiator(
                part,
                &radiator_record,
                None,
                &items,
                &mut powertrain,
                changes,
            );
            (vehicles, powertrain, part)
        };

        let mut changes = ChangeRecords::default();
        let (mut vehicles, mut powertrain, part) = run(&mut changes);
        let car_state = vehicles.get_mut(&key).unwrap();
        car_state.drivetrain.restore_gear(2);
        car_state.handbrake = true;
        car_state.ignition.ignition = true;
        let mut engine_state = powertrain.engine(part).unwrap().state.capture_save();
        engine_state.rpm = 2400.;
        engine_state.temperature = 86.;
        engine_state.elapsed = 12.;
        engine_state.last_run_time = 11.99;
        engine_state.supplied_fuel = 0.3;
        engine_state.start_draw = "5".into();
        powertrain
            .engine_mut(part)
            .unwrap()
            .state
            .restore_save(&engine_state)
            .unwrap();
        powertrain.radiator_mut(part).unwrap().fan_on = true;

        // Save the live states as the game does, through JSON.
        changes.keep_vehicle(key.clone(), vehicles[&key].state());
        changes.keep_engine(
            engine_key.clone(),
            powertrain.engine(part).unwrap().state.capture_save(),
        );
        changes.keep_radiator(engine_key.clone(), true);
        let json = serde_json::to_vec(&Saved { changes }).unwrap();

        // Load into a new session.
        let mut loaded = serde_json::from_slice::<Saved>(&json).unwrap().changes;
        let (vehicles, powertrain, part) = run(&mut loaded);
        let car_state = &vehicles[&key];
        assert_eq!(car_state.drivetrain.gear(), 2);
        assert!(car_state.handbrake);
        assert!(car_state.ignition.ignition);
        let restored = &powertrain.engine(part).unwrap().state;
        assert_eq!(restored.capture_save(), engine_state);
        assert!(restored.output().running);
        assert!(powertrain.radiator(part).unwrap().fan_on);
        // Each object took its state back.
        assert!(loaded.vehicle(&key).is_none() && loaded.engine(&engine_key).is_none());
    }
}
