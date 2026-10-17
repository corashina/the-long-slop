//! Bounded source generation. Publication uses absolute geometry and the current physics origin.
use super::{
    AbsolutePose, Placement, PlacementIndex, RoadIndex, RoadSegment, RoadSequence, StartupTerrain,
    TerrainContext, TerrainEvaluator, TerrainFoundation, TerrainRoadPoint, TerrainSurfaceJob,
    TerrainTile, WorldBounds, WorldLayer, WorldMetrics, WorldOrigin, WorldSurface, deform_road,
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

#[cfg(any(target_arch = "wasm32", test))]
use super::renderer_pending::{PipelineContinuation, RendererOwner};

const MIB: usize = 1024 * 1024;
const MAX_ROADS: usize = 512;
const MAX_NEAR: usize = 512;
const MAX_DISTANT: usize = 256;
const MAX_OBJECTS: usize = 32768;
const MAX_PENDING: usize = 32;
/// Ground around a placed player or vehicle that must exist before play continues.
const SUPPORT_RADIUS: f64 = 24.0;
/// Focus movement in metres before the structure window refreshes.
const STRUCTURE_REFRESH_DISTANCE: f32 = 1.0;
const ACTIVE_CPU: usize = 256 * MIB;
const ACTIVE_GPU: usize = 128 * MIB;
const PROTOTYPE_CPU: usize = 512 * MIB;
const PROTOTYPE_GPU: usize = 1024 * MIB;
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldRequest {
    pub session: u64,
    pub revision: u64,
    pub player: [f64; 3],
    pub vehicle: [f64; 3],
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
    pub dependency_staging_bytes: usize,
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
    pending_bytes: usize,
    renderer_ready: bool,
    renderer_reserved: bool,
    request: WorldRequest,
    published: PublishedSurface,
}
struct SelectedRoad {
    segment: RoadSegment,
    light: Option<super::WorldLight>,
}
struct TerrainWork {
    pending_bytes: usize,
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
    static_controls: Vec<super::contents::ContentInstance>,
    binding_baseline: Option<usize>,
    binding_bytes: usize,
    binding_owner_bytes: usize,
    binding_objects: usize,
    binding_reserved: usize,
    lamp_bindings: Vec<content_lamps::LampBinding>,
    seed: i32,
    structure_helpers: super::structure_helpers::StructureHelpers,
    reference: Arc<WorldReference>,
    presentation: PresentationAllocation,
    overlap: WorldMetrics,
    dependency_staging_reserve: usize,
    dependency_metadata: usize,
    changes: super::ChangeRecords,
    replay_roots: Vec<Placement>,
    replay_keys: Vec<super::ObjectKey>,
    replay_query: Vec<super::ObjectKey>,
    change_revision: u64,
    change_bounds: Option<[WorldBounds; 2]>,
    imported_baselines: Vec<(crate::reference::ItemId, super::ObjectSnapshot)>,
    road_light: super::source_lights::SourceRoadLight,
    lights: Vec<super::WorldLight>,
    wanted_lights: Vec<u32>,
    pub origin: WorldOrigin,
    request: WorldRequest,
    recovery: Option<PendingRecovery>,
    roads: RoadIndex,
    placements: PlacementIndex,
    props: super::prop_layers::PropLayers,
    contents: super::contents::SourceContents,
    evaluator: TerrainEvaluator,
    color_foundations: Vec<super::FoundationPoint>,
    color_helpers: Vec<TerrainFoundation>,
    tiles: [TerrainTile; 3],
    terrain_layer: u8,
    road_layers: [u8; 3],
    indices_ready: bool,
    plan_revision: u64,
    selection_inputs: u64,
    road_focus: [[f64; 3]; 2],
    /// Player and vehicle positions at the last structure refresh.
    structure_focus: [[f64; 3]; 2],
    planned_road_focus: [[f64; 3]; 2],
    terrain_context_dirty: bool,
    selection_changes: u64,
    road_candidates: Vec<u32>,
    next_road: usize,
    selected_roads: Vec<SelectedRoad>,
    next_geometry: usize,
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
    pub fn capture_save(
        &self,
        reference: &ReferenceManifest,
        items: &crate::interaction::items::ItemStore,
        interaction: &crate::interaction::InteractionState,
        physics: &PhysicsWorld,
        required: &[super::ObjectKey],
    ) -> crate::save::WorldSnapshot {
        let mut changes = self.changes.generated_records();
        for (key, item) in items.generated_bindings() {
            let object = super::ObjectKey::Generated(key.clone());
            let owner = self
                .contents
                .active
                .iter()
                .chain(&self.static_controls)
                .find(|c| c.item == Some(item.id) || c.bindings.iter().any(|b| b.item == item.id));
            let supply = self
                .supplies
                .iter()
                .find(|s| s.items.is_some_and(|(a, b)| a == item.id || b == item.id));
            let baseline = self
                .generated_baselines()
                .find(|(id, _)| *id == item.id)
                .map(|(_, s)| s)
                .or_else(|| {
                    supply.and_then(|s| {
                        s.baselines.as_ref().map(|b| {
                            if s.items.unwrap().0 == item.id {
                                &b[0]
                            } else {
                                &b[1]
                            }
                        })
                    })
                });
            if let (Some(current), Some(baseline)) = (
                super::changes::capture(item.id, items, interaction, physics, reference),
                baseline,
            ) {
                changes.record(object.clone(), current.clone(), baseline);
                if item.removed {
                    changes.remove(object.clone());
                }
                if let Some(owner) = owner {
                    changes.set_replay(&object, &owner.origin, &owner.content.placement.id);
                } else if let Some(supply) = supply {
                    if let Some(instance) = self
                        .structures
                        .iter()
                        .find(|s| s.placement.id == supply.key)
                    {
                        changes.set_replay(&object, &instance.placement, &supply.key);
                    }
                }
            }
        }
        let mut dependencies: Vec<_> = changes
            .iter()
            .filter_map(|(_, c)| {
                c.snapshot
                    .as_ref()
                    .and_then(|s| s.attachment.as_ref())
                    .and_then(|a| a.owner.clone())
            })
            .collect();
        dependencies.extend_from_slice(required);
        for item in items
            .iter()
            .filter(|i| matches!(items.object_key(i.id), super::ObjectKey::Imported(_)))
        {
            if let Some(snapshot) =
                super::changes::capture(item.id, items, interaction, physics, reference)
            {
                if let Some(owner) = snapshot.attachment.and_then(|a| a.owner) {
                    dependencies.push(owner);
                }
            }
        }
        while let Some(key) = dependencies.pop() {
            if !matches!(key, super::ObjectKey::Generated(_)) || changes.get(&key).is_some() {
                continue;
            }
            let Some(id) = items.resolve_key(&key) else {
                continue;
            };
            let Some(snapshot) =
                super::changes::capture(id, items, interaction, physics, reference)
            else {
                continue;
            };
            if let Some(owner) = snapshot.attachment.as_ref().and_then(|a| a.owner.clone()) {
                dependencies.push(owner);
            }
            changes.retain_dependency(key.clone(), snapshot);
            if let Some(owner) = self
                .contents
                .active
                .iter()
                .chain(&self.static_controls)
                .find(|c| c.item == Some(id) || c.bindings.iter().any(|b| b.item == id))
            {
                changes.set_replay(&key, &owner.origin, &owner.content.placement.id);
            }
        }
        crate::save::WorldSnapshot {
            seed: self.seed,
            changes,
            spawn: self.contents.state.capture_history(),
        }
    }
    pub fn restore_save(
        &mut self,
        saved: &crate::save::WorldSnapshot,
    ) -> Result<(), ReferenceError> {
        if self.seed != saved.seed || self.indices_ready || !self.structures.is_empty() {
            return Err(ReferenceError(
                "Source save must enter an unprepared matching stream".into(),
            ));
        }
        let history = super::SourceSpawnState::restore_history(saved.spawn.clone())?;
        if history.retained_bytes()
            + self.roads.retained_index_bytes()
            + self.placements.retained_index_bytes()
            > 16 * MIB
        {
            return Err(ReferenceError(
                "Restored source history exceeds checkpoint budget".into(),
            ));
        }
        self.contents.state = history;
        self.changes = saved.changes.clone();
        self.change_revision = u64::MAX;
        self.change_bounds = None;
        Ok(())
    }
    /// The presentation layer must publish and bind the complete target window before Commit.
    pub fn save_owner_ready(
        &self,
        key: &super::ObjectKey,
        items: &crate::interaction::items::ItemStore,
    ) -> bool {
        let Some(item) = items.resolve_key(key) else {
            return false;
        };
        match key {
            super::ObjectKey::Imported(_) => true,
            super::ObjectKey::Generated(_) => self
                .contents
                .active
                .iter()
                .chain(&self.static_controls)
                .find(|c| c.item == Some(item) || c.bindings.iter().any(|b| b.item == item))
                .map(|c| &c.content.placement.id)
                .or_else(|| {
                    self.supplies
                        .iter()
                        .find(|s| s.items.is_some_and(|(a, b)| a == item || b == item))
                        .map(|s| &s.key)
                })
                .is_some_and(|id| self.bindings_ready(id)),
        }
    }
    pub fn save_link_required(&self, position: [f64; 3]) -> bool {
        [self.request.player, self.request.vehicle]
            .iter()
            .any(|focus| {
                source_distance_squared(*focus, position)
                    < self.reference.distances["ItemSpawnDist"].powi(2)
            })
    }
    pub fn saved_bindings_ready(&self, items: &crate::interaction::items::ItemStore) -> bool {
        self.window_ready()
            && self.change_bounds.is_some()
            && self.change_revision == self.changes.revision()
            && self.replay_keys.iter().all(|key| {
                if matches!(key, super::ObjectKey::Imported(_)) {
                    return true;
                }
                let Some(change) = self.changes.get(key) else {
                    return false;
                };
                change.snapshot.is_none()
                    || ((items.resolve_key(key).is_some()
                        || self.lamp_bindings.iter().any(|b| {
                            let super::ObjectKey::Generated(id) = key else {
                                return false;
                            };
                            id.layer == b.parent.layer
                                && id.generator == b.parent.generator
                                && id.unit == b.parent.unit
                                && id.ordinal == b.parent.ordinal
                                && id.children.len() == b.parent.children.len() + 1
                                && id.children.starts_with(&b.parent.children)
                                && id.children.last().is_some_and(|(source, ordinal)| {
                                    source == &b.source && *ordinal == 0
                                })
                        }))
                        && change
                            .instance
                            .as_ref()
                            .is_some_and(|id| self.bindings_ready(id)))
            })
    }
    pub fn select_source_car(&mut self, car_id: i32) {
        if self.contents.state.context.current_car != car_id {
            self.contents.state.context.current_car = car_id;
            self.revise_selection();
        }
    }
    pub fn seed(&self) -> i32 {
        self.seed
    }
    pub fn focus(&self) -> [f64; 3] {
        self.request.player
    }
    pub fn terrain_sample(&mut self, point: [f64; 3]) -> (f32, [f32; 4]) {
        let height = self
            .evaluator
            .base_height([point[0] as f32, point[2] as f32], false);
        (height, self.terrain_color(point))
    }
    pub fn uses_painting_asset(&self, asset: crate::reference::AssetId) -> bool {
        self.contents.instances().any(|c| {
            c.appearance
                .iter()
                .any(|a| a.primary.is_some_and(|p| p.0 == asset))
        })
    }

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
        let contents = super::contents::SourceContents::new(world);
        let structure_helpers =
            super::structure_helpers::StructureHelpers::new(world, &contents.rules)?;
        let road_light = super::source_lights::SourceRoadLight::new(reference, seed)?;
        if shared_cpu_bytes
            .saturating_add(road_light.retained_bytes())
            .saturating_add(contents.rules.retained_bytes())
            .saturating_add(structure_helpers.retained_bytes())
            > PROTOTYPE_CPU
        {
            return Err(ReferenceError(
                "Source rule indices exceed prototype CPU budget".into(),
            ));
        }
        Ok(Self {
            static_controls: Vec::new(),
            binding_baseline: None,
            binding_bytes: 0,
            binding_owner_bytes: 0,
            binding_objects: reference.items.len(),
            binding_reserved: 0,
            lamp_bindings: Vec::new(),
            seed,
            structure_helpers,
            presentation: PresentationAllocation::default(),
            overlap: WorldMetrics::default(),
            dependency_staging_reserve: 0,
            dependency_metadata: 0,
            terrain_layer: *terrain_layer as u8,
            road_layers,
            reference: Arc::clone(world),
            changes: super::ChangeRecords::default(),
            replay_roots: Vec::new(),
            replay_keys: Vec::new(),
            replay_query: Vec::new(),
            change_revision: u64::MAX,
            change_bounds: None,
            imported_baselines: Vec::new(),
            road_light,
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
            props: super::prop_layers::PropLayers::new(world, seed)?,
            contents,
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
            selection_inputs: 0,
            road_focus: [[f64::INFINITY; 3]; 2],
            structure_focus: [[f64::INFINITY; 3]; 2],
            planned_road_focus: [[f64::INFINITY; 3]; 2],
            terrain_context_dirty: false,
            selection_changes: 0,
            road_candidates: Vec::new(),
            next_road: 0,
            selected_roads: Vec::new(),
            next_geometry: 0,
            wanted_roads: Vec::with_capacity(MAX_ROADS),
            wanted_tiles: Vec::with_capacity(MAX_NEAR + 2 * MAX_DISTANT),
            road_points: Vec::with_capacity(MAX_ROADS * super::ROAD_CONTROLS),
            structures: Vec::new(),
            recipes: HashMap::new(),
            supplies: Vec::new(),
            wanted_structures: Vec::with_capacity(MAX_OBJECTS),
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

    pub fn set_overlap(&mut self, overlap: WorldMetrics) -> Result<(), ReferenceError> {
        let previous = std::mem::replace(&mut self.overlap, overlap);
        if let Err(error) = self.set_presentation_allocation(self.presentation) {
            self.overlap = previous;
            return Err(error);
        }
        if self.metrics().checkpoint_bytes > 16 * MIB {
            self.overlap = previous;
            return Err(ReferenceError(
                "Source candidate history overlap exceeds its budget".into(),
            ));
        }
        Ok(())
    }
    pub fn set_presentation_allocation(
        &mut self,
        next: PresentationAllocation,
    ) -> Result<(), ReferenceError> {
        // Stream work can fill a shared budget between presentation updates.
        // A change fails only when it raises a budget above its limit.
        let current = self.presentation_budgets(self.presentation);
        for (budget, current) in self.presentation_budgets(next).into_iter().zip(current) {
            if budget.used > budget.limit && budget.used > current.used {
                return Err(ReferenceError(format!(
                    "{} budget exceeded: requested {} {}, limit {} {}.",
                    budget.name, budget.used, budget.unit, budget.limit, budget.unit,
                )));
            }
        }
        self.presentation = next;
        Ok(())
    }

    pub(crate) fn presentation_budgets(
        &self,
        next: PresentationAllocation,
    ) -> [super::metrics::ResourceBudget; 10] {
        use super::metrics::ResourceBudget;
        let current = self.metrics();
        let old = self.presentation;
        let active_cpu = current
            .active_cpu_bytes
            .saturating_sub(old.active_cpu)
            .saturating_add(next.active_cpu);
        let active_gpu = current
            .active_gpu_bytes
            .saturating_sub(old.active_gpu)
            .saturating_add(next.active_gpu);
        let prototype_cpu = current
            .prototype_cpu_bytes
            .saturating_sub(old.prototype_cpu)
            .saturating_add(next.prototype_cpu);
        let prototype_gpu = current
            .prototype_gpu_bytes
            .saturating_sub(old.prototype_gpu)
            .saturating_add(next.prototype_gpu);
        let objects = current
            .generated_objects
            .saturating_sub(old.objects)
            .saturating_add(next.objects);
        let pending = current
            .pending_work
            .saturating_sub(old.pending)
            .saturating_add(next.pending);
        let staging =
            next.staging_bytes + self.generation_pending_bytes() + self.overlap.pending_bytes;
        let jobs = next.jobs
            + self.overlap.generation_jobs
            + usize::from(self.terrain_work.is_some())
            + usize::from(self.contents.has_replay());
        [
            ResourceBudget {
                name: "World CPU",
                used: active_cpu,
                limit: ACTIVE_CPU,
                unit: "bytes",
            },
            ResourceBudget {
                name: "World GPU",
                used: active_gpu,
                limit: ACTIVE_GPU,
                unit: "bytes",
            },
            ResourceBudget {
                name: "Asset CPU",
                used: prototype_cpu,
                limit: PROTOTYPE_CPU,
                unit: "bytes",
            },
            ResourceBudget {
                name: "Asset GPU",
                used: prototype_gpu,
                limit: PROTOTYPE_GPU,
                unit: "bytes",
            },
            ResourceBudget {
                name: "Objects",
                used: objects,
                limit: MAX_OBJECTS,
                unit: "objects",
            },
            ResourceBudget {
                name: "Pending work",
                used: pending,
                limit: 32,
                unit: "requests",
            },
            ResourceBudget {
                name: "Staging",
                used: staging,
                limit: 32 * MIB,
                unit: "bytes",
            },
            ResourceBudget {
                name: "Dependency staging",
                used: next.dependency_staging_bytes,
                limit: next.staging_bytes,
                unit: "bytes",
            },
            ResourceBudget {
                name: "Generation staging",
                used: self.generation_pending_bytes() - self.binding_reserved,
                limit: 32 * MIB - self.dependency_staging_reserve,
                unit: "bytes",
            },
            ResourceBudget {
                name: "Jobs",
                used: jobs,
                limit: 2,
                unit: "jobs",
            },
        ]
    }
    fn generation_pending_bytes(&self) -> usize {
        self.binding_reserved
            + self.contents.pending_bytes()
            + self.props.pending_bytes()
            + self.terrain_work.as_ref().map_or(0, |w| w.pending_bytes)
            + self.ready.as_ref().map_or(0, |w| {
                w.pending_bytes
                    + if w.renderer_reserved {
                        surface_render_cpu(&w.published.surface)
                    } else {
                        0
                    }
            })
    }
    fn admit_generation(&mut self, bytes: usize) -> bool {
        if bytes > 32 * MIB - self.dependency_staging_reserve
            || self.metrics().active_cpu_bytes.saturating_add(bytes) > ACTIVE_CPU
        {
            self.failure = Some(format!(
                "Source generation budget exceeded: CPU requested {} bytes, limit {ACTIVE_CPU} bytes; staging requested {bytes} bytes, limit {} bytes",
                self.metrics().active_cpu_bytes.saturating_add(bytes),
                32 * MIB - self.dependency_staging_reserve
            ));
            return false;
        }
        bytes <= self.available_staging_bytes()
    }
    // Retained generation may wait for resources. Keep one checked input window free.
    // Grass and synchronous published bindings drain without further resource loads.
    pub fn available_staging_bytes(&self) -> usize {
        self.available_dependency_staging_bytes().min(
            (32 * MIB - self.dependency_staging_reserve)
                .saturating_sub(self.generation_pending_bytes() - self.binding_reserved),
        )
    }
    pub fn available_dependency_staging_bytes(&self) -> usize {
        (32 * MIB).saturating_sub(
            self.presentation.staging_bytes
                + self.generation_pending_bytes()
                + self.overlap.pending_bytes,
        )
    }
    pub fn dependency_staging_configured(&self) -> bool {
        self.dependency_staging_reserve != 0
    }
    pub fn configure_dependency_staging(
        &mut self,
        reserve: usize,
        metadata: usize,
    ) -> Result<(), ReferenceError> {
        if reserve == 0 || reserve >= 32 * MIB {
            return Err(ReferenceError(
                "Invalid dependency staging reservation".into(),
            ));
        }
        if self.dependency_staging_configured() {
            return Ok(());
        }
        self.dependency_staging_reserve = reserve;
        let mut allocation = self.presentation;
        allocation.prototype_cpu += metadata;
        self.set_presentation_allocation(allocation)?;
        self.dependency_metadata = metadata;
        Ok(())
    }
    pub(crate) fn can_request_independent(&self, claims: usize) -> bool {
        self.metrics()
            .pending_work
            .checked_add(claims)
            .is_some_and(|count| count <= MAX_PENDING)
    }
    pub fn available_external_jobs(&self) -> usize {
        if !self.indices_ready {
            return 0;
        }
        2usize.saturating_sub(
            self.presentation.jobs
                + self.overlap.generation_jobs
                + usize::from(self.terrain_work.is_some())
                + usize::from(self.contents.has_replay()),
        )
    }
    pub(crate) fn startup_job_audit(&self) -> serde_json::Value {
        serde_json::json!({
            "presentation": self.presentation.jobs,
            "pending": {
                "presentation": self.presentation.pending,
                "structures": self.ready_structures.len(),
                "contents": self.contents.pending_requests(),
                "props": self.props.pending_requests(),
                "surface": self.ready.is_some(),
                "terrain": self.terrain_work.is_some(),
                "replays": self.replay_roots.len(),
                "overlap": self.overlap.pending_work,
            },
            "terrain": self.terrain_work.is_some(),
            "contents": self.contents.has_replay(),
            "available": self.available_external_jobs(),
        })
    }
    /// A completed renderer job uses the same frame publication slot as collision work.
    pub fn take_presentation_publication(&mut self) -> bool {
        if self.failure.is_some() {
            return false;
        }
        std::mem::take(&mut self.publication_available)
    }
    /// Decoded headroom excludes the table and key needed to own a new prototype.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn prototype_headroom(&self, source: &SourceId) -> usize {
        let owner = if self.admissions.contains_key(source) {
            0
        } else {
            super::metrics::map_insert_bytes(&self.admissions) + source.file.capacity()
        };
        self.available_prototype_cpu_bytes().saturating_sub(owner)
    }
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn available_prototype_cpu_bytes(&self) -> usize {
        PROTOTYPE_CPU.saturating_sub(self.metrics().prototype_cpu_bytes)
    }
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn available_prototype_gpu_bytes(&self) -> usize {
        PROTOTYPE_GPU.saturating_sub(self.metrics().prototype_gpu_bytes)
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
        let owner_bytes =
            super::metrics::map_insert_bytes(&self.admissions) + source.file.capacity();
        if current
            .prototype_cpu_bytes
            .checked_add(cpu.saturating_add(owner_bytes))
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
    /// True when `resize_prototype` would stay inside the prototype budget.
    pub fn prototype_fits(&self, source: &SourceId, cpu: usize, gpu: usize) -> bool {
        let old = self
            .admissions
            .get(source)
            .copied()
            .unwrap_or(PrototypeAllocation {
                cpu: 0,
                gpu: 0,
                ready: false,
            });
        let current = self.metrics();
        current
            .prototype_cpu_bytes
            .saturating_sub(old.cpu)
            .saturating_add(cpu)
            <= PROTOTYPE_CPU
            && current
                .prototype_gpu_bytes
                .saturating_sub(old.gpu)
                .saturating_add(gpu)
                <= PROTOTYPE_GPU
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
        *self.admissions.get_mut(source).unwrap() = PrototypeAllocation {
            cpu,
            gpu,
            ready: old.ready,
        };
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
                (p[0] as f32 / size as f32).round_ties_even() as i64,
                (-p[2] as f32 / size as f32).round_ties_even() as i64,
            ]
        };
        // roadGenScript.controlDist is a squared distance of 10000 (Update 39509).
        let road_changed = [player, vehicle]
            .into_iter()
            .zip(self.road_focus)
            .any(|(a, b)| source_distance_squared(a, b) >= 10000.0);
        if road_changed {
            self.road_focus = [player, vehicle];
        }
        let changed = road_changed
            || self.tiles.iter().any(|tile| {
                cell(player, tile.size) != cell(self.request.player, tile.size)
                    || cell(vehicle, tile.size) != cell(self.request.vehicle, tile.size)
            });
        let before = [self.request.player, self.request.vehicle];
        let after = [player, vehicle];
        let moved = before != after;
        let prop_changed = self.props.focus_changed(before, after);
        let eligibility_changed = self
            .contents
            .eligibility_changed(&self.reference, before, after);
        let radius = f64::from(self.reference.distances["ItemSpawnDist"]);
        let selected = |points: [[f64; 3]; 2], position: [f64; 3]| {
            points.into_iter().any(|p| {
                (p[0] - position[0]).abs() <= radius && (p[2] - position[2]).abs() <= radius
            })
        };
        let replay_changed = moved
            && self.changes.iter().any(|(_, change)| {
                change.snapshot.as_ref().is_some_and(|s| {
                    selected(before, s.pose.position) != selected(after, s.pose.position)
                })
            });
        self.request.player = player;
        self.request.vehicle = vehicle;
        // Physics moves the focus a little every step. A full refresh after each
        // metre keeps spawn edges current without rebuilding the window every step.
        let structures_due = prop_changed
            || eligibility_changed
            || after
                .into_iter()
                .zip(self.structure_focus)
                .any(|(a, b)| source_distance_squared(a, b) >= STRUCTURE_REFRESH_DISTANCE.powi(2));
        let buildings_changed = structures_due && self.indices_ready && {
            self.structure_focus = after;
            self.refresh_structures()
        };
        self.terrain_context_dirty |= road_changed || buildings_changed;
        if changed || prop_changed || buildings_changed || eligibility_changed || replay_changed {
            self.revise_selection();
        }
    }
    fn revise_selection(&mut self) {
        self.request.revision = self.request.revision.wrapping_add(1);
        self.failure = None;
    }
    fn rewind_road(&mut self, key: SurfaceKey) {
        if key.layer == WorldLayer::Road {
            if let Some(index) = self
                .wanted_roads
                .iter()
                .position(|i| *i == key.unit[0] as u32)
            {
                self.next_geometry = self.next_geometry.min(index);
            }
        }
    }
    fn discard_unreserved_surface(&mut self) {
        if self.ready.as_ref().is_some_and(|w| !w.renderer_reserved) {
            let key = self.ready.take().unwrap().published.key;
            self.rewind_road(key);
        }
    }
    /// Advance source discovery once per ordinary application Update.
    fn prepare_descriptors(&mut self, budget_ms: f64) {
        #[cfg(target_arch = "wasm32")]
        let started = crate::browser::game_now();
        #[cfg(not(target_arch = "wasm32"))]
        let started = std::time::Instant::now();
        self.publication_available = true;
        self.last_road_work = 0;
        self.last_terrain_work = 0;
        if self.selection_inputs != self.contents.input_revision()
            || self.selection_changes != self.changes.revision()
        {
            self.revise_selection();
        }
        if self.failure.is_some() {
            return;
        }
        while !self.indices_ready {
            let slots =
                2usize.saturating_sub(self.presentation.jobs + self.overlap.generation_jobs);
            let before = (
                self.roads.prepared_segments(),
                self.placements.prepared_buildings(),
            );
            let road_work = usize::from(before.0 < super::ROAD_SEGMENTS);
            let road_ready = self.roads.prepare(if slots > 0 { 256 } else { 0 });
            self.last_road_work += (self.roads.prepared_segments() - before.0) as usize;
            let placement_ready = self
                .placements
                .prepare(if slots > road_work { 256 } else { 0 });
            self.indices_ready = road_ready && placement_ready;
            #[cfg(target_arch = "wasm32")]
            let elapsed = crate::browser::game_now() - started;
            #[cfg(not(target_arch = "wasm32"))]
            let elapsed = started.elapsed().as_secs_f64() * 1000.;
            if !self.indices_ready
                && (elapsed >= budget_ms
                    || before
                        == (
                            self.roads.prepared_segments(),
                            self.placements.prepared_buildings(),
                        ))
            {
                return;
            }
        }
        if self.startup.is_none() {
            if self.available_external_jobs() > 0 {
                self.prepare_startup();
            }
            return;
        }
        if self.plan_revision != self.request.revision {
            if let Err(error) = self.plan() {
                self.failure = Some(error.0);
            }
            if self.failure.is_some() || self.plan_revision != self.request.revision {
                return;
            }
        }
        if self.next_road < self.road_candidates.len() {
            self.discover_road();
        }
        // Placement needs the complete selected road-point set.
        if self.next_road == self.road_candidates.len() {
            self.prepare_props();
            self.prepare_contents();
        }
    }
    /// Keep payload construction and renderer publication downstream from source selection.
    pub fn prepare_geometry(&mut self) {
        if self.failure.is_some()
            || self.ready.is_some()
            || !self.indices_ready
            || self.startup.is_none()
            || self.plan_revision != self.request.revision
            || self.next_road != self.road_candidates.len()
        {
            return;
        }
        if self.next_geometry < self.wanted_roads.len() {
            self.prepare_road();
        } else {
            self.prepare_terrain();
        }
    }
    pub fn prepare(&mut self, budget_ms: f64) {
        self.prepare_descriptors(budget_ms);
        self.prepare_geometry();
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

    fn refresh_structures(&mut self) -> bool {
        let radius = self.reference.distances["BuildingGenDist"];
        let points = [self.request.player, self.request.vehicle];
        let bounds = union(
            window(points[0], radius as f64),
            window(points[1], radius as f64),
        );
        let numeric =
            |p: &Placement| p.id.layer == WorldLayer::Building && p.id.children.is_empty();
        let count = self.wanted_structures.iter().filter(|p| numeric(p)).count();
        let removed = self.wanted_structures.iter().any(|p| {
            numeric(p)
                && points
                    .into_iter()
                    .all(|f| source_distance_squared(f, p.pose.position) >= radius * radius)
        });
        self.placements
            .update_buildings_in(bounds, &mut self.wanted_structures, |position| {
                points
                    .into_iter()
                    .any(|p| source_distance_squared(p, position) < radius * radius)
            });
        for placement in self
            .props
            .placements()
            .iter()
            .chain(self.contents.active.iter().map(|c| &c.content.placement))
        {
            if !self.wanted_structures.iter().any(|p| p.id == placement.id) {
                self.wanted_structures.push(placement.clone());
            }
        }
        if let Err(error) = super::building_extras::append_ramps(
            &self.reference,
            &self.contents.rules,
            self.seed,
            &mut self.wanted_structures,
            MAX_OBJECTS,
        ) {
            self.failure = Some(error.0);
        }
        self.refresh_poles();
        self.ready_structures
            .retain(|id| self.wanted_structures.iter().any(|p| &p.id == id));
        removed || count != self.wanted_structures.iter().filter(|p| numeric(p)).count()
    }
    fn refresh_poles(&mut self) {
        self.wanted_structures
            .retain(|p| p.id.layer != WorldLayer::Lamp || self.lights.iter().any(|l| l.id == p.id));
        for light in &self.lights {
            if light.biome.is_none() || self.wanted_structures.iter().any(|p| p.id == light.id) {
                continue;
            }
            self.wanted_structures.push(Placement {
                id: light.id.clone(),
                prototype: light.prototype.clone(),
                pose: light.pole,
                scale: light.scale,
                bounds: WorldBounds {
                    min: light.pole.position.map(|v| v - light.range as f64),
                    max: light.pole.position.map(|v| v + light.range as f64),
                },
                object_seed: 0,
            });
        }
    }
    fn prepare_props(&mut self) {
        for _ in 0..4 {
            self.prepare_prop_layer();
            if self.failure.is_some() {
                break;
            }
        }
    }
    fn prepare_prop_layer(&mut self) {
        let budget = self.props.allocation_bound();
        if !self.admit_generation(budget) {
            return;
        }
        let can_request = self.metrics().pending_work < 32;
        if self.failure.is_some() {
            return;
        }
        let helper_radius = ["ObjGenDist", "MountainGenDist", "BuildingGenDist"]
            .into_iter()
            .map(|key| self.reference.distances[key])
            .fold(0_f32, f32::max) as f64;
        self.refresh_helper_context(union(
            window(self.request.player, helper_radius),
            window(self.request.vehicle, helper_radius),
        ));
        if self.failure.is_some() {
            return;
        }
        let context = TerrainContext {
            evaluator: &self.evaluator,
            tile: self.tiles[0],
            roads: &self.road_points,
            road_radius_squared: None,
            foundations: &self.color_helpers,
            holes: &[],
            biome_override: source_override(&self.reference),
        };
        let current = self.props.placements().len();
        let maximum = MAX_OBJECTS
            .saturating_sub(self.metrics().generated_objects)
            .saturating_add(current);
        if let Err(error) = self.props.advance(
            [self.request.player, self.request.vehicle],
            context,
            can_request,
            maximum,
        ) {
            self.failure = Some(error.0);
            return;
        }
        self.wanted_structures.retain(|p| {
            !matches!(
                p.id.layer,
                WorldLayer::Prop
                    | WorldLayer::Mountain
                    | WorldLayer::Tower
                    | WorldLayer::DesertBuilding
            ) || self.props.contains(&p.id)
                || (!p.id.children.is_empty()
                    && self.props.placements().iter().any(|parent| {
                        parent.id.layer == p.id.layer
                            && parent.id.unit == p.id.unit
                            && parent.id.ordinal == p.id.ordinal
                    }))
        });
        for p in self.props.placements() {
            if !self.wanted_structures.iter().any(|w| w.id == p.id) {
                self.wanted_structures.push(p.clone());
            }
        }
        if let Err(error) = super::building_extras::append_ramps(
            &self.reference,
            &self.contents.rules,
            self.seed,
            &mut self.wanted_structures,
            MAX_OBJECTS,
        ) {
            self.failure = Some(error.0);
        }
        self.ready_structures
            .retain(|id| self.wanted_structures.iter().any(|p| &p.id == id));
    }
    fn prepare_contents(&mut self) {
        if self.failure.is_some() {
            return;
        }
        self.select_changed_destinations();
        if self.failure.is_some() {
            return;
        }
        self.contents
            .select_revision(self.request.session, self.request.revision);
        self.contents
            .retain_requests(&self.wanted_structures, &self.replay_roots);
        let metrics = self.metrics();
        let pending = 32usize.saturating_sub(metrics.pending_work);
        let limit = 32usize.min(MAX_OBJECTS.saturating_sub(metrics.generated_objects));
        if limit == 0 {
            if !self
                .contents
                .discovery_ready(&self.reference, &self.wanted_structures, &self.replay_roots)
                .unwrap_or(false)
            {
                self.failure = Some(format!(
                    "Source content object budget exceeded: requested {}, limit {MAX_OBJECTS}",
                    metrics.generated_objects + 1
                ));
            }
            return;
        }
        let reservation = self
            .available_staging_bytes()
            .min(ACTIVE_CPU.saturating_sub(metrics.active_cpu_bytes));
        if !self.admit_generation(reservation) {
            return;
        }
        let available = (16 * MIB).saturating_sub(
            self.roads.retained_index_bytes() + self.placements.retained_index_bytes(),
        );
        let replay_admission = self.contents.has_replay() || self.available_external_jobs() > 0;
        if let Err(error) = self.contents.advance(
            &self.reference,
            &self.wanted_structures,
            if replay_admission {
                &self.replay_roots
            } else {
                &[]
            },
            [self.request.player, self.request.vehicle],
            available,
            limit,
            pending,
            reservation,
        ) {
            self.failure = Some(error.0);
            return;
        }
        self.wanted_structures.retain(|p| {
            self.changes
                .get(super::ObjectKey::Generated(p.id.clone()))
                .is_none_or(|change| {
                    change.snapshot.is_some()
                        && self
                            .replay_keys
                            .contains(&super::ObjectKey::Generated(p.id.clone()))
                })
        });
        self.wanted_structures.retain(|p| {
            let unbound = self
                .contents
                .active
                .iter()
                .any(|c| c.content.placement.id == p.id && c.item.is_none());
            !unbound
                || [self.request.player, self.request.vehicle]
                    .into_iter()
                    .any(|f| {
                        source_distance_squared(f, p.pose.position)
                            < self.reference.distances["ItemSpawnDist"].powi(2)
                    })
                || self.replay_keys.iter().any(|key| {
                    self.changes.get(key).is_some_and(|c| {
                        c.instance.as_ref() == Some(&p.id)
                            || c.snapshot
                                .as_ref()
                                .and_then(|s| s.attachment.as_ref())
                                .and_then(|a| a.owner.as_ref())
                                == Some(&super::ObjectKey::Generated(p.id.clone()))
                    })
                })
        });
        let missing = |p: &Placement| !self.wanted_structures.iter().any(|w| w.id == p.id);
        let missing_light = |l: &super::WorldLight| !self.lights.iter().any(|w| w.id == l.id);
        let count = self
            .contents
            .instances()
            .filter(|c| missing(&c.placement))
            .count();
        let light_count = self
            .contents
            .instances()
            .flat_map(|c| &c.lights)
            .filter(|l| missing_light(l))
            .count();
        let selected_bytes = self
            .contents
            .instances()
            .filter(|c| missing(&c.placement))
            .map(|c| super::contents::placement_heap(&c.placement))
            .sum::<usize>()
            + self
                .contents
                .instances()
                .flat_map(|c| &c.lights)
                .filter(|l| missing_light(l))
                .map(super::WorldLight::heap_bytes)
                .sum::<usize>();
        let allocation = selected_bytes
            + if self.wanted_structures.len() + count > self.wanted_structures.capacity() {
                (self.wanted_structures.len() + count + self.wanted_structures.capacity())
                    * std::mem::size_of::<Placement>()
            } else {
                0
            }
            + if self.lights.len() + light_count > self.lights.capacity() {
                (self.lights.len() + light_count + self.lights.capacity())
                    * std::mem::size_of::<super::WorldLight>()
            } else {
                0
            };
        if !self.admit_generation(allocation) {
            return;
        }
        if self.wanted_structures.try_reserve_exact(count).is_err()
            || self.lights.try_reserve_exact(light_count).is_err()
        {
            self.failure = Some("Selected content descriptor allocation failed".into());
            return;
        }
        for content in self.contents.instances() {
            let key = super::ObjectKey::Generated(content.placement.id.clone());
            let selected = self.replay_keys.iter().any(|key| {
                self.changes.get(key).is_some_and(|c| {
                    c.instance.as_ref() == Some(&content.placement.id)
                        || c.snapshot
                            .as_ref()
                            .and_then(|s| s.attachment.as_ref())
                            .and_then(|a| a.owner.as_ref())
                            == Some(&super::ObjectKey::Generated(content.placement.id.clone()))
                })
            });
            let changed = self.changes.get(&key);
            if changed.is_some_and(|c| c.snapshot.is_none() || !selected) {
                continue;
            }
            let baseline_near = [self.request.player, self.request.vehicle]
                .into_iter()
                .any(|p| {
                    source_distance_squared(p, content.placement.pose.position)
                        < self.reference.distances["ItemSpawnDist"].powi(2)
                });
            if !baseline_near
                && !selected
                && !self
                    .structures
                    .iter()
                    .any(|s| s.placement.id == content.placement.id)
            {
                continue;
            }
            if !self
                .wanted_structures
                .iter()
                .any(|p| p.id == content.placement.id)
            {
                self.wanted_structures.push(content.placement.clone());
            }
        }
        self.lights.retain(|light| {
            light.biome.is_some()
                || self
                    .contents
                    .instances()
                    .filter(|c| {
                        self.wanted_structures
                            .iter()
                            .any(|p| p.id == c.placement.id)
                    })
                    .any(|c| c.lights.iter().any(|l| l.id == light.id))
        });
        for content in self.contents.instances().filter(|c| {
            self.wanted_structures
                .iter()
                .any(|p| p.id == c.placement.id)
        }) {
            for light in &content.lights {
                if let Some(current) = self.lights.iter_mut().find(|l| l.id == light.id) {
                    current.enabled = light.enabled;
                    current.intensity = light.intensity;
                    current.pose = light.pose;
                } else {
                    self.lights.push(light.clone());
                }
            }
        }
        self.contents
            .discard_replay_siblings(&self.wanted_structures);
        self.replay_roots.retain(|root| {
            !self.contents.replay_finished(&root.id)
                || self.replay_keys.iter().any(|key| {
                    self.changes.get(key).is_some_and(|change| {
                        change.replay.as_ref().is_some_and(|p| p.id == root.id)
                            && change.instance.as_ref().is_some_and(|id| {
                                !self
                                    .contents
                                    .active
                                    .iter()
                                    .chain(&self.static_controls)
                                    .any(|c| &c.content.placement.id == id && c.restored)
                            })
                    })
                })
        });
    }

    fn select_changed_destinations(&mut self) {
        let radius = f64::from(self.reference.distances["ItemSpawnDist"]);
        let bounds = [self.request.player, self.request.vehicle].map(|focus| WorldBounds {
            min: [focus[0] - radius, f64::NEG_INFINITY, focus[2] - radius],
            max: [focus[0] + radius, f64::INFINITY, focus[2] + radius],
        });
        if self.change_revision != self.changes.revision() || self.change_bounds != Some(bounds) {
            self.replay_keys.clear();
            for bounds in bounds {
                let (count, bytes) = self.changes.query_reservation(bounds);
                if count > MAX_OBJECTS
                    || bytes.saturating_mul(4) > self.available_staging_bytes()
                    || self
                        .metrics()
                        .active_cpu_bytes
                        .saturating_add(bytes.saturating_mul(4))
                        > ACTIVE_CPU
                {
                    self.failure =
                        Some("Changed destination selection exceeds staging admission".into());
                    return;
                }
                self.replay_query.reserve(count);
                self.changes.changes_in(bounds, &mut self.replay_query);
                for key in self.replay_query.drain(..) {
                    if !self.replay_keys.contains(&key) {
                        self.replay_keys.push(key);
                    }
                }
            }
            if self.replay_keys.len() > MAX_OBJECTS {
                self.failure = Some("Changed destination replay exceeds active admission".into());
                return;
            }
            self.change_revision = self.changes.revision();
            self.change_bounds = Some(bounds);
        }
        self.replay_roots.retain(|root| {
            self.replay_keys.iter().any(|key| {
                self.changes
                    .get(key)
                    .and_then(|c| c.replay.as_ref())
                    .is_some_and(|origin| origin.id == root.id)
            })
        });
        // Selected keys are not work requests. Admit one caller at a time and
        // leave capacity for its first phase. Retry admission even with cached bounds.
        if !self.replay_roots.is_empty()
            || self.metrics().pending_work >= 31
            || self.available_external_jobs() == 0
        {
            return;
        }
        for key in &self.replay_keys {
            let Some(change) = self.changes.get(key) else {
                continue;
            };
            let Some(origin) = &change.replay else {
                continue;
            };
            let active = change.instance.as_ref().is_some_and(|id| {
                self.contents
                    .active
                    .iter()
                    .chain(&self.static_controls)
                    .any(|c| &c.content.placement.id == id)
            });
            if active {
                continue;
            }
            let descriptor_bytes = super::contents::placement_heap(origin)
                + usize::from(self.replay_roots.capacity() == 0) * std::mem::size_of::<Placement>();
            if descriptor_bytes > self.available_staging_bytes()
                || self
                    .metrics()
                    .active_cpu_bytes
                    .saturating_add(descriptor_bytes)
                    > ACTIVE_CPU
            {
                return;
            }
            if self.replay_roots.try_reserve_exact(1).is_err() {
                self.failure = Some("Changed destination descriptor allocation failed".into());
                return;
            }
            self.replay_roots.push(origin.clone());
            break;
        }
    }
    pub fn spawn_boundaries(&self) -> &[super::SpawnBoundary] {
        self.contents.state.boundaries()
    }
    pub fn contents_initialized(&self, id: &super::GeneratedId) -> bool {
        self.contents.initialized(id)
    }
    pub fn appearance(&self, id: &super::GeneratedId) -> &[super::NodeAppearance] {
        self.contents.appearance(id)
    }
    pub fn lamp_materials(
        &self,
        prototype: &SourceId,
        node: &SourceId,
        output: &mut Vec<SourceId>,
    ) -> Result<(), ReferenceError> {
        use super::spawn_rules::*;
        let mut controllers = Vec::new();
        self.contents.rules.children(
            &self.reference,
            prototype,
            "flashlightscript",
            true,
            &mut controllers,
        )?;
        for source in controllers {
            let f = self.contents.rules.get(&self.reference, source)?.fields;
            for lamp in array(f, "Lamps")? {
                let lamp = fields(lamp)?;
                if !array(lamp, "R")?.iter().any(|r| {
                    value_link(r)
                        .ok()
                        .and_then(|r| self.contents.rules.root(&self.reference, r).ok())
                        == Some(node)
                }) {
                    continue;
                }
                for field in ["mON", "mOFF"] {
                    if let Some(material) = optional(lamp, field)?.or(optional(f, field)?) {
                        if !output.contains(material) {
                            output.push(material.clone());
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn prop_material(&self, placement: &Placement) -> Option<(&SourceId, &SourceId)> {
        self.props.material(placement)
    }
    fn plan(&mut self) -> Result<(), ReferenceError> {
        let road_changed = self.planned_road_focus != self.road_focus;
        if road_changed {
            let road_distance =
                self.reference.distances["RoadGenDist"] as f64 + self.road_light.influence_margin();
            // One union query retains both support windows without copying full index arrays.
            let query = union(
                window(self.road_focus[0], road_distance),
                window(self.road_focus[1], road_distance),
            );
            let candidates = self.roads.candidate_count(query);
            let bytes = candidates.saturating_add(self.road_candidates.capacity())
                * std::mem::size_of::<u32>();
            if !self.admit_generation(bytes) {
                return Ok(());
            }
            self.road_candidates.clear();
            self.next_road = 0;
            self.next_geometry = 0;
            self.wanted_roads.clear();
            self.wanted_lights.clear();
            self.road_points.clear();
            self.road_candidates
                .try_reserve_exact(candidates)
                .map_err(|_| ReferenceError("Selected road index allocation failed".into()))?;
            self.roads.segments_in(query, &mut self.road_candidates);
            let radius = self.reference.distances["RoadGenDist"];
            let margin = self.road_light.influence_margin();
            self.selected_roads.retain(|r| {
                self.road_candidates.contains(&r.segment.index)
                    && (self.road_focus.into_iter().any(|p| {
                        source_distance_squared(p, r.segment.position()) <= radius * radius
                    }) || self.road_focus.into_iter().any(|p| {
                        intersects(r.segment.bounds, window(p, f64::from(radius) + margin))
                    }))
            });
            self.planned_road_focus = self.road_focus;
            if self.wanted_structures.len() > MAX_OBJECTS {
                return Err(ReferenceError("Source object count exceeds8192".into()));
            }
        }
        let buildings_changed = self.refresh_structures();
        self.terrain_context_dirty |= road_changed || buildings_changed;
        self.wanted_tiles.clear();
        if self.wanted_structures.len() > MAX_OBJECTS {
            return Err(ReferenceError(format!(
                "Source object count exceeds {MAX_OBJECTS}"
            )));
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
                    (point[0] as f32 / tile.size as f32).round_ties_even() as i64,
                    (-point[2] as f32 / tile.size as f32).round_ties_even() as i64,
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
        if let Some(work) = &mut self.terrain_work {
            if self.terrain_context_dirty || !self.wanted_tiles.contains(&work.key) {
                self.terrain_work = None;
            } else {
                work.request = self.request;
            }
        }
        if let Some(work) = &mut self.ready {
            if !work.renderer_reserved {
                let valid = if work.published.key.layer == WorldLayer::Road {
                    self.wanted_roads
                        .contains(&(work.published.key.unit[0] as u32))
                } else {
                    !self.terrain_context_dirty && self.wanted_tiles.contains(&work.published.key)
                };
                if valid {
                    work.request = self.request;
                } else {
                    self.discard_unreserved_surface();
                }
            }
        }
        self.terrain_context_dirty = false;
        self.selection_changes = self.changes.revision();
        self.plan_revision = self.request.revision;
        self.selection_inputs = self.contents.input_revision();
        Ok(())
    }
    fn discover_road(&mut self) {
        if self.available_external_jobs() == 0 {
            return;
        }
        let index = self.road_candidates[self.next_road];
        let existing = self
            .selected_roads
            .iter()
            .position(|r| r.segment.index == index);
        let segment = if let Some(i) = existing {
            self.selected_roads[i].segment
        } else {
            let segment = self.roads.prepare_segment(index, 8);
            self.last_road_work = self.roads.replay_work();
            let Some(segment) = segment else {
                return;
            };
            segment
        };
        let radius = self.reference.distances["RoadGenDist"];
        let geometry = self
            .road_focus
            .into_iter()
            .any(|p| source_distance_squared(p, segment.position()) <= radius * radius);
        let selected = geometry
            || self.road_focus.into_iter().any(|p| {
                intersects(
                    segment.bounds,
                    window(p, f64::from(radius) + self.road_light.influence_margin()),
                )
            });
        if !selected {
            self.next_road += 1;
            return;
        }
        let old_light = self
            .lights
            .iter()
            .find(|l| l.biome.is_some() && l.id.unit[0] == index as i64);
        let possible_light = existing.map_or(true, |i| self.selected_roads[i].light.is_some());
        let new_light = usize::from(possible_light && old_light.is_none());
        let old_poles = self
            .lights
            .iter()
            .filter(|l| {
                l.biome.is_some()
                    && !self.wanted_structures.iter().any(|p| p.id == l.id)
                    && !self.structures.iter().any(|p| p.placement.id == l.id)
            })
            .count();
        let new_pole = usize::from(
            new_light > 0
                && !self
                    .wanted_structures
                    .iter()
                    .any(|p| p.id.layer == WorldLayer::Lamp && p.id.unit[0] == index as i64)
                && !self.structures.iter().any(|p| {
                    p.placement.id.layer == WorldLayer::Lamp
                        && p.placement.id.unit[0] == index as i64
                }),
        );
        let required = self
            .metrics()
            .generated_objects
            .saturating_add(new_light + old_poles + new_pole);
        if required > MAX_OBJECTS {
            self.failure = Some(format!(
                "Source road {index} object budget exceeded: requested {required}, limit {MAX_OBJECTS}"
            ));
            return;
        }
        if existing.is_none() {
            if self.selected_roads.len() >= MAX_ROADS {
                self.failure = Some("Selected road descriptors exceed 512 roads".into());
                return;
            }
            // Reserve retained output, vector overlap, the light identity, and pole placement before selection.
            let prototype = self
                .reference
                .prototypes
                .iter()
                .find(|p| p.source == self.reference.road.prototypes[segment.biome as usize])
                .unwrap();
            let light_heap = self.road_light.retained_bytes()
                + prototype
                    .nodes
                    .iter()
                    .map(|n| n.source.file.capacity())
                    .sum::<usize>();
            let bytes = (self.selected_roads.capacity() + 1) * std::mem::size_of::<SelectedRoad>()
                + prototype.nodes.len() * std::mem::size_of::<super::road::RoadFrame>()
                + 2 * (std::mem::size_of::<super::WorldLight>() + light_heap)
                + (self.wanted_structures.capacity() + 1) * std::mem::size_of::<Placement>()
                + 2 * light_heap;
            if !self.admit_generation(bytes) {
                return;
            }
            if self.selected_roads.try_reserve_exact(1).is_err() {
                self.failure = Some("Selected road descriptor allocation failed".into());
                return;
            }
            if self.wanted_structures.try_reserve_exact(1).is_err() {
                self.failure = Some("Selected pole placement allocation failed".into());
                return;
            }
            let light = self.road_light.place(&self.reference, &segment);
            self.selected_roads.push(SelectedRoad { segment, light });
        }
        let retained = self
            .selected_roads
            .iter()
            .find(|r| r.segment.index == index)
            .unwrap();
        if let Some(light) = &retained.light {
            if !self.lights.iter().any(|l| l.id == light.id) {
                self.lights.push(light.clone());
            }
            self.wanted_lights.push(index);
        }
        self.refresh_poles();
        if self.metrics().generated_objects > MAX_OBJECTS {
            self.failure = Some(format!("Source object count exceeds {MAX_OBJECTS}"));
            return;
        }
        if geometry {
            if self.wanted_roads.len() == MAX_ROADS {
                self.failure = Some("Source road count exceeds 512".into());
                return;
            }
            self.wanted_roads.push(index);
            append_road_points(
                &mut self.road_points,
                segment,
                self.reference.road.def_point_range,
            );
        }
        self.next_road += 1;
    }
    fn prepare_road(&mut self) {
        if self.available_external_jobs() == 0 {
            return;
        }
        let index = self.wanted_roads[self.next_geometry];
        let segment = self
            .selected_roads
            .iter()
            .find(|r| r.segment.index == index)
            .unwrap()
            .segment;
        let key = SurfaceKey {
            layer: WorldLayer::Road,
            unit: [index as i64, 0],
        };
        if self.surfaces.iter().any(|s| s.key == key) {
            self.next_geometry += 1;
            return;
        }
        if !self
            .admissions
            .get(&self.reference.road.prototypes[segment.biome as usize])
            .is_some_and(|a| a.ready)
        {
            return;
        }
        let pending_bytes = super::road::road_surface_bound(&self.reference, segment.biome);
        if !self.can_request_independent(1) || !self.admit_generation(pending_bytes) {
            return;
        }
        let mut surface = WorldSurface::default();
        deform_road(&self.reference, &segment, &mut surface);
        self.next_geometry += 1;
        self.ready = Some(PreparedSurface {
            pending_bytes,
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
        if self.terrain_work.is_none() && self.available_external_jobs() == 0 {
            return;
        }
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
            if !self.terrain_ready[tier] {
                return;
            }
            // Reserve all output, border and context storage before constructing the job.
            let startup = self.startup.as_ref().map_or(0, |s| {
                s.foundations.len() * std::mem::size_of::<TerrainFoundation>()
                    + s.holes.len() * std::mem::size_of::<[[f32; 3]; 4]>()
            });
            let pending_bytes = TerrainSurfaceJob::allocation_bound(tile)
                + (self.placements.foundation_count()
                    + self.structure_helpers.count(&self.wanted_structures))
                    * 2
                    * (std::mem::size_of::<super::FoundationPoint>()
                        + std::mem::size_of::<TerrainFoundation>())
                + startup * 2
                + self.road_points.len() * std::mem::size_of::<TerrainRoadPoint>() * 2
                + 512;
            if !self.admit_generation(pending_bytes) {
                return;
            }
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
                self.structure_helpers
                    .append(&self.wanted_structures, &mut foundations);
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
                pending_bytes,
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
                pending_bytes: work.pending_bytes,
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
            .filter(|work| same_request(work.request, self.request))
            .map(|work| (work.request, &work.published))
    }
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn reserve_surface_renderer(
        &mut self,
        request: WorldRequest,
        key: SurfaceKey,
    ) -> Result<PipelineContinuation, ReferenceError> {
        let work = self
            .ready
            .as_ref()
            .filter(|work| same_request(work.request, request) && work.published.key == key)
            .ok_or_else(|| ReferenceError("Stale source renderer reservation".into()))?;
        if work.renderer_reserved {
            return Err(ReferenceError(
                "Source renderer already owns its claim".into(),
            ));
        }
        let metrics = self.metrics();
        if surface_render_cpu(&work.published.surface) > self.available_staging_bytes()
            || metrics
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
        Ok(PipelineContinuation::counted(RendererOwner::Surface(
            request, key,
        )))
    }
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn discard_surface_renderer(
        &mut self,
        claim: &mut PipelineContinuation,
    ) -> Result<(), ReferenceError> {
        let RendererOwner::Surface(request, key) = claim.owner else {
            return Err(ReferenceError("Wrong surface claim".into()));
        };
        if !self
            .ready
            .as_ref()
            .is_some_and(|w| same_request(w.request, request) && w.published.key == key)
        {
            return Err(ReferenceError("Missing draining surface claim".into()));
        }
        claim.release()?;
        self.ready = None;
        self.rewind_road(key);
        Ok(())
    }
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn accept_surface_renderer(
        &mut self,
        request: WorldRequest,
        key: SurfaceKey,
        claim: &PipelineContinuation,
    ) -> Result<(), ReferenceError> {
        let work = self
            .ready
            .as_mut()
            .filter(|work| same_request(work.request, request) && work.published.key == key)
            .ok_or_else(|| ReferenceError("Stale source renderer preparation".into()))?;
        if !work.renderer_reserved
            || !claim.drained()
            || !matches!(claim.owner, RendererOwner::Surface(r, owner) if same_request(r, request) && owner == key)
        {
            return Err(ReferenceError("Unreserved source renderer".into()));
        }
        work.renderer_ready = true;
        Ok(())
    }
    /// Publish at most one completed unit. Stale work never changes accepted collision.
    pub fn publish(
        &mut self,
        physics: &mut PhysicsWorld,
        required_lights: &[super::GeneratedId],
    ) -> Result<(), ReferenceError> {
        let result = self.publish_unit(physics, required_lights);
        if result.is_ok() && self.failure.is_none() {
            self.retire_surfaces(physics);
        }
        result
    }
    fn publication_light(
        &self,
        light: &super::WorldLight,
        required: &[super::GeneratedId],
    ) -> bool {
        required.contains(&light.id)
            || (light.biome.is_some()
                && self.ready.as_ref().is_some_and(|work| {
                    work.published.key.layer == WorldLayer::Road
                        && work.published.key.unit[0] == light.id.unit[0]
                }))
    }
    fn publish_unit(
        &mut self,
        physics: &mut PhysicsWorld,
        required_lights: &[super::GeneratedId],
    ) -> Result<(), ReferenceError> {
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
            self.discard_unreserved_surface();
            return Ok(());
        }
        // Hold the surface until every required owner publishes. Drain individually ready
        // structures meanwhile, so unrelated owners cannot fill the shared pending queue.
        // Each structure receipt already includes geometry, materials and caster pipelines.
        if self
            .lights
            .iter()
            .filter(|light| self.publication_light(light, required_lights))
            .any(|light| {
                !self
                    .structures
                    .iter()
                    .any(|s| light.owned_by(&s.placement.id))
            })
        {
            return self.publish_structure(physics, required_lights);
        }
        let Some(work) = self.ready.as_ref() else {
            return self.publish_structure(physics, required_lights);
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
        Ok(())
    }
    fn retire_surfaces(&mut self, physics: &mut PhysicsWorld) {
        // Keep old support until both new receiver windows have published near terrain.
        if self.recovery.is_none()
            && self.next_road == self.road_candidates.len()
            && self.supports(window(self.request.player, 3.0))
            && self.supports(window(self.request.vehicle, 4.0))
        {
            let distances = &self.reference.distances;
            let focus = [self.request.player, self.request.vehicle];
            let tiles = self.tiles;
            let mut removed = [false; 4];
            self.surfaces.retain(|surface| {
                let tier = if surface.key.layer == WorldLayer::Road {
                    0
                } else {
                    layer_tier(surface.key.layer) + 1
                };
                let inside = if tier == 0 {
                    self.wanted_roads.contains(&(surface.key.unit[0] as u32))
                } else {
                    let size = tiles[tier - 1].size as f32;
                    let field = [
                        "TerrainRemoveDist",
                        "DistantTerrainRemoveDist",
                        "DistantTerrain2RemoveDist",
                    ][tier - 1];
                    let radius = distances[field];
                    let center = [
                        surface.key.unit[0] as f32 * size,
                        surface.key.unit[1] as f32 * size,
                    ];
                    focus.into_iter().any(|p| {
                        let x = (p[0] as f32 / size).round_ties_even() * size;
                        let z = (-p[2] as f32 / size).round_ties_even() * size;
                        let dx = center[0] - x;
                        let dz = center[1] - z;
                        dx * dx + dz * dz < radius * radius
                    })
                };
                // The source removes at most one object per tier update.
                let keep = inside || removed[tier];
                if !keep {
                    removed[tier] = true;
                }
                if !keep {
                    if let Some(handle) = surface.collider {
                        physics.remove_collider(handle);
                    }
                }
                keep
            });
        }
        if self.recovery.is_none() && self.next_road == self.road_candidates.len() {
            self.lights.retain(|light| {
                light.biome.is_none() || self.wanted_lights.contains(&(light.id.unit[0] as u32))
            });
            self.refresh_poles();
        }
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
        if self.lights.iter().any(|light| {
            light.influences(swept)
                && !self
                    .structures
                    .iter()
                    .any(|s| light.owned_by(&s.placement.id))
        }) {
            return false;
        }
        self.ground_supports(swept)
    }
    /// Near terrain colliders cover the bounds. Structures and lights may still be pending.
    pub fn ground_supports(&self, swept: WorldBounds) -> bool {
        if !self.indices_ready
            || ![swept.min[0], swept.min[2], swept.max[0], swept.max[2]]
                .into_iter()
                .all(f64::is_finite)
        {
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
    fn refresh_helper_context(&mut self, bounds: WorldBounds) {
        let required = self.placements.foundation_count()
            + self.startup.as_ref().map_or(0, |s| s.foundations.len())
            + self.structure_helpers.count(&self.wanted_structures);
        let extra = required.saturating_sub(self.color_helpers.capacity())
            * std::mem::size_of::<TerrainFoundation>();
        if self.metrics().active_cpu_bytes.saturating_add(extra) > ACTIVE_CPU {
            self.failure = Some("Source helper context exceeds active CPU budget".into());
            return;
        }
        self.placements
            .foundations_in(bounds, &mut self.color_foundations);
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
        self.structure_helpers
            .append(&self.wanted_structures, &mut self.color_helpers);
    }
    pub fn terrain_color(&mut self, point: [f64; 3]) -> [f32; 4] {
        self.refresh_helper_context(window(point, 1000.));
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
    /// Play can start when the ground under the player and vehicle is ready.
    /// The rest of the window streams while playing.
    pub fn start_ready(&self) -> bool {
        self.failure.is_none()
            && self.plan_revision == self.request.revision
            && self.supports(window(self.request.player, SUPPORT_RADIUS))
            && self.supports(window(self.request.vehicle, SUPPORT_RADIUS))
    }
    pub fn recovery_window(&self) -> Option<WorldBounds> {
        self.recovery
            .map(|pending| window(pending.target.position, SUPPORT_RADIUS))
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
        let owner_bytes = super::metrics::map_insert_bytes(&self.recipes) + source.file.len();
        if self
            .metrics()
            .prototype_cpu_bytes
            .saturating_add(bytes)
            .saturating_add(owner_bytes)
            > PROTOTYPE_CPU
        {
            return Err(ReferenceError(
                "Source collision recipe exceeds prototype CPU budget".into(),
            ));
        }
        let recipe = super::StructureRecipe::new(&self.reference, prototype)?;
        self.recipes.insert(source.clone(), recipe);
        Ok(())
    }

    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn reserve_instance_renderer(
        &mut self,
        request: WorldRequest,
        id: &super::GeneratedId,
        cpu: usize,
        gpu: usize,
        objects: usize,
    ) -> Result<PipelineContinuation, ReferenceError> {
        if request.session != self.request.session
            || !self.wanted_structures.iter().any(|p| p.id == *id)
        {
            return Err(ReferenceError("Stale instance renderer admission".into()));
        }
        let mut next = self.presentation;
        next.active_cpu += cpu;
        next.active_gpu += gpu;
        next.objects += objects;
        next.pending += 1;
        self.set_presentation_allocation(next)?;
        Ok(PipelineContinuation::counted(RendererOwner::Structure(
            request,
            id.clone(),
        )))
    }
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn reserve_grass_renderer(
        &mut self,
        entity: u64,
        cpu: usize,
    ) -> Result<PipelineContinuation, ReferenceError> {
        let mut next = self.presentation;
        next.active_cpu += cpu;
        next.pending += 1;
        self.set_presentation_allocation(next)?;
        Ok(PipelineContinuation::counted(RendererOwner::Grass(
            self.request,
            entity,
        )))
    }
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn release_renderer_pending(
        &mut self,
        claim: &mut PipelineContinuation,
    ) -> Result<(), ReferenceError> {
        if matches!(claim.owner, RendererOwner::Surface(..)) {
            return Err(ReferenceError(
                "Surface claim belongs to its ready owner".into(),
            ));
        }
        if !claim.drained() {
            return Err(ReferenceError("Renderer physical work remains live".into()));
        }
        let mut next = self.presentation;
        next.pending = next
            .pending
            .checked_sub(1)
            .ok_or_else(|| ReferenceError("Missing renderer pending claim".into()))?;
        // Validate before changing the single-use proof.
        for budget in self.presentation_budgets(next) {
            if budget.used > budget.limit {
                return Err(ReferenceError("Renderer release exceeds budget".into()));
            }
        }
        claim.release()?;
        self.presentation = next;
        Ok(())
    }
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn accept_structure_renderer(
        &mut self,
        request: WorldRequest,
        id: &super::GeneratedId,
        claim: &mut PipelineContinuation,
    ) -> Result<(), ReferenceError> {
        if request.session != self.request.session
            || !self.wanted_structures.iter().any(|p| p.id == *id)
            || !matches!(&claim.owner, RendererOwner::Structure(r, owner) if *owner == *id && same_request(*r, request))
            || self.ready_structures.contains(id)
            || self.ready_structures.len() >= 32
            || !claim.drained()
        {
            return Err(ReferenceError(
                "Stale or live structure renderer claim".into(),
            ));
        }
        let pending = self
            .presentation
            .pending
            .checked_sub(1)
            .ok_or_else(|| ReferenceError("Missing instance renderer claim".into()))?;
        // No caller can observe the ready queue without its matching claim transfer.
        claim.release()?;
        self.presentation.pending = pending;
        self.ready_structures.push(id.clone());
        debug_assert!(self.metrics().pending_work <= 32);
        Ok(())
    }
    fn release_structure_renderer(&mut self, id: &super::GeneratedId) -> bool {
        let count = self.ready_structures.len();
        self.ready_structures.retain(|entry| entry != id);
        self.ready_structures.len() != count
    }
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn release_grass_renderer(
        &mut self,
        entity: u64,
        claim: &mut PipelineContinuation,
    ) -> Result<(), ReferenceError> {
        if !matches!(claim.owner, RendererOwner::Grass(_, owner) if owner == entity) {
            return Err(ReferenceError("Wrong Grass renderer claim".into()));
        }
        self.release_renderer_pending(claim)
    }
    fn publish_structure(
        &mut self,
        physics: &mut PhysicsWorld,
        required_lights: &[super::GeneratedId],
    ) -> Result<(), ReferenceError> {
        if self.plan_revision != self.request.revision {
            return Ok(());
        }
        let ready = |placement: &&Placement| {
            !self
                .structures
                .iter()
                .any(|instance| instance.placement.id == placement.id)
                && self.recipes.contains_key(&placement.prototype)
                && self.ready_structures.contains(&placement.id)
        };
        let Some(placement) = self
            .wanted_structures
            .iter()
            .filter(ready)
            .find(|placement| {
                self.lights.iter().any(|light| {
                    self.publication_light(light, required_lights) && light.owned_by(&placement.id)
                })
            })
            .or_else(|| {
                // Nearest structures publish first, so the start area is ready early.
                let request = self.request;
                self.wanted_structures.iter().filter(ready).min_by(|a, b| {
                    focus_distance_squared(request, a.pose.position)
                        .total_cmp(&focus_distance_squared(request, b.pose.position))
                })
            })
        else {
            return Ok(());
        };
        let mut points = Vec::new();
        if placement.id.layer == WorldLayer::Building && placement.id.children.is_empty() {
            self.placements
                .supplies_for_building(placement.id.unit[0] as u32, &mut points);
        }
        if self.metrics().generated_objects + 1 + points.len() * 2 > MAX_OBJECTS {
            return Err(ReferenceError(format!(
                "Source object count exceeds {MAX_OBJECTS}"
            )));
        }
        if self
            .metrics()
            .active_cpu_bytes
            .saturating_add(self.recipes[&placement.prototype].instance_bytes())
            .saturating_add(super::contents::placement_heap(placement))
            > ACTIVE_CPU
        {
            return Err(ReferenceError(
                "Source structure physics exceeds active CPU budget".into(),
            ));
        }
        let prototype = self
            .reference
            .prototypes
            .iter()
            .find(|p| p.source == placement.prototype)
            .unwrap();
        let (binding_reservation, binding_objects) =
            self.binding_reservation(prototype, &placement.id)?;
        if binding_reservation > self.available_dependency_staging_bytes()
            || self
                .metrics()
                .active_cpu_bytes
                .saturating_add(binding_reservation)
                .saturating_add(self.recipes[&placement.prototype].instance_bytes())
                .saturating_add(super::contents::placement_heap(placement))
                > ACTIVE_CPU
            || self
                .metrics()
                .generated_objects
                .saturating_add(binding_objects)
                > MAX_OBJECTS
        {
            return Err(ReferenceError(
                "Source generated bindings exceed admission budget".into(),
            ));
        }
        let appearance = self.contents.appearance(&placement.id);
        let instance = if placement.scale == prototype.prefab_pose.scale
            && !appearance.iter().any(|a| a.scale.is_some())
        {
            self.recipes[&placement.prototype].publish(placement.clone(), self.origin, physics)?
        } else {
            let extra = super::StructureRecipe::reservation(prototype);
            if self.metrics().active_cpu_bytes.saturating_add(extra * 2) > ACTIVE_CPU
                || extra + binding_reservation > self.available_dependency_staging_bytes()
            {
                return Err(ReferenceError(
                    "Source scaled collision exceeds allocation budget".into(),
                ));
            }
            let recipe = super::StructureRecipe::with_appearance(
                &self.reference,
                prototype,
                placement.scale,
                appearance,
            )?;
            let mut instance = recipe.publish(placement.clone(), self.origin, physics)?;
            instance.cpu_bytes += extra;
            instance
        };
        let appearance = self.contents.appearance(&placement.id);
        for (index, body) in prototype.bodies.iter().enumerate() {
            physics.rapier_mut().bodies[instance.bodies[index]].set_enabled(
                super::spawn_appearance::node_active(prototype, body.node as usize, appearance),
            );
        }
        for (source, handle) in &instance.colliders {
            let collider = prototype
                .colliders
                .iter()
                .find(|c| &c.source == source)
                .unwrap();
            physics.rapier_mut().colliders[*handle].set_enabled(
                collider.enabled
                    && super::spawn_appearance::node_active(
                        prototype,
                        collider.node as usize,
                        appearance,
                    ),
            );
        }
        for point in points {
            self.supplies.push(PublishedSupply {
                key: placement.id.clone(),
                point,
                definition: self.placements.supply_definition(&point).clone(),
                items: None,
                baselines: None,
            });
        }
        self.release_structure_renderer(&instance.placement.id);
        self.structures.push(instance);
        self.binding_reserved = binding_reservation;
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
        wheels: &mut crate::interaction::wheels::WheelAssemblies,
    ) -> Result<(), ReferenceError> {
        let owned = items.retained_binding_bytes()
            + interaction.retained_binding_bytes()
            + wheels.retained_binding_bytes();
        let baseline = *self.binding_baseline.get_or_insert(owned);
        self.binding_bytes = owned.saturating_sub(baseline);
        self.binding_owner_bytes = owned;
        self.binding_objects = items.len();
        if let Err(error) =
            self.sync_static_controls(reference, items, interaction, physics, wheels)
        {
            self.binding_reserved = 0;
            self.binding_owner_bytes = items.retained_binding_bytes()
                + interaction.retained_binding_bytes()
                + wheels.retained_binding_bytes();
            self.binding_bytes = self.binding_owner_bytes.saturating_sub(baseline);
            return Err(error);
        }
        if let Err(error) = self.sync_contents(reference, items, interaction, physics, wheels) {
            self.binding_reserved = 0;
            self.binding_owner_bytes = items.retained_binding_bytes()
                + interaction.retained_binding_bytes()
                + wheels.retained_binding_bytes();
            self.binding_bytes = self.binding_owner_bytes.saturating_sub(baseline);
            return Err(error);
        }
        if let Err(error) = self.sync_lamp_bindings(interaction, physics) {
            self.binding_reserved = 0;
            self.binding_owner_bytes = items.retained_binding_bytes()
                + interaction.retained_binding_bytes()
                + wheels.retained_binding_bytes();
            self.binding_bytes = self.binding_owner_bytes.saturating_sub(baseline);
            return Err(error);
        }
        self.sync_moving_lamps(items);
        self.sync_generated_food(reference, items, physics);
        self.binding_bytes = (items.retained_binding_bytes()
            + interaction.retained_binding_bytes()
            + wheels.retained_binding_bytes())
        .saturating_sub(baseline);

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
                if let Some(change) = self
                    .changes
                    .get(super::ObjectKey::Generated(key))
                    .and_then(|c| c.snapshot.as_ref())
                {
                    items.restore_generated(id, change)?;
                    if let Some(valve) = change.valve {
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
        physics.refresh_queries();
        self.binding_owner_bytes = items.retained_binding_bytes()
            + interaction.retained_binding_bytes()
            + wheels.retained_binding_bytes();
        self.binding_bytes = self.binding_owner_bytes.saturating_sub(baseline);
        self.binding_objects = items.len();
        self.binding_reserved = 0;
        Ok(())
    }

    pub fn sync_generated_food(
        &self,
        reference: &ReferenceManifest,
        items: &crate::interaction::items::ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        for content in &self.contents.active {
            let Some(item) = content.item.and_then(|id| items.get(id)) else {
                continue;
            };
            let Some(food) = item.food else { continue };
            let Some(instance) = self
                .structures
                .iter()
                .find(|i| i.placement.id == content.content.placement.id)
            else {
                continue;
            };
            let recipe = &reference.survival.foods[food.prototype_index];
            for (source, handle) in &instance.colliders {
                if let Some(collider) = recipe.colliders.iter().find(|c| &c.source == source) {
                    physics.rapier_mut().colliders[*handle].set_enabled(
                        content.restored
                            && !item.removed
                            && collider.enabled
                            && crate::starting_area::active_food_part(
                                reference,
                                collider.node,
                                &recipe.parts,
                                food.bite,
                            ),
                    );
                }
            }
        }
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
        wheels: &mut crate::interaction::wheels::WheelAssemblies,
    ) {
        if self.recovery.is_some()
            || self.plan_revision != self.request.revision
            || !self.supports(window(self.request.player, 3.0))
            || !self.supports(window(self.request.vehicle, 4.0))
        {
            return;
        }
        self.retire_contents(physics, items, interaction, wheels);
        let mut index = 0;
        while index < self.structures.len() {
            let id = &self.structures[index].placement.id;
            let radius = self.reference.distances["BuildingGenDist"];
            if self.contents.contains(id)
                || self
                    .static_controls
                    .iter()
                    .filter(|c| &c.content.placement.id == id)
                    .any(|c| c.has_external_dependents(items, &interaction.parts, wheels))
                || (!id.children.is_empty() && self.wanted_structures.iter().any(|p| &p.id == id))
                || (id.children.is_empty()
                    && id.layer != WorldLayer::Building
                    && self.props.contains(id))
                || (id.children.is_empty()
                    && id.layer == WorldLayer::Building
                    && [self.request.player, self.request.vehicle]
                        .into_iter()
                        .any(|p| {
                            source_distance_squared(
                                p,
                                self.structures[index].placement.pose.position,
                            ) < radius * radius
                        }))
                || self.supplies.iter().filter(|s| &s.key == id).any(|s| {
                    s.items.is_some_and(|(tank, cap)| {
                        !items.can_remove_generated(tank)
                            || !items.can_remove_generated(cap)
                            || items.has_external_dependents(|item| item == tank || item == cap)
                            || interaction
                                .parts
                                .has_external_dependents(|item| item == tank || item == cap)
                            || wheels.has_external_dependents(|item| item == tank || item == cap)
                    })
                })
            {
                index += 1;
                continue;
            }
            let instance = self.structures.swap_remove(index);
            if let Some(i) = self
                .static_controls
                .iter()
                .position(|c| c.content.placement.id == instance.placement.id)
            {
                let mut content = self.static_controls.swap_remove(i);
                content_bindings::rollback(
                    &mut content,
                    &instance,
                    items,
                    interaction,
                    physics,
                    wheels,
                );
            }
            let mut supply_index = 0;
            while supply_index < self.supplies.len() {
                if self.supplies[supply_index].key != instance.placement.id {
                    supply_index += 1;
                    continue;
                }
                let supply = self.supplies.swap_remove(supply_index);
                if let Some((tank, cap)) = supply.items {
                    for item in [tank, cap].into_iter() {
                        for (_, collider) in &instance.colliders {
                            interaction.unbind_generated(item, std::slice::from_ref(collider));
                        }
                        items.remove_generated(item);
                    }
                }
            }
            instance.retire(physics);
            break;
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
            && self.supports(window(pose.position, SUPPORT_RADIUS))
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

    /// True when the placement overlaps the ground that `start_ready` waits for.
    pub fn in_start_area(&self, placement: &Placement) -> bool {
        [self.request.player, self.request.vehicle]
            .into_iter()
            .any(|point| {
                let area = window(point, SUPPORT_RADIUS);
                [0, 2].into_iter().all(|axis| {
                    placement.bounds.min[axis] <= area.max[axis]
                        && placement.bounds.max[axis] >= area.min[axis]
                })
            })
    }
    /// Squared source distance from the nearer of the player and vehicle.
    pub fn focus_distance_squared(&self, position: [f64; 3]) -> f32 {
        focus_distance_squared(self.request, position)
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
        let imported_snapshot_bytes = self.imported_baselines.capacity()
            * std::mem::size_of::<(crate::reference::ItemId, super::ObjectSnapshot)>()
            + self
                .imported_baselines
                .iter()
                .map(|(_, s)| s.heap_bytes())
                .sum::<usize>();
        let mut metrics = WorldMetrics {
            shared_input_cpu_bytes: self.shared_cpu_bytes.unwrap_or(0) + self.dependency_metadata,
            imported_snapshot_bytes,
            road_checkpoints: self.roads.checkpoint_count(),
            building_checkpoints: self.placements.checkpoint_count(),
            road_replay_ms: self.roads.replay_ms(),
            peak_road_replay_ms: self.roads.peak_replay_ms(),
            generation_jobs: self.presentation.jobs
                + usize::from(self.terrain_work.is_some())
                + usize::from(self.contents.has_replay()),
            pending_bytes: self.presentation.staging_bytes + self.generation_pending_bytes(),
            pending_work: self
                .replay_roots
                .len()
                .max(usize::from(self.contents.has_replay()))
                + self.contents.pending_requests()
                + self.props.pending_requests()
                + self.ready_structures.len()
                + usize::from(self.ready.is_some())
                + usize::from(self.terrain_work.is_some())
                + self.presentation.pending,
            spawn_parents: self.contents.state.parent_count(),
            spawn_boundaries: self.contents.state.boundary_count(),
            spawn_boundary_bytes: self.contents.state.retained_bytes(),
            checkpoint_bytes: self.contents.state.retained_bytes()
                + self.roads.retained_index_bytes()
                + self.placements.retained_index_bytes(),
            prototype_cpu_bytes: super::metrics::map_bytes(
                self.admissions.capacity(),
                std::mem::size_of::<(SourceId, PrototypeAllocation)>(),
            ) + self
                .admissions
                .keys()
                .map(|id| id.file.capacity())
                .sum::<usize>()
                + super::metrics::map_bytes(
                    self.recipes.capacity(),
                    std::mem::size_of::<(SourceId, super::StructureRecipe)>(),
                )
                + self
                    .recipes
                    .keys()
                    .map(|id| id.file.capacity())
                    .sum::<usize>()
                + self.road_light.retained_bytes()
                + self.contents.rules.retained_bytes()
                + self.structure_helpers.retained_bytes()
                + self.shared_cpu_bytes.unwrap_or(0)
                + self.presentation.prototype_cpu
                + self
                    .recipes
                    .values()
                    .map(|recipe| recipe.cpu_bytes)
                    .sum::<usize>(),
            prototype_gpu_bytes: self.presentation.prototype_gpu,
            active_gpu_bytes: self.presentation.active_gpu,
            changed_record_bytes: self.changes.retained_bytes(),
            changed_objects: self.changes.len(),
            active_cpu_bytes: std::mem::size_of::<Self>()
                + imported_snapshot_bytes
                + self.replay_roots.capacity() * std::mem::size_of::<Placement>()
                + self
                    .replay_roots
                    .iter()
                    .map(super::contents::placement_heap)
                    .sum::<usize>()
                + (self.replay_keys.capacity() + self.replay_query.capacity())
                    * std::mem::size_of::<super::ObjectKey>()
                + self
                    .replay_keys
                    .iter()
                    .chain(&self.replay_query)
                    .map(super::changes::key_heap)
                    .sum::<usize>()
                + self.static_controls.capacity()
                    * std::mem::size_of::<super::contents::ContentInstance>()
                + self
                    .static_controls
                    .iter()
                    .map(super::contents::ContentInstance::control_heap_bytes)
                    .sum::<usize>()
                + self.binding_reserved
                + self.binding_bytes
                + self.lamp_bindings.capacity() * std::mem::size_of::<content_lamps::LampBinding>()
                + self
                    .lamp_bindings
                    .iter()
                    .map(content_lamps::LampBinding::heap_bytes)
                    .sum::<usize>()
                + self.contents.retained_bytes()
                + self.props.retained_bytes()
                + self.presentation.active_cpu
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
                    .map(super::contents::identity_heap)
                    .sum::<usize>()
                + self.selected_roads.capacity() * std::mem::size_of::<SelectedRoad>()
                + self
                    .selected_roads
                    .iter()
                    .filter_map(|r| r.light.as_ref())
                    .map(super::WorldLight::heap_bytes)
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
                + self
                    .lights
                    .iter()
                    .map(super::WorldLight::heap_bytes)
                    .sum::<usize>()
                + self.structures.capacity() * std::mem::size_of::<super::StructureInstance>()
                + self.supplies.capacity() * std::mem::size_of::<PublishedSupply>()
                + self.wanted_structures.capacity() * std::mem::size_of::<Placement>()
                + self
                    .wanted_structures
                    .iter()
                    .map(super::contents::placement_heap)
                    .sum::<usize>(),
            prepared_roads: self.roads.prepared_segments(),
            prepared_buildings: self.placements.prepared_buildings(),
            indices_ready: self.indices_ready,
            last_road_work: self.last_road_work,
            last_terrain_work: self.last_terrain_work,
            generated_objects: self
                .contents
                .active
                .iter()
                .chain(&self.static_controls)
                .map(|c| c.bindings.len() + usize::from(c.item.is_some()))
                .sum::<usize>()
                + self
                    .contents
                    .active
                    .iter()
                    .filter(|c| {
                        c.item.is_none()
                            && !self
                                .wanted_structures
                                .iter()
                                .any(|p| p.id == c.content.placement.id)
                    })
                    .count()
                + self.structures.len()
                + self
                    .wanted_structures
                    .iter()
                    .filter(|p| !self.structures.iter().any(|s| s.placement.id == p.id))
                    .count()
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
            let bytes = surface_bytes(&surface.surface)
                + surface
                    .prototype
                    .as_ref()
                    .map_or(0, |id| id.file.capacity());
            let render_bytes = surface_render_bytes(&surface.surface);
            metrics.active_cpu_bytes +=
                bytes + surface_render_cpu(&surface.surface) + surface.collision_bytes;
            metrics.active_gpu_bytes += render_bytes;
        }
        metrics.active_cpu_bytes += self.failure.as_ref().map_or(0, String::capacity);
        for supply in &self.supplies {
            metrics.active_cpu_bytes += super::contents::identity_heap(&supply.key)
                + supply.definition.tank.file.capacity()
                + supply.definition.cap.file.capacity()
                + supply.definition.fluids.capacity() * std::mem::size_of::<super::SupplyFluid>()
                + supply.baselines.as_ref().map_or(0, |values| {
                    values
                        .iter()
                        .map(super::ObjectSnapshot::heap_bytes)
                        .sum::<usize>()
                });
        }
        if let Some(work) = &self.ready {
            metrics.active_cpu_bytes += work.pending_bytes
                + work
                    .published
                    .prototype
                    .as_ref()
                    .map_or(0, |id| id.file.capacity());
            if work.renderer_reserved {
                metrics.active_cpu_bytes += surface_render_cpu(&work.published.surface);
                metrics.active_gpu_bytes += surface_render_bytes(&work.published.surface);
            }
        }
        if let Some(work) = &self.terrain_work {
            metrics.active_cpu_bytes += work.pending_bytes;
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
        metrics.active_cpu_bytes += self.overlap.active_cpu_bytes;
        metrics.active_gpu_bytes += self.overlap.active_gpu_bytes;
        metrics.prototype_cpu_bytes += self.overlap.prototype_cpu_bytes;
        metrics.prototype_gpu_bytes += self.overlap.prototype_gpu_bytes;
        metrics.pending_bytes += self.overlap.pending_bytes;
        metrics.pending_work += self.overlap.pending_work;
        metrics.generation_jobs += self.overlap.generation_jobs;
        metrics.generated_objects += self.overlap.generated_objects;
        metrics.checkpoint_bytes += self.overlap.checkpoint_bytes;
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
    surface_render_bytes(surface) * 2 + surface.submeshes.len() * 4096
}
fn surface_render_bytes(surface: &WorldSurface) -> usize {
    let attributes = surface.vertices.len() * (12 + 32)
        + surface.normals.len() * 12
        + surface.tangents.len() * 16
        + surface.uv.len() * 8
        + surface.colors.len() * 16;
    attributes
        .saturating_mul(surface.submeshes.len())
        .saturating_add(surface.triangles.len() * 12)
        .saturating_add(surface.submeshes.len() * 8)
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

fn focus_distance_squared(request: WorldRequest, position: [f64; 3]) -> f32 {
    source_distance_squared(request.player, position)
        .min(source_distance_squared(request.vehicle, position))
}
// Frozen source coordinates, with source float vector boundaries.
fn source_distance_squared(a: [f64; 3], b: [f64; 3]) -> f32 {
    let x = a[0] as f32 - b[0] as f32;
    let z = (-a[2]) as f32 - (-b[2]) as f32;
    x * x + z * z
}

#[path = "content_bindings.rs"]
pub(crate) mod content_bindings;

#[path = "content_lamps.rs"]
mod content_lamps;
