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
    starting_area::GarageRuntime,
};
use bevy::camera::CameraProjection;
use bevy::{ecs as bevy_ecs, ecs::system::SystemState, prelude::*};
use materials::grass::writers::GrassWriters;

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
    roots: Vec<Entity>,
    poses: CandidateImportedPoses,
    screen_height: f32,
    aspect_ratio: f32,
}
impl PreparedPresentation {
    pub fn can_start(world: &World) -> bool {
        world.resource::<SourcePrototypes>().settled()
            && world.resource::<materials::SourceMaterials>().settled()
            && world
                .resource::<WorldPipelines>()
                .settled()
                .unwrap_or(false)
            && world.resource::<GrassJobs>().settled()
            && world.resource::<GrassWriters>().ready().unwrap_or(false)
            && world
                .resource::<crate::app::Drive>()
                .runtime
                .stream
                .metrics()
                .generation_jobs
                == 0
            && world
                .resource::<crate::app::Drive>()
                .runtime
                .stream
                .metrics()
                .pending_work
                == 0
    }
    pub fn new(
        reference: &ReferenceManifest,
        candidate: &mut GarageRuntime,
        world: &mut World,
    ) -> Result<Self, SaveError> {
        validate_assets(reference, world)?;
        world.resource_scope(|world, mut pipelines: Mut<WorldPipelines>| {
            pipelines.reserve_cache(&mut world.resource_mut::<crate::app::Drive>().runtime.stream)
        })?;
        super::world::preflight::configure_staging(&mut candidate.stream)?;
        let mut overlap = world
            .resource::<crate::app::Drive>()
            .runtime
            .stream
            .metrics();
        let prototype = world.resource::<SourcePrototypes>().fork_shared_bytes();
        let material = world
            .resource::<materials::SourceMaterials>()
            .fork_shared_bytes();
        overlap.prototype_cpu_bytes = overlap
            .prototype_cpu_bytes
            .saturating_sub(overlap.shared_input_cpu_bytes + prototype.0 + material.0);
        overlap.prototype_gpu_bytes = overlap
            .prototype_gpu_bytes
            .saturating_sub(prototype.1 + material.1);
        candidate.stream.set_overlap(overlap)?;
        let mesh_count = world.query::<&Mesh3d>().iter(world).count();
        let mut admitted = candidate.stream.presentation_allocation();
        admitted.active_cpu += super::candidate_upload::UPLOAD_CONTROL_BYTES
            + std::mem::size_of::<Self>()
            + reference.nodes.len() * std::mem::size_of::<(Entity, Transform, Visibility)>()
            + 8192 * std::mem::size_of::<Entity>()
            + mesh_count.next_power_of_two() * 2 * (std::mem::size_of::<(Entity, Mat4)>() + 1)
            + world.resource::<lighting::SourceLighting>().clone_bytes();
        candidate.stream.set_presentation_allocation(admitted)?;
        let materials = world
            .resource::<materials::SourceMaterials>()
            .fork(&mut candidate.stream)?;
        let prototypes = world
            .resource::<SourcePrototypes>()
            .fork(&mut candidate.stream)?;
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
        Ok(Self {
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
            allocation: SourcePresentationAllocation::for_session(
                candidate.stream.frame().request.session,
            ),
            system: SystemState::new(world),
            updates: Vec::with_capacity(reference.nodes.len()),
            roots: Vec::with_capacity(8192),
        })
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
    fn overlap(&self, world: &World) -> crate::world::WorldMetrics {
        let mut overlap = world
            .resource::<crate::app::Drive>()
            .runtime
            .stream
            .metrics();
        let prototype = self.prototypes.shared_bytes();
        let material = self.materials.shared_bytes();
        overlap.prototype_cpu_bytes = overlap
            .prototype_cpu_bytes
            .saturating_sub(overlap.shared_input_cpu_bytes + prototype.0 + material.0);
        overlap.prototype_gpu_bytes = overlap
            .prototype_gpu_bytes
            .saturating_sub(prototype.1 + material.1);
        overlap
    }
    pub fn advance(
        &mut self,
        reference: &ReferenceManifest,
        candidate: &mut GarageRuntime,
        mode: super::camera::CameraMode,
        world: &mut World,
    ) -> Result<bool, SaveError> {
        if let Some(window) = world.query::<&Window>().iter(world).next() {
            self.screen_height = window.physical_height() as f32;
            self.aspect_ratio = window.width() / window.height().max(1.);
        }
        candidate.stream.set_overlap(self.overlap(world))?;
        candidate.initialize_storage_frame(1. / 60.)?;
        let pose = candidate.area.player.view_pose(&candidate.physics);
        let camera = if let Some(sleep) = candidate.sleep_camera() {
            super::camera::sleep_pose(sleep, &candidate.stream.origin)
        } else if mode == super::camera::CameraMode::Chase && candidate.area.player.seat().is_some()
        {
            super::camera::pose(mode, candidate.physics.snapshot(candidate.vehicle).pose)
        } else {
            super::transform(pose)
        };
        let local = camera.translation;
        let position = Vec3::from_array(
            candidate
                .stream
                .origin
                .absolute(local.to_array())
                .map(|v| v as f32),
        );
        let projection = PerspectiveProjection {
            fov: reference.player.field_of_view_degrees.to_radians(),
            aspect_ratio: self.aspect_ratio,
            ..default()
        };
        let clip = projection.get_clip_from_view()
            * Mat4::from_rotation_translation(camera.rotation, position).inverse();
        let rows = (clip * Mat4::from_scale(Vec3::new(1., 1., -1.))).transpose();
        let view = materials::grass::GrassView {
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
            fov_degrees: reference.player.field_of_view_degrees,
            ortho_size: None,
        };
        let inverse = clip.inverse();
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for x in [-1., 1.] {
            for y in [-1., 1.] {
                for z in [0.00001, 1.] {
                    let p = inverse.project_point3(Vec3::new(x, y, z));
                    min = min.min(p);
                    max = max.max(p);
                }
            }
        }
        let receivers = [crate::world::WorldBounds {
            min: min.to_array().map(|v| v as f64),
            max: max.to_array().map(|v| v as f64),
        }];
        imported_updates(candidate, world, &mut self.updates)?;
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
        let result = match self.system.get_mut(world) {
            Ok(mut renderer) => renderer.prepare(
                &mut candidate.stream,
                reference,
                &mut candidate.physics,
                &mut candidate.area,
                &mut candidate.wheels,
                view,
                candidate.lighting.cycle.output().shader_time_seconds as f32,
                lod,
                &receivers,
                false,
            ),
            Err(e) => Err(crate::reference::ReferenceError(e.to_string())),
        };
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
        result?;
        candidate.align_startup(reference)?;
        let restored = candidate.advance_saved_restore(reference)?;
        candidate.sync_world_activity(reference)?;
        let gpu_ready = self.materials.preparation.ready()?;
        let ready = gpu_ready
            && restored
            && !candidate.restore_pending()
            && candidate.stream.window_ready()
            && candidate.world_supported()
            && self.surfaces.ready(&candidate.stream)
            && self.instances.ready()
            && self.grass.settled()
            && self.writers.ready()?
            && self.pipelines.settled()?
            && self.lighting.road_lamps.ready();
        if ready {
            imported_updates(candidate, world, &mut self.updates)?;
            validate_assets(reference, world)?;
        }
        Ok(ready)
    }
    pub fn validate_commit(
        &self,
        reference: &ReferenceManifest,
        candidate: &GarageRuntime,
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
        if !self.materials.preparation.ready()?
            || candidate.restore_pending()
            || !candidate.world_supported()
            || !candidate.stream.window_ready()
            || !self.pipelines.settled()?
            || !self.writers.ready()?
            || !self.grass.settled()
            || !self.instances.ready()
            || !self.surfaces.ready(&candidate.stream)
            || !self.lighting.road_lamps.ready()
        {
            return Err(SaveError::MissingAssets(
                "Prepared source scene is no longer ready".into(),
            ));
        }
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.materials.preparation.cancel();
        self.prototypes.cancel();
    }
    pub fn drained(&mut self, world: &World) -> bool {
        self.materials.preparation.drained()
            && self.writers.drained()
            && self.prototypes.drain_ready(
                world.resource::<AssetServer>(),
                world.resource::<Assets<super::world::prototype_loader::SourceGltf>>(),
            )
            && self.materials.drain_ready(
                world.resource::<AssetServer>(),
                world.resource::<Assets<Image>>(),
            )
    }
    pub fn release(mut self, _candidate: &mut GarageRuntime, world: &mut World) {
        *world.resource_mut::<super::world::pipelines::CandidatePipelines>() = default();

        self.exchange(world);
        {
            let mut r = self.system.get_mut(world).expect("source owner resources");
            r.instances.release_all(&mut r.commands, &mut r.materials);
            r.grass.release_all(&mut r.meshes, &mut r.materials.grass);
            r.writers.release_all();
            r.surfaces.retire_session(&mut r.commands, &mut r.meshes);
            r.surfaces.reset_bindings(&mut r.terrain, &mut r.casters);
            r.lights.road_lamps.reset_instances(&mut r.commands);
        }
        self.system.apply(world);
        self.exchange(world);
        world.resource_scope(|world,mut assets:Mut<Assets<super::world::prototype_loader::SourceGltf>>|world.resource_scope(|world,mut meshes:Mut<Assets<Mesh>>|{self.prototypes.release_all(&mut assets,&mut meshes,&mut world.resource_mut::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>());}));
        world.resource_scope(|world, mut images: Mut<Assets<Image>>| {
            world.resource_scope(
                |world, mut materials: Mut<Assets<materials::SourceMaterial>>| {
                    self.materials
                        .release_all(world.resource::<AssetServer>(), &mut images, &mut materials)
                        .expect("drained source material ownership");
                },
            )
        });
    }
    pub fn commit(mut self, old: &mut GarageRuntime, world: &mut World) {
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
        self.exchange(world);
        self.release(old, world);
    }
}
fn imported_updates(
    candidate: &GarageRuntime,
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
        if let Some(binding) = body {
            let handle = candidate.area.bodies.get(&binding.body).ok_or_else(|| {
                SaveError::InvalidLinks(format!("Missing visual body {}", binding.body.0))
            })?;
            active = candidate.physics.body_active(*handle);
            let (_, _, saved) = candidate
                .area
                .render_poses
                .iter()
                .find(|(id, _, _)| *id == binding.body)
                .ok_or_else(|| SaveError::InvalidLinks("Missing body render pose".into()))?;
            pose = super::transform(*saved).with_scale(binding.scale);
        } else if let Some(binding) = item {
            let saved = candidate.area.items.get(binding.item).ok_or_else(|| {
                SaveError::InvalidLinks(format!("Missing visual item {}", binding.item.0))
            })?;
            active = saved.inactive_pose.is_none();
            let (_, _, rendered) = candidate
                .area
                .item_render_poses
                .iter()
                .find(|(id, _, _)| *id == binding.item)
                .ok_or_else(|| SaveError::InvalidLinks("Missing item render pose".into()))?;
            pose = super::transform(*rendered).with_scale(binding.scale);
        }
        if body.is_some() || item.is_some() || valve.is_some() {
            visibility = if active
                && valve.is_none_or(|v| {
                    (candidate.area.interaction.opening_valve(v.opening) > 0.) == v.open
                }) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if let Some(cap) = cap {
            let amount = candidate.area.interaction.opening_valve(cap.opening);
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
fn validate_assets(reference: &ReferenceManifest, world: &World) -> Result<(), SaveError> {
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
        !matches!(
            a.usage,
            AssetUsage::WorldPrototype | AssetUsage::WorldMaterial
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
    if source.state != lighting::RenderingState::Ready
        || !source.materials_ready
        || source.warming
        || source.road_lamps.recipe.is_none()
    {
        return Err(fail("Lighting assets are not ready".into()));
    }
    if materials.state != materials::RenderingState::Ready {
        return Err(fail("Material assets are not ready".into()));
    }
    let instruments = world.resource::<super::instruments::InstrumentVisuals>();
    if !instruments.bound || !instruments.prepared {
        return Err(fail("Instrument bindings are not ready".into()));
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
        if !server.is_loaded_with_dependencies(handle.id())
            || !world.resource::<Assets<Image>>().contains(handle.id())
        {
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
            choice.validate_assets(authored, standard)?;
        }
    }
    instruments.validate_assets(authored, standard)?;
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
