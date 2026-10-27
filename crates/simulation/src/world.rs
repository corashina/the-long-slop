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

mod draw_distance;
pub use draw_distance::DrawDistances;
mod cables;
pub use cables::aim;
mod lamps;
mod source_lights;
pub use source_lights::{WorldLight, required_world_lights};
mod placement;
mod prop_layers;
mod props;
pub use props::{PropCandidate, PropGenerator};
pub mod spawn_state;
pub mod unity_random;
pub use placement::{
    BUILDING_CHECKPOINT_INTERVAL, BuildingCheckpoint, BuildingGroupState, BuildingSequence,
    FoundationPoint, PlacementIndex, PlacementIndexSnapshot, SupplyDefinition, SupplyFluid,
    SupplyPoint,
};
pub use placement::{node_frame, place_point};
pub use spawn_state::{SourceSpawnState, SpawnBoundary, SpawnContext, SpawnPhase};
pub use unity_random::{UnityRandom, UnityRandomState};
mod road;
pub mod road_worker;
pub use road::{
    ROAD_CHECKPOINT_INTERVAL, ROAD_CONTROLS, ROAD_SEGMENTS, RoadBlock, RoadCheckpoint, RoadCurve,
    RoadIndex, RoadIndexSnapshot, RoadSegment, RoadSequence, RoadState,
};
pub use road_worker::{RoadJobId, RoadPoll, RoadWorker};
mod instances;
pub use instances::{StructureInstance, StructureRecipe, TriggerRoles};
pub mod metrics;
pub use metrics::WorldMetrics;
pub mod renderer_facts;
pub use renderer_facts::{RendererFact, RendererFacts};
pub mod stream;
mod stream_limits;
pub use stream::content_boxes::BoxRelease;
pub use stream::content_explosives::{Explosive, TankExplosion};
pub use stream::content_index::ContentIndex;
pub use stream::{
    PresentationAllocation, PresentationGrant, PresentationReport, PublishedSupply,
    PublishedSurface, SurfaceKey, WorldFrame, WorldRequest, WorldStream,
};
pub use stream_limits::StreamLimits;
mod start_car;
pub use start_car::{StartCar, StartCarChoice, menu_random, start_fields};
mod startup;
pub use startup::{StartupTerrain, source_world_pose};
mod terrain;
pub use ::terrain::{
    RandomState, SourceRandom, TerrainContext, TerrainEvaluator, TerrainFoundation, TerrainJob,
    TerrainRoadPoint, TerrainTile, biome, forced_color, source_perlin,
};
pub use terrain::{terrain_evaluator, terrain_output_bound, terrain_settings, terrain_tile};

/// Game mode. Biome: 0 Mixed, 1 Desert, 2 Winter, 3 Grass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameMode {
    pub biome: u8,
    pub start_car: StartCarChoice,
}

impl GameMode {
    /// Biome index for `biome`: 0 desert, 1 winter, 2 grass. Mixed uses the terrain color.
    pub fn biome_override(self) -> Option<u8> {
        self.biome.checked_sub(1)
    }
}

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

#[cfg(test)]
mod loot_audit;
mod spawn_contents;
mod spawn_initialization;
mod spawn_rules;
mod spawn_selection;

mod biome_materials;
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
pub mod content_vehicles;
mod content_wheels;

mod content_mechanisms;

pub use contents::GeneratedCapVisual;

pub fn hash(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

pub mod stream_profile;
