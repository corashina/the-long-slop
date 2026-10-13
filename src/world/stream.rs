//! Bounded source generation. Publication uses absolute geometry and the current physics origin.
use super::{
    AbsolutePose, Placement, PlacementIndex, RoadIndex, RoadSegment, RoadSequence, StartupTerrain,
    TerrainContext, TerrainEvaluator, TerrainFoundation, TerrainRoadPoint, TerrainSurfaceJob,
    TerrainTile, WorldBounds, WorldLayer, WorldOrigin, WorldSurface, deform_road,
};
use crate::reference::{ReferenceError, ReferenceManifest, SourceId, WorldReference};
use crate::vehicle::{ColliderHandle, PhysicsWorld};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

const MIB: usize = 1024 * 1024;
const MAX_ROADS: usize = 512;
const MAX_NEAR: usize = 512;
const MAX_DISTANT: usize = 256;
const MAX_OBJECTS: usize = 8192;
const ACTIVE_CPU: usize = 128 * MIB;
const ACTIVE_GPU: usize = 128 * MIB;
const PROTOTYPE_CPU: usize = 256 * MIB;
const PROTOTYPE_GPU: usize = 512 * MIB;
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldRequest {
    pub session: u64,
    pub revision: u64,
    pub player: [f64; 3],
    pub vehicle: [f64; 3],
}
#[derive(Clone, Copy, Debug, Default)]
pub struct WorldMetrics {
    pub road_segments: usize,
    pub near_tiles: usize,
    pub distant_tiles: [usize; 2],
    pub generated_objects: usize,
    pub pending_work: usize,
    pub pending_bytes: usize,
    pub active_cpu_bytes: usize,
    pub active_gpu_bytes: usize,
    pub prototype_cpu_bytes: usize,
    pub prototype_gpu_bytes: usize,
    pub checkpoint_bytes: usize,
    pub changed_record_bytes: usize,
    pub changed_objects: usize,
    pub prepared_roads: u32,
    pub prepared_buildings: u32,
    pub indices_ready: bool,
    pub last_road_work: usize,
    pub last_terrain_work: usize,
}
/// Renderer-owned allocations, including accepted/pending overlap and loader staging.
#[derive(Clone, Copy, Debug, Default)]
pub struct PresentationAllocation {
    pub active_cpu: usize,
    pub active_gpu: usize,
    pub prototype_cpu: usize,
    pub prototype_gpu: usize,
    pub jobs: usize,
    pub pending: usize,
    pub staging_bytes: usize,
    pub objects: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceKey {
    pub layer: WorldLayer,
    pub unit: [i64; 2],
}
pub struct PublishedSurface {
    pub key: SurfaceKey,
    pub surface: WorldSurface,
    pub prototype: Option<SourceId>,
    pub bounds: WorldBounds,
    collider: Option<ColliderHandle>,
    collision_bytes: usize,
    road: Option<RoadSegment>,
}
pub struct WorldFrame<'a> {
    pub request: WorldRequest,
    pub origin: WorldOrigin,
    pub surfaces: &'a [PublishedSurface],
    pub lights: &'a [super::WorldLight],
    pub structures: &'a [super::StructureInstance],
    pub supplies: &'a [PublishedSupply],
    pub startup: Option<&'a StartupTerrain>,
    pub metrics: WorldMetrics,
    pub failure: Option<&'a str>,
}
pub struct PublishedSupply {
    pub key: super::GeneratedId,
    pub point: super::SupplyPoint,
    pub definition: super::SupplyDefinition,
    pub items: Option<(crate::reference::ItemId, crate::reference::ItemId)>,
    baselines: Option<[super::ObjectSnapshot; 2]>,
}
#[derive(Clone, Copy)]
struct PendingRecovery {
    request: WorldRequest,
    target: AbsolutePose,
    supply: Option<(u32, usize)>,
    candidate: usize,
    best: Option<(f64, AbsolutePose)>,
}
pub struct RecoveryPermit {
    request: WorldRequest,
    player: [f64; 3],
    vehicle: [f64; 3],
}
struct PreparedSurface {
    renderer_ready: bool,
    renderer_reserved: bool,
    request: WorldRequest,
    published: PublishedSurface,
}
struct TerrainWork {
    request: WorldRequest,
    key: SurfaceKey,
    job: TerrainSurfaceJob,
    surface: WorldSurface,
    foundations: Vec<TerrainFoundation>,
    holes: Vec<[[f32; 3]; 4]>,
    roads: Vec<TerrainRoadPoint>,
}
#[derive(Clone, Copy)]
struct PrototypeAllocation {
    cpu: usize,
    gpu: usize,
    ready: bool,
}

/// Source world publication and session ownership.
pub struct WorldStream {
    reference: Arc<WorldReference>,
    presentation: PresentationAllocation,
    changes: super::ChangeRecords,
    road_light: super::source_lights::SourceRoadLight,
    lights: Vec<super::WorldLight>,
    wanted_lights: Vec<u32>,
    pub origin: WorldOrigin,
    request: WorldRequest,
    recovery: Option<PendingRecovery>,
    roads: RoadIndex,
    placements: PlacementIndex,
    evaluator: TerrainEvaluator,
    color_foundations: Vec<super::FoundationPoint>,
    color_helpers: Vec<TerrainFoundation>,
    tiles: [TerrainTile; 3],
    terrain_layer: u8,
    road_layers: [u8; 3],
    indices_ready: bool,
    plan_revision: u64,
    road_candidates: Vec<u32>,
    next_road: usize,
    wanted_roads: Vec<u32>,
    wanted_tiles: Vec<SurfaceKey>,
    road_points: Vec<TerrainRoadPoint>,
    structures: Vec<super::StructureInstance>,
    recipes: HashMap<SourceId, super::StructureRecipe>,
    supplies: Vec<PublishedSupply>,
    wanted_structures: Vec<Placement>,
    ready_structures: Vec<super::GeneratedId>,
    startup: Option<StartupTerrain>,
    startup_candidates: Vec<u32>,
    startup_next: usize,
    startup_points: Vec<TerrainRoadPoint>,
    surfaces: Vec<PublishedSurface>,
    ready: Option<PreparedSurface>,
    terrain_work: Option<TerrainWork>,
    admissions: HashMap<SourceId, PrototypeAllocation>,
    terrain_ready: [bool; 3],
    shared_cpu_bytes: Option<usize>,
    publication_available: bool,
    failure: Option<String>,
    last_road_work: usize,
    last_terrain_work: usize,
}

