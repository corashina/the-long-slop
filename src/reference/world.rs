//! Checked source world inputs. Prototype node indices belong to one asset instance.
mod fields;
mod grass;
pub use fields::WorldFields;
mod initial_materials;
mod initialization;
mod lod;
mod memory;
use super::{
    AssetId, AssetUsage, ColliderGeometry, LibraryProvenance, ReferenceError, ReferenceManifest,
    SourceHash, SourceId, Transform, finite, hash, require, transform,
};
pub use grass::*;
pub use initial_materials::*;
pub use initialization::*;
pub use lod::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const WORLD_SAMPLE_SHA256: &str =
    "243f2862c6fa3d38b4c7d8df56332a2016c09d412b11030564f7f21665ad8ad9";
pub const WORLD_MANAGED_SHA256: &str =
    "9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldReference {
    pub sample_payload_sha256: String,
    pub managed_sha256: String,
    pub library: LibraryProvenance,
    pub capture_library: LibraryProvenance,
    pub settings_context: String,
    pub spawn_context: WorldSpawnContext,
    pub initialization: WorldInitialization,
    pub generation_frame: Transform,
    /// Rotation of `mainscript.magneticField`; compasses point along its forward.
    pub magnetic_field: [f32; 4],
    /// mainscript.lightTurnOffValue: electronics output that powers radios and lamps.
    pub electronics_cutoff: f32,
    pub distances: BTreeMap<String, f32>,
    #[serde(deserialize_with = "compact_vec")]
    pub source_hashes: Vec<SourceHash>,
    pub road: WorldRoadReference,
    pub terrain: WorldTerrainReference,
    pub buildings: WorldBuildingReference,
    pub catalog: WorldCatalogReference,
    #[serde(deserialize_with = "compact_vec")]
    pub part_scope_slots: Vec<WorldPartScopeSlot>,
    #[serde(deserialize_with = "compact_vec")]
    pub generators: Vec<WorldGeneratorReference>,
    #[serde(deserialize_with = "compact_vec")]
    pub startup: Vec<WorldStartupReference>,
    #[serde(deserialize_with = "compact_vec")]
    pub startup_holes: Vec<WorldStartupHole>,
    pub collision_rules: WorldCollisionRules,
    pub grass_displacement: WorldGrassDisplacement,
    pub lod: WorldLodReference,
    #[serde(deserialize_with = "compact_vec")]
    pub initial_materials: Vec<WorldInitialMaterial>,
    #[serde(deserialize_with = "compact_vec")]
    pub mechanisms: Vec<WorldMechanism>,
    #[serde(deserialize_with = "compact_vec")]
    pub painting_materials: Vec<WorldPaintingMaterial>,
    #[serde(deserialize_with = "compact_vec")]
    pub prototypes: Vec<WorldPrototypeReference>,
    #[serde(deserialize_with = "compact_vec")]
    pub records: Vec<WorldSourceRecord>,
    #[serde(deserialize_with = "compact_vec")]
    pub receipts: Vec<WorldSourceReceipt>,
    #[serde(deserialize_with = "compact_vec")]
    pub exclusions: Vec<WorldExclusion>,
    #[serde(deserialize_with = "compact_vec")]
    pub material_textures: Vec<super::rendering::LightingTextureRecord>,
    pub pickup_hits: Vec<WorldPickupHit>,
}

