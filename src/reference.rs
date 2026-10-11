//! Validated installed-build data. Geometry uses converted body-local meters.

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

pub mod instruments;
pub mod ui;
pub use instruments::*;
pub use ui::*;
mod validation;
pub mod world;
pub use validation::ReferenceError;
pub(crate) use validation::{finite, hash, ids, require};

pub const APP_ID: u32 = 1_017_180;
pub const BUILD_ID: u32 = 11_140_890;

macro_rules! id {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub $inner);
    };
}
id!(ItemId, u32);
id!(AssetId, u32);
id!(NodeId, u32);
id!(BodyId, u32);
id!(ColliderId, u32);
id!(AnchorId, u32);
id!(SeatId, u32);
id!(OpeningId, u32);
id!(WheelSlotId, u8);

pub mod audio;
pub use audio::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceId {
    pub file: String,
    pub path_id: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceHash {
    pub file: String,
    pub sha256: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRecord {
    pub material_count: u32,
    pub id: AssetId,
    pub path: String,
    pub sha256: String,
    pub root: SourceId,
    pub dependencies: Vec<SourceHash>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeRecord {
    pub id: NodeId,
    pub source: SourceId,
    pub name: String,
    pub hierarchy_path: String,
    pub asset: AssetId,
    pub gltf_node: u32,
    pub parent: Option<NodeId>,
    pub active: bool,
    pub transform: Transform,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyRecord {
    pub id: BodyId,
    pub source: SourceId,
    pub node: NodeId,
    pub dynamic: bool,
    pub mass_kg: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub pose: Transform,
    pub parent_body: Option<BodyId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ColliderGeometry {
    Box {
        center: [f32; 3],
        half_extents: [f32; 3],
        rotation: [f32; 4],
    },
    Sphere {
        center: [f32; 3],
        radius: f32,
    },
    Capsule {
        a: [f32; 3],
        b: [f32; 3],
        radius: f32,
    },
    Convex {
        points: Vec<[f32; 3]>,
    },
    Trimesh {
        points: Vec<[f32; 3]>,
        triangles: Vec<[u32; 3]>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColliderRecord {
    pub id: ColliderId,
    pub source: SourceId,
    pub node: NodeId,
    pub body: BodyId,
    pub sensor: bool,
    pub enabled: bool,
    pub membership: u32,
    pub filter: u32,
    pub friction: f32,
    pub restitution: f32,
    pub geometry: ColliderGeometry,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorRecord {
    pub id: AnchorId,
    pub source: SourceId,
    pub node: NodeId,
    pub body: BodyId,
    pub transform: Transform,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HingeRecord {
    pub item: ItemId,
    pub source: SourceId,
    pub anchor: AnchorId,
    pub body: BodyId,
    pub axis: [f32; 3],
    pub closed_degrees: f32,
    pub min_degrees: f32,
    pub max_degrees: f32,
    pub drag_axis: [f32; 3],
    pub drag_input_axis: u8,
    pub drag_sensitivity: f32,
    pub input_inverted: [bool; 2],
    pub rotation_axes: [[f32; 3]; 2],
    pub relative_drag: bool,
    pub drag_anchor: AnchorId,
    pub motor_degrees_per_second: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Decoration,
    Door,
    Hood,
    Trunk,
    Container,
    Cap,
    Engine,
    Radiator,
    Tire,
    Rim,
    Wheel,
    Seat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRecord {
    pub id: ItemId,
    pub source: SourceId,
    pub node: NodeId,
    pub body: BodyId,
    pub name: String,
    pub kind: ItemKind,
    /// Source condition state: zero is new, four is rusty.
    pub condition: u8,
    /// Crash lerp a new part needs to fall off. `None` keeps the part on.
    pub fall_off_power: Option<f32>,
    pub mass_kg: f32,
    pub attached_to: Option<ItemId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpeningRecord {
    pub id: OpeningId,
    pub source: SourceId,
    pub tank: ItemId,
    pub cap: ItemId,
    pub anchor: AnchorId,
    pub flow_liters_per_second: f32,
    pub tilt_degrees: f32,
    pub tilt_axis: [f32; 3],
    pub angle_disabled: bool,
    pub spill_disabled: bool,
    pub tube_connected: bool,
    pub flow_radius: f32,
    pub can_toggle: bool,
    pub valve: f32,
    pub can_tune: bool,
    pub tune_rate: f32,
    pub visuals: CapVisualReference,
    pub presentation: FluidPresentation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapVisualReference {
    pub open_nodes: Vec<NodeId>,
    pub closed_nodes: Vec<NodeId>,
    pub cap_node: Option<NodeId>,
    pub closed_node: Option<NodeId>,
    pub open_node: Option<NodeId>,
    pub euler_rotation: bool,
    pub closed_euler_degrees: [f32; 3],
    pub open_euler_degrees: [f32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TankReference {
    pub item: ItemId,
    pub source: SourceId,
    pub capacity_liters: f32,
    /// Gasoline, oil, alcohol, water, blood, diesel.
    pub quantities_liters: [f32; 6],
    pub openings: Vec<OpeningId>,
    pub no_fill_this: bool,
    pub no_fill_from: bool,
    pub slow_oil: bool,
    pub role: TankRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TankRole {
    Vehicle,
    Portable,
    Supply,
    Fixed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurveKey {
    pub time: f32,
    pub value: f32,
    pub in_tangent: f32,
    pub out_tangent: f32,
    pub in_weight: f32,
    pub out_weight: f32,
    pub weighted_mode: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TorqueCurve {
    pub keys: Vec<CurveKey>,
    pub pre_wrap_mode: i32,
    pub post_wrap_mode: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineReference {
    pub source: SourceId,
    pub item: ItemId,
    pub fuel_tank: ItemId,
    pub oil_tank: ItemId,
    pub coolant_tank: ItemId,
    pub start_chance: f32,
    pub starter_delay_seconds: f32,
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
    pub optimal_temperature: [f32; 4],
    pub fuel_consumption: [f32; 6],
    pub two_stroke: bool,
    pub electronics_enabled: bool,
    pub torque_curve: TorqueCurve,
    pub globals: EngineGlobals,
    pub radiator: RadiatorReference,
    pub initial_temperature: f32,
    pub heat_loss_speed_modifier: f32,
    pub max_rpm_difference: f32,
    pub torque_modifier: f32,
    pub oil_fluid: u8,
    pub no_overheat: bool,
    pub kickstart: bool,
    pub start_chance_interval_seconds: f32,
    pub auto_shift_clutch_rpm: f32,
    pub auto_shift_clutch_rpm_range: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineGlobals {
    pub source: SourceId,
    pub fuel_consumption: f32,
    pub oil_consumption: f32,
    pub rpm_change: f32,
    pub car_temperature_bounds: [f32; 2],
    pub coolant_evaporate_rate: f32,
    pub temperature_difference_modifier: f32,
    pub temperature_move_modifier: f32,
    pub max_car_velocity: f32,
    pub awake_heat_modifier: f32,
    pub min_steam_temperature: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadiatorReference {
    pub source: SourceId,
    pub thermostat_range: [f32; 2],
    pub fan_temperature_on: f32,
    pub fan_temperature_off: f32,
    pub fan_on_when_engine_runs: bool,
    pub max_value: f32,
    pub max_fan_value: f32,
    pub max_fan_speed: f32,
    pub max_fluid_loss: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FluidRulesReference {
    pub source: SourceId,
    pub pour_speed: f32,
    pub slow_oil_modifier: f32,
    pub fluid_tune_speed: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerReference {
    pub source: SourceId,
    pub capsule_source: SourceId,
    pub camera_source: SourceId,
    pub walk_speed: f32,
    pub run_multiplier: f32,
    pub jump_force: f32,
    pub mass_kg: f32,
    pub duck_multiplier: f32,
    pub ctrl_duck_value: f32,
    pub duck_lerp: f32,
    pub head_lerp: f32,
    pub crouch_height: f32,
    pub capsule_radius: f32,
    pub capsule_height: f32,
    pub camera_height: f32,
    pub field_of_view_degrees: f32,
    pub aim_reach: f32,
    pub use_layer_mask: u32,
    pub get_out_layer_mask: u32,
    pub exit_capsule_half_height: f32,
    pub exit_capsule_radius: f32,
    pub standing_radius: f32,
    pub standing_height: f32,
    pub crouch_radius: f32,
    pub exit_distance: f32,
    pub exit_step_vertical: f32,
    pub exit_steps_vertical: u32,
    pub exit_step_horizontal: f32,
    pub exit_steps_horizontal: u32,
    pub carry_anchor_offset: [f32; 3],
    pub carry_offset_radius: f32,
    pub carry_drop_distance: f32,
    pub carry_force_multiplier: f32,
    pub carry_max_force: f32,
    pub carry_min_distance: f32,
    pub carry_max_distance: f32,
    pub obj_move_sensitivity: f32,
    pub obj_rotation_sensitivity: f32,
    pub look_sensitivity: [f32; 2],
    pub scroll_sensitivity: f32,
    pub camera_culling_mask: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatReference {
    pub id: SeatId,
    pub source: SourceId,
    pub item: ItemId,
    pub anchor: AnchorId,
    pub driver: bool,
    pub hit_nodes: Vec<NodeId>,
    pub yaw_limits_degrees: [f32; 2],
    pub exit_distance: f32,
    pub min_exit_angle_degrees: f32,
    pub exit_height: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WheelSlotReference {
    pub id: WheelSlotId,
    pub source: SourceId,
    pub anchor: AnchorId,
    pub mount_anchor: AnchorId,
    pub wheel: ItemId,
    pub tire: ItemId,
    pub rim: ItemId,
    pub radius: f32,
    pub suspension_distance: f32,
    pub center: [f32; 3],
    pub spring: f32,
    pub damper: f32,
    pub target_position: f32,
    pub steer: bool,
    pub drive: bool,
    pub brake: bool,
    pub handbrake: bool,
    pub forward_friction: [f32; 5],
    pub sideways_friction: [f32; 5],
    pub tire_condition_range: [u8; 2],
    pub rim_condition_range: [u8; 2],
    pub rim_main_types: Vec<String>,
    pub rim_types: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WheelPartReference {
    pub item: ItemId,
    pub source: SourceId,
    pub radius: f32,
    pub main_types: Vec<String>,
    pub part_types: Vec<String>,
    pub colliders: Vec<ColliderId>,
    pub tire_slot_anchor: Option<AnchorId>,
    pub tire_mount: Option<Transform>,
    pub tire_main_types: Vec<String>,
    pub tire_types: Vec<String>,
    pub local_rotation_after_mount: [f32; 4],
}

/// A general car part slot. The part fits only into its own slot.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartReference {
    pub item: ItemId,
    pub source: SourceId,
    /// `partslotscript.DT`: the part pose after removal.
    pub detach_node: Option<NodeId>,
    pub local_rotation_after_mount: [f32; 4],
}

/// One `headlightscript` state. `carscript.lampak` picks the state.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlightStateReference {
    pub intensity: f32,
    pub light: Option<SpotLightReference>,
    /// Lens `_EmissionColor` in this state. `None` is the dark lens.
    pub lens_emission: Option<[f32; 3]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpotLightReference {
    pub node: NodeId,
    pub color: [f32; 3],
    pub range: f32,
    /// Unity full cone angle.
    pub spot_degrees: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlightReference {
    pub item: ItemId,
    pub source: SourceId,
    /// `energyMove`: light energy change per second.
    pub energy_rate: f32,
    pub lens_node: NodeId,
    pub states: Vec<HeadlightStateReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SunColorKey {
    pub time: f32,
    pub color: [f32; 3],
}

/// `napszakvaltakozas` values. Biome arrays are desert, winter, and grass.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayCycleReference {
    pub source: SourceId,
    pub day_seconds: f32,
    pub night_seconds: f32,
    pub dawn_seconds: f32,
    pub dusk_seconds: f32,
    pub start_seconds: f32,
    pub day_intensity: f32,
    pub noon_intensity: f32,
    pub night_intensity: f32,
    pub day_ambient: f32,
    pub night_ambient: f32,
    pub day_angles: [f32; 2],
    pub midnight_angles: [f32; 2],
    pub moon_angles: [f32; 2],
    pub sun_colors: Vec<SunColorKey>,
    pub day_fog_colors: [[f32; 3]; 3],
    pub night_fog_colors: [[f32; 3]; 3],
}

/// `carscript.DamageStuff` values. Speeds are in km/h per frame.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrashReference {
    pub fall_off_speed: [f32; 2],
    pub max_fall_off_speed: f32,
    pub fall_off_counts: [[u8; 2]; 2],
    pub multiplier: f32,
    pub fall_off_multiplier: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleReference {
    pub source: SourceId,
    pub body: BodyId,
    pub root: NodeId,
    pub mass_kg: f32,
    pub serialized_mass_kg: f32,
    pub wheel_slots: [WheelSlotId; 4],
    pub colliders: Vec<ColliderId>,
    pub gear_ratios: Vec<f32>,
    pub final_drive_ratio: f32,
    pub max_steer_degrees: f32,
    pub brake_torque_nm: f32,
    pub handbrake_torque_nm: f32,
    pub wheel_slip: [f32; 4],
    /// Forward and side stiffness for a rim without a tire.
    pub rim_slip: [f32; 2],
    /// Car item. Its condition scales the crash speeds.
    pub item: ItemId,
    pub crash: CrashReference,
    pub auto_shift_stop_speed: f32,
    pub auto_shift_min_rpm: f32,
    pub auto_shift_max_rpm: f32,
    pub gear_free_run: Vec<bool>,
    pub dont_ignite_with_throttle_in_automatic: bool,
    pub auto_move_min: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartingAreaReference {
    pub source: SourceId,
    pub home_root: NodeId,
    pub home_pose: Transform,
    pub home_bounds: Bounds,
    pub player_spawn: Transform,
    pub car_pose: Transform,
    pub driveway_bounds: Bounds,
    pub driveway_height: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceManifest {
    pub ui: UiReference,
    pub instruments: InstrumentReference,
    pub app_id: u32,
    pub build_id: u32,
    pub source_hashes: Vec<SourceHash>,
    pub assets: Vec<AssetRecord>,
    pub nodes: Vec<NodeRecord>,
    pub bodies: Vec<BodyRecord>,
    pub colliders: Vec<ColliderRecord>,
    pub anchors: Vec<AnchorRecord>,
    pub hinges: Vec<HingeRecord>,
    pub wheel_slots: Vec<WheelSlotReference>,
    pub wheel_parts: Vec<WheelPartReference>,
    pub parts: Vec<PartReference>,
    pub headlights: Vec<HeadlightReference>,
    pub day_cycle: DayCycleReference,
    pub items: Vec<ItemRecord>,
    pub openings: Vec<OpeningRecord>,
    pub tanks: Vec<TankReference>,
    pub fluid_rules: FluidRulesReference,
    pub engine: EngineReference,
    pub player: PlayerReference,
    pub seats: Vec<SeatReference>,
    pub vehicle: VehicleReference,
    pub area: StartingAreaReference,
    pub world: world::WorldReference,
    pub audio: AudioReference,
}

impl ReferenceManifest {
    /// Validation keeps item IDs dense from 1.
    pub fn item(&self, id: ItemId) -> Option<&ItemRecord> {
        self.items.get((id.0 as usize).checked_sub(1)?)
    }
    /// Validation keeps opening IDs dense from 1.
    pub fn opening(&self, id: OpeningId) -> Option<&OpeningRecord> {
        self.openings.get((id.0 as usize).checked_sub(1)?)
    }
}