impl WorldStream {
    pub fn new(reference: &ReferenceManifest, seed: i32) -> Result<Self, ReferenceError> {
        let world = &reference.world;
        let shared_cpu_bytes = reference.world_retained_cpu_bytes();
        if shared_cpu_bytes > PROTOTYPE_CPU {
            return Err(ReferenceError(format!(
                "Source immutable graph exceeds prototype CPU budget: {shared_cpu_bytes}"
            )));
        }
        let evaluator = TerrainEvaluator::new(world, seed);
        let roads = RoadIndex::new(
            RoadSequence::new(world, seed)?,
            TerrainEvaluator::new(world, seed),
        );
        let terrain_transform = world
            .records
            .iter()
            .find(|record| record.source == world.terrain.prefabs[0])
            .ok_or_else(|| ReferenceError("Missing terrain prefab Transform".into()))?;
        let Some(crate::reference::WorldValue::Link(owner)) =
            terrain_transform.fields.get("m_GameObject")
        else {
            return Err(ReferenceError("Missing terrain prefab owner".into()));
        };
        let object = world
            .records
            .iter()
            .find(|record| &record.source == owner)
            .ok_or_else(|| ReferenceError("Missing terrain prefab GameObject".into()))?;
        let Some(crate::reference::WorldValue::Integer(terrain_layer)) =
            object.fields.get("m_Layer")
        else {
            return Err(ReferenceError("Missing terrain collision layer".into()));
        };
        let mut road_layers = [0; 3];
        for (index, source) in world.road.prototypes.iter().enumerate() {
            let prototype = world
                .prototypes
                .iter()
                .find(|prototype| &prototype.source == source)
                .unwrap();
            let collider = prototype
                .colliders
                .iter()
                .find(|collider| collider.deformed_skin.is_some())
                .ok_or_else(|| ReferenceError("Missing source road skin collider".into()))?;
            if collider.material.is_some() || collider.trigger || !collider.enabled {
                return Err(ReferenceError(
                    "Source road skin requires physical material binding".into(),
                ));
            }
            road_layers[index] = prototype.nodes[collider.node as usize].layer;
        }
        Ok(Self {
            presentation: PresentationAllocation::default(),
            terrain_layer: *terrain_layer as u8,
            road_layers,
            reference: Arc::clone(world),
            changes: super::ChangeRecords::default(),
            road_light: super::source_lights::SourceRoadLight::new(reference, seed)?,
            lights: Vec::with_capacity(MAX_ROADS),
            wanted_lights: Vec::with_capacity(MAX_ROADS),
            origin: WorldOrigin::default(),
            request: WorldRequest {
                session: NEXT_SESSION.fetch_add(1, Ordering::Relaxed),
                revision: 1,
                player: [0.0; 3],
                vehicle: [0.0; 3],
            },
            recovery: None,
            roads,
            placements: PlacementIndex::new(world, seed),
            evaluator,
            color_foundations: Vec::new(),
            color_helpers: Vec::new(),
            tiles: [
                TerrainTile::new(world, 0)?,
                TerrainTile::new(world, 1)?,
                TerrainTile::new(world, 2)?,
            ],
            indices_ready: false,
            plan_revision: 0,
            road_candidates: Vec::new(),
            next_road: 0,
            wanted_roads: Vec::with_capacity(MAX_ROADS),
            wanted_tiles: Vec::with_capacity(MAX_NEAR + 2 * MAX_DISTANT),
            road_points: Vec::with_capacity(MAX_ROADS * super::ROAD_CONTROLS),
            structures: Vec::new(),
            recipes: HashMap::new(),
            supplies: Vec::new(),
            wanted_structures: Vec::new(),
            ready_structures: Vec::with_capacity(32),
            startup: None,
            startup_candidates: Vec::new(),
            startup_next: 0,
            startup_points: Vec::new(),
            surfaces: Vec::new(),
            ready: None,
            terrain_work: None,
            admissions: HashMap::new(),
            terrain_ready: [false; 3],
            shared_cpu_bytes: Some(shared_cpu_bytes),
            publication_available: false,
            failure: None,
            last_road_work: 0,
            last_terrain_work: 0,
        })
    }

    pub fn set_presentation_allocation(
        &mut self,
        next: PresentationAllocation,
    ) -> Result<(), ReferenceError> {
        let current = self.metrics();
        let old = self.presentation;
        let exceeds = current
            .active_cpu_bytes
            .saturating_sub(old.active_cpu)
            .saturating_add(next.active_cpu)
            > ACTIVE_CPU
            || current
                .active_gpu_bytes
                .saturating_sub(old.active_gpu)
                .saturating_add(next.active_gpu)
                > ACTIVE_GPU
            || current
                .prototype_cpu_bytes
                .saturating_sub(old.prototype_cpu)
                .saturating_add(next.prototype_cpu)
                > PROTOTYPE_CPU
            || current
                .prototype_gpu_bytes
                .saturating_sub(old.prototype_gpu)
                .saturating_add(next.prototype_gpu)
                > PROTOTYPE_GPU
            || current
                .generated_objects
                .saturating_sub(old.objects)
                .saturating_add(next.objects)
                > MAX_OBJECTS
            || current
                .pending_work
                .saturating_sub(old.pending)
                .saturating_add(next.pending)
                > 32
            || next.staging_bytes > 32 * MIB
            || next.jobs + usize::from(self.terrain_work.is_some()) > 2;
        if exceeds {
            return Err(ReferenceError(
                "Source presentation admission exceeds its resource budget".into(),
            ));
        }
        self.presentation = next;
        Ok(())
    }
    pub fn available_staging_bytes(&self) -> usize {
        (32 * MIB).saturating_sub(self.presentation.staging_bytes)
    }
    pub fn available_external_jobs(&self) -> usize {
        if !self.indices_ready {
            return 0;
        }
        2usize.saturating_sub(self.presentation.jobs + usize::from(self.terrain_work.is_some()))
    }
    /// A completed renderer job uses the same frame publication slot as collision work.
    pub fn take_presentation_publication(&mut self) -> bool {
        if self.failure.is_some() {
            return false;
        }
        std::mem::take(&mut self.publication_available)
    }
    /// Reserve decoded bytes before loading an admitted prototype or terrain material.
    pub fn reserve_prototype(
        &mut self,
        source: SourceId,
        cpu: usize,
        gpu: usize,
    ) -> Result<(), ReferenceError> {
        if self.admissions.contains_key(&source) {
            return Ok(());
        }
        let current = self.metrics();
        if current
            .prototype_cpu_bytes
            .checked_add(cpu)
            .is_none_or(|v| v > PROTOTYPE_CPU)
            || current
                .prototype_gpu_bytes
                .checked_add(gpu)
                .is_none_or(|v| v > PROTOTYPE_GPU)
        {
            return Err(ReferenceError(format!(
                "Source prototype budget {}:{}",
                source.file, source.path_id
            )));
        }
        self.admissions.insert(
            source,
            PrototypeAllocation {
                cpu,
                gpu,
                ready: false,
            },
        );
        Ok(())
    }
    /// Replace a loader reservation with its retained allocation before binding.
    pub fn resize_prototype(
        &mut self,
        source: &SourceId,
        cpu: usize,
        gpu: usize,
    ) -> Result<(), ReferenceError> {
        let old = *self
            .admissions
            .get(source)
            .ok_or_else(|| ReferenceError("Missing prototype reservation".into()))?;
        let current = self.metrics();
        if current
            .prototype_cpu_bytes
            .saturating_sub(old.cpu)
            .saturating_add(cpu)
            > PROTOTYPE_CPU
            || current
                .prototype_gpu_bytes
                .saturating_sub(old.gpu)
                .saturating_add(gpu)
                > PROTOTYPE_GPU
        {
            return Err(ReferenceError(
                "Prototype allocation exceeds its resource budget".into(),
            ));
        }
        self.admissions.insert(
            source.clone(),
            PrototypeAllocation {
                cpu,
                gpu,
                ready: old.ready,
            },
        );
        Ok(())
    }
    pub fn presentation_allocation(&self) -> PresentationAllocation {
        self.presentation
    }
    pub fn bind_prototype(&mut self, source: &SourceId) -> Result<(), ReferenceError> {
        let allocation = self.admissions.get_mut(source).ok_or_else(|| {
            ReferenceError(format!(
                "Unreserved source binding {}:{}",
                source.file, source.path_id
            ))
        })?;
        allocation.ready = true;
        for (tier, material) in self.reference.terrain.materials.iter().enumerate() {
            if &material.source == source {
                self.terrain_ready[tier] = true;
            }
        }
        Ok(())
    }
    pub fn release_prototype(&mut self, source: &SourceId) -> Result<(), ReferenceError> {
        if self
            .surfaces
            .iter()
            .any(|s| s.prototype.as_ref() == Some(source))
            || self
                .structures
                .iter()
                .any(|s| &s.placement.prototype == source)
        {
            return Err(ReferenceError(
                "Cannot release an active source prototype".into(),
            ));
        }
        for (tier, material) in self.reference.terrain.materials.iter().enumerate() {
            if &material.source == source {
                self.terrain_ready[tier] = false;
            }
        }
        self.admissions.remove(source);
        self.recipes.remove(source);
        Ok(())
    }
    pub fn request_at(&mut self, player: [f64; 3], vehicle: [f64; 3]) {
        if self.recovery.is_some() {
            return;
        }
        if !player.into_iter().chain(vehicle).all(f64::is_finite) {
            self.failure = Some("Nonfinite source world request".into());
            return;
        }
        let cell = |p: [f64; 3], size: u32| {
            [
                (p[0] / size as f64).round_ties_even() as i64,
                (-p[2] / size as f64).round_ties_even() as i64,
            ]
        };
        let changed = self.tiles.iter().any(|tile| {
            cell(player, tile.size) != cell(self.request.player, tile.size)
                || cell(vehicle, tile.size) != cell(self.request.vehicle, tile.size)
        });
        self.request.player = player;
        self.request.vehicle = vehicle;
        if changed {
            self.request.revision = self.request.revision.wrapping_add(1);
            self.ready = None;
            self.terrain_work = None;
            self.failure = None;
        }
    }
    pub fn prepare(&mut self) {
        self.publication_available = true;
        self.last_road_work = 0;
        self.last_terrain_work = 0;
        if self.failure.is_some() || self.ready.is_some() || self.presentation.jobs >= 2 {
            return;
        }
        if !self.indices_ready {
            let slots = 2usize.saturating_sub(self.presentation.jobs);
            let before = self.roads.prepared_segments();
            let road_work = usize::from(before < super::ROAD_SEGMENTS);
            let road_ready = self.roads.prepare(if slots > 0 { 8 } else { 0 });
            self.last_road_work = (self.roads.prepared_segments() - before) as usize;
            let placement_ready = self
                .placements
                .prepare(if slots > road_work { 8 } else { 0 });
            self.indices_ready = road_ready && placement_ready;
            return;
        }
        if self.startup.is_none() {
            self.prepare_startup();
            return;
        }
        if self.plan_revision != self.request.revision {
            if let Err(error) = self.plan() {
                self.failure = Some(error.0);
            }
            return;
        }
        if self.next_road < self.road_candidates.len() {
            self.prepare_road();
            return;
        }
        self.prepare_terrain();
    }
    fn prepare_startup(&mut self) {
        if self.startup_candidates.is_empty() && self.startup_next == 0 {
            let bounds = self
                .reference
                .startup
                .iter()
                .map(|root| window(root.pose.translation.map(f64::from), 150.0))
                .reduce(union);
            if let Some(bounds) = bounds {
                self.roads.segments_in(bounds, &mut self.startup_candidates);
            }
        }
        if let Some(&index) = self.startup_candidates.get(self.startup_next) {
            let segment = self.roads.prepare_segment(index, 8);
            self.last_road_work = self.roads.replay_work();
            if let Some(segment) = segment {
                append_road_points(
                    &mut self.startup_points,
                    segment,
                    self.reference.road.def_point_range,
                );
                self.startup_next += 1;
            }
            return;
        }
        match StartupTerrain::new(&self.reference, &self.evaluator, &self.startup_points) {
            Ok(startup) => {
                let count = self.placements.foundation_count();
                let helpers = count + startup.foundations.len();
                let bytes = count * std::mem::size_of::<super::FoundationPoint>()
                    + helpers * std::mem::size_of::<TerrainFoundation>();
                if self.metrics().active_cpu_bytes + bytes > ACTIVE_CPU {
                    self.failure =
                        Some("Source color helper admission exceeds active CPU budget".into());
                    return;
                }
                self.color_foundations = Vec::with_capacity(count);
                self.color_helpers = Vec::with_capacity(helpers);
                self.startup = Some(startup);
            }
            Err(error) => self.failure = Some(error.0),
        }
        self.startup_points.clear();
        self.startup_candidates.clear();
    }

