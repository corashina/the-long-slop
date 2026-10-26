//! Source publication order and shared presentation ownership.
use super::{
    SourceWorldVisuals, grass_jobs::GrassJobs, instance_materials::InstanceMaterials,
    instances::SourceInstances, pipelines::WorldPipelines, prototype_loader::SourceGltf,
    prototypes::SourcePrototypes,
};
use crate::{
    presentation::{
        lighting::SourceLighting,
        materials::{grass::writers::GrassWriters, terrain::TerrainMaterial},
    },
    reference::{ReferenceError, ReferenceManifest},
    starting_area::StartingArea,
    vehicle::PhysicsWorld,
    world::WorldStream,
};
use bevy::{
    ecs as bevy_ecs, ecs::system::SystemParam, mesh::skinning::SkinnedMeshInverseBindposes,
    prelude::*,
};
use grass_mesh::GrassView;
#[derive(Resource, Default)]
pub struct SourcePresentationAllocation {
    session: u64,
    pub(crate) fixed: crate::reference::fixed_reservation::FixedAllocation,
    pub(crate) lighting_cpu: usize,
    instance_cache_cpu: usize,
    lamps: (usize, usize, usize),
    imported_writers: Vec<(Entity, Handle<Mesh>)>,
    writers_bound: bool,
    retired_materials: Vec<crate::reference::SourceId>,
}
impl SourcePresentationAllocation {
    pub fn bootstrap(
        session: u64,
        fixed: crate::reference::fixed_reservation::FixedAllocation,
    ) -> Self {
        Self {
            session,
            fixed,
            ..default()
        }
    }
    pub fn for_session(session: u64) -> Self {
        Self {
            session,
            ..default()
        }
    }
}
/// World object roots. The drive camera is excluded, so systems can also read its Transform.
pub type WorldRoots<'w, 's> =
    Query<'w, 's, &'static mut Transform, Without<crate::presentation::camera::DriveCamera>>;
