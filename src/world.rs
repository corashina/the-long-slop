use rapier3d::glamx::{Quat, Vec3};
use rapier3d::math::{Pose, Vector};
use rapier3d::prelude::ColliderBuilder;

use crate::reference::world::WorldReference;
use crate::reference::{Bounds, ColliderGeometry, ReferenceManifest};
use crate::vehicle::{ColliderHandle, PhysicsWorld, VehiclePose};
use generation::{FlatZone, RoadPath, RoadPoints, TerrainNoise};

pub mod generation;
mod road;
mod terrain;

pub use road::{PoleCollider, RoadMesh, mirror};
pub use terrain::Tile;

pub const REBASE_LIMIT: f32 = 2048.0;
/// Placeholder fuel stops sit beside every sixth road segment.
pub const STATION_SEGMENTS: usize = 6;
const STATION_OFFSET: f32 = 10.0;
/// `roadGenScript.controlDist`: road placement waits for 100 meters of travel.
const ROAD_REFRESH: f64 = 100.0;
/// `TerrainGenerator.distFromTerrain`: tiles read road segments that start this close.
const TILE_ROAD_RANGE: f64 = 1000.0;
/// `RoadCoordsForTerrainHeight2`: single height queries read road segments this close.
const POINT_ROAD_RANGE: f64 = 150.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct WorldOrigin {
    pub offset: [f64; 3],
}

impl WorldOrigin {
    pub fn absolute(&self, local: [f32; 3]) -> [f64; 3] {
        std::array::from_fn(|i| self.offset[i] + local[i] as f64)
    }

    pub fn local(&self, absolute: [f64; 3]) -> [f32; 3] {
        std::array::from_fn(|i| (absolute[i] - self.offset[i]) as f32)
    }

    pub fn shift(&mut self, shift: [f32; 3]) {
        for (offset, shift) in self.offset.iter_mut().zip(shift) {
            *offset += shift as f64;
        }
    }
}

/// Source generators in one place. Game space is Unity space with Z mirrored and the home lift removed.
pub struct Generator {
    pub reference: WorldReference,
    pub noise: TerrainNoise,
    pub path: RoadPath,
    zones: Vec<FlatZone>,
    /// The source lifts the home onto the terrain. The game keeps the home fixed and lowers the world.
    pub lift: f64,
}

impl Generator {
    pub fn new(reference: &WorldReference) -> Self {
        let noise = TerrainNoise::new(
            reference.seed,
            reference.edge_of_world,
            reference.edge_gradient,
        );
        let path = RoadPath::new(reference, &noise);
        let mut generator = Self {
            reference: reference.clone(),
            noise,
            path,
            zones: Vec::new(),
            lift: 0.0,
        };
        // `StartObjs` align to the road but not to buildings.
        let anchor = reference.height_anchor;
        let point = [anchor[0] as f64, anchor[2] as f64];
        generator.lift = generator.height(point, POINT_ROAD_RANGE, false) - anchor[1] as f64;
        generator.zones = reference
            .flat_zones
            .iter()
            .map(|zone| FlatZone {
                center: [
                    zone.center[0],
                    (zone.center[1] as f64 + generator.lift) as f32,
                    zone.center[2],
                ],
                inner_sqr: zone.inner_sqr,
                outer_sqr: zone.outer_sqr,
            })
            .collect();
        generator
    }

    /// Calculates road segments up to an absolute Unity Z.
    pub fn extend_to(&mut self, z: f64) {
        while self.path.end_z() < z && self.path.extend(&self.reference, &self.noise) {}
    }

    /// Bone ends of segments that start within `range` of a Unity X and Z.
    fn road_points(&self, point: [f64; 2], range: f64, points: &mut Vec<[f32; 3]>) {
        points.clear();
        let segments = &self.path.segments;
        // Segments run toward +Z. Yaw stays within 90 degrees.
        let first =
            segments.partition_point(|s| self.path.origin[2] + s.position[2] < point[1] - range);
        for index in first..segments.len() {
            let start = self.path.start(index);
            if start[2] > point[1] + range {
                break;
            }
            if (start[0] - point[0]).powi(2) + (start[2] - point[1]).powi(2) < range * range {
                points.extend((0..segments[index].points.len()).map(|b| self.path.point(index, b)));
            }
        }
    }

