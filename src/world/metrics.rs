//! Retained world ownership and logical allocation reservations.
//! These charges are not process memory or driver residency measurements.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ResourceBudget {
    pub name: &'static str,
    pub used: usize,
    pub limit: usize,
    pub unit: &'static str,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct WorldMetrics {
    pub shared_input_cpu_bytes: usize,
    /// Subset of active CPU: imported item snapshots retained for change capture.
    pub imported_snapshot_bytes: usize,
    pub road_checkpoints: usize,
    pub building_checkpoints: usize,
    pub road_replay_ms: f64,
    pub peak_road_replay_ms: f64,
    pub generation_jobs: usize,
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
    pub spawn_parents: usize,
    pub spawn_boundaries: usize,
    pub spawn_boundary_bytes: usize,
    pub prepared_roads: u32,
    pub prepared_buildings: u32,
    pub last_road_work: usize,
    pub last_terrain_work: usize,
}

/// Hash-table allocation bound includes spare buckets and control bytes.
pub(crate) fn map_bytes(capacity: usize, entry: usize) -> usize {
    if capacity == 0 {
        0
    } else {
        capacity
            .saturating_mul(2)
            .saturating_mul(entry + 1)
            .saturating_add(16)
    }
}

/// Reserve table growth before inserting a new owner. Include old/new table overlap.
pub(crate) fn map_insert_bytes<K, V>(map: &std::collections::HashMap<K, V>) -> usize {
    if map.len() < map.capacity() {
        0
    } else {
        map_bytes(
            map.capacity().saturating_add(1).saturating_mul(2),
            std::mem::size_of::<(K, V)>(),
        )
    }
}

/// Development snapshots. Asset handles are counted once, including caster sharing.
#[cfg(target_arch = "wasm32")]
pub(crate) mod browser {
    use crate::presentation::{
        materials::SourceMaterials,
        reference::ReferenceVisuals,
        world::{
            SourceWorldVisuals, grass_jobs::GrassJobs, instances::SourceInstances,
            prototype_loader::SourceGltf,
        },
    };
    use bevy::{
        asset::{AssetEvent, AssetId},
        ecs as bevy_ecs,
        mesh::{Indices, VertexAttributeValues},
        prelude::*,
    };
    use serde::Serialize;
    use std::collections::HashMap;

    #[derive(Resource, Default)]
    pub struct MeshMeasurements {
        bytes: HashMap<AssetId<Mesh>, usize>,
    }

