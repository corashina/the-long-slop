//! Vehicle records of car world prototypes and engine and radiator records of part nodes.
//! Every `u32` id is local to its world prototype: node, body or collider index.
use super::*;

/// Source `carscript` of a car world prototype root.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldVehicleReference {
    pub source: SourceId,
    /// Chassis body: the root Rigidbody (`carscript.RB`).
    pub body: u32,
    pub car_type: i32,
    /// Enabled chassis colliders.
    pub colliders: Vec<u32>,
    /// Source `massScript` masses and initial fluids of active nodes.
    pub mass_kg: f32,
    pub serialized_mass_kg: f32,
    /// Root `tankscript` node.
    pub fuel_tank: Option<u32>,
    /// `carscript.electronics` node.
    pub electronics: Option<u32>,
    /// Source `carscript.startingTime`.
    pub starter_delay_seconds: f32,
    pub driving: DrivingVehicleReference,
    pub handbrake: bool,
    pub hub: WheelHubReference,
    pub wheels: Vec<WorldWheelReference>,
    pub gear_ratios: Vec<f32>,
    pub gear_free_run: Vec<bool>,
    pub final_drive_ratio: f32,
    pub max_steer_degrees: f32,
    pub brake_torque_nm: f32,
    pub handbrake_torque_nm: f32,
    pub wheel_slip: [f32; 4],
    pub rim_slip: [f32; 2],
    pub auto_shift_stop_speed: f32,
    pub auto_shift_min_rpm: f32,
    pub auto_shift_max_rpm: f32,
    pub dont_ignite_with_throttle_in_automatic: bool,
    pub auto_move_min: f32,
    pub collision_gain: f32,
    /// Source `carscript.psSmoke` systems, posed relative to the prototype root.
    pub exhaust: Vec<ParticleReference>,
    pub seats: Vec<WorldSeatReference>,
    pub lights: Vec<WorldLightReference>,
    pub light_switch: Option<WorldLightSwitchReference>,
    /// Node ids are prototype node indices.
    pub instruments: Option<InstrumentReference>,
    /// Indices of `headlight2` lamps in `instruments.lamps`.
    pub high_beams: Vec<u32>,
}

/// A road WheelCollider of `carscript` and its wheel slot.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldWheelReference {
    pub source: SourceId,
    pub node: u32,
    pub collider: WheelColliderReference,
    pub slot: u32,
    pub mount: u32,
    pub steer: bool,
    pub drive: bool,
    pub brake: bool,
    pub handbrake: bool,
    pub sideways_stiffness_modifier: f32,
}

/// A primary `seatscript`. `hit_nodes` holds its node and the nodes of seats that name it `mainseat`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSeatReference {
    pub source: SourceId,
    pub node: u32,
    pub sit_node: u32,
    pub hit_nodes: Vec<u32>,
    pub driver: bool,
    pub driving2: bool,
    pub yaw_limits_degrees: [f32; 2],
    pub min_exit_angle_degrees: f32,
    pub exit_height: f32,
    pub show_sleep_text: bool,
    pub default_pitch_degrees: f32,
    pub pitch_limits_degrees: [f32; 2],
}

/// A `headlightscript` (headlight or cabin) or `rearlights` controller.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldLightReference {
    pub source: SourceId,
    pub kind: LightControllerKind,
    pub node: u32,
    pub electronics: Option<u32>,
    pub startup_on: bool,
    pub current_state: usize,
    pub requested_state: usize,
    pub energy: f32,
    pub energy_move: f32,
    pub enabled: bool,
    pub reverse_switch: bool,
    pub states: Vec<WorldLampStateReference>,
    pub cabin: Option<WorldCabinInputReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldLampStateReference {
    pub signal: LampSignal,
    /// Nodes of the source Light components.
    pub lights: Vec<u32>,
    pub intensity: f32,
    pub renderers: Vec<u32>,
    pub on_material: Option<MaterialLink>,
    pub off_material: Option<MaterialLink>,
    pub broken: bool,
    pub working_nodes: Vec<u32>,
    pub broken_nodes: Vec<u32>,
    pub dont_overwrite: bool,
}

/// Source `inputconnect` that routes a cabin lamp.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCabinInputReference {
    pub source: SourceId,
    pub switch_node: u32,
    pub modes: Vec<u8>,
    pub startup_mode: usize,
    pub door_slots: Vec<u32>,
    pub threshold: f32,
    pub include_children: bool,
    pub missing_slot_is_same: bool,
}

/// The `usablescript` that calls `carscript.InteractSwitchLights`, and the switch pose nodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldLightSwitchReference {
    pub usable: u32,
    pub nodes: Vec<u32>,
    /// `LightOff`, `LightLow`, `LightHigh` nodes.
    pub state_nodes: [Vec<u32>; 3],
}

/// Source `enginescript` of a part node.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnginePartReference {
    pub source: SourceId,
    pub node: u32,
    /// `enginescript.T` oil tank node.
    pub oil_tank: Option<u32>,
    pub start_chance: f32,
    pub starter_rpm: f32,
    pub starter_rpm_lerp: f32,
    pub idle_rpm: f32,
    pub max_rpm: f32,
    pub shutdown_rpm: f32,
    pub max_torque_nm: f32,
    pub starting_push_force: f32,
    pub consumption_modifier: f32,
    pub oil_consumption_modifier: f32,
    pub engine_brake_modifier: f32,
    pub rpm_change_modifier: f32,
    pub engine_start_time_modifier: f32,
    pub heat_gain_min: f32,
    pub heat_gain_max: f32,
    pub heat_loss: f32,
    pub initial_temperature: f32,
    pub heat_loss_speed_modifier: f32,
    pub optimal_temperature: [f32; 4],
    /// Source `enginescript.FuelConsumption`, by fluid type.
    pub fuel_consumption: [f32; 6],
    pub two_stroke: bool,
    pub torque_curve: TorqueCurve,
    pub max_rpm_difference: f32,
    pub torque_modifier: f32,
    pub oil_fluid: u8,
    pub no_overheat: bool,
    pub kickstart: bool,
    pub start_chance_interval_seconds: f32,
    pub auto_shift_clutch_rpm: f32,
    pub auto_shift_clutch_rpm_range: f32,
    pub smoke_rate: f32,
}

/// Source `coolantTankscript` of a part node.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadiatorPartReference {
    pub source: SourceId,
    pub node: u32,
    pub coolant_tank: Option<u32>,
    pub fan_node: Option<u32>,
    pub thermostat_range: [f32; 2],
    pub fan_temperature_on: f32,
    pub fan_temperature_off: f32,
    pub fan_on_when_engine_runs: bool,
    pub max_value: f32,
    pub max_fan_value: f32,
    pub max_fan_speed: f32,
    pub max_fluid_loss: f32,
}