    /// Game height at a Unity X and Z.
    pub fn height(&mut self, point: [f64; 2], range: f64, zones: bool) -> f64 {
        self.extend_to(point[1] + range);
        let mut points = Vec::new();
        self.road_points(point, range, &mut points);
        let road = RoadPoints {
            points: &points,
            range: self.reference.road.point_range,
        };
        let zones = if zones { &self.zones[..] } else { &[] };
        self.noise
            .height([point[0] as f32, point[1] as f32], zones, &road) as f64
            - self.lift
    }

    pub fn tile(&mut self, key: [i32; 2], rooms: &[Bounds]) -> Tile {
        let terrain = &self.reference.terrain;
        let (size, resolution) = (terrain.tile_size, terrain.resolution);
        let center = [key[0] as f64 * size as f64, key[1] as f64 * size as f64];
        self.extend_to(center[1] + TILE_ROAD_RANGE);
        let mut points = Vec::new();
        self.road_points(center, TILE_ROAD_RANGE, &mut points);
        let road = RoadPoints {
            points: &points,
            range: self.reference.road.point_range,
        };
        let lift = self.lift;
        terrain::build(
            key,
            size,
            resolution,
            |c| (self.noise.height(c, &self.zones, &road) as f64 - lift) as f32,
            rooms,
        )
    }

    /// `TerrainGenerator.GetTerrainColor` at an absolute game point on the surface.
    pub fn terrain_color(&self, point: [f64; 3]) -> [f32; 4] {
        self.noise.vertex_color(
            [point[0] as f32, -point[2] as f32],
            (point[1] + self.lift) as f32,
        )
    }

    /// Converts an absolute Unity point to game space.
    pub fn game(&self, point: [f64; 3]) -> [f64; 3] {
        [point[0], point[1] - self.lift, -point[2]]
    }

    /// Game start of one road segment.
    pub fn start(&self, index: usize) -> [f64; 3] {
        self.game(self.path.start(index))
    }

    pub fn segment(&self, index: usize) -> RoadMesh {
        let start = self.path.start(index);
        let biome = self
            .noise
            .biome([start[0] as f32, start[2] as f32], start[1] as f32);
        road::place(
            &self.reference.road,
            biome,
            &self.path.segments[index].rotations,
            index,
            self.reference.seed,
        )
    }

    /// Placeholder fuel stop beside the start of a road segment.
    pub fn station(&self, index: usize) -> Station {
        let yaw = self.path.segments[index].rotations[0][1].to_radians();
        let start = self.path.start(index);
        let right = [yaw.cos() as f64, 0.0, -yaw.sin() as f64];
        let offset = STATION_OFFSET as f64;
        Station {
            id: index / STATION_SEGMENTS,
            point: self.game(std::array::from_fn(|i| start[i] + right[i] * offset)),
            yaw: -yaw,
        }
    }
}

/// Game yaw turns local -Z toward the road heading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Station {
    pub id: usize,
    pub point: [f64; 3],
    pub yaw: f32,
}

impl Station {
    fn pose(&self, origin: &WorldOrigin, local: [f32; 3]) -> Pose {
        let rotation = Quat::from_rotation_y(self.yaw);
        let point = Vec3::from_array(origin.local(self.point)) + rotation * Vec3::from_array(local);
        Pose::from_parts(point, rotation)
    }
}

pub struct LoadedTile {
    pub tile: Tile,
    collider: ColliderHandle,
}

pub struct LoadedRoad {
    pub index: usize,
    pub start: [f64; 3],
    pub mesh: RoadMesh,
    pub station: Option<Station>,
    colliders: Vec<ColliderHandle>,
    pump: Option<ColliderHandle>,
}

pub struct WorldStream {
    pub origin: WorldOrigin,
    pub generator: Generator,
    tiles: Vec<LoadedTile>,
    roads: Vec<LoadedRoad>,
    /// `TerrainSetting.PreComputedCoords` in tile units, nearest first.
    offsets: Vec<[i32; 2]>,
    interiors: Vec<Bounds>,
    tile_keys: Option<[[i32; 2]; 2]>,
    road_points: Option<[[f64; 2]; 2]>,
    needed: Vec<usize>,
    station_items: Vec<(usize, crate::reference::ItemId)>,
}