/// Source hit sound of a world `pickupable` record with a `collideSound`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPickupHit {
    pub source: SourceId,
    pub collision_clip: u32,
    pub collision_gain: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSpawnContext {
    pub classification: String,
    pub engine_sha256: String,
    pub clock_input: String,
    pub initial_state: [u32; 4],
    pub startup_state: [u32; 4],
    pub startup_draws: u32,
    pub date: [i32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldMechanism {
    pub prototype: SourceId,
    pub node: u32,
    pub recipe: super::MechanismReference,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPrototypeReference {
    pub source: SourceId,
    pub role: WorldPrototypeRole,
    pub asset: AssetId,
    #[serde(deserialize_with = "compact_vec")]
    pub nodes: Vec<WorldPrototypeNode>,
    #[serde(deserialize_with = "compact_vec")]
    pub controls: Vec<u32>,
    #[serde(deserialize_with = "compact_vec")]
    pub excluded_actions: Vec<String>,
    #[serde(deserialize_with = "compact_vec")]
    pub deferred_materials: Vec<WorldSourceMaterial>,
    #[serde(deserialize_with = "compact_vec")]
    pub skins: Vec<WorldSkinReference>,
    #[serde(deserialize_with = "compact_vec")]
    pub bodies: Vec<WorldPrototypeBody>,
    #[serde(deserialize_with = "compact_vec")]
    pub colliders: Vec<WorldColliderReference>,
    pub prefab_pose: Transform,
    #[serde(default, deserialize_with = "compact_vec")]
    pub interiors: Vec<WorldInteriorReference>,
}

/// Source `interiorscript` of a world building. Doors are `usablescript` world mechanisms.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldInteriorReference {
    pub collider: SourceId,
    /// Interior index in the same prototype.
    pub parent: Option<u32>,
    pub invert: bool,
    pub minimum_doors_to_max: u32,
    pub maximum_transmission: f32,
    pub use_value: bool,
    pub value: f32,
    pub doors: Vec<SourceId>,
    pub invert_doors: Vec<bool>,
    pub parent_door: Option<SourceId>,
    pub method: super::AudioMethod,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldPrototypeRole {
    Road,
    Structure,
    Prop,
    Nested,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPrototypeNode {
    pub source: SourceId,
    pub name: String,
    pub parent: Option<u32>,
    pub active: bool,
    pub renderer_enabled: bool,
    pub layer: u8,
    pub transform: Transform,
    pub gltf_node: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSkinReference {
    pub source: SourceId,
    pub mesh: SourceId,
    pub node: u32,
    #[serde(deserialize_with = "compact_vec")]
    pub bones: Vec<u32>,
    #[serde(deserialize_with = "compact_vec")]
    pub bind_poses: Vec<[f32; 16]>,
    #[serde(deserialize_with = "compact_vec")]
    pub positions: Vec<[f32; 3]>,
    #[serde(deserialize_with = "compact_vec")]
    pub normals: Vec<[f32; 3]>,
    #[serde(deserialize_with = "compact_vec")]
    pub tangents: Vec<[f32; 4]>,
    #[serde(deserialize_with = "compact_vec")]
    pub uv: Vec<[f32; 2]>,
    #[serde(deserialize_with = "compact_vec")]
    pub joints: Vec<[u32; 4]>,
    #[serde(deserialize_with = "compact_vec")]
    pub weights: Vec<[f32; 4]>,
    #[serde(deserialize_with = "compact_vec")]
    pub triangles: Vec<Vec<[u32; 3]>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldColliderReference {
    pub body: Option<u32>,
    pub source: SourceId,
    pub node: u32,
    pub enabled: bool,
    pub trigger: bool,
    pub role: Option<TriggerRole>,
    pub material: Option<SourceId>,
    pub deformed_skin: Option<u32>,
    pub source_empty_mesh: Option<SourceId>,
    pub geometry: Option<ColliderGeometry>,
    pub step_surface: Option<super::StepSurface>,
}
/// Gameplay role of a trigger collider.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TriggerRole {
    Ladder {
        speed: f32,
        up: [f32; 3],
    },
    Hazard {
        damage_player: f32,
        damage_breakable: f32,
    },
}
impl WorldColliderReference {
    fn role_valid(&self) -> bool {
        self.role.is_none_or(|r| self.trigger && r.valid())
    }
}
impl TriggerRole {
    fn valid(&self) -> bool {
        match *self {
            Self::Ladder { speed, up } => {
                finite(&up)
                    && speed.is_finite()
                    && speed > 0.0
                    && ((up[0] * up[0] + up[1] * up[1] + up[2] * up[2]).sqrt() - 1.0).abs() <= 1e-3
            }
            Self::Hazard {
                damage_player,
                damage_breakable,
            } => {
                finite(&[damage_player, damage_breakable])
                    && damage_player >= 0.0
                    && damage_breakable >= 0.0
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSourceReceipt {
    pub source: SourceId,
    pub sha256: String,
}
/// Preserve caller fields and explicit null selections without unresolved Unity pointers.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum WorldValue {
    Empty,
    Bool(bool),
    Integer(i64),
    Number(f64),
    Text(String),
    Link(SourceId),
    Array(#[serde(deserialize_with = "compact_vec")] Vec<WorldValue>),
    Record(WorldFields),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSourceRecord {
    pub source: SourceId,
    pub class_name: String,
    pub fields: WorldFields,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldExclusion {
    pub source: SourceId,
    pub field: String,
    pub target: Option<SourceId>,
    pub reason: String,
    pub catalog: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldStartupReference {
    pub group: String,
    pub source: SourceId,
    pub pose: Transform,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCollisionRules {
    pub source: SourceId,
    pub record_sha256: String,
    pub layers: [u32; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldStartupHole {
    pub component: SourceId,
    pub filter: SourceId,
    pub mesh: SourceId,
    pub mesh_sha256: String,
    /// Exact first four vertices, before coordinate conversion.
    pub vertices: [[f32; 3]; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldNoiseReference {
    pub amplitude: f32,
    pub frequency: f32,
    pub lacunarity: f32,
    pub octaves: u32,
    pub persistence: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldRoadReference {
    #[serde(rename = "genAtStart")]
    pub gen_at_start: u32,
    #[serde(rename = "genInOneFrame")]
    pub gen_in_one_frame: u32,
    pub source: SourceId,
    #[serde(deserialize_with = "compact_vec")]
    pub prototypes: Vec<SourceId>,
    pub length: f32,
    #[serde(rename = "roadBoneNum")]
    pub road_bone_num: u32,
    #[serde(rename = "roadBoneLength")]
    pub road_bone_length: f32,
    #[serde(rename = "roadSegmentLength")]
    pub road_segment_length: f32,
    #[serde(rename = "roadAboveTerrain")]
    pub road_above_terrain: f32,
    pub width: f32,
    pub scale: f32,
    #[serde(rename = "XmaxAngle")]
    pub xmax_angle: f32,
    #[serde(rename = "YmaxAngle")]
    pub ymax_angle: f32,
    #[serde(rename = "startStraight")]
    pub start_straight: u32,
    #[serde(rename = "straightModifier")]
    pub straight_modifier: f32,
    #[serde(rename = "alignEdgeOfTheWorld")]
    pub align_edge_of_the_world: u8,
    #[serde(rename = "aliignRoad")]
    pub aliign_road: u8,
    #[serde(rename = "alignBuildings")]
    pub align_buildings: u8,
    #[serde(rename = "forceHomesick")]
    pub force_homesick: u8,
    #[serde(rename = "defPointRange")]
    pub def_point_range: f32,
    #[serde(rename = "defPointStep")]
    pub def_point_step: f32,
    #[serde(rename = "addPoint")]
    pub add_point: u32,
    #[serde(rename = "buildingBoneNum")]
    pub building_bone_num: u32,
    #[serde(deserialize_with = "compact_vec")]
    pub noises: Vec<WorldNoiseReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldTerrainReference {
    pub source: SourceId,
    #[serde(deserialize_with = "compact_vec")]
    pub prefabs: Vec<SourceId>,
    #[serde(deserialize_with = "compact_vec")]
    pub materials: Vec<WorldSourceMaterial>,
    #[serde(deserialize_with = "compact_vec")]
    pub textures: Vec<super::rendering::LightingTextureRecord>,
    pub game_settings: WorldGameSettings,
    #[serde(deserialize_with = "compact_vec")]
    pub tiles: Vec<WorldTileReference>,
    #[serde(rename = "edgeOfWorld")]
    pub edge_of_world: f32,
    #[serde(rename = "edgeOfWorldGradient")]
    pub edge_of_world_gradient: f32,
    #[serde(rename = "edgeOfWorldGenerationDist")]
    pub edge_of_world_generation_dist: f32,
    #[serde(rename = "edgeOfWorldWarpDist")]
    pub edge_of_world_warp_dist: f32,
    #[serde(rename = "edgeOfWorldWarpOffset")]
    pub edge_of_world_warp_offset: f32,
    #[serde(rename = "randomSeed")]
    pub random_seed: u8,
    pub seed: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldGameSettings {
    pub source: SourceId,
    pub biome: i32,
    #[serde(rename = "homesickMode")]
    pub homesick_mode: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldTileReference {
    #[serde(rename = "tileSize")]
    pub tile_size: u32,
    pub resolution: u32,
    #[serde(rename = "drawDistance")]
    pub draw_distance: u32,
    #[serde(rename = "YCoord")]
    pub y_coord: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldGeneratorReference {
    pub source: SourceId,
    #[serde(deserialize_with = "compact_vec")]
    pub types: Vec<WorldObjectTypeReference>,
    pub seedplus: i32,
    #[serde(rename = "noSpawnChance")]
    pub no_spawn_chance: f32,
    #[serde(rename = "pointDist")]
    pub point_dist: f32,
    #[serde(rename = "chunkDist")]
    pub chunk_dist: f32,
    #[serde(rename = "heightAdd")]
    pub height_add: f32,
    #[serde(rename = "affectedByRoad")]
    pub affected_by_road: u8,
    #[serde(rename = "affectsTerrain")]
    pub affects_terrain: u8,
    #[serde(rename = "alignToBuilding")]
    pub align_to_building: u8,
    #[serde(rename = "alignToRoad")]
    pub align_to_road: u8,
    pub buildingspawn: u8,
    #[serde(rename = "waitForTerrain")]
    pub wait_for_terrain: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldObjectTypeReference {
    pub prototype: Option<SourceId>,
    #[serde(rename = "InDesert")]
    pub in_desert: u8,
    #[serde(rename = "InGrass")]
    pub in_grass: u8,
    #[serde(rename = "InWinter")]
    pub in_winter: u8,
    #[serde(rename = "affectedByRoad")]
    pub affected_by_road: u8,
    pub chance: f32,
    #[serde(rename = "maxDistFromRoad")]
    pub max_dist_from_road: f32,
    #[serde(rename = "minDistFromBuilding")]
    pub min_dist_from_building: f32,
    #[serde(rename = "minDistFromRoad")]
    pub min_dist_from_road: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldBuildingReference {
    pub source: SourceId,
    #[serde(deserialize_with = "compact_vec")]
    pub types: Vec<WorldBuildingType>,
    #[serde(deserialize_with = "compact_vec")]
    pub groups: Vec<WorldBuildingGroup>,
    #[serde(rename = "defaultSides")]
    #[serde(deserialize_with = "compact_vec")]
    pub default_sides: Vec<WorldBuildingSide>,
    #[serde(rename = "minStartDist")]
    pub min_start_dist: f32,
    #[serde(rename = "maxStartDist")]
    pub max_start_dist: f32,
    #[serde(rename = "distModifier")]
    pub dist_modifier: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldBuildingType {
    pub type_id: i32,
    pub prototype: SourceId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldBuildingGroup {
    #[serde(deserialize_with = "compact_vec")]
    pub available: Vec<i32>,
    #[serde(rename = "currentAvailable")]
    #[serde(deserialize_with = "compact_vec")]
    pub current_available: Vec<i32>,
    #[serde(rename = "currentCount")]
    pub current_count: i32,
    #[serde(rename = "currentMaxCount")]
    pub current_max_count: i32,
    #[serde(rename = "maxCount")]
    pub max_count: i32,
    #[serde(rename = "minCount")]
    pub min_count: i32,
    #[serde(deserialize_with = "compact_vec")]
    pub types: Vec<WorldBuildingSelection>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldBuildingSelection {
    #[serde(rename = "type")]
    pub type_id: i32,
    #[serde(rename = "dontUseDefault")]
    pub dont_use_default: u8,
    #[serde(rename = "useModifier")]
    pub use_modifier: u8,
    #[serde(deserialize_with = "compact_vec")]
    pub sides: Vec<WorldBuildingSide>,
    #[serde(rename = "maxDist")]
    pub max_dist: f32,
    #[serde(rename = "minDist")]
    pub min_dist: f32,
    #[serde(rename = "xDistModifier")]
    pub x_dist_modifier: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldBuildingSide {
    #[serde(rename = "maxXpos")]
    pub max_xpos: f32,
    #[serde(rename = "maxYAngle")]
    pub max_y_angle: f32,
    #[serde(rename = "minXpos")]
    pub min_xpos: f32,
    #[serde(rename = "minYAngle")]
    pub min_y_angle: f32,
}
fn source_valid(source: &SourceId, hashes: &[SourceHash]) -> bool {
    source.path_id != 0 && hashes.iter().any(|h| h.file == source.file)
}
fn field(source: &SourceId, name: &str) -> String {
    format!("{}:{} {name}", source.file, source.path_id)
}
fn values_valid(value: &WorldValue, receipts: &HashSet<&SourceId>) -> bool {
    match value {
        WorldValue::Number(n) => n.is_finite(),
        WorldValue::Link(s) => receipts.contains(s),
        WorldValue::Array(a) => a.iter().all(|v| values_valid(v, receipts)),
        WorldValue::Record(a) => a.values().all(|v| values_valid(v, receipts)),
        _ => true,
    }
}
fn record_link<'a>(record: &'a WorldSourceRecord, name: &str) -> Option<&'a SourceId> {
    match record.fields.get(name) {
        Some(WorldValue::Link(source)) => Some(source),
        _ => None,
    }
}
fn checked_record<'a>(
    records: &HashMap<&SourceId, &'a WorldSourceRecord>,
    source: &SourceId,
    classes: &[&str],
) -> Result<&'a WorldSourceRecord, ReferenceError> {
    let record = records
        .get(source)
        .copied()
        .ok_or_else(|| ReferenceError(field(source, "missing ownership record")))?;
    require(
        classes.contains(&record.class_name.as_str()),
        &field(source, "ownership record class"),
    )?;
    Ok(record)
}
fn component_listed(record: &WorldSourceRecord, source: &SourceId) -> bool {
    record.components().any(|component| component == source)
}
fn node_links_match(
    record: &WorldSourceRecord,
    name: &str,
    indices: &[u32],
    nodes: &[WorldPrototypeNode],
) -> bool {
    matches!(record.fields.get(name), Some(WorldValue::Array(values)) if values.len() == indices.len()
        && values.iter().zip(indices).all(|(value, index)| matches!(value, WorldValue::Link(source) if nodes.get(*index as usize).is_some_and(|node| &node.source == source))))
}
impl WorldReference {
    pub fn validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError> {
        require(
            self.sample_payload_sha256 == WORLD_SAMPLE_SHA256
                && self.managed_sha256 == WORLD_MANAGED_SHA256,
            "world source sample/hash",
        )?;
        require(
            self.settings_context == "minimum_draw_distance_frozen_generation_frame"
                && transform(&self.generation_frame),
            "world source context/frame",
        )?;
        validate_world_libraries(&self.library, &self.capture_library, &manifest.library)?;
        require(
            self.spawn_context.classification == "controlled_startup_before_garage"
                && self.spawn_context.engine_sha256
                    == "553c1e7560e1b42bd4638c53d0168dcd1b4b9eb5b9df324b0a44f14b3f18bc86"
                && self.spawn_context.clock_input == "2026-10-12T09:00:00 local"
                && self.spawn_context.date == [2026, 10, 12]
                && self.spawn_context.initial_state
                    == [123456789, 2139825738, 225031475, 1427228704]
                && self.spawn_context.startup_state
                    == [2365297906, 2858699345, 3940856323, 829372343]
                && self.spawn_context.startup_draws == 4,
            "world controlled spawn context",
        )?;
        require(!self.source_hashes.is_empty(), "world source hashes")?;
        let mut files = HashSet::new();
        for h in &self.source_hashes {
            require(
                files.insert(&h.file)
                    && hash(&h.sha256)
                    && manifest
                        .source_hashes
                        .iter()
                        .any(|m| m.file == h.file && m.sha256 == h.sha256),
                &format!("{} world source hash", h.file),
            )?;
        }
        let receipts: HashSet<_> = self.receipts.iter().map(|r| &r.source).collect();
        require(
            !receipts.is_empty() && receipts.len() == self.receipts.len(),
            "world source receipt identities",
        )?;
        for r in &self.receipts {
            require(
                source_valid(&r.source, &self.source_hashes) && hash(&r.sha256),
                &field(&r.source, "record receipt"),
            )?;
        }
        self.initialization.validate(self, &receipts)?;
        for row in &self.mechanisms {
            let prototype = self
                .prototypes
                .iter()
                .find(|p| p.source == row.prototype)
                .ok_or_else(|| ReferenceError("Missing world mechanism prototype".into()))?;
            let node = |n: super::NodeId| n.0 > 0 && n.0 as usize <= prototype.nodes.len();
            require(
                (row.node as usize) < prototype.nodes.len()
                    && receipts.contains(&row.recipe.source),
                "world mechanism source/node",
            )?;
            require(
                row.recipe.driven_nodes.iter().all(|n| node(*n))
                    && node(row.recipe.drag.node)
                    && node(row.recipe.drag.node2),
                "world mechanism targets",
            )?;
            if let Some(rotation) = &row.recipe.rotation {
                require(
                    rotation.targets.iter().all(|t| node(t.node))
                        && rotation.sensitivity.is_finite(),
                    "world rotation inputs",
                )?;
            }
            if let super::EndpointReactionReference::Rotation {
                node: control,
                force,
                ..
            } = &row.recipe.reaction
            {
                require(
                    row.recipe.rotation.is_some()
                        && node(*control)
                        && force.is_finite()
                        && *force >= 0.,
                    "world mechanism endpoint reaction",
                )?;
            }
            if let Some(slide) = &row.recipe.slide {
                require(
                    slide.paths.iter().all(|p| {
                        node(p.node) && p.points.iter().all(|p| node(p.node) && p.value.is_finite())
                    }) && slide
                        .rotations
                        .iter()
                        .all(|r| node(r.node) && node(r.reference))
                        && slide
                            .animation_source
                            .as_ref()
                            .is_none_or(|s| receipts.contains(s)),
                    "world slide inputs",
                )?;
            }
            require(
                row.recipe.child.is_none_or(|id| {
                    self.mechanisms
                        .iter()
                        .any(|r| r.prototype == row.prototype && r.node + 1 == id.0)
                }),
                "world mechanism child",
            )?;
        }

        let records: HashSet<_> = self.records.iter().map(|r| &r.source).collect();
        require(
            records.len() == self.records.len(),
            "world duplicate source record",
        )?;
        let record_map: HashMap<_, _> = self.records.iter().map(|r| (&r.source, r)).collect();
        for r in &self.records {
            require(
                receipts.contains(&r.source) && !r.class_name.is_empty() && !r.fields.is_empty(),
                &field(&r.source, "record"),
            )?;
            for (k, v) in r.fields.iter() {
                require(values_valid(v, &receipts), &field(&r.source, k))?;
            }
            let required: &[&str] = match r.class_name.as_str() {
                "newRandomStuffSpawnScript" => &[
                    "items",
                    "chanceToSpawn",
                    "dontSpawn",
                    "holiday",
                    "buildingAffectsThis",
                ],
                "buildingscript" => &["newRandomStuff"],
                "terrainHeightAlignToBuildingScript" => &["helpPosCoords"],
                "digholescript2" => &["notAtStart"],
                "LODGroup" => &[
                    "m_GameObject",
                    "m_LocalReferencePoint",
                    "m_Size",
                    "m_LODs",
                    "m_FadeMode",
                    "m_AnimateCrossFading",
                ],
                _ => &[],
            };
            for name in required {
                require(r.fields.get(name).is_some(), &field(&r.source, name))?;
            }
            if r.class_name == "LODGroup" {
                require(
                    matches!(r.fields.get("m_LocalReferencePoint"), Some(WorldValue::Record(point))
                        if ["x", "y", "z"].iter().all(|axis|
                            matches!(point.get(axis), Some(WorldValue::Number(value))
                                if (*value as f32).is_finite()))),
                    &field(&r.source, "LOD reference point"),
                )?;
                require(
                    matches!(r.fields.get("m_Size"), Some(WorldValue::Number(size))
                        if *size >= 0. && (*size as f32).is_finite()),
                    &field(&r.source, "LOD size"),
                )?;
            }
        }
        let collision = checked_record(
            &record_map,
            &self.collision_rules.source,
            &["PhysicsManager"],
        )?;
        let rows = match collision.fields.get("m_LayerCollisionMatrix") {
            Some(WorldValue::Array(rows)) => rows,
            _ => return Err(ReferenceError("Missing source collision matrix".into())),
        };
        require(rows.len() == 32 && rows.iter().zip(self.collision_rules.layers).all(|(value, row)|
            matches!(value, WorldValue::Integer(value) if *value == row as i64))
            && self.receipts.iter().any(|receipt| receipt.source == self.collision_rules.source
                && receipt.sha256 == self.collision_rules.record_sha256), "source collision matrix receipt and rows")?;
        require(
            self.prototypes
                .iter()
                .all(|prototype| prototype.nodes.iter().all(|node| node.layer < 32)),
            "source prototype collision layers",
        )?;
        self.lod.validate(&record_map)?;
        for binding in &self.painting_materials {
            checked_record(&record_map, &binding.material, &["Material"])?;
            require(
                self.receipts.iter().any(|r| r.source == binding.texture),
                "Painting texture receipt",
            )?;
            require(
                manifest.rendering.materials.iter().any(|m| {
                    m.asset == binding.asset
                        && m.gltf_material == 0
                        && m.source == binding.material
                        && m.properties
                            .textures
                            .iter()
                            .any(|t| t.slot == "_MainTex" && t.source == binding.texture)
                }),
                "Painting source material and texture assignment",
            )?;
        }
        for binding in &self.initial_materials {
            binding.validate(&record_map)?;
        }
        self.grass_displacement
            .validate(&record_map, &self.receipts)?;
        let mut hole_components = HashSet::new();
        for hole in &self.startup_holes {
            let component = checked_record(&record_map, &hole.component, &["digholescript2"])?;
            let filter = checked_record(&record_map, &hole.filter, &["MeshFilter"])?;
            require(
                hole_components.insert(&hole.component)
                    && record_link(component, "m_GameObject")
                        == record_link(filter, "m_GameObject")
                    && record_link(filter, "m_Mesh") == Some(&hole.mesh)
                    && self
                        .receipts
                        .iter()
                        .any(|r| r.source == hole.mesh && r.sha256 == hole.mesh_sha256)
                    && hole.vertices.iter().all(|v| finite(v)),
                &field(&hole.component, "startup hole mesh"),
            )?;
        }
        for component in self.records.iter().filter(|r| {
            r.class_name == "digholescript2"
                && r.source.file == "level1"
                && matches!(r.source.path_id, 68463 | 77620)
        }) {
            require(
                hole_components.contains(&component.source),
                &field(&component.source, "required startup hole mesh"),
            )?;
        }
        for s in [
            &self.road.source,
            &self.terrain.source,
            &self.buildings.source,
        ] {
            require(records.contains(s), &field(s, "required world input"))?;
        }
        require(
            self.distances.get("drawDistance") == Some(&500.)
                && self.distances.values().all(|v| v.is_finite()),
            "world minimum draw distances",
        )?;
        for name in [
            "TerrainGenDist",
            "TerrainRemoveDist",
            "DistantTerrainGenDist",
            "DistantTerrainRemoveDist",
            "DistantTerrain2GenDist",
            "DistantTerrain2RemoveDist",
            "RoadGenDist",
            "BuildingGenDist",
            "BuildingRemoveDist",
            "ObjGenDist",
            "ObjRemoveDist",
            "MountainGenDist",
            "MountainRemoveDist",
            "ItemSpawnDist",
            "ItemRemoveDist",
            "ItemFreezeDist",
            "ItemUnFreezeDist",
        ] {
            require(
                self.distances.get(name).is_some_and(|v| *v > 0.),
                &format!("world distances.{name}"),
            )?;
        }
        let r = &self.road;
        require(
            finite(&[
                r.length,
                r.road_bone_length,
                r.road_segment_length,
                r.road_above_terrain,
                r.width,
                r.scale,
                r.xmax_angle,
                r.ymax_angle,
                r.straight_modifier,
                r.def_point_range,
                r.def_point_step,
            ]) && r.length > 0.
                && r.gen_at_start == 100
                && r.gen_in_one_frame == 1
                && r.road_bone_num > 0
                && r.road_bone_length > 0.
                && r.road_segment_length == r.road_bone_num as f32 * r.road_bone_length
                && r.length % r.road_segment_length == 0.
                && r.width > 0.
                && r.scale > 0.
                && r.prototypes.len() == 3
                && r.noises.len() == 2
                && [
                    r.align_edge_of_the_world,
                    r.aliign_road,
                    r.align_buildings,
                    r.force_homesick,
                ]
                .iter()
                .all(|v| *v <= 1),
            &field(&r.source, "road inputs"),
        )?;
        for n in &r.noises {
            require(
                finite(&[n.amplitude, n.frequency, n.lacunarity, n.persistence])
                    && n.octaves > 0
                    && n.octaves <= 32
                    && n.frequency > 0.,
                &field(&r.source, "noises"),
            )?;
        }
        let t = &self.terrain;
        require(
            t.tiles.len() == 3
                && t.prefabs.len() == 3
                && t.materials.len() == 3
                && t.prefabs.iter().all(|s| receipts.contains(s))
                && records.contains(&t.game_settings.source)
                && t.random_seed <= 1
                && finite(&[
                    t.edge_of_world,
                    t.edge_of_world_gradient,
                    t.edge_of_world_generation_dist,
                    t.edge_of_world_warp_dist,
                    t.edge_of_world_warp_offset,
                ])
                && t.edge_of_world > 0.
                && t.edge_of_world_gradient > 0.,
            &field(&t.source, "terrain inputs"),
        )?;
        for (tier, prefab) in t.prefabs.iter().enumerate() {
            let transform = checked_record(&record_map, prefab, &["Transform"])?;
            let owner = record_link(transform, "m_GameObject")
                .ok_or_else(|| ReferenceError(format!("Terrain prefab {tier} owner")))?;
            let object = checked_record(&record_map, owner, &["GameObject"])?;
            require(
                receipts.contains(owner)
                    && matches!(object.fields.get("m_Layer"),
                Some(WorldValue::Integer(layer)) if (0..32).contains(layer)),
                "terrain source collision layer",
            )?;
        }
        let resources: Vec<_> = t
            .materials
            .iter()
            .chain(self.prototypes.iter().flat_map(|p| &p.deferred_materials))
            .flat_map(|m| {
                [
                    (&m.shader.path, &m.shader.sha256),
                    (&m.shader.compiled_data.path, &m.shader.compiled_data.sha256),
                    (&m.source_data.path, &m.source_data.sha256),
                ]
            })
            .chain(
                t.textures
                    .iter()
                    .chain(&self.material_textures)
                    .flat_map(|x| {
                        [
                            (&x.path, &x.sha256),
                            (&x.mip_data.path, &x.mip_data.sha256),
                            (&x.source_data.path, &x.source_data.sha256),
                        ]
                        .into_iter()
                        .chain(x.chunks.iter().map(|c| (&c.data.path, &c.data.sha256)))
                    }),
            )
            .collect();
        for (path, digest) in resources {
            require(
                hash(digest)
                    && manifest
                        .assets
                        .iter()
                        .flat_map(|a| &a.dependencies)
                        .any(|d| &d.file == path && &d.sha256 == digest),
                "world terrain resource receipt",
            )?;
        }
        for material in &t.materials {
            require(
                receipts.contains(&material.source)
                    && material.shader.name == "TerrainBiome"
                    && source_valid(&material.shader.source, &self.source_hashes)
                    && material.floats.values().all(|v| v.is_finite())
                    && material.colors.values().all(|v| finite(v)),
                &field(&material.source, "terrain material"),
            )?;
            for link in &material.textures {
                require(
                    t.textures.iter().any(|x| x.id == link.texture)
                        && finite(&link.scale)
                        && finite(&link.offset),
                    &field(&material.source, &link.property),
                )?;
            }
        }
        for prototype in &self.prototypes {
            for material in &prototype.deferred_materials {
                require(
                    source_valid(&material.source, &self.source_hashes)
                        && !material.shader.name.is_empty()
                        && material.floats.values().all(|v| v.is_finite())
                        && material.colors.values().all(|v| finite(v)),
                    &field(&material.source, "deferred source shader"),
                )?;
                for link in &material.textures {
                    require(
                        self.material_textures.iter().any(|x| x.id == link.texture),
                        &field(&material.source, &link.property),
                    )?;
                }
            }
        }
        for material in t
            .materials
            .iter()
            .chain(self.prototypes.iter().flat_map(|p| &p.deferred_materials))
        {
            require(
                material
                    .valid_keywords
                    .iter()
                    .all(|keyword| !keyword.is_empty())
                    && material
                        .valid_keywords
                        .iter()
                        .enumerate()
                        .all(|(i, keyword)| !material.valid_keywords[..i].contains(keyword)),
                &field(&material.source, "source material keywords"),
            )?;
            require(
                material
                    .texture_coordinates
                    .iter()
                    .all(|(name, st)| !name.is_empty() && finite(st)),
                &field(&material.source, "texture coordinates"),
            )?;
            if material.shader.name == "TerrainBiome" {
                require(
                    material.texture_coordinates.contains_key("_texcoord"),
                    &field(&material.source, "_texcoord_ST"),
                )?;
            }
        }
        for target in t
            .materials
            .iter()
            .chain(self.prototypes.iter().flat_map(|p| &p.deferred_materials))
            .flat_map(|m| &m.render_targets)
        {
            require(
                records.contains(&target.source)
                    && !target.property.is_empty()
                    && finite(&target.scale)
                    && finite(&target.offset),
                &field(&target.source, "source render target"),
            )?;
        }
        for texture in t.textures.iter().chain(&self.material_textures) {
            require(
                source_valid(&texture.source, &self.source_hashes)
                    && texture.width > 0
                    && texture.height > 0
                    && texture.mip_count > 0
                    && texture.mip_bias.is_finite(),
                &field(&texture.source, "terrain texture"),
            )?;
        }
        for tile in &t.tiles {
            require(
                tile.tile_size > 0
                    && tile.resolution >= 2
                    && tile.resolution <= 1024
                    && tile.draw_distance > 0
                    && tile.y_coord.is_finite(),
                &field(&t.source, "tile"),
            )?;
        }
        let prototypes: HashSet<_> = self.prototypes.iter().map(|p| &p.source).collect();
        let assets: HashSet<_> = self.prototypes.iter().map(|p| p.asset).collect();
        require(
            !prototypes.is_empty()
                && prototypes.len() == self.prototypes.len()
                && assets.len() == self.prototypes.len(),
            "world prototype identity",
        )?;
        for (list, role) in [(&r.prototypes, WorldPrototypeRole::Road)] {
            for s in list {
                require(
                    self.prototypes
                        .iter()
                        .any(|p| p.source == *s && p.role == role),
                    &field(s, "prototype selection"),
                )?;
            }
        }
        for p in &self.prototypes {
            self.validate_prototype(p, manifest, &record_map, &receipts)?;
        }
        require(
            manifest
                .assets
                .iter()
                .filter(|a| a.usage == AssetUsage::WorldPrototype)
                .count()
                == assets.len(),
            "world unselected prototype asset",
        )?;
        for g in &self.generators {
            require(
                records.contains(&g.source)
                    && !g.types.is_empty()
                    && finite(&[g.no_spawn_chance, g.point_dist, g.chunk_dist, g.height_add])
                    && g.no_spawn_chance >= 0.
                    && g.point_dist > 0.
                    && g.chunk_dist > 0.,
                &field(&g.source, "object generator"),
            )?;
            require(
                [
                    g.affected_by_road,
                    g.affects_terrain,
                    g.align_to_building,
                    g.align_to_road,
                    g.buildingspawn,
                    g.wait_for_terrain,
                ]
                .iter()
                .all(|v| *v <= 1),
                &field(&g.source, "generator flags"),
            )?;
            for choice in &g.types {
                require(
                    choice
                        .prototype
                        .as_ref()
                        .is_none_or(|p| prototypes.contains(p))
                        && finite(&[
                            choice.chance,
                            choice.max_dist_from_road,
                            choice.min_dist_from_building,
                            choice.min_dist_from_road,
                        ])
                        && choice.chance >= 0.
                        && [
                            choice.in_desert,
                            choice.in_grass,
                            choice.in_winter,
                            choice.affected_by_road,
                        ]
                        .iter()
                        .all(|v| *v <= 1),
                    &field(&g.source, "objTypes"),
                )?;
            }
        }
        require(
            self.generators.len() == 4
                && self
                    .generators
                    .iter()
                    .map(|g| &g.source)
                    .collect::<HashSet<_>>()
                    .len()
                    == 4,
            "world selected generators",
        )?;
        let b = &self.buildings;
        require(
            !b.groups.is_empty()
                && !b.default_sides.is_empty()
                && finite(&[b.min_start_dist, b.max_start_dist, b.dist_modifier])
                && b.min_start_dist <= b.max_start_dist
                && b.dist_modifier > 0.,
            &field(&b.source, "building inputs"),
        )?;
        let types: HashSet<_> = b.types.iter().map(|v| v.type_id).collect();
        require(
            types.len() == b.types.len()
                && b.types.iter().all(|v| prototypes.contains(&v.prototype)),
            &field(&b.source, "building type links"),
        )?;
        for group in &b.groups {
            require(
                group.min_count > 0
                    && group.min_count <= group.max_count
                    && !group.types.is_empty(),
                &field(&b.source, "groups"),
            )?;
            for choice in &group.types {
                require(
                    types.contains(&choice.type_id)
                        && finite(&[choice.min_dist, choice.max_dist, choice.x_dist_modifier])
                        && choice.min_dist <= choice.max_dist
                        && choice.min_dist > 0.
                        && choice.dont_use_default <= 1
                        && choice.use_modifier <= 1
                        && (choice.dont_use_default == 0 || !choice.sides.is_empty()),
                    &field(&b.source, "groups.types"),
                )?;
            }
        }
        for side in b.default_sides.iter().chain(
            b.groups
                .iter()
                .flat_map(|g| g.types.iter().flat_map(|t| &t.sides)),
        ) {
            require(
                finite(&[
                    side.min_xpos,
                    side.max_xpos,
                    side.min_y_angle,
                    side.max_y_angle,
                ]) && side.min_xpos <= side.max_xpos
                    && side.min_y_angle <= side.max_y_angle,
                &field(&b.source, "sides"),
            )?;
        }
        require(!self.startup.is_empty(), "world startup frames")?;
        for frame in &self.startup {
            require(
                source_valid(&frame.source, &self.source_hashes) && transform(&frame.pose),
                &field(&frame.source, "startup pose"),
            )?;
            let mut node = checked_record(&record_map, &frame.source, &["Transform"])?;
            let mut ancestors = HashSet::new();
            while let Some(parent) = record_link(node, "m_Father") {
                require(
                    ancestors.insert(&node.source),
                    &field(&frame.source, "startup transform cycle"),
                )?;
                node = checked_record(&record_map, parent, &["Transform"])?;
            }
        }
        require(
            records.contains(&self.catalog.source),
            &field(&self.catalog.source, "catalog source"),
        )?;
        for slot in &self.part_scope_slots {
            require(
                records.contains(&slot.source) && !slot.part_types.is_empty(),
                &field(&slot.source, "supported part slot"),
            )?;
        }
        for (i, item) in self.catalog.items.iter().enumerate() {
            require(
                item.index == i as u32
                    && !item.field.is_empty()
                    && item
                        .compatible_slots
                        .iter()
                        .all(|s| self.part_scope_slots.iter().any(|slot| &slot.source == s))
                    && !item.name.is_empty()
                    && receipts.contains(&item.source)
                    && item
                        .prototype
                        .as_ref()
                        .is_none_or(|p| prototypes.contains(p))
                    && (item.prototype.is_some() || !item.exclusion.is_empty()),
                &field(&item.source, "catalog target/scope"),
            )?;
        }
        for (i, c) in self.catalog.categories.iter().enumerate() {
            let label = format!("level1:66024 categories[{i}]");
            require(
                c.index == i as u32
                    && !c.name.is_empty()
                    && c.items
                        .as_ref()
                        .is_none_or(|v| v.iter().all(|n| (*n as usize) < self.catalog.items.len()))
                    && c.invalid_category_indices
                        == c.categories
                            .iter()
                            .flatten()
                            .copied()
                            .filter(|n| (*n as usize) >= self.catalog.categories.len())
                            .collect::<Vec<_>>(),
                &label,
            )?;
            require(
                c.initialized == (c.items.is_some() || c.categories.is_some()),
                &label,
            )?;
            for weights in [&c.item_chances, &c.category_chances] {
                require(
                    weights
                        .as_ref()
                        .is_none_or(|v| !v.is_empty() && v.iter().all(|n| *n > 0)),
                    &label,
                )?;
            }
            if let (Some(weights), Some(items)) = (&c.item_chances, &c.items) {
                require(weights.len() == items.len(), &label)?;
            }
            if let Some(weights) = &c.category_chances {
                require(
                    weights.len()
                        == c.categories
                            .as_ref()
                            .or(c.items.as_ref())
                            .map_or(0, |a| a.len()),
                    &label,
                )?;
            }
        }
        for e in &self.exclusions {
            require(
                records.contains(&e.source)
                    && !e.field.is_empty()
                    && !e.reason.is_empty()
                    && e.target.as_ref().is_none_or(|s| receipts.contains(s)),
                &field(&e.source, "scope exclusion"),
            )?;
        }
        Ok(())
    }

    fn validate_prototype(
        &self,
        p: &WorldPrototypeReference,
        manifest: &ReferenceManifest,
        records: &HashMap<&SourceId, &WorldSourceRecord>,
        receipts: &HashSet<&SourceId>,
    ) -> Result<(), ReferenceError> {
        let f = field(&p.source, "prototype");
        let a = manifest
            .assets
            .iter()
            .find(|a| a.id == p.asset)
            .ok_or_else(|| ReferenceError(format!("Missing {f} asset")))?;
        require(
            a.usage == AssetUsage::WorldPrototype && a.root == p.source,
            &f,
        )?;
        require(
            !p.nodes.is_empty() && p.nodes[0].source == p.source && transform(&p.prefab_pose),
            &f,
        )?;
        let sources: HashSet<_> = p.nodes.iter().map(|n| &n.source).collect();
        require(
            sources.len() == p.nodes.len(),
            &field(&p.source, "duplicate node source"),
        )?;
        let mut objects = Vec::with_capacity(p.nodes.len());
        for (i, n) in p.nodes.iter().enumerate() {
            require(
                n.layer < 32
                    && n.gltf_node == i as u32
                    && receipts.contains(&n.source)
                    && finite(&n.transform.translation)
                    && finite(&n.transform.scale)
                    && finite(&n.transform.rotation)
                    && (n.transform.rotation.iter().map(|v| v * v).sum::<f32>() - 1.).abs() < 0.002
                    && n.parent.is_none_or(|v| v < i as u32),
                &field(&n.source, "prototype node ownership"),
            )?;
            let node = checked_record(records, &n.source, &["Transform", "RectTransform"])?;
            let object_id = record_link(node, "m_GameObject")
                .ok_or_else(|| ReferenceError(field(&n.source, "m_GameObject")))?;
            let object = checked_record(records, object_id, &["GameObject"])?;
            require(
                component_listed(object, &n.source),
                &field(&n.source, "GameObject Transform ownership"),
            )?;
            objects.push(object);
            if let Some(parent) = n.parent {
                require(
                    record_link(node, "m_Father") == Some(&p.nodes[parent as usize].source),
                    &field(&n.source, "m_Father"),
                )?;
                let parent = checked_record(
                    records,
                    &p.nodes[parent as usize].source,
                    &["Transform", "RectTransform"],
                )?;
                require(
                    matches!(parent.fields.get("m_Children"), Some(WorldValue::Array(children)) if children.iter().any(|v| matches!(v, WorldValue::Link(source) if source == &n.source))),
                    &field(&n.source, "parent m_Children"),
                )?;
            } else {
                require(
                    i == 0
                        && matches!(
                            node.fields.get("m_Father"),
                            Some(WorldValue::Empty) | Some(WorldValue::Link(_))
                        )
                        && record_link(node, "m_Father")
                            .is_none_or(|parent| !sources.contains(parent)),
                    &field(&n.source, "prototype root parent"),
                )?;
            }
        }
        // Every source component needs type and owner evidence before selecting a body.
        let mut source_bodies = HashMap::new();
        for (index, object) in objects.iter().enumerate() {
            for source in object.components() {
                let component = records.get(source).ok_or_else(|| {
                    ReferenceError(field(source, "missing source component type/owner record"))
                })?;
                require(
                    record_link(component, "m_GameObject") == Some(&object.source),
                    &field(source, "source component GameObject ownership"),
                )?;
                if component.class_name == "Rigidbody" {
                    require(
                        source_bodies
                            .insert(index as u32, &component.source)
                            .is_none(),
                        &field(&object.source, "unique source Rigidbody"),
                    )?;
                }
            }
        }
        require(
            p.controls.iter().all(|i| (*i as usize) < p.nodes.len())
                && p.controls.iter().collect::<HashSet<_>>().len() == p.controls.len(),
            &field(&p.source, "road controls"),
        )?;
        if p.role == WorldPrototypeRole::Road {
            require(
                p.controls.len() == self.road.road_bone_num as usize && !p.skins.is_empty(),
                &field(&p.source, "road controls/skin"),
            )?;
        }
        let road_rules: Vec<_> = objects
            .iter()
            .flat_map(|object| object.components())
            .filter_map(|source| records.get(source).copied())
            .filter(|record| record.class_name == "roadscript")
            .collect();
        if p.role == WorldPrototypeRole::Road {
            require(
                road_rules.len() == 1
                    && node_links_match(road_rules[0], "bones", &p.controls, &p.nodes),
                &field(&p.source, "source road controls"),
            )?;
        }
        let mut skin_sources = HashSet::new();
        for skin in &p.skins {
            let f = field(&skin.source, "skin bindings/weights");
            let count = skin.positions.len();
            require(
                receipts.contains(&skin.source)
                    && receipts.contains(&skin.mesh)
                    && skin_sources.insert(&skin.source)
                    && (skin.node as usize) < p.nodes.len()
                    && !skin.bones.is_empty()
                    && skin.bones.len() == skin.bind_poses.len()
                    && skin.bones.iter().all(|i| (*i as usize) < p.nodes.len())
                    && skin.bones.iter().collect::<HashSet<_>>().len() == skin.bones.len()
                    && skin.bind_poses.iter().all(|v| finite(v))
                    && count > 0
                    && skin.normals.len() == count
                    && (skin.tangents.is_empty() || skin.tangents.len() == count)
                    && skin.uv.len() == count
                    && skin.joints.len() == count
                    && skin.weights.len() == count,
                &f,
            )?;
            let renderer = checked_record(records, &skin.source, &["SkinnedMeshRenderer"])?;
            let object = objects[skin.node as usize];
            require(
                record_link(renderer, "m_GameObject") == Some(&object.source)
                    && component_listed(object, &skin.source)
                    && record_link(renderer, "m_Mesh") == Some(&skin.mesh)
                    && node_links_match(renderer, "m_Bones", &skin.bones, &p.nodes),
                &field(&skin.source, "source skin ownership/bones/mesh"),
            )?;
            require(
                skin.positions.iter().all(|v| finite(v))
                    && skin.normals.iter().all(|v| finite(v))
                    && skin.tangents.iter().all(|v| finite(v))
                    && skin.uv.iter().all(|v| finite(v)),
                &f,
            )?;
            for (j, w) in skin.joints.iter().zip(&skin.weights) {
                require(
                    finite(w)
                        && w.iter().all(|v| *v >= 0. && *v <= 1.)
                        && (w.iter().sum::<f32>() - 1.).abs() < 0.0001
                        && j.iter().all(|v| (*v as usize) < skin.bones.len()),
                    &f,
                )?;
            }
            require(
                !skin.triangles.is_empty()
                    && skin.triangles.iter().all(|g| {
                        !g.is_empty() && g.iter().flatten().all(|i| (*i as usize) < count)
                    }),
                &field(&skin.source, "skin triangles"),
            )?;
        }
        let mut body_nodes = HashMap::new();
        let mut body_sources = HashSet::new();
        for (body_index, b) in p.bodies.iter().enumerate() {
            require(
                receipts.contains(&b.source)
                    && body_sources.insert(&b.source)
                    && body_nodes.insert(b.node, body_index as u32).is_none()
                    && (b.node as usize) < p.nodes.len()
                    && transform(&b.pose)
                    && finite(&[b.mass_kg, b.linear_damping, b.angular_damping])
                    && b.mass_kg > 0.
                    && b.linear_damping >= 0.
                    && b.angular_damping >= 0.,
                &field(&b.source, "prototype body"),
            )?;
            let body = checked_record(records, &b.source, &["Rigidbody"])?;
            let object = objects[b.node as usize];
            require(
                record_link(body, "m_GameObject") == Some(&object.source)
                    && component_listed(object, &b.source),
                &field(&b.source, "source Rigidbody ownership"),
            )?;
        }
        require(
            source_bodies.len() == p.bodies.len(),
            &field(&p.source, "complete source Rigidbody bindings"),
        )?;
        for (node, source) in &source_bodies {
            require(
                body_nodes
                    .get(node)
                    .is_some_and(|body| &p.bodies[*body as usize].source == *source),
                &field(source, "required local Rigidbody"),
            )?;
        }
        let mut collider_sources = HashSet::new();
        for c in &p.colliders {
            let f = field(&c.source, "prototype collision owner");
            require(
                receipts.contains(&c.source) && collider_sources.insert(&c.source)
                    && c.material
                        .as_ref()
                        .is_none_or(|s| self.receipts.iter().any(|r| &r.source == s))
                    && c.body.is_none_or(|i| (i as usize) < p.bodies.len())
                    && (c.node as usize) < p.nodes.len()
                    && ([c.deformed_skin.is_some(), c.geometry.is_some(), c.source_empty_mesh.is_some()].into_iter().filter(|v| *v).count() == 1)
                    && c.source_empty_mesh.as_ref().is_none_or(|s| self.records.iter().any(|r| &r.source == s && r.class_name == "Mesh" && matches!(r.fields.get("m_VertexData"), Some(WorldValue::Record(v)) if matches!(v.get("m_VertexCount"),Some(WorldValue::Integer(0))))))
                    && c.deformed_skin.is_none_or(|i| (i as usize) < p.skins.len())
                    && c.role_valid(),
                &f,
            )?;
            let collider = checked_record(
                records,
                &c.source,
                &[
                    "BoxCollider",
                    "SphereCollider",
                    "CapsuleCollider",
                    "MeshCollider",
                ],
            )?;
            let object = objects[c.node as usize];
            require(
                record_link(collider, "m_GameObject") == Some(&object.source)
                    && component_listed(object, &c.source)
                    && record_link(collider, "m_Material") == c.material.as_ref(),
                &field(&c.source, "source collider ownership/material"),
            )?;
            let mut ancestor = Some(c.node);
            let mut nearest_body = None;
            while let Some(node) = ancestor {
                if let Some(source) = source_bodies.get(&node) {
                    nearest_body = Some(*source);
                    break;
                }
                ancestor = p.nodes[node as usize].parent;
            }
            require(
                c.body.map(|index| &p.bodies[index as usize].source) == nearest_body,
                &field(&c.source, "nearest source Rigidbody"),
            )?;
            if let Some(skin) = c.deformed_skin {
                require(
                    collider.class_name == "MeshCollider"
                        && matches!(collider.fields.get("m_Mesh"), Some(WorldValue::Empty))
                        && road_rules.iter().any(|road| {
                            record_link(road, "meshCollider") == Some(&c.source)
                                && record_link(road, "skinnedMeshRenderer")
                                    == Some(&p.skins[skin as usize].source)
                        }),
                    &field(&c.source, "source deformed skin owner"),
                )?;
            }
            if let Some(mesh) = &c.source_empty_mesh {
                require(
                    record_link(collider, "m_Mesh") == Some(mesh),
                    &field(&c.source, "source empty mesh owner"),
                )?;
            }
            if let Some(shape) = &c.geometry {
                require(collider_valid(shape), &f)?;
            }
        }
        let doors: Vec<_> = self
            .mechanisms
            .iter()
            .filter(|m| m.prototype == p.source)
            .map(|m| &m.recipe.source)
            .collect();
        interiors_valid(p, &doors)
    }
}
/// Interior colliders, doors and parents resolve inside the prototype.
fn interiors_valid(p: &WorldPrototypeReference, doors: &[&SourceId]) -> Result<(), ReferenceError> {
    let door = |s: &SourceId| doors.contains(&s);
    for (i, interior) in p.interiors.iter().enumerate() {
        // A parent chain longer than the list has a cycle.
        let mut parent = interior.parent;
        let mut steps = 0;
        while let Some(index) = parent
            && steps <= p.interiors.len()
        {
            parent = p.interiors.get(index as usize).and_then(|r| r.parent);
            steps += 1;
        }
        require(
            p.colliders.iter().any(|c| c.source == interior.collider)
                && interior
                    .parent
                    .is_none_or(|v| (v as usize) < p.interiors.len())
                && steps <= p.interiors.len()
                && interior.doors.iter().all(door)
                && interior.parent_door.as_ref().is_none_or(door)
                && interior.invert_doors.len() == interior.doors.len()
                && finite(&[interior.maximum_transmission, interior.value])
                && super::audio::method(&interior.method)
                && interior.method.class == "interiorscript"
                && interior.method.method == "GetValue",
            &field(&p.source, &format!("interior {i}")),
        )?;
    }
    Ok(())
}
fn collider_valid(g: &ColliderGeometry) -> bool {
    match g {
        ColliderGeometry::Box {
            center,
            half_extents,
            rotation,
        } => {
            finite(center)
                && finite(half_extents)
                && half_extents.iter().all(|v| *v > 0.)
                && finite(rotation)
        }
        ColliderGeometry::Sphere { center, radius } => {
            finite(center) && radius.is_finite() && *radius > 0.
        }
        ColliderGeometry::Capsule { a, b, radius } => {
            finite(a) && finite(b) && radius.is_finite() && *radius > 0.
        }
        ColliderGeometry::Convex { points } => {
            points.len() >= 4 && points.iter().all(|v| finite(v))
        }
        ColliderGeometry::Trimesh { points, triangles } => {
            !points.is_empty()
                && !triangles.is_empty()
                && points.iter().all(|v| finite(v))
                && triangles
                    .iter()
                    .flatten()
                    .all(|i| (*i as usize) < points.len())
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCatalogReference {
    pub source: SourceId,
    #[serde(deserialize_with = "compact_vec")]
    pub items: Vec<WorldCatalogItem>,
    #[serde(deserialize_with = "compact_vec")]
    pub categories: Vec<WorldCategory>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCatalogItem {
    pub field: String,
    #[serde(deserialize_with = "compact_vec")]
    pub compatible_slots: Vec<SourceId>,
    pub index: u32,
    pub name: String,
    pub source: SourceId,
    pub prototype: Option<SourceId>,
    pub exclusion: String,
    #[serde(deserialize_with = "compact_vec")]
    pub excluded_actions: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCategory {
    #[serde(deserialize_with = "compact_vec")]
    pub invalid_category_indices: Vec<u32>,
    pub initialized: bool,
    pub index: u32,
    pub name: String,
    pub items: Option<Vec<u32>>,
    pub categories: Option<Vec<u32>>,
    pub item_chances: Option<Vec<u32>>,
    pub category_chances: Option<Vec<u32>>,
    pub depend_on_car: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPrototypeBody {
    pub source: SourceId,
    pub node: u32,
    pub mass_kg: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub kinematic: bool,
    pub use_gravity: bool,
    pub constraints: u32,
    pub pose: Transform,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPartScopeSlot {
    pub source: SourceId,
    #[serde(deserialize_with = "compact_vec")]
    pub main_types: Vec<String>,
    #[serde(deserialize_with = "compact_vec")]
    pub part_types: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSourceMaterial {
    pub source: SourceId,
    #[serde(deserialize_with = "compact_vec")]
    pub valid_keywords: Vec<String>,
    pub shader: super::rendering::ShaderReference,
    pub source_data: super::rendering::ResourceReference,
    pub floats: BTreeMap<String, f32>,
    pub colors: BTreeMap<String, [f32; 4]>,
    #[serde(deserialize_with = "compact_vec")]
    pub textures: Vec<super::rendering::TextureLink>,
    #[serde(deserialize_with = "compact_vec")]
    pub render_targets: Vec<WorldRenderTarget>,
    pub texture_coordinates: BTreeMap<String, [f32; 4]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldRenderTarget {
    pub property: String,
    pub source: SourceId,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}

/// Source inputs are immutable after decoding; retain no deserializer growth slack.
fn compact_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    let values = Vec::<T>::deserialize(deserializer)?;
    // Input decoding precedes world jobs. This is the only live compaction buffer.
    let overlap = values
        .capacity()
        .saturating_add(values.len())
        .saturating_mul(std::mem::size_of::<T>());
    if values.capacity() != values.len() && overlap > 32 * 1024 * 1024 {
        return Err(serde::de::Error::custom(
            "Source input compaction exceeds shared pending budget",
        ));
    }
    Ok(values.into_boxed_slice().into_vec())
}

fn validate_world_libraries(
    current: &LibraryProvenance,
    capture: &LibraryProvenance,
    accepted: &LibraryProvenance,
) -> Result<(), ReferenceError> {
    require(
        current.schema_version == 1
            && current.manifest_sha256 == accepted.manifest_sha256
            && current.catalog_sha256 == accepted.catalog_sha256
            && current.source_manifest_sha256 == accepted.source_manifest_sha256,
        "world library receipt",
    )?;
    require(
        capture.schema_version == 1
            && capture.manifest_sha256
                == "92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99"
            && capture.catalog_sha256 == current.catalog_sha256
            && capture.source_manifest_sha256 == current.source_manifest_sha256,
        "world capture library receipt",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collider(trigger: bool, role: &str) -> WorldColliderReference {
        serde_json::from_str(&format!(
            r#"{{"body":null,"source":{{"file":"a","path_id":1}},"node":0,"enabled":true,"trigger":{trigger},"role":{role},"material":null,"deformed_skin":null,"source_empty_mesh":null,"geometry":null,"step_surface":"sand"}}"#
        ))
        .unwrap()
    }

    #[test]
    fn trigger_roles_round_trip_and_need_a_trigger() {
        for (role, value) in [
            ("null", None),
            (
                r#"{"ladder":{"speed":2.5,"up":[0.0,1.0,0.0]}}"#,
                Some(TriggerRole::Ladder {
                    speed: 2.5,
                    up: [0.0, 1.0, 0.0],
                }),
            ),
            (
                r#"{"hazard":{"damage_player":100.0,"damage_breakable":5.0}}"#,
                Some(TriggerRole::Hazard {
                    damage_player: 100.0,
                    damage_breakable: 5.0,
                }),
            ),
        ] {
            let c = collider(true, role);
            assert_eq!(c.role, value);
            assert_eq!(c.step_surface, Some(crate::reference::StepSurface::Sand));
            assert_eq!(
                serde_json::to_value(c.role).unwrap(),
                serde_json::from_str::<serde_json::Value>(role).unwrap()
            );
            assert!(c.role_valid());
            assert_eq!(collider(false, role).role_valid(), value.is_none());
        }
        assert!(!collider(true, r#"{"ladder":{"speed":2.5,"up":[0.0,2.0,0.0]}}"#).role_valid());
    }

    #[test]
    fn interiors_parse_and_resolve_inside_the_prototype() {
        let s = |id: u32| format!(r#"{{"file":"a","path_id":{id}}}"#);
        let method = format!(
            r#"{{"class":"interiorscript","method":"GetValue","file":"managed/Assembly-CSharp.cs","line_start":1,"line_end":2,"sha256":"{}"}}"#,
            "0".repeat(64)
        );
        let interior = |parent: &str, door: u32| {
            format!(
                r#"{{"collider":{},"parent":{parent},"invert":false,"minimum_doors_to_max":1,"maximum_transmission":1.0,"use_value":false,"value":0.0,"doors":[{}],"invert_doors":[false],"parent_door":null,"method":{method}}}"#,
                s(1),
                s(door)
            )
        };
        let prototype = |interiors: &[String]| -> WorldPrototypeReference {
            let collider = serde_json::to_string(&collider(true, "null")).unwrap();
            serde_json::from_str(&format!(
                r#"{{"source":{},"role":"structure","asset":1,"nodes":[],"controls":[],"excluded_actions":[],"deferred_materials":[],"skins":[],"bodies":[],"colliders":[{collider}],"prefab_pose":{{"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1]}},"interiors":[{}]}}"#,
                s(9),
                interiors.join(",")
            ))
            .unwrap()
        };
        let door = SourceId {
            file: "a".into(),
            path_id: 5,
        };
        let doors = [&door];
        let valid = prototype(&[interior("null", 5), interior("0", 5)]);
        assert_eq!(valid.interiors[1].parent, Some(0));
        assert!(interiors_valid(&valid, &doors).is_ok());
        for bad in [
            prototype(&[interior("null", 6)]),
            prototype(&[interior("1", 5)]),
            prototype(&[interior("1", 5), interior("0", 5)]),
            prototype(&[interior("null", 5).replace(&s(1), &s(2))]),
            prototype(&[interior("null", 5).replace("[false]", "[]")]),
            prototype(&[interior("null", 5).replace("GetValue", "Start")]),
        ] {
            assert!(interiors_valid(&bad, &doors).is_err());
        }
    }
}
