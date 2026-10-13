use bevy::asset::RenderAssetUsages;
use bevy::ecs as bevy_ecs;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::world::{ChunkData, ChunkKey, LegacyWorldStream, PropKind, road_sample};

#[derive(Component)]
pub struct ChunkVisual(pub [f64; 3]);

struct ChunkAssets {
    key: ChunkKey,
    lateral_band: i64,
    entity: Entity,
    terrain: Handle<Mesh>,
}

#[derive(Resource)]
pub struct WorldVisuals {
    chunks: Vec<ChunkAssets>,
    cube: Handle<Mesh>,
    terrain_material: Handle<StandardMaterial>,
    stripe: Handle<StandardMaterial>,
    rock: Handle<StandardMaterial>,
    cactus: Handle<StandardMaterial>,
    post: Handle<StandardMaterial>,
    wall: Handle<StandardMaterial>,
    roof: Handle<StandardMaterial>,
    pump: Handle<StandardMaterial>,
}

impl WorldVisuals {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            chunks: Vec::with_capacity(11),
            cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            terrain_material: materials.add(StandardMaterial {
                perceptual_roughness: 1.0,
                ..default()
            }),
            stripe: materials.add(Color::srgb(0.86, 0.75, 0.48)),
            rock: materials.add(Color::srgb(0.44, 0.32, 0.22)),
            cactus: materials.add(Color::srgb(0.25, 0.34, 0.19)),
            post: materials.add(Color::srgb(0.80, 0.75, 0.63)),
            wall: materials.add(Color::srgb(0.69, 0.61, 0.45)),
            roof: materials.add(Color::srgb(0.49, 0.25, 0.16)),
            pump: materials.add(Color::srgb(0.67, 0.18, 0.10)),
        }
    }

    pub fn sync(
        &mut self,
        stream: &LegacyWorldStream,
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
                    let material = match prop.kind {
                        PropKind::Rock => &self.rock,
                        PropKind::Cactus => &self.cactus,
                        PropKind::Post => &self.post,
                    };
                    let position = chunk_local(data, prop.absolute_point);
                    root.spawn((
                        Mesh3d(self.cube.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_translation(position)
                            .with_scale(Vec3::from_array(prop.half_size) * 2.0),
                    ));
                    if prop.kind == PropKind::Cactus {
                        for side in [-1.0, 1.0] {
                            root.spawn((
                                Mesh3d(self.cube.clone()),
                                MeshMaterial3d(self.cactus.clone()),
                                Transform::from_translation(
                                    position + Vec3::new(side * 0.5, 0.2, 0.0),
                                )
                                .with_scale(Vec3::new(0.7, 0.2, 0.2)),
                            ));
                            root.spawn((
                                Mesh3d(self.cube.clone()),
                                MeshMaterial3d(self.cactus.clone()),
                                Transform::from_translation(
                                    position + Vec3::new(side * 0.8, 0.5, 0.0),
                                )
                                .with_scale(Vec3::new(0.2, 0.8, 0.2)),
                            ));
                        }
                    }
                }
                for station in &data.stations {
                    let point = chunk_local(data, station.absolute_point);
                    for (offset, scale, material) in [
                        (
                            Vec3::new(6.0, 1.5, 0.0),
                            Vec3::new(4.0, 3.0, 6.0),
                            &self.wall,
                        ),
                        (
                            Vec3::new(6.0, 3.05, 0.0),
                            Vec3::new(4.4, 0.15, 6.4),
                            &self.roof,
                        ),
                        (
                            Vec3::new(0.0, 0.7, -3.0),
                            Vec3::new(0.7, 1.4, 0.7),
                            &self.pump,
                        ),
                        (
                            Vec3::new(0.0, 1.0, -2.64),
                            Vec3::new(0.4, 0.3, 0.02),
                            &self.post,
                        ),
                    ] {
                        root.spawn((
                            Mesh3d(self.cube.clone()),
                            MeshMaterial3d(material.clone()),
                            Transform::from_translation(point + offset).with_scale(scale),
                        ));
                    }
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
    mesh
}

mod lamps;
pub(crate) use lamps::{RoadLamps, finish_road_lamp_warmup, prepare_road_lamp};

mod source;
pub use source::{SourceWorldVisuals, SurfaceMaterial};

pub mod preflight;
pub mod prototype_loader;

pub mod prototypes;

pub mod pipelines;

mod surface_bindings;

pub mod grass_jobs;

pub mod instance_materials;

pub mod lod;

pub mod instances;

pub mod runtime;
