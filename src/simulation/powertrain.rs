//! Engine and radiator parts. Each car copy reads the parts that are mounted on it.
use crate::interaction::items::ItemStore;
use crate::reference::{
    EnginePartReference, EngineReference, ItemId, RadiatorPartReference, ReferenceManifest,
};
use crate::vehicle::garage_vehicle_index;
use crate::world::ObjectKey;

/// An engine part item and its source `enginescript`.
#[derive(Clone, Debug)]
pub struct EnginePart {
    pub item: ItemId,
    pub record: EnginePartReference,
    /// Item of `enginescript.T`.
    pub oil_tank: Option<ItemId>,
}

/// A radiator part item and its source `coolantTankscript`.
#[derive(Clone, Debug)]
pub struct RadiatorPart {
    pub item: ItemId,
    pub record: RadiatorPartReference,
    pub coolant_tank: Option<ItemId>,
}

/// Indices of the parts mounted on one car copy. Valid until the next registry change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Powertrain {
    pub engine: Option<usize>,
    pub radiator: Option<usize>,
}

/// Every live engine and radiator part, in registration order.
#[derive(Default)]
pub struct PowertrainParts {
    engines: Vec<EnginePart>,
    radiators: Vec<RadiatorPart>,
}

impl PowertrainParts {
    /// V03 bridge: the engine and radiator of each old garage car. Task 9 removes it.
    /// The old coolant tank item is the radiator item, as in the old engine rules.
    pub fn from_garage(reference: &ReferenceManifest) -> Self {
        let mut parts = Self::default();
        for vehicle in &reference.vehicles {
            let e = &vehicle.engine;
            parts.register_engine(EnginePart {
                item: e.item,
                record: garage_engine(e),
                oil_tank: Some(e.oil_tank),
            });
            parts.register_radiator(RadiatorPart {
                item: e.coolant_tank,
                record: garage_radiator(e),
                coolant_tank: Some(e.coolant_tank),
            });
        }
        parts
    }
    pub fn register_engine(&mut self, part: EnginePart) {
        self.engines.retain(|p| p.item != part.item);
        self.engines.push(part);
    }
    pub fn register_radiator(&mut self, part: RadiatorPart) {
        self.radiators.retain(|p| p.item != part.item);
        self.radiators.push(part);
    }
    /// Removes the engine or radiator record of a retired item.
    pub fn remove(&mut self, item: ItemId) {
        self.engines.retain(|p| p.item != item);
        self.radiators.retain(|p| p.item != item);
    }
    /// The engine and radiator mounted on car `key`: the parts whose mount chain reaches its root item.
    pub fn mounted_on(&self, items: &ItemStore, key: &ObjectKey) -> Powertrain {
        // V03 bridge: old garage car slots have no owner item. Task 9 removes it.
        if garage_vehicle_index(key).is_some() {
            return self.mounted(|item| items.mounted(item));
        }
        let root = items.resolve_key(key);
        self.mounted(|item| root.is_some_and(|root| items.descends(item, root)))
    }
    /// The first registered engine and radiator for which `on_car` is true.
    fn mounted(&self, on_car: impl Fn(ItemId) -> bool) -> Powertrain {
        Powertrain {
            engine: self.engines.iter().position(|p| on_car(p.item)),
            radiator: self.radiators.iter().position(|p| on_car(p.item)),
        }
    }
    pub fn engine(&self, index: usize) -> &EnginePart {
        &self.engines[index]
    }
    pub fn radiator(&self, index: usize) -> &RadiatorPart {
        &self.radiators[index]
    }
}

