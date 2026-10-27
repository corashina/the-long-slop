//! The renderer view of a world stream. The renderer reads only this copy.
//! `WorldStream::sync_renderer` updates it after each world step. It allocates only for changed data.
use super::{PublishedSurface, SurfaceKey, WorldRequest, WorldStream};
use crate::reference::{ReferenceError, ReferenceManifest, SourceId, WorldReference};
use crate::world::{
    GeneratedId, NodeAppearance, Placement, StartupTerrain, SupplyDefinition, SupplyPoint,
    WorldBounds, WorldLayer, WorldLight, WorldOrigin, WorldSurface,
};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub struct RendererSurface {
    pub key: SurfaceKey,
    pub surface: Arc<WorldSurface>,
    pub prototype: Option<SourceId>,
    pub bounds: WorldBounds,
}
impl RendererSurface {
    fn of(published: &PublishedSurface) -> Self {
        Self {
            key: published.key,
            surface: published.surface.clone(),
            prototype: published.prototype.clone(),
            bounds: published.bounds,
        }
    }
    fn same(&self, published: &PublishedSurface) -> bool {
        self.key == published.key && Arc::ptr_eq(&self.surface, &published.surface)
    }
}
pub struct RendererStructure {
    pub fixed_decoration: bool,
    pub cpu_bytes: usize,
    pub placement: Placement,
}
pub struct RendererSupply {
    pub key: GeneratedId,
    pub point: SupplyPoint,
    pub definition: SupplyDefinition,
    pub items: Option<(crate::reference::ItemId, crate::reference::ItemId)>,
}
pub struct RendererFrame<'a> {
    pub request: WorldRequest,
    pub origin: WorldOrigin,
    pub surfaces: &'a [RendererSurface],
    pub lights: &'a [WorldLight],
    pub structures: &'a [RendererStructure],
    pub supplies: &'a [RendererSupply],
    pub startup: Option<&'a StartupTerrain>,
    pub failure: Option<&'a str>,
}
/// Lamp material variants by prototype and node.
type LampChoices = Vec<(SourceId, SourceId, Arc<[SourceId]>)>;
/// Fixed lookups from the world reference. Copies of one world share them.
struct Lookups {
    reference: Arc<WorldReference>,
    rules: crate::world::spawn_rules::SourceRules,
    biome_materials: crate::world::biome_materials::BiomeMaterials,
    /// Lamp material variants by prototype and node, filled by reads.
    lamp_materials: Mutex<LampChoices>,
}
/// Values of a pending or published structure placement that need the stream to compute.
struct PlacementValues {
    /// Biome of the terrain color at the placement position.
    biome: u8,
    prop_material: Option<(SourceId, SourceId)>,
    mark: u64,
}
struct ContentValues {
    appearance: Vec<NodeAppearance>,
    mark: u64,
}
pub struct RendererWorld {
    lookups: Arc<Lookups>,
    request: WorldRequest,
    pub origin: WorldOrigin,
    failure: Option<String>,
    surfaces: Vec<RendererSurface>,
    prepared: Option<(WorldRequest, RendererSurface)>,
    lights: Vec<WorldLight>,
    structures: Vec<RendererStructure>,
    supplies: Vec<RendererSupply>,
    startup: Option<StartupTerrain>,
    pending: Vec<Placement>,
    /// Content initialization of each pending structure.
    initialized: Vec<bool>,
    placements: HashMap<GeneratedId, PlacementValues>,
    contents: HashMap<GeneratedId, ContentValues>,
    /// Pole rope appearance by road segment index.
    cables: Vec<(i64, Vec<NodeAppearance>)>,
    required_structures: Vec<GeneratedId>,
    required_lights: Vec<GeneratedId>,
    /// Structures that change replay needs.
    replay: Vec<GeneratedId>,
    required_ready: bool,
    membership: (u64, u64, u64),
    /// Road prototypes by biome that selected roads need.
    road_required: [bool; 3],
    /// Road prototypes by index that the stream has built.
    road_loaded: [bool; 3],
    holds: HashSet<SourceId>,
    hold_insert_bytes: usize,
    prototype_cpu_bytes: usize,
    staging_configured: bool,
    mark: u64,
}

