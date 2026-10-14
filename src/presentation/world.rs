use super::materials::{GeneratedWorldMaterial, SourceMaterial, generated_mesh};
use bevy::asset::RenderAssetUsages;
use bevy::ecs as bevy_ecs;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::world::{ChunkData, ChunkKey, PUMP_HALF_SIZE, PropKind, WorldStream, road_sample};

#[derive(Component)]
pub struct ChunkVisual(pub [f64; 3]);

#[derive(Clone)]
struct ChunkAssets {
    key: ChunkKey,
    lateral_band: i64,
    entity: Entity,
    terrain: Handle<Mesh>,
}

#[derive(Resource, Clone)]
pub struct WorldVisuals {
    chunks: Vec<ChunkAssets>,
    cube: Handle<Mesh>,
    terrain_material: Handle<SourceMaterial>,
    stripe: Handle<SourceMaterial>,
    rock: Handle<SourceMaterial>,
    cactus: Handle<SourceMaterial>,
    post: Handle<SourceMaterial>,
    pump: Handle<SourceMaterial>,
}

impl WorldVisuals {
    pub(crate) fn validate_reused_assets(
        &self,
        meshes: &Assets<Mesh>,
        materials: &Assets<SourceMaterial>,
    ) -> Result<(), crate::save::SaveError> {
        if !meshes.contains(self.cube.id()) {
            return Err(crate::save::SaveError::MissingAssets(
                "generated chunk cube mesh".into(),
            ));
        }
        for (name, handle) in [
            ("terrain", &self.terrain_material),
            ("stripe", &self.stripe),
            ("rock", &self.rock),
            ("cactus", &self.cactus),
            ("post", &self.post),
            ("pump", &self.pump),
        ] {
            if !materials.contains(handle.id()) {
                return Err(crate::save::SaveError::MissingAssets(format!(
                    "generated chunk {name} material"
                )));
            }
        }
        Ok(())
    }
    pub(crate) fn validate_prepared_terrain(
        &self,
        meshes: &Assets<Mesh>,
    ) -> Result<(), crate::save::SaveError> {
        for chunk in &self.chunks {
            if !meshes.contains(chunk.terrain.id()) {
                return Err(crate::save::SaveError::MissingAssets(format!(
                    "prepared terrain mesh {}",
                    chunk.key.0
                )));
            }
        }
        Ok(())
    }

