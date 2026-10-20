use super::{InstanceId, SaveError, SourceKey};
use serde::{Deserialize, Serialize};

pub const SAVE_SCHEMA_VERSION: u32 = 9;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceHash {
    pub file: String,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceStamp {
    pub app_id: u32,
    pub build_id: u32,
    pub manifest_sha256: String,
    pub source_hashes: Vec<SourceHash>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveSnapshot {
    pub driving_assists: crate::reference::DrivingAssists,
    pub schema_version: u32,
    pub saved_at: String,
    pub source: SourceStamp,
    pub world: WorldSnapshot,
    pub player: PlayerSnapshot,
    pub inventory: InventorySnapshot,
    pub vehicles: Vec<VehicleSnapshot>,
    pub bodies: Vec<BodySnapshot>,
    pub items: Vec<ItemSnapshot>,
    pub attachments: AttachmentsSnapshot,
    pub mechanisms: MechanismsSnapshot,
    pub progress: ProgressSnapshot,
    pub lighting: LightingSnapshot,
    pub conditions: ConditionSnapshot,
    pub view: ViewSnapshot,
}
impl SaveSnapshot {
    pub fn from_json(bytes: &[u8]) -> Result<Self, SaveError> {
        #[derive(Deserialize)]
        struct Version {
            schema_version: u32,
        }
        let version: Version =
            serde_json::from_slice(bytes).map_err(|e| SaveError::CorruptData(e.to_string()))?;
        if version.schema_version != SAVE_SCHEMA_VERSION {
            return Err(SaveError::IncompatibleSources("Schema version".into()));
        }
        serde_json::from_slice(bytes).map_err(|e| SaveError::CorruptData(e.to_string()))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbsolutePose {
    pub position: [f64; 3],
    pub rotation: [f32; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalPose {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BodyIdentity {
    Source { source: SourceKey },
    Assembly { root: InstanceId },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodySnapshot {
    pub identity: BodyIdentity,
    pub pose: AbsolutePose,
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub sleeping: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSnapshot {
    pub generation: crate::world::GenerationSnapshot,
    pub seed: i32,
    #[serde(with = "super::identity::lossless")]
    pub changes: crate::world::ChangeRecords,
    #[serde(with = "super::identity::lossless")]
    pub spawn: crate::world::spawn_state::SpawnHistory,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerSnapshot {
    pub pose: AbsolutePose,
    pub yaw: f32,
    pub pitch: f32,
    pub seat: Option<SeatSnapshot>,
    pub posture: f32,
    pub vertical_velocity: f32,
    pub recoil_velocity: [f32; 3],
    pub running: bool,
    pub carry: Option<CarrySnapshot>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatSnapshot {
    pub vehicle: InstanceId,
    pub source: SourceKey,
    pub local_pose: LocalPose,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarrySnapshot {
    pub item: InstanceId,
    pub offset: [f32; 3],
    pub rotation: [f32; 4],
    pub local_grip: LocalPose,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventorySnapshot {
    pub slots: [Option<InstanceId>; 4],
    pub selected: u8,
    pub equipped: Option<InstanceId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoodSnapshot {
    pub source: SourceKey,
    pub bite: u8,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleSnapshot {
    pub instance: InstanceId,
    pub body: BodySnapshot,
    pub engine: EngineSnapshot,
    pub ignition: bool,
    pub gear: usize,
    pub handbrake: bool,
    pub trip_m: f64,
    pub requested_headlights: u8,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSnapshot {
    pub rpm: f32,
    pub temperature: f32,
    pub elapsed: f32,
    pub last_run_time: f32,
    pub last_start_chance_time: f32,
    pub random_start: f32,
    pub supplied_fuel: f32,
    pub fan_on: bool,
    pub oil_penalty: f32,
    pub fuel_suitability: f32,
    pub heat_penalty: f32,
    pub start_draw: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSnapshot {
    pub instance: InstanceId,
    pub pose: AbsolutePose,
    /// Offset in the owning body or attachment frame.
    pub local_pose: LocalPose,
    pub location: ItemLocationSave,
    pub temperature: Option<f32>,
    pub fluids: Option<Vec<crate::fluids::FluidEntry>>,
    pub food: Option<FoodSnapshot>,
    pub removed: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemLocationSave {
    World,
    Held,
    Stored {
        slot: u8,
    },
    Equipped,
    PartSlot {
        slot: SourceKey,
    },
    WheelSlot {
        slot: SourceKey,
    },
    Attached {
        owner: InstanceId,
    },
    GeneratedAttachment {
        #[serde(with = "super::identity::lossless")]
        attachment: crate::world::AttachmentSnapshot,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentsSnapshot {
    pub parts: Vec<PartAttachment>,
    pub wheels: Vec<WheelAttachment>,
    pub tires: Vec<TireAttachment>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartAttachment {
    pub slot: SourceKey,
    pub part: Option<InstanceId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WheelAttachment {
    pub slot: SourceKey,
    pub rim: Option<InstanceId>,
    pub rotation: f32,
    pub rpm: f32,
    pub previous_rotation: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TireAttachment {
    pub rim: InstanceId,
    pub tire: InstanceId,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MechanismsSnapshot {
    pub controls: Vec<MechanismSnapshot>,
    pub openings: Vec<OpeningSnapshot>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MechanismSnapshot {
    pub source: SourceKey,
    pub degrees: [f32; 2],
    pub slide: f32,
    pub enabled: bool,
    pub turn_state: u8,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpeningSnapshot {
    pub source: SourceKey,
    pub valve: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressSnapshot {
    pub selected_vehicle: InstanceId,
    pub start_attempts: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LightingSnapshot {
    pub cycle: CycleSnapshot,
    pub controllers: Vec<ControllerSnapshot>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CycleSnapshot {
    pub elapsed_seconds: f64,
    pub cycle_seconds: f64,
    pub evaluated_cycle_seconds: f64,
    pub cloud_coordinate: f32,
    pub star_rotation: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerSnapshot {
    pub source: SourceKey,
    pub current_state: usize,
    pub requested_state: usize,
    pub refreshed_state: usize,
    pub energy: f32,
    pub enabled: bool,
    pub on: bool,
    pub cabin_mode: Option<usize>,
    pub switch_motion: Option<SwitchMotionSave>,
    pub switch_poses: Vec<SwitchPoseSnapshot>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SwitchMotionSave {
    Initial,
    Moving { progress: f32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewSnapshot {
    pub camera: CameraSave,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CameraSave {
    FirstPerson,
    Chase,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwitchPoseSnapshot {
    pub node: SourceKey,
    pub pose: LocalPose,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionSnapshot {
    pub surfaces: Vec<SurfaceSnapshot>,
    pub washes: Vec<WashSnapshot>,
    pub tools: Vec<ToolSnapshot>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSnapshot {
    pub source: SourceKey,
    pub state: u8,
    pub state2: u32,
    pub state3: u32,
    pub color: [f32; 4],
    pub repair_progress: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WashSnapshot {
    pub source: SourceKey,
    pub active: bool,
    pub on: bool,
    pub destroyed: bool,
    pub repair_progress: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolSnapshot {
    pub source: SourceKey,
    pub quantity: Option<f32>,
    pub color: Option<[f32; 4]>,
    pub disabled: bool,
}

#[cfg(test)]
mod g04_ordered_state {
    use super::*;

    #[test]
    fn schema_six_and_seven_are_rejected_before_payload_reconstruction() {
        for version in [6, 7] {
            let payload = format!(r#"{{"schema_version":{version}}}"#);
            assert!(matches!(
                SaveSnapshot::from_json(payload.as_bytes()),
                Err(SaveError::IncompatibleSources(_))
            ));
        }
    }
}