    fn plan(&mut self) -> Result<(), ReferenceError> {
        self.road_candidates.clear();
        self.next_road = 0;
        self.wanted_roads.clear();
        self.wanted_lights.clear();
        self.road_points.clear();
        self.wanted_tiles.clear();
        self.wanted_structures.clear();
        let road_distance =
            self.reference.distances["RoadGenDist"] as f64 + self.road_light.influence_margin();
        // One union query retains both support windows without copying full index arrays.
        let query = union(
            window(self.request.player, road_distance),
            window(self.request.vehicle, road_distance),
        );
        self.roads.segments_in(query, &mut self.road_candidates);
        let building_distance = self.reference.distances["BuildingGenDist"] as f64;
        let bounds = union(
            window(self.request.player, building_distance),
            window(self.request.vehicle, building_distance),
        );
        self.placements
            .buildings_in(bounds, &mut self.wanted_structures);
        self.wanted_structures.retain(|p| {
            intersects(p.bounds, window(self.request.player, building_distance))
                || intersects(p.bounds, window(self.request.vehicle, building_distance))
        });
        if self.wanted_structures.len() > MAX_OBJECTS {
            return Err(ReferenceError("Source object count exceeds8192".into()));
        }
        for (tier, tile) in self.tiles.iter().enumerate() {
            let radius = tile.draw_distance as i64;
            let side = (radius * 2 + 1).max(4);
            let count =
                (((radius as f64 + 0.5).powi(2) * std::f32::consts::PI as f64) as usize).max(5);
            let mut offsets = Vec::with_capacity((side * side) as usize);
            for z in 0..side {
                for x in 0..side {
                    offsets.push([x - radius, z - radius]);
                }
            }
            offsets.sort_by_key(|p| p[0] * p[0] + p[1] * p[1]);
            if count > offsets.len() {
                return Err(ReferenceError("Source terrain coordinate count".into()));
            }
            for point in [self.request.player, self.request.vehicle] {
                let center = [
                    (point[0] / tile.size as f64).round_ties_even() as i64,
                    (-point[2] / tile.size as f64).round_ties_even() as i64,
                ];
                for offset in &offsets[..count] {
                    let key = SurfaceKey {
                        layer: tier_layer(tier),
                        unit: [center[0] + offset[0], center[1] + offset[1]],
                    };
                    if !self.wanted_tiles.contains(&key) {
                        self.wanted_tiles.push(key);
                    }
                }
            }
            let active = self
                .wanted_tiles
                .iter()
                .filter(|k| k.layer == tier_layer(tier))
                .count();
            if active > if tier == 0 { MAX_NEAR } else { MAX_DISTANT } {
                return Err(ReferenceError(format!(
                    "Source terrain tier {tier} count budget"
                )));
            }
        }
        self.plan_revision = self.request.revision;
        Ok(())
    }
    fn prepare_road(&mut self) {
        let index = self.road_candidates[self.next_road];
        let segment = self.roads.prepare_segment(index, 8);
        self.last_road_work = self.roads.replay_work();
        let Some(segment) = segment else {
            return;
        };
        self.next_road += 1;
        let radius = self.reference.distances["RoadGenDist"] as f64;
        if !self
            .lights
            .iter()
            .any(|light| light.id.unit[0] == index as i64)
        {
            if let Some(light) = self.road_light.place(&self.reference, &segment) {
                if [self.request.player, self.request.vehicle]
                    .into_iter()
                    .any(|point| {
                        intersects(
                            segment.bounds,
                            window(point, radius + self.road_light.influence_margin()),
                        )
                    })
                {
                    self.lights.push(light);
                }
            }
        }
        if self
            .lights
            .iter()
            .any(|light| light.id.unit[0] == index as i64)
        {
            self.wanted_lights.push(index);
        }
        if self.metrics().generated_objects > MAX_OBJECTS {
            self.failure = Some("Source object count exceeds 8192".into());
            return;
        }
        if ![self.request.player, self.request.vehicle]
            .into_iter()
            .any(|p| intersects(segment.bounds, window(p, radius)))
        {
            return;
        }
        if self.wanted_roads.len() == MAX_ROADS {
            self.failure = Some("Source road count exceeds512".into());
            return;
        }
        self.wanted_roads.push(index);
        append_road_points(
            &mut self.road_points,
            segment,
            self.reference.road.def_point_range,
        );
        let key = SurfaceKey {
            layer: WorldLayer::Road,
            unit: [index as i64, 0],
        };
        if self.surfaces.iter().any(|s| s.key == key) {
            return;
        }
        let mut surface = WorldSurface::default();
        deform_road(&self.reference, &segment, &mut surface);
        self.ready = Some(PreparedSurface {
            renderer_ready: false,
            renderer_reserved: false,
            request: self.request,
            published: PublishedSurface {
                key,
                bounds: surface_bounds(&surface),
                surface,
                prototype: Some(self.reference.road.prototypes[segment.biome as usize].clone()),
                collider: None,
                collision_bytes: 0,
                road: Some(segment),
            },
        });
    }
    fn prepare_terrain(&mut self) {
        if self.terrain_work.is_none() {
            let Some(key) = self
                .wanted_tiles
                .iter()
                .find(|key| !self.surfaces.iter().any(|s| s.key == **key))
                .copied()
            else {
                return;
            };
            let tier = layer_tier(key.layer);
            let tile = self.tiles[tier];
            let mut foundations = Vec::new();
            let mut holes = Vec::new();
            if tier == 0 {
                if let Some(startup) = &self.startup {
                    foundations.extend_from_slice(&startup.foundations);
                    let center = [
                        key.unit[0] as f32 * tile.size as f32,
                        key.unit[1] as f32 * tile.size as f32,
                    ];
                    holes.extend(
                        startup
                            .holes
                            .iter()
                            .zip(&startup.hole_centers)
                            .filter(|(_, position)| {
                                let x = position[0] - center[0];
                                let z = position[2] - center[1];
                                x * x + z * z < tile.size as f32 * tile.size as f32 * 1.25
                            })
                            .map(|(hole, _)| *hole),
                    );
                }
                let mut generated = Vec::new();
                let center = [
                    key.unit[0] as f64 * tile.size as f64,
                    0.0,
                    -key.unit[1] as f64 * tile.size as f64,
                ];
                self.placements
                    .foundations_in(window(center, tile.size as f64), &mut generated);
                foundations.extend(generated.iter().map(|p| TerrainFoundation {
                    position: [
                        p.position[0] as f32,
                        p.position[1] as f32,
                        -p.position[2] as f32,
                    ],
                    range_squared: p.range_squared,
                }));
            }
            let mut work = TerrainWork {
                request: self.request,
                key,
                job: TerrainSurfaceJob::default(),
                surface: WorldSurface::default(),
                foundations,
                holes,
                roads: self
                    .road_points
                    .iter()
                    .filter(|point| {
                        let x = point.segment_origin[0] - key.unit[0] as f32 * tile.size as f32;
                        let z = point.segment_origin[1] - key.unit[1] as f32 * tile.size as f32;
                        x * x + z * z < 1_000_000.0
                    })
                    .copied()
                    .collect(),
            };
            work.job.begin(
                key.unit,
                TerrainContext {
                    evaluator: &self.evaluator,
                    tile,
                    roads: &work.roads,
                    road_radius_squared: None,
                    foundations: &work.foundations,
                    holes: &work.holes,
                    biome_override: source_override(&self.reference),
                },
                &mut work.surface,
            );
            self.terrain_work = Some(work);
        }
        let work = self.terrain_work.as_mut().unwrap();
        if !same_request(work.request, self.request) {
            self.terrain_work = None;
            return;
        }
        let tier = layer_tier(work.key.layer);
        let before = work.job.prepared_vertices();
        let done = work.job.advance(
            TerrainContext {
                evaluator: &self.evaluator,
                tile: self.tiles[tier],
                roads: &work.roads,
                road_radius_squared: None,
                foundations: &work.foundations,
                holes: &work.holes,
                biome_override: source_override(&self.reference),
            },
            &mut work.surface,
        );
        self.last_terrain_work = work.job.prepared_vertices() - before;
        if done {
            let work = self.terrain_work.take().unwrap();
            self.ready = Some(PreparedSurface {
                renderer_ready: false,
                renderer_reserved: false,
                request: work.request,
                published: PublishedSurface {
                    key: work.key,
                    bounds: surface_bounds(&work.surface),
                    surface: work.surface,
                    prototype: None,
                    collider: None,
                    collision_bytes: 0,
                    road: None,
                },
            });
        }
    }
    pub fn prepared_surface(&self) -> Option<(WorldRequest, &PublishedSurface)> {
        self.ready
            .as_ref()
            .map(|work| (work.request, &work.published))
    }
    pub fn reserve_surface_renderer(
        &mut self,
        request: WorldRequest,
        key: SurfaceKey,
    ) -> Result<(), ReferenceError> {
        let work = self
            .ready
            .as_ref()
            .filter(|work| same_request(work.request, request) && work.published.key == key)
            .ok_or_else(|| ReferenceError("Stale source renderer reservation".into()))?;
        if work.renderer_reserved {
            return Ok(());
        }
        let metrics = self.metrics();
        if metrics
            .active_cpu_bytes
            .saturating_add(surface_render_cpu(&work.published.surface))
            > ACTIVE_CPU
            || metrics
                .active_gpu_bytes
                .saturating_add(surface_render_bytes(&work.published.surface))
                > ACTIVE_GPU
        {
            return Err(ReferenceError(
                "Source renderer allocation exceeds its resource budget".into(),
            ));
        }
        self.ready.as_mut().unwrap().renderer_reserved = true;
        Ok(())
    }
    pub fn accept_surface_renderer(
        &mut self,
        request: WorldRequest,
        key: SurfaceKey,
    ) -> Result<(), ReferenceError> {
        let work = self
            .ready
            .as_mut()
            .filter(|work| same_request(work.request, request) && work.published.key == key)
            .ok_or_else(|| ReferenceError("Stale source renderer preparation".into()))?;
        if !work.renderer_reserved {
            return Err(ReferenceError("Unreserved source renderer".into()));
        }
        work.renderer_ready = true;
        Ok(())
    }
    /// Publish at most one completed unit. Stale work never changes accepted collision.
    pub fn publish(&mut self, physics: &mut PhysicsWorld) -> Result<(), ReferenceError> {
        if !self.publication_available || self.failure.is_some() {
            return Ok(());
        }
        if self.shared_cpu_bytes.is_none() {
            return Ok(());
        }
        if self
            .ready
            .as_ref()
            .is_some_and(|work| !same_request(work.request, self.request))
        {
            self.ready = None;
            return Ok(());
        }
        let Some(work) = self.ready.as_ref() else {
            return self.publish_structure(physics);
        };
        let admitted = match &work.published.prototype {
            Some(source) => self.admissions.get(source).is_some_and(|v| v.ready),
            None => self.terrain_ready[layer_tier(work.published.key.layer)],
        };
        if !admitted || !work.renderer_ready {
            return Ok(());
        }
        let metrics = self.metrics();
        // Geometry and renderer staging were reserved before allocation. Rapier adds collision storage.
        let count = match work.published.key.layer {
            WorldLayer::Road => (metrics.road_segments, MAX_ROADS),
            WorldLayer::Terrain => (metrics.near_tiles, MAX_NEAR),
            WorldLayer::DistantTerrain => (metrics.distant_tiles[0], MAX_DISTANT),
            WorldLayer::DistantTerrain2 => (metrics.distant_tiles[1], MAX_DISTANT),
            _ => unreachable!(),
        };
        if count.0 >= count.1
            || metrics.checkpoint_bytes > 16 * MIB
            || metrics.active_cpu_bytes > ACTIVE_CPU
            || metrics.active_gpu_bytes > ACTIVE_GPU
        {
            let error = ReferenceError(format!(
                "Source geometry budget at {:?}",
                work.published.key
            ));
            self.failure = Some(error.0.clone());
            return Err(error);
        }
        self.publication_available = false;
        let mut work = self.ready.take().unwrap();
        if matches!(
            work.published.key.layer,
            WorldLayer::Road | WorldLayer::Terrain
        ) {
            let layer = work.published.road.map_or(self.terrain_layer, |road| {
                self.road_layers[road.biome as usize]
            });
            match physics.insert_source_mesh(
                &work.published.surface,
                self.origin.local(work.published.surface.origin),
                layer,
                self.reference.collision_rules.layers[layer as usize],
                0.6,
                0.0,
                ACTIVE_CPU.saturating_sub(metrics.active_cpu_bytes),
            ) {
                Ok((collider, collision_bytes)) => {
                    work.published.collider = Some(collider);
                    work.published.collision_bytes = collision_bytes;
                }
                Err(error) => {
                    self.failure = Some(error.0.clone());
                    return Err(error);
                }
            }
        }
        self.surfaces.push(work.published);
        // Keep old support until both new receiver windows have published near terrain.
        if self.recovery.is_none()
            && self.next_road == self.road_candidates.len()
            && self.supports(window(self.request.player, 3.0))
            && self.supports(window(self.request.vehicle, 4.0))
        {
            self.surfaces.retain(|surface| {
                let keep = if surface.key.layer == WorldLayer::Road {
                    self.wanted_roads.contains(&(surface.key.unit[0] as u32))
                } else {
                    self.wanted_tiles.contains(&surface.key)
                };
                if !keep {
                    if let Some(handle) = surface.collider {
                        physics.remove_collider(handle);
                    }
                }
                keep
            });
        }
        if self.recovery.is_none() && self.next_road == self.road_candidates.len() {
            self.lights
                .retain(|light| self.wanted_lights.contains(&(light.id.unit[0] as u32)));
        }
        Ok(())
    }
    pub fn supports(&self, swept: WorldBounds) -> bool {
        if !self.indices_ready
            || ![swept.min[0], swept.min[2], swept.max[0], swept.max[2]]
                .into_iter()
                .all(f64::is_finite)
        {
            return false;
        }
        if self.wanted_structures.iter().any(|p| {
            [0, 2].into_iter().all(|axis| {
                p.bounds.min[axis] <= swept.max[axis] && p.bounds.max[axis] >= swept.min[axis]
            }) && !self.structures.iter().any(|s| s.placement.id == p.id)
        }) {
            return false;
        }
        let size = self.tiles[0].size as f64;
        let x0 = ((swept.min[0] + size * 0.5) / size).floor() as i64;
        let x1 = ((swept.max[0] + size * 0.5) / size).floor() as i64;
        let z0 = ((-swept.max[2] + size * 0.5) / size).floor() as i64;
        let z1 = ((-swept.min[2] + size * 0.5) / size).floor() as i64;
        if x1 < x0 || z1 < z0 || (x1 - x0 + 1).saturating_mul(z1 - z0 + 1) > MAX_NEAR as i64 {
            return false;
        }
        (x0..=x1).all(|x| {
            (z0..=z1).all(|z| {
                self.surfaces.iter().any(|s| {
                    s.key
                        == SurfaceKey {
                            layer: WorldLayer::Terrain,
                            unit: [x, z],
                        }
                        && s.collider.is_some()
                })
            })
        })
    }
    pub fn terrain_color(&mut self, point: [f64; 3]) -> [f32; 4] {
        self.placements
            .foundations_in(window(point, 1000.0), &mut self.color_foundations);
        self.color_helpers.clear();
        if let Some(startup) = &self.startup {
            self.color_helpers.extend_from_slice(&startup.foundations);
        }
        self.color_helpers
            .extend(self.color_foundations.iter().map(|p| TerrainFoundation {
                position: [
                    p.position[0] as f32,
                    p.position[1] as f32,
                    -p.position[2] as f32,
                ],
                range_squared: p.range_squared,
            }));
        let xz = [point[0] as f32, -point[2] as f32];
        let context = TerrainContext {
            evaluator: &self.evaluator,
            tile: self.tiles[0],
            roads: &self.road_points,
            road_radius_squared: Some(1_000_000.0),
            foundations: &self.color_helpers,
            holes: &[],
            biome_override: source_override(&self.reference),
        };
        let color = self.evaluator.color(xz, context.height(xz));
        match source_override(&self.reference) {
            Some(0) => [1., 0., 0., color[3]],
            Some(1) => [0., 0., 1., color[3]],
            Some(2) => [0., 1., 0., color[3]],
            _ => color,
        }
    }
    pub fn window_ready(&self) -> bool {
        self.indices_ready
            && self.failure.is_none()
            && self.plan_revision == self.request.revision
            && self.next_road == self.road_candidates.len()
            && self.wanted_roads.iter().all(|id| {
                self.surfaces
                    .iter()
                    .any(|s| s.key.layer == WorldLayer::Road && s.key.unit[0] == *id as i64)
            })
            && self
                .wanted_tiles
                .iter()
                .all(|key| self.surfaces.iter().any(|s| s.key == *key))
            && self
                .wanted_structures
                .iter()
                .all(|p| self.structures.iter().any(|s| s.placement.id == p.id))
    }
    pub fn recovery_window(&self) -> Option<WorldBounds> {
        self.recovery
            .map(|pending| window(pending.target.position, 24.0))
    }
    pub fn recovery_target(
        &mut self,
        point: [f64; 3],
        prefer_supply: bool,
    ) -> Result<AbsolutePose, ReferenceError> {
        if let Some(pending) = self.recovery {
            return Ok(pending.target);
        }
        if !self.indices_ready {
            return Err(ReferenceError("Source recovery index is preparing".into()));
        }
        if prefer_supply {
            if let Some(supply) = self.placements.nearest_supply_point(point) {
                let pose = supply.cap_pose;
                let owner = (supply.building_index, supply.supply_index);
                self.request_at(pose.position, pose.position);
                self.recovery = Some(PendingRecovery {
                    request: self.request,
                    target: pose,
                    supply: Some(owner),
                    candidate: 0,
                    best: None,
                });
                return Ok(pose);
            }
        }
        let mut nearest = None;
        for surface in &self.surfaces {
            let Some(road) = surface.road else {
                continue;
            };
            for (control_index, control) in road.source_coords.iter().copied().enumerate() {
                let adjacent = if control_index == 0 {
                    1
                } else {
                    control_index - 1
                };
                let a = if control_index == 0 {
                    control
                } else {
                    road.source_coords[adjacent]
                };
                let b = if control_index == 0 {
                    road.source_coords[adjacent]
                } else {
                    control
                };
                let yaw = (-(b[0] - a[0])).atan2(b[2] - a[2]);
                let rotation = rapier3d::math::Rotation::from_rotation_y(yaw).to_array();
                let position = [
                    road.source_origin[0] + road.source_position[0] + control[0] as f64,
                    road.source_origin[1]
                        + road.source_position[1]
                        + control[1] as f64
                        + self.reference.road.road_above_terrain as f64,
                    -(road.source_origin[2] + road.source_position[2] + control[2] as f64),
                ];
                let distance = (0..3)
                    .map(|a| (position[a] - point[a]).powi(2))
                    .sum::<f64>();
                if nearest.as_ref().is_none_or(|(d, _)| distance < *d) {
                    nearest = Some((distance, AbsolutePose { position, rotation }));
                }
            }
        }
        let pose = nearest
            .map(|(_, pose)| pose)
            .ok_or_else(|| ReferenceError("Source recovery road is not published".into()))?;
        self.request_at(pose.position, pose.position);
        self.recovery = Some(PendingRecovery {
            request: self.request,
            target: pose,
            supply: None,
            candidate: 0,
            best: None,
        });
        Ok(pose)
    }
    /// The material cache admits the complete visible prototype before this recipe is bound.
    pub fn bind_structure_recipe(&mut self, source: &SourceId) -> Result<(), ReferenceError> {
        if self.recipes.contains_key(source) {
            return Ok(());
        }
        if !self
            .admissions
            .get(source)
            .is_some_and(|allocation| allocation.ready)
        {
            return Err(ReferenceError(
                "Structure material binding is not ready".into(),
            ));
        }
        let prototype = self
            .reference
            .prototypes
            .iter()
            .find(|prototype| &prototype.source == source)
            .ok_or_else(|| ReferenceError("Missing structure prototype".into()))?;
        let bytes = super::StructureRecipe::reservation(prototype);
        if self.metrics().prototype_cpu_bytes.saturating_add(bytes) > PROTOTYPE_CPU {
            return Err(ReferenceError(
                "Source collision recipe exceeds prototype CPU budget".into(),
            ));
        }
        let recipe = super::StructureRecipe::new(&self.reference, prototype)?;
        self.recipes.insert(source.clone(), recipe);
        Ok(())
    }

