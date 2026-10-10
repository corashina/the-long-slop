//! Validated installed-build data. Geometry uses converted body-local meters.

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionMasks {
    pub use_mask: u32,
    pub pickup_mask: u32,
    pub attach_mask: u32,
    pub push_mask: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionBinding {
    pub collider: ColliderId,
    pub use_owner: Option<ItemId>,
    pub pickup_owner: Option<ItemId>,
    pub removal_owner: Option<ItemId>,
    pub push_owner: Option<ItemId>,
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
    pub condition: f32,
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
    pub action_masks: ActionMasks,
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
    pub app_id: u32,
    pub build_id: u32,
    pub source_hashes: Vec<SourceHash>,
    pub assets: Vec<AssetRecord>,
    pub nodes: Vec<NodeRecord>,
    pub bodies: Vec<BodyRecord>,
    pub colliders: Vec<ColliderRecord>,
    pub action_bindings: Vec<ActionBinding>,
    pub anchors: Vec<AnchorRecord>,
    pub hinges: Vec<HingeRecord>,
    pub wheel_slots: Vec<WheelSlotReference>,
    pub wheel_parts: Vec<WheelPartReference>,
    pub items: Vec<ItemRecord>,
    pub openings: Vec<OpeningRecord>,
    pub tanks: Vec<TankReference>,
    pub fluid_rules: FluidRulesReference,
    pub engine: EngineReference,
    pub player: PlayerReference,
    pub seats: Vec<SeatReference>,
    pub vehicle: VehicleReference,
    pub area: StartingAreaReference,
}

#[derive(Debug)]
pub struct ReferenceError(pub String);
impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ReferenceError {}

fn require(ok: bool, field: &str) -> Result<(), ReferenceError> {
    if ok {
        Ok(())
    } else {
        Err(ReferenceError(format!("Invalid reference field: {field}")))
    }
}
fn finite(v: &[f32]) -> bool {
    v.iter().all(|x| x.is_finite())
}
fn transform(t: &Transform) -> bool {
    finite(&t.translation)
        && finite(&t.rotation)
        && finite(&t.scale)
        && t.scale.iter().all(|x| *x != 0.0)
        && (t.rotation.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 0.002
}
fn ids<T: Eq + std::hash::Hash>(
    values: impl Iterator<Item = T>,
    field: &str,
) -> Result<HashSet<T>, ReferenceError> {
    let mut result = HashSet::new();
    for value in values {
        require(result.insert(value), field)?;
    }
    Ok(result)
}
fn hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

impl ReferenceManifest {
    pub fn from_json(bytes: &[u8]) -> Result<Self, ReferenceError> {
        require(bytes.len() <= 64 * 1024 * 1024, "manifest size")?;
        let manifest: Self =
            serde_json::from_slice(bytes).map_err(|e| ReferenceError(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), ReferenceError> {
        require(
            self.app_id == APP_ID && self.build_id == BUILD_ID,
            "app/build",
        )?;
        let asset_ids = ids(self.assets.iter().map(|r| r.id), "duplicate asset ID")?;
        let node_ids = ids(self.nodes.iter().map(|r| r.id), "duplicate node ID")?;
        let body_ids = ids(self.bodies.iter().map(|r| r.id), "duplicate body ID")?;
        let collider_ids = ids(self.colliders.iter().map(|r| r.id), "duplicate collider ID")?;
        let anchor_ids = ids(self.anchors.iter().map(|r| r.id), "duplicate anchor ID")?;
        let item_ids = ids(self.items.iter().map(|r| r.id), "duplicate item ID")?;
        let opening_ids = ids(self.openings.iter().map(|r| r.id), "duplicate opening ID")?;
        ids(self.seats.iter().map(|r| r.id), "duplicate seat ID")?;
        let wheel_ids = ids(self.wheel_slots.iter().map(|r| r.id), "duplicate wheel ID")?;
        ids(
            self.wheel_slots.iter().map(|r| r.rim),
            "duplicate mounted rim",
        )?;
        ids(
            self.wheel_slots.iter().map(|r| r.tire),
            "duplicate mounted tire",
        )?;
        ids(
            self.wheel_slots.iter().map(|r| r.wheel),
            "duplicate mounted wheel",
        )?;
        require(
            self.assets
                .iter()
                .enumerate()
                .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .nodes
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .bodies
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .colliders
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .anchors
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .items
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .seats
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .wheel_slots
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u8),
            "dense runtime IDs",
        )?;
        ids(
            self.nodes.iter().map(|r| &r.source),
            "duplicate node source",
        )?;
        require(
            !self.assets.is_empty() && !self.source_hashes.is_empty(),
            "assets/source hashes",
        )?;
        for r in &self.source_hashes {
            require(!r.file.is_empty() && hash(&r.sha256), "source hash")?;
        }
        for r in &self.assets {
            require(
                r.path.starts_with("reference/") && !r.path.contains("..") && hash(&r.sha256),
                "asset path/hash",
            )?;
            let bindings = ids(
                self.nodes
                    .iter()
                    .filter(|n| n.asset == r.id)
                    .map(|n| n.gltf_node),
                "duplicate glTF node binding",
            )?;
            require(
                (0..bindings.len() as u32).all(|id| bindings.contains(&id)),
                "glTF node bindings",
            )?;
            ids(
                r.dependencies.iter().map(|dependency| &dependency.file),
                "duplicate asset dependency",
            )?;
            for dependency in &r.dependencies {
                require(
                    dependency.file.starts_with("reference/")
                        && !dependency.file.contains("..")
                        && hash(&dependency.sha256),
                    "asset dependency hash/path",
                )?;
            }
        }
        require(
            self.nodes.len() <= 100_000 && self.colliders.len() <= 100_000,
            "record count",
        )?;
        let node_map: HashMap<_, _> = self.nodes.iter().map(|r| (r.id, r)).collect();
        for r in &self.nodes {
            require(
                asset_ids.contains(&r.asset)
                    && r.parent
                        .is_none_or(|id| node_ids.contains(&id) && id != r.id),
                "node binding",
            )?;
            require(transform(&r.transform), "node transform")?;
            let mut parent = r.parent;
            for _ in 0..64 {
                if parent.is_none() {
                    break;
                }
                require(parent != Some(r.id), "node cycle")?;
                parent = parent
                    .and_then(|id| node_map.get(&id))
                    .and_then(|n| n.parent);
            }
            require(parent.is_none(), "node cycle")?;
        }
        for r in &self.bodies {
            require(
                r.parent_body
                    .is_none_or(|id| body_ids.contains(&id) && id != r.id),
                "body parent binding",
            )?;
            require(
                node_ids.contains(&r.node) && transform(&r.pose),
                "body binding/pose",
            )?;
            require(
                r.mass_kg.is_finite() && r.mass_kg >= 0.0 && (!r.dynamic || r.mass_kg > 0.0),
                "body mass",
            )?;
            require(
                finite(&[r.linear_damping, r.angular_damping])
                    && r.linear_damping >= 0.0
                    && r.angular_damping >= 0.0,
                "body damping",
            )?;
            require(r.pose.scale == [1.0; 3], "body scale")?;
            let mut parent = r.parent_body;
            for _ in 0..64 {
                if parent.is_none() {
                    break;
                }
                require(parent != Some(r.id), "body cycle")?;
                parent = self
                    .bodies
                    .iter()
                    .find(|body| Some(body.id) == parent)
                    .and_then(|body| body.parent_body);
            }
            require(parent.is_none(), "body cycle/depth")?;
        }
        for r in &self.colliders {
            require(
                body_ids.contains(&r.body) && node_ids.contains(&r.node),
                "collider binding",
            )?;
            require(
                r.friction.is_finite()
                    && r.friction >= 0.0
                    && r.restitution.is_finite()
                    && (0.0..=1.0).contains(&r.restitution),
                "collider material",
            )?;
            let valid = match &r.geometry {
                ColliderGeometry::Box {
                    center,
                    half_extents,
                    rotation,
                } => {
                    finite(center)
                        && finite(rotation)
                        && finite(half_extents)
                        && half_extents.iter().all(|x| *x > 0.0)
                }
                ColliderGeometry::Sphere { center, radius } => {
                    finite(center) && radius.is_finite() && *radius > 0.0
                }
                ColliderGeometry::Capsule { a, b, radius } => {
                    finite(a) && finite(b) && radius.is_finite() && *radius > 0.0
                }
                ColliderGeometry::Convex { points } => {
                    points.len() >= 4
                        && points.len() <= 1_000_000
                        && points.iter().all(|p| finite(p))
                }
                ColliderGeometry::Trimesh { points, triangles } => {
                    !points.is_empty()
                        && points.len() <= 1_000_000
                        && !triangles.is_empty()
                        && triangles.len() <= 2_000_000
                        && points.iter().all(|p| finite(p))
                        && triangles
                            .iter()
                            .flatten()
                            .all(|i| (*i as usize) < points.len())
                }
            };
            require(valid, "collider geometry")?;
        }
        let action_ids = ids(
            self.action_bindings.iter().map(|r| r.collider),
            "duplicate action collider",
        )?;
        require(
            action_ids == collider_ids,
            "complete action collider bindings",
        )?;
        for binding in &self.action_bindings {
            require(
                [
                    binding.use_owner,
                    binding.pickup_owner,
                    binding.removal_owner,
                    binding.push_owner,
                ]
                .iter()
                .all(|owner| owner.is_none_or(|id| item_ids.contains(&id))),
                "action owner binding",
            )?;
        }
        for r in &self.anchors {
            require(
                node_ids.contains(&r.node) && body_ids.contains(&r.body) && transform(&r.transform),
                "anchor binding",
            )?;
        }
        for r in &self.items {
            require(
                node_ids.contains(&r.node)
                    && body_ids.contains(&r.body)
                    && r.attached_to
                        .is_none_or(|id| item_ids.contains(&id) && id != r.id),
                "item binding",
            )?;
            require(
                r.condition.is_finite()
                    && (0.0..=1.0).contains(&r.condition)
                    && r.mass_kg.is_finite()
                    && r.mass_kg >= 0.0,
                "item condition/mass",
            )?;
        }
        for r in &self.hinges {
            require(
                item_ids.contains(&r.item)
                    && anchor_ids.contains(&r.anchor)
                    && body_ids.contains(&r.body),
                "hinge binding",
            )?;
            require(
                finite(&r.axis)
                    && finite(&[
                        r.closed_degrees,
                        r.min_degrees,
                        r.max_degrees,
                        r.drag_sensitivity,
                    ])
                    && r.min_degrees <= r.max_degrees
                    && r.drag_sensitivity > 0.0
                    && r.drag_input_axis < 2
                    && finite(&r.drag_axis)
                    && r.rotation_axes.iter().all(|axis| finite(axis))
                    && anchor_ids.contains(&r.drag_anchor)
                    && r.motor_degrees_per_second
                        .is_none_or(|speed| speed.is_finite() && speed > 0.0),
                "hinge parameters",
            )?;
        }
        for r in &self.openings {
            let visuals = &r.visuals;
            require(
                r.valve.is_finite()
                    && (0.0..=1.0).contains(&r.valve)
                    && r.tune_rate.is_finite()
                    && r.tune_rate >= 0.0
                    && (!r.can_tune || r.tune_rate > 0.0),
                "opening valve/tuning",
            )?;
            require(
                visuals
                    .open_nodes
                    .iter()
                    .chain(&visuals.closed_nodes)
                    .all(|id| node_ids.contains(id))
                    && [visuals.cap_node, visuals.closed_node, visuals.open_node]
                        .iter()
                        .all(|id| id.is_none_or(|id| node_ids.contains(&id)))
                    && finite(&visuals.closed_euler_degrees)
                    && finite(&visuals.open_euler_degrees),
                "cap visual bindings",
            )?;
            require(
                visuals.closed_node.is_some() == visuals.open_node.is_some()
                    && (visuals.cap_node.is_some()
                        || (visuals.closed_node.is_none() && !visuals.euler_rotation)),
                "cap visual control",
            )?;
            require(
                item_ids.contains(&r.tank)
                    && item_ids.contains(&r.cap)
                    && anchor_ids.contains(&r.anchor),
                "opening binding",
            )?;
            require(
                finite(&[r.flow_liters_per_second, r.tilt_degrees])
                    && r.flow_liters_per_second > 0.0
                    && finite(&r.tilt_axis)
                    && r.flow_radius.is_finite()
                    && r.flow_radius > 0.0,
                "opening flow",
            )?;
        }
        ids(self.tanks.iter().map(|r| r.item), "duplicate tank")?;
        for r in &self.tanks {
            require(
                item_ids.contains(&r.item) && r.openings.iter().all(|id| opening_ids.contains(id)),
                "tank binding",
            )?;
            require(
                r.capacity_liters.is_finite()
                    && r.capacity_liters > 0.0
                    && finite(&r.quantities_liters)
                    && r.quantities_liters.iter().all(|v| *v >= 0.0)
                    && r.quantities_liters.iter().sum::<f32>() <= r.capacity_liters + 0.001,
                "tank quantities",
            )?;
        }
        let e = &self.engine;
        require(
            item_ids.contains(&e.item)
                && [e.fuel_tank, e.oil_tank, e.coolant_tank]
                    .iter()
                    .all(|id| self.tanks.iter().any(|t| t.item == *id)),
            "engine tank bindings",
        )?;
        require(
            finite(&[
                e.start_chance,
                e.starter_delay_seconds,
                e.starter_rpm,
                e.starter_rpm_lerp,
                e.idle_rpm,
                e.max_rpm,
                e.shutdown_rpm,
                e.max_torque_nm,
                e.starting_push_force,
                e.consumption_modifier,
                e.oil_consumption_modifier,
                e.engine_brake_modifier,
                e.rpm_change_modifier,
                e.engine_start_time_modifier,
                e.heat_gain_min,
                e.heat_gain_max,
                e.heat_loss,
            ]) && finite(&e.optimal_temperature)
                && finite(&e.fuel_consumption),
            "engine parameters",
        )?;
        require(
            (0.0..=1.0).contains(&e.start_chance)
                && e.shutdown_rpm > 0.0
                && e.max_rpm > e.idle_rpm
                && e.max_torque_nm > 0.0,
            "engine limits",
        )?;
        require(
            e.torque_curve.keys.len() >= 2 && e.torque_curve.keys.len() <= 1024,
            "torque curve",
        )?;
        for k in &e.torque_curve.keys {
            require(
                finite(&[
                    k.time,
                    k.value,
                    k.in_tangent,
                    k.out_tangent,
                    k.in_weight,
                    k.out_weight,
                ]) && k.weighted_mode <= 3,
                "curve key",
            )?;
        }
        require(
            e.torque_curve
                .keys
                .windows(2)
                .all(|k| k[0].time < k[1].time),
            "curve key order",
        )?;
        let p = &self.player;
        require(p.camera_culling_mask != 0, "camera culling mask")?;
        require(
            p.scroll_sensitivity.is_finite() && p.scroll_sensitivity > 0.0,
            "scroll sensitivity",
        )?;
        require(
            finite(&p.carry_anchor_offset)
                && finite(&p.look_sensitivity)
                && finite(&[
                    p.carry_offset_radius,
                    p.carry_drop_distance,
                    p.carry_force_multiplier,
                    p.carry_max_force,
                    p.carry_min_distance,
                    p.carry_max_distance,
                    p.obj_move_sensitivity,
                    p.obj_rotation_sensitivity,
                ])
                && p.carry_offset_radius > 0.0
                && p.carry_drop_distance > 0.0
                && p.carry_min_distance >= 0.0
                && p.carry_max_distance >= p.carry_min_distance,
            "carry controls",
        )?;
        require(
            finite(&[
                e.auto_shift_clutch_rpm,
                e.auto_shift_clutch_rpm_range,
                self.vehicle.auto_shift_stop_speed,
                self.vehicle.auto_shift_min_rpm,
                self.vehicle.auto_shift_max_rpm,
            ]) && self.vehicle.auto_shift_min_rpm < self.vehicle.auto_shift_max_rpm,
            "automatic shift thresholds",
        )?;
        require(
            e.oil_fluid < 6
                && finite(&[
                    e.max_rpm_difference,
                    e.torque_modifier,
                    e.start_chance_interval_seconds,
                    e.globals.min_steam_temperature,
                ])
                && e.max_rpm_difference > 0.0
                && e.start_chance_interval_seconds > 0.0
                && e.globals.temperature_difference_modifier > 0.0
                && e.globals.max_car_velocity > 0.0,
            "engine scalar bounds",
        )?;
        require(
            finite(&[
                p.standing_radius,
                p.standing_height,
                p.crouch_radius,
                p.exit_distance,
                p.exit_step_vertical,
                p.exit_step_horizontal,
            ]) && p.standing_radius > 0.0
                && p.standing_height >= p.standing_radius * 2.0
                && p.crouch_radius > 0.0
                && p.crouch_height >= p.crouch_radius * 2.0
                && p.exit_distance > 0.0
                && p.exit_steps_vertical <= 100
                && p.exit_steps_horizontal <= 100,
            "effective player geometry/exit scan",
        )?;
        require(
            finite(&[
                e.initial_temperature,
                e.heat_loss_speed_modifier,
                e.globals.fuel_consumption,
                e.globals.oil_consumption,
                e.globals.rpm_change,
                e.globals.coolant_evaporate_rate,
                e.globals.temperature_difference_modifier,
                e.globals.temperature_move_modifier,
                e.globals.max_car_velocity,
                e.globals.awake_heat_modifier,
            ]) && finite(&e.globals.car_temperature_bounds),
            "engine globals",
        )?;
        require(
            finite(&e.radiator.thermostat_range)
                && finite(&[
                    e.radiator.fan_temperature_on,
                    e.radiator.fan_temperature_off,
                    e.radiator.max_value,
                    e.radiator.max_fan_value,
                    e.radiator.max_fan_speed,
                    e.radiator.max_fluid_loss,
                ]),
            "radiator parameters",
        )?;
        require(
            finite(&[
                self.fluid_rules.pour_speed,
                self.fluid_rules.slow_oil_modifier,
                self.fluid_rules.fluid_tune_speed,
            ]) && self.fluid_rules.pour_speed > 0.0,
            "fluid rules",
        )?;
        require(
            finite(&[
                p.mass_kg,
                p.duck_multiplier,
                p.ctrl_duck_value,
                p.duck_lerp,
                p.head_lerp,
            ]) && p.mass_kg > 0.0
                && (0.0..=1.0).contains(&p.ctrl_duck_value),
            "player movement",
        )?;
        require(
            finite(&[
                p.walk_speed,
                p.run_multiplier,
                p.jump_force,
                p.crouch_height,
                p.capsule_radius,
                p.capsule_height,
                p.camera_height,
                p.field_of_view_degrees,
                p.aim_reach,
                p.exit_capsule_half_height,
                p.exit_capsule_radius,
            ]) && p.walk_speed > 0.0
                && p.capsule_radius > 0.0
                && p.capsule_height >= p.capsule_radius * 2.0
                && p.crouch_height >= p.capsule_radius * 2.0
                && p.crouch_height < p.capsule_height
                && p.aim_reach > 0.0
                && p.action_masks.use_mask != 0
                && p.action_masks.pickup_mask != 0
                && p.action_masks.attach_mask != 0
                && p.action_masks.push_mask != 0
                && p.get_out_layer_mask != 0
                && p.exit_capsule_half_height > 0.0
                && p.exit_capsule_radius > 0.0,
            "player parameters",
        )?;
        let mut seat_hit_nodes = HashSet::new();
        for r in &self.seats {
            require(
                item_ids.contains(&r.item)
                    && anchor_ids.contains(&r.anchor)
                    && !r.hit_nodes.is_empty()
                    && r.hit_nodes.iter().all(|node| {
                        node_ids.contains(node)
                            && seat_hit_nodes.insert(*node)
                            && self
                                .nodes
                                .iter()
                                .any(|record| record.id == *node && record.active)
                    })
                    && finite(&r.yaw_limits_degrees)
                    && r.yaw_limits_degrees[0] <= r.yaw_limits_degrees[1]
                    && r.exit_distance.is_finite()
                    && r.exit_distance > 0.0,
                "seat binding/limits",
            )?;
        }
        require(
            self.seats.iter().filter(|s| s.driver).count() == 1,
            "driver seat",
        )?;
        for r in &self.wheel_slots {
            require(
                anchor_ids.contains(&r.anchor)
                    && [r.wheel, r.tire, r.rim]
                        .iter()
                        .all(|id| item_ids.contains(id)),
                "wheel bindings",
            )?;
            require(
                finite(&[
                    r.radius,
                    r.suspension_distance,
                    r.spring,
                    r.damper,
                    r.target_position,
                ]) && finite(&r.center)
                    && finite(&r.forward_friction)
                    && finite(&r.sideways_friction)
                    && r.radius > 0.0
                    && r.suspension_distance > 0.0,
                "wheel parameters",
            )?;
        }
        ids(
            self.wheel_parts.iter().map(|part| part.item),
            "duplicate wheel part",
        )?;
        for part in &self.wheel_parts {
            require(
                item_ids.contains(&part.item)
                    && !part.colliders.is_empty()
                    && part.colliders.iter().all(|id| collider_ids.contains(id))
                    && part
                        .tire_slot_anchor
                        .is_none_or(|id| anchor_ids.contains(&id)),
                "wheel part bindings",
            )?;
            require(
                part.radius.is_finite()
                    && part.radius > 0.0
                    && finite(&part.local_rotation_after_mount)
                    && part.tire_mount.as_ref().is_none_or(transform)
                    && part.main_types.len() <= 16
                    && part.part_types.len() <= 16
                    && !part.part_types.is_empty(),
                "wheel part geometry/fit",
            )?;
        }
        require(
            self.wheel_slots.iter().all(|slot| {
                self.wheel_parts.iter().any(|part| part.item == slot.rim)
                    && self.wheel_parts.iter().any(|part| part.item == slot.tire)
                    && anchor_ids.contains(&slot.mount_anchor)
            }),
            "wheel mount bindings",
        )?;
        let v = &self.vehicle;
        require(
            body_ids.contains(&v.body)
                && node_ids.contains(&v.root)
                && v.wheel_slots.iter().all(|id| wheel_ids.contains(id))
                && v.colliders.iter().all(|id| collider_ids.contains(id)),
            "vehicle bindings",
        )?;
        require(
            wheel_ids.len() == 4
                && v.wheel_slots.iter().collect::<HashSet<_>>().len() == 4
                && !v.colliders.is_empty(),
            "four road wheels/colliders",
        )?;
        require(
            finite(&[
                v.mass_kg,
                v.serialized_mass_kg,
                v.final_drive_ratio,
                v.max_steer_degrees,
                v.brake_torque_nm,
                v.handbrake_torque_nm,
            ]) && finite(&v.gear_ratios)
                && v.gear_ratios.iter().any(|ratio| *ratio == 0.0)
                && v.gear_free_run.len() == v.gear_ratios.len()
                && v.mass_kg > 0.0,
            "vehicle parameters",
        )?;
        let a = &self.area;
        require(
            node_ids.contains(&a.home_root)
                && transform(&a.home_pose)
                && transform(&a.player_spawn)
                && transform(&a.car_pose),
            "area binding/poses",
        )?;
        for b in [&a.home_bounds, &a.driveway_bounds] {
            require(
                finite(&b.min) && finite(&b.max) && (0..3).all(|i| b.min[i] <= b.max[i]),
                "area bounds",
            )?;
        }
        require(a.driveway_height.is_finite(), "driveway height")?;
        Ok(())
    }
}
