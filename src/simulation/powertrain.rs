//! Engine and radiator parts. Part state lives on the part. Each car copy reads the parts mounted on it.
use std::collections::BTreeMap;

use crate::interaction::items::ItemStore;
use crate::reference::{
    EnginePartReference, EngineReference, ItemId, RadiatorPartReference, ReferenceManifest,
};
use crate::simulation::engine::EngineState;
use crate::vehicle::garage_vehicle_index;
use crate::world::ObjectKey;

/// An engine part item and its source `enginescript` record and state.
#[derive(Clone, Debug)]
pub struct EnginePart {
    pub item: ItemId,
    pub record: EnginePartReference,
    /// Item of `enginescript.T`.
    pub oil_tank: Option<ItemId>,
    /// Source `enginescript` runtime fields. They move with the part.
    pub state: EngineState,
}

impl EnginePart {
    pub fn new(item: ItemId, record: EnginePartReference, oil_tank: Option<ItemId>) -> Self {
        let state = EngineState::new(record.initial_temperature);
        Self {
            item,
            record,
            oil_tank,
            state,
        }
    }
}

/// A radiator part item and its source `coolantTankscript` record and state.
#[derive(Clone, Debug)]
pub struct RadiatorPart {
    pub item: ItemId,
    pub record: RadiatorPartReference,
    pub coolant_tank: Option<ItemId>,
    /// Source `coolantTankscript` fan hysteresis state.
    pub fan_on: bool,
}

impl RadiatorPart {
    pub fn new(item: ItemId, record: RadiatorPartReference, coolant_tank: Option<ItemId>) -> Self {
        Self {
            item,
            record,
            coolant_tank,
            fan_on: false,
        }
    }
}

/// Items of the parts mounted on one car copy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Powertrain {
    pub engine: Option<ItemId>,
    pub radiator: Option<ItemId>,
}

/// Every live engine and radiator part by item.
#[derive(Default)]
pub struct PowertrainParts {
    engines: BTreeMap<ItemId, EnginePart>,
    radiators: BTreeMap<ItemId, RadiatorPart>,
    /// V03 bridge: engine and radiator items of each old garage car. Task 9 removes it.
    garage: Vec<(ItemId, ItemId)>,
}

