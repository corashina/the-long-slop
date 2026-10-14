pub mod changes;
pub mod types;
pub use changes::{
    AttachmentSnapshot, ChangeRecords, MechanismSnapshot, ObjectChange, ObjectSnapshot,
};
pub use types::*;

mod lamps;
mod source_lights;
pub use source_lights::{WorldLight, required_world_lights};
mod noise;
mod placement;
mod prop_layers;
mod props;
pub use props::{PropCandidate, PropGenerator};
mod random;
mod spawn_state;
mod unity_random;
pub use placement::{
    BUILDING_CHECKPOINT_INTERVAL, BuildingCheckpoint, BuildingGroupState, BuildingSequence,
    FoundationPoint, PlacementIndex, SupplyDefinition, SupplyFluid, SupplyPoint,
};
pub use spawn_state::{SourceSpawnState, SpawnBoundary, SpawnContext, SpawnPhase};
pub use unity_random::{UnityRandom, UnityRandomState};
mod road;
pub use noise::{TerrainEvaluator, source_perlin};
pub use random::{RandomState, SourceRandom};
pub use road::{
    ROAD_CHECKPOINT_INTERVAL, ROAD_CONTROLS, ROAD_SEGMENTS, RoadCheckpoint, RoadCurve, RoadIndex,
    RoadSegment, RoadSequence, RoadState, deform_road,
};
mod instances;
pub use instances::{StructureInstance, StructureRecipe};
pub(crate) mod metrics;
pub use metrics::WorldMetrics;
mod stream;
pub use stream::{
    PresentationAllocation, PublishedSupply, PublishedSurface, SurfaceKey, WorldFrame,
    WorldRequest, WorldStream,
};
mod startup;
pub use startup::StartupTerrain;
mod terrain;
pub use terrain::{
    TerrainContext, TerrainFoundation, TerrainRoadPoint, TerrainSurfaceJob, TerrainTile, biome,
    terrain_surface,
};

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