    pub fn accept_structure_renderer(
        &mut self,
        request: WorldRequest,
        id: &super::GeneratedId,
    ) -> Result<(), ReferenceError> {
        if request.session != self.request.session
            || !self.wanted_structures.iter().any(|p| p.id == *id)
        {
            return Err(ReferenceError("Stale structure renderer".into()));
        }
        if !self.ready_structures.contains(id) {
            if self.ready_structures.len() >= 32 {
                return Err(ReferenceError("Structure publication queue cap".into()));
            }
            self.ready_structures.push(id.clone());
        }
        Ok(())
    }
    fn publish_structure(&mut self, physics: &mut PhysicsWorld) -> Result<(), ReferenceError> {
        if self.plan_revision != self.request.revision {
            return Ok(());
        }
        let Some(placement) = self.wanted_structures.iter().find(|placement| {
            !self
                .structures
                .iter()
                .any(|instance| instance.placement.id == placement.id)
                && self.recipes.contains_key(&placement.prototype)
                && self.ready_structures.contains(&placement.id)
        }) else {
            return Ok(());
        };
        let mut points = Vec::new();
        self.placements
            .supplies_for_building(placement.id.unit[0] as u32, &mut points);
        if self.metrics().generated_objects + 1 + points.len() * 2 > MAX_OBJECTS {
            return Err(ReferenceError("Source object count exceeds 8192".into()));
        }
        if self
            .metrics()
            .active_cpu_bytes
            .saturating_add(self.recipes[&placement.prototype].instance_bytes())
            > ACTIVE_CPU
        {
            return Err(ReferenceError(
                "Source structure physics exceeds active CPU budget".into(),
            ));
        }
        let instance =
            self.recipes[&placement.prototype].publish(placement.clone(), self.origin, physics)?;
        for point in points {
            self.supplies.push(PublishedSupply {
                key: placement.id.clone(),
                point,
                definition: self.placements.supply_definition(&point).clone(),
                items: None,
                baselines: None,
            });
        }
        self.ready_structures
            .retain(|id| *id != instance.placement.id);
        self.structures.push(instance);
        self.publication_available = false;
        Ok(())
    }