impl RendererWorld {
    pub fn new(stream: &WorldStream) -> Self {
        let reference = stream.reference.clone();
        Self::with_lookups(Arc::new(Lookups {
            rules: crate::world::spawn_rules::SourceRules::new(&reference),
            biome_materials: crate::world::biome_materials::BiomeMaterials::new(
                &reference.biome_material_sets,
            ),
            reference,
            lamp_materials: Mutex::default(),
        }))
    }
    /// An empty copy that shares the fixed lookups.
    pub fn fork(&self) -> Self {
        Self::with_lookups(self.lookups.clone())
    }
    fn with_lookups(lookups: Arc<Lookups>) -> Self {
        Self {
            lookups,
            request: WorldRequest {
                session: 0,
                revision: 0,
                player: [0.; 3],
                vehicle: [0.; 3],
            },
            origin: WorldOrigin::default(),
            failure: None,
            surfaces: Vec::new(),
            prepared: None,
            lights: Vec::new(),
            structures: Vec::new(),
            supplies: Vec::new(),
            startup: None,
            pending: Vec::new(),
            initialized: Vec::new(),
            placements: HashMap::new(),
            contents: HashMap::new(),
            cables: Vec::new(),
            required_structures: Vec::new(),
            required_lights: Vec::new(),
            replay: Vec::new(),
            required_ready: false,
            membership: (0, u64::MAX, u64::MAX),
            road_required: [false; 3],
            road_loaded: [false; 3],
            holds: HashSet::new(),
            hold_insert_bytes: 0,
            prototype_cpu_bytes: 0,
            staging_configured: false,
            mark: 0,
        }
    }
    pub fn frame(&self) -> RendererFrame<'_> {
        RendererFrame {
            request: self.request,
            origin: self.origin,
            surfaces: &self.surfaces,
            lights: &self.lights,
            structures: &self.structures,
            supplies: &self.supplies,
            startup: self.startup.as_ref(),
            failure: self.failure.as_deref(),
        }
    }
    pub fn prepared_surface(&self) -> Option<(WorldRequest, &RendererSurface)> {
        self.prepared
            .as_ref()
            .map(|(request, surface)| (*request, surface))
    }
    pub fn pending_structures(&self) -> &[Placement] {
        &self.pending
    }
    pub fn structure_required(&self, id: &GeneratedId) -> bool {
        self.required_structures.contains(id)
    }
    pub fn structure_replay_required(&self, id: &GeneratedId) -> bool {
        self.replay.contains(id)
    }
    /// True while the stream still wants the structure.
    pub fn structure_wanted(&self, request: WorldRequest, id: &GeneratedId) -> bool {
        request.session == self.request.session && self.pending.iter().any(|p| p.id == *id)
    }
    /// Session, published structure revision and pending structure revision.
    pub fn structure_membership(&self) -> (u64, u64, u64) {
        self.membership
    }
    pub fn required_lights(&self) -> &[GeneratedId] {
        &self.required_lights
    }
    pub fn required_ready(&self) -> bool {
        self.required_ready
    }
    pub fn road_prototype_required(&self, source: &SourceId) -> bool {
        self.lookups
            .reference
            .road
            .prototypes
            .iter()
            .zip(self.road_required)
            .any(|(prototype, required)| required && prototype == source)
    }
    pub fn holds_prototype(&self, source: &SourceId) -> bool {
        self.holds.contains(source)
    }
    /// True if the stream holds the prototype or has room for the hold.
    pub fn can_hold_prototype(&self, source: &SourceId) -> bool {
        self.holds_prototype(source)
            || self
                .prototype_cpu_bytes
                .checked_add(hold_cost(
                    &self.lookups.reference,
                    &self.road_loaded,
                    self.hold_insert_bytes,
                    source,
                ))
                .is_some_and(|v| v <= super::PROTOTYPE_CPU)
    }
    pub fn dependency_staging_configured(&self) -> bool {
        self.staging_configured
    }
    pub fn contents_initialized(&self, id: &GeneratedId) -> bool {
        self.pending
            .iter()
            .position(|p| p.id == *id)
            .is_none_or(|i| self.initialized[i])
    }
    pub fn appearance(&self, id: &GeneratedId) -> &[NodeAppearance] {
        if id.layer == WorldLayer::Lamp {
            return self
                .cables
                .iter()
                .find(|(index, _)| *index == id.unit[0])
                .map_or(&[], |(_, cable)| cable);
        }
        self.contents
            .get(id)
            .map_or(&[], |content| &content.appearance)
    }
    /// Source `materialChangeScript.Start` biome for renderer `node`, from the object position, not the surface.
    /// `None` when no active marked node holds the renderer.
    pub fn renderer_biome(
        &self,
        prototype: &crate::reference::WorldPrototypeReference,
        placement: &Placement,
        node: usize,
    ) -> Option<u8> {
        if !crate::world::biome_materials::under_mark(prototype, node)
            || !crate::world::spawn_appearance::node_active(
                prototype,
                node,
                self.appearance(&placement.id),
            )
        {
            return None;
        }
        self.placements.get(&placement.id).map(|p| p.biome)
    }
    /// Biome material for a first material. `None` keeps the material.
    pub fn biome_material(&self, material: &SourceId, biome: u8) -> Option<&SourceId> {
        self.lookups.biome_materials.variant(material, biome)
    }
    pub fn lamp_materials(
        &self,
        prototype: &SourceId,
        node: &SourceId,
    ) -> Result<Arc<[SourceId]>, ReferenceError> {
        let lookups = &*self.lookups;
        if let Some((_, _, output)) = lookups
            .lamp_materials
            .lock()
            .unwrap()
            .iter()
            .find(|(p, n, _)| p == prototype && n == node)
        {
            return Ok(output.clone());
        }
        let output: Arc<[SourceId]> =
            super::lamp_materials_of(&lookups.rules, &lookups.reference, prototype, node)?.into();
        lookups.lamp_materials.lock().unwrap().push((
            prototype.clone(),
            node.clone(),
            output.clone(),
        ));
        Ok(output)
    }
    pub fn lod_owner(&self, source: &SourceId) -> Option<&SourceId> {
        self.lookups
            .rules
            .full_record_owner(&self.lookups.reference, source)
    }
    pub fn moving_nodes(
        &self,
        reference: &ReferenceManifest,
        prototype: &crate::reference::WorldPrototypeReference,
    ) -> Result<Vec<bool>, ReferenceError> {
        super::moving_nodes::moving_nodes_of(&self.lookups.rules, reference, prototype)
    }
    pub fn prop_material(&self, placement: &Placement) -> Option<(&SourceId, &SourceId)> {
        self.placements
            .get(&placement.id)
            .and_then(|p| p.prop_material.as_ref())
            .map(|(renderer, material)| (renderer, material))
    }
    /// Writes the primary appearance assets of all contents to `assets`.
    pub fn primary_assets(&self, assets: &mut Vec<crate::reference::AssetId>) {
        assets.clear();
        for content in self.contents.values() {
            assets.extend(
                content
                    .appearance
                    .iter()
                    .filter_map(|a| a.primary.map(|p| p.0)),
            );
        }
    }
    /// Squared source distance from the nearer of the player and vehicle.
    pub fn focus_distance_squared(&self, position: [f64; 3]) -> f32 {
        super::focus_distance_squared(self.request, position)
    }
}