fn offsets(generate_distance: f32, size: f32) -> Vec<[i32; 2]> {
    let draw = ((generate_distance / size) as i32).max(1);
    let needed = (((draw as f32 + 0.5).powi(2) * std::f32::consts::PI) as usize).max(5);
    let side = (draw * 2 + 1).max(4);
    let mut offsets: Vec<[i32; 2]> = (0..side * side)
        .map(|i| [i % side - draw, i / side - draw])
        .collect();
    // The source bubble sort is stable.
    offsets.sort_by_key(|o| o[0] * o[0] + o[1] * o[1]);
    offsets.truncate(needed);
    offsets
}

impl WorldStream {
    pub fn new(reference: &ReferenceManifest) -> Self {
        let world = &reference.world;
        let home = reference
            .nodes
            .iter()
            .find(|node| node.id == reference.area.home_root)
            .expect("validated home root")
            .asset;
        let mut interiors = Vec::new();
        for collider in &reference.colliders {
            let Some(node) = reference.nodes.iter().find(|node| node.id == collider.node) else {
                continue;
            };
            if !node.name.starts_with("interior") || node.asset != home {
                continue;
            }
            let ColliderGeometry::Convex { points } = &collider.geometry else {
                continue;
            };
            let body = reference
                .bodies
                .iter()
                .find(|body| body.id == collider.body)
                .unwrap();
            let pose = crate::vehicle::to_rapier(VehiclePose {
                position: body.pose.translation,
                rotation: body.pose.rotation,
            });
            let mut bounds = Bounds {
                min: [f32::INFINITY; 3],
                max: [f32::NEG_INFINITY; 3],
            };
            for point in points {
                let point = (pose * Vector::from_array(*point)).to_array();
                for axis in 0..3 {
                    bounds.min[axis] = bounds.min[axis].min(point[axis]);
                    bounds.max[axis] = bounds.max[axis].max(point[axis]);
                }
            }
            interiors.push(bounds);
        }
        Self {
            origin: WorldOrigin::default(),
            generator: Generator::new(world),
            tiles: Vec::new(),
            roads: Vec::new(),
            offsets: offsets(world.terrain.generate_distance, world.terrain.tile_size),
            interiors,
            tile_keys: None,
            road_points: None,
            needed: Vec::new(),
            station_items: Vec::new(),
        }
    }

    pub fn tiles(&self) -> impl Iterator<Item = &LoadedTile> {
        self.tiles.iter()
    }

    pub fn roads(&self) -> impl Iterator<Item = &LoadedRoad> {
        self.roads.iter()
    }

    /// Loads the world around the focus and the car. Both are `ImportantObj` in the source.
    pub fn sync_at(&mut self, focus: [f64; 3], physics: &mut PhysicsWorld) {
        let car = self.origin.absolute(physics.snapshot().pose.position);
        let points = [[focus[0], -focus[2]], [car[0], -car[2]]];
        let tiles = self.sync_tiles(points, physics);
        let roads = self.sync_roads(points, physics);
        if tiles || roads {
            physics.refresh_queries();
        }
    }

    fn sync_tiles(&mut self, points: [[f64; 2]; 2], physics: &mut PhysicsWorld) -> bool {
        let terrain = &self.generator.reference.terrain;
        let size = terrain.tile_size as f64;
        // `Mathf.RoundToInt` rounds half to even.
        let keys = points.map(|p| p.map(|v| (v / size).round_ties_even() as i32));
        if self.tile_keys == Some(keys) {
            return false;
        }
        self.tile_keys = Some(keys);
        let remove = (terrain.remove_distance as f64 / size).powi(2);
        self.tiles.retain(|loaded| {
            let key = loaded.tile.key;
            let near = keys.iter().any(|k| {
                ((key[0] - k[0]) as f64).powi(2) + ((key[1] - k[1]) as f64).powi(2) < remove
            });
            if !near {
                physics.remove_static(loaded.collider);
            }
            near
        });
        for center in keys {
            for offset in &self.offsets {
                let key = [center[0] + offset[0], center[1] + offset[1]];
                if self.tiles.iter().any(|loaded| loaded.tile.key == key) {
                    continue;
                }
                let tile = self.generator.tile(key, &self.interiors);
                let collider = physics.insert_static(
                    ColliderBuilder::trimesh(
                        tile.positions
                            .iter()
                            .copied()
                            .map(Vector::from_array)
                            .collect(),
                        tile.indices.clone(),
                    )
                    .expect("terrain tile triangles")
                    .translation(Vector::from_array(self.origin.local(tile.center)))
                    .user_data(crate::vehicle::TERRAIN_COLLIDER_FLAG),
                );
                self.tiles.push(LoadedTile { tile, collider });
            }
        }
        true
    }