    /// Call after publication and before movement or recovery. Repeated calls preserve stock and valves.
    pub fn sync_supplies(
        &mut self,
        reference: &ReferenceManifest,
        items: &mut crate::interaction::items::ItemStore,
        interaction: &mut crate::interaction::InteractionState,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        for supply in &mut self.supplies {
            if supply.items.is_some() {
                continue;
            }
            let instance = self
                .structures
                .iter()
                .find(|instance| instance.placement.id == supply.key)
                .ok_or_else(|| ReferenceError("Supply has no published structure".into()))?;
            let prototype = self
                .reference
                .prototypes
                .iter()
                .find(|prototype| prototype.source == instance.placement.prototype)
                .ok_or_else(|| ReferenceError("Supply has no prototype".into()))?;
            let mut tank_key = supply.key.clone();
            tank_key.children.push((supply.definition.tank.clone(), 0));
            let mut cap_key = supply.key.clone();
            cap_key.children.push((supply.definition.cap.clone(), 0));
            let tank =
                items.insert_generated(tank_key.clone(), prototype, supply.point.tank_pose)?;
            let cap = items.insert_generated(cap_key.clone(), prototype, supply.point.cap_pose)?;
            items.initialize_generated_supply(tank, &supply.definition)?;
            interaction.register_generated_supply(
                tank,
                cap,
                &supply.definition,
                &supply.point,
                reference,
            )?;
            supply.baselines = Some([
                items.generated_snapshot(tank, None).unwrap(),
                items
                    .generated_snapshot(cap, interaction.generated_valve(cap))
                    .unwrap(),
            ]);
            for (key, id) in [(tank_key, tank), (cap_key, cap)] {
                if let Some(change) = self.changes.get(super::ObjectKey::Generated(key)) {
                    items.restore_generated(id, &change.snapshot)?;
                    if let Some(valve) = change.snapshot.valve {
                        interaction.restore_generated_valve(id, valve)?;
                    }
                }
            }
            let cap_record = self
                .reference
                .records
                .iter()
                .find(|record| record.source == supply.definition.cap)
                .ok_or_else(|| ReferenceError("Missing source cap owner".into()))?;
            let Some(crate::reference::WorldValue::Link(owner)) =
                cap_record.fields.get("m_GameObject")
            else {
                return Err(ReferenceError("Missing source cap GameObject".into()));
            };
            for (source, handle) in &instance.colliders {
                let record = self
                    .reference
                    .records
                    .iter()
                    .find(|record| &record.source == source)
                    .ok_or_else(|| ReferenceError("Missing source collider owner".into()))?;
                if matches!(record.fields.get("m_GameObject"), Some(crate::reference::WorldValue::Link(value)) if value == owner)
                {
                    physics.rapier_mut().colliders[*handle].user_data = cap.0 as u128 + 1;
                    interaction.bind_surface(
                        *handle,
                        crate::reference::ActionBinding {
                            collider: crate::reference::ColliderId(0),
                            use_owner: Some(cap),
                            pickup_owner: None,
                            removal_owner: None,
                            push_owner: None,
                        },
                    );
                }
            }
            supply.items = Some((tank, cap));
        }
        Ok(())
    }