/// V03 bridge: old garage engine record in the engine part form. Task 9 removes it.
fn garage_engine(e: &EngineReference) -> EnginePartReference {
    EnginePartReference {
        source: e.source.clone(),
        // Old records have no prototype node. Runtime rules do not read it.
        node: 0,
        oil_tank: None,
        start_chance: e.start_chance,
        starter_rpm: e.starter_rpm,
        starter_rpm_lerp: e.starter_rpm_lerp,
        idle_rpm: e.idle_rpm,
        max_rpm: e.max_rpm,
        shutdown_rpm: e.shutdown_rpm,
        max_torque_nm: e.max_torque_nm,
        starting_push_force: e.starting_push_force,
        consumption_modifier: e.consumption_modifier,
        oil_consumption_modifier: e.oil_consumption_modifier,
        engine_brake_modifier: e.engine_brake_modifier,
        rpm_change_modifier: e.rpm_change_modifier,
        engine_start_time_modifier: e.engine_start_time_modifier,
        heat_gain_min: e.heat_gain_min,
        heat_gain_max: e.heat_gain_max,
        heat_loss: e.heat_loss,
        initial_temperature: e.initial_temperature,
        heat_loss_speed_modifier: e.heat_loss_speed_modifier,
        optimal_temperature: e.optimal_temperature,
        fuel_consumption: e.fuel_consumption,
        two_stroke: e.two_stroke,
        torque_curve: e.torque_curve.clone(),
        max_rpm_difference: e.max_rpm_difference,
        torque_modifier: e.torque_modifier,
        oil_fluid: e.oil_fluid,
        no_overheat: e.no_overheat,
        kickstart: e.kickstart,
        start_chance_interval_seconds: e.start_chance_interval_seconds,
        auto_shift_clutch_rpm: e.auto_shift_clutch_rpm,
        auto_shift_clutch_rpm_range: e.auto_shift_clutch_rpm_range,
        smoke_rate: e.smoke_rate,
    }
}

