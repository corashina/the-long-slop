//! Source publication order and shared presentation ownership.
use super::{
    SourceWorldVisuals, grass_jobs::GrassJobs, instance_materials::InstanceMaterials,
    instances::SourceInstances, pipelines::WorldPipelines, prototype_loader::SourceGltf,
    prototypes::SourcePrototypes,
};
use crate::{
    presentation::{
        lighting::SourceLighting,
        materials::{
            grass::{GrassView, writers::GrassWriters},
            terrain::TerrainMaterial,
        },
    },
    reference::{ReferenceError, ReferenceManifest},
    starting_area::StartingArea,
    vehicle::PhysicsWorld,
    world::{WorldBounds, WorldStream},
};
use bevy::{
    ecs as bevy_ecs, ecs::system::SystemParam, mesh::skinning::SkinnedMeshInverseBindposes,
    prelude::*,
};
#[derive(Resource, Default)]
pub struct SourcePresentationAllocation {
    session: u64,
    lamps: (usize, usize, usize),
    imported_writers: Vec<(Entity, Handle<Mesh>)>,
    writers_bound: bool,
    retired_materials: Vec<crate::reference::SourceId>,
}
impl SourcePresentationAllocation {
    pub fn for_session(session: u64) -> Self {
        Self {
            session,
            ..default()
        }
    }
}
#[derive(SystemParam)]
pub struct WorldPresentation<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub roots: Query<'w, 's, &'static mut Transform>,
    pub prototypes: ResMut<'w, SourcePrototypes>,
    pub pipelines: ResMut<'w, WorldPipelines>,
    pub surfaces: ResMut<'w, SourceWorldVisuals>,
    pub instances: ResMut<'w, SourceInstances>,
    pub grass: ResMut<'w, GrassJobs>,
    pub writers: ResMut<'w, GrassWriters>,
    pub materials: InstanceMaterials<'w>,
    pub terrain: ResMut<'w, Assets<TerrainMaterial>>,
    pub casters: ResMut<'w, Assets<crate::presentation::materials::lod::SurfaceCasterMaterial>>,
    pub assets: ResMut<'w, Assets<SourceGltf>>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub skins: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    pub images: ResMut<'w, Assets<Image>>,
    pub server: Res<'w, AssetServer>,
    pub lights: ResMut<'w, SourceLighting>,
    pub imported: Res<'w, crate::presentation::reference::ReferenceVisuals>,
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
        view: GrassView,
        time: f32,
        lod_view: super::lod::LodView,
        receivers: &[WorldBounds],
        enabled: bool,
        light_outputs: &[crate::lighting::LightOutput],
    ) -> Result<(), ReferenceError> {
        super::preflight::configure_staging(stream)?;
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
            self.grass.release_domains();
            self.materials.cache.begin_world_session();
            self.surfaces
                .retire_session(&mut self.commands, &mut self.meshes);
            self.surfaces
                .reset_bindings(&mut self.terrain, &mut self.casters);
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
        self.prototypes.retire_unused(stream, reference);
        let mut failure = self.pipelines.prepare(stream).err();
        self.writers.configure(stream, &reference.world)?;
        self.writers.update_frame(
            view.position,
            Vec3::from_array(stream.origin.offset.map(|v| v as f32)),
            time,
        );
        if let Err(error) = self.materials.cache.prepare_world_primary(
            stream,
            &self.server,
            &mut self.images,
            &mut self.materials.primary,
        ) {
            failure.get_or_insert(error);
        }
        if let Err(error) =
            self.materials
                .cache
                .prepare_world_deferred(stream, &self.server, &mut self.images)
        {
            failure.get_or_insert(error);
        }
        if let Err(error) = self.prototypes.prepare(
            stream,
            reference,
            &self.server,
            &mut self.assets,
            &mut self.meshes,
            &mut self.skins,
        ) {
            failure.get_or_insert(error);
        }
        stream.prepare();
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
                &self.images,
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
                            &self.images,
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
                    &self.images,
                    &self.server,
                )
            {
                self.materials.cache.release_world_asset(
                    binding.asset,
                    &mut self.images,
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
        self.surfaces.prepare_bindings(
            stream,
            reference,
            &mut self.prototypes,
            &mut self.materials.cache,
            &mut self.materials.primary,
            &mut self.terrain,
            &mut self.casters,
        )?;
        self.instances.prepare(
            stream,
            reference,
            &mut self.prototypes,
            &self.assets,
            &mut self.meshes,
            &mut self.materials,
            &mut self.grass,
            &mut self.pipelines,
            &mut self.commands,
            view,
        )?;
        self.instances.select_lods(
            reference,
            lod_view,
            &mut self.commands,
            &mut self.materials,
            &mut self.grass,
        );
        self.surfaces
            .prepare_surface(stream, &mut self.meshes, &mut self.pipelines)?;
        let lamps = super::lamps::source_lamp_admission(
            &stream.frame(),
            receivers,
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
        // Light admission and complete renderer preparation precede collision mutation.
        if let Some(required) = self.lights.road_lamps.admitted_lights() {
            stream.publish(physics, required)?;
        }
        stream.sync_supplies(
            reference,
            &mut area.items,
            &mut area.interaction,
            physics,
            wheels,
        )?;
        self.surfaces.sync(
            &stream.frame(),
            &mut self.commands,
            &mut self.meshes,
            &mut self.roots,
        )?;
        self.instances.sync(
            stream,
            reference,
            physics,
            &area.items,
            &mut self.commands,
            &mut self.roots,
            &mut self.materials,
        )?;
        super::lamps::sync_source_lamps(
            &stream.frame(),
            enabled,
            &mut self.lights,
            &mut self.commands,
            &mut self.roots,
        )?;
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
        self.grass.prepare(
            stream,
            self.writers.ready()?,
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials.grass,
            &mut self.pipelines,
        )?;
        self.instances
            .request_grass(stream, &mut self.grass, view)?;
        stream.capture_changes(reference, &mut area.items, &area.interaction, physics);
        stream.retire_structures(physics, &mut area.items, &mut area.interaction, wheels);
        self.instances.retire(
            stream,
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &mut self.grass,
            &mut self.writers,
        )?;
        self.prototypes.retire_unused(stream, reference);
        Ok(())
    }
}
