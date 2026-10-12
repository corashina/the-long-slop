//! Installed Lada engine rules at the fixed gameplay step.

use crate::fluids::{FluidMixture, VehicleFluids, spill, suitability};
use crate::reference::{EngineReference, TorqueCurve};

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

#[derive(Clone, Copy, Debug)]
pub struct EngineInput {
    pub engine_present: bool,
    pub radiator_present: bool,
    pub ignition_held: bool,
    pub ignition_pressed: bool,
    pub ignition_released: bool,
    pub throttle: f32,
    /// Clutch engagement. Zero disconnects the wheels.
    pub clutch: f32,
    pub wheel_rpm: f32,
    pub free_run: bool,
    pub speed_kmh: f32,
    pub ambient_temperature: f32,
    pub ambient_heat_modifier: f32,
    pub sleep_modifier: f32,
    pub radiator_condition: u8,
}

impl Default for EngineInput {
    fn default() -> Self {
        Self {
            engine_present: true,
            radiator_present: true,
            ignition_held: false,
            ignition_pressed: false,
            ignition_released: false,
            throttle: 0.,
            clutch: 0.,
            wheel_rpm: 0.,
            free_run: false,
            speed_kmh: 0.,
            ambient_temperature: 0.,
            ambient_heat_modifier: 0.,
            sleep_modifier: 1.,
            radiator_condition: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EngineOutput {
    pub ignition: bool,
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
    pub ignition: bool,
    pub rpm: f32,
    pub temperature: f32,
    elapsed: f32,
    last_run_time: f32,
    last_start_chance_time: f32,
    random_start: f32,
    ignition_hold_seconds: f32,
    last_ignition_held: bool,
    starter: bool,
    supplied_fuel: f32,
    fan_on: bool,
    last_output: EngineOutput,
}

impl EngineState {
    pub fn new(reference: &EngineReference) -> Self {
        Self {
            ignition: false,
            rpm: 0.0,
            temperature: reference.initial_temperature,
            elapsed: 0.0,
            last_run_time: -1.0,
            last_start_chance_time: 0.0,
            random_start: 1.0,
            ignition_hold_seconds: 0.0,
            last_ignition_held: false,
            starter: false,
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
        random_sample: f32,
        dt: f32,
    ) -> EngineOutput {
        if !dt.is_finite() || dt <= 0.0 {
            return self.last_output;
        }
        self.elapsed += dt;
        if !input.engine_present {
            self.ignition = false;
            self.starter = false;
            self.rpm = 0.;
            self.supplied_fuel = 0.;
            self.last_run_time = -1.;
            self.last_ignition_held = false;
            self.ignition_hold_seconds = 0.;
            self.temperature += temperature_change(
                self.temperature,
                0.,
                reference.heat_loss,
                input,
                reference,
                dt,
            );
            self.last_output = EngineOutput {
                temperature: self.temperature,
                ..EngineOutput::default()
            };
            return self.last_output;
        }
        if input.ignition_pressed {
            self.ignition_hold_seconds = 0.0;
        }
        if input.ignition_held {
            if self.ignition_hold_seconds < reference.starter_delay_seconds {
                self.ignition_hold_seconds += dt;
            } else {
                self.ignition = true;
                self.starter = reference.electronics_enabled;
            }
        } else if input.ignition_released || self.last_ignition_held {
            if self.ignition_hold_seconds < reference.starter_delay_seconds {
                self.ignition = !self.ignition;
                self.ignition_hold_seconds = 0.0;
                self.starter = false;
            }
        } else {
            self.starter = false;
            self.ignition_hold_seconds = 0.0;
        }
        self.last_ignition_held = input.ignition_held;

        let required = FluidMixture {
            quantities_liters: reference.fuel_consumption,
        };
        let fuel_suitability = suitability(fluids.fuel, &required);
        let oil_penalty = inverse_lerp(
            fluids.sump.capacity_liters * 0.3,
            0.0,
            fluids.sump.mixture.amount(reference.oil_fluid as usize),
        )
        .powi(3);
        let bounds = reference.globals.car_temperature_bounds;
        self.temperature = self.temperature.clamp(bounds[0], bounds[1]);
        if reference.no_overheat {
            self.temperature = self.temperature.min(reference.optimal_temperature[2]);
        }
        let heat_penalty = inverse_lerp(
            reference.optimal_temperature[2],
            reference.optimal_temperature[3],
            self.temperature,
        )
        .powi(3);
        let can_run = self.rpm > reference.shutdown_rpm
            && oil_penalty < 0.99
            && heat_penalty < 0.99
            && fuel_suitability > 0.3
            && self.random_start <= reference.start_chance;
        let ignition = self.ignition && reference.electronics_enabled;
        let min_supplied = inverse_lerp(0.0, reference.max_rpm, reference.idle_rpm);
        if ignition && can_run {
            self.supplied_fuel = lerp(min_supplied, 1.0, input.throttle * fuel_suitability);
            self.last_run_time = self.elapsed;
        } else if self.elapsed - self.last_run_time >= RUN_GRACE_SECONDS {
            self.supplied_fuel = 0.0;
        }
        let running = self.elapsed - self.last_run_time < RUN_GRACE_SECONDS;
        let consumed_fuel = if running {
            spill(
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
            )
        } else {
            0.0
        };
        self.supplied_fuel *= fuel_suitability;
        let target_rpm = reference.max_rpm * self.supplied_fuel;
        let mut clutch = input.clutch.clamp(0.0, 1.0);
        if input.free_run
            && (!reference.kickstart || running || !ignition)
            && target_rpm < input.wheel_rpm
        {
            clutch = 0.0;
        }
        if self.starter {
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
                self.random_start = random_sample.clamp(0.0, 1.0);
            }
        }
        let next_rpm = lerp(target_rpm, input.wheel_rpm, clutch);
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
        if self.starter && target_rpm <= reference.starter_rpm {
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
            input.speed_kmh * reference.heat_loss_speed_modifier,
            reference.heat_loss,
            input,
            reference,
            dt,
        );
        if self.rpm > 10.0 {
            spill(
                fluids.sump,
                reference.globals.oil_consumption
                    * reference.oil_consumption_modifier
                    * inverse_lerp(0.0, reference.max_rpm, self.rpm)
                    * dt
                    * input.sleep_modifier,
            );
        }
        if input.radiator_present && self.temperature > reference.globals.min_steam_temperature {
            spill(
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
        if running && input.radiator_present {
            let radiator = &reference.radiator;
            let condition = usize::from(input.radiator_condition.min(4));
            let value_factor = [1.0, 0.75, 0.5, 0.25, 0.125][condition];
            let fan_value = radiator.max_fan_value * [1.0, 0.9, 0.8, 0.7, 0.6][condition];
            let leak = radiator.max_fluid_loss * [0.0, 0.125, 0.25, 0.5, 1.0][condition];
            let mut speed = input.speed_kmh;
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
            spill(
                fluids.coolant,
                leak / 60.0 * dt * thermostat * input.sleep_modifier,
            );
        }
        self.last_output = EngineOutput {
            ignition: self.ignition,
            starter: self.starter,
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