    pub fn changes(&self) -> &super::ChangeRecords {
        &self.changes
    }

    /// Retire only after the replacement collision windows support both receivers.
    pub fn retire_structures(
        &mut self,
        physics: &mut PhysicsWorld,
        items: &mut crate::interaction::items::ItemStore,
        interaction: &mut crate::interaction::InteractionState,
    ) {
        if self.recovery.is_some()
            || self.plan_revision != self.request.revision
            || !self.supports(window(self.request.player, 3.0))
            || !self.supports(window(self.request.vehicle, 4.0))
        {
            return;
        }
        let mut index = 0;
        while index < self.structures.len() {
            let id = &self.structures[index].placement.id;
            if self.wanted_structures.iter().any(|p| &p.id == id)
                || self.supplies.iter().filter(|s| &s.key == id).any(|s| {
                    s.items.is_some_and(|(tank, cap)| {
                        !items.can_remove_generated(tank) || !items.can_remove_generated(cap)
                    })
                })
            {
                index += 1;
                continue;
            }
            let instance = self.structures.swap_remove(index);
            let mut supply_index = 0;
            while supply_index < self.supplies.len() {
                if self.supplies[supply_index].key != instance.placement.id {
                    supply_index += 1;
                    continue;
                }
                let supply = self.supplies.swap_remove(supply_index);
                if let (Some((tank, cap)), Some(baselines)) = (supply.items, supply.baselines) {
                    for (ordinal, (item, source, valve)) in [
                        (tank, &supply.definition.tank, None),
                        (
                            cap,
                            &supply.definition.cap,
                            interaction.generated_valve(cap),
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let mut key = supply.key.clone();
                        key.children.push((source.clone(), 0));
                        if let Some(snapshot) = items.generated_snapshot(item, valve) {
                            self.changes.record(
                                super::ObjectKey::Generated(key),
                                snapshot,
                                &baselines[ordinal],
                            );
                        }
                        for (_, collider) in &instance.colliders {
                            interaction.unbind_generated(item, std::slice::from_ref(collider));
                        }
                        items.remove_generated(item);
                    }
                }
            }
            instance.retire(physics);
        }
    }

    /// A recovery target selects a loading window, never a final chassis pose.
    pub fn recovery_ready(&self, pose: AbsolutePose, require_supply: bool) -> bool {
        let Some(recovery) = &self.recovery else {
            return false;
        };
        recovery.target == pose
            && same_request(recovery.request, self.request)
            && self.failure.is_none()
            && self.next_road == self.road_candidates.len()
            && self.plan_revision == self.request.revision
            && self.supports(window(pose.position, 24.0))
            && (!require_supply
                || recovery.supply.as_ref().is_some_and(|owner| {
                    self.supplies.iter().any(|supply| {
                        (supply.point.building_index, supply.point.supply_index) == *owner
                            && supply.items.is_some()
                    })
                }))
    }

    /// Search eight published road controls per update. No source index replay occurs here.
    pub fn prepare_recovery_pose(
        &mut self,
        physics: &PhysicsWorld,
        mut clearance: impl FnMut(&PhysicsWorld, AbsolutePose) -> bool,
    ) -> Result<Option<AbsolutePose>, ReferenceError> {
        use rapier3d::math::{Pose, Rotation, Vector};
        use rapier3d::prelude::{QueryFilter, Ray};
        let Some(mut pending) = self.recovery else {
            return Ok(None);
        };
        if !self.recovery_ready(pending.target, pending.supply.is_some()) {
            return Ok(None);
        }
        let mut cursor = 0;
        let mut work = 0;
        let ground = |handle: ColliderHandle, _: &rapier3d::prelude::Collider| {
            self.surfaces
                .iter()
                .any(|surface| surface.collider == Some(handle))
        };
        for surface in &self.surfaces {
            let Some(road) = surface.road else {
                continue;
            };
            for (index, control) in road.source_coords.iter().enumerate() {
                cursor += 1;
                if cursor <= pending.candidate {
                    continue;
                }
                if work == 8 {
                    self.recovery = Some(pending);
                    return Ok(None);
                }
                pending.candidate = cursor;
                work += 1;
                let position = [
                    road.source_origin[0] + road.source_position[0] + control[0] as f64,
                    road.source_origin[1] + road.source_position[1] + control[1] as f64,
                    -(road.source_origin[2] + road.source_position[2] + control[2] as f64),
                ];
                if !self.supports(window(position, 7.0)) {
                    continue;
                }
                let distance = (0..3)
                    .map(|axis| (position[axis] - pending.target.position[axis]).powi(2))
                    .sum::<f64>();
                if pending.best.is_some_and(|(best, _)| distance >= best) {
                    continue;
                }
                let previous = road.source_coords[index.saturating_sub(1)];
                let next = road.source_coords[(index + 1).min(road.source_coords.len() - 1)];
                let rotation = Rotation::from_rotation_y(
                    (-(next[0] - previous[0])).atan2(next[2] - previous[2]),
                );
                let local = Vector::from_array(self.origin.local(position));
                let ray = Ray::new(local + Vector::Y * 4.0, -Vector::Y);
                let Some((_, height)) = physics
                    .queries(QueryFilter::default().exclude_sensors().predicate(&ground))
                    .cast_ray(&ray, 8.0, true)
                else {
                    continue;
                };
                let pose =
                    Pose::from_parts(ray.origin + ray.dir * height + Vector::Y * 1.1, rotation);
                let absolute = AbsolutePose {
                    position: self.origin.absolute(pose.translation.to_array()),
                    rotation: pose.rotation.to_array(),
                };
                let clear = clearance(physics, absolute);
                if clear {
                    pending.best = Some((
                        distance,
                        AbsolutePose {
                            position: self.origin.absolute(pose.translation.to_array()),
                            rotation: pose.rotation.to_array(),
                        },
                    ));
                }
            }
        }
        self.recovery = Some(pending);
        pending
            .best
            .map(|(_, pose)| Some(pose))
            .ok_or_else(|| ReferenceError("No clear supported source recovery pose".into()))
    }

    /// Check all destination support before the caller can mutate transported bodies.
    pub fn preflight_recovery(
        &self,
        target: AbsolutePose,
        player: [f64; 3],
        vehicle: [f64; 3],
    ) -> Result<RecoveryPermit, ReferenceError> {
        if self.recovery.as_ref().is_none_or(|pending| {
            pending.best.is_none_or(|(_, pose)| pose != target)
                || !same_request(pending.request, self.request)
        }) {
            return Err(ReferenceError("Stale source recovery destination".into()));
        }
        if !self.supports(window(player, 3.0)) || !self.supports(window(vehicle, 4.0)) {
            return Err(ReferenceError(
                "Source recovery destination is unsupported".into(),
            ));
        }
        Ok(RecoveryPermit {
            request: self.request,
            player,
            vehicle,
        })
    }
    /// Validate the permit, transport once, then release pinned support without another fallible check.
    pub fn finish_recovery(
        &mut self,
        permit: RecoveryPermit,
        transport: impl FnOnce(),
    ) -> Result<(), ReferenceError> {
        if self.recovery.is_none() || !same_request(permit.request, self.request) {
            return Err(ReferenceError("Stale source recovery permit".into()));
        }
        transport();
        self.recovery = None;
        self.request_at(permit.player, permit.vehicle);
        Ok(())
    }

    pub fn fail(&mut self, error: ReferenceError) {
        self.failure = Some(error.0);
    }

    pub fn pending_structures(&self) -> &[Placement] {
        &self.wanted_structures
    }

    /// Rebase between physics steps. Pending geometry stays in absolute coordinates.
    pub fn rebase(&mut self, position: [f32; 3], physics: &mut PhysicsWorld) -> Option<[f32; 3]> {
        if position[0].abs().max(position[2].abs()) <= super::REBASE_LIMIT {
            return None;
        }
        let shift = [
            (position[0] / 128.0).round() * 128.0,
            0.0,
            (position[2] / 128.0).round() * 128.0,
        ];
        physics.rebase(shift);
        self.origin.shift(shift);
        Some(shift)
    }

    pub fn frame(&self) -> WorldFrame<'_> {
        WorldFrame {
            request: self.request,
            origin: self.origin,
            surfaces: &self.surfaces,
            lights: &self.lights,
            structures: &self.structures,
            supplies: &self.supplies,
            startup: self.startup.as_ref(),
            metrics: self.metrics(),
            failure: self.failure.as_deref(),
        }
    }
    pub fn metrics(&self) -> WorldMetrics {
        let mut metrics = WorldMetrics {
            pending_bytes: self.presentation.staging_bytes,
            pending_work: self.ready_structures.len()
                + usize::from(self.ready.is_some())
                + usize::from(self.terrain_work.is_some())
                + self.presentation.pending,
            checkpoint_bytes: self.roads.retained_index_bytes()
                + self.placements.retained_index_bytes(),
            prototype_cpu_bytes: self.shared_cpu_bytes.unwrap_or(0)
                + self.presentation.prototype_cpu
                + self
                    .recipes
                    .values()
                    .map(|recipe| recipe.cpu_bytes)
                    .sum::<usize>(),
            prototype_gpu_bytes: self.presentation.prototype_gpu,
            active_gpu_bytes: self.presentation.active_gpu,
            changed_record_bytes: self.changes.retained_bytes(),
            changed_objects: self.changes.iter().count(),
            active_cpu_bytes: self.presentation.active_cpu
                + self.color_foundations.capacity() * std::mem::size_of::<super::FoundationPoint>()
                + self.color_helpers.capacity() * std::mem::size_of::<TerrainFoundation>()
                + self
                    .structures
                    .iter()
                    .map(|instance| instance.cpu_bytes)
                    .sum::<usize>()
                + self.ready_structures.capacity() * std::mem::size_of::<super::GeneratedId>()
                + self
                    .ready_structures
                    .iter()
                    .map(|id| id.generator.file.capacity())
                    .sum::<usize>()
                + self.road_candidates.capacity() * std::mem::size_of::<u32>()
                + self.wanted_roads.capacity() * std::mem::size_of::<u32>()
                + self.wanted_lights.capacity() * std::mem::size_of::<u32>()
                + self.wanted_tiles.capacity() * std::mem::size_of::<SurfaceKey>()
                + self.road_points.capacity() * std::mem::size_of::<TerrainRoadPoint>()
                + self.startup_candidates.capacity() * std::mem::size_of::<u32>()
                + self.startup_points.capacity() * std::mem::size_of::<TerrainRoadPoint>()
                + self.surfaces.capacity() * std::mem::size_of::<PublishedSurface>()
                + self.lights.capacity() * std::mem::size_of::<super::WorldLight>()
                + self.structures.capacity() * std::mem::size_of::<super::StructureInstance>()
                + self.supplies.capacity() * std::mem::size_of::<PublishedSupply>()
                + self.wanted_structures.capacity() * std::mem::size_of::<Placement>(),
            prepared_roads: self.roads.prepared_segments(),
            prepared_buildings: self.placements.prepared_buildings(),
            indices_ready: self.indices_ready,
            last_road_work: self.last_road_work,
            last_terrain_work: self.last_terrain_work,
            generated_objects: self.structures.len()
                + self.supplies.len() * 2
                + self.lights.len()
                + self.presentation.objects,
            ..WorldMetrics::default()
        };
        for surface in &self.surfaces {
            match surface.key.layer {
                WorldLayer::Road => metrics.road_segments += 1,
                WorldLayer::Terrain => metrics.near_tiles += 1,
                WorldLayer::DistantTerrain => metrics.distant_tiles[0] += 1,
                WorldLayer::DistantTerrain2 => metrics.distant_tiles[1] += 1,
                _ => {}
            }
            let bytes = surface_bytes(&surface.surface);
            let render_bytes = surface_render_bytes(&surface.surface);
            metrics.active_cpu_bytes +=
                bytes + surface_render_cpu(&surface.surface) + surface.collision_bytes;
            metrics.active_gpu_bytes += render_bytes;
        }
        if let Some(work) = &self.ready {
            metrics.active_cpu_bytes += surface_bytes(&work.published.surface);
            if work.renderer_reserved {
                metrics.active_cpu_bytes += surface_render_cpu(&work.published.surface);
                metrics.active_gpu_bytes += surface_render_bytes(&work.published.surface);
            }
        }
        if let Some(work) = &self.terrain_work {
            metrics.active_cpu_bytes += work.job.retained_bytes()
                + surface_bytes(&work.surface)
                + work.foundations.capacity() * std::mem::size_of::<TerrainFoundation>()
                + work.holes.capacity() * std::mem::size_of::<[[f32; 3]; 4]>()
                + work.roads.capacity() * std::mem::size_of::<TerrainRoadPoint>();
        }
        if let Some(startup) = &self.startup {
            metrics.active_cpu_bytes += startup.foundations.capacity()
                * std::mem::size_of::<TerrainFoundation>()
                + startup.holes.capacity() * std::mem::size_of::<[[f32; 3]; 4]>()
                + startup.hole_centers.capacity() * std::mem::size_of::<[f32; 3]>()
                + startup.root_offsets.capacity() * std::mem::size_of::<(SourceId, f32)>()
                + startup
                    .root_offsets
                    .iter()
                    .map(|(id, _)| id.file.capacity())
                    .sum::<usize>();
        }
        for allocation in self.admissions.values() {
            metrics.prototype_cpu_bytes += allocation.cpu;
            metrics.prototype_gpu_bytes += allocation.gpu;
        }
        metrics
    }
}

