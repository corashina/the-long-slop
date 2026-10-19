mod required;
pub use required::{REQUIRED_VIEW_CAPACITY, RequiredContent, RequiredView, RequiredViews};
mod generation;
pub use generation::*;
pub mod changes;
pub mod types;
pub use changes::{
    AttachmentSnapshot, ChangeRecords, MechanismSnapshot, ObjectChange, ObjectSnapshot,
};
pub use types::*;

mod lamps;
mod source_lights;
pub use source_lights::{WorldLight, required_world_lights};
mod placement;
mod prop_layers;
mod props;
pub use props::{PropCandidate, PropGenerator};
pub mod spawn_state;
mod unity_random;
pub use placement::{
    BUILDING_CHECKPOINT_INTERVAL, BuildingCheckpoint, BuildingGroupState, BuildingSequence,
    FoundationPoint, PlacementIndex, PlacementIndexSnapshot, SupplyDefinition, SupplyFluid,
    SupplyPoint,
};
pub use spawn_state::{SourceSpawnState, SpawnBoundary, SpawnContext, SpawnPhase};
pub use unity_random::{UnityRandom, UnityRandomState};
mod road;
pub use road::{
    ROAD_CHECKPOINT_INTERVAL, ROAD_CONTROLS, ROAD_SEGMENTS, RoadBlock, RoadCheckpoint, RoadCurve,
    RoadIndex, RoadIndexSnapshot, RoadSegment, RoadSequence, RoadState, deform_road,
};
mod instances;
pub use instances::{StructureInstance, StructureRecipe};
pub(crate) mod metrics;
pub use metrics::WorldMetrics;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod renderer_pending;
pub(crate) mod stream;
pub use stream::content_index::ContentIndex;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod upload_receipt;
pub use stream::{
    PresentationAllocation, PublishedSupply, PublishedSurface, SurfaceKey, WorldFrame,
    WorldRequest, WorldStream,
};
mod startup;
pub use startup::StartupTerrain;
mod terrain;
pub use ::terrain::{
    RandomState, SourceRandom, TerrainContext, TerrainEvaluator, TerrainFoundation, TerrainJob,
    TerrainRoadPoint, TerrainTile, biome, source_perlin,
};
pub use terrain::{terrain_evaluator, terrain_output_bound, terrain_settings, terrain_tile};

pub const MAX_ABSOLUTE_COORDINATE: f64 = (1_u64 << 45) as f64;
pub const REBASE_LIMIT: f32 = 2048.0;
pub const DEFAULT_SEED: u64 = 42;

#[derive(Clone, Copy, Debug, Default)]
pub struct WorldOrigin {
    pub offset: [f64; 3],
}

impl WorldOrigin {
    pub fn absolute(&self, local: [f32; 3]) -> [f64; 3] {
        std::array::from_fn(|i| self.offset[i] + local[i] as f64)
    }

    pub fn local(&self, absolute: [f64; 3]) -> [f32; 3] {
        std::array::from_fn(|i| (absolute[i] - self.offset[i]) as f32)
    }

    pub fn shift(&mut self, shift: [f32; 3]) {
        for (offset, shift) in self.offset.iter_mut().zip(shift) {
            *offset += shift as f64;
        }
    }
}

mod spawn_contents;
mod spawn_initialization;
mod spawn_rules;
mod spawn_selection;

mod building_extras;
mod spawn_appearance;
mod spawn_building;
mod spawn_conditions;
mod spawn_lamps;
pub use spawn_appearance::NodeAppearance;
mod spawn_slots;
mod spawn_starts;
mod structure_helpers;

mod contents;

mod content_containers;

mod content_parts;

mod content_slots;
mod content_wheels;

mod content_mechanisms;

pub use contents::GeneratedCapVisual;

pub(crate) fn hash(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}
pub mod texture_batch_ownership;

pub mod stream_profile;
