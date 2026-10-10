use bevy::asset::RenderAssetUsages;
use bevy::ecs as bevy_ecs;
use bevy::gltf::{
    Gltf, GltfAssetLabel, GltfLoaderSettings, convert_coordinates::GltfConvertCoordinates,
};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::reference::world::WorldReference;
use crate::world::{RoadMesh, Tile, WorldStream, mirror};

/// Absolute game point of a streamed entity. The origin shift moves it each frame.
#[derive(Component)]
pub struct WorldVisual(pub [f64; 3]);

struct PolePart {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
}

#[derive(Resource)]
pub struct WorldVisuals {
    /// Readiness waits for the whole world asset.
    pub asset: Handle<Gltf>,
    tiles: Vec<([i32; 2], Entity, Handle<Mesh>)>,
    roads: Vec<(usize, Entity, Handle<Mesh>)>,
    terrain: Handle<StandardMaterial>,
    /// Road material and pole parts for each biome.
    biomes: Vec<(Handle<StandardMaterial>, Vec<PolePart>)>,
    cube: Handle<Mesh>,
    wall: Handle<StandardMaterial>,
    roof: Handle<StandardMaterial>,
    pump: Handle<StandardMaterial>,
    sign: Handle<StandardMaterial>,
}

fn load<A: Asset>(server: &AssetServer, path: String) -> Handle<A> {
    server
        .load_builder()
        .with_settings(|settings: &mut GltfLoaderSettings| {
            settings.load_cameras = false;
            settings.load_lights = false;
            settings.load_animations = false;
            settings.convert_coordinates = Some(GltfConvertCoordinates {
                rotate_scene_entity: false,
                rotate_meshes: false,
            });
        })
        .load(path)
}

fn material(server: &AssetServer, path: &str, index: u32) -> Handle<StandardMaterial> {
    let label = GltfAssetLabel::Material {
        index: index as usize,
        is_scale_inverted: false,
    };
    load(server, format!("{path}#{label}/std"))
}

fn matrix(values: [f32; 16]) -> Transform {
    Transform::from_matrix(Mat4::from_cols_array(&values))
}

impl WorldVisuals {
    pub fn new(
        server: &AssetServer,
        world: &WorldReference,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        let path = &world.asset.path;
        let biomes = world
            .road
            .biomes
            .iter()
            .map(|biome| {
                let parts = biome
                    .pole
                    .renderers
                    .iter()
                    .map(|renderer| PolePart {
                        mesh: load(
                            server,
                            GltfAssetLabel::Primitive {
                                mesh: renderer.mesh as usize,
                                primitive: 0,
                            }
                            .from_asset(path.clone())
                            .to_string(),
                        ),
                        material: material(server, path, renderer.material),
                        transform: matrix(mirror(renderer.matrix)),
                    })
                    .collect();
                (material(server, path, biome.material), parts)
            })
            .collect();
        Self {
            asset: load(server, path.clone()),
            tiles: Vec::with_capacity(32),
            roads: Vec::with_capacity(128),
            terrain: material(server, path, world.terrain_material),
            biomes,
            cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            wall: materials.add(Color::srgb(0.69, 0.61, 0.45)),
            roof: materials.add(Color::srgb(0.49, 0.25, 0.16)),
            pump: materials.add(Color::srgb(0.67, 0.18, 0.10)),
            sign: materials.add(Color::srgb(0.80, 0.75, 0.63)),
        }
    }

    pub fn sync(
        &mut self,
        stream: &WorldStream,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
    ) {
        self.tiles.retain(|(key, entity, mesh)| {
            if stream.tiles().any(|loaded| loaded.tile.key == *key) {
                return true;
            }
            commands.entity(*entity).despawn();
            meshes.remove(mesh.id());
            false
        });
        self.roads.retain(|(index, entity, mesh)| {
            if stream.roads().any(|road| road.index == *index) {
                return true;
            }
            commands.entity(*entity).despawn();
            meshes.remove(mesh.id());
            false
        });
        for loaded in stream.tiles() {
            let tile = &loaded.tile;
            if self.tiles.iter().any(|(key, _, _)| *key == tile.key) {
                continue;
            }
            let mesh = meshes.add(tile_mesh(tile));
            let entity = commands
                .spawn((
                    WorldVisual(tile.center),
                    Transform::from_translation(Vec3::from_array(stream.origin.local(tile.center))),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(self.terrain.clone()),
                ))
                .id();
            self.tiles.push((tile.key, entity, mesh));
        }
        for road in stream.roads() {
            if self.roads.iter().any(|(index, _, _)| *index == road.index) {
                continue;
            }
            let mesh = meshes.add(road_mesh(&road.mesh));
            let (material, parts) = &self.biomes[road.mesh.biome];
            let entity = commands
                .spawn((
                    WorldVisual(road.start),
                    Transform::from_translation(Vec3::from_array(stream.origin.local(road.start))),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                ))
                .id();
            commands.entity(entity).with_children(|root| {
                root.spawn((
                    matrix(road.mesh.pole.to_cols_array()),
                    Visibility::default(),
                ))
                .with_children(|pole| {
                    for part in parts {
                        pole.spawn((
                            part.transform,
                            Mesh3d(part.mesh.clone()),
                            MeshMaterial3d(part.material.clone()),
                        ));
                    }
                });
                let Some(station) = road.station else {
                    return;
                };
                let point = Vec3::from_array(std::array::from_fn(|i| {
                    (station.point[i] - road.start[i]) as f32
                }));
                let rotation = Quat::from_rotation_y(station.yaw);
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
                        &self.sign,
                    ),
                ] {
                    root.spawn((
                        Mesh3d(self.cube.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_translation(point + rotation * offset)
                            .with_rotation(rotation)
                            .with_scale(scale),
                    ));
                }
            });
            self.roads.push((road.index, entity, mesh));
        }
    }
}

fn mesh(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    indices: &[[u32; 3]],
) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec());
    mesh.insert_indices(Indices::U32(indices.iter().flatten().copied().collect()));
    mesh
}

fn tile_mesh(tile: &Tile) -> Mesh {
    mesh(&tile.positions, &tile.normals, &tile.uvs, &tile.indices)
}

fn road_mesh(road: &RoadMesh) -> Mesh {
    mesh(&road.positions, &road.normals, &road.uvs, &road.indices)
}