fn source_override(reference: &WorldReference) -> Option<u8> {
    (reference.terrain.game_settings.biome != 0)
        .then_some((reference.terrain.game_settings.biome - 1) as u8)
}
fn same_request(a: WorldRequest, b: WorldRequest) -> bool {
    a.session == b.session && a.revision == b.revision
}
fn tier_layer(tier: usize) -> WorldLayer {
    match tier {
        0 => WorldLayer::Terrain,
        1 => WorldLayer::DistantTerrain,
        _ => WorldLayer::DistantTerrain2,
    }
}
fn layer_tier(layer: WorldLayer) -> usize {
    match layer {
        WorldLayer::Terrain => 0,
        WorldLayer::DistantTerrain => 1,
        WorldLayer::DistantTerrain2 => 2,
        _ => panic!("terrain layer"),
    }
}
fn window(point: [f64; 3], radius: f64) -> WorldBounds {
    WorldBounds {
        min: [point[0] - radius, f64::NEG_INFINITY, point[2] - radius],
        max: [point[0] + radius, f64::INFINITY, point[2] + radius],
    }
}
fn union(a: WorldBounds, b: WorldBounds) -> WorldBounds {
    WorldBounds {
        min: std::array::from_fn(|i| a.min[i].min(b.min[i])),
        max: std::array::from_fn(|i| a.max[i].max(b.max[i])),
    }
}
fn intersects(a: WorldBounds, b: WorldBounds) -> bool {
    (0..3).all(|i| a.min[i] <= b.max[i] && a.max[i] >= b.min[i])
}
fn surface_bounds(surface: &WorldSurface) -> WorldBounds {
    let mut bounds = WorldBounds {
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
    };
    for p in &surface.vertices {
        for a in 0..3 {
            let v = surface.origin[a] + p[a] as f64;
            bounds.min[a] = bounds.min[a].min(v);
            bounds.max[a] = bounds.max[a].max(v);
        }
    }
    bounds
}
fn surface_bytes(surface: &WorldSurface) -> usize {
    surface.vertices.capacity() * 12
        + surface.normals.capacity() * 12
        + surface.tangents.capacity() * 16
        + surface.uv.capacity() * 8
        + surface.colors.capacity() * 16
        + surface.triangles.capacity() * 12
        + surface.submeshes.capacity() * std::mem::size_of::<std::ops::Range<usize>>()
}

// Bevy retains one vertex buffer for each material range. Casters share those handles.
fn surface_render_cpu(surface: &WorldSurface) -> usize {
    surface_render_bytes(surface) * 2 + surface.submeshes.len() * 2048
}
fn surface_render_bytes(surface: &WorldSurface) -> usize {
    let attributes = surface.vertices.len() * 12
        + surface.normals.len() * 12
        + surface.tangents.len() * 16
        + surface.uv.len() * 8
        + surface.colors.len() * 16;
    attributes
        .saturating_mul(surface.submeshes.len())
        .saturating_add(surface.triangles.len() * 12)
}

fn append_road_points(output: &mut Vec<TerrainRoadPoint>, segment: RoadSegment, range: f32) {
    for point in segment.source_coords {
        output.push(TerrainRoadPoint {
            position: std::array::from_fn(|axis| {
                (segment.source_origin[axis] + segment.source_position[axis] + point[axis] as f64)
                    as f32
            }),
            segment_origin: [
                segment.source_position[0] as f32,
                segment.source_position[2] as f32,
            ],
            range,
        });
    }
}
