//! Checked source world inputs. Prototype node indices belong to one asset instance.
use super::{
    AssetId, AssetUsage, ColliderGeometry, LibraryProvenance, ReferenceError, ReferenceManifest,
    SourceHash, SourceId, Transform, finite, hash, require, transform,
};
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
    pub settings_context: String,
    pub generation_frame: Transform,
    pub distances: BTreeMap<String, f32>,
    pub source_hashes: Vec<SourceHash>,
    pub road: WorldRoadReference,
    pub terrain: WorldTerrainReference,
    pub buildings: WorldBuildingReference,
    pub catalog: WorldCatalogReference,
    pub part_scope_slots: Vec<WorldPartScopeSlot>,
    pub generators: Vec<WorldGeneratorReference>,
    pub startup: Vec<WorldStartupReference>,
    pub prototypes: Vec<WorldPrototypeReference>,
    pub records: Vec<WorldSourceRecord>,
    pub receipts: Vec<WorldSourceReceipt>,
    pub exclusions: Vec<WorldExclusion>,
    pub material_textures: Vec<super::rendering::LightingTextureRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPrototypeReference {
    pub source: SourceId,
    pub role: WorldPrototypeRole,
    pub asset: AssetId,
    pub nodes: Vec<WorldPrototypeNode>,
    pub controls: Vec<u32>,
    pub excluded_actions: Vec<String>,
    pub deferred_materials: Vec<WorldSourceMaterial>,
    pub skins: Vec<WorldSkinReference>,
    pub bodies: Vec<WorldPrototypeBody>,
    pub colliders: Vec<WorldColliderReference>,
    pub prefab_pose: Transform,
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
    pub bones: Vec<u32>,
    pub bind_poses: Vec<[f32; 16]>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub joints: Vec<[u32; 4]>,
    pub weights: Vec<[f32; 4]>,
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
    pub material: Option<SourceId>,
    pub deformed_skin: Option<u32>,
    pub source_empty_mesh: Option<SourceId>,
    pub geometry: Option<ColliderGeometry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSourceReceipt {
    pub source: SourceId,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSourceLink {
    pub field: String,
    pub target: SourceId,
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
    Array(Vec<WorldValue>),
    Record(BTreeMap<String, WorldValue>),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSourceRecord {
    pub source: SourceId,
    pub class_name: String,
    pub fields: BTreeMap<String, WorldValue>,
    pub links: Vec<WorldSourceLink>,
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
    pub source: SourceId,
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
    pub noises: Vec<WorldNoiseReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldTerrainReference {
    pub source: SourceId,
    pub prefabs: Vec<SourceId>,
    pub materials: Vec<WorldSourceMaterial>,
    pub textures: Vec<super::rendering::LightingTextureRecord>,
    pub game_settings: WorldGameSettings,
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
    pub types: Vec<WorldBuildingType>,
    pub groups: Vec<WorldBuildingGroup>,
    #[serde(rename = "defaultSides")]
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
    pub available: Vec<i32>,
    #[serde(rename = "currentAvailable")]
    pub current_available: Vec<i32>,
    #[serde(rename = "currentCount")]
    pub current_count: i32,
    #[serde(rename = "currentMaxCount")]
    pub current_max_count: i32,
    #[serde(rename = "maxCount")]
    pub max_count: i32,
    #[serde(rename = "minCount")]
    pub min_count: i32,
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
fn links(value: &WorldValue, path: String, result: &mut Vec<(String, SourceId)>) {
    match value {
        WorldValue::Link(s) => result.push((path, s.clone())),
        WorldValue::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                links(v, format!("{path}[{i}]"), result);
            }
        }
        WorldValue::Record(a) => {
            for (k, v) in a {
                links(v, format!("{path}.{k}"), result);
            }
        }
        _ => {}
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
    record
        .links
        .iter()
        .any(|link| link.field.starts_with("m_Component[") && &link.target == source)
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
            self.settings_context == "saved_default_5000_frozen_generation_frame"
                && transform(&self.generation_frame),
            "world source context/frame",
        )?;
        require(
            self.library.schema_version == 1
                && self.library.manifest_sha256 == manifest.library.manifest_sha256
                && self.library.catalog_sha256 == manifest.library.catalog_sha256
                && self.library.source_manifest_sha256 == manifest.library.source_manifest_sha256,
            "world library receipt",
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
            for (k, v) in &r.fields {
                require(values_valid(v, &receipts), &field(&r.source, k))?;
            }
            let mut expected = vec![];
            for (k, v) in &r.fields {
                links(v, k.clone(), &mut expected);
            }
            require(
                expected.len() == r.links.len()
                    && expected
                        .iter()
                        .all(|(k, v)| r.links.iter().any(|l| l.field == *k && l.target == *v)),
                &field(&r.source, "resolved links"),
            )?;
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
                _ => &[],
            };
            for name in required {
                require(r.fields.contains_key(*name), &field(&r.source, name))?;
            }
        }
        for s in [
            &self.road.source,
            &self.terrain.source,
            &self.buildings.source,
        ] {
            require(records.contains(s), &field(s, "required world input"))?;
        }
        require(
            self.distances.get("drawDistance") == Some(&5000.)
                && self.distances.values().all(|v| v.is_finite()),
            "world saved-default distances",
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
            .flat_map(|object| object.links.iter())
            .filter(|link| link.field.starts_with("m_Component["))
            .filter_map(|link| records.get(&link.target).copied())
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
        for (index, object) in objects.iter().enumerate() {
            for component in object
                .links
                .iter()
                .filter(|link| link.field.starts_with("m_Component["))
            {
                if records
                    .get(&component.target)
                    .is_some_and(|r| r.class_name == "Rigidbody")
                {
                    require(
                        body_nodes.get(&(index as u32)).is_some_and(|body| {
                            p.bodies[*body as usize].source == component.target
                        }),
                        &field(&component.target, "required local Rigidbody"),
                    )?;
                }
            }
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
                    && c.deformed_skin.is_none_or(|i| (i as usize) < p.skins.len()),
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
                if let Some(body) = body_nodes.get(&node) {
                    nearest_body = Some(*body);
                    break;
                }
                ancestor = p.nodes[node as usize].parent;
            }
            require(
                c.body == nearest_body,
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
        Ok(())
    }
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
    pub items: Vec<WorldCatalogItem>,
    pub categories: Vec<WorldCategory>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCatalogItem {
    pub field: String,
    pub compatible_slots: Vec<SourceId>,
    pub index: u32,
    pub name: String,
    pub source: SourceId,
    pub prototype: Option<SourceId>,
    pub exclusion: String,
    pub excluded_actions: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCategory {
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
    pub main_types: Vec<String>,
    pub part_types: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSourceMaterial {
    pub source: SourceId,
    pub shader: super::rendering::ShaderReference,
    pub source_data: super::rendering::ResourceReference,
    pub floats: BTreeMap<String, f32>,
    pub colors: BTreeMap<String, [f32; 4]>,
    pub textures: Vec<super::rendering::TextureLink>,
    pub render_targets: Vec<WorldRenderTarget>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldRenderTarget {
    pub property: String,
    pub source: SourceId,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}