#[derive(SystemParam)]
pub struct WorldPresentation<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub roots: WorldRoots<'w, 's>,
    pub prototypes: ResMut<'w, SourcePrototypes>,
    pub pipelines: ResMut<'w, WorldPipelines>,
    pub surfaces: ResMut<'w, SourceWorldVisuals>,
    pub instances: ResMut<'w, SourceInstances>,
    pub grass: ResMut<'w, GrassJobs>,
    pub writers: ResMut<'w, GrassWriters>,
    pub materials: InstanceMaterials<'w>,
    pub terrain: ResMut<'w, Assets<TerrainMaterial>>,
    pub assets: ResMut<'w, Assets<SourceGltf>>,
    pub plans: ResMut<'w, Assets<super::preflight::SourcePlan>>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub skins: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    pub batches: ResMut<'w, Assets<crate::presentation::texture_batches::SourceTextureBatch>>,
    pub chains: ResMut<'w, Assets<crate::presentation::primary_image::SourcePrimaryImage>>,
    pub images: ResMut<'w, Assets<Image>>,
    pub server: Res<'w, AssetServer>,
    pub lights: ResMut<'w, SourceLighting>,
    pub imported: ResMut<'w, crate::presentation::reference::ReferenceVisuals>,
    pub fixed: ResMut<'w, crate::presentation::fixed_assets::SourceFixedAssets>,
    pub gltfs: Res<'w, Assets<bevy::gltf::Gltf>>,
    pub gltf_meshes: Res<'w, Assets<bevy::gltf::GltfMesh>>,
    pub shaders: Res<'w, Assets<Shader>>,
    pub dither: ResMut<'w, Assets<crate::presentation::lighting_assets::CasterDitherBytes>>,
    pub lighting_assets: ResMut<'w, crate::presentation::lighting_assets::LightingAssets>,
    pub imported_meshes: Query<'w, 's, (Entity, &'static ChildOf, &'static Mesh3d)>,
    pub world_poses: Query<'w, 's, &'static GlobalTransform>,
    pub saved_poses: Res<'w, crate::presentation::save::CandidateImportedPoses>,
    pub allocation: ResMut<'w, SourcePresentationAllocation>,
}
impl WorldPresentation<'_, '_> {
    pub fn prepare(
        &mut self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        physics: &mut PhysicsWorld,
        area: &mut StartingArea,
        wheels: &mut crate::interaction::wheels::WheelAssemblies,
        vehicles: &mut std::collections::BTreeMap<
            crate::world::ObjectKey,
            crate::starting_area::VehicleRuntime,
        >,
        powertrain: &mut crate::simulation::powertrain::PowertrainParts,
        view: GrassView,
        time: f32,
        lod_view: super::lod::LodView,
        enabled: bool,
        light_outputs: &[crate::lighting::LightOutput],
        required_views: &[crate::world::RequiredView],
        budget_ms: f64,
    ) -> Result<(), ReferenceError> {
        let started = crate::browser::game_now();
        loop {
            let before = stream.metrics();
            let remaining = (budget_ms - (crate::browser::game_now() - started)).max(0.);
            self.prepare_step(
                stream,
                reference,
                physics,
                area,
                wheels,
                vehicles,
                powertrain,
                view,
                time,
                lod_view,
                enabled,
                light_outputs,
                required_views,
                remaining,
            )?;
            if budget_ms <= 0.
                || crate::browser::game_now() - started >= budget_ms
                || before == stream.metrics()
            {
                return Ok(());
            }
        }
    }
    fn prepare_step(
        &mut self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        physics: &mut PhysicsWorld,
        area: &mut StartingArea,
        wheels: &mut crate::interaction::wheels::WheelAssemblies,
        vehicles: &mut std::collections::BTreeMap<
            crate::world::ObjectKey,
            crate::starting_area::VehicleRuntime,
        >,
        powertrain: &mut crate::simulation::powertrain::PowertrainParts,
        view: GrassView,
        time: f32,
        lod_view: super::lod::LodView,
        enabled: bool,
        light_outputs: &[crate::lighting::LightOutput],
        required_views: &[crate::world::RequiredView],
        budget_ms: f64,
    ) -> Result<(), ReferenceError> {
        let _profile =
            crate::app::cpu_profile::Timer::new(crate::app::cpu_profile::Stage::RenderPreparation);
        let mut timer =
            crate::world::stream_profile::Timer::new(crate::world::stream_profile::Stage::Cleanup);
        super::preflight::configure_staging(stream, &reference.file_catalog)?;
        let mut failure = self
            .fixed
            .poll(
                stream,
                &mut self.allocation.fixed,
                reference,
                &self.server,
                &self.gltfs,
                &self.gltf_meshes,
                &self.shaders,
                &self.meshes,
                &mut self.images,
                &mut self.dither,
                &mut self.lights,
                &mut self.lighting_assets,
            )
            .err();
        if self.allocation.session != stream.frame().request.session {
            self.allocation.session = stream.frame().request.session;
            self.allocation.lamps = (0, 0, 0);
            self.allocation.writers_bound = false;
            self.allocation.imported_writers.clear();
            self.instances.retire(
                stream,
                &mut self.commands,
                &mut self.meshes,
                &mut self.materials,
                &mut self.grass,
                &mut self.writers,
            )?;
            self.grass.release_domains(stream)?;
            self.materials.cache.begin_world_session();
            self.surfaces
                .retire_session(&mut self.commands, &mut self.meshes);
            self.surfaces.reset_bindings(&mut self.terrain);
        }
        // Cancellation and cleanup run before any material or stream failure can return.
        self.instances.retire(
            stream,
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &mut self.grass,
            &mut self.writers,
        )?;
        self.surfaces.retire_stale(stream, &mut self.meshes)?;
        self.prototypes
            .retire_unused(stream, reference, &self.instances);
        let lighting_cpu = self.lights.clone_bytes();
        let mut controls = stream.presentation_allocation();
        controls.active_cpu = controls
            .active_cpu
            .checked_sub(self.allocation.lighting_cpu)
            .and_then(|n| n.checked_add(lighting_cpu))
            .ok_or_else(|| ReferenceError("Fixed lighting control accounting".into()))?;
        stream.set_presentation_allocation(controls)?;
        self.allocation.lighting_cpu = lighting_cpu;
        timer.next(crate::world::stream_profile::Stage::FixedInputs);
        if let Err(error) = self.fixed.prepare_metadata(
            stream,
            &mut self.allocation.fixed,
            reference,
            &self.server,
            &self.plans,
        ) {
            failure.get_or_insert(error);
        }
        if failure.is_some() || stream.frame().failure.is_some() {
            self.materials.cache.cancel_fixed();
        }
        if failure.is_none() && stream.frame().failure.is_none() && self.fixed.descriptors_ready() {
            if let Err(error) = self.fixed.admit_payloads(
                stream,
                &mut self.allocation.fixed,
                reference,
                &self.server,
                &mut self.imported,
                &mut self.materials.cache,
                &mut self.lights,
                &mut self.lighting_assets,
                &mut self.images,
            ) {
                failure.get_or_insert(error);
            }
        }
        timer.next(crate::world::stream_profile::Stage::Pipelines);
        if let Err(error) = self.pipelines.prepare(stream) {
            failure.get_or_insert(error);
        }
        self.writers.configure(stream, &reference.world)?;
        self.writers.update_frame(
            view.position,
            Vec3::from_array(stream.origin.offset.map(|v| v as f32)),
            time,
        );
        timer.next(crate::world::stream_profile::Stage::PrimaryTextures);
        if let Err(error) = self.materials.cache.prepare_world_primary(
            stream,
            &self.server,
            &mut self.images,
            &mut self.chains,
            &mut self.materials.primary,
        ) {
            failure.get_or_insert(error);
        }
        timer.next(crate::world::stream_profile::Stage::DeferredTextures);
        if let Err(error) = self.materials.cache.prepare_world_deferred(
            stream,
            &self.server,
            &mut self.images,
            &mut self.batches,
        ) {
            failure.get_or_insert(error);
        }
        timer.next(crate::world::stream_profile::Stage::Prototypes);
        if let Err(error) = self.prototypes.prepare(
            stream,
            reference,
            &self.server,
            &mut self.assets,
            &mut self.plans,
            &mut self.meshes,
            &mut self.skins,
        ) {
            failure.get_or_insert(error);
        }
        timer.next(crate::world::stream_profile::Stage::Stream);
        stream.prepare(budget_ms);
        stream.select_required(required_views);
        timer.next(crate::world::stream_profile::Stage::MaterialRetirement);
        while let Some(source) = self.prototypes.take_retired() {
            self.allocation.retired_materials.push(source);
        }
        let mut index = 0;
        while index < self.allocation.retired_materials.len() {
            let source = &self.allocation.retired_materials[index];
            if self.prototypes.requested(source) {
                self.allocation.retired_materials.swap_remove(index);
                continue;
            }
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == *source)
                .expect("retired prototype");
            if !self.materials.cache.world_asset_retirable(
                prototype.asset,
                &self.chains,
                &self.server,
            ) || prototype.deferred_materials.iter().any(|m| {
                !self.materials.cache.world_textures.retirable(
                    &m.source,
                    &self.images,
                    &self.server,
                )
            }) || prototype
                .nodes
                .iter()
                .filter_map(|n| super::preflight::initial_material(reference, &n.source))
                .any(|(asset, _)| {
                    !self.prototypes.references_asset(reference, asset)
                        && !self.materials.cache.world_asset_retirable(
                            asset,
                            &self.chains,
                            &self.server,
                        )
                })
            {
                index += 1;
                continue;
            }
            self.materials.cache.release_world_asset(
                prototype.asset,
                &mut self.images,
                &mut self.chains,
                &mut self.materials.primary,
                &self.server,
            )?;
            for (asset, _) in prototype
                .nodes
                .iter()
                .filter_map(|n| super::preflight::initial_material(reference, &n.source))
            {
                if !self.prototypes.references_asset(reference, asset) {
                    self.materials.cache.release_world_asset(
                        asset,
                        &mut self.images,
                        &mut self.chains,
                        &mut self.materials.primary,
                        &self.server,
                    )?;
                }
            }
            for material in &prototype.deferred_materials {
                if !self
                    .prototypes
                    .references_material(reference, &material.source)
                {
                    self.materials.cache.world_textures.release(
                        &material.source,
                        &mut self.images,
                        &self.server,
                    )?;
                }
            }
            self.allocation.retired_materials.swap_remove(index);
        }
        for binding in &reference.world.painting_materials {
            if !stream.uses_painting_asset(binding.asset)
                && self.materials.cache.world_asset_retirable(
                    binding.asset,
                    &self.chains,
                    &self.server,
                )
            {
                self.materials.cache.release_world_asset(
                    binding.asset,
                    &mut self.images,
                    &mut self.chains,
                    &mut self.materials.primary,
                    &self.server,
                )?;
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        if let Some(error) = stream.frame().failure {
            return Err(ReferenceError(error.to_owned()));
        }
        timer.next(crate::world::stream_profile::Stage::Descriptors);
        self.surfaces
            .request_descriptors(stream, reference, &mut self.prototypes)?;
        self.instances
            .request_descriptors(stream, reference, &mut self.prototypes)?;
        // MaterialProperties resolves ShaderRef::Path during preparation.
        // Descriptor discovery stays available while fixed shader inputs wait.
        if !self.fixed.pipeline_inputs_ready() {
            return Ok(());
        }
        timer.next(crate::world::stream_profile::Stage::Bindings);
        self.surfaces.prepare_bindings(
            stream,
            reference,
            &mut self.prototypes,
            &self.server,
            &mut self.materials.cache,
            &mut self.materials.primary,
            &mut self.terrain,
        )?;
        timer.next(crate::world::stream_profile::Stage::Instances);
        self.instances.prepare(
            stream,
            reference,
            &mut self.prototypes,
            &self.server,
            &self.assets,
            &mut self.meshes,
            &mut self.materials,
            &mut self.grass,
            &mut self.pipelines,
            &self.fixed,
            &mut self.commands,
            view,
        )?;
        timer.next(crate::world::stream_profile::Stage::Lods);
        self.instances.select_lods(
            reference,
            lod_view,
            &mut self.commands,
            &mut self.materials,
            &mut self.grass,
        );
        timer.next(crate::world::stream_profile::Stage::Surface);
        self.surfaces.prepare_surface(
            stream,
            &mut self.meshes,
            &mut self.pipelines,
            &self.fixed,
        )?;
        timer.next(crate::world::stream_profile::Stage::Lamps);
        let lamps = super::lamps::source_lamp_admission(
            &stream.frame(),
            stream.required_content().lights,
            &mut self.lights,
            reference,
            light_outputs,
            enabled,
        )?;
        let old = self.allocation.lamps;
        let mut a = stream.presentation_allocation();
        a.active_cpu = a.active_cpu.saturating_sub(old.0) + lamps.0;
        a.active_gpu = a.active_gpu.saturating_sub(old.1) + lamps.1;
        a.objects = a.objects.saturating_sub(old.2) + lamps.2;
        stream.set_presentation_allocation(a)?;
        self.allocation.lamps = lamps;
        timer.next(crate::world::stream_profile::Stage::Publication);
        // Light admission and complete renderer preparation precede collision mutation.
        if let Some(required) = self.lights.road_lamps.admitted_lights() {
            stream.publish(physics, required)?;
        }
        timer.next(crate::world::stream_profile::Stage::Supplies);
        stream.sync_supplies(
            reference,
            &mut area.items,
            &mut area.interaction,
            physics,
            wheels,
            vehicles,
            powertrain,
        )?;
        timer.next(crate::world::stream_profile::Stage::SurfaceSync);
        self.surfaces.sync(
            &stream.frame(),
            &mut self.commands,
            &mut self.meshes,
            &mut self.roots,
        )?;
        timer.next(crate::world::stream_profile::Stage::InstanceSync);
        self.instances.sync(
            stream,
            reference,
            physics,
            &area.items,
            vehicles,
            powertrain,
            &mut self.commands,
            &mut self.roots,
            &mut self.materials,
        )?;
        let instance_cache_cpu = self.instances.cache_bytes();
        if self.allocation.instance_cache_cpu != instance_cache_cpu {
            let mut allocation = stream.presentation_allocation();
            allocation.active_cpu = allocation
                .active_cpu
                .checked_sub(self.allocation.instance_cache_cpu)
                .and_then(|bytes| bytes.checked_add(instance_cache_cpu))
                .ok_or_else(|| ReferenceError("Generated presentation cache accounting".into()))?;
            stream.set_presentation_allocation(allocation)?;
            self.allocation.instance_cache_cpu = instance_cache_cpu;
        }
        physics.clear_presentation_bodies();
        stream.clear_presentation_changes();
        timer.next(crate::world::stream_profile::Stage::LampSync);
        super::lamps::sync_source_lamps(
            &stream.frame(),
            reference,
            enabled,
            &mut self.lights,
            &mut self.commands,
            &mut self.roots,
        )?;
        timer.next(crate::world::stream_profile::Stage::Writers);
        self.instances
            .request_writers(stream, reference, &mut self.writers, &self.meshes)?;
        if !self.allocation.writers_bound && self.imported.bound {
            for writer in &reference.world.grass_displacement.writers {
                if !writer.enabled {
                    continue;
                }
                let Some(node) = reference
                    .nodes
                    .iter()
                    .find(|n| n.source == writer.transform && n.active)
                else {
                    continue;
                };
                let Some(&entity) = self.imported.nodes.get(&node.id) else {
                    continue;
                };
                for (primitive, parent, mesh) in &self.imported_meshes {
                    if parent.parent() == entity {
                        self.allocation
                            .imported_writers
                            .push((primitive, mesh.0.clone()));
                    }
                }
            }
            let mut a = stream.presentation_allocation();
            a.active_cpu += self.allocation.imported_writers.capacity()
                * std::mem::size_of::<(Entity, Handle<Mesh>)>();
            stream.set_presentation_allocation(a)?;
            self.allocation.writers_bound = true;
        }
        for (entity, mesh) in &self.allocation.imported_writers {
            if let Some(pose) = self.saved_poses.0.get(entity).copied().or_else(|| {
                self.world_poses
                    .get(*entity)
                    .ok()
                    .map(GlobalTransform::to_matrix)
            }) {
                self.writers
                    .request(stream, *entity, mesh.clone(), pose, &self.meshes)?;
            }
        }
        self.writers.prepare(stream, &self.meshes)?;
        // Existing expansions must release their slots even when a new writer is pending.
        timer.next(crate::world::stream_profile::Stage::Grass);
        self.grass.prepare(
            stream,
            self.writers.ready()?,
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials.grass,
            &mut self.pipelines,
            &self.fixed,
        )?;
        self.instances
            .request_grass(stream, &mut self.grass, view)?;
        timer.next(crate::world::stream_profile::Stage::Capture);
        stream.capture_changes(reference, &mut area.items, &area.interaction, physics);
        timer.next(crate::world::stream_profile::Stage::FinalCleanup);
        stream.retire_structures(
            physics,
            &mut area.items,
            &mut area.interaction,
            wheels,
            vehicles,
            powertrain,
        )?;
        self.instances.retire(
            stream,
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &mut self.grass,
            &mut self.writers,
        )?;
        self.prototypes
            .retire_unused(stream, reference, &self.instances);
        Ok(())
    }
}
