//! Installed Lada engine rules at the fixed gameplay step.

use crate::fluids::{FluidContainer, FluidMixture, VehicleFluids, suitability};
use crate::reference::{EngineReference, TorqueCurve};
use crate::save::{EngineSnapshot, SaveError, identity::unsigned};

const RUN_GRACE_SECONDS: f32 = 0.2;

fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        0.0
    } else {
        ((value - a) / (b - a)).clamp(0.0, 1.0)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// The installed Lada curve has unweighted keys and clamps at both ends.
pub(crate) fn torque(curve: &TorqueCurve, rpm: f32) -> f32 {
    let keys = &curve.keys;
    if rpm <= keys[0].time {
        return keys[0].value;
    }
    for pair in keys.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if rpm <= b.time {
            let width = b.time - a.time;
            let t = (rpm - a.time) / width;
            let t2 = t * t;
            let t3 = t2 * t;
            return (2.0 * t3 - 3.0 * t2 + 1.0) * a.value
                + (t3 - 2.0 * t2 + t) * width * a.out_tangent
                + (-2.0 * t3 + 3.0 * t2) * b.value
                + (t3 - t2) * width * b.in_tangent;
        }
    }
    keys.last().unwrap().value
}

fn temperature_change(
    temperature: f32,
    speed: f32,
    transfer: f32,
    input: EngineInput,
    reference: &EngineReference,
    dt: f32,
) -> f32 {
    let globals = &reference.globals;
    let difference = ((input.ambient_temperature - temperature)
        / globals.temperature_difference_modifier)
        .abs()
        .clamp(1.0, 1000.0);
    let direction = if input.ambient_temperature > temperature {
        1.0
    } else {
        -1.0
    };
    direction
        * dt
        * (1.0
            + globals.temperature_move_modifier
                * inverse_lerp(0.0, globals.max_car_velocity, speed))
        * (transfer / 60.0)
        * difference
        * input.sleep_modifier
}