    #[cfg(feature = "save-fixture")]
    pub(crate) fn fixture_material(&self) -> bevy::asset::AssetId<SourceMaterial> {
        self.terrain_material.id()
    }
    pub(crate) fn empty_copy(&self) -> Self {
        let mut copy = self.clone();
        copy.chunks.clear();
        copy
    }
    pub(crate) fn release(self, world: &mut World) {
        for chunk in self.chunks {
            world.despawn(chunk.entity);
            world
                .resource_mut::<Assets<Mesh>>()
                .remove(chunk.terrain.id());
        }
    }

    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<SourceMaterial>) -> Self {
        Self {
            chunks: Vec::with_capacity(11),
            cube: meshes.add(generated_mesh(Cuboid::new(1.0, 1.0, 1.0).into())),
            terrain_material: materials.add(GeneratedWorldMaterial::Terrain.material()),
            stripe: materials.add(GeneratedWorldMaterial::Stripe.material()),
            rock: materials.add(GeneratedWorldMaterial::Rock.material()),
            cactus: materials.add(GeneratedWorldMaterial::Cactus.material()),
            post: materials.add(GeneratedWorldMaterial::Post.material()),
            pump: materials.add(GeneratedWorldMaterial::Pump.material()),
        }
    }

    pub fn sync(
        &mut self,
        stream: &WorldStream,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
    ) {
        self.chunks.retain(|chunk| {
            if stream
                .chunks()
                .any(|c| c.data.key == chunk.key && c.data.lateral_band == chunk.lateral_band)
            {
                return true;
            }
            commands.entity(chunk.entity).despawn();
            meshes.remove(chunk.terrain.id());
            false
        });
        for chunk in stream.chunks() {
            if self
                .chunks
                .iter()
                .any(|c| c.key == chunk.data.key && c.lateral_band == chunk.data.lateral_band)
            {
                continue;
            }
            let data = &chunk.data;
            let terrain = meshes.add(terrain_mesh(data));
            let entity = commands
                .spawn((
                    ChunkVisual(data.origin()),
                    Transform::from_translation(Vec3::from_array(
                        stream.origin.local(data.origin()),
                    )),
                    Visibility::default(),
                ))
                .id();
            commands.entity(entity).with_children(|root| {
                root.spawn((
                    Mesh3d(terrain.clone()),
                    MeshMaterial3d(self.terrain_material.clone()),
                    GeneratedWorldMaterial::Terrain,
                ));
                let start = data.key.0 as f64 * 128.0;
                for i in 0..16 {
                    if data.lateral_band.abs() > 1 {
                        break;
                    }
                    let sample = road_sample(stream.seed, start + i as f64 * 8.0 + 4.0);
                    let yaw = (-sample.tangent[0]).atan2(-sample.tangent[2]);
                    root.spawn((
                        Mesh3d(self.cube.clone()),
                        MeshMaterial3d(self.stripe.clone()),
                        GeneratedWorldMaterial::Stripe,
                        Transform::from_xyz(
                            (sample.center[0] - data.origin()[0]) as f32,
                            sample.center[1] as f32 + 0.018,
                            -(i as f32 * 8.0 + 4.0),
                        )
                        .with_rotation(Quat::from_rotation_y(yaw))
                        .with_scale(Vec3::new(0.12, 0.015, 3.0)),
                    ));
                }
                for prop in &data.props {
                    let (material, identity) = match prop.kind {
                        PropKind::Rock => (&self.rock, GeneratedWorldMaterial::Rock),
                        PropKind::Cactus => (&self.cactus, GeneratedWorldMaterial::Cactus),
                        PropKind::Post => (&self.post, GeneratedWorldMaterial::Post),
                    };
                    let position = chunk_local(data, prop.absolute_point);
                    root.spawn((
                        Mesh3d(self.cube.clone()),
                        MeshMaterial3d(material.clone()),
                        identity,
                        Transform::from_translation(position)
                            .with_scale(Vec3::from_array(prop.half_size) * 2.0),
                    ));
                    if prop.kind == PropKind::Cactus {
                        for side in [-1.0, 1.0] {
                            root.spawn((
                                Mesh3d(self.cube.clone()),
                                MeshMaterial3d(self.cactus.clone()),
                                GeneratedWorldMaterial::Cactus,
                                Transform::from_translation(
                                    position + Vec3::new(side * 0.5, 0.2, 0.0),
                                )
                                .with_scale(Vec3::new(0.7, 0.2, 0.2)),
                            ));
                            root.spawn((
                                Mesh3d(self.cube.clone()),
                                MeshMaterial3d(self.cactus.clone()),
                                GeneratedWorldMaterial::Cactus,
                                Transform::from_translation(
                                    position + Vec3::new(side * 0.8, 0.5, 0.0),
                                )
                                .with_scale(Vec3::new(0.2, 0.8, 0.2)),
                            ));
                        }
                    }
                }
                // The source stations have no pump. This one keeps the roadside refuel stop.
                for station in &data.stations {
                    root.spawn((
                        Mesh3d(self.cube.clone()),
                        MeshMaterial3d(self.pump.clone()),
                        GeneratedWorldMaterial::Pump,
                        Transform::from_translation(
                            chunk_local(data, station.pump_point) + Vec3::Y * PUMP_HALF_SIZE[1],
                        )
                        .with_scale(Vec3::from_array(PUMP_HALF_SIZE) * 2.0),
                    ));
                }
            });
            self.chunks.push(ChunkAssets {
                key: data.key,
                lateral_band: data.lateral_band,
                entity,
                terrain,
            });
        }
    }
}

fn chunk_local(data: &ChunkData, point: [f64; 3]) -> Vec3 {
    Vec3::new(
        (point[0] - data.origin()[0]) as f32,
        point[1] as f32,
        (point[2] - data.origin()[2]) as f32,
    )
}

fn terrain_mesh(chunk: &ChunkData) -> Mesh {
    let colors: Vec<[f32; 4]> = chunk
        .vertices
        .iter()
        .enumerate()
        .map(|(i, vertex)| {
            let lateral = chunk.lateral_samples[i % chunk.lateral_samples.len()].abs();
            let x = vertex[0] as f64 + chunk.origin()[0];
            let z = vertex[2] as f64 + chunk.origin()[2];
            let variation = ((x * 0.033).sin() * (z * 0.027).cos()) as f32 * 0.035;
            let color = if lateral <= 4.0 {
                Color::srgb(0.22 + variation * 0.2, 0.21 + variation * 0.2, 0.18)
            } else {
                Color::srgb(0.59 + variation, 0.44 + variation, 0.28 + variation * 0.6)
            };
            color.to_linear().to_f32_array()
        })
        .collect();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, chunk.vertices.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(
        chunk.indices.iter().flatten().copied().collect(),
    ));
    mesh.compute_smooth_normals();
    generated_mesh(mesh)
}

/// Upper bound from this generator: terrain, stripes, rocks, posts, cactus parts, pumps.
/// Source imported primitives have a separate capacity.
pub fn source_shadow_capacity() -> usize {
    const TERRAIN: usize = 1;
    const STRIPES: usize = 16;
    const ROCKS: usize = 2;
    const POSTS: usize = 2;
    const CACTUS_PARTS: usize = 1 + 2 * 2;
    const PUMPS: usize = 1;
    crate::world::active_chunks(0.).len()
        * (TERRAIN + STRIPES + ROCKS + POSTS + CACTUS_PARTS + PUMPS)
}

pub(crate) mod lamps;
mod prototype;
mod stations;
pub(crate) use lamps::{RoadLamps, finish_road_lamp_warmup, prepare_road_lamp, sync_road_lamps};
pub(crate) use stations::{StationModels, prepare_stations, sync_stations};