/// V03 bridge: old garage radiator record in the radiator part form. Task 9 removes it.
fn garage_radiator(e: &EngineReference) -> RadiatorPartReference {
    let r = &e.radiator;
    RadiatorPartReference {
        source: r.source.clone(),
        node: 0,
        coolant_tank: None,
        fan_node: r.fan_node.map(|n| n.0),
        thermostat_range: r.thermostat_range,
        fan_temperature_on: r.fan_temperature_on,
        fan_temperature_off: r.fan_temperature_off,
        fan_on_when_engine_runs: r.fan_on_when_engine_runs,
        max_value: r.max_value,
        max_fan_value: r.max_fan_value,
        max_fan_speed: r.max_fan_speed,
        max_fluid_loss: r.max_fluid_loss,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluids::{FluidContainer, FluidEntry};
    use crate::interaction::items::{ItemLocation, ItemState};
    use crate::reference::{EngineGlobals, PartSlotId};
    use crate::simulation::engine::{
        EngineInput, EngineRecords, EngineState, InstalledEngineInput,
    };
    use crate::vehicle::VehiclePose;
    use serde_json::json;

    const CARS: [ItemId; 2] = [ItemId(1), ItemId(2)];
    const SLOTS: [PartSlotId; 2] = [PartSlotId(1), PartSlotId(2)];
    const ENGINE: ItemId = ItemId(3);
    const OIL: ItemId = ItemId(4);
    const FUEL: [ItemId; 2] = [ItemId(5), ItemId(6)];

    fn item(id: ItemId, location: ItemLocation, fluid: Option<(u8, f32)>) -> ItemState {
        let pose = VehiclePose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
        };
        ItemState {
            id,
            body: None,
            inactive_pose: None,
            location,
            pose,
            local_pose: pose,
            generated_condition: None,
            temperature: None,
            fluid: fluid.map(|(fluid_type, amount)| {
                FluidContainer::new(10., &[FluidEntry { fluid_type, amount }])
            }),
            food: None,
            hit_sound: None,
            removed: false,
            pending_attachment: false,
            buried: false,
        }
    }

    /// Two car copies with an engine slot each. The engine starts in the first car.
    fn world() -> (ItemStore, PowertrainParts) {
        let mut items = ItemStore::from_states(vec![
            item(CARS[0], ItemLocation::World, None),
            item(CARS[1], ItemLocation::World, None),
            item(ENGINE, ItemLocation::PartSlot(SLOTS[0]), None),
            item(OIL, ItemLocation::Attached(ENGINE), Some((3, 5.))),
            item(FUEL[0], ItemLocation::Attached(CARS[0]), Some((2, 5.))),
            item(FUEL[1], ItemLocation::Attached(CARS[1]), Some((2, 5.))),
        ]);
        for (slot, car) in SLOTS.into_iter().zip(CARS) {
            items.register_generated_slot(slot, Some(car));
        }
        let mut record: EnginePartReference =
            serde_json::from_value(crate::reference::engine_part_fixture()).unwrap();
        // The starter must lift the rpm past the shutdown rpm at this step length.
        record.starter_rpm = 600.;
        let mut parts = PowertrainParts::default();
        parts.register_engine(EnginePart {
            item: ENGINE,
            record,
            oil_tank: Some(OIL),
        });
        (items, parts)
    }

    fn globals() -> EngineGlobals {
        serde_json::from_value(json!({
            "source": {"file": "level1", "path_id": 64919}, "fuel_consumption": 1.0,
            "oil_consumption": 0.0, "rpm_change": 2000.0, "car_temperature_bounds": [-50.0, 150.0],
            "coolant_evaporate_rate": 0.0, "temperature_difference_modifier": 10.0,
            "temperature_move_modifier": 1.0, "max_car_velocity": 100.0,
            "outside_temperature_modifier": 0.0, "min_steam_temperature": 100.0
        }))
        .unwrap()
    }

    /// Holds the starter for two seconds, then drives with half clutch. Returns the last output.
    fn run(
        items: &mut ItemStore,
        parts: &PowertrainParts,
        car: usize,
    ) -> crate::simulation::engine::EngineOutput {
        let key = ObjectKey::Imported(CARS[car]);
        let globals = globals();
        let mut state = EngineState::new(20.);
        let mut output = Default::default();
        for step in 0..200 {
            let Some(engine) = parts
                .mounted_on(items, &key)
                .engine
                .map(|i| parts.engine(i))
            else {
                return state.stop();
            };
            output = state.step(
                EngineInput {
                    installed: Some(InstalledEngineInput {
                        ignition: true,
                        starter: step < 100,
                        throttle: 1.,
                        clutch: 0.5,
                        electronics_enabled: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                items.engine_fluids([Some(FUEL[car]), engine.oil_tank, None]),
                EngineRecords {
                    engine: &engine.record,
                    radiator: None,
                    globals: &globals,
                },
                |_| 0.,
                0.02,
            );
        }
        output
    }

    #[test]
    fn engine_part_moved_to_another_car_runs_there() {
        let (mut items, parts) = world();
        let keys = CARS.map(ObjectKey::Imported);
        assert!(parts.mounted_on(&items, &keys[0]).engine.is_some());
        assert!(parts.mounted_on(&items, &keys[1]).engine.is_none());
        assert!(run(&mut items, &parts, 0).running);

        items.get_mut(ENGINE).unwrap().location = ItemLocation::PartSlot(SLOTS[1]);
        assert!(parts.mounted_on(&items, &keys[0]).engine.is_none());
        assert_eq!(parts.mounted_on(&items, &keys[1]).engine, Some(0));
        let before = items.get(FUEL[1]).unwrap().fluid.as_ref().unwrap().total();
        let output = run(&mut items, &parts, 1);
        assert!(output.running && output.torque_nm > 0., "{output:?}");
        // The engine burns the fuel of the car it is mounted on.
        assert!(items.get(FUEL[1]).unwrap().fluid.as_ref().unwrap().total() < before);
    }

    #[test]
    fn empty_engine_slot_gives_no_torque() {
        let (mut items, parts) = world();
        items.get_mut(ENGINE).unwrap().location = ItemLocation::World;
        for car in 0..2 {
            let output = run(&mut items, &parts, car);
            assert!(!output.running && !output.starter, "{output:?}");
            assert_eq!((output.rpm, output.torque_nm), (0., 0.));
        }
    }
}