/// Stream bytes of a new prototype hold.
pub(super) fn hold_cost(
    reference: &WorldReference,
    road_loaded: &[bool; 3],
    insert_bytes: usize,
    source: &SourceId,
) -> usize {
    let road_bytes = reference
        .road
        .prototypes
        .iter()
        .position(|id| id == source)
        .filter(|&index| !road_loaded[index])
        .map_or(0, |_| {
            crate::world::road::road_prototype_bytes(
                reference
                    .prototypes
                    .iter()
                    .find(|p| &p.source == source)
                    .expect("validated road prototype"),
            )
        });
    (insert_bytes + source.file.capacity()).saturating_add(road_bytes)
}

impl WorldStream {
    /// Copies the renderer view of the stream into `world`. Allocates only for changed data.
    pub fn sync_renderer(&self, world: &mut RendererWorld) {
        let session_changed = world.request.session != self.request.session;
        world.request = self.request;
        world.origin = self.origin;
        if world.failure.as_deref() != self.failure.as_deref() {
            world.failure.clone_from(&self.failure);
        }
        if world.surfaces.len() != self.surfaces.len()
            || world
                .surfaces
                .iter()
                .zip(&self.surfaces)
                .any(|(a, b)| !a.same(b))
        {
            world.surfaces.clear();
            world
                .surfaces
                .extend(self.surfaces.iter().map(RendererSurface::of));
        }
        match self.prepared_surface() {
            None => world.prepared = None,
            Some((request, published)) => {
                if world
                    .prepared
                    .as_ref()
                    .is_none_or(|(r, s)| *r != request || !s.same(published))
                {
                    world.prepared = Some((request, RendererSurface::of(published)));
                }
            }
        }
        if world.lights.len() != self.lights.len()
            || world
                .lights
                .iter()
                .zip(&self.lights)
                .any(|(a, b)| !same_light(a, b))
        {
            world.lights.clone_from(&self.lights);
        }
        let membership = self.structure_membership();
        let structures_changed =
            (membership.0, membership.1) != (world.membership.0, world.membership.1);
        let pending_changed =
            (membership.0, membership.2) != (world.membership.0, world.membership.2);
        world.membership = membership;
        if structures_changed {
            world.structures.clear();
            world
                .structures
                .extend(self.structures.iter().map(|s| RendererStructure {
                    fixed_decoration: s.fixed_decoration,
                    cpu_bytes: s.cpu_bytes,
                    placement: s.placement.clone(),
                }));
        }
        if pending_changed {
            world.pending.clone_from(&self.wanted_structures);
        }
        if structures_changed || pending_changed {
            world.mark += 1;
            let mark = world.mark;
            for placement in world
                .pending
                .iter()
                .chain(world.structures.iter().map(|s| &s.placement))
            {
                if let Some(values) = world.placements.get_mut(&placement.id) {
                    values.mark = mark;
                    continue;
                }
                let p = placement.pose.position;
                let color = self
                    .evaluator
                    .color([p[0] as f32, -p[2] as f32], p[1] as f32);
                world.placements.insert(
                    placement.id.clone(),
                    PlacementValues {
                        biome: crate::world::biome(color, self.mode.biome_override()),
                        prop_material: self
                            .props
                            .material(placement)
                            .map(|(r, m)| (r.clone(), m.clone())),
                        mark,
                    },
                );
            }
            world.placements.retain(|_, values| values.mark == mark);
        }
        world.initialized.resize(world.pending.len(), false);
        for (initialized, placement) in world.initialized.iter_mut().zip(&world.pending) {
            *initialized = self.contents.initialized(&placement.id);
        }
        self.sync_renderer_contents(world);
        let mut count = 0;
        for road in &self.selected_roads {
            let index = i64::from(road.segment.index);
            match world.cables.get_mut(count) {
                Some(entry) if entry.0 == index && entry.1 == road.cable => {}
                Some(entry) => {
                    entry.0 = index;
                    entry.1.clone_from(&road.cable);
                }
                None => world.cables.push((index, road.cable.clone())),
            }
            count += 1;
        }
        world.cables.truncate(count);
        if session_changed || world.startup.is_some() != self.startup.is_some() {
            world.startup = self.startup.clone();
        }
        if world.supplies.len() != self.supplies.len()
            || world
                .supplies
                .iter()
                .zip(&self.supplies)
                .any(|(a, b)| a.key != b.key || a.items != b.items)
        {
            world.supplies.clear();
            world
                .supplies
                .extend(self.supplies.iter().map(|s| RendererSupply {
                    key: s.key.clone(),
                    point: s.point,
                    definition: s.definition.clone(),
                    items: s.items,
                }));
        }
        crate::world::required::select(
            &mut world.required_structures,
            self.required.structures.iter(),
        );
        crate::world::required::select(&mut world.required_lights, self.required.lights.iter());
        crate::world::required::select(
            &mut world.replay,
            self.replay_roots.iter().map(|root| &root.id).chain(
                self.replay_indices
                    .iter()
                    .filter_map(|&index| self.changes.get_indexed(index).1.instance.as_ref()),
            ),
        );
        world.required_ready = self.required_ready();
        world.road_required = [false; 3];
        for road in &self.selected_roads {
            if world.required_ready || self.required.roads.contains(&road.segment.index) {
                world.road_required[road.segment.biome as usize] = true;
            }
        }
        world.road_loaded = self.road_prototypes.each_ref().map(Option::is_some);
        if world.holds.len() != self.prototype_holds.len()
            || self
                .prototype_holds
                .keys()
                .any(|k| !world.holds.contains(k))
        {
            world.holds.clear();
            world.holds.extend(self.prototype_holds.keys().cloned());
        }
        world.hold_insert_bytes = crate::world::metrics::map_insert_bytes(&self.prototype_holds);
        world.prototype_cpu_bytes = self.metrics().prototype_cpu_bytes;
        world.staging_configured = self.dependency_staging_configured();
    }
    /// Content appearance by placement. Active contents come before cursor roots.
    fn sync_renderer_contents(&self, world: &mut RendererWorld) {
        world.mark += 1;
        let mark = world.mark;
        let mut count = 0;
        for (id, content) in self
            .contents
            .active
            .iter()
            .map(|c| (&c.content.placement.id, &c.content))
            .chain(self.contents.cursor_roots())
        {
            match world.contents.get_mut(id) {
                Some(values) if values.mark == mark => continue,
                Some(values) => {
                    values.mark = mark;
                    if values.appearance != content.appearance {
                        values.appearance.clone_from(&content.appearance);
                    }
                }
                None => {
                    world.contents.insert(
                        id.clone(),
                        ContentValues {
                            appearance: content.appearance.clone(),
                            mark,
                        },
                    );
                }
            }
            count += 1;
        }
        if count != world.contents.len() {
            world.contents.retain(|_, values| values.mark == mark);
        }
    }
}

/// Lights compare by their changing values. The light record is fixed by its id.
fn same_light(a: &WorldLight, b: &WorldLight) -> bool {
    a.id == b.id
        && a.pole == b.pole
        && a.pose == b.pose
        && a.biome == b.biome
        && a.prototype == b.prototype
        && a.scale == b.scale
        && a.range == b.range
        && a.enabled == b.enabled
        && a.intensity == b.intensity
        && a.node == b.node
}