    fn sync_roads(&mut self, points: [[f64; 2]; 2], physics: &mut PhysicsWorld) -> bool {
        if let Some(last) = self.road_points
            && points.iter().zip(&last).all(|(p, l)| {
                (p[0] - l[0]).powi(2) + (p[1] - l[1]).powi(2) < ROAD_REFRESH * ROAD_REFRESH
            })
        {
            return false;
        }
        self.road_points = Some(points);
        let range = self.generator.reference.road.generate_distance as f64;
        let far = points.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
        let near = points.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
        self.generator.extend_to(far + range);
        let path = &self.generator.path;
        self.needed.clear();
        let first = path
            .segments
            .partition_point(|s| path.origin[2] + s.position[2] < near - range);
        for index in first..path.segments.len() {
            let start = path.start(index);
            if start[2] > far + range {
                break;
            }
            // `GetNearRoads` compares squared distance with the squared range.
            if points
                .iter()
                .any(|p| (start[0] - p[0]).powi(2) + (start[2] - p[1]).powi(2) <= range * range)
            {
                self.needed.push(index);
            }
        }
        self.roads.retain(|road| {
            let keep = self.needed.contains(&road.index);
            if !keep {
                for handle in &road.colliders {
                    physics.remove_static(*handle);
                }
            }
            keep
        });
        for i in 0..self.needed.len() {
            let index = self.needed[i];
            if self.roads.iter().any(|road| road.index == index) {
                continue;
            }
            let road = self.place_road(index, physics);
            self.roads.push(road);
        }
        true
    }

    fn place_road(&self, index: usize, physics: &mut PhysicsWorld) -> LoadedRoad {
        let mesh = self.generator.segment(index);
        let start = self.generator.start(index);
        let local = Vector::from_array(self.origin.local(start));
        let mut colliders = Vec::with_capacity(mesh.colliders.len() + 3);
        colliders.push(
            physics.insert_static(
                ColliderBuilder::trimesh(
                    mesh.positions
                        .iter()
                        .copied()
                        .map(Vector::from_array)
                        .collect(),
                    mesh.indices.clone(),
                )
                .expect("road segment triangles")
                .translation(local),
            ),
        );
        for collider in &mesh.colliders {
            let builder = match *collider {
                PoleCollider::Capsule { a, b, radius } => {
                    ColliderBuilder::capsule_from_endpoints(a + local, b + local, radius)
                }
                PoleCollider::Box { matrix, half_size } => {
                    let (_, rotation, translation) = matrix.to_scale_rotation_translation();
                    ColliderBuilder::cuboid(half_size.x, half_size.y, half_size.z)
                        .position(Pose::from_parts(translation + local, rotation))
                }
            };
            colliders.push(physics.insert_static(builder));
        }
        let mut station = None;
        let mut pump = None;
        if index > 0 && index % STATION_SEGMENTS == 0 {
            let stop = self.generator.station(index);
            colliders.push(
                physics.insert_static(
                    ColliderBuilder::cuboid(2.0, 1.5, 3.0)
                        .position(stop.pose(&self.origin, [6.0, 1.5, 0.0])),
                ),
            );
            let handle = physics.insert_static(
                ColliderBuilder::cuboid(0.35, 0.7, 0.35)
                    .position(stop.pose(&self.origin, [0.0, 0.7, -3.0])),
            );
            colliders.push(handle);
            pump = Some(handle);
            station = Some(stop);
        }
        LoadedRoad {
            index,
            start,
            mesh,
            station,
            colliders,
            pump,
        }
    }