// Source ChangeWithoutMix is sequential. Tank.Upd removes one depleted entry.
// Coolant changes do not call Tank.Upd. Keep their retained zero entries.
fn drain(container: &mut FluidContainer, liters: f32) -> f32 {
    let quantities = container.mixture.quantities_liters;
    let before = container.mixture.total();
    if before > 0.0 && liters > 0.0 {
        container.change_without_mix(-liters);
        // Finite-state correction: overdraw must not publish fluid creation.
        for (amount, prior) in container
            .mixture
            .quantities_liters
            .iter_mut()
            .zip(quantities)
        {
            *amount = amount.max(0.0).min(prior.max(0.0));
        }
    }
    before - container.mixture.total()
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InstalledEngineInput {
    pub ignition: bool,
    pub starter: bool,
    pub throttle: f32,
    /// Clutch engagement. Zero disconnects the wheels.
    pub clutch: f32,
    pub wheel_rpm: f32,
    pub free_run: bool,
    pub speed_kmh: f32,
    pub radiator_condition: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct EngineInput {
    pub installed: Option<InstalledEngineInput>,
    pub ambient_temperature: f32,
    pub ambient_heat_modifier: f32,
    pub sleep_modifier: f32,
}

impl Default for EngineInput {
    fn default() -> Self {
        Self {
            installed: None,
            ambient_temperature: 0.,
            ambient_heat_modifier: 0.,
            sleep_modifier: 1.,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EngineOutput {
    pub starter: bool,
    pub running: bool,
    pub rpm: f32,
    pub target_rpm: f32,
    pub torque_nm: f32,
    pub brake_torque_nm: f32,
    pub fuel_suitability: f32,
    pub oil_penalty: f32,
    pub heat_penalty: f32,
    pub consumed_fuel_liters: f32,
    pub temperature: f32,
}

#[derive(Clone, Debug)]
pub struct EngineState {
    pub rpm: f32,
    pub temperature: f32,
    elapsed: f32,
    last_run_time: f32,
    last_start_chance_time: f32,
    random_start: f32,
    oil_penalty: f32,
    fuel_suitability: f32,
    heat_penalty: f32,
    start_draw: u64,
    supplied_fuel: f32,
    fan_on: bool,
    last_output: EngineOutput,
}

impl EngineState {
    pub fn clear_input(&mut self) {
        self.last_output.starter = false;
    }
    pub fn running(&self) -> bool {
        self.elapsed - self.last_run_time < RUN_GRACE_SECONDS
    }
    pub fn capture_save(&self) -> crate::save::EngineSnapshot {
        crate::save::EngineSnapshot {
            rpm: self.rpm,
            temperature: self.temperature,
            elapsed: self.elapsed,
            last_run_time: self.last_run_time,
            last_start_chance_time: self.last_start_chance_time,
            random_start: self.random_start,
            supplied_fuel: self.supplied_fuel,
            fan_on: self.fan_on,
            oil_penalty: self.oil_penalty,
            fuel_suitability: self.fuel_suitability,
            heat_penalty: self.heat_penalty,
            start_draw: self.start_draw.to_string(),
        }
    }
    pub fn restore_save(&mut self, saved: &EngineSnapshot) -> Result<EngineOutput, SaveError> {
        let start_draw = unsigned(&saved.start_draw)?;
        self.rpm = saved.rpm;
        self.temperature = saved.temperature;
        self.elapsed = saved.elapsed;
        self.last_run_time = saved.last_run_time;
        self.last_start_chance_time = saved.last_start_chance_time;
        self.random_start = saved.random_start;
        self.supplied_fuel = saved.supplied_fuel;
        self.fan_on = saved.fan_on;
        self.oil_penalty = saved.oil_penalty;
        self.fuel_suitability = saved.fuel_suitability;
        self.heat_penalty = saved.heat_penalty;
        self.start_draw = start_draw;
        self.clear_input();
        self.last_output = EngineOutput {
            running: self.running(),
            rpm: self.rpm,
            temperature: self.temperature,
            oil_penalty: self.oil_penalty,
            fuel_suitability: self.fuel_suitability,
            heat_penalty: self.heat_penalty,
            ..Default::default()
        };
        Ok(self.last_output)
    }
    pub fn new(reference: &EngineReference) -> Self {
        Self {
            rpm: 0.0,
            temperature: reference.initial_temperature,
            elapsed: 0.0,
            last_run_time: -1.0,
            last_start_chance_time: 0.0,
            random_start: 1.0,
            oil_penalty: 0.,
            fuel_suitability: 0.,
            heat_penalty: 0.,
            start_draw: 0,
            supplied_fuel: 0.0,
            fan_on: false,
            last_output: EngineOutput::default(),
        }
    }

    pub fn step(
        &mut self,
        input: EngineInput,
        fluids: VehicleFluids<'_>,
        reference: &EngineReference,
        mut start_sample: impl FnMut(u64) -> f32,
        dt: f32,
    ) -> EngineOutput {
        if !dt.is_finite() || dt <= 0.0 {
            return self.last_output;
        }
        self.elapsed += dt;
        let installed = input.installed.unwrap_or_default();
        let required = FluidMixture {
            quantities_liters: reference.fuel_consumption,
        };
        if input.installed.is_some() {
            self.oil_penalty = inverse_lerp(
                fluids.sump.capacity_liters * 0.3,
                0.0,
                fluids.sump.mixture.amount(reference.oil_fluid as usize),
            )
            .powi(3);
        }
        let oil_penalty = self.oil_penalty;
        let can_run = self.rpm > reference.shutdown_rpm
            && oil_penalty < 0.99
            && self.heat_penalty < 0.99
            && self.fuel_suitability > 0.3
            && self.random_start <= reference.start_chance;
        let ignition = installed.ignition && reference.electronics_enabled;
        let min_supplied = inverse_lerp(0.0, reference.max_rpm, reference.idle_rpm);
        if ignition && can_run {
            self.supplied_fuel = lerp(
                min_supplied,
                1.0,
                installed.throttle * self.fuel_suitability,
            );
            self.last_run_time = self.elapsed;
        } else if self.elapsed - self.last_run_time >= RUN_GRACE_SECONDS {
            self.supplied_fuel = 0.0;
        }
        let running = self.running();
        self.fuel_suitability = suitability(fluids.fuel, &required);
        let fuel_suitability = self.fuel_suitability;
        let consumed_fuel = if running {
            let removed = drain(
                fluids.fuel,
                self.supplied_fuel
                    * dt
                    * input.sleep_modifier
                    * reference.globals.fuel_consumption
                    * lerp(
                        required.total() * 0.2,
                        required.total(),
                        inverse_lerp(0.0, reference.max_rpm, self.rpm),
                    ),
            );
            // Source Fuel.Upd also runs when supplied fuel is zero.
            fluids.fuel.cleanup_one();
            removed
        } else {
            0.0
        };
        self.supplied_fuel *= fuel_suitability;
        let target_rpm = reference.max_rpm * self.supplied_fuel;
        let mut clutch = installed.clutch.clamp(0.0, 1.0);
        if installed.free_run
            && (!reference.kickstart || running || !ignition)
            && target_rpm < installed.wheel_rpm
        {
            clutch = 0.0;
        }
        if installed.starter {
            if self.rpm < reference.starter_rpm {
                self.rpm = lerp(
                    self.rpm,
                    reference.starter_rpm,
                    reference.starter_rpm_lerp * dt,
                );
            }
            if !running
                && self.elapsed - self.last_start_chance_time
                    > reference.start_chance_interval_seconds * reference.engine_start_time_modifier
            {
                self.last_start_chance_time = self.elapsed;
                self.random_start = start_sample(self.start_draw).clamp(0.0, 1.0);
                self.start_draw = self.start_draw.wrapping_add(1);
            }
        }
        let next_rpm = lerp(target_rpm, installed.wheel_rpm, clutch);
        self.rpm += (next_rpm - self.rpm)
            .clamp(-reference.globals.rpm_change, reference.globals.rpm_change)
            * reference.rpm_change_modifier
            * dt;
        let mut rpm_difference = self.rpm - target_rpm;
        let curve_output = torque(&reference.torque_curve, self.rpm);
        let mut actual_output = if target_rpm > self.rpm && running {
            curve_output
        } else {
            0.0
        };
        if installed.starter && target_rpm <= reference.starter_rpm {
            rpm_difference =
                (self.rpm - reference.starter_rpm) * reference.starter_rpm_lerp * 100.0;
            actual_output =
                (curve_output * reference.starter_rpm_lerp).min(reference.max_torque_nm * 0.5);
        }
        let brake_output = reference.max_torque_nm
            * reference.engine_brake_modifier
            * if self.rpm < 0.0 {
                1.0
            } else {
                inverse_lerp(0.0, reference.max_rpm_difference, rpm_difference)
            };
        let bounds = reference.globals.car_temperature_bounds;
        self.temperature = self.temperature.clamp(bounds[0], bounds[1]);
        if reference.no_overheat {
            self.temperature = self.temperature.min(reference.optimal_temperature[2]);
        }
        self.heat_penalty = inverse_lerp(
            reference.optimal_temperature[2],
            reference.optimal_temperature[3],
            self.temperature,
        )
        .powi(3);
        let heat_penalty = self.heat_penalty;
        actual_output *= (1.0 - heat_penalty) * (1.0 - oil_penalty);
        if running {
            self.temperature += dt
                * (lerp(
                    reference.heat_gain_min,
                    reference.heat_gain_max,
                    inverse_lerp(reference.idle_rpm, reference.max_rpm, self.rpm),
                ) + input.ambient_heat_modifier)
                / 60.0;
        }
        self.temperature += temperature_change(
            self.temperature,
            installed.speed_kmh * reference.heat_loss_speed_modifier,
            reference.heat_loss,
            input,
            reference,
            dt,
        );
        if input.installed.is_some() && self.rpm > 10.0 && fluids.sump.mixture.total() > 0.0 {
            drain(
                fluids.sump,
                reference.globals.oil_consumption
                    * reference.oil_consumption_modifier
                    * inverse_lerp(0.0, reference.max_rpm, self.rpm)
                    * dt
                    * input.sleep_modifier,
            );
            fluids.sump.cleanup_one();
        }
        if installed.radiator_condition.is_some()
            && self.temperature > reference.globals.min_steam_temperature
        {
            drain(
                fluids.coolant,
                reference.globals.coolant_evaporate_rate / 60.0
                    * dt
                    * input.sleep_modifier
                    * inverse_lerp(
                        reference.globals.min_steam_temperature,
                        bounds[1],
                        self.temperature,
                    ),
            );
        }
        if running && let Some(condition) = installed.radiator_condition {
            let radiator = &reference.radiator;
            let condition = usize::from(condition.min(4));
            let value_factor = [1.0, 0.75, 0.5, 0.25, 0.125][condition];
            let fan_value = radiator.max_fan_value * [1.0, 0.9, 0.8, 0.7, 0.6][condition];
            let leak = radiator.max_fluid_loss * [0.0, 0.125, 0.25, 0.5, 1.0][condition];
            let mut speed = installed.speed_kmh;
            if self.temperature > radiator.fan_temperature_on || radiator.fan_on_when_engine_runs {
                self.fan_on = true;
                speed = (speed + fan_value).clamp(fan_value, reference.globals.max_car_velocity);
            } else if self.temperature < radiator.fan_temperature_off {
                self.fan_on = false;
            }
            let thermostat = inverse_lerp(
                radiator.thermostat_range[0],
                radiator.thermostat_range[1],
                self.temperature,
            );
            let total = fluids.coolant.mixture.total();
            let water_blood = if total > 0.0 {
                fluids
                    .coolant
                    .mixture
                    .amount(3)
                    .max(fluids.coolant.mixture.amount(4))
                    / total
            } else {
                0.0
            };
            let cooling = radiator.max_value
                * value_factor
                * thermostat
                * inverse_lerp(0.0, fluids.coolant.capacity_liters * 0.5, total)
                * water_blood.clamp(0.5, 1.0);
            self.temperature +=
                temperature_change(self.temperature, speed, cooling, input, reference, dt);
            drain(
                fluids.coolant,
                leak / 60.0 * dt * thermostat * input.sleep_modifier,
            );
        }
        self.last_output = EngineOutput {
            starter: installed.starter,
            running,
            rpm: self.rpm,
            target_rpm,
            torque_nm: actual_output.max(0.0) * reference.torque_modifier * clutch,
            brake_torque_nm: brake_output.max(0.0) * reference.torque_modifier * clutch,
            fuel_suitability,
            oil_penalty,
            heat_penalty,
            consumed_fuel_liters: consumed_fuel,
            temperature: self.temperature,
        };
        self.last_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::ReferenceManifest;

    #[test]
    #[ignore = "requires the installed build 11140890 export"]
    fn zero_fuel_drain_keeps_source_cleanup() {
        let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
        let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
        assert_eq!(reference.build_id, 11140890);
        assert_eq!(reference.vehicles.len(), 1);
        let source = &reference.vehicles[0].engine;
        assert_eq!(source.source.file, "level1");
        assert_eq!(source.source.path_id, 68282);
        let mut engine = EngineState::new(source);
        let mut state = engine.capture_save();
        state.elapsed = 1.;
        state.last_run_time = 1.;
        engine.restore_save(&state).unwrap();
        let container =
            |id| FluidContainer::from(reference.tanks.iter().find(|tank| tank.item == id).unwrap());
        let mut fuel = container(source.fuel_tank);
        let mut sump = container(source.oil_tank);
        let mut coolant = container(source.coolant_tank);
        fuel.restore_contents([1., 0., 0., 0., 0., 0.], &[3, 0]);
        let output = engine.step(
            EngineInput::default(),
            VehicleFluids {
                fuel: &mut fuel,
                sump: &mut sump,
                coolant: &mut coolant,
            },
            source,
            |_| panic!("no starter"),
            0.02,
        );
        assert_eq!(output.consumed_fuel_liters, 0.);
        assert_eq!(fuel.retained_order(), &[0]);
        fuel.restore_contents([0.; 6], &[0, 3]);
        sump.restore_contents([0.; 6], &[0, 1]);
        coolant.restore_contents([0.; 6], &[3, 4]);
        engine.rpm = 2000.;
        engine.step(
            EngineInput {
                installed: Some(InstalledEngineInput {
                    radiator_condition: Some(4),
                    ..Default::default()
                }),
                ..Default::default()
            },
            VehicleFluids {
                fuel: &mut fuel,
                sump: &mut sump,
                coolant: &mut coolant,
            },
            source,
            |_| panic!("no starter"),
            0.02,
        );
        assert_eq!(fuel.retained_order(), &[3]);
        assert_eq!(sump.retained_order(), &[0, 1]);
        assert_eq!(coolant.retained_order(), &[3, 4]);
    }
}