    // Run in Last, before render extraction discards CPU vertex arrays.
    // Removed assets release their entry. Idle frames do not allocate.
    pub fn collect_mesh_bytes(
        mut events: MessageReader<AssetEvent<Mesh>>,
        meshes: Res<Assets<Mesh>>,
        mut measurements: ResMut<MeshMeasurements>,
    ) {
        for event in events.read() {
            match *event {
                AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                    if let Some(mesh) = meshes.get(id) {
                        if mesh.try_attributes().is_ok() {
                            measurements.bytes.insert(
                                id,
                                mesh.get_vertex_buffer_size()
                                    + mesh.get_index_buffer_bytes().map_or(0, <[u8]>::len),
                            );
                        }
                    }
                }
                AssetEvent::Removed { id } => {
                    measurements.bytes.remove(&id);
                }
                _ => {}
            }
        }
    }

    #[derive(Default, Serialize)]
    struct AssetsMeasured {
        mesh_handles: usize,
        image_handles: usize,
        logical_gpu_bytes: usize,
        unknown_gpu_handles: usize,
        retained_cpu_payload_bytes: usize,
    }
    #[derive(Default, Serialize)]
    struct AssetGroups {
        imported_baseline: AssetsMeasured,
        world_prototypes: AssetsMeasured,
        active_world: AssetsMeasured,
        pending_world: AssetsMeasured,
        other_renderer: AssetsMeasured,
    }
    impl AssetGroups {
        fn group(&mut self, group: u8) -> &mut AssetsMeasured {
            match group {
                0 => &mut self.imported_baseline,
                1 => &mut self.world_prototypes,
                2 => &mut self.active_world,
                3 => &mut self.pending_world,
                _ => &mut self.other_renderer,
            }
        }
    }
    fn vector_bytes<T>(v: &Vec<T>) -> usize {
        v.capacity() * std::mem::size_of::<T>()
    }
    fn attribute_bytes(values: &VertexAttributeValues) -> usize {
        macro_rules! capacities { ($($variant:ident),*) => { match values { $(VertexAttributeValues::$variant(v) => vector_bytes(v),)* } }; }
        capacities!(
            Uint8,
            Uint8x2,
            Uint8x4,
            Sint8,
            Sint8x2,
            Sint8x4,
            Unorm8,
            Unorm8x2,
            Unorm8x4,
            Snorm8,
            Snorm8x2,
            Snorm8x4,
            Uint16,
            Uint16x2,
            Uint16x4,
            Sint16,
            Sint16x2,
            Sint16x4,
            Unorm16,
            Unorm16x2,
            Unorm16x4,
            Snorm16,
            Snorm16x2,
            Snorm16x4,
            Float16,
            Float16x2,
            Float16x4,
            Float32,
            Float32x2,
            Float32x3,
            Float32x4,
            Uint32,
            Uint32x2,
            Uint32x3,
            Uint32x4,
            Sint32,
            Sint32x2,
            Sint32x3,
            Sint32x4,
            Float64,
            Float64x2,
            Float64x3,
            Float64x4,
            Unorm10_10_10_2,
            Unorm8x4Bgra
        )
    }
    pub(crate) fn mesh_payload(mesh: &Mesh) -> usize {
        mesh.try_attributes().map_or(0, |attributes| {
            attributes.map(|(_, v)| attribute_bytes(v)).sum::<usize>()
        }) + mesh
            .try_indices_option()
            .ok()
            .flatten()
            .map_or(0, |indices| match indices {
                Indices::U16(v) => vector_bytes(v),
                Indices::U32(v) => vector_bytes(v),
            })
    }
    fn image_bytes(image: &Image) -> Option<usize> {
        let d = &image.texture_descriptor;
        let bytes = d.format.block_copy_size(None)? as usize;
        let (w, h) = d.format.block_dimensions();
        Some(
            (0..d.mip_level_count)
                .map(|level| {
                    let size = d.size.mip_level_size(level, d.dimension);
                    size.width.div_ceil(w) as usize
                        * size.height.div_ceil(h) as usize
                        * size.depth_or_array_layers as usize
                        * bytes
                        * d.sample_count as usize
                })
                .sum(),
        )
    }
    #[derive(Serialize)]
    struct Snapshot {
        session: u64,
        revision: u64,
        world: super::WorldMetrics,
        startup: serde_json::Value,
        assets: AssetGroups,
        physics_bodies: usize,
        physics_colliders: usize,
        active_items: usize,
        measurement_cache_cpu_bytes: usize,
        process_bytes: Option<usize>,
        driver_bytes: Option<usize>,
        limits: &'static str,
    }
    #[allow(clippy::too_many_arguments)]
    pub fn publish(
        drive: Res<crate::app::Drive>,
        measured: Res<MeshMeasurements>,
        meshes: Res<Assets<Mesh>>,
        images: Res<Assets<Image>>,
        imported: Res<ReferenceVisuals>,
        gltfs: Res<Assets<bevy::gltf::Gltf>>,
        gltf_meshes: Res<Assets<bevy::gltf::GltfMesh>>,
        prototypes: Res<Assets<SourceGltf>>,
        surfaces: Res<SourceWorldVisuals>,
        instances: Res<SourceInstances>,
        grass: Res<GrassJobs>,
        writers: Res<crate::presentation::materials::grass::writers::GrassWriters>,
        materials: Res<SourceMaterials>,
        server: Res<AssetServer>,
        pipelines: Res<crate::presentation::world::pipelines::WorldPipelines>,
        fixed: (
            Res<crate::presentation::fixed_assets::SourceFixedAssets>,
            Res<crate::app::RuntimeReference>,
        ),
    ) {
        if !crate::browser::game_measure_requested() {
            return;
        }
        let (fixed, reference) = fixed;
        let mut mesh_owners = HashMap::new();
        for (_, handle) in &imported.gltfs {
            if let Some(gltf) = gltfs.get(handle) {
                for mesh in &gltf.meshes {
                    if let Some(mesh) = gltf_meshes.get(mesh) {
                        for primitive in &mesh.primitives {
                            mesh_owners.insert(primitive.mesh.id(), 0);
                        }
                    }
                }
            }
        }
        for (_, prototype) in prototypes.iter() {
            for handle in &prototype.meshes {
                mesh_owners.entry(handle.id()).or_insert(1);
            }
        }
        for (id, pending) in surfaces
            .measurement_meshes()
            .chain(instances.measurement_meshes())
            .chain(grass.measurement_meshes())
        {
            mesh_owners.entry(id).or_insert(if pending { 3 } else { 2 });
        }
        let mut image_owners = HashMap::new();
        for (id, world) in materials.measurement_images() {
            image_owners
                .entry(id)
                .and_modify(|group: &mut u8| *group = (*group).min(u8::from(world)))
                .or_insert(u8::from(world));
        }
        let mut groups = AssetGroups::default();
        for (id, mesh) in meshes.iter() {
            let group = groups.group(*mesh_owners.get(&id).unwrap_or(&4));
            group.mesh_handles += 1;
            group.retained_cpu_payload_bytes += mesh_payload(mesh);
            if !mesh
                .asset_usage
                .contains(bevy::asset::RenderAssetUsages::RENDER_WORLD)
            {
                continue;
            }
            if let Some(bytes) = measured.bytes.get(&id) {
                group.logical_gpu_bytes += bytes;
            } else {
                group.unknown_gpu_handles += 1;
            }
        }
        for (id, image) in images.iter() {
            let group = groups.group(*image_owners.get(&id).unwrap_or(&4));
            group.image_handles += 1;
            group.retained_cpu_payload_bytes += image.data.as_ref().map_or(0, Vec::capacity);
            if !image
                .asset_usage
                .contains(bevy::asset::RenderAssetUsages::RENDER_WORLD)
            {
                continue;
            }
            if let Some(bytes) = image_bytes(image) {
                group.logical_gpu_bytes += bytes;
            } else {
                group.unknown_gpu_handles += 1;
            }
        }
        let runtime = &drive.runtime;
        let frame = runtime.stream.frame();
        let car = runtime.physics.snapshot(runtime.vehicle);
        let snapshot = Snapshot {
            session: frame.request.session,
            revision: frame.request.revision,
            world: frame.metrics,
            startup: serde_json::json!({
                "fixed": fixed.startup_audit(),
                "simulation_seconds": runtime.simulation_seconds,
                "vehicles": (0..runtime.vehicles.len()).map(|index| serde_json::json!({
                    "index": index,
                    "active": runtime.physics.body_active(runtime.physics.chassis_handle(index)),
                    "absolute_position": runtime.persistent_world.absolute_vehicle_position(
                        &runtime.physics, &frame.origin, index),
                })).collect::<Vec<_>>(),
                "retained_source_bodies": runtime.area.bodies.iter().filter_map(|(id, handle)| {
                    let record = reference.0.bodies.iter().find(|body| body.id == *id)?;
                    runtime.persistent_world.body_snapshot(*handle, crate::save::BodyIdentity::Source {
                        source: crate::save::SourceKey::from(&record.source),
                    })
                }).collect::<Vec<_>>(),
                "player": {
                    "absolute_position": frame.origin.absolute(runtime.area.player.body_pose(&runtime.physics).position),
                    "controller": runtime.area.player.body_pose(&runtime.physics).position,
                    "body": runtime.physics.rapier().bodies[runtime.area.player.body_handle()].translation().to_array(),
                    "grounded": runtime.area.player.grounded(),
                },
                "vehicle": {
                    "position": car.pose.position, "rotation": car.pose.rotation,
                    "velocity": car.linear_velocity, "angular_velocity": car.angular_velocity,
                    "wheel_contacts": car.wheels_contact,
                },
                "supported": runtime.world_supported(),
                "required_ready": runtime.stream.required_ready(),
                "failure": frame.failure,
                "surfaces": surfaces.startup_audit(),
                "textures": materials.world_textures.startup_audit(&images, &server),
                "primary": materials.primary_startup_audit(&server),
                "gpu_ledger": {
                    "prototype_gpu": runtime.stream.presentation_allocation().prototype_gpu,
                    "active_gpu": runtime.stream.presentation_allocation().active_gpu,
                },
                "jobs": runtime.stream.startup_job_audit(),
                "instances": instances.startup_audit(&grass),
                "grass": grass.startup_audit(),
                "writers": writers.startup_audit(),
                "pipelines": pipelines.startup_audit(),
            }),
            assets: groups,
            physics_bodies: runtime.physics.rapier().bodies.len(),
            physics_colliders: runtime.physics.collider_count(),
            active_items: runtime.area.items.len(),
            measurement_cache_cpu_bytes: super::map_bytes(
                measured.bytes.capacity(),
                std::mem::size_of::<(AssetId<Mesh>, usize)>(),
            ),
            process_bytes: None,
            driver_bytes: None,
            limits: "World values are capacity charges and reservations; pending overlaps active/prototype charges. Asset values count unique logical mesh/image storage and retained CPU payloads. They exclude ECS/physics allocator overhead, skins, uniforms, and renderer-internal targets. Imported asset payloads are a partial baseline. Logical handles do not prove GPU allocation or residency. Snapshot scratch and JSON are transient development allocations.",
        };
        if let Ok(json) = serde_json::to_string(&snapshot) {
            crate::browser::game_measurements(&json);
        }
    }
}