impl PowertrainParts {
    /// V03 bridge: the engine and radiator of each old garage car. Task 9 removes it.
    /// The old coolant tank item is the radiator item, as in the old engine rules.
    pub fn from_garage(reference: &ReferenceManifest) -> Self {
        let mut parts = Self::default();
        for vehicle in &reference.vehicles {
            let e = &vehicle.engine;
            parts.garage.push((e.item, e.coolant_tank));
            parts.register_engine(EnginePart::new(e.item, garage_engine(e), Some(e.oil_tank)));
            parts.register_radiator(RadiatorPart::new(
                e.coolant_tank,
                garage_radiator(e),
                Some(e.coolant_tank),
            ));
        }
        parts
    }
    pub fn register_engine(&mut self, part: EnginePart) {
        self.engines.insert(part.item, part);
    }
    pub fn register_radiator(&mut self, part: RadiatorPart) {
        self.radiators.insert(part.item, part);
    }
    /// Removes the engine or radiator of a retired item.
    pub fn remove(&mut self, item: ItemId) {
        self.engines.remove(&item);
        self.radiators.remove(&item);
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
    /// The first engine and radiator by item for which `on_car` is true.
    fn mounted(&self, on_car: impl Fn(ItemId) -> bool) -> Powertrain {
        Powertrain {
            engine: self.engines.keys().copied().find(|item| on_car(*item)),
            radiator: self.radiators.keys().copied().find(|item| on_car(*item)),
        }
    }
    pub fn engine(&self, item: ItemId) -> Option<&EnginePart> {
        self.engines.get(&item)
    }
    pub fn engine_mut(&mut self, item: ItemId) -> Option<&mut EnginePart> {
        self.engines.get_mut(&item)
    }
    pub fn engines_mut(&mut self) -> impl Iterator<Item = &mut EnginePart> {
        self.engines.values_mut()
    }
    pub fn radiator(&self, item: ItemId) -> Option<&RadiatorPart> {
        self.radiators.get(&item)
    }
    pub fn radiator_mut(&mut self, item: ItemId) -> Option<&mut RadiatorPart> {
        self.radiators.get_mut(&item)
    }
    /// V03 bridge: engine and radiator items of old garage car `index`. Task 9 removes it.
    pub fn garage_parts(&self, index: usize) -> (ItemId, ItemId) {
        self.garage[index]
    }
    /// The engine and the radiator of one car, both writable.
    pub fn pair_mut(
        &mut self,
        powertrain: Powertrain,
    ) -> (Option<&mut EnginePart>, Option<&mut RadiatorPart>) {
        (
            powertrain
                .engine
                .and_then(|item| self.engines.get_mut(&item)),
            powertrain
                .radiator
                .and_then(|item| self.radiators.get_mut(&item)),
        )
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
        EngineInput, EngineOutput, EngineRecords, InstalledEngineInput,
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
        parts.register_engine(EnginePart::new(ENGINE, record, Some(OIL)));
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

    /// One engine step on car `car`. No mounted engine: no output.
    fn step(
        items: &mut ItemStore,
        parts: &mut PowertrainParts,
        car: usize,
        starter: bool,
        dt: f32,
    ) -> EngineOutput {
        let key = ObjectKey::Imported(CARS[car]);
        let Some(item) = parts.mounted_on(items, &key).engine else {
            return EngineOutput::default();
        };
        let part = parts.engine_mut(item).unwrap();
        part.state.step(
            EngineInput {
                installed: Some(InstalledEngineInput {
                    ignition: true,
                    starter,
                    throttle: 1.,
                    clutch: 0.5,
                    electronics_enabled: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            items.engine_fluids([Some(FUEL[car]), part.oil_tank, None]),
            EngineRecords {
                engine: &part.record,
                radiator: None,
                globals: &globals(),
            },
            None,
            |_| 0.,
            dt,
        )
    }

    /// Holds the starter for two seconds, then drives with half clutch. Returns the last output.
    fn run(items: &mut ItemStore, parts: &mut PowertrainParts, car: usize) -> EngineOutput {
        let mut output = EngineOutput::default();
        for n in 0..200 {
            output = step(items, parts, car, n < 100, 0.02);
        }
        output
    }

    #[test]
    fn engine_part_moved_to_another_car_runs_there() {
        let (mut items, mut parts) = world();
        let keys = CARS.map(ObjectKey::Imported);
        assert!(parts.mounted_on(&items, &keys[0]).engine.is_some());
        assert!(parts.mounted_on(&items, &keys[1]).engine.is_none());
        assert!(run(&mut items, &mut parts, 0).running);

        items.get_mut(ENGINE).unwrap().location = ItemLocation::PartSlot(SLOTS[1]);
        assert!(parts.mounted_on(&items, &keys[0]).engine.is_none());
        assert_eq!(parts.mounted_on(&items, &keys[1]).engine, Some(ENGINE));
        let before = items.get(FUEL[1]).unwrap().fluid.as_ref().unwrap().total();
        let output = run(&mut items, &mut parts, 1);
        assert!(output.running && output.torque_nm > 0., "{output:?}");
        // The engine burns the fuel of the car it is mounted on.
        assert!(items.get(FUEL[1]).unwrap().fluid.as_ref().unwrap().total() < before);
    }

    #[test]
    fn engine_temperature_moves_with_the_part() {
        let (mut items, mut parts) = world();
        let keys = CARS.map(ObjectKey::Imported);
        run(&mut items, &mut parts, 0);
        let temperature = parts.engine(ENGINE).unwrap().state.temperature;
        // The part left its initial temperature on car A.
        assert_ne!(temperature, 20.);

        items.get_mut(ENGINE).unwrap().location = ItemLocation::PartSlot(SLOTS[1]);
        // Car A has no engine state. Car B reads the moved part state.
        assert_eq!(parts.mounted_on(&items, &keys[0]), Powertrain::default());
        let mounted = parts.mounted_on(&items, &keys[1]).engine.unwrap();
        assert_eq!(
            parts.engine(mounted).unwrap().state.temperature,
            temperature
        );
        let output = step(&mut items, &mut parts, 1, false, 1e-4);
        assert!(
            (output.temperature - temperature).abs() < 0.01,
            "{output:?}"
        );
        assert_eq!(step(&mut items, &mut parts, 0, false, 0.02).temperature, 0.);
    }

    #[test]
    fn empty_engine_slot_gives_no_torque() {
        let (mut items, mut parts) = world();
        items.get_mut(ENGINE).unwrap().location = ItemLocation::World;
        for car in 0..2 {
            let output = run(&mut items, &mut parts, car);
            assert!(!output.running && !output.starter, "{output:?}");
            assert_eq!((output.rpm, output.torque_nm), (0., 0.));
        }
    }
}