    /// Call between fixed steps. Stored metadata remains absolute.
    pub fn rebase(&mut self, position: [f32; 3], physics: &mut PhysicsWorld) -> Option<[f32; 3]> {
        if position[0].abs().max(position[2].abs()) <= REBASE_LIMIT {
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

    /// Nearest road bone end to a game point, with the game yaw along the road.
    pub fn nearest_road(&self, point: [f64; 3]) -> ([f64; 3], f32) {
        let path = &self.generator.path;
        let target = [point[0], -point[2]];
        let mut best = (f64::MAX, 0, 0);
        for (index, segment) in path.segments.iter().enumerate() {
            for bone in 0..segment.points.len() {
                let p = path.point(index, bone);
                let d = (p[0] as f64 - target[0]).powi(2) + (p[2] as f64 - target[1]).powi(2);
                if d < best.0 {
                    best = (d, index, bone);
                }
            }
        }
        let (_, index, bone) = best;
        let end = path.point(index, bone);
        let start = if bone > 0 {
            path.point(index, bone - 1)
        } else {
            path.start(index).map(|v| v as f32)
        };
        // Unity heading (dx, dz) becomes game yaw atan2(-dx, dz).
        let yaw = (-(end[0] - start[0])).atan2(end[2] - start[2]);
        (self.generator.game(end.map(|v| v as f64)), yaw)
    }

    /// Placeholder fuel stop nearest to a game point.
    pub fn nearest_station(&mut self, point: [f64; 3]) -> Station {
        let index = {
            let path = &self.generator.path;
            let target = [point[0], -point[2]];
            (0..path.segments.len())
                .min_by(|a, b| {
                    let d = |i: usize| {
                        let s = path.start(i);
                        (s[0] - target[0]).powi(2) + (s[2] - target[1]).powi(2)
                    };
                    d(*a).total_cmp(&d(*b))
                })
                .unwrap_or(0)
        };
        let id = ((index as f64 / STATION_SEGMENTS as f64).round() as usize).max(1);
        let index = id * STATION_SEGMENTS;
        let z = self.generator.path.origin[2]
            + (index as f64 + 1.0) * self.generator.reference.road.segment_length as f64;
        self.generator.extend_to(z);
        self.generator.station(index)
    }

    /// Load the destination before the caller resets the vehicle.
    pub fn reset_pose(&mut self, at_station: bool, physics: &mut PhysicsWorld) -> VehiclePose {
        let car = self.origin.absolute(physics.snapshot().pose.position);
        let range = self.generator.reference.road.generate_distance as f64;
        self.generator.extend_to(-car[2] + range);
        let (mut point, yaw) = if at_station {
            let station = self.nearest_station(car);
            (station.point, station.yaw)
        } else {
            self.nearest_road(car)
        };
        self.sync_at(point, physics);
        point[1] += 1.1;
        VehiclePose {
            position: self.origin.local(point),
            rotation: [0.0, (yaw * 0.5).sin(), 0.0, (yaw * 0.5).cos()],
        }
    }
}

pub fn sync_station_supplies(
    stream: &mut WorldStream,
    items: &mut crate::interaction::items::ItemStore,
    interaction: &mut crate::interaction::InteractionState,
    physics: &mut PhysicsWorld,
) {
    for road in &stream.roads {
        let (Some(station), Some(handle)) = (road.station, road.pump) else {
            continue;
        };
        let item = if let Some((_, item)) = stream
            .station_items
            .iter()
            .find(|(id, _)| *id == station.id)
        {
            *item
        } else {
            let item = crate::reference::ItemId(0x8000_0000 + stream.station_items.len() as u32);
            let pose = crate::vehicle::from_rapier(physics.rapier().colliders[handle].position());
            items.insert_supply(item, pose);
            // Retain the road stop's original four-liter-per-second rate.
            interaction
                .register_pump(item, 4.0)
                .expect("valid station pump");
            stream.station_items.push((station.id, item));
            item
        };
        physics.rapier_mut().colliders[handle].user_data = item.0 as u128 + 1;
        let state = items.get_mut(item).expect("registered station supply");
        state.pose = crate::vehicle::from_rapier(physics.rapier().colliders[handle].position());
        state
            .fluid
            .as_mut()
            .expect("pump reservoir")
            .mixture
            .quantities_liters = [1000.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    }
}
