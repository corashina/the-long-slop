//! Hidden save renderer ownership. Resource exchange is confined to exclusive calls.
use super::world::{
    SourceWorldVisuals,
    grass_jobs::GrassJobs,
    instances::SourceInstances,
    pipelines::WorldPipelines,
    prototypes::SourcePrototypes,
    runtime::{SourcePresentationAllocation, WorldPresentation},
};
use super::{lighting, materials, reference::*};
use crate::{
    reference::{AssetUsage, ReferenceManifest},
    save::SaveError,
};
use bevy::camera::CameraProjection;
use bevy::{ecs as bevy_ecs, ecs::system::SystemState, prelude::*};
use materials::grass::writers::GrassWriters;
use simulation::boundary::{PoseKey, PoseRow, SimFrame, StateBlock};

#[derive(Component)]
pub struct HiddenWorld;
#[derive(Resource, Default)]
pub struct CandidateImportedPoses(pub std::collections::HashMap<Entity, Mat4>);

pub struct PreparedPresentation {
    pub(crate) surfaces: SourceWorldVisuals,
    pub(crate) prototypes: SourcePrototypes,
    pub(crate) pipelines: WorldPipelines,
    pub(crate) instances: SourceInstances,
    pub(crate) grass: GrassJobs,
    pub(crate) writers: GrassWriters,
    pub(crate) materials: materials::SourceMaterials,
    pub(crate) lighting: lighting::SourceLighting,
    allocation: SourcePresentationAllocation,
    system: SystemState<WorldPresentation<'static, 'static>>,
    updates: Vec<(Entity, Transform, Visibility)>,
    conditions: Option<super::conditions::PreparedConditions>,
    roots: Vec<Entity>,
    poses: CandidateImportedPoses,
    screen_height: f32,
    aspect_ratio: f32,
    required_views: Option<Box<crate::world::RequiredViews>>,
    /// The candidate frame. The candidate driver fills it before each `advance`.
    pub(crate) frame: super::sim_frame::SimFrameResource,
    /// Renderer input of the next candidate step.
    command: simulation::boundary::CandidateCommand,
}
impl PreparedPresentation {
    pub fn can_start(world: &World) -> bool {
        world
            .resource::<super::fixed_assets::SourceFixedAssets>()
            .payloads_ready()
            && world.resource::<SourcePrototypes>().settled()
            && world.resource::<materials::SourceMaterials>().settled()
            && world
                .resource::<WorldPipelines>()
                .settled()
                .unwrap_or(false)
            && world.resource::<GrassJobs>().settled()
            && world.resource::<GrassWriters>().ready().unwrap_or(false)
            && world
                .resource::<super::sim_frame::SimFrameResource>()
                .frame
                .state
                .flags
                & StateBlock::WORLD_IDLE
                != 0
    }
    /// The first candidate command carries the renderer facts and the budget report of the copy.
    pub fn new(
        reference: &ReferenceManifest,
        stream: &crate::world::WorldStream,
        world: &mut World,
    ) -> Result<Self, SaveError> {
        validate_assets(reference, world)?;
        world.resource_scope(|world, mut pipelines: Mut<WorldPipelines>| {
            pipelines
                .reserve_cache(&mut world.resource_mut::<SourcePresentationAllocation>().ledger)
        })?;
        // The candidate renderer spends its own credit from the candidate stream.
        let mut ledger = crate::ownership::PresentationLedger::default();
        let mut facts = crate::world::RendererFacts::default();
        let mut renderer_world = Box::new(
            world
                .resource::<SourcePresentationAllocation>()
                .world
                .as_ref()
                .map_or_else(|| crate::world::RendererWorld::new(stream), |w| w.fork()),
        );
        stream.sync_renderer(&mut renderer_world);
        super::world::preflight::configure_staging(
            &renderer_world,
            &mut ledger,
            &mut facts,
            &reference.file_catalog,
        )?;
        let mut fixed_snapshot = crate::ownership::fixed::FixedAllocation::default();
        world
            .resource::<super::fixed_assets::SourceFixedAssets>()
            .account(&mut ledger, &mut fixed_snapshot)?;
        let mesh_count = world.query::<&Mesh3d>().iter(world).count();
        let mut admitted = ledger.allocation();
        admitted.active_cpu += world
            .resource::<super::conditions::ConditionVisuals>()
            .candidate_bytes()
            + super::candidate_upload::UPLOAD_CONTROL_BYTES
            + std::mem::size_of::<Self>()
            + std::mem::size_of::<crate::world::RequiredViews>()
            + reference.nodes.len() * std::mem::size_of::<(Entity, Transform, Visibility)>()
            + 8192 * std::mem::size_of::<Entity>()
            + mesh_count.next_power_of_two() * 2 * (std::mem::size_of::<(Entity, Mat4)>() + 1)
            + world.resource::<lighting::SourceLighting>().clone_bytes();
        ledger.set_allocation(admitted)?;
        let materials = world
            .resource::<materials::SourceMaterials>()
            .fork(&mut ledger)?;
        let prototypes =
            world
                .resource::<SourcePrototypes>()
                .fork(&renderer_world, &mut ledger, &mut facts)?;
        // The first candidate step charges the copy. A budget failure stops the save there.
        let report = ledger.report();
        let lighting = world.resource::<lighting::SourceLighting>().prepare_copy();
        let screen_height = world
            .query::<&Window>()
            .iter(world)
            .next()
            .map_or(1., |w| w.physical_height() as f32);
        let aspect_ratio = world
            .query::<&Window>()
            .iter(world)
            .next()
            .map_or(1., |w| w.width() / w.height().max(1.));
        let shared = shared_bytes(&prototypes, &materials);
        let fixed = shared_fixed(world);
        Ok(Self {
            required_views: None,
            aspect_ratio,
            screen_height,
            poses: CandidateImportedPoses(std::collections::HashMap::with_capacity(mesh_count)),
            surfaces: default(),
            prototypes,
            pipelines: default(),
            instances: default(),
            grass: default(),
            writers: default(),
            materials,
            lighting,
            allocation: {
                let mut allocation =
                    SourcePresentationAllocation::for_session(stream.frame().request.session);
                allocation.fixed = fixed_snapshot;
                allocation.ledger = ledger;
                allocation.lighting_cpu =
                    world.resource::<lighting::SourceLighting>().clone_bytes();
                allocation.world = Some(renderer_world);
                allocation
            },
            system: SystemState::new(world),
            conditions: None,
            updates: Vec::with_capacity(reference.nodes.len()),
            roots: Vec::with_capacity(8192),
            frame: default(),
            command: simulation::boundary::CandidateCommand {
                world: simulation::boundary::WorldCommand {
                    facts,
                    report: Some(report),
                    ..default()
                },
                shared,
                fixed,
            },
        })
    }
    fn renderer_world(&self) -> &crate::world::RendererWorld {
        self.allocation
            .world
            .as_deref()
            .expect("candidate renderer world")
    }
    fn exchange(&mut self, world: &mut World) {
        macro_rules! exchange {
            ($field:ident,$ty:ty) => {
                let mut owner = world
                    .remove_resource::<$ty>()
                    .expect("source renderer owner");
                std::mem::swap(&mut owner, &mut self.$field);
                world.insert_resource(owner);
            };
        }
        exchange!(surfaces, SourceWorldVisuals);
        exchange!(prototypes, SourcePrototypes);
        exchange!(pipelines, WorldPipelines);
        exchange!(instances, SourceInstances);
        exchange!(grass, GrassJobs);
        exchange!(writers, GrassWriters);
        exchange!(materials, materials::SourceMaterials);
        exchange!(lighting, lighting::SourceLighting);
        exchange!(allocation, SourcePresentationAllocation);
        exchange!(poses, CandidateImportedPoses);
    }
    /// Renderer input of the next candidate step.
    pub fn take_command(&mut self) -> simulation::boundary::CandidateCommand {
        std::mem::take(&mut self.command)
    }
    /// One renderer step on the candidate frame. `stream` is the candidate world.
    pub fn advance(
        &mut self,
        reference: &ReferenceManifest,
        stream: &crate::world::WorldStream,
        world: &mut World,
    ) -> Result<bool, SaveError> {
        if let Some(window) = world.query::<&Window>().iter(world).next() {
            self.screen_height = window.physical_height() as f32;
            self.aspect_ratio = window.width() / window.height().max(1.);
        }
        if let Some(failure) = self.frame.failure.take() {
            return Err(crate::reference::ReferenceError(failure).into());
        }
        // Only the active renderer reads these candidate outputs.
        self.frame.screens.clear();
        self.frame.screen_set = None;
        self.frame.feedback = None;
        self.frame.home_cable = None;
        self.frame.measurements = None;
        self.frame.changed_conditions.clear();
        let state = self.frame.frame.state;
        let camera = crate::app::frame_camera(&state, state.look.pose(1., [0.; 2]));
        let local = camera.translation;
        let position = Vec3::from_array(std::array::from_fn(|i| {
            (state.stream_origin[i] + f64::from(local[i])) as f32
        }));
        let projection = super::camera_projection::player_perspective(
            state.fov_degrees,
            self.aspect_ratio,
            state.far_plane,
        );
        let clip = projection.get_clip_from_view()
            * Mat4::from_rotation_translation(camera.rotation, position).inverse();
        let rows = (clip * Mat4::from_scale(Vec3::new(1., 1., -1.))).transpose();
        let view = grass_mesh::GrassView {
            position: position * Vec3::new(1., 1., -1.),
            screen_height: self.screen_height,
            clip_planes: [
                rows.w_axis + rows.x_axis,
                rows.w_axis - rows.x_axis,
                rows.w_axis + rows.y_axis,
                rows.w_axis - rows.y_axis,
            ]
            .map(|p| p / p.truncate().length()),
        };
        let lod = super::world::lod::LodView {
            position: local,
            fov_degrees: state.fov_degrees,
            ortho_size: None,
        };
        imported_updates(&self.frame, world, &mut self.updates)?;
        candidate_required_views(
            reference,
            &self.frame.frame,
            camera.to_matrix(),
            self.aspect_ratio,
            self.screen_height,
            &mut self.required_views,
        )?;
        self.poses.0.clear();
        let mut query = world.query::<(Entity, &ChildOf, &Mesh3d)>();
        for (entity, parent, _) in query.iter(world) {
            if !world
                .resource::<ReferenceVisuals>()
                .nodes
                .values()
                .any(|e| *e == parent.parent())
            {
                continue;
            }
            let mut current = Some(entity);
            let mut matrix = Mat4::IDENTITY;
            let mut depth = 0;
            while let Some(node) = current {
                let pose = self
                    .updates
                    .iter()
                    .find(|(id, _, _)| *id == node)
                    .map(|(_, pose, _)| pose)
                    .or_else(|| world.get::<Transform>(node));
                if let Some(pose) = pose {
                    matrix = pose.to_matrix() * matrix;
                }
                current = world.get::<ChildOf>(node).map(ChildOf::parent);
                depth += 1;
                if depth > reference.nodes.len() + 4 {
                    return Err(SaveError::InvalidLinks(
                        "Cyclic imported renderer parent".into(),
                    ));
                }
            }
            self.poses.0.insert(entity, matrix);
        }
        self.exchange(world);
        let command = &mut self.command.world;
        command.views.clear();
        if let Some(views) = &self.required_views {
            command.views.extend_from_slice(views.as_slice());
        }
        let grant = self.frame.grant.take();
        let frame = &self.frame.frame;
        let result = match self.system.get_mut(world) {
            Ok(mut renderer) => {
                renderer.sync_mirror(stream);
                renderer.prepare_frame(
                    reference,
                    grant,
                    view,
                    frame.lighting.cycle.shader_time_seconds as f32,
                    lod,
                    false,
                    &frame.lights,
                    command,
                )
            }
            Err(e) => Err(crate::reference::ReferenceError(e.to_string())),
        }
        .and_then(|()| {
            // The hidden instances take the structure views of the candidate.
            self.system
                .get_mut(world)
                .map_err(|e| crate::reference::ReferenceError(e.to_string()))?
                .sync_instances(reference, &mut self.frame)
        });
        self.system.apply(world);
        self.exchange(world);
        {
            let mut hidden = world.resource_mut::<super::world::pipelines::CandidatePipelines>();
            hidden.0 = Some(self.pipelines.clone());
            hidden.1 = Some(self.materials.preparation.clone());
        }
        // Restore active owners before returning any preparation error.
        self.roots.clear();
        self.roots.extend(self.instances.roots());
        self.roots.extend(self.surfaces.roots());
        self.roots.extend(self.lighting.road_lamps.roots());
        let mut index = 0;
        while index < self.roots.len() {
            let root = self.roots[index];
            index += 1;
            if let Some(children) = world.get::<Children>(root) {
                self.roots.extend(children.iter());
            }
        }
        for &root in &self.roots {
            if let Ok(mut entity) = world.get_entity_mut(root) {
                entity.insert(HiddenWorld);
            }
        }
        for root in self
            .instances
            .roots()
            .chain(self.surfaces.roots())
            .chain(self.lighting.road_lamps.roots())
        {
            if let Ok(mut entity) = world.get_entity_mut(root) {
                entity.insert(Visibility::Hidden);
            }
        }
        self.command.shared = shared_bytes(&self.prototypes, &self.materials);
        self.command.fixed = shared_fixed(world);
        result?;
        let gpu_ready = self.materials.preparation.ready()?;
        let ready = gpu_ready
            && state.flags & StateBlock::RESTORED != 0
            && state.flags & StateBlock::WORLD_READY != 0
            && self.surfaces.ready(self.renderer_world())
            && self.instances.ready()
            && self.grass.settled()
            && self.writers.ready()?
            && self.pipelines.settled()?
            && self.lighting.road_lamps.ready();
        if ready && self.conditions.is_none() {
            let frame = &self.frame;
            self.conditions = Some(world.resource_scope(
                |world, visuals: Mut<super::conditions::ConditionVisuals>| {
                    visuals.prepare_candidate(frame, world)
                },
            )?);
        }
        if ready {
            validate_assets(reference, world)?;
        }
        Ok(ready)
    }
    pub fn validate_commit(
        &self,
        reference: &ReferenceManifest,
        world: &World,
    ) -> Result<(), SaveError> {
        validate_assets(reference, world)?;
        if world
            .iter_entities()
            .filter_map(|e| e.get::<Window>())
            .any(|w| {
                w.physical_height() as f32 != self.screen_height
                    || w.width() / w.height().max(1.) != self.aspect_ratio
            })
        {
            return Err(SaveError::MissingAssets("Prepared viewport changed".into()));
        }
        if self.conditions.is_none()
            || !self.materials.preparation.ready()?
            || self.frame.frame.state.flags & StateBlock::RESTORED == 0
            || self.frame.frame.state.flags & StateBlock::WORLD_READY == 0
            || !self.pipelines.settled()?
            || !self.writers.ready()?
            || !self.grass.settled()
            || !self.instances.ready()
            || !self.surfaces.ready(self.renderer_world())
            || !self.lighting.road_lamps.ready()
        {
            return Err(SaveError::MissingAssets(
                "Prepared source scene is no longer ready".into(),
            ));
        }
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.grass.cancel();
        self.materials.preparation.cancel();
        self.materials.world_textures.cancel();
        self.prototypes.cancel();
    }
    pub fn drained(&mut self, world: &mut World) -> bool {
        self.materials.preparation.drained()
            && self.pipelines.drained()
            && self.surfaces.drained()
            && self.instances.drained()
            && self.grass.drained()
            && self.writers.drained()
            && self.prototypes.drain_ready(
                world.resource::<AssetServer>(),
                world.resource::<Assets<super::world::prototype_loader::SourceGltf>>(),
                world.resource::<Assets<super::world::preflight::SourcePlan>>(),
            )
            && world.resource_scope(
                |world, mut batches: Mut<Assets<super::texture_batches::SourceTextureBatch>>| {
                    self.materials.drain_ready(
                        world.resource::<AssetServer>(),
                        world.resource::<Assets<super::primary_image::SourcePrimaryImage>>(),
                        &mut batches,
                    )
                },
            )
    }
    pub fn release(mut self, world: &mut World) {
        *world.resource_mut::<super::world::pipelines::CandidatePipelines>() = default();

        self.exchange(world);
        {
            let mut r = self.system.get_mut(world).expect("source owner resources");
            r.instances.release_all(&mut r.commands, &mut r.materials);
            r.grass.release_all(&mut r.meshes, &mut r.materials.grass);
            r.writers.release_all();
            r.surfaces.retire_session(&mut r.commands, &mut r.meshes);
            r.surfaces.reset_bindings(&mut r.terrain);
            r.lights.road_lamps.reset_instances(&mut r.commands);
        }
        self.system.apply(world);
        self.exchange(world);
        world.resource_scope(|world,mut assets:Mut<Assets<super::world::prototype_loader::SourceGltf>>|world.resource_scope(|world,mut plans:Mut<Assets<super::world::preflight::SourcePlan>>|world.resource_scope(|world,mut meshes:Mut<Assets<Mesh>>|{self.prototypes.release_all(&mut assets,&mut plans,&mut meshes,&mut world.resource_mut::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>());})));
        world.resource_scope(|world, mut images: Mut<Assets<Image>>| {
            world.resource_scope(
                |world, mut materials: Mut<Assets<materials::SourceMaterial>>| {
                    world.resource_scope(|world, mut chains: Mut<Assets<super::primary_image::SourcePrimaryImage>>| self.materials
                        .release_all(world.resource::<AssetServer>(), &mut images, &mut chains, &mut materials)
                        .expect("drained source material ownership"));
                },
            )
        });
    }
    pub fn commit(mut self, world: &mut World) {
        self.materials.preparation.promote();
        self.lighting.road_lamps.end_overlap();
        for &root in &self.roots {
            if let Ok(mut e) = world.get_entity_mut(root) {
                e.remove::<HiddenWorld>();
            }
        }
        for root in self
            .instances
            .roots()
            .chain(self.surfaces.roots())
            .chain(self.lighting.road_lamps.roots())
        {
            if let Ok(mut e) = world.get_entity_mut(root) {
                e.insert(Visibility::Inherited);
            }
        }
        for (entity, pose, visibility) in self.updates.drain(..) {
            world.entity_mut(entity).insert((pose, visibility));
        }
        self.conditions
            .take()
            .expect("prepared conditions")
            .commit(world);
        self.exchange(world);
        self.release(world);
    }
}
fn imported_updates(
    frame: &super::sim_frame::SimFrameResource,
    world: &mut World,
    updates: &mut Vec<(Entity, Transform, Visibility)>,
) -> Result<(), SaveError> {
    updates.clear();
    let mut query = world.query::<(
        Entity,
        &Transform,
        &Visibility,
        Option<&BodyVisual>,
        Option<&ItemVisual>,
        Option<&CapVisual>,
        Option<&ValveVisibility>,
    )>();
    for (entity, transform, visible, body, item, cap, valve) in query.iter(world) {
        if !world
            .resource::<ReferenceVisuals>()
            .nodes
            .values()
            .any(|id| *id == entity)
        {
            continue;
        }
        if body.is_none() && item.is_none() && cap.is_none() && valve.is_none() {
            continue;
        }
        let mut pose = *transform;
        let mut visibility = *visible;
        let mut active = true;
        let key = body
            .map(|b| (PoseKey::Body(b.body), b.scale))
            .or(item.map(|i| (PoseKey::Item(i.item), i.scale)));
        if let Some((key, scale)) = key {
            let (_, row) = frame
                .pose(key)
                .ok_or_else(|| SaveError::InvalidLinks(format!("Missing render pose {key:?}")))?;
            active = row.flags & PoseRow::VISIBLE != 0;
            pose = super::transform(row.pose()).with_scale(scale);
        }
        if body.is_some() || item.is_some() || valve.is_some() {
            visibility =
                if active && valve.is_none_or(|v| (frame.frame.valve(v.opening) > 0.) == v.open) {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
        }
        if let Some(cap) = cap {
            let amount = frame.frame.valve(cap.opening);
            pose = cap.closed;
            if cap.euler {
                let a = cap.closed_euler.lerp(cap.open_euler, amount) * std::f32::consts::PI / 180.;
                pose.rotation = Quat::from_rotation_y(a.y)
                    * Quat::from_rotation_x(a.x)
                    * Quat::from_rotation_z(a.z);
            } else {
                pose.translation = cap.closed.translation.lerp(cap.opened.translation, amount);
                pose.rotation = cap
                    .closed
                    .rotation
                    .lerp(cap.opened.rotation, amount)
                    .normalize();
            }
        }
        updates.push((entity, pose, visibility));
    }
    Ok(())
}
/// Prototype bytes that the candidate shares with the active renderer: CPU, GPU.
fn shared_bytes(
    prototypes: &SourcePrototypes,
    materials: &materials::SourceMaterials,
) -> [usize; 2] {
    let (prototype, material) = (prototypes.shared_bytes(), materials.shared_bytes());
    [prototype.0 + material.0, prototype.1 + material.1]
}
fn shared_fixed(world: &World) -> [usize; 2] {
    let (cpu, gpu) = world
        .resource::<super::fixed_assets::SourceFixedAssets>()
        .shared_bytes();
    [cpu, gpu]
}
fn validate_assets(reference: &ReferenceManifest, world: &World) -> Result<(), SaveError> {
    world
        .resource::<super::conditions::ConditionVisuals>()
        .validate_assets(world)?;
    let fail = |path: String| SaveError::MissingAssets(path);
    let server = world.resource::<AssetServer>();
    let gltfs = world.resource::<Assets<bevy::gltf::Gltf>>();
    let visuals = world.resource::<ReferenceVisuals>();
    let source = world.resource::<lighting::SourceLighting>();
    let materials = world.resource::<materials::SourceMaterials>();
    if visuals.failed || !visuals.bound {
        return Err(fail("Reference bindings are not ready".into()));
    }
    for asset in reference.assets.iter().filter(|a| {
        matches!(
            a.usage,
            AssetUsage::UniqueScene
                | AssetUsage::RoadLampPrototype
                | AssetUsage::SkyPrototype
                | AssetUsage::ItemPrototype
                | AssetUsage::PlayerPrototype
                | AssetUsage::RagdollPrototype
                | AssetUsage::SkyTraffic
        )
    }) {
        let (_, handle) = visuals
            .handles
            .iter()
            .find(|(path, _)| *path == asset.path)
            .ok_or_else(|| fail(asset.path.clone()))?;
        if !server.is_loaded_with_dependencies(handle.id())
            || gltfs.get(handle).is_none_or(|gltf| gltf.scenes.is_empty())
        {
            return Err(fail(asset.path.clone()));
        }
        if asset.usage == AssetUsage::UniqueScene {
            for node in reference.nodes.iter().filter(|node| node.asset == asset.id) {
                let entity = visuals
                    .nodes
                    .get(&node.id)
                    .ok_or_else(|| fail(format!("{}: {}", asset.path, node.hierarchy_path)))?;
                if world.get::<Transform>(*entity).is_none()
                    || world.get::<bevy::gltf::GltfExtras>(*entity).is_none()
                {
                    return Err(fail(format!("{}: {}", asset.path, node.hierarchy_path)));
                }
            }
        }
    }
    if !world
        .resource::<super::fixed_assets::SourceFixedAssets>()
        .payloads_ready()
        || source.state != lighting::RenderingState::Ready
        || !source.materials_ready
        || source.warming
        || source.road_lamps.recipe.is_none()
    {
        return Err(fail("Lighting assets are not ready".into()));
    }
    if materials.state != materials::RenderingState::Ready {
        return Err(fail("Material assets are not ready".into()));
    }
    for light in &source.lights {
        if world.get::<lighting::SourceLight>(light.entity).is_none()
            || light
                .node
                .is_some_and(|node| world.get::<Transform>(node).is_none())
        {
            return Err(fail(format!("Light binding {}", light.id.0)));
        }
    }
    for (_, path, handle) in &source.textures {
        if !world.resource::<Assets<Image>>().contains(handle.id()) {
            return Err(fail(path.clone()));
        }
    }
    for handle in materials.handles.values() {
        if !world
            .resource::<Assets<materials::SourceMaterial>>()
            .contains(handle.id())
        {
            return Err(fail("Source material handle".into()));
        }
    }
    let meshes = world.resource::<Assets<Mesh>>();
    let standard = world.resource::<Assets<StandardMaterial>>();
    let authored = world.resource::<Assets<materials::SourceMaterial>>();
    for lamp in &source.lamp_materials {
        for choice in &lamp.choices {
            choice.validate_assets(authored)?;
        }
    }
    for entity in world.iter_entities() {
        // Generated GPU-only surfaces are validated by their pipeline tickets.
        if entity
            .get::<ChildOf>()
            .is_some_and(|parent| visuals.nodes.values().any(|node| *node == parent.parent()))
            && entity
                .get::<Mesh3d>()
                .is_some_and(|mesh| !meshes.contains(mesh.0.id()))
        {
            return Err(fail("Imported renderer mesh is missing".into()));
        }
        let receiver = entity.get::<MeshMaterial3d<materials::SourceMaterial>>();
        let sky = entity.get::<MeshMaterial3d<super::sky::SkyMaterial>>();
        if let Some(material) = entity.get::<MeshMaterial3d<StandardMaterial>>() {
            if !standard.contains(material.0.id()) {
                return Err(fail(format!(
                    "rendered standard material on {:?}",
                    entity.id()
                )));
            }
        }
        if receiver.is_some_and(|material| !authored.contains(material.0.id())) {
            return Err(fail(format!(
                "rendered source material on {:?}",
                entity.id()
            )));
        }
        if sky.is_some_and(|material| {
            !world
                .resource::<Assets<super::sky::SkyMaterial>>()
                .contains(material.0.id())
        }) {
            return Err(fail("sky material".into()));
        }
    }
    if !world
        .iter_entities()
        .any(|entity| entity.contains::<super::camera::DriveCamera>())
    {
        return Err(fail("Camera binding".into()));
    }
    Ok(())
}

fn candidate_required_views(
    reference: &ReferenceManifest,
    frame: &SimFrame,
    camera: Mat4,
    aspect: f32,
    height: f32,
    views: &mut Option<Box<crate::world::RequiredViews>>,
) -> Result<(), SaveError> {
    use super::view_frame::{camera_snapshot, required_view};
    let lighting = &reference.render_lighting;
    let extent = UVec2::new((height * aspect).max(1.) as u32, height.max(1.) as u32);
    let forward = crate::app::light_forward(&frame.lighting.cycle);
    let origin = frame.state.stream_origin;
    let first =
        super::camera::world_camera_settings(frame.state.fov_degrees, frame.state.far_plane);
    let first = required_view(
        &camera_snapshot(
            extent,
            camera.inverse(),
            first.near,
            first.far,
            first.fov_degrees,
            first.mask,
            lighting.shadows.distance,
        ),
        origin,
        forward,
        lighting.lights[0].mask,
    )?;
    let views = views.get_or_insert_with(|| Box::new(crate::world::RequiredViews::new(first)));
    views.reset(first);
    // The car probe binds with the car copy after load. It has no required view yet.
    let positions = [Vec3::ZERO; super::lighting_scene::PROBE_COUNT];
    let enabled = [false; super::lighting_scene::PROBE_COUNT];
    super::view_frame::append_probe_required_views(
        views,
        &lighting.probes,
        &positions,
        &enabled,
        origin,
        forward,
        lighting.lights[0].mask,
    )?;
    Ok(())
}
